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

#[gpui::test]
fn round4_selection_hit_matches_padded_canvas_paint(cx: &mut TestAppContext) {
    let script = select_script(true)
        .replace(
            "multiple:true",
            "multiple:true,part_styles:#{content:style().padding(px(20)).border(px(10))}",
        )
        .replace("width:50.0,height:40.0", "width:10.0,height:10.0")
        .replace("20.0,20.0,50.0,40.0", "20.0,20.0,10.0,10.0")
        .replace("100.0,20.0,50.0,40.0", "100.0,20.0,10.0,10.0");
    let (w, view) = mount(cx, &script, "padded-hit");
    let mut v = VisualTestContext::from_window(*w, cx);
    let area = bounds(&mut v, &view, "grid", "Selection");
    // render_canvas puts its 100%-sized GPUI Canvas inside the styled node's border+padding.
    let painted_center = point(
        px((area.x + 10. + 20. + 25.) as f32),
        px((area.y + 10. + 20. + 25.) as f32),
    );
    v.simulate_mouse_down(painted_center, MouseButton::Left, Modifiers::default());
    v.simulate_mouse_up(painted_center, MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    let actual = status(&mut v, &view);
    println!(
        "padded Canvas click at painted a center: area={area:?}, click={painted_center:?}, selected={actual}"
    );
    assert_eq!(actual, "[\"a\"]");
}
fn large_canvas_enclose(cx: &mut TestAppContext, scale: f64, name: &str) {
    let span = 5.0 / scale;
    let script=select_script(true)
 .replace("x:20.0,y:20.0,width:50.0,height:40.0",&format!("x:499.0,y:499.0,width:{span},height:{span}"))
 .replace("canvas_rect(\"a\",20.0,20.0,50.0,40.0",&format!("canvas_rect(\"a\",499.0,499.0,{span},{span}"))
 .replace("width(px(260)).height(px(120))","width(px(1000)).height(px(1000))")
 .replace("multiple:true","multiple:true,marquee:\"enclose\"")
 .replace("theme_color(\"accent\"))])),on_selection_change",&format!("theme_color(\"accent\"))])).accessibility_role(\"image\").accessibility_label(\"Canvas\").motion(motion_transition(\"scale_x\",{scale:.1},{scale:.1},#{{duration_ms:1,intent:\"essential\"}})).motion(motion_transition(\"scale_y\",{scale:.1},{scale:.1},#{{duration_ms:1,intent:\"essential\"}})),on_selection_change"));
    let (w, view) = mount(cx, &script, name);
    let mut v = VisualTestContext::from_window(*w, cx);
    let canvas = bounds(&mut v, &view, "image", "Canvas");
    let transform = Affine2D::scale(scale, scale)
        .unwrap()
        .around((canvas.width / 2., canvas.height / 2.))
        .unwrap();
    let target = transform.transform_bounds(GeometryBounds::new(499., 499., span, span).unwrap());
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
    println!("1000px Canvas scale={scale}, visible target={target:?}, selected={actual}");
    assert_eq!(actual, "[\"a\"]");
}
#[gpui::test]
fn round4_large_canvas_high_zoom_enclose(cx: &mut TestAppContext) {
    large_canvas_enclose(cx, 1e6, "large-high-zoom");
}
#[gpui::test]
fn round4_large_canvas_lower_zoom_control(cx: &mut TestAppContext) {
    large_canvas_enclose(cx, 1e4, "large-low-zoom-control");
}
