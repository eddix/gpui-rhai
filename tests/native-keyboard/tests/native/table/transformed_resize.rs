//! Table handles consume the layout axis, including after an earlier preview frame.

use gpui::{Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{AutomationCommand, ElementGeometry, MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::{AR_LOCALE, EN_LOCALE};

use super::{mount_refreshed, table};
use crate::support::{Step, center, last_error, settle, snapshot};

const SCRIPT: &str = r#"
import "components/table" as table;
fn state_schema(){#{fields:#{
    output:#{schema:#{type:"string"},"default":#{type:"string",value:"untouched"}},
    commits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    changed:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    width:#{schema:#{type:"number"},"default":#{type:"float",value:140.0}}
}}}
fn init(ctx){
    ctx.set_locale("__LOCALE__");
    ctx.register_action("geometry.change",Fn("change"));
}
fn change(ctx,p){ctx.set_state("changed",true);}
fn resized(ctx,p){
    let count=ctx.get_state("commits")+1;
    ctx.set_state("commits",count);
    ctx.set_state("output",`${p.key}:${p.width.value}:${count}`);
    ctx.set_state("width",p.width.value);
}
fn view(ctx){
    let angle=if ctx.get_state("changed"){35}else{90};
    let cols=[
        #{key:"a",title:"Alpha",width:#{kind:"fixed",value:ctx.get_state("width")},max_width:260},
        #{key:"b",title:"Beta",width:#{kind:"fixed",value:240.0}},
    ];
    let grid=table::Table(#{key:"grid",label:"Rotated table",row_key:"id",height:122.0,
        resizable_columns:true,columns:cols,rows:[#{id:"only",a:"Alpha body",b:"Beta body"}],
        on_column_resize:Fn("resized")})
        .with_style(style().width(px(300)).transform(#{rotate:angle,scale_x:1.5,scale_y:0.75}));
    column([text(ctx.get_state("output")).accessibility_role("status"),grid])
        .with_style(style().padding_left(px(180)).padding_top(px(175)))
}
"#;

fn setup(cx: &mut TestAppContext, rtl: bool) -> (VisualTestContext, ScriptViewHandle) {
    let source = SCRIPT.replace("__LOCALE__", if rtl { "ar" } else { "en" });
    let (window, view) = mount_refreshed(
        cx,
        table(source)
            .locale("en.rhai", EN_LOCALE)
            .locale("ar.rhai", AR_LOCALE)
            .motion(MotionPreference::None),
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    ready(&mut visual, &view);
    (visual, view)
}

fn ready(v: &mut VisualTestContext, view: &ScriptViewHandle) {
    settle(v, Step::Tick, 4);
    assert_eq!(last_error(v, view), None);
}

fn geometry(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> ElementGeometry {
    snapshot(v, view)
        .find_by_role_and_name(role, name)
        .next()
        .unwrap_or_else(|| panic!("missing {role} {name}"))
        .geometry
        .unwrap()
}

fn output(v: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    snapshot(v, view)
        .nodes()
        .find(|n| n.role == "status")
        .unwrap()
        .name
        .clone()
}

fn down(v: &mut VisualTestContext, start: Point<Pixels>) {
    v.simulate_mouse_move(start, None, Modifiers::none());
    v.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.5, "{actual} != {expected}");
}

#[gpui::test]
fn rotated_table_uses_native_layout_widths_and_one_proposal_in_both_directions(
    cx: &mut TestAppContext,
) {
    for rtl in [false, true] {
        let (mut v, view) = setup(cx, rtl);
        let a = |v: &mut VisualTestContext| geometry(v, &view, "columnheader", "Alpha").layout;
        let b = |v: &mut VisualTestContext| geometry(v, &view, "columnheader", "Beta").layout;
        let body_a =
            |v: &mut VisualTestContext| geometry(v, &view, "gridcell", "Alpha body").layout.width;
        let name = if rtl {
            "تغيير حجم العمود Alpha"
        } else {
            "Resize Alpha column"
        };
        let grip = geometry(&mut v, &view, "separator", name);
        assert!(
            grip.visual.width > 20. && grip.visual.height < 15.,
            "{grip:?}"
        );
        near(a(&mut v).width, 140.);
        near(b(&mut v).width, 240.);
        let start = center(grip.visual);
        // Independent witness: local X scales by 1.5 and projects to visual Y.
        let point_at = |local: f32| {
            point(
                start.x,
                start.y + px(if rtl { -local } else { local } * 1.5),
            )
        };
        down(&mut v, start);
        for delta in [18., 30.] {
            v.simulate_mouse_move(point_at(delta), MouseButton::Left, Modifiers::none());
            ready(&mut v, &view);
            near(a(&mut v).width, 140. + f64::from(delta));
            near(body_a(&mut v), a(&mut v).width);
            near(b(&mut v).width, 240.);
            assert_eq!(output(&mut v, &view), "untouched");
        }
        v.simulate_mouse_up(point_at(36.), MouseButton::Left, Modifiers::none());
        ready(&mut v, &view);
        let accepted = output(&mut v, &view);
        let event = accepted
            .strip_prefix("a:")
            .unwrap()
            .strip_suffix(":1")
            .unwrap();
        near(event.parse().unwrap(), 176.);
        near(a(&mut v).width, 176.);
        near(body_a(&mut v), 176.);
    }
}

#[gpui::test]
fn rotated_table_discards_a_preview_after_the_owner_changes_transform(cx: &mut TestAppContext) {
    let (mut v, view) = setup(cx, false);
    let a = |v: &mut VisualTestContext| geometry(v, &view, "columnheader", "Alpha").layout;
    let grip = geometry(&mut v, &view, "separator", "Resize Alpha column");
    let start = center(grip.visual);
    let end = point(start.x, start.y + px(45.));
    down(&mut v, start);
    v.simulate_mouse_move(end, MouseButton::Left, Modifiers::none());
    ready(&mut v, &view);
    near(a(&mut v).width, 170.);
    v.update(|window, cx| {
        view.automate(
            AutomationCommand::Action {
                id: "geometry.change".into(),
                payload: None,
            },
            window,
            cx,
        )
    })
    .unwrap();
    ready(&mut v, &view);
    // No intervening move: release itself must revalidate the boundary.
    v.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    ready(&mut v, &view);
    assert_eq!(output(&mut v, &view), "untouched");
    near(a(&mut v).width, 140.);
}

#[gpui::test]
fn table_scroll_during_capture_invalidates_the_column_drag(cx: &mut TestAppContext) {
    let (mut v, view) = setup(cx, false);
    let a = |v: &mut VisualTestContext| geometry(v, &view, "columnheader", "Alpha");
    let grip = geometry(&mut v, &view, "separator", "Resize Alpha column");
    let start = center(grip.visual);
    let end = point(start.x, start.y + px(45.));
    down(&mut v, start);
    v.simulate_mouse_move(end, MouseButton::Left, Modifiers::none());
    ready(&mut v, &view);
    let preview = a(&mut v);
    near(preview.layout.width, 170.);
    let over_table = center(geometry(&mut v, &view, "table", "Rotated table").visual);
    v.simulate_event(gpui::ScrollEvent {
        position: over_table,
        delta: gpui::ScrollDelta::Pixels(point(px(-24.), px(0.))),
        touch_phase: gpui::TouchPhase::Moved,
        modifiers: Modifiers::none(),
        ..gpui::ScrollEvent::default()
    });
    ready(&mut v, &view);
    assert!(
        (a(&mut v).visual.y - preview.visual.y).abs() > 20.,
        "wheel really scrolled"
    );
    v.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    ready(&mut v, &view);
    assert_eq!(output(&mut v, &view), "untouched");
    near(a(&mut v).layout.width, 140.);
}
