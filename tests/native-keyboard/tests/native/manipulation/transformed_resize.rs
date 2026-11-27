//! A drawn resize handle and its proposal must use the same presented axes.

use gpui::{Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{AutomationCommand, ElementGeometry, MotionPreference, ScriptViewHandle};

use crate::support::{Mount, Step, center, last_error, settle, snapshot};

const SETUP: &str = r#"
fn state_schema(){#{fields:#{
    output:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
    changed:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}
}}}
fn init(ctx){ctx.register_action("geometry.change",Fn("change"));}
fn change(ctx,value){ctx.set_state("changed",true);}
fn frame(ctx,contents){
    let angle=if ctx.get_state("changed"){30}else{90};
    column([
        text(ctx.get_state("output")).accessibility_role("status"),
        contents.with_style(style().width(px(320)).height(px(220))
            .transform(#{rotate:angle,scale_x:1.5,scale_y:0.75})),
    ]).with_style(style().padding_left(px(190)).padding_top(px(155)))
}
"#;

fn mount(
    cx: &mut TestAppContext,
    import: &str,
    entry: &str,
    body: &str,
    rtl: bool,
) -> (VisualTestContext, ScriptViewHandle) {
    let script = format!("{import}\n{SETUP}\n{body}");
    let (mut visual, view) = Mount::new(script)
        .registry(entry)
        .motion(MotionPreference::None)
        .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
        .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
        .mount(cx);
    visual.update(|window, cx| {
        window.activate_window();
        view.select_locale(if rtl { "ar" } else { "en" }, cx)
            .unwrap();
    });
    ready(&mut visual, &view);
    (visual, view)
}

fn ready(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    settle(visual, Step::Tick, 4);
    assert_eq!(last_error(visual, view), None);
}

fn geometry(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> ElementGeometry {
    snapshot(visual, view)
        .find_by_role_and_name(role, name)
        .next()
        .unwrap_or_else(|| panic!("missing {role} {name}"))
        .geometry
        .unwrap()
}

fn output(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    snapshot(visual, view)
        .nodes()
        .find(|n| n.role == "status")
        .unwrap()
        .name
        .clone()
}

fn move_by(start: Point<Pixels>, dx: f32, dy: f32) -> Point<Pixels> {
    point(start.x + px(dx), start.y + px(dy))
}

fn drag_start(visual: &mut VisualTestContext, start: Point<Pixels>, end: Point<Pixels>) {
    visual.simulate_mouse_move(start, None, Modifiers::none());
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::none());
}

fn drag_end(visual: &mut VisualTestContext, end: Point<Pixels>) {
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
}

fn change_transform(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual
        .update(|window, cx| {
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
    ready(visual, view);
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.1, "{actual} != {expected}");
}

fn split_case(cx: &mut TestAppContext, orientation: &str, rtl: bool) {
    let script = r#"
fn resized(ctx,value){ctx.set_state("output",`${value}`);}
fn view(ctx){
    let pane=box([]).accessibility_role("region").accessibility_label("Start pane")
        .with_style(style().width(relative(1.0)).height(relative(1.0)));
    frame(ctx,split::SplitPane(#{key:"split",label:"Split grip",size:0.5,orientation:"__AXIS__",
        min_start:40.0,min_end:40.0,start:pane,end:text("End"),on_resize:Fn("resized")}))
}
"#
    .replace("__AXIS__", orientation);
    let (mut visual, view) = mount(
        cx,
        r#"import "components/split_pane" as split;"#,
        "components/split_pane",
        &script,
        rtl,
    );
    let pane_size = |visual: &mut VisualTestContext| {
        let g = geometry(visual, &view, "region", "Start pane").layout;
        if orientation == "horizontal" {
            g.width
        } else {
            g.height
        }
    };
    let start = center(geometry(&mut visual, &view, "separator", "Split grip").visual);
    let initial = pane_size(&mut visual);
    near(
        initial,
        if orientation == "horizontal" {
            160.
        } else {
            110.
        },
    );
    // Independent witness: rotate(90) after nonuniform scale maps local
    // +24 x to +36 y, and local +24 y to -18 x. RTL changes only logical X.
    let end = if orientation == "horizontal" {
        move_by(start, 0., if rtl { -36. } else { 36. })
    } else {
        move_by(start, -18., 0.)
    };
    drag_start(&mut visual, start, end);
    ready(&mut visual, &view);
    near(pane_size(&mut visual), initial + 24.);
    assert!(
        output(&mut visual, &view).is_empty(),
        "preview is not a proposal"
    );
    drag_end(&mut visual, end);
    ready(&mut visual, &view);
    let first = output(&mut visual, &view);
    let share: f64 = first.parse().unwrap();
    near(share * initial * 2., initial + 24.);
    near(pane_size(&mut visual), initial); // Host declined to adopt it.

    drag_start(&mut visual, start, end);
    ready(&mut visual, &view);
    near(pane_size(&mut visual), initial + 24.);
    change_transform(&mut visual, &view);
    drag_end(&mut visual, move_by(end, 20., 15.));
    ready(&mut visual, &view);
    assert_eq!(
        output(&mut visual, &view),
        first,
        "old transform cannot commit"
    );
    near(pane_size(&mut visual), initial);
}

#[gpui::test]
fn split_drag_uses_presented_axes_in_ltr_rtl_and_vertical_layouts(cx: &mut TestAppContext) {
    split_case(cx, "horizontal", false);
    split_case(cx, "horizontal", true);
    split_case(cx, "vertical", true);
}

#[gpui::test]
fn resizable_corner_preserves_the_opposite_anchor_in_its_presented_plane(cx: &mut TestAppContext) {
    let script = r#"
fn resized(ctx,p){ctx.set_state("output",`${p.x},${p.y},${p.width},${p.height}`);}
fn view(ctx){
    let surface=box([]).accessibility_role("region").accessibility_label("Card surface")
        .with_style(style().width(relative(1.0)).height(relative(1.0)));
    frame(ctx,resize::Resizable(#{key:"card",label:"Card",content:surface,handles:["nw"],
        rect:#{x:70.0,y:75.0,width:130.0,height:90.0}, on_resize:Fn("resized")}))
}
"#;
    let (mut visual, view) = mount(
        cx,
        r#"import "components/resizable" as resize;"#,
        "components/resizable",
        script,
        false,
    );
    let pane_size = |v: &mut VisualTestContext| geometry(v, &view, "region", "Card surface").layout;
    let initial = pane_size(&mut visual);
    let start = center(geometry(&mut visual, &view, "separator", "Card: nw resize handle").visual);
    let end = move_by(start, 12., -30.); // Local (-20, -16).
    drag_start(&mut visual, start, end);
    ready(&mut visual, &view);
    let preview = pane_size(&mut visual);
    near(preview.width - initial.width, 20.);
    near(preview.height - initial.height, 16.);
    assert!(
        output(&mut visual, &view).is_empty(),
        "preview is not a proposal"
    );
    drag_end(&mut visual, end);
    ready(&mut visual, &view);
    let first = output(&mut visual, &view);
    let rect: Vec<f64> = first.split(',').map(|n| n.parse().unwrap()).collect();
    assert_eq!(rect.len(), 4);
    for (actual, expected) in rect.into_iter().zip([50., 59., 150., 106.]) {
        near(actual, expected);
    }
    near(pane_size(&mut visual).width, initial.width);

    drag_start(&mut visual, start, end);
    ready(&mut visual, &view);
    near(pane_size(&mut visual).width, initial.width + 20.);
    change_transform(&mut visual, &view);
    // Exercise move-driven cancellation as well as the SplitPane release path.
    let stale = move_by(end, 12., -12.);
    visual.simulate_mouse_move(stale, Some(MouseButton::Left), Modifiers::none());
    ready(&mut visual, &view);
    near(pane_size(&mut visual).width, initial.width);
    drag_end(&mut visual, stale);
    ready(&mut visual, &view);
    assert_eq!(output(&mut visual, &view), first);
}
