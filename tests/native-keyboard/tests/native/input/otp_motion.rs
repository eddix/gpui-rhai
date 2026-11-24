#[cfg(feature = "vendor-gpui")]
use super::platform_input;
use super::{
    ClipboardItem, Duration, FIELD, Instant, ManualRuntimeClock, MotionPreference, Mount,
    ScriptViewHandle, Step, TestAppContext, VisualTestContext, by_test_id, cells, click_center,
    dispatch, field_box, last_error, painted_quads, refresh, settle, text_of,
};

const RING: u32 = 0xc11a_7fff;
const PILL: u32 = 0x3bdb_70ff;

#[gpui::test]
fn otp_input_single_cell_success_reserves_room_for_the_label(cx: &mut TestAppContext) {
    let source = animated_field().replace("length:6,", "length:1,success_label:\"Signed in now\",");
    let (mut visual, view) = super::mount(cx, &source, gpui_rhai_registry::MODERN_LIGHT_THEME);
    let editor = field_box(&mut visual, &view);
    let later = crate::support::node_by_test_id(&mut visual, &view, "verify")
        .geometry
        .unwrap()
        .visual;
    assert!(
        editor.width > 80.,
        "label gets its own constrained measure: {editor:?}"
    );
    assert_eq!(cells(&mut visual, &view, 48.).len(), 1);
    dispatch(&mut visual, &view, by_test_id("success"), "click").unwrap();
    refresh(&mut visual);
    let color: gpui::Hsla = gpui::rgba(PILL).into();
    let pill = painted_quads(&mut visual)
        .iter()
        .find(|q| q.background.as_solid() == Some(color))
        .unwrap()
        .bounds;
    assert!(pill.width > 80.);
    assert!(f64::from(pill.right()) < editor.x + editor.width + 1.);
    assert_eq!(
        crate::support::node_by_test_id(&mut visual, &view, "verify")
            .geometry
            .unwrap()
            .visual,
        later
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

fn animated_field() -> String {
    FIELD.replace(
        "separator:true,status:",
        r#"
        separator:true,part_styles:#{
            active:style().border(px(2)).border_color(rgb(0xc11a7f)).shadows([]),
            success:style().background(rgb(0x3bdb70)).shadows([])
        },status:"#,
    )
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn digit_blurs(visual: &mut VisualTestContext) -> usize {
    fn count(scene: &gpui::Scene) -> usize {
        let active: usize = scene
            .isolated_groups
            .iter()
            .map(|group| usize::from(group.source_blur_sigma() > 0.) + count(group.source()))
            .sum();
        #[cfg(feature = "gpui-fast")]
        let active = active
            + scene
                .layers
                .frames
                .iter()
                .filter_map(|layer| layer.content.scene())
                .map(count)
                .sum::<usize>();
        active
    }
    visual.update(|window, _| window.visual_probe_with_painted_scene(count))
}

fn ring_x(visual: &mut VisualTestContext) -> f32 {
    let ink: gpui::Hsla = gpui::rgba(RING).into();
    painted_quads(visual)
        .iter()
        .find(|q| q.border_color == ink && q.border_widths[0] > 0.)
        .expect("visible ring from native paint")
        .bounds
        .x
}

fn manual_view(
    cx: &mut TestAppContext,
    policy: MotionPreference,
) -> (VisualTestContext, ScriptViewHandle, ManualRuntimeClock) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let runtime = clock.clock();
    let (mut visual, view) = Mount::components(animated_field())
        .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
        .motion(policy)
        .with(move |view| view.runtime_clock(runtime))
        .mount(cx);
    visual.update(|window, _| window.activate_window());
    refresh(&mut visual);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view, clock)
}

fn animate(visual: &mut VisualTestContext, clock: &ManualRuntimeClock, ms: u64) {
    clock.advance(Duration::from_millis(ms));
    settle(visual, Step::Refresh, 4);
}

fn caret_alpha(visual: &mut VisualTestContext, editor: gpui_rhai::GeometryBounds) -> Option<f32> {
    painted_quads(visual)
        .iter()
        .find(|quad| {
            (quad.bounds.width - 1.).abs() < 0.1
                && (f64::from(quad.bounds.height) - editor.height * 0.4).abs() < 0.6
                && f64::from(quad.bounds.y) >= editor.y
        })
        .and_then(|quad| quad.background.as_solid())
        .map(|color| color.a)
}

#[gpui::test]
fn otp_input_caret_blinks_only_while_active_and_respects_live_motion_settings(
    cx: &mut TestAppContext,
) {
    let (mut visual, view, clock) = manual_view(cx, MotionPreference::Normal);
    let editor = field_box(&mut visual, &view);
    assert_eq!(caret_alpha(&mut visual, editor), None);
    click_center(&mut visual, editor);
    refresh(&mut visual);
    #[cfg(feature = "vendor-gpui")]
    let geometry = platform_input(&mut visual, |handler| handler.bounds_for_range(0..0));
    assert_eq!(caret_alpha(&mut visual, editor), Some(1.));
    visual
        .update(|_, app| view.take_render_statistics(app))
        .unwrap();
    animate(&mut visual, &clock, 550);
    assert_eq!(caret_alpha(&mut visual, editor), None);
    #[cfg(feature = "vendor-gpui")]
    assert_eq!(
        platform_input(&mut visual, |handler| handler.bounds_for_range(0..0)),
        geometry
    );
    let statistics = visual
        .update(|_, app| view.take_render_statistics(app))
        .unwrap();
    assert_eq!((statistics.polls, statistics.commits), (0, 0));
    visual
        .update(|_, app| view.set_motion_preference(MotionPreference::Reduced, app))
        .unwrap();
    refresh(&mut visual);
    animate(&mut visual, &clock, 300);
    assert_eq!(
        caret_alpha(&mut visual, editor),
        Some(1.),
        "reduced motion must not leave the caret at the blink midpoint"
    );
    visual
        .update(|_, app| view.set_motion_preference(MotionPreference::Normal, app))
        .unwrap();
    refresh(&mut visual);
    animate(&mut visual, &clock, 550);
    assert_eq!(caret_alpha(&mut visual, editor), None);
    animate(&mut visual, &clock, 550);
    assert_eq!(caret_alpha(&mut visual, editor), Some(1.));
    visual.deactivate_window();
    refresh(&mut visual);
    assert_eq!(caret_alpha(&mut visual, editor), None);
    animate(&mut visual, &clock, 550);
    assert_eq!(caret_alpha(&mut visual, editor), None);
    assert_eq!(last_error(&mut visual, &view), None);
}

fn error_step(visual: &mut VisualTestContext, clock: &ManualRuntimeClock, ms: u64) {
    clock.advance(Duration::from_millis(ms));
    visual.executor().advance_clock(Duration::from_millis(ms));
    settle(visual, Step::Refresh, 4);
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn otp_input_discard_staggers_without_writing_or_clearing_a_new_attempt(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut fonts = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("otp_discard"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for replace in [false, true] {
        let (mut visual, view, clock) = manual_view(&mut fonts, MotionPreference::Normal);
        let editor = field_box(&mut visual, &view);
        let last_cell = *cells(&mut visual, &view, 48.).last().expect("last slot");
        click_center(&mut visual, editor);
        visual.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string("123456".into())));
        visual.simulate_keystrokes(&crate::support::primary_keys("secondary-v"));
        refresh(&mut visual);
        animate(&mut visual, &clock, 600);
        let previous = text_of(&mut visual, &view, "changes");
        dispatch(&mut visual, &view, by_test_id("fail"), "click").unwrap();
        refresh(&mut visual);
        error_step(&mut visual, &clock, 700);
        assert_eq!(digit_blurs(&mut visual), 0);
        error_step(&mut visual, &clock, 36);
        assert_eq!(digit_blurs(&mut visual), 1);
        let centers = visual.update(|window, _| {
            window.visual_probe_with_painted_scene(|scene| {
                scene
                    .isolated_groups
                    .iter()
                    .filter(|group| group.source_blur_sigma() > 0.)
                    .map(|group| group.content_mask.bounds.center().x.0 / window.scale_factor())
                    .collect::<Vec<_>>()
            })
        });
        assert!(
            centers
                .iter()
                .any(|x| (*x - last_cell.x - last_cell.width / 2.).abs() < 1.),
            "the first departing glyph must be in the last cell: {centers:?}"
        );
        error_step(&mut visual, &clock, 21);
        assert_eq!(digit_blurs(&mut visual), 2);
        assert_eq!(text_of(&mut visual, &view, "code"), "123456");
        assert_eq!(text_of(&mut visual, &view, "changes"), previous);
        if replace {
            dispatch(&mut visual, &view, by_test_id("replace"), "click").unwrap();
            refresh(&mut visual);
        }
        error_step(&mut visual, &clock, 1200);
        assert_eq!(digit_blurs(&mut visual), 0);
        if replace {
            assert_eq!(text_of(&mut visual, &view, "code"), "653401");
            assert_eq!(text_of(&mut visual, &view, "changes"), previous);
        } else {
            assert_eq!(text_of(&mut visual, &view, "code"), "");
            assert_eq!(text_of(&mut visual, &view, "changes"), "2");
        }
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn otp_input_staggers_glyphs_and_retargets_one_ring_without_moving_the_input(
    cx: &mut TestAppContext,
) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut native_fonts = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("otp_native_digits"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut visual, view, clock) = manual_view(&mut native_fonts, MotionPreference::Normal);
    let editor = field_box(&mut visual, &view);
    let cells_before = cells(&mut visual, &view, 48.);
    click_center(&mut visual, editor);
    refresh(&mut visual);
    let start = ring_x(&mut visual);
    assert!((start - cells_before[0].x).abs() < 0.55);
    visual.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string("123".into())));
    visual.simulate_keystrokes(&crate::support::primary_keys("secondary-v"));
    refresh(&mut visual);
    assert_eq!(text_of(&mut visual, &view, "code"), "123");
    assert_eq!(digit_blurs(&mut visual), 0);
    visual
        .update(|_, cx| view.take_render_statistics(cx))
        .unwrap();
    animate(&mut visual, &clock, 30);
    let in_flight = ring_x(&mut visual);
    assert!(in_flight > start + 1. && in_flight < cells_before[3].x - 1.);
    assert_eq!(digit_blurs(&mut visual), 1);
    animate(&mut visual, &clock, 27);
    assert_eq!(digit_blurs(&mut visual), 2);
    let before_turn = ring_x(&mut visual);
    let no_script = visual
        .update(|_, cx| view.take_render_statistics(cx))
        .unwrap();
    assert_eq!((no_script.polls, no_script.commits), (0, 0));
    visual.simulate_keystrokes("home");
    refresh(&mut visual);
    assert!(
        (ring_x(&mut visual) - before_turn).abs() < 0.55,
        "retarget starts at the presented position"
    );
    assert_eq!(
        platform_input(&mut visual, |handler| handler
            .selected_text_range(false)
            .unwrap()
            .range),
        0..1
    );
    animate(&mut visual, &clock, 400);
    assert_eq!(digit_blurs(&mut visual), 0);
    assert!((ring_x(&mut visual) - start).abs() < 0.55);
    assert_eq!(field_box(&mut visual, &view), editor);
    assert_eq!(last_error(&mut visual, &view), None);
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn otp_input_success_morph_uses_the_cell_envelope_and_preserves_following_geometry(
    cx: &mut TestAppContext,
) {
    for policy in [MotionPreference::Normal, MotionPreference::None] {
        let (mut visual, view, clock) = manual_view(cx, policy);
        let editor = field_box(&mut visual, &view);
        click_center(&mut visual, editor);
        let previous_focus = visual
            .update(|window, cx| window.focused(cx))
            .expect("OTP focused");
        visual.simulate_input("123456");
        dispatch(&mut visual, &view, by_test_id("verify"), "click").unwrap();
        refresh(&mut visual);
        animate(&mut visual, &clock, 480);
        let later = crate::support::node_by_test_id(&mut visual, &view, "verify")
            .geometry
            .unwrap()
            .visual;
        let capsule = |visual: &mut VisualTestContext| {
            let color: gpui::Hsla = gpui::rgba(PILL).into();
            painted_quads(visual)
                .into_iter()
                .find(|q| q.background.as_solid() == Some(color))
                .map(|q| q.bounds)
        };
        dispatch(&mut visual, &view, by_test_id("success"), "click").unwrap();
        refresh(&mut visual);
        let early = capsule(&mut visual);
        if policy == MotionPreference::Normal {
            assert!(early.is_none());
        } else {
            assert!(early.is_some());
        }
        assert!(
            visual
                .update(|window, _| window.visual_probe_take_input())
                .is_none()
        );
        animate(&mut visual, &clock, 285);
        let intermediate = capsule(&mut visual).expect("success surface");
        animate(&mut visual, &clock, 660);
        let settled = capsule(&mut visual).expect("terminal success surface");
        if policy == MotionPreference::Normal {
            assert!(
                intermediate.width > settled.width + 30.,
                "{intermediate:?} to {settled:?}"
            );
            assert!(intermediate.height > settled.height + 1.);
        } else {
            assert_eq!(intermediate, settled);
        }
        assert!((settled.height - 36.).abs() < 0.55);
        assert!(
            (f64::from(settled.x + settled.width / 2.) - (editor.x + editor.width / 2.)).abs() < 1.
        );
        assert_eq!(
            crate::support::node_by_test_id(&mut visual, &view, "verify")
                .geometry
                .unwrap()
                .visual,
            later
        );
        assert_eq!(last_error(&mut visual, &view), None);
        visual.simulate_keystrokes("tab");
        refresh(&mut visual);
        let current = visual.update(|window, cx| window.focused(cx));
        assert!(
            current.is_some() && current != Some(previous_focus),
            "Tab skips the confirmed editor"
        );
    }
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
