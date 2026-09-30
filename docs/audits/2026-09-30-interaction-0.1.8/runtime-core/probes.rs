use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use gpui::{Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, WindowHandle, div, point, px};
use gpui_rhai::*;

struct Host { host: ScriptViewHost, views: Vec<ScriptViewHandle> }
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut content = div().size_full().flex();
        for view in &self.views {
            content = content.child(div().w(px(240.0)).h(px(300.0)).child(view.element().unwrap()));
        }
        self.host.container(content)
    }
}
fn mount(cx: &mut TestAppContext, scripts: &[&str]) -> (WindowHandle<Host>, Vec<ScriptViewHandle>) {
    cx.update(gpui_rhai::install);
    let prepared: Vec<_> = scripts.iter().map(|script| {
        let entry = ModuleId::parse("main").unwrap();
        EmbeddedScriptView::new(entry.clone(), EmbeddedScriptSource::new(BTreeMap::from([
            (entry, (*script).to_owned()),
        ])), include_str!(concat!(env!("GPUI_RHAI_AUDIT_ROOT"), "/registry/themes/default_dark.rhai")))
            .prepare().unwrap()
    }).collect();
    let capture = Rc::new(RefCell::new(Vec::new()));
    let capture2 = capture.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("runtime-core-probe", cx).unwrap();
        let views = prepared.into_iter().enumerate().map(|(index, view)|
            view.mount(ScriptViewConfig::new(format!("view-{index}")), host.clone(), window, cx).unwrap()
        ).collect::<Vec<_>>();
        *capture2.borrow_mut() = views.clone();
        Host { host, views }
    });
    cx.run_until_parked(); cx.refresh().unwrap(); cx.run_until_parked();
    let views = capture.borrow().clone();
    (window, views)
}
fn text_values(node: &UiNode, output: &mut Vec<String>) {
    match node.kind() {
        UiNodeKind::Text { text } => output.push(text.to_string()),
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } =>
            children.iter().for_each(|child| text_values(child, output)),
        _ => {}
    }
}
fn texts(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<String> {
    visual.run_until_parked();
    visual.update(|_, cx| {
        assert!(view.last_error(cx).unwrap().is_none());
        let mut values = vec![];
        text_values(&view.root(cx).unwrap().unwrap(), &mut values);
        values
    })
}
const CAPTURE_SCRIPT: &str = r#"
fn state_schema() { #{fields:#{ status:#{schema:#{type:"string"},"default":#{type:"string",value:"idle"}} }} }
fn down(ctx, event) { ctx.set_state("status", "down"); event_response().capture_pointer().stop() }
fn moved(ctx, event) { if event.captured { ctx.set_state("status", "captured-move"); } event_response().stop() }
fn up(ctx, event) { if event.captured { ctx.set_state("status", "captured-up"); } event_response().release_pointer().stop() }
fn view(ctx) { box([text(ctx.get_state("status"))]).with_key("capture-box")
    .with_style(style().width(px(80)).height(px(80)))
    .on("pointer_down", Fn("down")).on("pointer_move", Fn("moved")).on("pointer_up", Fn("up")) }
"#;

#[gpui::test]
fn capture_single_view_control(cx: &mut TestAppContext) {
    let (window, views) = mount(cx, &[CAPTURE_SCRIPT]);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_mouse_down(point(px(20.0), px(20.0)), MouseButton::Left, Modifiers::default());
    assert_eq!(texts(&mut visual, &views[0]), ["down"]);
    visual.simulate_mouse_move(point(px(180.0), px(20.0)), MouseButton::Left, Modifiers::default());
    println!("single-view move: {:?}", texts(&mut visual, &views[0]));
    assert_eq!(texts(&mut visual, &views[0]), ["captured-move"]);
    visual.simulate_mouse_up(point(px(180.0), px(20.0)), MouseButton::Left, Modifiers::default());
    assert_eq!(texts(&mut visual, &views[0]), ["captured-up"]);
}

#[gpui::test]
fn capture_first_view_survives_second_view_paint(cx: &mut TestAppContext) {
    let (window, views) = mount(cx, &[CAPTURE_SCRIPT, "fn view(ctx) { text(\"passive second view\") }"]);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_mouse_down(point(px(20.0), px(20.0)), MouseButton::Left, Modifiers::default());
    assert_eq!(texts(&mut visual, &views[0]), ["down"]);
    visual.simulate_mouse_move(point(px(180.0), px(20.0)), MouseButton::Left, Modifiers::default());
    let actual = texts(&mut visual, &views[0]);
    println!("two-view move: {actual:?}");
    visual.simulate_mouse_up(point(px(180.0), px(20.0)), MouseButton::Left, Modifiers::default());
    println!("two-view up: {:?}", texts(&mut visual, &views[0]));
    assert_eq!(actual, ["captured-move"], "capture must route to the owning first view");
}

const DRAG_SCRIPT: &str = r#"
fn state_schema() { #{fields:#{
    visible:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}},
    status:#{schema:#{type:"string"},"default":#{type:"string",value:"idle"}}
}} }
fn hide(ctx, event) { ctx.set_state("visible", false); }
fn dropped(ctx, event) { ctx.set_state("status", `dropped:${event.source_id}`); }
fn ended(ctx, event) { }
fn view(ctx) {
    let source = if ctx.get_state("visible") {
        gpui_rhai::DragSourcePrimitive(#{key:"source",source_id:"deleted-resource",payload_type:"item",
            payload:#{id:7},operation:"move",threshold:4.0,on_drag_end:Fn("ended")})
            .with_key("source").with_style(style().width(px(80)).height(px(80)))
    } else { text("source removed").with_style(style().width(px(80)).height(px(80))) };
    column([
        source,
        gpui_rhai::DropZonePrimitive(#{key:"target",target_id:"target",payload_types:["item"],
            operations:["move"],on_drop:Fn("dropped")}).with_key("target")
            .with_style(style().width(px(80)).height(px(80))),
        text("Hide source").test_id("hide").on("click",Fn("hide")),
        text(ctx.get_state("status"))
    ])
}
"#;

#[gpui::test]
fn application_drag_live_source_control(cx: &mut TestAppContext) {
    let (window, views) = mount(cx, &[DRAG_SCRIPT]);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_mouse_down(point(px(20.0), px(20.0)), MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(point(px(20.0), px(110.0)), MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    visual.simulate_mouse_up(point(px(20.0), px(110.0)), MouseButton::Left, Modifiers::default());
    let actual = texts(&mut visual, &views[0]);
    println!("drop from live source: {actual:?}");
    assert!(actual.contains(&"dropped:deleted-resource".to_owned()));
}

#[gpui::test]
fn application_drag_must_cancel_when_source_unmounts(cx: &mut TestAppContext) {
    let (window, views) = mount(cx, &[DRAG_SCRIPT]);
    let view = &views[0];
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_mouse_down(point(px(20.0), px(20.0)), MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(point(px(20.0), px(110.0)), MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    visual.update(|window,cx| view.automate(AutomationCommand::Dispatch {
        locator: AutomationLocator::TestId{id:"hide".to_owned()},event:"click".to_owned(),payload:None,
    },window,cx)).unwrap();
    let removed = texts(&mut visual,view);
    assert!(removed.contains(&"source removed".to_owned()), "{removed:?}");
    visual.simulate_mouse_up(point(px(20.0), px(110.0)), MouseButton::Left, Modifiers::default());
    let actual = texts(&mut visual,view);
    println!("drop after source unmount: {actual:?}");
    assert!(!actual.iter().any(|value|value.starts_with("dropped:")),
        "unmounted drag source must not submit stale payload to a surviving target");
}
