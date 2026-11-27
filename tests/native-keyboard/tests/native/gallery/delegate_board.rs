//! The assignment fixture uses native drag routing and the keyboard focus path.

use std::time::{Duration, Instant};

use gpui::{
    Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, point, px, size,
};
use gpui_rhai::{
    GeometryBounds, ManualRuntimeClock, MotionPreference, ScriptViewHandle, UiNode, UiNodeKind,
    UiValue,
};

use super::conversation::{gallery, node, ready};
use crate::support::{Mount, bounds, center, click_center, snapshot};

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let rect = bounds(visual, view, role, name);
    visual.simulate_mouse_move(center(rect), None, Modifiers::none());
    ready(visual, view);
    let rect = bounds(visual, view, role, name);
    click_center(visual, rect);
    ready(visual, view);
}

fn tick(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(700));
    ready(visual, view);
}

fn within(box_: GeometryBounds, child: GeometryBounds) -> bool {
    let x = child.x + child.width / 2.0;
    let y = child.y + child.height / 2.0;
    x > box_.x && x < box_.x + box_.width && y > box_.y && y < box_.y + box_.height
}

fn in_zone(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    name: &str,
    zone: &str,
) -> bool {
    let source = bounds(visual, view, "button", name);
    let zone = bounds(visual, view, "group", zone);
    within(zone, source)
}

fn start_drag(visual: &mut VisualTestContext, start: Point<Pixels>) {
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_move(
        point(start.x + px(12.), start.y + px(3.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    visual.run_until_parked();
}

fn release(visual: &mut VisualTestContext, end: Point<Pixels>) {
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    visual.run_until_parked();
}

fn drag_to(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str, zone: &str) {
    let source = center(bounds(visual, view, "button", name));
    let target = center(bounds(visual, view, "group", zone));
    start_drag(visual, source);
    release(visual, target);
    ready(visual, view);
}

const NOTES: &str = "Move Draft release notes";
const BUILDER: &str = "Builder queue";
const PLANNER: &str = "Planner queue";
const PILE: &str = "Unassigned";

#[gpui::test]
fn gallery_delegate_board_drag_is_controlled_and_can_return_to_the_pile(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "delegate_board", "modern-light");
    visual.simulate_resize(size(px(1280.), px(940.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Decline next assignment");
    drag_to(&mut visual, &view, NOTES, BUILDER);
    node(&mut visual, &view, "status", "Waiting for the demo Host…");
    assert!(in_zone(&mut visual, &view, NOTES, PILE));
    tick(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined this assignment.",
    );
    assert!(in_zone(&mut visual, &view, NOTES, PILE));

    drag_to(&mut visual, &view, NOTES, BUILDER);
    tick(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "Accepted manual assignment: 1",
    );
    assert!(in_zone(&mut visual, &view, NOTES, BUILDER));
    drag_to(&mut visual, &view, NOTES, PILE);
    tick(&mut visual, &view);
    assert!(in_zone(&mut visual, &view, NOTES, PILE));
    node(&mut visual, &view, "status", "Assignment proposals: 3");
}

#[gpui::test]
fn gallery_delegate_board_keyboard_cancel_restores_source_focus_and_move_works(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "delegate_board", "clean-dark");
    visual.simulate_resize(size(px(650.), px(1110.)));
    ready(&mut visual, &view);
    let pile = bounds(&mut visual, &view, "group", PILE);
    let planner = bounds(&mut visual, &view, "group", PLANNER);
    assert!(planner.y > pile.y + pile.height);
    assert!(
        node(&mut visual, &view, "button", NOTES)
            .description
            .contains("7 min")
    );
    click(&mut visual, &view, "button", NOTES);
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "button", "Assign to Planner");
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("button", "Assign to Planner")
            .next()
            .is_none()
    );
    node(&mut visual, &view, "status", "Assignment proposals: 0");
    visual.simulate_keystrokes("space");
    ready(&mut visual, &view);
    node(&mut visual, &view, "button", "Assign to Planner");
    visual.simulate_keystrokes("down");
    ready(&mut visual, &view);
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    tick(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "Accepted manual assignment: 1",
    );
    assert!(in_zone(&mut visual, &view, NOTES, BUILDER));
}

#[gpui::test]
fn gallery_delegate_board_auto_is_atomic_and_does_not_retry_the_same_revision(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "delegate_board", "modern-dark");
    visual.simulate_resize(size(px(1290.), px(990.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Decline next assignment");
    click(&mut visual, &view, "switch", "Auto");
    tick(&mut visual, &view);
    tick(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined this assignment.",
    );
    assert!(in_zone(&mut visual, &view, NOTES, PILE));
    node(&mut visual, &view, "status", "Assignment proposals: 1");
    tick(&mut visual, &view);
    node(&mut visual, &view, "status", "Assignment proposals: 1");

    click(&mut visual, &view, "button", "Add sample task");
    tick(&mut visual, &view);
    tick(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted auto assignment: 4");
    assert!(in_zone(&mut visual, &view, NOTES, BUILDER));
    assert!(in_zone(
        &mut visual,
        &view,
        "Move Prepare fixtures",
        PLANNER
    ));
    assert!(in_zone(
        &mut visual,
        &view,
        "Move Check keyboard paths",
        BUILDER
    ));
    assert!(in_zone(
        &mut visual,
        &view,
        "Move Inspect night build 1",
        PLANNER
    ));
    node(&mut visual, &view, "status", "Assignment proposals: 2");
    click(&mut visual, &view, "button", "Add sample task");
    tick(&mut visual, &view);
    tick(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted auto assignment: 1");
    assert!(in_zone(
        &mut visual,
        &view,
        "Move Inspect night build 2",
        PLANNER
    ));
}

#[gpui::test]
fn gallery_delegate_board_replacement_cancels_an_older_native_drag(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "delegate_board", "clean-light");
    visual.simulate_resize(size(px(1290.), px(990.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Schedule new snapshot");
    let source = center(bounds(&mut visual, &view, "button", NOTES));
    start_drag(&mut visual, source);
    tick(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "Replaced the accepted snapshot.",
    );
    let target = center(bounds(&mut visual, &view, "group", BUILDER));
    release(&mut visual, target);
    ready(&mut visual, &view);
    tick(&mut visual, &view);
    assert!(in_zone(&mut visual, &view, NOTES, PILE));
    node(&mut visual, &view, "status", "Assignment proposals: 0");
    click(&mut visual, &view, "button", "Assign all");
    tick(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted all assignment: 3");
}

#[gpui::test]
fn delegate_board_two_boards_with_the_same_local_ids_do_not_cross_accept(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(
        r#"
        import "patterns/delegate_board" as board;
        fn state_schema(){#{fields:#{message:#{schema:#{type:"string"},"default":#{type:"string",value:"No proposal"}}}}}
        fn record(ctx,p){ctx.set_state("message",p.board+":"+p.moves[0].task+":"+p.moves[0].agent);}
        fn side(label,instance){
            board::DelegateBoard(#{key:instance,board:instance,revision:0,now_ms:0,
                agents:[#{id:"a",name:label+" agent"}],tasks:[#{id:"same/task",title:label+" task",estimate:8}],
                on_assign:Fn("record")}).with_style(style().width(relative(0.48)).min_width(px(0)))
        }
        fn view(ctx){column([row([side("North","north"),side("South","south")]).with_style(style().gap(px(12))),
            text(ctx.get_state("message")).with_key("message").accessibility_role("status")]).with_style(style().padding(px(12)))}
    "#,
    )
    .registry("patterns/delegate_board")
    .bundled_assets()
    .mount(cx);
    visual.simulate_resize(size(px(1330.), px(800.)));
    ready(&mut visual, &view);
    drag_to(&mut visual, &view, "Move North task", "South agent queue");
    node(&mut visual, &view, "status", "No proposal");
    drag_to(&mut visual, &view, "Move North task", "North agent queue");
    node(&mut visual, &view, "status", "north:same/task:a");
    drag_to(&mut visual, &view, "Move South task", "South agent queue");
    node(&mut visual, &view, "status", "south:same/task:a");
}

#[gpui::test]
fn delegate_board_accepted_motion_uses_the_shared_layout_clock(cx: &mut TestAppContext) {
    fn moving_card(node: &UiNode) -> Option<(&UiValue, &UiValue)> {
        if node.key().is_some_and(|key| key.as_str() == "move-sample") {
            return Some((
                node.attributes().get("shared_layout_group")?,
                node.attributes().get("layout_motion_duration_ms")?,
            ));
        }
        match node.kind() {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                children.iter().find_map(moving_card)
            }
            _ => None,
        }
    }
    const SOURCE: &str = r#"
        import "patterns/delegate_board" as board;
        fn state_schema(){#{fields:#{assigned:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn moved(ctx,p){if p.board=="animated"&&p.revision==0&&p.moves[0].task=="sample"{ctx.set_state("assigned",true);}}
        fn view(ctx){
            let changed=ctx.get_state("assigned");
            board::DelegateBoard(#{key:"board",board:"animated",revision:if changed{1}else{0},now_ms:0,
                agents:[#{id:"b",name:"Builder"}],
                tasks:[#{id:"sample",title:"Inspect sample",estimate:4,agent:if changed{"b"}else{()}}],
                on_assign:Fn("moved")})
        }
    "#;
    for preference in [MotionPreference::Normal, MotionPreference::None] {
        let manual = ManualRuntimeClock::new(Instant::now());
        let clock = manual.clock();
        let (mut visual, view) = Mount::new(SOURCE)
            .registry("patterns/delegate_board")
            .bundled_assets()
            .motion(preference)
            .with(move |view| view.runtime_clock(clock))
            .mount(cx);
        visual.simulate_resize(size(px(1100.), px(850.)));
        ready(&mut visual, &view);
        let original = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
        let attrs_before = moving_card(&original).expect("source shared identity and duration");
        let before = bounds(&mut visual, &view, "button", "Move Inspect sample");
        click(&mut visual, &view, "button", "Assign all");
        let moved = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
        let attrs_after = moving_card(&moved).expect("destination shared identity and duration");
        assert_eq!(attrs_before, attrs_after);
        let start = bounds(&mut visual, &view, "button", "Move Inspect sample");
        manual.advance(Duration::from_millis(80));
        ready(&mut visual, &view);
        let middle = bounds(&mut visual, &view, "button", "Move Inspect sample");
        manual.advance(Duration::from_millis(950));
        ready(&mut visual, &view);
        let end = bounds(&mut visual, &view, "button", "Move Inspect sample");
        assert!(end.x > before.x + 200.);
        if preference == MotionPreference::Normal {
            assert!(
                middle.x > start.x && middle.x < end.x,
                "{before:?} -> {start:?} -> {middle:?} -> {end:?}"
            );
        } else {
            assert!((start.x - end.x).abs() < 1. && (middle.x - end.x).abs() < 1.);
        }
    }
}
