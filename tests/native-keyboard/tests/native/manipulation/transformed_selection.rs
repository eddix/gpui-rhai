use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Pixels, Point, Render, ScrollDelta, ScrollEvent,
    TestAppContext, VisualTestContext, Window, WindowHandle, point, px,
};
use gpui_rhai::*;
use std::{cell::RefCell, rc::Rc};

use crate::support::{Mount, ReplaceOnce, bounds, center, snapshot};

/// Renders nothing while the view is suspended.
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
    let prepared = Mount::new(script)
        .registry("components/selection_area")
        .registry("components/rotatable")
        .registry("components/pan_zoom")
        .registry("components/dialog")
        .motion(MotionPreference::None)
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
    snapshot(v, view)
        .nodes()
        .find(|n| n.role == "status")
        .unwrap()
        .name
        .clone()
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
 .replace_once("x:20.0,y:20.0,width:50.0,height:40.0",&format!("x:{world:.1},y:{world:.1},width:{span:.1},height:{span:.1}"))
 .replace_once("canvas_rect(\"a\",20.0,20.0,50.0,40.0",&format!("canvas_rect(\"a\",{world:.1},{world:.1},{span:.1},{span:.1}"))
 .replace_once("theme_color(\"accent\"))])),on_selection_change",&format!("theme_color(\"accent\"))])).accessibility_role(\"image\").accessibility_label(\"Canvas\").motion(motion_transition(\"scale_x\",{scale:.12},{scale:.12},#{{duration_ms:1,intent:\"essential\"}})).motion(motion_transition(\"scale_y\",{scale:.12},{scale:.12},#{{duration_ms:1,intent:\"essential\"}})),on_selection_change"));
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
 .replace_once("x:20.0,y:20.0,width:50.0,height:40.0","x:100.0,y:40.0,width:10.0,height:10.0")
 .replace_once("canvas_rect(\"a\",20.0,20.0,50.0,40.0","canvas_rect(\"a\",100.0,40.0,10.0,10.0")
 .replace_once("x:100.0,y:20.0,width:50.0,height:40.0","x:200.0,y:80.0,width:20.0,height:20.0").replace_once("canvas_rect(\"b\",100.0,20.0,50.0,40.0","canvas_rect(\"b\",200.0,80.0,20.0,20.0").replace_once("multiple:true","multiple:true,part_styles:#{content:style().padding_left(px(17)).padding_right(px(3)).padding_top(px(13)).padding_bottom(px(5)).border_left(px(2)).border_right(px(4)).border_top(px(3)).border_bottom(px(1))}")
 .replace_once("theme_color(\"accent\"))])),on_selection_change","theme_color(\"accent\"))])).accessibility_role(\"image\").accessibility_label(\"Canvas\").motion(motion_transition(\"scale_x\",-1.5,-1.5,#{duration_ms:1,intent:\"essential\"})).motion(motion_transition(\"scale_y\",0.75,0.75,#{duration_ms:1,intent:\"essential\"})).motion(motion_transition(\"rotate\",30.0,30.0,#{duration_ms:1,intent:\"essential\"})),on_selection_change");
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

fn relative_point(origin: Point<Pixels>, delta: (f64, f64)) -> Point<Pixels> {
    point(origin.x + px(delta.0 as f32), origin.y + px(delta.1 as f32))
}

#[gpui::test]
fn pan_zoom_uses_the_inverse_plane_for_drag_and_wheel_anchor(cx: &mut TestAppContext) {
    let script = r#"
import "components/pan_zoom" as pan;
fn state_schema(){#{fields:#{
    x:#{schema:#{type:"number"},"default":#{type:"float",value:0.0}},
    y:#{schema:#{type:"number"},"default":#{type:"float",value:0.0}},
    scale:#{schema:#{type:"number"},"default":#{type:"float",value:1.0}},
    commits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
}}}
fn changed(ctx,val){ctx.set_state("x",val.x);ctx.set_state("y",val.y);ctx.set_state("scale",val.scale);
    ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){
    let x=ctx.get_state("x"); let y=ctx.get_state("y"); let s=ctx.get_state("scale");
    let content=pan::PanZoom(#{key:"pan", label:"Mapped viewport",transform:#{x:x,y:y,scale:s},
        wheel_zoom:"always",content:canvas(canvas_scene([canvas_rect("marker",125,70,20,20,rgba(0xfe0000ff))])),
        on_transform_change:Fn("changed")})
        .with_style(style().width(px(300)).height(px(180)).border(px(0))
            .transform(#{scale_x:1.5,scale_y:0.75,rotate:90}));
    column([text(`${x},${y},${s},${ctx.get_state("commits")}`).accessibility_role("status"),content])
        .with_style(style().padding_top(px(100)).padding_left(px(200)))
}
"#;
    let (w, view) = mount(cx, script, "affine-pan");
    let mut visual = VisualTestContext::from_window(*w, cx);
    let b = bounds(&mut visual, &view, "region", "Mapped viewport");
    assert!(b.height > 440. && b.width < 140., "{b:?}");
    let center = center(b);
    // x=96 in the draw plane appears y=144 from the window center. This
    // point is intentionally outside the original unrotated Y bounds.
    let start = relative_point(center, (0., 144.));
    let end = relative_point(start, (-12., -45.));
    let mouse = Modifiers::default();
    visual.simulate_mouse_down(start, MouseButton::Left, mouse);
    visual.simulate_mouse_move(end, MouseButton::Left, mouse);
    visual.run_until_parked();
    let preview_x = signal_value(&mut visual, &view, SignalProperty::TranslateX);
    let preview_y = signal_value(&mut visual, &view, SignalProperty::TranslateY);
    visual.simulate_mouse_up(end, MouseButton::Left, mouse);
    visual.run_until_parked();
    let numeric = |s: String| {
        s.split(',')
            .map(|v| v.parse::<f64>().unwrap())
            .collect::<Vec<_>>()
    };
    let pan = numeric(status(&mut visual, &view));
    assert!(
        (preview_x + 30.).abs() < 0.03 && (preview_y - 16.).abs() < 0.03,
        "preview ({preview_x}, {preview_y}), commit {pan:?}, b={b:?}, s={start:?}, e={end:?}, error={:?}",
        crate::support::last_error(&mut visual, &view)
    );
    let close =
        |got: f64, expected: f64| assert!((got - expected).abs() < 0.03, "{got} != {expected}");
    close(pan[0], -30.);
    close(pan[1], 16.);
    assert_eq!(&pan[2..], &[1., 1.]);

    // A wheel anchor (60,-32) from the drawable center is (24,90)
    // from the center in window space; it must not be interpreted as
    // that raw pixel offset. Factor and fixed-point formula are independent.
    visual.simulate_event(ScrollEvent {
        position: relative_point(center, (24., 90.)),
        delta: ScrollDelta::Lines(point(0.0, -4.0)),
        ..Default::default()
    });
    visual.run_until_parked();
    let zoom = numeric(status(&mut visual, &view));
    let ratio = 0.16_f64.exp();
    close(zoom[0], (1. - ratio) * 60. + ratio * -30.);
    close(zoom[1], (1. - ratio) * -32. + ratio * 16.);
    close(zoom[2], ratio);
    assert_eq!(zoom[3], 2.);
    visual.simulate_event(ScrollEvent {
        position: relative_point(center, (24., 90.)),
        delta: ScrollDelta::Lines(point(0.0, -4.0)),
        ..Default::default()
    });
    visual.run_until_parked();
    let second = numeric(status(&mut visual, &view));
    close(second[0], (1. - ratio) * 60. + ratio * zoom[0]);
    close(second[1], (1. - ratio) * -32. + ratio * zoom[1]);
    close(second[2], ratio * ratio);
    assert_eq!(second[3], 3.);
}

#[gpui::test]
fn marquee_matches_window_rectangle_and_inverts_view_box_before_selecting(cx: &mut TestAppContext) {
    let script = select_script(true)
        .replace_once("theme_color(\"accent\"))])),on_selection_change",
            "theme_color(\"accent\"))]).view_box(0,0,130,60)),on_selection_change")
        .replace_once("height(px(120)))])", "height(px(120)).border(px(0)).transform(#{scale_x:-1.25,scale_y:0.9,rotate:30}))]).with_style(style().padding_top(px(120)).padding_left(px(120)))");
    let (w, view) = mount(cx, &script, "affine-selection");
    let mut visual = VisualTestContext::from_window(*w, cx);
    let window_center = center(bounds(&mut visual, &view, "grid", "Selection"));
    let (sin, cos) = 30_f64.to_radians().sin_cos();
    let map = |x: f64, y: f64| {
        let (a, b) = (-1.25 * (x * 2. - 130.), 0.9 * (y * 2. - 60.));
        relative_point(window_center, (cos * a - sin * b, sin * a + cos * b))
    };
    let target = map(40., 40.);
    let start = relative_point(target, (-11., -13.));
    let end = relative_point(target, (11., 13.));
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    let painted = visual.update(|window, _| {
        let scale = window.scale_factor();
        window.visual_probe_with_painted_scene(|scene| {
            let mut points = Vec::new();
            marquee_vertices(scene, gpui::TransformationMatrix::unit(), &mut points);
            for (x, y) in &mut points {
                *x /= scale;
                *y /= scale;
            }
            points
        })
    });
    assert!(!painted.is_empty(), "marquee must actually paint");
    for (x, y) in [
        (f32::from(start.x), f32::from(start.y)),
        (f32::from(end.x), f32::from(start.y)),
        (f32::from(end.x), f32::from(end.y)),
        (f32::from(start.x), f32::from(end.y)),
    ] {
        assert!(
            painted
                .iter()
                .any(|p| (p.0 - x).abs() < 0.1 && (p.1 - y).abs() < 0.1),
            "missing corner ({x},{y}): {painted:?}"
        );
    }
    assert_eq!(
        status(&mut visual, &view),
        "[]",
        "preview is not a proposal"
    );
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert_eq!(status(&mut visual, &view), "[\"a\"]");
}

fn marquee_vertices(
    scene: &gpui::Scene,
    transform: gpui::TransformationMatrix,
    vertices: &mut Vec<(f32, f32)>,
) {
    for path in &scene.paths {
        if !path
            .color
            .as_solid()
            .is_some_and(|ink| (ink.a - (32. / 255.)).abs() < 0.001)
        {
            continue;
        }
        let [[a, b], [c, d]] = transform.rotation_scale;
        let [tx, ty] = transform.translation;
        vertices.extend(path.vertices.iter().map(|v| {
            let x = v.xy_position.x.0;
            let y = v.xy_position.y.0;
            (a * x + b * y + tx, c * x + d * y + ty)
        }));
    }
    for group in &scene.isolated_groups {
        marquee_vertices(
            group.source(),
            transform.compose(group.transform()),
            vertices,
        );
    }
    #[cfg(feature = "gpui-fast")]
    for child in scene.layers.frames.iter().flat_map(|f| f.content.scenes()) {
        marquee_vertices(child, transform, vertices);
    }
}

#[gpui::test]
fn rotatable_reads_angles_before_the_ancestor_reflection_and_skew(cx: &mut TestAppContext) {
    let (w, view) = mount(
        cx,
        r#"
import "components/rotatable" as rot;
fn state_schema(){#{fields:#{angle:#{schema:#{type:"number"},"default":#{type:"float",value:0.0}}}}}
fn changed(ctx,value){ctx.set_state("angle",value);}
fn view(ctx){let a=ctx.get_state("angle");
    column([text(`${a}`).accessibility_role("status"),
        rot::Rotatable(#{key:"rot",label:"Mapped rotation",angle:a,pivot:#{x:105.0,y:60.0},
            content:canvas(canvas_scene([canvas_rect("marker",180,80,8,8,rgba(0xdd1122ff))])),
            on_rotate:Fn("changed")}).with_style(style().width(px(260)).height(px(140)).border(px(0))
                .transform(#{scale_x:-1.4,scale_y:0.8,skew_x:15,rotate:25}))
    ]).with_style(style().padding_top(px(90)).padding_left(px(160)))
}
"#,
        "affine-rotate",
    );
    let mut visual = VisualTestContext::from_window(*w, cx);
    let center = center(bounds(&mut visual, &view, "slider", "Mapped rotation"));
    let map = |x, y| {
        let b = y * 0.8;
        let a = -1.4 * x + b * 15_f64.to_radians().tan();
        let (sin, cos) = 25_f64.to_radians().sin_cos();
        relative_point(center, (cos * a - sin * b, sin * a + cos * b))
    };
    let start = map(-25. + 55., -10. + 12.);
    let end = map(-25. + 20., -10. + 45.);
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    let preview = signal_value(&mut visual, &view, SignalProperty::Rotate);
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    let actual = status(&mut visual, &view).parse::<f64>().unwrap();
    let expected = 45_f64.atan2(20.).to_degrees() - 12_f64.atan2(55.).to_degrees();
    assert!(
        (preview - expected).abs() < 0.1,
        "preview {preview} != {expected}"
    );
    assert!((actual - expected).abs() < 0.1, "{actual} != {expected}");
}

#[gpui::test]
fn canvas_pointer_payload_hits_the_painted_shape_inside_a_transformed_parent(
    cx: &mut TestAppContext,
) {
    let (w, view) = mount(
        cx,
        r#"
fn state_schema(){#{fields:#{hit:#{schema:#{type:"string"},"default":#{type:"string",value:"unseen"}}}}}
fn hit(ctx,event){ctx.set_state("hit",if event.canvas_key==(){"none"}else{event.canvas_key});}
fn view(ctx){column([text(ctx.get_state("hit")).accessibility_role("status"),
    column([canvas(canvas_scene([canvas_rect("shape",10,8,12,12,theme_color("accent"))])
        .view_box(0,0,100,60)).with_key("canvas")
        .with_style(style().width(px(200)).height(px(120)))
        .accessibility_role("button").accessibility_label("Mapped vector").on("pointer_down",Fn("hit"))])
    .with_style(style().width(px(200)).height(px(120)).transform(#{scale_x:-1.0,rotate:90}))
]).with_style(style().padding_top(px(120)).padding_left(px(150)))}
"#,
        "affine-canvas-payload",
    );
    let mut visual = VisualTestContext::from_window(*w, cx);
    let center = center(bounds(&mut visual, &view, "button", "Mapped vector"));
    // source (16,14) -> drawable (32,28), relative (-68,-32),
    // reflected across X then rotated clockwise -> (+32,+68).
    visual.simulate_click(relative_point(center, (32., 68.)), Modifiers::default());
    visual.run_until_parked();
    assert_eq!(status(&mut visual, &view), "shape");
    visual.simulate_click(relative_point(center, (-32., -68.)), Modifiers::default());
    visual.run_until_parked();
    assert_eq!(status(&mut visual, &view), "none");
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
