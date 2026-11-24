use gpui::{Modifiers, ScrollDelta, ScrollEvent, TestAppContext, point, px, size};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Mount, bounds, center, glyphs_with_ink, snapshot};

#[gpui::test]
#[cfg(target_os = "macos")]
fn memory_panel_metadata_separator_has_its_own_paint_and_reading_scale(cx: &mut TestAppContext) {
    use crate::support::PresetRisks;
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    use gpui_rhai::{AuditRules, GeometryBounds, UiValue};

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("memory_separator_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "memory_panel", "clean-light");
    v.simulate_resize(size(px(1440.), px(1250.)));
    ready(&mut v, &view);
    let rules = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    for (family, appearance, ink, separator, height) in [
        ("Clean", "Light", 0x00000073, 0x00000045, 16.),
        ("Clean", "Dark", 0xffffff75, 0xffffff4d, 16.),
        ("Modern", "Light", 0x8f8f8fff, 0x8f8f8fff, 20.),
        ("Modern", "Dark", 0x8b8b92ff, 0x8b8b92ff, 20.),
    ] {
        v.update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        for (scale, ratio) in [("100", 1.), ("200", 2.)] {
            super::acceptance::action(
                &mut v,
                &view,
                "gallery.set_scale",
                UiValue::String(scale.to_owned()),
            );
            ready(&mut v, &view);
            let source = bounds(&mut v, &view, "text", "From a chat");
            let date = bounds(&mut v, &view, "text", "Mar 7");
            assert!(
                (date.height - height * ratio).abs() < 0.75,
                "{family}/{appearance}/{scale}: wrong metadata line: {date:?}"
            );
            assert!(glyphs_with_ink(&mut v, date, ink) >= 4);
            let x = source.x + source.width;
            let gap = GeometryBounds {
                x,
                y: date.y,
                width: date.x - x,
                height: date.height,
            };
            assert!(
                gap.width > 0.,
                "{family}/{appearance}/{scale}: {source:?} vs {date:?}"
            );
            assert_eq!(
                glyphs_with_ink(&mut v, gap, separator),
                1,
                "{family}/{appearance}/{scale}: {gap:?}"
            );
            assert!(!snapshot(&mut v, &view).nodes().any(|n| n.name == "·"));
            if scale == "100" && appearance == "Light" {
                let (theme, findings) = v.update(|_, cx| {
                    (
                        view.theme_snapshot(cx).unwrap().variant,
                        view.composition_audit_with(&rules, cx).unwrap(),
                    )
                });
                for id in ["rust", "tea", "orbit"] {
                    let matching = format!("/item:{id}/0/meta/0/description/separator");
                    let actual = findings
                        .iter()
                        .find(|f| f.path.ends_with(&matching))
                        .unwrap_or_else(|| {
                            panic!("{family}: separator must remain reportable: {matching}")
                        });
                    assert!(risks.record(&theme, actual), "{actual:?}");
                    let mut other = actual.clone();
                    other.path.push_str("/unreviewed");
                    assert!(!risks.record(&theme, &other));
                }
            }
        }
    }
    risks.assert_reported();
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn memory_panel_footer_action_keeps_its_preset_size_and_body_ink(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("memory_footer_action_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "memory_panel", "modern-light");
    for (family, appearance, ink, height) in [
        ("Modern", "Light", 0x404040ff, 26.),
        ("Modern", "Dark", 0xc8c8ccff, 26.),
        ("Clean", "Light", 0x0000009c, 28.),
        ("Clean", "Dark", 0xffffff9e, 28.),
    ] {
        v.update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        ready(&mut v, &view);
        let action = bounds(&mut v, &view, "button", "Manage");
        assert!(
            glyphs_with_ink(&mut v, action, ink) >= 5,
            "{family}/{appearance}: {action:?}"
        );
        assert!(
            (action.height - height).abs() <= 1.,
            "{family}/{appearance}: {action:?}"
        );
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn horizontal_fades(visual: &mut gpui::VisualTestContext) -> usize {
    visual.update(|window, _| {
        window.visual_probe_with_painted_scene(|scene| {
            let mut n = 0;
            crate::support::walk_live_paint(
                scene,
                gpui::TransformationMatrix::default(),
                &mut |scene, _| {
                    n += scene
                        .isolated_groups
                        .iter()
                        .filter(|group| {
                            let ramps = group.alpha_ramps();
                            ramps[4] == 1. && ramps[1] + ramps[3] > 0.
                        })
                        .count();
                },
            );
            n
        })
    })
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn memory_panel_clean_counts_follow_selection_without_changing_idle_ink(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("memory_filter_count_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, muted, body) in [
        ("clean-light", 0x00000073, 0x0000009c),
        ("clean-dark", 0xffffff75, 0xffffff9e),
    ] {
        let (mut v, view) = gallery(&mut real, "memory_panel", theme);
        v.simulate_resize(size(px(1300.), px(970.)));
        ready(&mut v, &view);
        let work = bounds(&mut v, &view, "radio", "Work");
        assert_eq!(
            glyphs_with_ink(&mut v, work, muted),
            1,
            "{theme}: idle count"
        );
        click(&mut v, &view, "radio", "Work");
        let work = bounds(&mut v, &view, "radio", "Work");
        assert_eq!(
            glyphs_with_ink(&mut v, work, muted),
            0,
            "{theme}: selected count"
        );
        assert_eq!(
            glyphs_with_ink(&mut v, work, body),
            1,
            "{theme}: selected count"
        );
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn memory_panel_keeps_preset_filter_metrics_and_places_actions_below_the_fact(
    cx: &mut TestAppContext,
) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("memory_panel_text_roles"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, text, chip_height, switch_width, switch_paint) in [
        ("modern-light", 0x404040ff, 26., 28., 0x171717ff),
        ("modern-dark", 0xc8c8ccff, 26., 28., 0xedededff),
        ("clean-light", 0x0000009c, 28., 26., 0x0060e2c3),
        ("clean-dark", 0xffffff9e, 28., 26., 0x4fb0feff),
    ] {
        let (mut v, view) = gallery(&mut real, "memory_panel", theme);
        v.simulate_resize(size(px(1300.), px(970.)));
        ready(&mut v, &view);
        let chip = bounds(&mut v, &view, "radio", "Work");
        assert!(
            (chip.height - chip_height).abs() <= 0.5,
            "{theme}: {chip:?}"
        );
        assert!(
            glyphs_with_ink(&mut v, chip, text) >= 4,
            "{theme}: category text must use body ink"
        );
        let toggle = bounds(&mut v, &view, "switch", "Use memory");
        let quad = crate::support::filled_including_rounded_clips(&mut v, switch_paint)
            .into_iter()
            .map(|q| q.visible())
            .find(|q| {
                (q.width - switch_width).abs() <= 0.5
                    && (q.height - 16.).abs() <= 0.5
                    && f64::from(q.y) >= toggle.y
                    && f64::from(q.bottom()) <= toggle.y + toggle.height
                    && f64::from(q.x) > toggle.x + toggle.width / 2.
                    && f64::from(q.right()) <= toggle.x + toggle.width
            });
        assert!(
            quad.is_some(),
            "{theme}: the exact active track trails its label"
        );
        let fact = bounds(&mut v, &view, "text", "Builds Rust tools");
        v.simulate_mouse_move(center(fact), None, Modifiers::none());
        ready(&mut v, &view);
        let edit = bounds(&mut v, &view, "button", "Edit memory: Builds Rust tools");
        let date = bounds(&mut v, &view, "text", "Mar 7");
        assert!(
            edit.y + edit.height / 2. >= fact.y + fact.height,
            "{theme}: {fact:?}, {edit:?}"
        );
        assert!(
            (edit.y + edit.height / 2. - date.y - date.height / 2.).abs() <= 1.,
            "{theme}: {date:?} {edit:?}"
        );
        let list = bounds(&mut v, &view, "list", "Saved memories");
        let add = bounds(&mut v, &view, "button", "Add memory");
        assert!(
            (list.width - 12. - add.width).abs() <= 0.5,
            "{theme}: {list:?}, {add:?}"
        );
    }
}

#[gpui::test]
fn memory_panel_keyboard_reveals_an_offscreen_category_without_wrapping(cx: &mut TestAppContext) {
    use gpui_rhai::{MotionPreference, UiValue};
    let script = r#"
import "patterns/memory_panel" as memory;
fn view(ctx) {
    let names=["Development","Services","Tool catalog","System choices","Long projects","Travel plans"];
    let records=[];
    for i in 0..names.len{records.push(#{id:i.to_string(),text:names[i]+" memory",category:names[i]});}
    memory::MemoryPanel(#{key:"small",workspace:"settings",memories:records,categories:names})
        .with_style(style().width(px(250)))
}
"#;
    let (mut v, view) = Mount::new(script)
        .registry("patterns/memory_panel")
        .theme(gpui_rhai_registry::CLEAN_DARK_THEME)
        .motion(MotionPreference::None)
        .mount(cx);
    ready(&mut v, &view);
    let all = bounds(&mut v, &view, "radio", "All");
    let end = node(&mut v, &view, "radio", "Travel plans");
    let before = end.geometry.unwrap();
    assert!((before.visual.y - all.y).abs() <= 0.5);
    assert!(before.clip.is_some_and(|g| g.width < 0.5), "{before:?}");
    #[cfg(feature = "vendor-gpui")]
    assert_eq!(
        horizontal_fades(&mut v),
        1,
        "trailing content is not yet visible"
    );
    click(&mut v, &view, "radio", "All");
    v.simulate_keystrokes("end");
    ready(&mut v, &view);
    let revealed = node(&mut v, &view, "radio", "Travel plans");
    let after = revealed.geometry.unwrap();
    assert_eq!(revealed.id, end.id);
    assert_eq!(revealed.checked, Some(UiValue::Bool(true)));
    assert!((after.visual.y - all.y).abs() <= 0.5);
    let search = bounds(&mut v, &view, "button", "Search memories");
    assert!(after.visual.x < before.visual.x);
    assert!(
        after.visual.x + after.visual.width <= search.x,
        "{after:?}, {search:?}"
    );
    assert!(
        after
            .clip
            .is_none_or(|g| g.width >= after.visual.width - 0.5),
        "{after:?}"
    );
    #[cfg(feature = "vendor-gpui")]
    assert_eq!(
        horizontal_fades(&mut v),
        0,
        "last chip is fully visible, without a paint mask"
    );
    v.simulate_keystrokes("home");
    ready(&mut v, &view);
    let returned = node(&mut v, &view, "radio", "All");
    assert_eq!(returned.checked, Some(UiValue::Bool(true)));
    assert!((returned.geometry.unwrap().visual.x - all.x).abs() <= 0.5);
    #[cfg(feature = "vendor-gpui")]
    assert_eq!(
        horizontal_fades(&mut v),
        1,
        "scrolling back restores the fade"
    );
    let strip = bounds(&mut v, &view, "radiogroup", "Memory categories");
    v.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
    for (dx, expected) in [(-900., 0), (900., 1)] {
        v.simulate_event(ScrollEvent {
            position: center(strip),
            delta: ScrollDelta::Pixels(point(px(dx), px(0.))),
            ..Default::default()
        });
        ready(&mut v, &view);
        #[cfg(feature = "vendor-gpui")]
        assert_eq!(horizontal_fades(&mut v), expected, "wheel delta {dx}");
        #[cfg(not(feature = "vendor-gpui"))]
        let _ = expected;
        assert_eq!(
            node(&mut v, &view, "radio", "All").checked,
            Some(UiValue::Bool(true))
        );
    }
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx).unwrap())
            .timings
            .is_empty(),
        "paint-only scroll boundary changes must not re-evaluate Rhai"
    );

    click(&mut v, &view, "button", "Add memory");
    let tree = snapshot(&mut v, &view);
    let group = tree
        .find_by_role_and_name("radiogroup", "New memory category")
        .next()
        .unwrap();
    let radios: Vec<_> = tree
        .nodes()
        .filter(|n| n.parent == Some(group.id) && n.role == "radio")
        .collect();
    assert_eq!(radios.len(), 6);
    let top = radios[0].geometry.unwrap().visual.y;
    assert!(
        radios
            .iter()
            .all(|r| (r.geometry.unwrap().visual.y - top).abs() <= 0.5),
        "Clean's new-memory categories are another one-line port"
    );
    let tail = radios
        .iter()
        .find(|r| r.name == "Travel plans")
        .unwrap()
        .geometry
        .unwrap();
    assert!(tail.clip.is_some_and(|g| g.width < 0.5), "{tail:?}");
}

#[gpui::test]
fn memory_panel_only_offers_undo_for_a_confirmed_removal(cx: &mut TestAppContext) {
    for theme in ["modern-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "memory_panel", theme);
        v.simulate_resize(size(px(1280.), px(950.)));
        ready(&mut v, &view);
        node(&mut v, &view, "list", "Saved memories");
        click(&mut v, &view, "button", "Refuse next change");
        click(&mut v, &view, "button", "Forget memory: Builds Rust tools");
        node(&mut v, &view, "status", "The Host refused the change");
        assert!(
            snapshot(&mut v, &view)
                .find_by_role_and_name("button", "Undo")
                .next()
                .is_none()
        );
        click(&mut v, &view, "button", "Forget memory: Builds Rust tools");
        node(&mut v, &view, "status", "The Host accepted: forget");
        assert!(
            snapshot(&mut v, &view)
                .find_by_role_and_name("button", "Edit memory: Builds Rust tools")
                .next()
                .is_none()
        );
        click(&mut v, &view, "button", "Undo");
        node(&mut v, &view, "status", "The Host accepted: restore");
        node(&mut v, &view, "button", "Edit memory: Builds Rust tools");
    }
}

#[gpui::test]
fn memory_panel_search_typing_and_controlled_toggle_preserve_records(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "memory_panel", "clean-light");
    v.simulate_resize(size(px(1280.), px(950.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Narrow memory");
    click(&mut v, &view, "button", "Search memories");
    node(&mut v, &view, "text_field", "Search memories");
    v.simulate_input("tea");
    ready(&mut v, &view);
    node(&mut v, &view, "button", "Edit memory: Prefers tea");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Edit memory: Builds Rust tools")
            .next()
            .is_none()
    );
    click(&mut v, &view, "button", "Close search");
    click(&mut v, &view, "button", "Resume memory: Prefers tea");
    node(&mut v, &view, "button", "Pause memory: Prefers tea");
    click(&mut v, &view, "switch", "Use memory");
    node(&mut v, &view, "status", "Memory disabled by Host");
    click(&mut v, &view, "switch", "Use memory");
    node(&mut v, &view, "status", "Memory enabled by Host");
    node(&mut v, &view, "button", "Edit memory: Builds Rust tools");
}

#[gpui::test]
fn memory_panel_adds_via_native_editor_and_can_switch_categories(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "memory_panel", "modern-dark");
    v.simulate_resize(size(px(1280.), px(950.)));
    ready(&mut v, &view);
    click(&mut v, &view, "radio", "Preferences");
    node(&mut v, &view, "button", "Edit memory: Prefers tea");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Edit memory: Builds Rust tools")
            .next()
            .is_none()
    );
    click(&mut v, &view, "button", "Add memory");
    click(&mut v, &view, "text_field", "New memory");
    v.simulate_input("  Prefers  natural light  ");
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "The Host accepted: add");
    node(
        &mut v,
        &view,
        "button",
        "Edit memory: Prefers natural light",
    );
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
