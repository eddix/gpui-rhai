use std::time::Duration;

use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px, size};
use gpui_rhai::{ScriptViewHandle, UiValue};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Mount, bounds, glyph_rects_with_ink, snapshot};

fn tick(v: &mut VisualTestContext, view: &ScriptViewHandle, ms: u64) {
    v.executor().advance_clock(Duration::from_millis(ms));
    ready(v, view);
}

fn used(v: &mut VisualTestContext, view: &ScriptViewHandle, count: i64) {
    assert_eq!(
        node(v, view, "progressbar", "Token usage").value,
        Some(UiValue::Integer(count))
    );
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn context_window_modern_summary_aligns_painted_digits(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("context_window_summary_baseline"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let source = r#"
        import "patterns/context_window" as context;
        fn view(ctx){context::ContextWindow(#{key:"capacity",context:"capacity",revision:0,limit:100000,
            cells:48,parts:[#{id:"chat",label:"Conversation",tokens:91000}],
            part_styles:#{big:style().text_color(rgb(0x96204c)),of:style().text_color(rgb(0x08774a)),
                warn:style().text_color(rgb(0x3115ac))}}).with_style(style().width(px(WIDTH)))}
    "#;
    for (theme, width) in [
        (gpui_rhai_registry::MODERN_LIGHT_THEME, "360"),
        (gpui_rhai_registry::MODERN_DARK_THEME, "260"),
    ] {
        let (mut v, view) = Mount::new(source.replace("WIDTH", width))
            .registry("patterns/context_window")
            .theme(theme)
            .bundled_assets()
            .motion(gpui_rhai::MotionPreference::None)
            .mount(&mut real);
        ready(&mut v, &view);
        let summary = bounds(&mut v, &view, "progressbar", "Token usage");
        let bottom = |v: &mut VisualTestContext, ink: u32, index: usize, expected: usize| {
            let mut glyphs = glyph_rects_with_ink(v, summary, ink);
            // Core Text's zero-ink spaces can still allocate an atlas tile.
            glyphs.retain(|glyph| glyph.height > 3.);
            glyphs.sort_by(|a, b| a.x.total_cmp(&b.x));
            assert_eq!(glyphs.len(), expected, "{width}, {ink:08x}: {glyphs:?}");
            glyphs[index].bottom()
        };
        // The second glyph of “91K” and “91%”, and the third non-space glyph
        // of “of 100K tokens” are all the same digit. Compare those, avoiding
        // a false difference from ascenders, punctuation, or descenders.
        let big = bottom(&mut v, 0x96204cff, 1usize, 3);
        let capacity = bottom(&mut v, 0x08774aff, 2usize, 12);
        let percent = bottom(&mut v, 0x3115acff, 1usize, 3);
        assert!(
            (big - capacity).abs() <= 1. && (big - percent).abs() <= 1.,
            "width {width}: digit bottoms {big}, {capacity}, {percent}"
        );
    }
}

#[gpui::test]
fn context_window_keeps_the_designs_distinct_summary_line_boxes(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "context_window", "modern-light");
    v.simulate_resize(size(px(1300.), px(1060.)));
    ready(&mut v, &view);
    let summary = node(&mut v, &view, "progressbar", "Token usage");
    let modern = summary.geometry.unwrap().visual;
    assert!((modern.height - 30.).abs() <= 0.5, "{modern:?}");
    used(&mut v, &view, 91_000);
    v.update(|_, cx| assert!(view.select_theme("Clean", "Dark", cx).unwrap()));
    ready(&mut v, &view);
    let clean = node(&mut v, &view, "progressbar", "Token usage");
    assert_eq!(clean.id, summary.id);
    let clean = clean.geometry.unwrap().visual;
    assert!((clean.height - 32.).abs() <= 0.5, "{clean:?}");
    used(&mut v, &view, 91_000);
}

#[gpui::test]
fn context_window_accepted_item_totals_and_compaction_wait_for_the_host(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "context_window", "modern-light");
    v.simulate_resize(size(px(1300.), px(1060.)));
    ready(&mut v, &view);
    used(&mut v, &view, 91_000);
    node(&mut v, &view, "status", "9K tokens left");

    click(&mut v, &view, "button", "Refuse next change");
    click(&mut v, &view, "button", "Files");
    let item = node(&mut v, &view, "listitem", "Change log");
    assert_eq!(item.description, "9,000 tokens");
    click(&mut v, &view, "button", "Remove Change log");
    used(&mut v, &view, 91_000);
    node(&mut v, &view, "status", "Waiting for context approval");
    tick(&mut v, &view, 410);
    node(&mut v, &view, "status", "Context change refused");
    node(&mut v, &view, "button", "Remove Change log");
    used(&mut v, &view, 91_000);

    click(&mut v, &view, "button", "Remove Change log");
    tick(&mut v, &view, 410);
    node(&mut v, &view, "status", "Accepted remove at revision 1");
    used(&mut v, &view, 82_000);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("listitem", "Change log")
            .next()
            .is_none()
    );
    assert_eq!(node(&mut v, &view, "button", "Files").expanded, Some(true));

    click(&mut v, &view, "button", "Compact Conversation");
    used(&mut v, &view, 82_000);
    node(&mut v, &view, "status", "Compacting");
    tick(&mut v, &view, 1030);
    node(&mut v, &view, "status", "Accepted compact at revision 2");
    node(&mut v, &view, "status", "Saved 27K tokens");
    used(&mut v, &view, 55_000);
    assert_eq!(
        node(&mut v, &view, "button", "Compact Conversation").description,
        "Compacted"
    );
    click(&mut v, &view, "button", "New chat");
    node(&mut v, &view, "status", "New conversation requested");
    used(&mut v, &view, 55_000);
}

#[gpui::test]
fn context_window_layout_reflows_cells_and_keyboard_attention_tracks_the_part(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "context_window", "clean-dark");
    v.simulate_resize(size(px(1300.), px(990.)));
    ready(&mut v, &view);
    let gap = |v: &mut VisualTestContext| {
        let count = bounds(v, &view, "progressbar", "Token usage");
        let list = bounds(v, &view, "list", "Context parts");
        list.y - (count.y + count.height)
    };
    let wide = gap(&mut v);
    assert!(
        wide > 35.,
        "the six-row field must take real layout space: {wide}"
    );
    let summary = bounds(&mut v, &view, "progressbar", "Token usage");
    // The first occupied mosaic tile is below the figure; hover the actual
    // native hit area, not a row or a synthetic callback.
    v.simulate_mouse_move(
        point(
            px((summary.x + 4.) as f32),
            px((summary.y + summary.height + 14.) as f32),
        ),
        None,
        Modifiers::none(),
    );
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "progressbar", "Token usage").description,
        "System prompt: 8,000 tokens"
    );
    click(&mut v, &view, "button", "Narrow card");
    let narrow = gap(&mut v);
    assert!(narrow > wide + 5., "wide: {wide}; narrow: {narrow}");
    let board = bounds(&mut v, &view, "group", "Context window");
    assert!(board.width <= 262., "{board:?}");
    click(&mut v, &view, "button", "Files");
    v.simulate_mouse_move(point(px(8.), px(8.)), None, Modifiers::none());
    ready(&mut v, &view);
    v.simulate_keystrokes("up");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "progressbar", "Token usage").description,
        "Tools: 11,000 tokens"
    );
    v.update(|_, cx| assert!(view.select_theme("Modern", "Light", cx).unwrap()));
    ready(&mut v, &view);
    v.simulate_keystrokes("up");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "progressbar", "Token usage").description,
        "System prompt: 8,000 tokens"
    );
}

#[gpui::test]
fn context_window_reports_overflow_and_missing_limit_without_inventing_capacity(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "context_window", "clean-light");
    v.simulate_resize(size(px(1290.), px(940.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Lower limit");
    node(&mut v, &view, "status", "21K tokens over the limit");
    used(&mut v, &view, 70_000);
    let usage = node(&mut v, &view, "progressbar", "Token usage");
    assert_eq!(usage.description, "91,000 of 70,000 tokens used");
    click(&mut v, &view, "button", "Remove limit");
    assert_eq!(
        node(&mut v, &view, "status", "Token usage").description,
        "91,000 tokens used; limit unavailable"
    );
    node(&mut v, &view, "status", "No token limit available");
}
#[gpui::test]
#[cfg(target_os = "macos")]
fn context_window_footer_presents_preset_ink_and_a_bounded_action(cx: &mut TestAppContext) {
    use crate::support::{PresetRisks, glyphs_with_ink, snapshot};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("context_window_footer_paint"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "context_window", "modern-light");
    v.simulate_resize(size(px(1250.), px(950.)));
    ready(&mut v, &view);
    let id = node(&mut v, &view, "button", "New chat").id;
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    for (family, appearance, note_line, button_height, note_ink, button_ink) in [
        ("Modern", "Light", 20., 26., 0x8f8f8fff, 0x404040ff),
        ("Modern", "Dark", 20., 26., 0x8b8b92ff, 0xc8c8ccff),
        ("Clean", "Light", 16., 28., 0x00000073, 0x000000ff),
        ("Clean", "Dark", 16., 28., 0xffffff75, 0xffffffff),
    ] {
        v.update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        ready(&mut v, &view);
        let status = snapshot(&mut v, &view)
            .nodes()
            .find(|n| n.role == "status" && n.name.ends_with("tokens left"))
            .unwrap()
            .geometry
            .unwrap()
            .visual;
        let container = bounds(&mut v, &view, "group", "Context window");
        let button = bounds(&mut v, &view, "button", "New chat");
        assert!(
            (status.height - note_line).abs() < 0.75,
            "{family}/{appearance}: {status:?}"
        );
        assert!(
            (button.height - button_height).abs() < 0.75 && button.width <= container.width / 2.,
            "{family}/{appearance}: {button:?}"
        );
        assert!(
            glyphs_with_ink(&mut v, status, note_ink) >= 8,
            "{family}/{appearance}: note ink"
        );
        assert!(
            glyphs_with_ink(&mut v, button, button_ink) >= 5,
            "{family}/{appearance}: action ink"
        );
        assert_eq!(node(&mut v, &view, "button", "New chat").id, id);
        if appearance == "Light" {
            let (stock, findings) = v.update(|_, cx| {
                (
                    view.theme_snapshot(cx).unwrap().variant,
                    view.composition_audit_with(&rules, cx).unwrap(),
                )
            });
            let suffixes = if family == "Clean" {
                &[
                    "/capacity/footer/note",
                    "/capacity/card/summary/number:percent",
                ][..]
            } else {
                &["/capacity/footer/note"][..]
            };
            for suffix in suffixes {
                let finding = findings
                    .iter()
                    .find(|f| f.path.ends_with(suffix))
                    .unwrap_or_else(|| panic!("{family}: expected warning {suffix}"));
                assert!(risks.record(&stock, finding), "{finding:?}");
                let mut wrong = finding.clone();
                wrong.contrast.as_mut().unwrap().background_srgb = Some([0.8; 3]);
                assert!(!risks.record(&stock, &wrong));
            }
        }
    }
    risks.assert_reported();
    click(&mut v, &view, "button", "New chat");
    node(&mut v, &view, "status", "New conversation requested");
}
