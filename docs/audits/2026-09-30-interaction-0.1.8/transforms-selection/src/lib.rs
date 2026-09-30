#![allow(unused_imports)]
use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, ScrollDelta, ScrollWheelEvent,
    TestAppContext, VisualTestContext, Window, WindowHandle, point, px,
};
use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../..");
struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}
fn mount(
    cx: &mut TestAppContext,
    script: &str,
    name: &str,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let mut sources = BTreeMap::from([(entry.clone(), script.to_owned())]);
    for name in ["selection_area", "rotatable", "pan_zoom"] {
        sources.insert(
            ModuleId::parse(&format!("components/{name}")).unwrap(),
            std::fs::read_to_string(format!("{ROOT}/registry/components/{name}.rhai")).unwrap(),
        );
    }
    let prepared = EmbeddedScriptView::new(
        entry,
        EmbeddedScriptSource::new(sources),
        std::fs::read_to_string(format!("{ROOT}/registry/themes/default_dark.rhai")).unwrap(),
    )
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let name = name.to_owned();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&name, cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new(&name), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let view = captured.borrow().as_ref().unwrap().clone();
    (window, view)
}
fn find_signal(node: &UiNode, property: SignalProperty) -> Option<NativeSignal> {
    if let Some((_, signal)) = node.signal_bindings().find(|(p, _)| *p == property) {
        return Some(signal.clone());
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(|n| find_signal(n, property))
        }
        _ => None,
    }
}
fn status(v: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|n| n.role == "status")
            .unwrap()
            .name
            .clone()
    })
}
fn bounds(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name(role, name)
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    })
}
fn select_script(multiple: bool) -> String {
    format!(
        r#"
import "components/selection_area" as select;
fn state_schema(){{#{{fields:#{{ selected:#{{schema:#{{type:"array",items:#{{type:"string"}}}},"default":#{{type:"array",value:[]}}}},active:#{{schema:#{{type:"optional",value:#{{type:"string"}}}},"default":#{{type:"null"}}}},anchor:#{{schema:#{{type:"optional",value:#{{type:"string"}}}},"default":#{{type:"null"}}}} }} }} }}
fn change(ctx,value){{ctx.set_state("selected",value.selected_keys);ctx.set_state("active",value.active_key);ctx.set_state("anchor",value.anchor_key);}}
fn view(ctx){{column([text(`${{ctx.get_state("selected")}}`).accessibility_role("status"),select::SelectionArea(#{{key:"sel",label:"Selection",multiple:{multiple},selected_keys:ctx.get_state("selected"),active_key:ctx.get_state("active"),anchor_key:ctx.get_state("anchor"),targets:[#{{key:"a",x:20.0,y:20.0,width:50.0,height:40.0}},#{{key:"b",x:100.0,y:20.0,width:50.0,height:40.0}}],content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,50.0,40.0,theme_color("accent")),canvas_rect("b",100.0,20.0,50.0,40.0,theme_color("accent"))])),on_selection_change:Fn("change")}}).with_style(style().width(px(260)).height(px(120)))])}}
"#
    )
}
#[gpui::test]
fn audit_blank_click_should_clear(cx: &mut TestAppContext) {
    let (w, view) = mount(cx, &select_script(true), "blank");
    let mut v = VisualTestContext::from_window(*w, cx);
    let b = bounds(&mut v, &view, "grid", "Selection");
    let p = |x, y| point(px((b.x + x) as f32), px((b.y + y) as f32));
    for pos in [p(40., 40.), p(220., 90.)] {
        v.simulate_mouse_down(pos, MouseButton::Left, Modifiers::default());
        v.simulate_mouse_up(pos, MouseButton::Left, Modifiers::default());
        v.run_until_parked();
        println!("selection={}", status(&mut v, &view));
    }
    assert_eq!(status(&mut v, &view), "[]");
}
#[gpui::test]
fn audit_single_mode_marquee(cx: &mut TestAppContext) {
    let (w, view) = mount(cx, &select_script(false), "single");
    let mut v = VisualTestContext::from_window(*w, cx);
    let b = bounds(&mut v, &view, "grid", "Selection");
    let p = |x, y| point(px((b.x + x) as f32), px((b.y + y) as f32));
    v.simulate_mouse_down(p(5., 5.), MouseButton::Left, Modifiers::default());
    v.simulate_mouse_move(p(180., 80.), MouseButton::Left, Modifiers::default());
    v.simulate_mouse_up(p(180., 80.), MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    let actual = status(&mut v, &view);
    println!("multiple=false selected={actual}");
    assert!(!actual.contains(','));
}
#[gpui::test]
fn audit_initial_rotation_pivot(cx: &mut TestAppContext) {
    let (w, view) = mount(
        cx,
        r#"
import "components/rotatable" as rot;
fn view(ctx){rot::Rotatable(#{key:"r",label:"Rotation",angle:90.0,pivot:#{x:0.0,y:0.0},content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,30.0,30.0,theme_color("accent"))])).accessibility_role("image").accessibility_label("Canvas")}).with_style(style().width(px(300)).height(px(220)))}
"#,
        "initial-rotation",
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    let b = bounds(&mut v, &view, "image", "Canvas");
    println!("initial rotated canvas visual={b:?}");
    assert!(b.x < -200.0, "pivot translation not applied: {b:?}");
}

#[gpui::test]
fn audit_rotated_canvas_marquee(cx: &mut TestAppContext) {
    let script=select_script(true).replace("x:20.0,y:20.0,width:50.0,height:40.0","x:70.0,y:75.0,width:8.0,height:8.0").replace("canvas_rect(\"a\",20.0,20.0,50.0,40.0","canvas_rect(\"a\",70.0,75.0,8.0,8.0").replace("on_selection_change:Fn(\"change\")","on_selection_change:Fn(\"change\")").replace("theme_color(\"accent\"))])),on_selection_change", "theme_color(\"accent\"))])).motion(motion_transition(\"rotate\",45.0,45.0,#{duration_ms:1,intent:\"essential\"})),on_selection_change");
    let (w, view) = mount(cx, &script, "rotated-selection");
    let mut v = VisualTestContext::from_window(*w, cx);
    let b = bounds(&mut v, &view, "grid", "Selection");
    let p = |x, y| point(px((b.x + x) as f32), px((b.y + y) as f32));
    v.simulate_mouse_down(p(20., 20.), MouseButton::Left, Modifiers::default());
    v.simulate_mouse_move(p(100., 100.), MouseButton::Left, Modifiers::default());
    v.simulate_mouse_up(p(100., 100.), MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    let actual = status(&mut v, &view);
    println!("rotated marquee selected={actual}");
    assert!(actual.contains("a"));
}

#[gpui::test]
fn audit_finite_panzoom_scale_does_not_panic(cx: &mut TestAppContext) {
    let _ = mount(
        cx,
        r#"
import "components/pan_zoom" as pan;
fn view(ctx){pan::PanZoom(#{key:"big",label:"Finite zoom",transform:#{x:0.0,y:0.0,scale:1.0e308},max_scale:1.0e308,content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,30.0,30.0,theme_color("accent"))]))}).with_style(style().width(px(300)).height(px(220)))}
"#,
        "finite-scale",
    );
}

#[gpui::test]
fn audit_cancelled_modifier_wheel_must_not_commit(cx: &mut TestAppContext) {
    let (w, view) = mount(
        cx,
        r#"
import "components/pan_zoom" as pan;
fn state_schema(){#{fields:#{ commits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}} }}}
fn changed(ctx,value){ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){column([text(`${ctx.get_state("commits")}`).accessibility_role("status"),pan::PanZoom(#{key:"p",label:"Viewport",transform:#{x:0.0,y:0.0,scale:1.0},wheel_zoom:"modifier",content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,30.0,30.0,theme_color("accent"))])),on_transform_change:Fn("changed")}).with_style(style().width(px(300)).height(px(220)))])}
"#,
        "wheel-cancel",
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    let b = bounds(&mut v, &view, "region", "Viewport");
    let p = point(px((b.x + 100.) as f32), px((b.y + 100.) as f32));
    for (phase, delta, control) in [
        (gpui::TouchPhase::Started, -40., true),
        (gpui::TouchPhase::Cancelled, 0., false),
        (gpui::TouchPhase::Ended, 0., false),
    ] {
        v.simulate_event(ScrollWheelEvent {
            position: p,
            delta: ScrollDelta::Pixels(point(px(0.), px(delta))),
            touch_phase: phase,
            modifiers: Modifiers {
                control,
                ..Default::default()
            },
            ..Default::default()
        });
        v.run_until_parked();
        if phase == gpui::TouchPhase::Cancelled {
            let scale = v.update(|_, cx| {
                let root = view.root(cx).unwrap().unwrap();
                let signal = find_signal(&root, SignalProperty::ScaleX).unwrap();
                view.read_signal(&signal, cx).unwrap()
            });
            println!("immediately after Cancelled scale={scale:?} (expected Float(1.0))");
        }
    }
    let actual = status(&mut v, &view);
    println!("cancelled wheel commits={actual}");
    assert_eq!(actual, "0");
}
