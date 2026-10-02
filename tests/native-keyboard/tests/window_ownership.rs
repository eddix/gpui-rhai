use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use gpui::{
    Context, IntoElement, ParentElement, Render, TestAppContext, VisualTestContext, Window,
    WindowHandle,
};
use gpui_rhai::*;

struct Host {
    host: ScriptViewHost,
    views: Vec<ScriptViewHandle>,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(
            gpui::div().children(self.views.iter().filter_map(|view| view.element().ok())),
        )
    }
}

const SOURCE: &str = r#"
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn requested(ctx,payload){ctx.set_state("status","requested");}
fn init(ctx){ctx.set_close_handler(Fn("requested"));}
fn close(ctx,payload){ctx.close_window(ctx.window_id());}
fn focus(ctx,payload){ctx.focus_window(ctx.window_id());}
fn open(ctx,payload){ctx.open_window("child","Child",400,300,false);}
fn close_child(ctx,payload){ctx.close_window("child");}
fn view(ctx){column([text(ctx.get_state("status")).accessibility_role("status"),
text("Close").test_id("close").on_click(Fn("close")),
text("Focus").test_id("focus").on_click(Fn("focus")),
text("Open").test_id("open").on_click(Fn("open")),
text("Close child").test_id("close-child").on_click(Fn("close_child"))])}
"#;

fn prepared(source: &str) -> PreparedScriptView {
    let entry = ModuleId::parse("main").unwrap();
    EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry, source.to_owned())])),
        include_str!("../../../registry/themes/default_dark.rhai"),
    )
    .prepare()
    .unwrap()
}

fn mount(
    cx: &mut TestAppContext,
    source: &str,
    owner: bool,
) -> (WindowHandle<Host>, ScriptViewHandle, ScriptViewHost) {
    cx.update(gpui_rhai::install);
    let prepared = prepared(source);
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("rust", cx).unwrap();
        let config = ScriptViewConfig::new("owner");
        let view = if owner {
            prepared.mount_window(config, host.clone(), window, cx)
        } else {
            prepared.mount(config, host.clone(), window, cx)
        }
        .unwrap();
        *capture.borrow_mut() = Some((view.clone(), host.clone()));
        Host {
            host,
            views: vec![view],
        }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let (view, host) = captured.borrow().as_ref().unwrap().clone();
    (window, view, host)
}

fn dispatch(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    id: &str,
) -> Result<AutomationResult, ScriptViewError> {
    visual.update(|window, cx| {
        view.automate(
            AutomationCommand::Dispatch {
                locator: AutomationLocator::TestId { id: id.to_owned() },
                event: "click".to_owned(),
                payload: None,
            },
            window,
            cx,
        )
    })
}

#[gpui::test]
fn managed_embedding_focuses_opens_and_closes_real_native_windows(cx: &mut TestAppContext) {
    let (window, view, _) = mount(cx, SOURCE, true);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.deactivate_window();
    dispatch(&mut visual, &view, "focus").unwrap();
    visual.run_until_parked();
    assert!(visual.update(|window, _| window.is_window_active()));
    assert!(
        visual
            .update(|_, cx| view.last_error(cx).unwrap())
            .is_none()
    );
    dispatch(&mut visual, &view, "open").unwrap();
    visual.run_until_parked();
    assert_eq!(cx.windows().len(), 2);
    dispatch(&mut visual, &view, "close-child").unwrap();
    visual.run_until_parked();
    assert_eq!(cx.windows().len(), 1);
    assert!(cx.windows().contains(&*window));
    dispatch(&mut visual, &view, "close").unwrap();
    visual.run_until_parked();
    assert!(!cx.windows().contains(&*window));
    assert_eq!(view.state(), ScriptViewState::Disposed);
}

#[gpui::test]
fn native_close_confirmation_and_forced_close_share_the_owner(cx: &mut TestAppContext) {
    let (window, view, _) = mount(cx, SOURCE, true);
    let mut visual = VisualTestContext::from_window(*window, cx);
    assert!(!visual.simulate_close());
    visual.run_until_parked();
    let status = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.role == "status")
            .unwrap()
            .name
            .clone()
    });
    assert_eq!(status, "requested");
    assert!(cx.windows().contains(&*window));
    dispatch(&mut visual, &view, "close").unwrap();
    visual.run_until_parked();
    assert!(!cx.windows().contains(&*window));
}

#[gpui::test]
fn ordinary_embedding_stays_disabled_beside_a_command_owner(cx: &mut TestAppContext) {
    let (window, owner, host) = mount(cx, SOURCE, true);
    let mut visual = VisualTestContext::from_window(*window, cx);
    let ordinary = visual.update(|window, cx| {
        prepared(&SOURCE.replace(
            "fn init(ctx){ctx.set_close_handler(Fn(\"requested\"));}",
            "fn init(ctx){}",
        ))
        .mount(ScriptViewConfig::new("ordinary"), host.clone(), window, cx)
        .unwrap()
    });
    window
        .update(cx, |root, _, cx| {
            root.views.push(ordinary.clone());
            cx.notify();
        })
        .unwrap();
    visual.run_until_parked();
    let error = dispatch(&mut visual, &ordinary, "close")
        .unwrap_err()
        .to_string();
    assert!(error.contains("unavailable for embedded view"), "{error}");
    assert!(cx.windows().contains(&*window));
    dispatch(&mut visual, &owner, "close").unwrap();
    visual.run_until_parked();
    assert_eq!(ordinary.state(), ScriptViewState::Disposed);
    assert_eq!(owner.state(), ScriptViewState::Disposed);
}

#[gpui::test]
fn owner_conflicts_fail_without_revoking_the_existing_owner_and_dispose_allows_replacement(
    cx: &mut TestAppContext,
) {
    let (window, owner, host) = mount(cx, SOURCE, true);
    let mut visual = VisualTestContext::from_window(*window, cx);
    let error = visual.update(|window, cx| {
        prepared(SOURCE)
            .mount_window(ScriptViewConfig::new("second"), host.clone(), window, cx)
            .err()
            .unwrap()
    });
    assert!(matches!(error, ScriptViewError::WindowCommandOwner { .. }));
    let error = visual.update(|window, cx| {
        let other_host = ScriptViewHost::new("alias", cx).unwrap();
        prepared(SOURCE)
            .mount_window(ScriptViewConfig::new("alias-owner"), other_host, window, cx)
            .err()
            .unwrap()
    });
    assert!(
        matches!(error, ScriptViewError::WindowCommandOwner { .. }),
        "another Host must not bypass native-window ownership"
    );
    dispatch(&mut visual, &owner, "focus").unwrap();
    visual.update(|_, cx| owner.dispose(cx).unwrap());
    assert!(
        cx.windows().contains(&*window),
        "dispose must not close Rust's window"
    );
    let replacement = visual.update(|window, cx| {
        prepared(SOURCE)
            .mount_window(ScriptViewConfig::new("owner"), host.clone(), window, cx)
            .unwrap()
    });
    window
        .update(cx, |root, _, cx| {
            root.views = vec![replacement.clone()];
            cx.notify();
        })
        .unwrap();
    visual.run_until_parked();
    dispatch(&mut visual, &replacement, "close").unwrap();
    visual.run_until_parked();
    assert_eq!(replacement.state(), ScriptViewState::Disposed);
}

#[gpui::test]
fn failed_mount_releases_the_command_claim_and_a_host_cannot_bind_another_window(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_rhai::install);
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("rollback", cx).unwrap();
        assert!(
            prepared("fn init(ctx){throw \"failed\";}fn view(ctx){text(\"bad\")}")
                .mount_window(ScriptViewConfig::new("failed"), host.clone(), window, cx)
                .is_err()
        );
        let view = prepared(SOURCE)
            .mount_window(ScriptViewConfig::new("good"), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some((view.clone(), host.clone()));
        Host {
            host,
            views: vec![view],
        }
    });
    cx.run_until_parked();
    let (view, host) = captured.borrow().as_ref().unwrap().clone();
    let other = cx.add_window(move |window, cx| {
        let error = prepared("fn view(ctx){text(\"other\")}")
            .mount(ScriptViewConfig::new("other"), host.clone(), window, cx)
            .err()
            .unwrap();
        assert!(matches!(error, ScriptViewError::WrongNativeWindow(_)));
        Empty
    });
    other
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    dispatch(&mut visual, &view, "focus").unwrap();
    // Rust can remove its window without invoking the script confirmation.
    visual.update(|window, _| window.remove_window());
    visual.run_until_parked();
    assert_eq!(
        view.state(),
        ScriptViewState::Disposed,
        "external handles must not keep a closed window's view active"
    );
}

struct Empty;
impl Render for Empty {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
    }
}
