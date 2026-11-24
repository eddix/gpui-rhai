use std::time::{Duration, Instant};

use gpui::{Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, ScriptViewHandle};

use crate::support::{
    ActiveHost, Mount, Step, bounds, by_test_id, center, click_center, dispatch, last_error,
    settle, text_of,
};

const FIELD: &str = r#"
 import "components/number_input" as field;
 import "components/button" as button;
 fn state_schema(){#{fields:#{
    value:#{schema:#{type:"optional",value:#{type:"number"}},"default":#{type:"float",value:2.5}},
    reject:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    change_count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
 }}}
 fn changed(ctx,v){
    ctx.set_state("change_count",ctx.get_state("change_count")+1);
    if !ctx.get_state("reject"){ctx.set_state("value",v);}
 }
 fn toggle(ctx,v){ctx.set_state("reject",!ctx.get_state("reject"));}
 fn external(ctx,v){ctx.set_state("value",7.25);}
 fn view(ctx){column([
   field::NumberInput(#{key:"budget",label:"Budget",value:ctx.get_state("value"),
        min:0,max:10,step:0.5,large_step:2,placeholder:"Empty",unit:"credits",on_change:Fn("changed")})
       .with_style(style().width(px(230))),
   field::NumberInput(#{key:"readonly",label:"Read only",value:4,read_only:true})
       .with_style(style().width(px(180))),
   button::Button(#{key:"after",text:"After"}),
   text(ctx.get_state("change_count").to_string()).test_id("count"),
   text(if ctx.get_state("value")==(){ "empty" }else{ctx.format_number(ctx.get_state("value"))}).test_id("value"),
   text("reject").test_id("reject").on_click(Fn("toggle")),
   text("external").test_id("external").on_click(Fn("external"))
 ]).with_style(style().padding(px(20)).items_start().gap(px(12)))}
"#;

const HOLD: &str = r#"
 import "components/number_input" as field;
 fn state_schema(){#{fields:#{
    value:#{schema:#{type:"number"},"default":#{type:"float",value:2.5}},
    count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    epoch:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    step:#{schema:#{type:"number"},"default":#{type:"float",value:0.5}},
    disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    alternate:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    hidden:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}
 }}}
 fn changed(ctx,v){ctx.set_state("count",ctx.get_state("count")+1);ctx.set_state("value",v);}
 fn new_handler(ctx,v){ctx.set_state("count",ctx.get_state("count")+100);ctx.set_state("value",v);}
 fn set_external(ctx,p){ctx.set_state("value",0.5);}
 fn change_step(ctx,p){ctx.set_state("step",0.25);}
 fn swap_callback(ctx,p){ctx.set_state("alternate",true);}
 fn disable(ctx,p){ctx.set_state("disabled",true);}
 fn hide(ctx,p){ctx.set_state("hidden",!ctx.get_state("hidden"));}
 fn remount(ctx,p){ctx.set_state("epoch",ctx.get_state("epoch")+1);}
 fn view(ctx){
    let items=[];
    if !ctx.get_state("hidden"){items.push(field::NumberInput(#{
        key:`held-${ctx.get_state("epoch")}`,label:"Budget",value:ctx.get_state("value"),
        min:0,max:4.0,step:ctx.get_state("step"),disabled:ctx.get_state("disabled"),
        on_change:if ctx.get_state("alternate"){Fn("new_handler")}else{Fn("changed")}
    }).with_style(style().width(px(230))));}
    column(items+[
        text(ctx.get_state("count").to_string()).test_id("count"),
        text(ctx.format_number(ctx.get_state("value"))).test_id("value"),
        text("External").test_id("external").on_click(Fn("set_external")),
        text("Step").test_id("step").on_click(Fn("change_step")),
        text("Replace callback").test_id("callback").on_click(Fn("swap_callback")),
        text("Disable").test_id("disable").on_click(Fn("disable")),
        text("Unmount").test_id("hide").on_click(Fn("hide")),
        text("Remount").test_id("remount").on_click(Fn("remount")),
    ]).with_style(style().padding(px(20)).items_start().gap(px(12)))
 }
"#;

fn hold_window(cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
    let (window, view) = Mount::components(HOLD)
        .motion(MotionPreference::None)
        .mount_root(cx, |host, view| ActiveHost { host, view });
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

fn press_increase(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Point<Pixels> {
    let position = center(bounds(visual, view, "button", "Increase"));
    visual.simulate_mouse_move(position, None, Modifiers::none());
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::none());
    settle(visual, Step::Frame, 3);
    position
}

fn wait_press(visual: &mut VisualTestContext, millis: u64) {
    visual
        .executor()
        .advance_clock(Duration::from_millis(millis));
    settle(visual, Step::Frame, 3);
}

#[gpui::test]
fn number_input_hold_uses_live_value_and_stops_at_limit(cx: &mut TestAppContext) {
    let (mut visual, view) = hold_window(cx);
    let held = press_increase(&mut visual, &view);
    assert_eq!(
        text_of(&mut visual, &view, "value"),
        "3",
        "first step is on mouse-down"
    );
    wait_press(&mut visual, 399);
    assert_eq!(text_of(&mut visual, &view, "count"), "1");
    dispatch(&mut visual, &view, by_test_id("external"), "click").unwrap();
    wait_press(&mut visual, 1);
    assert_eq!(
        text_of(&mut visual, &view, "value"),
        "1",
        "repeat reads the current committed value"
    );
    for (delay, next) in [
        (120, "1.5"),
        (108, "2"),
        (97, "2.5"),
        (87, "3"),
        (78, "3.5"),
        (70, "4"),
    ] {
        wait_press(&mut visual, delay);
        assert_eq!(text_of(&mut visual, &view, "value"), next);
    }
    wait_press(&mut visual, 800);
    visual.simulate_mouse_up(held, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(
        text_of(&mut visual, &view, "count"),
        "8",
        "limit and release do not emit extra proposals"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn number_input_hold_cancels_on_ownership_and_lifecycle_changes(cx: &mut TestAppContext) {
    for cause in [
        "release",
        "leave",
        "deactivate",
        "escape",
        "suspend",
        "step",
        "callback",
        "disable",
        "hide",
        "remount",
        "same-key",
    ] {
        let (mut visual, view) = hold_window(cx);
        let held = press_increase(&mut visual, &view);
        assert_eq!(text_of(&mut visual, &view, "count"), "1", "{cause}: press");
        match cause {
            "release" => visual.simulate_mouse_up(held, MouseButton::Left, Modifiers::none()),
            "leave" => {
                visual.simulate_mouse_move(
                    point(px(500.), px(500.)),
                    Some(MouseButton::Left),
                    Modifiers::none(),
                );
                visual.simulate_mouse_move(held, Some(MouseButton::Left), Modifiers::none());
            }
            "deactivate" => {
                visual.deactivate_window();
                visual.run_until_parked();
                visual.update(|window, _| window.activate_window());
            }
            "escape" => visual.simulate_keystrokes("escape"),
            "suspend" => {
                visual
                    .update(|window, cx| view.suspend(window, cx))
                    .unwrap();
                wait_press(&mut visual, 800);
                visual.update(|_, cx| view.resume(cx)).unwrap();
            }
            "same-key" => {
                for _ in 0..2 {
                    dispatch(&mut visual, &view, by_test_id("hide"), "click").unwrap();
                    settle(&mut visual, Step::Frame, 3);
                }
            }
            target => {
                dispatch(&mut visual, &view, by_test_id(target), "click").unwrap();
            }
        }
        wait_press(&mut visual, 800);
        visual.simulate_mouse_up(held, MouseButton::Left, Modifiers::none());
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(
            text_of(&mut visual, &view, "count"),
            "1",
            "{cause}: no stale repeat or release click"
        );
        assert_eq!(last_error(&mut visual, &view), None, "{cause}");
    }
}

#[gpui::test]
fn native_repeat_press_keeps_keyboard_and_touch_tap_one_shot(cx: &mut TestAppContext) {
    let source = r#"
      fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
      fn bump(ctx,p){ctx.set_state("count",ctx.get_state("count")+1);}
      fn view(ctx){column([
        row([text("Act")]).with_key("act").accessibility_role("button").accessibility_label("Act")
            .with_style(style().width(px(70)).height(px(34))
                .active(style().background(rgb(0x7744ff))))
            .on_click(Fn("bump")).repeat_press(),
        text(ctx.get_state("count").to_string()).test_id("count")
      ]).with_style(style().padding(px(20)).gap(px(12)))}
    "#;
    let (mut visual, view) = Mount::new(source).mount(cx);
    visual.update(|window, _| window.activate_window());
    visual.update(|window, cx| view.focus(window, cx)).unwrap();
    key(&mut visual, "tab enter");
    wait_press(&mut visual, 900);
    assert_eq!(text_of(&mut visual, &view, "count"), "1");
    let position = center(bounds(&mut visual, &view, "button", "Act"));
    for phase in [gpui::TouchPhase::Started, gpui::TouchPhase::Ended] {
        visual.simulate_event(gpui::TouchEvent {
            id: gpui::TouchId(42),
            phase,
            position,
            predicted_position: None,
            force: None,
        });
        settle(&mut visual, Step::Frame, 3);
    }
    wait_press(&mut visual, 900);
    assert_eq!(text_of(&mut visual, &view, "count"), "2");
    let pressed_paint = |visual: &mut VisualTestContext| {
        let pressed: gpui::Hsla = gpui::rgba(0x7744_ffff).into();
        crate::support::painted_quads(visual).iter().any(|quad| {
            quad.background.as_solid() == Some(pressed) && (quad.bounds.width - 70.0).abs() < 0.1
        })
    };
    assert!(!pressed_paint(&mut visual));
    visual.simulate_mouse_move(position, None, Modifiers::none());
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::Frame, 3);
    assert!(
        pressed_paint(&mut visual),
        "the coordinated lease owns active paint"
    );
    visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::none());
    wait_press(&mut visual, 900);
    assert!(!pressed_paint(&mut visual));
    assert_eq!(text_of(&mut visual, &view, "count"), "3");
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn native_repeat_press_does_not_repeat_a_failed_callback(cx: &mut TestAppContext) {
    let reports = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = reports.clone();
    let (mut visual, view) = Mount::new(
        r#"
        fn fail(ctx,p){throw "refuse repeat";}
        fn view(ctx){row([text("Fault")]).with_key("repeat").accessibility_role("button")
            .accessibility_label("Fault").on_click(Fn("fail")).repeat_press()
            .with_style(style().width(px(80)).height(px(30)))}
    "#,
    )
    .config(move |config| {
        config
            .show_error_banner(false)
            .on_error(move |_, _| count.set(count.get() + 1))
    })
    .mount(cx);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 3);
    let held = center(bounds(&mut visual, &view, "button", "Fault"));
    visual.simulate_mouse_down(held, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::TickRefresh, 3);
    assert_eq!(reports.get(), 1);
    assert!(
        last_error(&mut visual, &view).is_some_and(|message| message.contains("refuse repeat"))
    );
    for delay in [400, 120, 900] {
        wait_press(&mut visual, delay);
    }
    assert_eq!(reports.get(), 1);
    visual.simulate_mouse_up(held, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_down(held, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::TickRefresh, 3);
    assert_eq!(reports.get(), 2, "a fresh press can try again");
}

fn setup(cx: &mut TestAppContext, theme: &str) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(FIELD)
        .theme(theme)
        .motion(MotionPreference::None)
        .mount(cx);
    visual.update(|w, _| w.activate_window());
    settle(&mut visual, Step::TickRefresh, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

fn key(visual: &mut VisualTestContext, keys: &str) {
    visual.simulate_keystrokes(keys);
    settle(visual, Step::TickRefresh, 3);
}

fn edit(visual: &mut VisualTestContext, value: &str) {
    key(visual, &crate::support::primary_keys("secondary-a"));
    if value.is_empty() {
        key(visual, "backspace");
    } else {
        visual.simulate_input(value);
        settle(visual, Step::TickRefresh, 3);
    }
}

#[gpui::test]
fn number_input_commits_draft_steps_and_preserves_single_focus_owner(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = setup(cx, theme);
        let editor = bounds(&mut visual, &view, "spinbutton", "Budget");
        let unit = bounds(&mut visual, &view, "text", "credits");
        assert!(editor.width < 60.0, "{name}: {editor:?}");
        assert!(
            (unit.x - editor.x - editor.width - 4.0).abs() < 1.0,
            "{name}: {unit:?} {editor:?}"
        );
        click_center(&mut visual, editor);
        let focus = visual.update(|w, a| w.focused(a)).unwrap();
        edit(&mut visual, "-");
        assert_eq!(text_of(&mut visual, &view, "count"), "0", "{name}");
        key(&mut visual, "escape");
        key(&mut visual, "up");
        assert_eq!(text_of(&mut visual, &view, "value"), "3", "{name}");
        key(&mut visual, "pageup");
        assert_eq!(text_of(&mut visual, &view, "value"), "5", "{name}");
        let down = bounds(&mut visual, &view, "button", "Decrease");
        click_center(&mut visual, down);
        settle(&mut visual, Step::TickRefresh, 3);
        assert_eq!(text_of(&mut visual, &view, "value"), "4.5", "{name}");
        assert_eq!(visual.update(|w, a| w.focused(a)), Some(focus.clone()));
        edit(&mut visual, "9.25");
        assert_eq!(text_of(&mut visual, &view, "value"), "4.5", "{name}");
        key(&mut visual, "enter");
        assert_eq!(text_of(&mut visual, &view, "value"), "9.25", "{name}");
        edit(&mut visual, "22");
        key(&mut visual, "tab");
        assert_eq!(text_of(&mut visual, &view, "value"), "10", "{name}");
        assert_ne!(visual.update(|w, a| w.focused(a)), Some(focus.clone()));
        key(&mut visual, "up");
        assert_eq!(text_of(&mut visual, &view, "value"), "10", "{name}");
        key(&mut visual, "shift-tab");
        assert_eq!(visual.update(|w, a| w.focused(a)), Some(focus));
        key(&mut visual, "home");
        assert_eq!(text_of(&mut visual, &view, "value"), "0", "{name}");
        key(&mut visual, "end");
        assert_eq!(text_of(&mut visual, &view, "value"), "10", "{name}");
        edit(&mut visual, "");
        key(&mut visual, "enter");
        assert_eq!(text_of(&mut visual, &view, "value"), "empty", "{name}");
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn number_input_rejected_proposals_and_external_revision_drop_only_the_draft(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = setup(cx, gpui_rhai_registry::MODERN_LIGHT_THEME);
    let editor = bounds(&mut visual, &view, "spinbutton", "Budget");
    click_center(&mut visual, editor);
    dispatch(&mut visual, &view, by_test_id("reject"), "click").unwrap();
    settle(&mut visual, Step::TickRefresh, 3);
    edit(&mut visual, "5");
    key(&mut visual, "enter");
    assert_eq!(text_of(&mut visual, &view, "count"), "1");
    assert_eq!(text_of(&mut visual, &view, "value"), "2.5");
    dispatch(&mut visual, &view, by_test_id("reject"), "click").unwrap();
    edit(&mut visual, "8");
    dispatch(&mut visual, &view, by_test_id("external"), "click").unwrap();
    settle(&mut visual, Step::TickRefresh, 3);
    key(&mut visual, "enter");
    assert_eq!(text_of(&mut visual, &view, "count"), "1");
    assert_eq!(text_of(&mut visual, &view, "value"), "7.25");
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn number_input_accepts_rtl_localized_digits_and_places_unit_after_value(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(FIELD)
        .theme(gpui_rhai_registry::CLEAN_DARK_THEME)
        .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
        .motion(MotionPreference::None)
        .mount(cx);
    visual.update(|w, _| w.activate_window());
    settle(&mut visual, Step::TickRefresh, 4);
    let editor = bounds(&mut visual, &view, "spinbutton", "Budget");
    let unit = bounds(&mut visual, &view, "text", "credits");
    assert!(
        (editor.x - unit.x - unit.width - 4.0).abs() < 1.0,
        "{unit:?} {editor:?}"
    );
    click_center(&mut visual, unit);
    edit(&mut visual, "٤٫٢٥");
    key(&mut visual, "enter");
    assert_eq!(text_of(&mut visual, &view, "value"), "٤٫٢٥");
    key(&mut visual, "up");
    assert_eq!(text_of(&mut visual, &view, "value"), "٤٫٥");
    edit(&mut visual, "٤.١٢");
    key(&mut visual, "enter");
    assert_eq!(text_of(&mut visual, &view, "value"), "٤٫٥");
    assert_eq!(text_of(&mut visual, &view, "count"), "2");
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn number_input_steps_replay_at_the_actual_value_and_leave_the_steppers_still(
    cx: &mut TestAppContext,
) {
    for motion in [MotionPreference::Normal, MotionPreference::None] {
        let manual = ManualRuntimeClock::new(Instant::now());
        let clock = manual.clock();
        let (mut visual, view) = Mount::components(FIELD)
            .motion(motion)
            .with(move |view| view.runtime_clock(clock))
            .mount(cx);
        visual.update(|window, _| window.activate_window());
        settle(&mut visual, Step::Frame, 4);
        let editor = |visual: &mut VisualTestContext| {
            crate::support::nodes(visual, &view)
                .into_iter()
                .find(|node| node.name == "Budget" && node.role == "spinbutton")
                .unwrap()
                .geometry
                .unwrap()
        };
        let step = bounds(&mut visual, &view, "button", "Increase");
        let idle = editor(&mut visual);
        assert!((idle.visual.y - idle.layout.y).abs() < 0.1);
        click_center(&mut visual, step);
        settle(&mut visual, Step::Refresh, 4);
        let entry = editor(&mut visual);
        let expected = if motion == MotionPreference::Normal {
            5.0
        } else {
            0.0
        };
        assert!(
            (entry.visual.y - idle.visual.y - expected).abs() < 0.1,
            "{motion:?}: idle={idle:?}, entry={entry:?}"
        );
        assert_eq!(bounds(&mut visual, &view, "button", "Increase"), step);
        manual.advance(Duration::from_millis(220));
        settle(&mut visual, Step::Refresh, 4);
        let landed = editor(&mut visual);
        assert!(
            (landed.visual.y - idle.visual.y).abs() < 0.1,
            "{motion:?}: {landed:?}"
        );
        let down = bounds(&mut visual, &view, "button", "Decrease");
        click_center(&mut visual, down);
        settle(&mut visual, Step::Refresh, 4);
        let reverse = editor(&mut visual);
        assert!(
            (reverse.visual.y - idle.visual.y + expected).abs() < 0.1,
            "{motion:?}: {reverse:?}"
        );
        assert_eq!(text_of(&mut visual, &view, "value"), "2.5");
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn number_input_advertises_first_frame_spinbutton_actions_but_not_when_read_only(
    cx: &mut TestAppContext,
) {
    let (mut visual, _) = setup(cx, gpui_rhai_registry::MODERN_LIGHT_THEME);
    visual.update(|window, _| window.visual_probe_activate_a11y());
    settle(&mut visual, Step::TickRefresh, 3);
    let tree: serde_json::Value = visual.update(|window, _| {
        serde_json::from_str(
            &window
                .debug_a11y_tree_json()
                .expect("native accessibility frame"),
        )
        .unwrap()
    });
    let semantics = |label| {
        tree["nodes"]
            .as_object()
            .unwrap()
            .values()
            .map(|node| &node["aria"])
            .find(|aria| aria["label"] == label)
            .unwrap()
    };
    let budget = semantics("Budget");
    assert_eq!(budget["role"], "SpinButton");
    assert_eq!(budget["numeric_value"], 2.5);
    assert_eq!(budget["min_numeric_value"], 0.0);
    assert_eq!(budget["max_numeric_value"], 10.0);
    let actions = budget["on_action"].as_array().unwrap();
    for action in ["Focus", "SetValue", "Increment", "Decrement"] {
        assert!(
            actions.iter().any(|advertised| advertised == action),
            "{budget}"
        );
    }
    let read_only = semantics("Read only");
    let readonly_actions = read_only["on_action"].as_array().unwrap();
    assert!(
        readonly_actions.iter().any(|action| action == "Focus"),
        "{read_only}"
    );
    for action in ["SetValue", "Increment", "Decrement"] {
        assert!(
            !readonly_actions
                .iter()
                .any(|advertised| advertised == action),
            "{read_only}"
        );
    }
    let button = semantics("Increase");
    assert!(
        button["on_action"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action == "Click"),
        "the repeating pointer path must not remove one-shot accessibility click: {button}"
    );
}
