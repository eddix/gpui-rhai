use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{
    AuditRules, ColorValue, Length, LetterSpacing, ManualRuntimeClock, MotionPreference, UiNode,
    UiNodeKind, UiValue,
};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::acceptance::action;
use super::collaboration_stream::glyph_band_center;
use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, Mount, PresetRisks, bounds, live_fills, snapshot};

#[gpui::test]
fn root_cause_tree_keeps_the_original_caption_risks_on_their_own_surfaces(cx: &mut TestAppContext) {
    let mut risks = PresetRisks::new();
    let contrast = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let (mut visual, view) = gallery(cx, "root_cause_tree", "modern-light");
    visual.simulate_resize(size(px(1450.), px(980.)));
    for (family, appearance) in [
        ("Modern", "Light"),
        ("Modern", "Dark"),
        ("Clean", "Light"),
        ("Clean", "Dark"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        for density in ["comfortable", "compact"] {
            action(
                &mut visual,
                &view,
                "gallery.set_density",
                UiValue::String(density.to_owned()),
            );
            ready(&mut visual, &view);
            let (theme, findings) = visual.update(|_, cx| {
                (
                    view.theme_snapshot(cx).unwrap().variant,
                    view.composition_audit_with(&contrast, cx).unwrap(),
                )
            });
            let specimen = findings.iter().filter(|f| f.path.contains("/sized/case/"));
            let source = specimen
                .clone()
                .find(|f| f.path.ends_with("/evidence-record/caption/source"))
                .expect("faint source caption remains in the raw audit");
            assert!(risks.record(&theme, source));
            let mut wrong_paper = source.clone();
            wrong_paper.contrast.as_mut().unwrap().background_srgb = Some([0.5; 3]);
            assert!(!risks.record(&theme, &wrong_paper));
            let unexplained = specimen
                .filter(|f| !risks.record(&theme, f))
                .collect::<Vec<_>>();
            assert!(
                unexplained.is_empty(),
                "{family}/{appearance}/{density}: {unexplained:#?}"
            );
        }
    }
    risks.assert_reported();
}

fn descendant<'a>(node: &'a UiNode, predicate: &impl Fn(&UiNode) -> bool) -> Option<&'a UiNode> {
    if predicate(node) {
        return Some(node);
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => children
            .iter()
            .find_map(|child| descendant(child, predicate)),
        UiNodeKind::Overlay { trigger, .. } => descendant(trigger, predicate),
        _ => None,
    }
}

fn by_key<'a>(node: &'a UiNode, key: &str) -> &'a UiNode {
    descendant(node, &|n| n.key().is_some_and(|k| k.as_str() == key))
        .unwrap_or_else(|| panic!("could not find {key}"))
}

#[gpui::test]
fn root_cause_tree_reuses_the_supplied_icon_in_the_selected_evidence(cx: &mut TestAppContext) {
    let source = r#"
        import "patterns/root_cause_tree" as tree;
        fn view(ctx){tree::RootCauseTree(#{key:"tree",session:"marked",causes:[
            #{id:"fault",title:"Failure"},#{id:"case",parent:"fault",title:"Marked cause",
                icon:box([]).with_style(style().width(px(6)).height(px(6)).background(rgb(0xb32876))),
                verdict:"Accepted evidence"}]}).with_style(style().width(px(350)))}
    "#;
    for theme in [
        gpui_rhai_registry::MODERN_DARK_THEME,
        gpui_rhai_registry::CLEAN_LIGHT_THEME,
    ] {
        let (mut v, view) = Mount::new(source)
            .registry("patterns/root_cause_tree")
            .theme(theme)
            .motion(MotionPreference::None)
            .mount(cx);
        v.update(|window, _| window.activate_window());
        ready(&mut v, &view);
        assert_eq!(live_fills(&mut v, 0xb32876ff).len(), 1);
        click(&mut v, &view, "treeitem", "Marked cause");
        let heading = bounds(&mut v, &view, "heading", "Marked cause evidence");
        let marks = live_fills(&mut v, 0xb32876ff);
        assert_eq!(marks.len(), 2, "diagram icon and evidence icon: {marks:?}");
        assert!(
            marks.iter().any(|mark| f64::from(mark.right()) <= heading.x
                && (f64::from(mark.y) - heading.y).abs() < 12.),
            "missing icon beside evidence heading: {heading:?} {marks:?}"
        );
        node(&mut v, &view, "text", "Accepted evidence");
    }
}

#[gpui::test]
fn root_cause_tree_keyboard_routes_similar_unicode_keys_without_aliasing(cx: &mut TestAppContext) {
    let (mut v, view) = Mount::new(
        r#"
        import "patterns/root_cause_tree" as tree;
        fn view(ctx){
            tree::RootCauseTree(#{key:"tree",session:"unicode-references",causes:[
                #{id:"first",title:"Failure"},
                #{id:"case/🧪",parent:"first",title:"Slash case",status:"checking"},
                #{id:"case:🧪",parent:"first",title:"Colon case",status:"open"}
            ]}).with_style(style().width(px(350)))
        }
    "#,
    )
    .registry("patterns/root_cause_tree")
    .theme(gpui_rhai_registry::CLEAN_DARK_THEME)
    .motion(MotionPreference::None)
    .mount(cx);
    v.update(|window, _| window.activate_window());
    ready(&mut v, &view);
    click(&mut v, &view, "treeitem", "Failure");
    for (key, label) in [
        ("end", "Colon case"),
        ("home", "Failure"),
        ("down", "Slash case"),
    ] {
        v.simulate_keystrokes(key);
        ready(&mut v, &view);
        let goal = node(&mut v, &view, "treeitem", label);
        let focus = v.update(|window, cx| view.debug_frame_digest(window, cx).unwrap().focus_path);
        assert_eq!(focus.first(), Some(&goal.id), "{key} must reach {label}");
    }
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    node(&mut v, &view, "heading", "Slash case evidence");
}

#[gpui::test]
fn root_cause_tree_file_names_use_their_own_body_ink_and_caption_size(cx: &mut TestAppContext) {
    for theme in ["modern-light", "modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "root_cause_tree", theme);
        v.simulate_resize(size(px(1380.), px(1050.)));
        ready(&mut v, &view);
        let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
        let style = &descendant(&root, &|node| {
            matches!(node.kind(), UiNodeKind::Text { text } if text.as_str() == "publish.rhai")
        })
            .expect("preview has a source filename")
            .style()
            .base;
        assert_eq!(
            style.text_color,
            Some(ColorValue::Token("root_cause.text".into())),
            "{theme}"
        );
        assert_eq!(style.font_size, Some(Length::Pixels(11.)), "{theme}");
        assert_eq!(style.line_height, Some(Length::Pixels(16.)), "{theme}");
        assert_eq!(style.font_weight, Some(400), "{theme}");
    }
}

#[gpui::test]
fn root_cause_tree_evidence_roles_keep_their_style_and_do_not_invent_source_metadata(
    cx: &mut TestAppContext,
) {
    for theme in ["modern-light", "modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "root_cause_tree", theme);
        v.simulate_resize(size(px(1380.), px(1050.)));
        ready(&mut v, &view);
        let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
        let evidence = by_key(&root, "evidence-record");
        let kind = &by_key(evidence, "kind").style().base;
        let source = &by_key(evidence, "source").style().base;
        let panel = &by_key(by_key(&root, "selected:token"), "panel-status")
            .style()
            .base;
        assert_eq!(kind.font_size, Some(Length::Pixels(10.)), "{theme}");
        assert_eq!(source.font_size, kind.font_size, "{theme}");
        assert_eq!(source.line_height, kind.line_height, "{theme}");
        assert_eq!(
            source.letter_spacing,
            Some(LetterSpacing::Em(0.)),
            "{theme}"
        );
        assert_eq!(panel.font_size, Some(Length::Pixels(12.)), "{theme}");
        assert_eq!(panel.line_height, Some(Length::Pixels(16.)), "{theme}");
        let quiet = &by_key(by_key(&root, "selected:token"), "verdict")
            .style()
            .base;
        assert_eq!(
            quiet.text_color,
            Some(ColorValue::Token("root_cause.muted".into()))
        );
        click(&mut v, &view, "treeitem", "Missing draft");
        let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
        let evidence = by_key(&root, "evidence-record");
        assert!(
            descendant(evidence, &|n| n
                .key()
                .is_some_and(|k| k.as_str() == "source"))
            .is_none()
        );
        let verdict = &by_key(by_key(&root, "selected:artifact"), "verdict")
            .style()
            .base;
        assert_eq!(
            verdict.text_color,
            Some(ColorValue::Token("root_cause.text".into()))
        );
    }
}

#[gpui::test]
fn root_cause_tree_gate_and_cards_share_the_committed_board_at_all_reading_scales(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "root_cause_tree", "modern-dark");
    v.simulate_resize(size(px(1380.), px(1050.)));
    ready(&mut v, &view);
    let root_id = node(&mut v, &view, "treeitem", "Publish job failed").id;
    let contrast = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    for density in ["comfortable", "compact"] {
        action(
            &mut v,
            &view,
            "gallery.set_density",
            UiValue::String(density.into()),
        );
        for (scale, factor) in [("100", 1.), ("125", 1.25), ("150", 1.5), ("100", 1.)] {
            action(
                &mut v,
                &view,
                "gallery.set_scale",
                UiValue::String(scale.into()),
            );
            ready(&mut v, &view);
            let root = bounds(&mut v, &view, "treeitem", "Publish job failed");
            let child = bounds(&mut v, &view, "treeitem", "Publisher lacks access");
            let (label, and) = v.update(|window, cx| {
                let findings = view.composition_audit_with(&contrast, cx).unwrap();
                let digest = view.debug_frame_digest(window, cx).unwrap();
                let lookup = |suffix: &str| {
                    let id = findings
                        .iter()
                        .find(|f| f.path.ends_with(suffix))
                        .unwrap_or_else(|| panic!("missing painted gate {suffix} in {findings:#?}"))
                        .node
                        .unwrap();
                    digest
                        .geometry
                        .iter()
                        .find(|(i, _)| *i == id)
                        .unwrap()
                        .1
                        .visual
                };
                (lookup("/gate:failure/pill/0"), lookup("/gate:auth/pill/0"))
            });
            assert!(
                (root.height - 84. * factor).abs() <= 1.,
                "{density}/{scale}: {root:?}"
            );
            assert!(
                (child.y - root.y - 128. * factor).abs() <= 1.,
                "{density}/{scale}: {root:?} {child:?}"
            );
            assert!(
                (label.x + label.width / 2. - root.x - root.width / 2.).abs() <= 1.,
                "{density}/{scale}: {label:?} {root:?}"
            );
            assert!(
                (and.x + and.width / 2. - child.x - child.width / 2.).abs() <= 1.,
                "{density}/{scale}: {and:?} {child:?}"
            );
            assert!(
                (label.y + label.height / 2. - root.y - root.height - 44. * 0.31 * factor).abs()
                    <= 1.,
                "{density}/{scale}: {label:?} {root:?}"
            );
            assert_eq!(
                node(&mut v, &view, "treeitem", "Publish job failed").id,
                root_id
            );
        }
    }
}

#[gpui::test]
fn root_cause_tree_exposes_actions_only_after_the_and_gate_is_complete(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "root_cause_tree", "modern-light");
    v.simulate_resize(size(px(1380.), px(1050.)));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Investigating");
    node(&mut v, &view, "heading", "Wrong job identity evidence");
    click(&mut v, &view, "treeitem", "Role not granted");
    assert_eq!(
        node(&mut v, &view, "treeitem", "Role not granted").description,
        "Confirmed"
    );
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Fix it")
            .next()
            .is_none()
    );
    click(&mut v, &view, "button", "Confirm final evidence");
    node(&mut v, &view, "status", "Cause found");
    assert_eq!(
        node(&mut v, &view, "treeitem", "Role not granted").description,
        "Root cause"
    );
    node(&mut v, &view, "document", "Policy record");
    node(&mut v, &view, "button", "Fix it");
    click(&mut v, &view, "treeitem", "Missing draft");
    node(&mut v, &view, "document", "Artifact log");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Fix it")
            .next()
            .is_none()
    );
}

#[gpui::test]
fn root_cause_tree_rejections_keep_the_accepted_evidence_and_allow_retry(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "root_cause_tree", "clean-dark");
    v.simulate_resize(size(px(1380.), px(1050.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Confirm final evidence");
    click(&mut v, &view, "treeitem", "Role not granted");
    click(&mut v, &view, "button", "Refuse next action");
    click(&mut v, &view, "button", "Fix it");
    node(&mut v, &view, "status", "Action proposed: fix");
    assert!(node(&mut v, &view, "button", "Fix it").disabled);
    v.executor().advance_clock(Duration::from_millis(600));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Action refused by Host");
    node(&mut v, &view, "document", "Policy record");
    assert!(!node(&mut v, &view, "button", "Retry from here").disabled);
    click(&mut v, &view, "button", "Retry from here");
    v.executor().advance_clock(Duration::from_millis(600));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Action accepted: retry");
    node(&mut v, &view, "status", "Cause found");
}

#[gpui::test]
fn root_cause_tree_navigation_and_identities_survive_reflow(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "root_cause_tree", "modern-dark");
    v.simulate_resize(size(px(1380.), px(1050.)));
    ready(&mut v, &view);
    let branch = bounds(&mut v, &view, "treeitem", "Publisher lacks access");
    let peer = bounds(&mut v, &view, "treeitem", "Missing draft");
    assert!((branch.y - peer.y).abs() <= 1. && peer.x > branch.x + branch.width);
    click(&mut v, &view, "treeitem", "Publisher lacks access");
    v.simulate_keystrokes("right enter");
    ready(&mut v, &view);
    node(&mut v, &view, "heading", "Missing draft evidence");
    let stable = node(&mut v, &view, "treeitem", "Role not granted").id;
    click(&mut v, &view, "button", "Narrow fault tree");
    assert_eq!(
        node(&mut v, &view, "treeitem", "Role not granted").id,
        stable
    );
    let branch = bounds(&mut v, &view, "treeitem", "Publisher lacks access");
    let child = bounds(&mut v, &view, "treeitem", "Role not granted");
    let peer = bounds(&mut v, &view, "treeitem", "Missing draft");
    assert!(child.x > branch.x && child.y > branch.y + branch.height && peer.y > child.y);
    click(&mut v, &view, "treeitem", "Publisher lacks access");
    v.simulate_keystrokes("down enter");
    ready(&mut v, &view);
    node(&mut v, &view, "heading", "Role not granted evidence");
}

fn proof_extent(scene: &gpui::Scene) -> f32 {
    let mut h = 0.;
    for p in &scene.paths {
        let Some(paint) = p.color.as_solid() else {
            continue;
        };
        let c = paint.to_rgb();
        if (c.r - 131. / 255.).abs() < 0.009
            && (c.g - 199. / 255.).abs() < 0.009
            && c.b > 0.98
            && c.a > 0.95
        {
            h += p.bounds.size.width.0 + p.bounds.size.height.0;
        }
    }
    #[cfg(feature = "gpui-fast")]
    for layer in scene.layers.frames.iter().filter(|l| l.tile_size == 0) {
        h += layer.content.scenes().map(proof_extent).sum::<f32>();
    }
    h
}

#[gpui::test]
fn root_cause_tree_accepted_path_draws_on_the_native_clock(cx: &mut TestAppContext) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "root_cause_tree".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .runtime_clock(clock.clock())
    .motion_preference(MotionPreference::Normal)
    .prepare()
    .unwrap();
    let (win, view) =
        super::mount_prepared(cx, prepared, ("fault", "gallery"), false, |host, view| {
            Host { host, view }
        });
    let mut v = VisualTestContext::from_window(*win, cx);
    v.update(|win, _| win.activate_window());
    v.simulate_resize(size(px(1380.), px(1050.)));
    ready(&mut v, &view);
    let measure = |v: &mut VisualTestContext| {
        v.update(|window, _| window.visual_probe_with_painted_scene(proof_extent))
    };
    assert_eq!(measure(&mut v), 0.);
    click(&mut v, &view, "button", "Confirm final evidence");
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    clock.advance(Duration::from_millis(130));
    ready(&mut v, &view);
    let middle = measure(&mut v);
    clock.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    let end = measure(&mut v);
    assert!(middle > 0. && end > middle + 10., "{middle} to {end}");
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
}

#[gpui::test]
fn root_cause_tree_checking_uses_a_glyph_sweep_and_clears_it_on_proof(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;

    let mut native = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("fault_tree_glyph_sweep"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (window_id, preference) in [
        ("normal", MotionPreference::Normal),
        ("reduced", MotionPreference::Reduced),
        ("disabled", MotionPreference::None),
    ] {
        let clock = ManualRuntimeClock::new(Instant::now());
        let prepared = acceptance::view(&AcceptanceLaunch {
            page: "root_cause_tree".into(),
            theme: "clean-dark".into(),
            ..Default::default()
        })
        .unwrap()
        .runtime_clock(clock.clock())
        .motion_preference(preference)
        .prepare()
        .unwrap();
        let (win, view) = super::mount_prepared(
            &mut native,
            prepared,
            (window_id, "gallery"),
            false,
            |host, view| Host { host, view },
        );
        let mut v = VisualTestContext::from_window(*win, &native);
        v.update(|window, _| window.activate_window());
        v.simulate_resize(size(px(1380.), px(1050.)));
        ready(&mut v, &view);
        let before = node(&mut v, &view, "treeitem", "Wrong job identity");
        let sample = |v: &mut VisualTestContext| {
            v.update(|w, _| w.visual_probe_with_painted_scene(glyph_band_center))
        };
        clock.advance(Duration::from_millis(350));
        ready(&mut v, &view);
        let left = sample(&mut v);
        clock.advance(Duration::from_millis(200));
        ready(&mut v, &view);
        let right = sample(&mut v);
        if preference == MotionPreference::Normal {
            assert!(
                right.unwrap() > left.unwrap() + 10.,
                "{left:?} -> {right:?}"
            );
        } else {
            assert_eq!((left, right), (None, None));
        }
        click(&mut v, &view, "button", "Confirm final evidence");
        node(&mut v, &view, "status", "Cause found");
        let after = node(&mut v, &view, "treeitem", "Wrong job identity");
        assert_eq!(before.id, after.id);
        assert_eq!(sample(&mut v), None);
    }
}
