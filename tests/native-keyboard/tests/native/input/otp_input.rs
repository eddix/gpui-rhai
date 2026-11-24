use std::time::{Duration, Instant};

#[path = "otp_motion.rs"]
mod presentation;

use gpui::{ClipboardItem, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, ScriptViewHandle};

use crate::support::{
    Mount, Step, bounds, by_test_id, click_center, dispatch, last_error, nodes, painted_quads,
    settle, text_of,
};

const FIELD: &str = r#"
 import "components/otp_input" as otp;
 fn state_schema(){#{fields:#{
    code:#{schema:#{type:"string"},"default":#{type:"string",value:""},sensitive:true},
    status:#{schema:#{type:"string"},"default":#{type:"string",value:"idle"}},
    changes:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    completed:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    resends:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    blocked:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
 }}}
 fn changed(ctx,v){ctx.set_state("changes",ctx.get_state("changes")+1);ctx.set_state("code",v);}
 fn complete(ctx,v){ctx.set_state("completed",ctx.get_state("completed")+1);}
 fn resend(ctx,p){ctx.set_state("resends",ctx.get_state("resends")+1);}
 fn set_status(name,ctx,p){ctx.set_state("status",name);}
 fn replace(ctx,p){ctx.set_state("code","653401");}
 fn view(ctx){column([
    otp::OtpInput(#{key:"verify",label:"Verification code",value:ctx.get_state("code"),length:6,
        separator:true,status:ctx.get_state("status"),error_message:"Try another code",
        resend_seconds:2,on_resend:Fn("resend"),on_change:Fn("changed"),on_complete:Fn("complete")})
      .with_style(style().width(relative(1.0)).max_width(px(370))),
    text(ctx.get_state("code")).test_id("code"),
    text(ctx.get_state("changes").to_string()).test_id("changes"),
    text(ctx.get_state("completed").to_string()).test_id("complete"),
    text(ctx.get_state("resends").to_string()).test_id("resends"),
    text("verify").test_id("verify").on_click(Fn("set_status").curry("verifying")),
    text("fail").test_id("fail").on_click(Fn("set_status").curry("error")),
    text("success").test_id("success").on_click(Fn("set_status").curry("success")),
    text("replace").test_id("replace").on_click(Fn("replace")),
 ]).with_style(style().padding(px(20)).gap(px(10)).items_start().width(relative(1.0)))}
"#;

fn mount(
    cx: &mut TestAppContext,
    source: &str,
    theme: &str,
) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(source)
        .theme(theme)
        .motion(MotionPreference::None)
        .mount(cx);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

fn refresh(visual: &mut VisualTestContext) {
    settle(visual, Step::Frame, 4);
}

fn tick(visual: &mut VisualTestContext, millis: u64) {
    visual
        .executor()
        .advance_clock(Duration::from_millis(millis));
    settle(visual, Step::Refresh, 4);
}

fn field_box(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> gpui_rhai::GeometryBounds {
    bounds(visual, view, "text_field", "Verification code")
}

fn cells(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    nominal_width: f32,
) -> Vec<crate::support::Rect> {
    let editor = field_box(visual, view);
    let mut cells = painted_quads(visual)
        .into_iter()
        .map(|quad| quad.bounds)
        .filter(|rect| {
            (rect.width - nominal_width).abs() < 0.5
                && f64::from(rect.y) >= editor.y - 0.55
                && f64::from(rect.bottom()) <= editor.y + editor.height + 0.55
        })
        .collect::<Vec<_>>();
    cells.sort_by(|a, b| a.x.total_cmp(&b.x));
    cells.dedup_by(|a, b| (a.x - b.x).abs() < 0.1);
    cells
}

#[gpui::test]
fn otp_input_normalizes_paste_and_replaces_the_clicked_cell(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = mount(cx, FIELD, theme);
        let editor = field_box(&mut visual, &view);
        let nominal = if name.starts_with("clean") {
            40.0
        } else {
            48.0
        };
        let positions = cells(&mut visual, &view, nominal);
        assert_eq!(positions.len(), 6, "{name}: {editor:?} {positions:?}");
        assert!(positions[3].x - positions[2].x > positions[1].x - positions[0].x + 8.0);
        assert!(
            positions
                .iter()
                .all(|cell| f64::from(cell.right()) <= editor.x + editor.width + 0.5)
        );
        click_center(&mut visual, editor);
        visual.simulate_input("１２a٣");
        refresh(&mut visual);
        assert_eq!(text_of(&mut visual, &view, "code"), "123");
        assert_eq!(text_of(&mut visual, &view, "complete"), "0");
        visual.update(|_, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string("０９-٨７ ６５".into()))
        });
        visual.simulate_keystrokes("cmd-v");
        refresh(&mut visual);
        assert_eq!(text_of(&mut visual, &view, "code"), "098765");
        assert_eq!(text_of(&mut visual, &view, "complete"), "1");
        let third = positions[2];
        let mouse = point(
            px(third.x + third.width / 2.),
            px(third.y + third.height / 2.),
        );
        visual.simulate_mouse_down(mouse, MouseButton::Left, Modifiers::none());
        visual.simulate_mouse_up(mouse, MouseButton::Left, Modifiers::none());
        visual.simulate_input("4");
        refresh(&mut visual);
        assert_eq!(text_of(&mut visual, &view, "code"), "094765");
        assert_eq!(text_of(&mut visual, &view, "complete"), "2");
        let before_invalid = text_of(&mut visual, &view, "changes");
        visual.simulate_input("invalid");
        refresh(&mut visual);
        assert_eq!(text_of(&mut visual, &view, "changes"), before_invalid);
        assert_eq!(text_of(&mut visual, &view, "code"), "094765");
        visual.simulate_keystrokes("up right delete");
        refresh(&mut visual);
        assert_eq!(text_of(&mut visual, &view, "code"), "04765");
        let semantic = nodes(&mut visual, &view);
        assert_eq!(
            semantic
                .iter()
                .filter(|node| node.role == "text_field")
                .count(),
            1
        );
        assert!(
            !semantic
                .iter()
                .any(|node| node.name == "Verification code" && node.value.is_some())
        );
        assert_eq!(last_error(&mut visual, &view), None, "{name}");
    }
}

#[gpui::test]
fn otp_input_verifying_and_error_use_the_same_editor_and_never_clear_a_replacement(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = mount(cx, FIELD, gpui_rhai_registry::CLEAN_DARK_THEME);
    let editor = field_box(&mut visual, &view);
    click_center(&mut visual, editor);
    let focus = visual.update(|window, app| window.focused(app)).unwrap();
    visual.simulate_input("123456");
    refresh(&mut visual);
    dispatch(&mut visual, &view, by_test_id("verify"), "click").unwrap();
    refresh(&mut visual);
    assert_eq!(
        visual.update(|window, app| window.focused(app)),
        Some(focus)
    );
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("654321");
    refresh(&mut visual);
    assert_eq!(text_of(&mut visual, &view, "code"), "123456");
    dispatch(&mut visual, &view, by_test_id("fail"), "click").unwrap();
    refresh(&mut visual);
    assert!(
        nodes(&mut visual, &view)
            .iter()
            .any(|node| node.name == "Try another code")
    );
    tick(&mut visual, 400);
    dispatch(&mut visual, &view, by_test_id("replace"), "click").unwrap();
    tick(&mut visual, 1600);
    assert_eq!(text_of(&mut visual, &view, "code"), "653401");
    dispatch(&mut visual, &view, by_test_id("verify"), "click").unwrap();
    dispatch(&mut visual, &view, by_test_id("fail"), "click").unwrap();
    refresh(&mut visual);
    tick(&mut visual, 1100);
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(text_of(&mut visual, &view, "code"), "");
    assert_eq!(text_of(&mut visual, &view, "complete"), "1");
    dispatch(&mut visual, &view, by_test_id("success"), "click").unwrap();
    refresh(&mut visual);
    assert!(
        nodes(&mut visual, &view)
            .iter()
            .any(|node| node.name == "Verified" && node.role == "status")
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn otp_input_resend_countdown_rearms_once_per_activation(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, FIELD, gpui_rhai_registry::MODERN_LIGHT_THEME);
    let available = |visual: &mut VisualTestContext| {
        nodes(visual, &view)
            .iter()
            .any(|node| node.role == "button" && node.name == "Resend code")
    };
    assert!(!available(&mut visual));
    for _ in 0..2 {
        tick(&mut visual, 1100);
    }
    assert!(available(&mut visual));
    let click = bounds(&mut visual, &view, "button", "Resend code");
    click_center(&mut visual, click);
    refresh(&mut visual);
    assert_eq!(text_of(&mut visual, &view, "resends"), "1");
    assert!(!available(&mut visual));
    for _ in 0..2 {
        tick(&mut visual, 1100);
    }
    assert!(available(&mut visual));
    assert_eq!(text_of(&mut visual, &view, "resends"), "1");
    assert_eq!(last_error(&mut visual, &view), None);
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn platform_input<T>(
    visual: &mut VisualTestContext,
    f: impl FnOnce(&mut gpui::PlatformInputHandler) -> T,
) -> T {
    let mut handler = visual
        .update(|window, _| window.visual_probe_take_input())
        .expect("focused segmented editor");
    let output = f(&mut handler);
    visual.update(|window, _| window.visual_probe_restore_input(handler));
    output
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn otp_input_platform_preedit_and_resized_cell_geometry_share_one_owner(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, FIELD, gpui_rhai_registry::MODERN_DARK_THEME);
    let editor = field_box(&mut visual, &view);
    click_center(&mut visual, editor);
    refresh(&mut visual);
    let focus = platform_input(&mut visual, |input| input.focus_id());
    platform_input(&mut visual, |input| {
        input.replace_and_mark_text_in_range(None, "１２", Some(2..2))
    });
    refresh(&mut visual);
    assert_eq!(text_of(&mut visual, &view, "changes"), "0");
    platform_input(&mut visual, |input| {
        assert_eq!(input.marked_text_range(), Some(0..2));
        let a = input.bounds_for_range(0..1).unwrap();
        let b = input.bounds_for_range(1..2).unwrap();
        assert!(a.right() < b.left());
        assert_eq!(input.character_index_for_point(b.center()), Some(1));
        input.unmark_text();
    });
    refresh(&mut visual);
    assert_eq!(text_of(&mut visual, &view, "code"), "12");
    assert_eq!(text_of(&mut visual, &view, "changes"), "1");
    platform_input(&mut visual, |input| {
        input.replace_and_mark_text_in_range(None, "中", Some(1..1))
    });
    refresh(&mut visual);
    platform_input(&mut visual, gpui::PlatformInputHandler::unmark_text);
    refresh(&mut visual);
    assert_eq!(text_of(&mut visual, &view, "changes"), "1");
    let wide = platform_input(&mut visual, |input| input.bounds_for_range(1..2).unwrap());
    visual.simulate_resize(gpui::size(px(205.), px(430.)));
    refresh(&mut visual);
    let narrow = platform_input(&mut visual, |input| {
        assert_eq!(input.focus_id(), focus);
        let b = input.bounds_for_range(1..2).unwrap();
        assert_eq!(input.character_index_for_point(b.center()), Some(1));
        b
    });
    assert!(narrow.size.width < wide.size.width, "{narrow:?}, {wide:?}");
    visual.simulate_mouse_down(narrow.center(), MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(narrow.center(), MouseButton::Left, Modifiers::none());
    visual.simulate_input("8");
    refresh(&mut visual);
    assert_eq!(text_of(&mut visual, &view, "code"), "18");
    platform_input(&mut visual, |input| {
        input.replace_and_mark_text_in_range(None, "５", Some(1..1))
    });
    refresh(&mut visual);
    dispatch(&mut visual, &view, by_test_id("replace"), "click").unwrap();
    refresh(&mut visual);
    platform_input(&mut visual, |input| {
        assert_eq!(input.focus_id(), focus);
        assert_eq!(input.marked_text_range(), None);
        input.unmark_text();
    });
    refresh(&mut visual);
    assert_eq!(text_of(&mut visual, &view, "code"), "653401");
    assert_eq!(text_of(&mut visual, &view, "changes"), "2");
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn otp_input_verification_shimmer_samples_the_shared_clock_only_when_enabled(
    cx: &mut TestAppContext,
) {
    for policy in [MotionPreference::Normal, MotionPreference::None] {
        let manual = ManualRuntimeClock::new(Instant::now());
        let clock = manual.clock();
        let (mut visual, view) = Mount::components(FIELD)
            .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
            .motion(policy)
            .with(move |view| view.runtime_clock(clock))
            .mount(cx);
        visual.update(|window, _| window.activate_window());
        dispatch(&mut visual, &view, by_test_id("verify"), "click").unwrap();
        refresh(&mut visual);
        visual
            .update(|_, cx| view.take_render_statistics(cx))
            .unwrap();
        manual.advance(Duration::from_millis(320));
        settle(&mut visual, Step::Refresh, 3);
        let gradient = |v: &mut VisualTestContext| {
            painted_quads(v)
                .into_iter()
                .filter(|q| {
                    q.background.as_solid().is_none() && (q.bounds.width - 48. * 0.38).abs() < 1.0
                })
                .map(|q| q.bounds)
                .collect::<Vec<_>>()
        };
        let before = gradient(&mut visual);
        assert_eq!(!before.is_empty(), policy == MotionPreference::Normal);
        manual.advance(Duration::from_millis(160));
        settle(&mut visual, Step::Refresh, 3);
        if policy == MotionPreference::Normal {
            assert_ne!(gradient(&mut visual), before);
        } else {
            assert!(gradient(&mut visual).is_empty());
        }
        let frames = visual
            .update(|_, cx| view.take_render_statistics(cx))
            .unwrap();
        assert_eq!(
            (frames.polls, frames.commits),
            (0, 0),
            "moving the native sweep must not run Rhai"
        );
        dispatch(&mut visual, &view, by_test_id("success"), "click").unwrap();
        settle(&mut visual, Step::Refresh, 3);
        assert!(gradient(&mut visual).is_empty());
        assert_eq!(last_error(&mut visual, &view), None);
    }
}
