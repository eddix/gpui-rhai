//! Queue origin is independent of whichever view later pumps the Runtime.
use gpui::{Context, IntoElement, ParentElement, Render, TestAppContext, Window};
use gpui_rhai::*;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

struct Host {
    domain: ScriptViewHost,
    owner: Option<ScriptViewHandle>,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.domain.container(
            gpui::div().children(self.owner.iter().filter_map(|view| view.element().ok())),
        )
    }
}

fn dispatch(view: &ScriptViewHandle, id: &str, window: &mut Window, cx: &mut gpui::App) {
    view.automate(
        AutomationCommand::Dispatch {
            locator: AutomationLocator::TestId { id: id.into() },
            event: "click".into(),
            payload: None,
        },
        window,
        cx,
    )
    .unwrap();
}

fn source(operation: &str, fail_first: bool, init_open: bool) -> String {
    let queued = match operation {
        "open" => "ctx.open_window(\"late\",\"Late\",400,300,false);",
        "close" => "ctx.close_window(\"child\");",
        "focus" => "ctx.focus_window(\"child\");",
        _ => unreachable!(),
    };
    r#"
fn init(ctx) { INIT }
fn open_child(ctx,v) { ctx.open_window("child","Child",400,300,false); }
fn queued(ctx,v) { QUEUED }
fn fail(ctx,v) { throw "independent delivery failure"; }
fn view(ctx) {
 if ctx.window_id()=="domain" {
  timeout("a-first",100,false,Fn("FIRST"),());
  timeout("m-middle",100,false,Fn("SECOND"),());
  timeout("z-last-failure",100,false,Fn("fail"),());
 }
 column([text("Open child").test_id("open-child").on_click(Fn("open_child")),
  text("Open late").test_id("open-late").on_click(Fn("queued"))])
}
"#.to_owned().replace("INIT", if init_open { "if ctx.window_id()==\"domain\" { ctx.open_window(\"child\",\"Child\",400,300,false); ctx.focus_window(\"child\"); }" } else { "" })
        .replace("QUEUED",queued).replace("FIRST",if fail_first { "fail" } else { "queued" })
        .replace("SECOND",if fail_first { "queued" } else { "fail" })
}

fn setup(
    cx: &mut TestAppContext,
    operation: &str,
    fail_first: bool,
    init_open: bool,
) -> gpui::WindowHandle<Host> {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(
            entry,
            source(operation, fail_first, init_open),
        )])),
        include_str!("../../../registry/themes/default_dark.rhai"),
    )
    .runtime_clock(ManualRuntimeClock::new(Instant::now()).clock())
    .prepare()
    .unwrap();
    let window = cx.add_window(move |window, cx| {
        let domain = ScriptViewHost::new("domain", cx).unwrap();
        let owner = prepared
            .mount_window(ScriptViewConfig::new("owner"), domain.clone(), window, cx)
            .unwrap();
        Host {
            domain,
            owner: Some(owner),
        }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    if !init_open {
        window
            .update(cx, |host, window, cx| {
                dispatch(host.owner.as_ref().unwrap(), "open-child", window, cx)
            })
            .unwrap();
        cx.run_until_parked();
    }
    assert_eq!(
        cx.windows().len(),
        2,
        "setup requires an actual secondary window"
    );
    window
}

fn advance_failing_batch(host: &Host, window: &mut Window, cx: &mut gpui::App) {
    let owner = host.owner.as_ref().unwrap();
    assert!(
        owner
            .automate(AutomationCommand::AdvanceTime { millis: 100 }, window, cx)
            .is_err()
    );
    // One batch processes every independent delivery, including successes
    // after an error. Its aggregate failure leaves window work for a peer pump.
}

#[gpui::test]
fn peer_pump_cannot_reauthorize_revoked_open_close_or_focus(cx: &mut TestAppContext) {
    for operation in ["open", "close", "focus"] {
        for fail_first in [false, true] {
            for drop_only in [false, true] {
                let source = setup(cx, operation, fail_first, false);
                let child = cx
                    .windows()
                    .into_iter()
                    .find(|window| *window != *source)
                    .unwrap();
                let mut visual = gpui::VisualTestContext::from_window(child, cx);
                visual.deactivate_window();
                drop(visual);
                source
                    .update(cx, |host, window, cx| {
                        advance_failing_batch(host, window, cx);
                        let owner = host.owner.take().unwrap();
                        if !drop_only {
                            owner.dispose(cx).unwrap();
                        }
                        drop(owner);
                        cx.notify();
                    })
                    .unwrap();
                cx.run_until_parked();
                cx.background_executor
                    .advance_clock(Duration::from_millis(32));
                cx.run_until_parked();
                assert_eq!(
                    cx.windows().len(),
                    2,
                    "revoked {operation}, fail_first={fail_first}, drop_only={drop_only}"
                );
                assert!(cx.windows().contains(&child));
                if operation == "focus" {
                    assert!(
                        !child
                            .update(cx, |_, window, _| window.is_window_active())
                            .unwrap(),
                        "a revoked focus cannot activate its target"
                    );
                }
                for window in cx.windows() {
                    window
                        .update(cx, |_, window, _| window.remove_window())
                        .unwrap();
                }
                cx.run_until_parked();
            }
        }
    }
}

#[gpui::test]
fn live_origin_survives_peer_delivery_and_initial_open_focus_uses_one_reservation(
    cx: &mut TestAppContext,
) {
    for operation in ["open", "close"] {
        let source = setup(cx, operation, false, true);
        source
            .update(cx, |host, window, cx| {
                advance_failing_batch(host, window, cx)
            })
            .unwrap();
        cx.background_executor
            .advance_clock(Duration::from_millis(32));
        cx.run_until_parked();
        assert_eq!(cx.windows().len(), if operation == "open" { 3 } else { 1 });
        for window in cx.windows() {
            window
                .update(cx, |_, window, _| window.remove_window())
                .unwrap();
        }
        cx.run_until_parked();
    }
}
