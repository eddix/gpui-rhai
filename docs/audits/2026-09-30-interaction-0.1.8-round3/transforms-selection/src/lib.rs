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
    for name in ["selection_area", "rotatable", "pan_zoom", "dialog"] {
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

const PAN_SCRIPT: &str = r#"
import "components/pan_zoom" as pan;
fn state_schema(){#{fields:#{commits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn changed(ctx,value){ctx.set_state("commits",ctx.get_state("commits")+1);}
fn panel(key,label,callback){pan::PanZoom(#{key:key,label:label,transform:#{x:0.0,y:0.0,scale:1.0},wheel_zoom:"modifier",content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,30.0,30.0,theme_color("accent"))])),on_transform_change:callback}).with_style(style().width(px(300)).height(px(180)))}
fn view(ctx){column([text(`${ctx.get_state("commits")}`).accessibility_role("status"),panel("first","First",Fn("changed")),panel("second","Second",())])}
"#;
#[gpui::test]
fn round3_escape_cancels_pointer_pan(cx: &mut TestAppContext) {
    let (w, view) = mount(cx, PAN_SCRIPT, "pointer-pan-escape");
    let mut v = VisualTestContext::from_window(*w, cx);
    let b = bounds(&mut v, &view, "region", "First");
    let p = |x, y| point(px((b.x + x) as f32), px((b.y + y) as f32));
    v.simulate_mouse_down(p(100., 100.), MouseButton::Left, Modifiers::default());
    v.simulate_mouse_move(p(140., 120.), MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    assert!(signal_value(&mut v, &view, SignalProperty::TranslateX) > 0.0);
    v.simulate_keystrokes("escape");
    v.run_until_parked();
    let after_escape = signal_value(&mut v, &view, SignalProperty::TranslateX);
    v.simulate_mouse_move(p(170., 120.), MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    let after_move = signal_value(&mut v, &view, SignalProperty::TranslateX);
    v.simulate_mouse_up(p(170., 120.), MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    let commits = status(&mut v, &view);
    println!(
        "pointer pan: after Escape tx={after_escape}, after extra move tx={after_move}, commits={commits}"
    );
    assert_eq!(commits, "0");
    assert_eq!(after_move, 0.0);
}
#[gpui::test]
fn round3_wheel_owner_cancels_when_other_control_has_focus(cx: &mut TestAppContext) {
    let (w, view) = mount(cx, PAN_SCRIPT, "wheel-other-focus");
    let mut v = VisualTestContext::from_window(*w, cx);
    let first = bounds(&mut v, &view, "region", "First");
    let second = bounds(&mut v, &view, "region", "Second");
    let p = point(px((first.x + 100.) as f32), px((first.y + 100.) as f32));
    let focus = point(px((second.x + 100.) as f32), px((second.y + 100.) as f32));
    v.simulate_mouse_down(focus, MouseButton::Left, Modifiers::default());
    v.simulate_mouse_up(focus, MouseButton::Left, Modifiers::default());
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
    assert!(zoomed > 1.0);
    v.simulate_keystrokes("escape");
    v.run_until_parked();
    let after = signal_value(&mut v, &view, SignalProperty::ScaleX);
    v.simulate_event(ScrollWheelEvent {
        position: p,
        delta: ScrollDelta::Pixels(point(px(0.), px(0.))),
        touch_phase: gpui::TouchPhase::Ended,
        ..Default::default()
    });
    v.run_until_parked();
    let commits = status(&mut v, &view);
    println!("wheel other focus: zoomed={zoomed}, after Escape={after}, commits={commits}");
    assert_eq!(after, 1.0);
    assert_eq!(commits, "0");
}
#[gpui::test]
fn round3_idle_panzoom_does_not_swallow_dialog_escape(cx: &mut TestAppContext) {
    let (w, view) = mount(
        cx,
        r#"
import "components/pan_zoom" as pan;import "components/dialog" as dialog;
fn state_schema(){#{fields:#{open:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}}}}}
fn changed(ctx,value){ctx.set_state("open",value);}
fn view(ctx){column([text(`${ctx.get_state("open")}`).accessibility_role("status"),dialog::Dialog(#{key:"d",title:"Canvas editor",open:ctx.get_state("open"),on_open_change:Fn("changed"),content:pan::PanZoom(#{key:"p",label:"Editor viewport",transform:#{x:0.0,y:0.0,scale:1.0},content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,30.0,30.0,theme_color("accent"))]))}).with_style(style().width(px(300)).height(px(180)))})])}
"#,
        "idle-dialog-escape",
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    let b = bounds(&mut v, &view, "region", "Editor viewport");
    let p = point(px((b.x + 100.) as f32), px((b.y + 100.) as f32));
    v.simulate_mouse_down(p, MouseButton::Left, Modifiers::default());
    v.simulate_mouse_up(p, MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    v.simulate_keystrokes("escape");
    v.run_until_parked();
    let actual = status(&mut v, &view);
    println!("idle PanZoom dialog open={actual}");
    assert_eq!(actual, "false");
}
#[gpui::test]
fn round3_pivot_uses_canvas_content_part_size(cx: &mut TestAppContext) {
    let (w, view) = mount(
        cx,
        r#"
import "components/rotatable" as rot;
fn view(ctx){rot::Rotatable(#{key:"r",label:"Rotation",angle:90.0,pivot:#{x:0.0,y:0.0},part_styles:#{content:style().width(px(200)).height(px(100))},content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,30.0,30.0,theme_color("accent"))])).accessibility_role("image").accessibility_label("Canvas")}).with_style(style().width(px(300)).height(px(220)))}
"#,
        "part-content-pivot",
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    let image = bounds(&mut v, &view, "image", "Canvas");
    let tx = signal_value(&mut v, &view, SignalProperty::TranslateX);
    let ty = signal_value(&mut v, &view, SignalProperty::TranslateY);
    println!("Canvas part size: bounds={image:?}, tx={tx},ty={ty}");
    assert_eq!(
        image.width, 200.0,
        "probe must actually change content size"
    );
    assert!((tx + (image.width + image.height) / 2.0).abs() < 0.01);
    assert!((ty - (image.width - image.height) / 2.0).abs() < 0.01);
}

#[gpui::test]
fn round3_resize_during_rotation_preserves_live_pivot(cx: &mut TestAppContext) {
    let (w, view) = mount(
        cx,
        r#"
import "components/rotatable" as rot;
fn view(ctx){rot::Rotatable(#{key:"r",label:"Rotation",angle:0.0,pivot:#{x:0.0,y:0.0},content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,30.0,30.0,theme_color("accent"))])).accessibility_role("image").accessibility_label("Canvas")}).with_style(style().width(relative(1.0)).height(px(220)))}
"#,
        "resize-active-rotation",
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    let before = bounds(&mut v, &view, "image", "Canvas");
    let p = |x, y| point(px((before.x + x) as f32), px((before.y + y) as f32));
    v.simulate_mouse_down(p(100., 10.), MouseButton::Left, Modifiers::default());
    v.simulate_mouse_move(p(100., 30.), MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    cx.simulate_window_resize((*w).into(), gpui::size(px(600.0), px(480.0)));
    cx.run_until_parked();
    cx.refresh().unwrap();
    v.run_until_parked();
    v.simulate_mouse_move(p(10., 100.), MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    let after = bounds(&mut v, &view, "image", "Canvas");
    let angle = signal_value(&mut v, &view, SignalProperty::Rotate);
    let tx = signal_value(&mut v, &view, SignalProperty::TranslateX);
    let ty = signal_value(&mut v, &view, SignalProperty::TranslateY);
    let rotation = Affine2D::rotation_degrees(angle)
        .unwrap()
        .around((after.width / 2.0, after.height / 2.0))
        .unwrap();
    let transformed_origin = rotation.map_point((0.0, 0.0));
    println!(
        "resize while rotate: old width={}, current width={}, angle={}, tx={}, ty={}, projected pivot delta=({}, {})",
        before.width,
        after.width,
        angle,
        tx,
        ty,
        transformed_origin.0 + tx,
        transformed_origin.1 + ty
    );
    v.simulate_mouse_up(p(10., 100.), MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    assert!(
        (transformed_origin.0 + tx).abs() < 1.0 && (transformed_origin.1 + ty).abs() < 1.0,
        "live preview uses stale gesture viewport after resize"
    );
}

#[gpui::test]
fn round3_enclose_remains_valid_at_supported_large_zoom(cx: &mut TestAppContext) {
    let script=select_script(true)
 .replace("x:20.0,y:20.0,width:50.0,height:40.0","x:129.0,y:59.0,width:0.000005,height:0.000005")
 .replace("canvas_rect(\"a\",20.0,20.0,50.0,40.0","canvas_rect(\"a\",129.0,59.0,0.000005,0.000005")
 .replace("multiple:true","multiple:true,marquee:\"enclose\"")
 .replace("theme_color(\"accent\"))])),on_selection_change","theme_color(\"accent\"))])).accessibility_role(\"image\").accessibility_label(\"Zoomed Canvas\").motion(motion_transition(\"scale_x\",1000000.0,1000000.0,#{duration_ms:1,intent:\"essential\"})).motion(motion_transition(\"scale_y\",1000000.0,1000000.0,#{duration_ms:1,intent:\"essential\"})),on_selection_change");
    let (w, view) = mount(cx, &script, "zoom-enclose");
    let mut v = VisualTestContext::from_window(*w, cx);
    let canvas = bounds(&mut v, &view, "image", "Zoomed Canvas");
    let transform = Affine2D::scale(1000000.0, 1000000.0)
        .unwrap()
        .around((canvas.width / 2.0, canvas.height / 2.0))
        .unwrap();
    let target =
        transform.transform_bounds(GeometryBounds::new(129.0, 59.0, 0.000005, 0.000005).unwrap());
    let start = point(
        px((canvas.x + target.x - 7.5) as f32),
        px((canvas.y + target.y - 7.5) as f32),
    );
    let end = point(
        px((canvas.x + target.x + target.width + 7.5) as f32),
        px((canvas.y + target.y + target.height + 7.5) as f32),
    );
    v.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    v.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    v.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    let actual = status(&mut v, &view);
    println!("enclose at zoom1e6: painted target={target:?}, actual selected={actual}");
    assert_eq!(actual, "[\"a\"]");
}
