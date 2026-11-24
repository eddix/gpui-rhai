use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, ScriptViewHandle};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, bounds, snapshot};

fn find_bar(scene: &gpui::Scene, mid_y: f32, scale: f32) -> Option<(f32, f32)> {
    for q in &scene.quads {
        let Some(paint) = q.background.as_solid() else {
            continue;
        };
        let c = paint.to_rgb();
        let y = q.bounds.center().y.0 / scale;
        if (y - mid_y).abs() < 12.0
            && (c.r - 245.0 / 255.0).abs() < 0.007
            && (c.g - 238.0 / 255.0).abs() < 0.007
            && (c.b - 253.0 / 255.0).abs() < 0.007
            && c.a > 0.98
            && q.bounds.size.width.0 / scale > 5.0
        {
            return Some((q.bounds.origin.x.0 / scale, q.bounds.size.width.0 / scale));
        }
    }
    #[cfg(feature = "gpui-fast")]
    for layer in scene.layers.frames.iter().filter(|l| l.tile_size == 0) {
        if let Some(bar) = layer
            .content
            .scenes()
            .find_map(|s| find_bar(s, mid_y, scale))
        {
            return Some(bar);
        }
    }
    None
}

fn llm_bar(v: &mut VisualTestContext, view: &ScriptViewHandle) -> (f32, f32) {
    let target = bounds(v, view, "treeitem", "Plan the guide");
    v.update(|window, _| {
        window.visual_probe_with_painted_scene(|scene| {
            find_bar(
                scene,
                (target.y + target.height / 2.) as f32,
                window.scale_factor(),
            )
        })
    })
    .expect("LLM bar painted in its own row")
}

#[gpui::test]
fn trace_waterfall_presents_distinct_reference_type_and_keeps_semantic_labels(
    cx: &mut TestAppContext,
) {
    use gpui_rhai::{ColorValue, Length, UiNode, UiNodeKind};

    fn descendant<'a>(
        root: &'a UiNode,
        predicate: &impl Fn(&UiNode) -> bool,
    ) -> Option<&'a UiNode> {
        if predicate(root) {
            return Some(root);
        }
        if let UiNodeKind::Box { children } | UiNodeKind::Fragment { children } = root.kind() {
            return children
                .iter()
                .find_map(|child| descendant(child, predicate));
        }
        if let UiNodeKind::Overlay { trigger, .. } = root.kind() {
            return descendant(trigger, predicate);
        }
        None
    }

    for theme in ["modern-light", "modern-dark", "clean-light", "clean-dark"] {
        let clean = theme.starts_with("clean");
        let (mut v, view) = gallery(cx, "trace_waterfall", theme);
        v.simulate_resize(size(px(1380.), px(1040.)));
        ready(&mut v, &view);
        let metric = node(&mut v, &view, "group", "Duration");
        let label = snapshot(&mut v, &view)
            .nodes()
            .find(|n| n.parent == Some(metric.id) && n.role == "text" && n.name == "Duration")
            .unwrap_or_else(|| panic!("{theme}: the spoken label stays mixed case"))
            .id;
        let at = |v: &mut VisualTestContext, narrow| {
            let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
            let metric = descendant(&root, &|n| {
                n.key().is_some_and(|k| k.as_str() == "metric:0")
            })
            .unwrap();
            let UiNodeKind::Box { children } = metric.kind() else {
                panic!("stat cell owns its label and number");
            };
            let assert_type = |part: &UiNode, font, line, weight| {
                let style = &part.style().base;
                assert_eq!(
                    style.font_size,
                    Some(Length::Pixels(font)),
                    "{theme}/{narrow}"
                );
                assert_eq!(
                    style.line_height,
                    Some(Length::Pixels(line)),
                    "{theme}/{narrow}"
                );
                assert_eq!(style.font_weight, Some(weight), "{theme}/{narrow}");
            };
            assert_type(
                &children[0],
                if clean { 10. } else { 12. },
                16.,
                if clean { 500 } else { 400 },
            );
            assert_type(
                &children[1],
                if clean || narrow { 15. } else { 16. },
                if clean { 24. } else { 22. },
                if clean { 500 } else { 600 },
            );
            let expected = if clean { "DURATION" } else { "Duration" };
            assert!(
                matches!(children[0].kind(), UiNodeKind::Text{text, ..} if text.as_str() == expected),
                "{theme}/{narrow}"
            );
            let axis =
                descendant(&root, &|n| n.key().is_some_and(|k| k.as_str() == "axis")).unwrap();
            // The track chooses its own step when its measured width changes.
            let track = descendant(axis, &|n| {
                n.key().is_some_and(|k| k.as_str() == "axis-track")
            })
            .unwrap();
            let tick = descendant(track, &|n| matches!(n.kind(), UiNodeKind::Text { .. }))
                .expect("the time grid has numeric tick labels");
            assert_type(
                tick,
                if clean { 12. } else { 11. },
                if clean { 16. } else { 14. },
                400,
            );
            let name = descendant(
                axis,
                &|n| matches!(n.kind(), UiNodeKind::Text{text, ..} if matches!(text.as_str(), "Span" | "SPAN")),
            );
            if narrow {
                assert!(name.is_none());
            } else {
                assert_type(
                    name.unwrap(),
                    if clean { 10. } else { 11. },
                    if clean { 16. } else { 14. },
                    if clean { 500 } else { 400 },
                );
                let span = descendant(&root, &|n| {
                    n.key().is_some_and(|k| k.as_str() == "span:outline")
                })
                .expect("the completed LLM row is initially visible");
                let duration =
                    descendant(span, &|n| n.key().is_some_and(|k| k.as_str() == "duration"))
                        .expect("wide rows show the duration on the lane");
                let text = descendant(duration, &|n| matches!(n.kind(), UiNodeKind::Text { .. }))
                    .expect("duration has a numeric text readout");
                assert_eq!(
                    text.style().base.text_color,
                    Some(ColorValue::Token("trace.text".into())),
                    "{theme}"
                );
                assert_eq!(
                    text.style().base.font_size,
                    Some(Length::Pixels(if clean { 12. } else { 11.5 })),
                    "{theme}"
                );
            }
        };
        at(&mut v, false);
        let rules = gpui_rhai::AuditRules::from_ids([
            "mixed-type-in-row",
            "spacing-not-nested",
            "low-contrast-text",
        ])
        .unwrap();
        let findings = v.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
        let span_chrome =
            |path: &str| path.contains("/viewport/tree/span:") && path.contains("/trigger");
        assert!(
            findings
                .iter()
                .filter(|f| span_chrome(&f.path))
                .all(|f| f.rule == gpui_rhai::AuditRule::LowContrastText),
            "{theme}: {findings:#?}"
        );
        if theme == "clean-dark" {
            assert!(
                findings.iter().any(
                    |f| span_chrome(&f.path) && f.rule == gpui_rhai::AuditRule::LowContrastText
                )
            );
        }
        let before = bounds(&mut v, &view, "group", "Duration");
        click(&mut v, &view, "button", "Narrow trace");
        at(&mut v, true);
        let row_end = bounds(&mut v, &view, "group", "Errors");
        let after = node(&mut v, &view, "group", "Duration");
        assert_eq!(after.id, metric.id);
        let label = snapshot(&mut v, &view)
            .nodes()
            .find(|n| n.id == label)
            .unwrap()
            .clone();
        assert_eq!(label.name, "Duration");
        let label_bounds = label.geometry.unwrap().visual;
        let after = after.geometry.unwrap().visual;
        assert!(row_end.y >= after.y + after.height && label_bounds.height > 0.0);
        assert!((before.height - after.height).abs() <= 1.);
    }
}

#[gpui::test]
fn trace_waterfall_axis_caption_keeps_its_own_logical_inset(cx: &mut TestAppContext) {
    use super::acceptance::action;
    use gpui_rhai::{AuditRules, UiValue};

    let (mut v, view) = gallery(cx, "trace_waterfall", "clean-dark");
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    let color = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let alignment = AuditRules::from_ids(["text-edge-misaligned"]).unwrap();
    for density in ["comfortable", "compact"] {
        action(
            &mut v,
            &view,
            "gallery.set_density",
            UiValue::String(density.into()),
        );
        for (scale, inset) in [("100", 16.0), ("150", 24.0)] {
            action(
                &mut v,
                &view,
                "gallery.set_scale",
                UiValue::String(scale.into()),
            );
            ready(&mut v, &view);
            let first_cell = bounds(&mut v, &view, "group", "Duration");
            let axis = v.update(|window, cx| {
                let id = view
                    .composition_audit_with(&color, cx)
                    .unwrap()
                    .into_iter()
                    .find(|finding| {
                        finding.path.contains("/card/axis/")
                            && finding.path.ends_with("/axis-label")
                    })
                    .expect("the visible axis caption still reports the stock contrast risk")
                    .node
                    .unwrap();
                view.debug_frame_digest(window, cx)
                    .unwrap()
                    .geometry
                    .into_iter()
                    .find(|(node, _)| *node == id)
                    .unwrap()
                    .1
                    .visual
            });
            assert!(
                (axis.x - first_cell.x - inset).abs() <= 0.5,
                "{density}/{scale}: {axis:?} from {first_cell:?}"
            );
            let findings = v.update(|_, cx| view.composition_audit_with(&alignment, cx).unwrap());
            assert!(
                findings
                    .iter()
                    .all(|finding| !finding.path.ends_with("/sized/run/card")),
                "{density}/{scale}: {findings:?}"
            );
        }
    }
}

#[gpui::test]
fn trace_waterfall_uses_one_time_domain_before_and_after_rescale(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "trace_waterfall", "modern-light");
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    let first = llm_bar(&mut v, &view);
    let running = node(&mut v, &view, "treeitem", "Write draft").description;
    assert!(running.contains("1.9 s"), "{running}");
    click(&mut v, &view, "button", "Advance trace clock");
    node(&mut v, &view, "status", "Trace clock advanced");
    let later = llm_bar(&mut v, &view);
    assert!(
        (later.1 / first.1 - 0.5).abs() < 0.035,
        "10 s -> 20 s: {first:?} to {later:?}"
    );
    let running = node(&mut v, &view, "treeitem", "Write draft").description;
    assert!(running.contains("8.5 s"), "{running}");
    click(&mut v, &view, "button", "Finish trace");
    node(&mut v, &view, "status", "Done");
    click(&mut v, &view, "button", "Advance trace clock");
    let still = llm_bar(&mut v, &view);
    assert!(
        (still.1 - later.1).abs() <= 1.0,
        "accepted finished spans must not follow a later clock: {later:?}, {still:?}"
    );
}

#[gpui::test]
fn trace_waterfall_controlled_disclosure_keeps_rolled_up_metrics(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "trace_waterfall", "clean-dark");
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    let root = node(&mut v, &view, "treeitem", "Prepare onboarding draft").description;
    let slow = node(&mut v, &view, "treeitem", "Reconcile notes").description;
    assert!(slow.contains("Slowest"), "{slow}");
    click(&mut v, &view, "button", "Refuse next trace proposal");
    click(&mut v, &view, "button", "Collapse Collect references");
    node(&mut v, &view, "status", "Trace proposal declined");
    node(&mut v, &view, "treeitem", "Reconcile notes");
    click(&mut v, &view, "button", "Collapse Collect references");
    node(&mut v, &view, "status", "Trace folding accepted");
    let branch = node(&mut v, &view, "treeitem", "Collect references");
    assert_eq!(branch.expanded, Some(false));
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("treeitem", "Reconcile notes")
            .next()
            .is_none()
    );
    assert_eq!(
        node(&mut v, &view, "treeitem", "Prepare onboarding draft").description,
        root
    );
    v.simulate_keystrokes("right");
    ready(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "treeitem", "Collect references").expanded,
        Some(true)
    );
    node(&mut v, &view, "treeitem", "Reconcile notes");
}

#[gpui::test]
fn trace_waterfall_narrow_layout_and_details_work_with_native_input(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "trace_waterfall", "modern-dark");
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    let duration = bounds(&mut v, &view, "group", "Duration");
    let errors = bounds(&mut v, &view, "group", "Errors");
    assert!(errors.x > duration.x + duration.width && (errors.y - duration.y).abs() <= 1.);
    let wide_row = bounds(&mut v, &view, "treeitem", "Plan the guide");
    click(&mut v, &view, "button", "Narrow trace");
    let narrow_row = bounds(&mut v, &view, "treeitem", "Plan the guide");
    assert!(narrow_row.width < wide_row.width && narrow_row.height > wide_row.height + 10.);
    assert!(
        bounds(&mut v, &view, "group", "Errors").y > bounds(&mut v, &view, "group", "Duration").y
    );
    click(&mut v, &view, "treeitem", "Open archive");
    node(&mut v, &view, "dialog", "Open archive details");
    node(&mut v, &view, "status", "Details opened");
    click(&mut v, &view, "button", "Close details");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("dialog", "Open archive details")
            .next()
            .is_none()
    );
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    node(&mut v, &view, "dialog", "Open archive details");
}

#[gpui::test]
fn trace_waterfall_rescales_in_native_frames_without_reexecuting_rhai(cx: &mut TestAppContext) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "trace_waterfall".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .runtime_clock(clock.clock())
    .motion_preference(MotionPreference::Normal)
    .prepare()
    .unwrap();
    let (window, view) =
        super::mount_prepared(cx, prepared, ("trace", "gallery"), false, |host, view| {
            Host { host, view }
        });
    let mut v = VisualTestContext::from_window(*window, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1380.), px(1040.)));
    ready(&mut v, &view);
    clock.advance(Duration::from_millis(850));
    ready(&mut v, &view);
    let start = llm_bar(&mut v, &view);
    click(&mut v, &view, "button", "Advance trace clock");
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    clock.advance(Duration::from_millis(230));
    ready(&mut v, &view);
    let middle = llm_bar(&mut v, &view);
    assert!(
        middle.1 < start.1 - 3. && middle.1 > start.1 * 0.5 + 3.,
        "{start:?} -> {middle:?}"
    );
    clock.advance(Duration::from_millis(600));
    ready(&mut v, &view);
    let finish = llm_bar(&mut v, &view);
    assert!((finish.1 / start.1 - 0.5).abs() < 0.035);
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
