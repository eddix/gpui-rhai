use std::time::Duration;

use super::conversation::{click, gallery, node, ready};
use crate::support::{bounds, center, snapshot};
use gpui::{
    Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, point, px, size,
};
use gpui_rhai::{ScriptViewHandle, UiValue};

#[gpui::test]
fn scheduled_runs_card_marks_paint_on_the_card_paper_and_grips_are_not_text(
    cx: &mut TestAppContext,
) {
    use crate::support::live_fills;
    let (mut v, view) = gallery(cx, "scheduled_runs", "modern-dark");
    v.simulate_resize(size(px(1380.), px(980.)));
    ready(&mut v, &view);
    let task = bounds(&mut v, &view, "button", "Inspect Audit package");
    assert!(
        live_fills(&mut v, 0x1c1c1fff).iter().any(|quad| {
            (quad.width - 18.).abs() <= 0.5
                && (quad.height - 18.).abs() <= 0.5
                && f64::from(quad.x) >= task.x
                && f64::from(quad.right()) < task.x + task.width
                && f64::from(quad.y) >= task.y
                && f64::from(quad.bottom()) < task.y + task.height
        }),
        "{task:?}"
    );
    node(&mut v, &view, "button", "Drag Audit package");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("text", "⋮")
            .next()
            .is_none()
    );
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn scheduled_runs_date_ink_matches_the_header_inset_in_all_presets(cx: &mut TestAppContext) {
    use crate::support::{Mount, glyph_rects_with_ink};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("scheduled_runs_date_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, weekday, date) in [
        (
            gpui_rhai_registry::MODERN_LIGHT_THEME,
            0x8a8a8aff,
            0x6b6b6bff,
        ),
        (
            gpui_rhai_registry::MODERN_DARK_THEME,
            0xffffff73,
            0xffffff9e,
        ),
        (
            gpui_rhai_registry::CLEAN_LIGHT_THEME,
            0x00000073,
            0x0000009c,
        ),
        (gpui_rhai_registry::CLEAN_DARK_THEME, 0xffffff73, 0xffffff9e),
    ] {
        for (scale, factor) in [("100", 1.), ("150", 1.5)] {
            let source = r#"
                import "patterns/scheduled_runs" as week;
                fn view(ctx){
                    week::ScheduledRuns(#{key:"schedule",board:"schedule",revision:0,mode:"manual",
                        days:[#{id:"mon",label:"Mon",date:"23"}],periods:[#{id:"am",label:"07:00",limit:100}],
                        tasks:[],assignments:[]}).env(#{scale:"__SCALE__"})
                }
            "#.replace("__SCALE__",scale);
            let (mut v, view) = Mount::new(source)
                .registry("patterns/scheduled_runs")
                .theme(theme)
                .bundled_assets()
                .motion(gpui_rhai::MotionPreference::None)
                .mount(&mut real);
            v.simulate_resize(size(px(1100.), px(840.)));
            ready(&mut v, &view);
            let cell = bounds(&mut v, &view, "columnheader", "Mon");
            assert!(
                (cell.height - 26.0 * factor).abs() <= 0.5,
                "{scale}: {cell:?}"
            );
            let weekday = glyph_rects_with_ink(&mut v, cell, weekday);
            let date = glyph_rects_with_ink(&mut v, cell, date);
            assert_eq!(weekday.len(), 3, "{scale}: {cell:?}: {weekday:?}");
            assert_eq!(date.len(), 2, "{scale}: {cell:?}: {date:?}");
            let last = |glyphs: &[crate::support::Rect]| {
                glyphs
                    .iter()
                    .map(|g| f64::from(g.bottom()))
                    .reduce(f64::max)
                    .unwrap()
            };
            let a = last(&weekday) - cell.y;
            let b = last(&date) - cell.y;
            assert!(
                (a - b).abs() <= 1.5 * factor && a.max(b) <= 19.0 * factor + 0.5,
                "{scale}: weekday bottom={a}, date bottom={b}, header {cell:?}"
            );
        }
    }
}

#[gpui::test]
fn scheduled_runs_singular_task_count_uses_the_singular_label(cx: &mut TestAppContext) {
    use crate::support::Mount;
    let (mut v, view) = Mount::new(
        r#"
        import "patterns/scheduled_runs" as week;
        fn view(ctx){
            week::ScheduledRuns(#{key:"schedule",board:"schedule",revision:0,mode:"manual",
                days:[#{id:"mon",label:"Mon"}],periods:[#{id:"am",label:"07:00",limit:100}],
                tasks:[#{id:"print",title:"Print receipt",units:20}],assignments:[]})
        }
    "#,
    )
    .registry("patterns/scheduled_runs")
    .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
    .locale("zh_cn.rhai", gpui_rhai_registry::ZH_CN_LOCALE)
    .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
    .motion(gpui_rhai::MotionPreference::None)
    .mount(cx);
    ready(&mut v, &view);
    node(&mut v, &view, "text", "1 task");
    for (locale, label) in [("zh-CN", "1 项任务"), ("ar", "مهمة واحدة")] {
        assert!(v.update(|_, cx| view.select_locale(locale, cx)).unwrap());
        ready(&mut v, &view);
        node(&mut v, &view, "text", label);
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn scheduled_runs_expands_the_selected_name_without_replacing_its_avatar(cx: &mut TestAppContext) {
    use crate::support::{glyph_rects_with_ink, live_fills};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("scheduled_runs_crew_chrome"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (preset, ink, paper) in [
        ("modern-light", 0x171717ff, 0xffffffff),
        ("modern-dark", 0xffffffe6, 0x1c1c1fff),
        ("clean-light", 0x000000e0, 0xffffffff),
        ("clean-dark", 0xffffffe6, 0x1c1c1fff),
    ] {
        let (mut v, view) = gallery(&mut real, "scheduled_runs", preset);
        v.simulate_resize(size(px(1450.), px(1050.)));
        ready(&mut v, &view);
        let id = node(&mut v, &view, "button", "Atlas").id;
        let closed = bounds(&mut v, &view, "button", "Atlas");
        let cell = bounds(&mut v, &view, "gridcell", "Tue 07:00");
        assert!(
            (closed.width - 26.).abs() <= 0.5 && (closed.height - 26.).abs() <= 0.5,
            "{preset}: {closed:?}"
        );
        assert!((cell.height - 156.).abs() <= 1., "{preset}: {cell:?}");
        let grip = glyph_rects_with_ink(&mut v, closed, ink);
        assert_eq!(grip.len(), 1, "{preset}: {grip:?}");
        assert!(
            live_fills(&mut v, paper).into_iter().any(|bounds| {
                (f64::from(bounds.x) - closed.x).abs() <= 0.5
                    && (f64::from(bounds.y) - closed.y).abs() <= 0.5
                    && (f64::from(bounds.width) - closed.width).abs() <= 0.5
                    && (f64::from(bounds.height) - closed.height).abs() <= 0.5
            }),
            "{preset}: the agent should use its own card paper"
        );

        click(&mut v, &view, "button", "Atlas");
        let selected = node(&mut v, &view, "button", "Atlas");
        assert_eq!(selected.id, id);
        assert_eq!(selected.pressed, Some(true));
        let open = bounds(&mut v, &view, "button", "Atlas");
        let mut glyphs = glyph_rects_with_ink(&mut v, open, ink);
        glyphs.sort_by(|a, b| a.x.total_cmp(&b.x));
        assert!(
            open.width > closed.width + 20. && (open.height - closed.height).abs() <= 0.5,
            "{preset}: {closed:?} -> {open:?}"
        );
        assert!(
            glyphs.len() >= 6 && (glyphs[0].height - grip[0].height).abs() <= 1.,
            "{preset}: the mark and expanded name must both paint: {glyphs:?}"
        );

        click(&mut v, &view, "button", "Atlas");
        let again = node(&mut v, &view, "button", "Atlas");
        assert_eq!(again.id, id);
        assert_eq!(again.pressed, Some(false));
        assert!((bounds(&mut v, &view, "button", "Atlas").width - closed.width).abs() <= 0.5);
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
fn repeating_period_at(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    label: &str,
) -> Option<f32> {
    use crate::support::{logical_paint_rect, walk_live_paint};
    let area = bounds(v, view, "gridcell", label);
    v.update(|window, _| {
        let scale = window.scale_factor();
        window.visual_probe_with_painted_scene(|scene| {
            let mut period = None;
            walk_live_paint(
                scene,
                gpui::TransformationMatrix::unit(),
                &mut |scene, placement| {
                    for fill in &scene.layered_fills {
                        let rect = logical_paint_rect(fill.bounds, placement, scale);
                        if (f64::from(rect.x) - area.x).abs() > 1.
                            || (f64::from(rect.y) - area.y).abs() > 1.
                            || (f64::from(rect.width) - area.width).abs() > 1.
                        {
                            continue;
                        }
                        for layer in &*fill.layers {
                            if let Some(projected) = layer.repeat_period {
                                period = Some(
                                    projected.0
                                        / scale
                                        / layer.direction.x.hypot(layer.direction.y),
                                );
                            }
                        }
                    }
                },
            );
            period
        })
    })
}

fn advance(v: &mut VisualTestContext, view: &ScriptViewHandle) {
    v.executor().advance_clock(Duration::from_millis(390));
    ready(v, view);
}

fn held(v: &mut VisualTestContext, view: &ScriptViewHandle, at: &str, amount: i64) {
    assert_eq!(
        node(v, view, "progressbar", at).value,
        Some(UiValue::Integer(amount)),
        "{at}"
    );
}

fn drag_to(v: &mut VisualTestContext, view: &ScriptViewHandle, name: &str, target: &str) {
    let start = center(bounds(v, view, "button", name));
    let end = center(bounds(v, view, "group", target));
    drag_between(v, view, start, end);
}

fn drag_between(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    start: Point<Pixels>,
    end: Point<Pixels>,
) {
    v.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    v.simulate_mouse_move(
        point(start.x + px(12.), start.y + px(3.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    v.run_until_parked();
    v.simulate_mouse_move(end, MouseButton::Left, Modifiers::none());
    v.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    ready(v, view);
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn scheduled_runs_note_and_period_use_independent_preset_ink(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("scheduled_note_and_clock"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, note, time, date) in [
        ("modern-light", 0x8a8a8aff, 0xb5b5b5ff, 0x6b6b6bff),
        ("modern-dark", 0xffffff73, 0xffffff47, 0xffffff9e),
        ("clean-light", 0x00000073, 0x00000045, 0x0000009c),
        ("clean-dark", 0xffffff73, 0xffffff47, 0xffffff9e),
    ] {
        let (mut v, view) = gallery(&mut real, "scheduled_runs", theme);
        v.simulate_resize(size(px(1450.), px(1050.)));
        ready(&mut v, &view);
        let subtitle = bounds(&mut v, &view, "text", "5-hour sessions");
        let time_cell = bounds(&mut v, &view, "rowheader", "07:00");
        assert!(
            crate::support::glyphs_with_ink(&mut v, subtitle, note) >= 8,
            "{theme}: note ink"
        );
        assert!(
            crate::support::glyphs_with_ink(&mut v, time_cell, time) >= 4,
            "{theme}: time ink"
        );
        let column = bounds(&mut v, &view, "columnheader", "Tue");
        assert!(
            crate::support::glyphs_with_ink(&mut v, column, date) >= 2,
            "{theme}: date ink"
        );
        let rules = gpui_rhai::AuditRules::from_ids(["mixed-type-in-row"]).unwrap();
        let findings = v.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
        assert!(
            findings
                .iter()
                .all(|finding| !finding.path.contains("/days/")),
            "{theme}: the weekday annotation is not a peer of the date: {findings:?}"
        );
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn scheduled_runs_aligns_the_title_and_note_on_a_painted_baseline(cx: &mut TestAppContext) {
    use crate::support::{Mount, glyph_rects_with_ink};
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("scheduled_runs_heading_baseline"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let source = r#"
        import "patterns/scheduled_runs" as week;
        fn view(ctx){
            week::ScheduledRuns(#{key:"schedule",board:"schedule",revision:0,mode:"manual",
                days:[#{id:"mon",label:"Mon"}],periods:[#{id:"am",label:"07:00",limit:100}],
                tasks:[],assignments:[],title:"H",note:"HH",
                part_styles:#{title:style().text_color(rgb(0x96204c)),
                    note:style().text_color(rgb(0x08774a))}})
        }
    "#;
    let mixed_type = gpui_rhai::AuditRules::from_ids(["mixed-type-in-row"]).unwrap();
    for (theme, note_gap) in [
        (gpui_rhai_registry::MODERN_DARK_THEME, 4.),
        (gpui_rhai_registry::CLEAN_LIGHT_THEME, 4.),
    ] {
        let (mut v, view) = Mount::new(source)
            .registry("patterns/scheduled_runs")
            .theme(theme)
            .bundled_assets()
            .motion(gpui_rhai::MotionPreference::None)
            .mount(&mut real);
        ready(&mut v, &view);
        let title = bounds(&mut v, &view, "text", "H");
        let note = bounds(&mut v, &view, "text", "HH");
        assert!((title.height - note.height - note_gap).abs() <= 1.);
        let title_ink = glyph_rects_with_ink(&mut v, title, 0x96204cff);
        let note_ink = glyph_rects_with_ink(&mut v, note, 0x08774aff);
        assert_eq!(title_ink.len(), 1);
        assert_eq!(note_ink.len(), 2);
        assert!(
            (title_ink[0].bottom() - note_ink[0].bottom()).abs() <= 1.,
            "{title_ink:?} {note_ink:?}"
        );
        let findings = v.update(|_, cx| view.composition_audit_with(&mixed_type, cx).unwrap());
        assert!(
            !findings
                .iter()
                .any(|f| f.path.ends_with("/heading-text") || f.path.ends_with("/backlog-head")),
            "{findings:?}"
        );
    }
}

#[gpui::test]
fn scheduled_runs_backlog_keeps_its_header_inset_and_the_shared_card_pitch(
    cx: &mut TestAppContext,
) {
    for theme in ["modern-dark", "clean-light"] {
        let (mut v, view) = gallery(cx, "scheduled_runs", theme);
        v.simulate_resize(size(px(1340.), px(1040.)));
        ready(&mut v, &view);
        let stack = bounds(&mut v, &view, "group", "To schedule");
        let title = bounds(&mut v, &view, "text", "To schedule");
        let first = bounds(&mut v, &view, "button", "Inspect Audit package");
        let second = bounds(&mut v, &view, "button", "Inspect Sync media");
        assert!(
            (title.x - stack.x - 12.).abs() <= 0.5,
            "{theme}: 4px stack padding + 8px title inset: {stack:?} {title:?}"
        );
        assert!(
            (second.y - first.y - 68.).abs() <= 0.5,
            "{theme}: 64px card + 4px spacing: {first:?} {second:?}"
        );
        assert!(
            (first.x - second.x).abs() <= 0.5,
            "{theme}: cards share the same rail: {first:?} {second:?}"
        );
    }
}

#[gpui::test]
fn scheduled_runs_auto_proposal_is_atomic_and_manual_preserves_booked_tasks(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "scheduled_runs", "modern-light");
    v.simulate_resize(size(px(1340.), px(960.)));
    ready(&mut v, &view);
    let track = node(&mut v, &view, "radiogroup", "Scheduling");
    assert_eq!(track.orientation.as_deref(), Some("horizontal"));
    let auto = node(&mut v, &view, "radio", "Auto");
    let manual = node(&mut v, &view, "radio", "Manual");
    assert_eq!(auto.checked, Some(UiValue::Bool(false)));
    assert_eq!(manual.checked, Some(UiValue::Bool(true)));
    let track_bounds = bounds(&mut v, &view, "radiogroup", "Scheduling");
    let item = bounds(&mut v, &view, "radio", "Auto");
    assert!(
        (track_bounds.height - 32.0).abs() <= 0.5 && (item.height - 26.0).abs() <= 0.5,
        "{track_bounds:?} {item:?}"
    );
    click(&mut v, &view, "button", "Refuse next proposal");
    click(&mut v, &view, "radio", "Auto");
    node(&mut v, &view, "status", "Waiting for schedule approval");
    assert_eq!(
        node(&mut v, &view, "radio", "Manual").checked,
        Some(UiValue::Bool(true))
    );
    held(&mut v, &view, "Mon 12:00", 75);
    held(&mut v, &view, "Tue 07:00", 0);
    advance(&mut v, &view);
    node(&mut v, &view, "status", "Schedule change refused");
    held(&mut v, &view, "Tue 07:00", 0);
    assert_eq!(
        node(&mut v, &view, "radio", "Auto").checked,
        Some(UiValue::Bool(false))
    );

    fn track_ref(node: &gpui_rhai::UiNode) -> Option<gpui_rhai::ElementRef> {
        if let Some(reference) = node.element_ref().filter(|r| r.id().key() == "mode-track") {
            return Some(reference.clone());
        }
        match node.kind() {
            gpui_rhai::UiNodeKind::Box { children }
            | gpui_rhai::UiNodeKind::Fragment { children } => children.iter().find_map(track_ref),
            _ => None,
        }
    }
    let reference = v
        .update(|_, cx| track_ref(&view.root(cx).unwrap().unwrap()))
        .unwrap();
    v.update(|window, cx| view.focus_element(&reference, window, cx))
        .unwrap();
    let focus = v.update(|window, cx| window.focused(cx)).unwrap();
    v.simulate_keystrokes("right");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Waiting for schedule approval");
    assert_eq!(
        node(&mut v, &view, "radio", "Auto").checked,
        Some(UiValue::Bool(false))
    );
    advance(&mut v, &view);
    node(&mut v, &view, "status", "Accepted auto at revision 1");
    assert_eq!(node(&mut v, &view, "radio", "Auto").id, auto.id);
    assert_eq!(
        node(&mut v, &view, "radio", "Auto").checked,
        Some(UiValue::Bool(true))
    );
    assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focus));
    held(&mut v, &view, "Mon 07:00", 55);
    held(&mut v, &view, "Mon 12:00", 100);
    held(&mut v, &view, "Tue 07:00", 100);
    held(&mut v, &view, "Tue 12:00", 85);
    assert_eq!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Inspect Sync media")
            .count(),
        3
    );
    click(&mut v, &view, "button", "Atlas");
    assert_eq!(node(&mut v, &view, "button", "Atlas").pressed, Some(true));
    let tuesday = bounds(&mut v, &view, "gridcell", "Tue 07:00");
    let target = center(bounds(&mut v, &view, "group", "Drop on Wed 07:00"));
    let source = snapshot(&mut v, &view)
        .find_by_role_and_name("button", "Drag Sync media")
        .filter_map(|node| node.geometry)
        .map(|geometry| geometry.visual)
        .find(|rect| {
            rect.x >= tuesday.x
                && rect.x < tuesday.x + tuesday.width
                && rect.y >= tuesday.y
                && rect.y < tuesday.y + tuesday.height
        })
        .expect("Tuesday morning has the 65-unit piece");
    drag_between(&mut v, &view, center(source), target);
    advance(&mut v, &view);
    node(&mut v, &view, "status", "Accepted move at revision 2");
    held(&mut v, &view, "Wed 07:00", 65);
    click(&mut v, &view, "radio", "Manual");
    advance(&mut v, &view);
    node(&mut v, &view, "status", "Accepted manual at revision 3");
    held(&mut v, &view, "Mon 07:00", 55);
    held(&mut v, &view, "Mon 12:00", 100);
    held(&mut v, &view, "Tue 07:00", 0);
    held(&mut v, &view, "Tue 12:00", 40);
    held(&mut v, &view, "Wed 07:00", 65);
    assert_eq!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Inspect Sync media")
            .count(),
        3
    );
}

#[gpui::test]
fn scheduled_runs_native_drag_places_and_then_moves_a_booked_part(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "scheduled_runs", "clean-dark");
    v.simulate_resize(size(px(1340.), px(960.)));
    ready(&mut v, &view);

    drag_to(&mut v, &view, "Drag Audit package", "Drop on Mon 07:00");
    held(&mut v, &view, "Mon 07:00", 55);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("status", "Waiting for schedule approval")
            .next()
            .is_none()
    );

    drag_to(&mut v, &view, "Drag Audit package", "Drop on Tue 07:00");
    node(&mut v, &view, "status", "Waiting for schedule approval");
    held(&mut v, &view, "Tue 07:00", 0);
    advance(&mut v, &view);
    node(&mut v, &view, "status", "Accepted place at revision 1");
    held(&mut v, &view, "Tue 07:00", 35);
    drag_to(&mut v, &view, "Drag Audit package", "Drop on Wed 12:00");
    held(&mut v, &view, "Tue 07:00", 35);
    advance(&mut v, &view);
    node(&mut v, &view, "status", "Accepted move at revision 2");
    held(&mut v, &view, "Tue 07:00", 0);
    held(&mut v, &view, "Wed 12:00", 35);
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn scheduled_runs_keyboard_uses_same_proposal_and_blanks_ineligible_windows(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "scheduled_runs", "clean-light");
    v.simulate_resize(size(px(1340.), px(990.)));
    ready(&mut v, &view);
    assert_eq!(repeating_period_at(&mut v, &view, "Mon 07:00"), None);
    click(&mut v, &view, "button", "Inspect Audit package");
    node(&mut v, &view, "dialog", "Audit package");
    click(&mut v, &view, "button", "Move with keyboard");
    assert_eq!(
        node(&mut v, &view, "button", "Place in Mon 07:00").description,
        "Unavailable"
    );
    let period = repeating_period_at(&mut v, &view, "Mon 07:00")
        .expect("blocked slot has a real repeating fill");
    assert!(
        (period - 7.).abs() < 0.01,
        "source geometry uses a seven-pixel period"
    );
    assert_eq!(repeating_period_at(&mut v, &view, "Tue 12:00"), None);
    v.simulate_keystrokes("up enter");
    ready(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "This session cannot accept the task.",
    );
    held(&mut v, &view, "Mon 07:00", 55);
    v.simulate_keystrokes("down right enter");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Waiting for schedule approval");
    held(&mut v, &view, "Tue 12:00", 0);
    advance(&mut v, &view);
    node(&mut v, &view, "status", "Accepted place at revision 1");
    assert_eq!(repeating_period_at(&mut v, &view, "Mon 07:00"), None);
    held(&mut v, &view, "Tue 12:00", 35);
    click(&mut v, &view, "button", "Narrow board");
    let board = bounds(&mut v, &view, "group", "Publishing week");
    let backlog = bounds(&mut v, &view, "group", "To schedule");
    let grid = bounds(&mut v, &view, "grid", "Week schedule");
    assert!(
        board.width <= 482. && grid.y >= backlog.y + backlog.height - 1.,
        "{board:?} {backlog:?} {grid:?}"
    );
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn scheduled_runs_pointer_eligibility_paints_after_threshold_and_clears_on_cancel_or_drop(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "scheduled_runs", "clean-dark");
    v.simulate_resize(size(px(1340.), px(990.)));
    ready(&mut v, &view);
    let start = center(bounds(&mut v, &view, "button", "Drag Audit package"));
    let invalid = center(bounds(&mut v, &view, "gridcell", "Mon 07:00"));
    let modifiers = Modifiers::none();

    v.simulate_mouse_down(start, MouseButton::Left, modifiers);
    v.simulate_mouse_move(
        point(start.x + px(2.), start.y),
        MouseButton::Left,
        modifiers,
    );
    ready(&mut v, &view);
    assert_eq!(repeating_period_at(&mut v, &view, "Mon 07:00"), None);
    v.simulate_mouse_move(invalid, MouseButton::Left, modifiers);
    ready(&mut v, &view);
    assert!(
        repeating_period_at(&mut v, &view, "Mon 07:00").is_some(),
        "the active native drag marks blocked capacity"
    );
    assert_eq!(repeating_period_at(&mut v, &view, "Tue 07:00"), None);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Place in Tue 07:00")
            .next()
            .is_none(),
        "pointer feedback must not add keyboard placement buttons"
    );
    let cell = node(&mut v, &view, "gridcell", "Mon 07:00").id;
    v.update(|_, cx| view.select_theme("Modern", "Light", cx))
        .unwrap();
    ready(&mut v, &view);
    assert_eq!(node(&mut v, &view, "gridcell", "Mon 07:00").id, cell);
    assert!(
        repeating_period_at(&mut v, &view, "Mon 07:00").is_some(),
        "switching designs must not drop an active drag"
    );
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    assert_eq!(repeating_period_at(&mut v, &view, "Mon 07:00"), None);
    let accepted = center(bounds(&mut v, &view, "gridcell", "Tue 07:00"));
    v.simulate_mouse_up(accepted, MouseButton::Left, modifiers);
    ready(&mut v, &view);
    held(&mut v, &view, "Tue 07:00", 0);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("status", "Waiting for schedule approval")
            .next()
            .is_none()
    );

    let start = center(bounds(&mut v, &view, "button", "Drag Audit package"));
    v.simulate_mouse_down(start, MouseButton::Left, modifiers);
    v.simulate_mouse_move(accepted, MouseButton::Left, modifiers);
    ready(&mut v, &view);
    assert!(
        repeating_period_at(&mut v, &view, "Mon 07:00").is_some(),
        "a second gesture cannot reuse cancelled state"
    );
    v.simulate_mouse_up(accepted, MouseButton::Left, modifiers);
    ready(&mut v, &view);
    assert_eq!(repeating_period_at(&mut v, &view, "Mon 07:00"), None);
    node(&mut v, &view, "status", "Waiting for schedule approval");
    advance(&mut v, &view);
    held(&mut v, &view, "Tue 07:00", 35);
}

#[gpui::test]
fn scheduled_runs_card_keeps_usage_on_a_separate_accented_line(cx: &mut TestAppContext) {
    use crate::support::{PresetRisks, paint::live_painted_quads};
    let audit = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    for (theme, chip) in [
        ("modern-light", 0x0000000f),
        ("modern-dark", 0xffffff12),
        ("clean-light", 0x0000000d),
        ("clean-dark", 0xffffff12),
    ] {
        let (mut v, view) = gallery(cx, "scheduled_runs", theme);
        v.simulate_resize(size(px(1340.), px(990.)));
        ready(&mut v, &view);
        let card = bounds(&mut v, &view, "button", "Inspect Reindex archive");
        let title = bounds(&mut v, &view, "text", "Reindex archive");
        let meta = bounds(&mut v, &view, "text", "Atlas · Prior session");
        let value = bounds(&mut v, &view, "text", "55%");
        assert!(
            title.y + title.height <= meta.y + 0.5
                && meta.y + meta.height <= value.y + 0.5
                && value.y + value.height <= card.y + card.height + 0.5,
            "{theme}: rows need distinct space: {title:?} {meta:?} {value:?}, {card:?}"
        );
        assert!(
            (title.x - meta.x).abs() <= 1.0 && value.x + value.width > card.x + card.width - 24.,
            "{theme}: align the copy, then trail the usage pill independently"
        );
        let mut ink = gpui::rgba(chip);
        if !gpui_rhai::EXTENDED_RENDERING {
            // The public painter applies the historical card's opacity to
            // each primitive; the vendor painter composites the group.
            ink.a *= 0.45;
        }
        assert!(
            live_painted_quads(&mut v).iter().any(|quad| {
                let bounds = quad.bounds;
                quad.background
                    .as_solid()
                    .map(gpui::Rgba::from)
                    .is_some_and(|fill| {
                        (fill.r - ink.r).abs() < 0.002
                            && (fill.g - ink.g).abs() < 0.002
                            && (fill.b - ink.b).abs() < 0.002
                            && (fill.a - ink.a).abs() < 0.002
                    })
                    && f64::from(bounds.x) < value.x
                    && f64::from(bounds.right()) > value.x + value.width
                    && (f64::from(bounds.y) - value.y).abs() <= 1.0
                    && (f64::from(bounds.height) - value.height).abs() <= 1.0
            }),
            "{theme}: no actual badge fill in the historical card"
        );
        let (variant, raw) = v.update(|_, cx| {
            (
                view.theme_snapshot(cx).unwrap().variant,
                view.composition_audit_with(&audit, cx).unwrap(),
            )
        });
        let historic: Vec<_> = raw
            .iter()
            .filter(|finding| {
                finding
                    .path
                    .contains("/cards/part:last-run/detail/trigger/")
            })
            .collect();
        assert_eq!(historic.len(), 4, "{theme}: {historic:#?}");
        let mut policy = PresetRisks::new();
        for finding in historic {
            assert!(policy.record(&variant, finding), "{theme}: {finding:#?}");
        }
        let metrics: Vec<_> = raw
            .iter()
            .filter(|finding| {
                finding.path.contains("/stack/items/")
                    && finding.path.ends_with("/detail/trigger/stats/0/0")
            })
            .collect();
        assert_eq!(metrics.len(), 2, "{theme}: {metrics:#?}");
        for finding in metrics {
            assert!(policy.record(&variant, finding), "{theme}: {finding:#?}");
        }
        policy.assert_reported();
    }
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn scheduled_runs_clean_uses_live_host_paper_under_its_history(cx: &mut TestAppContext) {
    use super::acceptance::action;
    use crate::support::Mount;
    #[cfg(feature = "gpui-fast")]
    use apple_fast::metal_renderer::MetalRenderer;
    #[cfg(feature = "gpui-pre")]
    use apple_pre::metal_renderer::MetalRenderer;

    let (mut v, view) = Mount::new(r#"
        import "patterns/scheduled_runs" as week;
        fn state_schema(){#{fields:#{second:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            opaque:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn init(ctx){
            ctx.register_action("preview.change_paper",Fn("change_paper"));
            ctx.register_action("preview.set_opaque",Fn("set_opaque"));
        }
        fn change_paper(ctx,value){ctx.set_state("second",value);}
        fn set_opaque(ctx,value){ctx.set_state("opaque",value);}
        fn view(ctx){
            let paper=if ctx.get_state("second"){rgb(0x703f54)}else{rgb(0x32506a)};
            let plan=week::ScheduledRuns(#{key:"calendar",board:"calendar",revision:0,mode:"manual",
                title:"Hosted week",days:[#{id:"mon",label:"Mon"},#{id:"tue",label:"Tue"}],
                periods:[#{id:"am",label:"07:00",limit:100},#{id:"pm",label:"12:00",limit:100}],
                current:#{day:"mon",period:"pm"},
                tasks:[#{id:"archive",title:"Archive",units:55},#{id:"current",title:"Current task",units:75}],
                assignments:[#{id:"prior",task:"archive",day:"mon",period:"am",units:55,ordinal:1,origin:"booked"},
                    #{id:"today",task:"current",day:"mon",period:"pm",units:75,ordinal:1,origin:"booked"}],
                part_styles:if ctx.get_state("opaque"){#{root:style().background(theme_color("scheduled_runs.card"))}}else{#{}}});
            column([plan]).with_style(style().background(paper).padding(px(16)))
        }
    "#).registry("patterns/scheduled_runs").theme(gpui_rhai_registry::CLEAN_DARK_THEME)
        .motion(gpui_rhai::MotionPreference::None).mount(cx);
    v.simulate_resize(size(px(1200.), px(900.)));
    ready(&mut v, &view);
    let history = node(&mut v, &view, "button", "Inspect Archive").id;
    for (second, opaque, expected_paper) in [
        (false, false, [50, 80, 106]),
        (true, false, [112, 63, 84]),
        (true, true, [28, 28, 31]),
    ] {
        action(&mut v, &view, "preview.change_paper", UiValue::Bool(second));
        action(&mut v, &view, "preview.set_opaque", UiValue::Bool(opaque));
        ready(&mut v, &view);
        assert_eq!(node(&mut v, &view, "button", "Inspect Archive").id, history);
        let board = bounds(&mut v, &view, "group", "Hosted week");
        let percent = bounds(&mut v, &view, "text", "55%");
        let active = bounds(&mut v, &view, "button", "Inspect Current task");
        let (outer, dimmed_badge, opaque_card) = v.update(|window, _| {
            let scale = window.scale_factor();
            let extent = window.viewport_size();
            let device = size(
                gpui::DevicePixels((f32::from(extent.width) * scale).ceil() as i32),
                gpui::DevicePixels((f32::from(extent.height) * scale).ceil() as i32),
            );
            window.visual_probe_with_painted_scene(|scene| {
                let mut renderer = MetalRenderer::new_headless(Default::default()).unwrap();
                let image = renderer.render_scene_to_image(scene, device).unwrap();
                let at = |x: f64, y: f64| {
                    let scale = f64::from(scale);
                    let p = image
                        .get_pixel((x * scale).floor() as u32, (y * scale).floor() as u32)
                        .0;
                    [i32::from(p[0]), i32::from(p[1]), i32::from(p[2])]
                };
                (
                    at(board.x + board.width / 2., board.y + 4.),
                    at(percent.x - 2., percent.y + percent.height / 2.),
                    at(active.x + active.width - 14., active.y + 7.),
                )
            })
        });
        assert_eq!(
            outer, expected_paper,
            "Host paper or explicit caller override"
        );
        assert_eq!(
            opaque_card,
            [28, 28, 31],
            "the current booking owns its card fill"
        );
        if gpui_rhai::EXTENDED_RENDERING {
            for (i, card) in [28., 28., 31.].into_iter().enumerate() {
                let pill = card + (255. - card) * (18. / 255.);
                let expected = (pill * 0.45 + f64::from(expected_paper[i]) * 0.55).round() as i32;
                assert!(
                    (dimmed_badge[i] - expected).abs() <= 2,
                    "{expected_paper:?} {dimmed_badge:?}"
                );
            }
        } else {
            assert_ne!(
                dimmed_badge, expected_paper,
                "historical badge is still painted"
            );
            assert_ne!(
                dimmed_badge, opaque_card,
                "historical badge remains distinct"
            );
        }
    }
}

#[gpui::test]
fn scheduled_runs_history_is_inspectable_but_has_no_move_action(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "scheduled_runs", "modern-light");
    v.simulate_resize(size(px(1340.), px(990.)));
    ready(&mut v, &view);
    let historical = node(&mut v, &view, "button", "Inspect Reindex archive").id;
    click(&mut v, &view, "button", "Inspect Reindex archive");
    node(&mut v, &view, "dialog", "Reindex archive");
    let past = snapshot(&mut v, &view);
    for action in ["Drag Reindex archive", "Move with keyboard"] {
        assert!(
            past.find_by_role_and_name("button", action)
                .next()
                .is_none(),
            "{action}"
        );
    }
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "button", "Inspect Reindex archive").id,
        historical
    );
    let focus = v
        .update(|win, cx| view.debug_frame_digest(win, cx))
        .unwrap()
        .focus_path;
    assert_eq!(focus.first(), Some(&historical));

    click(&mut v, &view, "button", "Inspect Build release");
    node(&mut v, &view, "dialog", "Build release");
    click(&mut v, &view, "button", "Move with keyboard");
    node(&mut v, &view, "button", "Place in Tue 07:00");
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    held(&mut v, &view, "Mon 07:00", 55);
    held(&mut v, &view, "Mon 12:00", 75);
}

#[gpui::test]
fn scheduled_runs_advancing_the_session_retires_a_keyboard_pickup(cx: &mut TestAppContext) {
    use super::acceptance::action;
    use crate::support::Mount;

    let (mut v, view) = Mount::new(r#"
        import "patterns/scheduled_runs" as week;
        fn state_schema() { #{fields:#{past:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}} }
        fn init(ctx) { ctx.register_action("test.advance",Fn("advance")); }
        fn advance(ctx, value) { ctx.set_state("past",value); }
        fn propose(ctx, data) { throw "No proposal should be emitted by this clock transition"; }
        fn view(ctx) {
            week::ScheduledRuns(#{ key:"calendar",board:"calendar",revision:0,mode:"manual",
                days:[#{id:"mon",label:"Mon"},#{id:"tue",label:"Tue"}],
                periods:[#{id:"am",label:"07:00",limit:100},#{id:"pm",label:"12:00",limit:100}],
                current:#{day:"mon",period:if ctx.get_state("past"){"pm"}else{"am"}},
                tasks:[#{id:"archive",title:"Archive",units:55}],
                assignments:[#{id:"booking",task:"archive",day:"mon",period:"am",units:55,ordinal:1,origin:"booked"}],
                on_propose:Fn("propose") })
        }
    "#).registry("patterns/scheduled_runs").motion(gpui_rhai::MotionPreference::None).mount(cx);
    v.simulate_resize(size(px(1220.), px(900.)));
    ready(&mut v, &view);
    let id = node(&mut v, &view, "button", "Inspect Archive").id;
    click(&mut v, &view, "button", "Inspect Archive");
    click(&mut v, &view, "button", "Move with keyboard");
    node(&mut v, &view, "button", "Place in Mon 12:00");

    for time_has_advanced in [true, false] {
        // The Host clock can advance without changing the booking revision.
        action(
            &mut v,
            &view,
            "test.advance",
            UiValue::Bool(time_has_advanced),
        );
        ready(&mut v, &view);
        assert!(
            !snapshot(&mut v, &view)
                .nodes()
                .any(|n| n.role == "button" && n.name.starts_with("Place in ")),
            "a pickup cannot remain armed or revive after the source went historical"
        );
        assert_eq!(node(&mut v, &view, "button", "Inspect Archive").id, id);
        let focus = v
            .update(|win, cx| view.debug_frame_digest(win, cx))
            .unwrap()
            .focus_path;
        assert_eq!(focus.first(), Some(&id));
        held(&mut v, &view, "Mon 07:00", 55);
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn scheduled_runs_past_keeps_chips_and_meter_opacity_on_their_own_surfaces(
    cx: &mut TestAppContext,
) {
    use crate::support::{logical_paint_rect, walk_live_paint};

    for theme in ["modern-light", "modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "scheduled_runs", theme);
        v.simulate_resize(size(px(1340.), px(990.)));
        ready(&mut v, &view);
        let task = bounds(&mut v, &view, "button", "Inspect Reindex archive");
        let bar = bounds(&mut v, &view, "progressbar", "Mon 07:00");
        let cell = bounds(&mut v, &view, "gridcell", "Mon 07:00");
        let (track, fill) = v.update(|_, cx| {
            let variant = view.theme_snapshot(cx).unwrap().variant;
            (
                gpui::rgba(
                    variant
                        .tokens
                        .color("scheduled_runs.track")
                        .unwrap()
                        .as_rgba_hex(),
                ),
                gpui::rgba(
                    variant
                        .tokens
                        .color("scheduled_runs.capacity")
                        .unwrap()
                        .as_rgba_hex(),
                ),
            )
        });

        let (groups, quads) = v.update(|window, _| {
            let scale = window.scale_factor();
            window.visual_probe_with_painted_scene(|scene| {
                let mut groups = Vec::new();
                let mut quads = Vec::new();
                walk_live_paint(
                    scene,
                    gpui::TransformationMatrix::unit(),
                    &mut |scene, placement| {
                        groups.extend(scene.isolated_groups.iter().map(|group| {
                            (
                                logical_paint_rect(group.bounds, placement, scale),
                                group.opacity(),
                            )
                        }));
                        quads.extend(scene.quads.iter().filter_map(|quad| {
                            quad.background.as_solid().map(|ink| {
                                (
                                    logical_paint_rect(quad.bounds, placement, scale),
                                    gpui::Rgba::from(ink),
                                )
                            })
                        }));
                    },
                );
                (groups, quads)
            })
        });
        let x = task.x + task.width / 2.;
        let y = task.y + task.height / 2.;
        assert!(
            groups.iter().any(|(rect, opacity)| {
                (opacity - 0.45).abs() < 0.001
                    && f64::from(rect.x) <= x
                    && f64::from(rect.right()) >= x
                    && f64::from(rect.y) <= y
                    && f64::from(rect.bottom()) >= y
                    && f64::from(rect.bottom()) < bar.y + 1.0
            }),
            "{theme}: only the past chip group must composite at 0.45, task={task:?}, bar={bar:?}, groups={groups:?}"
        );
        assert!(
            !groups.iter().any(|(rect, opacity)| {
                *opacity < 0.99
                    && f64::from(rect.x) <= cell.x + 2.
                    && f64::from(rect.right()) >= cell.x + cell.width - 2.
                    && f64::from(rect.y) <= cell.y + 2.
                    && f64::from(rect.bottom()) >= bar.y + bar.height - 1.
            }),
            "{theme}: the whole past cell must not be faded"
        );
        for (color, opacity, width) in [(track, 1.0, bar.width), (fill, 0.5, bar.width * 0.55)] {
            assert!(
                quads.iter().any(|(rect, ink)| {
                    (ink.r - color.r).abs() < 0.003
                        && (ink.g - color.g).abs() < 0.003
                        && (ink.b - color.b).abs() < 0.003
                        && (ink.a - opacity).abs() < 0.003
                        && (f64::from(rect.x) - bar.x).abs() <= 1.0
                        && (f64::from(rect.y) - bar.y).abs() <= 1.0
                        && (f64::from(rect.width) - width).abs() <= 1.0
                }),
                "{theme}: no {color:?} bar paint at alpha {opacity}, bar {bar:?}"
            );
        }

        #[cfg(target_os = "macos")]
        v.update(|window, _| {
            #[cfg(feature = "gpui-fast")]
            use apple_fast::metal_renderer::MetalRenderer;
            #[cfg(feature = "gpui-pre")]
            use apple_pre::metal_renderer::MetalRenderer;

            let scale = window.scale_factor();
            let extent = window.viewport_size();
            let device_extent = size(
                gpui::DevicePixels((f32::from(extent.width) * scale).ceil() as i32),
                gpui::DevicePixels((f32::from(extent.height) * scale).ceil() as i32),
            );
            window.visual_probe_with_painted_scene(|scene| {
                let mut renderer = MetalRenderer::new_headless(Default::default()).unwrap();
                let image = renderer.render_scene_to_image(scene, device_extent).unwrap();
                let y = ((bar.y + bar.height / 2.0) * f64::from(scale)).floor() as u32;
                for (at, amount) in [(0.25, 0.5), (0.75, 0.0)] {
                    let x = ((bar.x + bar.width * at) * f64::from(scale)).floor() as u32;
                    let pixel = image.get_pixel(x, y);
                    for (channel, (track, fill)) in [(track.r, fill.r), (track.g, fill.g), (track.b, fill.b)].into_iter().enumerate() {
                        let expected = (track + (fill - track) * amount) * 255.0;
                        assert!(
                            (f32::from(pixel[channel]) - expected).abs() <= 2.0,
                            "{theme}: pixel {x},{y} channel {channel}: {pixel:?}, expected {expected}"
                        );
                    }
                }
            });
        });

        click(&mut v, &view, "button", "Inspect Reindex archive");
        node(&mut v, &view, "dialog", "Reindex archive");
    }
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
