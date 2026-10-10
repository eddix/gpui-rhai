use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, WindowHandle, point, px,
};
use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host
            .container(if self.view.state() == ScriptViewState::Suspended {
                gpui::div().into_any_element()
            } else {
                self.view.element().unwrap()
            })
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
            ModuleId::parse(format!("components/{name}")).unwrap(),
            std::fs::read_to_string(format!("{ROOT}/registry/components/{name}.rhai")).unwrap(),
        );
    }
    let prepared = EmbeddedScriptView::new(
        entry,
        EmbeddedScriptSource::new(sources),
        std::fs::read_to_string(format!("{ROOT}/registry/themes/default_dark.rhai")).unwrap(),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
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

fn low_scale_select(cx: &mut TestAppContext, scale: f64, name: &str) {
    let world = 1.0 / scale;
    let span = 5.0 / scale;
    let script=select_script(true)
 .replace("x:20.0,y:20.0,width:50.0,height:40.0",&format!("x:{world:.1},y:{world:.1},width:{span:.1},height:{span:.1}"))
 .replace("canvas_rect(\"a\",20.0,20.0,50.0,40.0",&format!("canvas_rect(\"a\",{world:.1},{world:.1},{span:.1},{span:.1}"))
 .replace("theme_color(\"accent\"))])),on_selection_change",&format!("theme_color(\"accent\"))])).accessibility_role(\"image\").accessibility_label(\"Canvas\").motion(motion_transition(\"scale_x\",{scale:.12},{scale:.12},#{{duration_ms:1,intent:\"essential\"}})).motion(motion_transition(\"scale_y\",{scale:.12},{scale:.12},#{{duration_ms:1,intent:\"essential\"}})),on_selection_change"));
    let (w, view) = mount(cx, &script, name);
    let mut v = VisualTestContext::from_window(*w, cx);
    let canvas = bounds(&mut v, &view, "image", "Canvas");
    let transform = Affine2D::scale(scale, scale)
        .unwrap()
        .around((canvas.width / 2., canvas.height / 2.))
        .unwrap();
    let target = transform.transform_bounds(GeometryBounds::new(world, world, span, span).unwrap());
    let center = point(
        px((canvas.x + target.x + target.width / 2.) as f32),
        px((canvas.y + target.y + target.height / 2.) as f32),
    );
    v.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    v.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    let actual = status(&mut v, &view);
    println!(
        "low zoom={scale}, visible={target:?}, inverse available={}, selected={actual}",
        transform.inverse().is_some()
    );
    assert_eq!(actual, "[\"a\"]");
}
#[gpui::test]
fn small_invertible_zoom_selection(cx: &mut TestAppContext) {
    low_scale_select(cx, 1e-9, "small-zoom");
}
#[gpui::test]
fn larger_invertible_zoom_control(cx: &mut TestAppContext) {
    low_scale_select(cx, 1e-7, "larger-zoom-control");
}
#[gpui::test]
fn asymmetric_insets_negative_scale_rotation_click_control(cx: &mut TestAppContext) {
    let script=select_script(true)
 .replace("x:20.0,y:20.0,width:50.0,height:40.0","x:100.0,y:40.0,width:10.0,height:10.0")
 .replace("canvas_rect(\"a\",20.0,20.0,50.0,40.0","canvas_rect(\"a\",100.0,40.0,10.0,10.0")
 .replace("x:100.0,y:20.0,width:50.0,height:40.0","x:200.0,y:80.0,width:20.0,height:20.0").replace("canvas_rect(\"b\",100.0,20.0,50.0,40.0","canvas_rect(\"b\",200.0,80.0,20.0,20.0").replace("multiple:true","multiple:true,part_styles:#{content:style().padding_left(px(17)).padding_right(px(3)).padding_top(px(13)).padding_bottom(px(5)).border_left(px(2)).border_right(px(4)).border_top(px(3)).border_bottom(px(1))}")
 .replace("theme_color(\"accent\"))])),on_selection_change","theme_color(\"accent\"))])).accessibility_role(\"image\").accessibility_label(\"Canvas\").motion(motion_transition(\"scale_x\",-1.5,-1.5,#{duration_ms:1,intent:\"essential\"})).motion(motion_transition(\"scale_y\",0.75,0.75,#{duration_ms:1,intent:\"essential\"})).motion(motion_transition(\"rotate\",30.0,30.0,#{duration_ms:1,intent:\"essential\"})),on_selection_change");
    let script = script
        .replace(
            "x:100.0,y:20.0,width:50.0,height:40.0",
            "x:200.0,y:80.0,width:20.0,height:20.0",
        )
        .replace(
            "canvas_rect(\"b\",100.0,20.0,50.0,40.0",
            "canvas_rect(\"b\",200.0,80.0,20.0,20.0",
        );
    let (w, view) = mount(cx, &script, "asymmetric-selection");
    let mut v = VisualTestContext::from_window(*w, cx);
    v.simulate_scale_factor_change(1.25);
    v.run_until_parked();
    cx.refresh().unwrap();
    let outer = bounds(&mut v, &view, "image", "Canvas");
    let transform = Affine2D::scale(-1.5, 0.75)
        .unwrap()
        .then(Affine2D::rotation_degrees(30.).unwrap())
        .unwrap()
        .around(((outer.width - 26.) / 2., (outer.height - 22.) / 2.))
        .unwrap();
    let local = transform.map_point((105., 45.));
    let target = point(
        px((outer.x + 19. + local.0) as f32),
        px((outer.y + 16. + local.1) as f32),
    );
    v.simulate_mouse_down(target, MouseButton::Left, Modifiers::default());
    v.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
    v.run_until_parked();
    let actual = status(&mut v, &view);
    println!("asymmetric/negative/rotate at DPI1.25 click={target:?}, selected={actual}");
    assert_eq!(actual, "[\"a\"]");
}
#[gpui::test]
fn rotation_suspend_resume_resize_cycles_control(cx: &mut TestAppContext) {
    let (w, view) = mount(
        cx,
        r#"
import "components/rotatable" as rot;
fn state_schema(){#{fields:#{commits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn changed(ctx,value){ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){column([text(`${ctx.get_state("commits")}`).accessibility_role("status"),rot::Rotatable(#{key:"r",label:"Rotation",angle:20.0,pivot:#{x:100.0,y:50.0},part_styles:#{content:style().padding_left(px(17)).padding_top(px(13)).border(px(2))},content:canvas(canvas_scene([canvas_rect("a",95.0,45.0,10.0,10.0,theme_color("accent"))])),on_rotate:Fn("changed")}).with_style(style().width(relative(1.0)).height(px(220)))])}
"#,
        "cycle-rotation",
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    for width in [600., 900.] {
        let area = bounds(&mut v, &view, "slider", "Rotation");
        let p = |x, y| point(px((area.x + x) as f32), px((area.y + y) as f32));
        v.simulate_mouse_down(p(160., 60.), MouseButton::Left, Modifiers::default());
        v.simulate_mouse_move(p(140., 100.), MouseButton::Left, Modifiers::default());
        v.run_until_parked();
        let preview = signal_value(&mut v, &view, SignalProperty::Rotate);
        assert!((preview - 20.).abs() > 1., "must start actual preview");
        let angle_signal = v.update(|_, cx| {
            find_signal(&view.root(cx).unwrap().unwrap(), SignalProperty::Rotate).unwrap()
        });
        v.update(|window, cx| view.suspend(window, cx).unwrap());
        v.run_until_parked();
        let restored = v.update(|_, cx| match view.read_signal(&angle_signal, cx).unwrap() {
            SignalValue::Float(x) => x,
            _ => panic!("wrong type"),
        });
        assert!((restored - 20.).abs() < 0.001);
        v.update(|_, cx| view.resume(cx).unwrap());
        cx.run_until_parked();
        cx.simulate_window_resize(*w, gpui::size(px(width), px(500.)));
        cx.refresh().unwrap();
        v.run_until_parked();
        v.simulate_mouse_up(p(140., 100.), MouseButton::Left, Modifiers::default());
        v.run_until_parked();
        println!(
            "suspend/resume to width={width}: preview={preview}, source restored={restored}, commits={}",
            status(&mut v, &view)
        );
        assert_eq!(status(&mut v, &view), "0");
    }
}
