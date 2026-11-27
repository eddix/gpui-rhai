use gpui::{
    Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, point, px, size,
};
use gpui_rhai::{AutomationCommand, MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::{AR_LOCALE, EN_LOCALE};

use crate::support::{Mount, Step, bounds, center, last_error, live_fills, settle, snapshot};

const INK: u32 = 0xdd42_7eff;
const SCRIPT: &str = r#"
import "components/slider" as slider;
import "components/range_slider" as range;
fn state_schema(){#{fields:#{
    value:#{schema:#{type:"number"},"default":#{type:"float",value:20.0}},
    low:#{schema:#{type:"number"},"default":#{type:"float",value:20.0}},
    high:#{schema:#{type:"number"},"default":#{type:"float",value:80.0}},
    calls:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    turned:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}}
}}}
fn init(ctx){
    ctx.set_locale("__LOCALE__");
    ctx.register_action("test.rotate",Fn("rotate"));
    ctx.register_action("test.replace",Fn("replace"));
}
fn rotate(ctx,p){ctx.set_state("turned",false);}
fn replace(ctx,p){ctx.set_state("value",35.0);}
fn change_one(ctx,value){
    if __REJECT__ { return; }
    ctx.set_state("value",value);
    ctx.set_state("calls",ctx.get_state("calls")+1);
}
fn change_range(ctx,value){
    ctx.set_state("low",value.low);ctx.set_state("high",value.high);
    ctx.set_state("calls",ctx.get_state("calls")+1);
}
fn view(ctx){
    let angle=if ctx.get_state("turned"){90}else{30};
    let axis="__AXIS__";
    let parts=#{
        control:style().transform(#{rotate:angle,scale_x:1.5,scale_y:0.75}),
        track:if axis=="horizontal"{style().width(px(160))}else{style().height(px(120))},
        thumb:style().background(rgba(0xdd427eff)),
    };
    let status=`${ctx.get_state("value")}:${ctx.get_state("low")}:${ctx.get_state("high")}:${ctx.get_state("calls")}`;
    let control=if __RANGE__{
        range::RangeSlider(#{key:"range",label:"Frequency",orientation:axis,step:5.0,minimum_gap:10.0,
            part_styles:parts, values:#{low:ctx.get_state("low"),high:ctx.get_state("high")},on_change:Fn("change_range")})
    }else{
        slider::Slider(#{key:"slider",label:"Level",orientation:axis,step:5.0,
            part_styles:parts,value:ctx.get_state("value"),on_change:Fn("change_one")})
    };
    column([text(status).accessibility_role("status"),control])
        .with_style(style().padding_top(px(215)).padding_left(px(250)))
}
"#;

fn mount(
    cx: &mut TestAppContext,
    range: bool,
    rtl: bool,
    horizontal: bool,
    reject: bool,
) -> (VisualTestContext, ScriptViewHandle) {
    let source = SCRIPT
        .replace("__RANGE__", if range { "true" } else { "false" })
        .replace("__REJECT__", if reject { "true" } else { "false" })
        .replace("__LOCALE__", if rtl { "ar" } else { "en" })
        .replace(
            "__AXIS__",
            if horizontal { "horizontal" } else { "vertical" },
        );
    let (mut v, view) = Mount::new(source)
        .registry("components/range_slider")
        .locale("en.rhai", EN_LOCALE)
        .locale("ar.rhai", AR_LOCALE)
        .motion(MotionPreference::None)
        .mount(cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1050.), px(750.)));
    ready(&mut v, &view);
    (v, view)
}

fn ready(v: &mut VisualTestContext, view: &ScriptViewHandle) {
    settle(v, Step::Tick, 4);
    assert_eq!(last_error(v, view), None);
}

fn status(v: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<f64> {
    snapshot(v, view)
        .nodes()
        .find(|node| node.role == "status")
        .unwrap()
        .name
        .split(':')
        .map(|value| value.parse().unwrap())
        .collect()
}

fn projected(origin: Point<Pixels>, value: f32, rtl: bool, horizontal: bool) -> Point<Pixels> {
    let unit = value / 100. - 0.5;
    // Rotate 90 degrees after nonuniform scale. Known track spans are
    // 160 and 120, not the parent control's 240 and 180.
    if horizontal {
        let axis = if rtl { -1. } else { 1. };
        point(origin.x, origin.y + px(unit * 160. * 1.5 * axis))
    } else {
        point(origin.x + px(unit * 120. * 0.75), origin.y)
    }
}

fn assert_thumb(v: &mut VisualTestContext, expected: Point<Pixels>) {
    let fills = live_fills(v, INK);
    assert!(
        fills.iter().any(|rect| {
            (rect.x + rect.width / 2. - f32::from(expected.x)).abs() <= 1.1
                && (rect.y + rect.height / 2. - f32::from(expected.y)).abs() <= 1.1
        }),
        "paint must agree with pointer {expected:?}: {fills:?}"
    );
}

fn down(v: &mut VisualTestContext, p: Point<Pixels>) {
    v.simulate_mouse_move(p, None, Modifiers::none());
    v.simulate_mouse_down(p, MouseButton::Left, Modifiers::none());
}

fn action(v: &mut VisualTestContext, view: &ScriptViewHandle, id: &str) {
    v.update(|window, cx| {
        view.automate(
            AutomationCommand::Action {
                id: id.into(),
                payload: None,
            },
            window,
            cx,
        )
    })
    .unwrap();
    ready(v, view);
}

#[gpui::test]
fn slider_drag_follows_a_shortened_rotated_track_in_horizontal_rtl_and_vertical(
    cx: &mut TestAppContext,
) {
    for horizontal in [true, false] {
        let (mut v, view) = mount(cx, false, true, horizontal, false);
        let origin = center(bounds(&mut v, &view, "slider", "Level"));
        let p = |value| projected(origin, value, true, horizontal);
        assert_thumb(&mut v, p(20.));
        down(&mut v, p(20.));
        v.simulate_mouse_move(p(65.), MouseButton::Left, Modifiers::none());
        ready(&mut v, &view);
        assert_thumb(&mut v, p(65.));
        assert_eq!(status(&mut v, &view), [20., 20., 80., 0.]);
        v.simulate_mouse_up(p(65.), MouseButton::Left, Modifiers::none());
        ready(&mut v, &view);
        assert_eq!(status(&mut v, &view), [65., 20., 80., 1.]);
        assert_thumb(&mut v, p(65.));
    }
}

#[gpui::test]
fn range_slider_selects_the_mapped_thumb_and_constrains_it_in_the_drawn_track(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = mount(cx, true, false, true, false);
    let origin = center(bounds(&mut v, &view, "group", "Frequency"));
    let p = |value| projected(origin, value, false, true);
    assert_thumb(&mut v, p(80.));
    down(&mut v, p(80.));
    v.simulate_mouse_move(p(10.), MouseButton::Left, Modifiers::none());
    ready(&mut v, &view);
    assert_thumb(&mut v, p(20.));
    assert_thumb(&mut v, p(30.));
    assert_eq!(status(&mut v, &view), [20., 20., 80., 0.]);
    v.simulate_mouse_up(p(10.), MouseButton::Left, Modifiers::none());
    ready(&mut v, &view);
    assert_eq!(status(&mut v, &view), [20., 20., 30., 1.]);
}

#[gpui::test]
fn slider_rejects_stale_geometry_and_restores_unaccepted_pointer_previews(cx: &mut TestAppContext) {
    for (invalidate, value, reject) in [
        (Some("test.rotate"), 20., false),
        (Some("test.replace"), 35., false),
        (None, 20., true),
    ] {
        let (mut v, view) = mount(cx, false, false, true, reject);
        let origin = center(bounds(&mut v, &view, "slider", "Level"));
        let p = |value| projected(origin, value, false, true);
        down(&mut v, p(20.));
        v.simulate_mouse_move(p(65.), MouseButton::Left, Modifiers::none());
        ready(&mut v, &view);
        assert_thumb(&mut v, p(65.));
        if let Some(id) = invalidate {
            action(&mut v, &view, id);
        }
        v.simulate_mouse_up(p(65.), MouseButton::Left, Modifiers::none());
        ready(&mut v, &view);
        assert_eq!(status(&mut v, &view), [f64::from(value), 20., 80., 0.]);
        if invalidate != Some("test.rotate") {
            assert_thumb(&mut v, p(value));
        }
    }
}
