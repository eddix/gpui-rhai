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

fn signal_value(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    property: SignalProperty,
) -> f64 {
    v.update(|_, cx| {
        let root = view.root(cx).unwrap().unwrap();
        let signal = find_signal(&root, property).unwrap();
        match view.read_signal(&signal, cx).unwrap() {
            SignalValue::Float(value) => value,
            _ => panic!("expected float"),
        }
    })
}
#[gpui::test]
fn round2_initial_pivot_with_separate_handle(cx: &mut TestAppContext) {
    let (w, view) = mount(
        cx,
        r#"
import "components/rotatable" as rot;
fn view(ctx){rot::Rotatable(#{key:"r",label:"Rotation",angle:90.0,pivot:#{x:0.0,y:0.0},handle:text("R"),content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,30.0,30.0,theme_color("accent"))])).accessibility_role("image").accessibility_label("Canvas")}).with_style(style().width(px(300)).height(px(220)))}
"#,
        "handle-pivot",
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    let image = bounds(&mut v, &view, "image", "Canvas");
    let tx = signal_value(&mut v, &view, SignalProperty::TranslateX);
    let ty = signal_value(&mut v, &view, SignalProperty::TranslateY);
    println!("handle pivot: canvas={image:?}, translation=({tx},{ty})");
    assert!(
        tx < -200.0 && ty > 30.0,
        "pivot compensation used wrong surface dimensions"
    );
}
#[gpui::test]
fn round2_resize_recomputes_pivot(cx: &mut TestAppContext) {
    let (w, view) = mount(
        cx,
        r#"
import "components/rotatable" as rot;
fn view(ctx){rot::Rotatable(#{key:"r",label:"Rotation",angle:90.0,pivot:#{x:0.0,y:0.0},content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,30.0,30.0,theme_color("accent"))])).accessibility_role("image").accessibility_label("Canvas")}).with_style(style().width(relative(1.0)).height(px(220)))}
"#,
        "resize-pivot",
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    let before = bounds(&mut v, &view, "image", "Canvas");
    let before_tx = signal_value(&mut v, &view, SignalProperty::TranslateX);
    cx.simulate_window_resize((*w).into(), gpui::size(px(600.0), px(480.0)));
    cx.run_until_parked();
    cx.refresh().unwrap();
    v.run_until_parked();
    let after = bounds(&mut v, &view, "image", "Canvas");
    let after_tx = signal_value(&mut v, &view, SignalProperty::TranslateX);
    println!("resize pivot: before={before:?} tx={before_tx}, after={after:?} tx={after_tx}");
    assert_ne!(
        before.width, after.width,
        "probe must actually resize canvas"
    );
    assert!(
        (after_tx - before_tx + (after.width - before.width) / 2.0).abs() < 2.0,
        "pivot compensation remained stale after resize"
    );
}
#[gpui::test]
fn round2_horizontal_marquee_does_not_select_far_collinear_object(cx: &mut TestAppContext) {
    let script = select_script(true)
        .replace(
            "x:20.0,y:20.0,width:50.0,height:40.0",
            "x:180.0,y:20.0,width:50.0,height:40.0",
        )
        .replace(
            "canvas_rect(\"a\",20.0,20.0,50.0,40.0",
            "canvas_rect(\"a\",180.0,20.0,50.0,40.0",
        );
    let (w, view) = mount(cx, &script, "line-marquee");
    let mut v = VisualTestContext::from_window(*w, cx);
    let b = bounds(&mut v, &view, "grid", "Selection");
    let p = |x, y| point(px((b.x + x) as f32), px((b.y + y) as f32));
    // Interaction and Canvas share the same origin. Stay exactly on the target's top edge.
    v.simulate_mouse_down(p(5., 20.), MouseButton::Left, Modifiers::default());
    v.simulate_mouse_move(p(50., 20.), MouseButton::Left, Modifiers::default());
    v.simulate_mouse_up(p(50., 20.), MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    let actual = status(&mut v, &view);
    println!("horizontal marquee actual={actual}");
    assert_eq!(actual, "[]", "no target intersects short segment x=5..50");
}
#[gpui::test]
fn round2_single_mode_space_respects_selection_invariant(cx: &mut TestAppContext) {
    let script = select_script(false)
        .replace(
            "selected_keys:ctx.get_state(\"selected\")",
            "selected_keys:[\"a\"]",
        )
        .replace("active_key:ctx.get_state(\"active\")", "active_key:\"b\"");
    let (w, view) = mount(cx, &script, "single-space");
    let mut v = VisualTestContext::from_window(*w, cx);
    let b = bounds(&mut v, &view, "grid", "Selection");
    let blank = point(px((b.x + 220.0) as f32), px((b.y + 90.0) as f32));
    v.simulate_mouse_down(blank, MouseButton::Left, Modifiers::default());
    v.simulate_mouse_up(blank, MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    v.simulate_keystrokes("space");
    v.run_until_parked();
    let actual = status(&mut v, &view);
    println!("single-mode Space actual={actual}");
    assert!(!actual.contains(','));
}

#[gpui::test]
fn round2_escape_cancels_active_wheel_preview(cx: &mut TestAppContext) {
    let (w, view) = mount(
        cx,
        r#"
import "components/pan_zoom" as pan;
fn state_schema(){#{fields:#{commits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn changed(ctx,value){ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){column([text(`${ctx.get_state("commits")}`).accessibility_role("status"),pan::PanZoom(#{key:"p",label:"Viewport",transform:#{x:0.0,y:0.0,scale:1.0},wheel_zoom:"modifier",content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,30.0,30.0,theme_color("accent"))])),on_transform_change:Fn("changed")}).with_style(style().width(px(300)).height(px(220)))])}
"#,
        "wheel-escape",
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    let b = bounds(&mut v, &view, "region", "Viewport");
    let p = point(px((b.x + 100.) as f32), px((b.y + 100.) as f32));
    v.simulate_mouse_down(p, MouseButton::Left, Modifiers::default());
    v.simulate_mouse_up(p, MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    v.simulate_event(ScrollWheelEvent {
        position: p,
        delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
        touch_phase: gpui::TouchPhase::Started,
        modifiers: Modifiers {
            control: true,
            ..Default::default()
        },
        ..Default::default()
    });
    v.run_until_parked();
    let zoomed = signal_value(&mut v, &view, SignalProperty::ScaleX);
    assert!(zoomed > 1.0, "probe must start zoom");
    v.simulate_keystrokes("escape");
    v.run_until_parked();
    let after = signal_value(&mut v, &view, SignalProperty::ScaleX);
    println!("wheel Escape: zoomed={zoomed}, after Escape={after}, expected=1.0");
    v.simulate_event(ScrollWheelEvent {
        position: p,
        delta: ScrollDelta::Pixels(point(px(0.), px(0.))),
        touch_phase: gpui::TouchPhase::Ended,
        ..Default::default()
    });
    v.run_until_parked();
    println!(
        "wheel after Escape then Ended commits={}",
        status(&mut v, &view)
    );
    assert_eq!(after, 1.0, "Escape did not cancel owned wheel preview");
}

#[gpui::test]
fn round2_negative_nonuniform_rotation_enclose_control(cx: &mut TestAppContext) {
    let script = select_script(true)
        .replace("x:20.0,y:20.0,width:50.0,height:40.0", "x:100.0,y:50.0,width:20.0,height:20.0")
        .replace("canvas_rect(\"a\",20.0,20.0,50.0,40.0", "canvas_rect(\"a\",100.0,50.0,20.0,20.0")
        .replace("x:100.0,y:20.0,width:50.0,height:40.0", "x:200.0,y:80.0,width:20.0,height:20.0")
        .replace("canvas_rect(\"b\",100.0,20.0,50.0,40.0", "canvas_rect(\"b\",200.0,80.0,20.0,20.0")
        .replace("multiple:true", "multiple:true,marquee:\"enclose\"")
        .replace("theme_color(\"accent\"))])),on_selection_change", "theme_color(\"accent\"))])).accessibility_role(\"image\").accessibility_label(\"Objects\").motion(motion_transition(\"scale_x\",1.5,1.5,#{duration_ms:1,intent:\"essential\"})).motion(motion_transition(\"scale_y\",-0.5,-0.5,#{duration_ms:1,intent:\"essential\"})).motion(motion_transition(\"rotate\",30.0,30.0,#{duration_ms:1,intent:\"essential\"})),on_selection_change");
    let (w, view) = mount(cx, &script, "negative-enclose");
    let mut v = VisualTestContext::from_window(*w, cx);
    let canvas = bounds(&mut v, &view, "image", "Objects");
    let transform = Affine2D::scale(1.5, -0.5)
        .unwrap()
        .then(Affine2D::rotation_degrees(30.0).unwrap())
        .unwrap()
        .around((canvas.width / 2.0, canvas.height / 2.0))
        .unwrap();
    let target = transform.transform_bounds(GeometryBounds::new(100.0, 50.0, 20.0, 20.0).unwrap());
    let start = point(
        px((canvas.x + target.x - 2.0) as f32),
        px((canvas.y + target.y - 2.0) as f32),
    );
    let end = point(
        px((canvas.x + target.x + target.width + 2.0) as f32),
        px((canvas.y + target.y + target.height + 2.0) as f32),
    );
    v.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    v.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    v.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    let actual = status(&mut v, &view);
    println!("negative/nonuniform/rotation enclose control={actual}");
    assert_eq!(actual, "[\"a\"]");
}
