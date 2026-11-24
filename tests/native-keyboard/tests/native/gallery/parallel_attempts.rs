use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, UiValue};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, Mount, PresetRisks, bounds, glyphs_with_ink, snapshot};

#[gpui::test]
fn parallel_attempts_reviewed_ink_is_scoped_to_the_actual_fixture_roles(cx: &mut TestAppContext) {
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    for (preset, expected) in [
        ("modern-light", 15),
        ("modern-dark", 0),
        ("clean-light", 15),
        ("clean-dark", 13),
    ] {
        let (mut visual, view) = gallery(cx, "parallel_attempts", preset);
        visual.simulate_resize(size(px(1380.), px(980.)));
        for density in ["comfortable", "compact"] {
            super::acceptance::action(
                &mut visual,
                &view,
                "gallery.set_density",
                UiValue::String(density.into()),
            );
            ready(&mut visual, &view);
            let (stock, findings) = visual.update(|_, cx| {
                (
                    view.theme_snapshot(cx).unwrap().variant,
                    view.composition_audit_with(&rules, cx).unwrap(),
                )
            });
            let attempts = findings
                .iter()
                .filter(|finding| {
                    finding.path.contains("/page-parallel_attempts/")
                        && finding.path.contains("/sized/parallel-attempts/")
                })
                .collect::<Vec<_>>();
            assert_eq!(attempts.len(), expected, "{preset}/{density}: {attempts:?}");
            for finding in attempts {
                assert!(
                    risks.record(&stock, finding),
                    "{preset}/{density}: {finding:?}"
                );
                let mut wrong = finding.clone();
                wrong.contrast.as_mut().unwrap().background_srgb = Some([0.4; 3]);
                assert!(!risks.record(&stock, &wrong));
                wrong.contrast.clone_from(&finding.contrast);
                wrong.contrast.as_mut().unwrap().element_opacity = 0.5;
                assert!(!risks.record(&stock, &wrong));
                wrong.contrast.clone_from(&finding.contrast);
                wrong.path.push_str("/unreviewed-child");
                assert!(!risks.record(&stock, &wrong));
            }
        }
    }
    risks.assert_reported();
}

#[gpui::test]
#[cfg(target_os = "macos")]
fn parallel_attempts_uses_the_name_line_height_and_emphasizes_total_values(
    cx: &mut TestAppContext,
) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("parallel_attempt_roles"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let audit = gpui_rhai::AuditRules::from_ids(["mixed-type-in-row"]).unwrap();
    for (theme, primary, muted, title_ink) in [
        ("modern-light", 0x171717ff, 0x8a8a8aff, 0x171717ff),
        ("modern-dark", 0xf0f0f0ff, 0x8a8a8aff, 0xf5f5f5ff),
        ("clean-light", 0x000000e0, 0x00000073, 0x000000ff),
        ("clean-dark", 0xffffffe6, 0xffffff73, 0xffffffff),
    ] {
        let (mut v, view) = gallery(&mut real, "parallel_attempts", theme);
        v.simulate_resize(size(px(1380.), px(1000.)));
        ready(&mut v, &view);
        for (scale, line_height) in [("100", 20.), ("150", 30.)] {
            super::acceptance::action(
                &mut v,
                &view,
                "gallery.set_scale",
                UiValue::String(scale.into()),
            );
            ready(&mut v, &view);
            for name in ["Attempt A", "Atlas Mini"] {
                let label = bounds(&mut v, &view, "text", name);
                assert!(
                    (label.height - line_height).abs() <= 0.5,
                    "{theme}/{scale}, {name}: {label:?}"
                );
            }
            let stats = bounds(&mut v, &view, "status", "1 of 3 done · $0.32");
            assert!(
                glyphs_with_ink(&mut v, stats, primary) >= 5,
                "{theme}/{scale}: stats values must use primary ink"
            );
            assert!(
                glyphs_with_ink(&mut v, stats, muted) >= 5,
                "{theme}/{scale}: the stats suffix must keep its own ink"
            );
            let model = bounds(&mut v, &view, "text", "Atlas Mini");
            assert!(
                glyphs_with_ink(&mut v, model, muted) >= 5,
                "{theme}/{scale}: model ink"
            );
            let title = bounds(&mut v, &view, "text", "Best of 3");
            assert!(
                glyphs_with_ink(&mut v, title, title_ink) >= 5,
                "{theme}/{scale}: title ink"
            );
            let findings = v.update(|_, cx| view.composition_audit_with(&audit, cx).unwrap());
            assert!(
                !findings
                    .iter()
                    .any(|finding| finding.path.contains("/parallel-attempts")),
                "{theme}/{scale}: {findings:#?}"
            );
        }
    }
}

#[gpui::test]
fn parallel_attempts_settle_without_moving_cards_and_keep_keyboard_pick_on_resize(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "parallel_attempts", "modern-light");
    v.simulate_resize(size(px(1250.), px(950.)));
    ready(&mut v, &view);
    let live = bounds(&mut v, &view, "group", "Attempt A");
    node(&mut v, &view, "status", "1 of 3 done · $0.32");
    assert!(!snapshot(&mut v, &view).nodes().any(|n| n.role == "radio"));
    click(&mut v, &view, "button", "Advance progress");
    assert_eq!(bounds(&mut v, &view, "group", "Attempt A"), live);
    click(&mut v, &view, "button", "Finish trials");
    assert_eq!(bounds(&mut v, &view, "group", "Attempt A"), live);
    assert_eq!(
        node(&mut v, &view, "radio", "Using Attempt A").checked,
        Some(UiValue::Bool(true))
    );
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("radio", "Use Attempt C")
            .next()
            .is_none()
    );
    click(&mut v, &view, "radio", "Using Attempt A");
    v.simulate_keystrokes("down");
    ready(&mut v, &view);
    let focus = v.update(|window, cx| window.focused(cx)).unwrap();
    let id = node(&mut v, &view, "radio", "Use Attempt B").id;
    v.simulate_resize(size(px(740.), px(950.)));
    ready(&mut v, &view);
    let narrow = bounds(&mut v, &view, "group", "Attempt A");
    assert!(narrow.width < live.width && narrow.height > live.height);
    assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focus));
    assert_eq!(node(&mut v, &view, "radio", "Use Attempt B").id, id);
    v.simulate_keystrokes("space");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Selected trace at revision 2");
    node(&mut v, &view, "radio", "Using Attempt B");
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn parallel_attempts_quiet_row_uses_group_opacity_and_restores_on_keyboard_focus(
    cx: &mut TestAppContext,
) {
    use crate::support::{logical_paint_rect, walk_live_paint};

    for (theme, quiet) in [("modern-light", 0.55), ("clean-dark", 0.50)] {
        let (mut v, view) = gallery(cx, "parallel_attempts", theme);
        v.simulate_resize(size(px(1320.), px(1000.)));
        ready(&mut v, &view);
        click(&mut v, &view, "button", "Finish trials");
        let row = bounds(&mut v, &view, "group", "Attempt B");
        let id = node(&mut v, &view, "radio", "Use Attempt B").id;
        let fades = |v: &mut VisualTestContext| {
            v.update(|window, _| {
                let scale = window.scale_factor();
                let midpoint = (row.x + row.width * 0.5, row.y + row.height * 0.5);
                window.visual_probe_with_painted_scene(|scene| {
                    let mut alphas = Vec::new();
                    walk_live_paint(
                        scene,
                        gpui::TransformationMatrix::default(),
                        &mut |scene, matrix| {
                            for group in &scene.isolated_groups {
                                let rect = logical_paint_rect(group.bounds, matrix, scale);
                                if f64::from(rect.x) <= midpoint.0
                                    && midpoint.0 < f64::from(rect.right())
                                    && f64::from(rect.y) <= midpoint.1
                                    && midpoint.1 < f64::from(rect.bottom())
                                    && f64::from(rect.width) >= row.width * 0.8
                                    && f64::from(rect.height) <= row.height + 24.0
                                {
                                    alphas.push(group.opacity());
                                }
                            }
                        },
                    );
                    alphas
                })
            })
        };
        let dimmed = fades(&mut v);
        assert!(
            dimmed.iter().any(|alpha| (*alpha - quiet).abs() < 0.001),
            "{theme}: whole-row isolated fade missing: {dimmed:?}"
        );
        click(&mut v, &view, "radio", "Using Attempt A");
        v.simulate_keystrokes("down");
        ready(&mut v, &view);
        let focused = fades(&mut v);
        assert!(
            focused.iter().all(|alpha| *alpha >= 0.999),
            "{theme}: keyboard focus must clear the fade: {focused:?}"
        );
        let choice = node(&mut v, &view, "radio", "Use Attempt B");
        assert_eq!(choice.id, id);
        assert_eq!(choice.checked, Some(UiValue::Bool(false)));
        assert_eq!(bounds(&mut v, &view, "group", "Attempt B"), row);
    }
}

#[gpui::test]
fn parallel_attempts_leaves_controlled_selection_to_host_in_all_presets(cx: &mut TestAppContext) {
    for theme in ["modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "parallel_attempts", theme);
        v.simulate_resize(size(px(1230.), px(930.)));
        ready(&mut v, &view);
        click(&mut v, &view, "button", "Host-controlled pick");
        click(&mut v, &view, "button", "Finish trials");
        assert_eq!(
            node(&mut v, &view, "radio", "Use Attempt A").checked,
            Some(UiValue::Bool(false))
        );
        click(&mut v, &view, "button", "Refuse next pick");
        click(&mut v, &view, "radio", "Use Attempt A");
        node(&mut v, &view, "status", "The Host kept its selection");
        assert_eq!(
            node(&mut v, &view, "radio", "Use Attempt A").checked,
            Some(UiValue::Bool(false))
        );
        v.simulate_keystrokes("space");
        ready(&mut v, &view);
        node(&mut v, &view, "status", "Selected api/界 at revision 1");
        node(&mut v, &view, "radio", "Using Attempt A");
        click(&mut v, &view, "button", "Narrow card");
        assert!(bounds(&mut v, &view, "group", "Attempt A").width <= 370.5);
        click(&mut v, &view, "button", "Empty attempts");
        node(&mut v, &view, "status", "0 of 0 done · $0");
        assert!(!snapshot(&mut v, &view).nodes().any(|n| n.role == "radio"));
    }
}

#[gpui::test]
fn parallel_attempts_at_capacity_ties_and_credits_remain_exact(cx: &mut TestAppContext) {
    let (mut v, view) = Mount::new(
        r#"
        import "patterns/parallel_attempts" as a;
        fn view(ctx){
            let attempts=[];
            for i in 0..24{
                attempts.push(#{id:"attempt/"+i.to_string(),name:"Attempt "+i.to_string(),
                    status:if i==0{"failed"}else{"done"},score:if i==0{100}else{94},
                    progress:0.37,cost_micros:if i==0{-30000}else{1000}});
            }
            column([a::ParallelAttempts(#{key:"tie",run:"tied",attempts:attempts})]).with_style(style().width(px(672)))
        }
        "#,
    )
    .registry("patterns/parallel_attempts")
    .motion(MotionPreference::None)
    .mount(cx);
    ready(&mut v, &view);
    node(&mut v, &view, "status", "24 of 24 done · −$0.007");
    assert_eq!(
        node(&mut v, &view, "radio", "Using Attempt 1").checked,
        Some(UiValue::Bool(true))
    );
    assert_eq!(
        node(&mut v, &view, "radio", "Use Attempt 2").checked,
        Some(UiValue::Bool(false))
    );
}

#[gpui::test]
fn parallel_attempts_never_resurrects_a_pick_from_a_previous_run(cx: &mut TestAppContext) {
    let (mut v, view) = Mount::new(
        r#"
        import "patterns/parallel_attempts" as a;
        fn state_schema(){#{fields:#{other:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn toggle(ctx,e){ctx.set_state("other",!ctx.get_state("other"));}
        fn view(ctx){
            let change=text("Switch run").with_key("toggle").accessibility_role("button")
                .accessibility_label("Switch run").on_click(Fn("toggle"));
            let attempts=[#{id:"a",name:"Attempt A",status:"done",score:95},#{id:"b",name:"Attempt B",status:"done",score:88}];
            column([change,a::ParallelAttempts(#{key:"same",run:if ctx.get_state("other"){"second"}else{"first"},
                attempts:attempts})]).with_style(style().width(px(660)))
        }
        "#,
    )
    .registry("patterns/parallel_attempts")
    .motion(MotionPreference::None)
    .mount(cx);
    click(&mut v, &view, "radio", "Use Attempt B");
    node(&mut v, &view, "radio", "Using Attempt B");
    click(&mut v, &view, "button", "Switch run");
    node(&mut v, &view, "radio", "Using Attempt A");
    click(&mut v, &view, "button", "Switch run");
    node(&mut v, &view, "radio", "Using Attempt A");
}

#[gpui::test]
fn parallel_attempts_best_badge_scales_once_per_mount(cx: &mut TestAppContext) {
    for (theme, preference) in [
        ("modern-light", MotionPreference::Normal),
        ("clean-dark", MotionPreference::Normal),
        ("clean-light", MotionPreference::Reduced),
        ("modern-dark", MotionPreference::None),
    ] {
        let clock = ManualRuntimeClock::new(Instant::now());
        let prepared = acceptance::view(&AcceptanceLaunch {
            page: "parallel_attempts".into(),
            theme: theme.into(),
            ..Default::default()
        })
        .unwrap()
        .motion_preference(preference)
        .runtime_clock(clock.clock())
        .prepare()
        .unwrap();
        let (window, view) = super::mount_prepared(
            cx,
            prepared,
            ("race-best", "gallery"),
            false,
            |host, view| Host { host, view },
        );
        let mut v = VisualTestContext::from_window(*window, cx);
        v.update(|window, _| window.activate_window());
        v.simulate_resize(size(px(1320.), px(1000.)));
        clock.advance(Duration::from_millis(800));
        ready(&mut v, &view);
        click(&mut v, &view, "button", "Finish trials");
        let badge = |v: &mut VisualTestContext| node(v, &view, "text", "Best");
        let check = |v: &mut VisualTestContext, expected: f64| {
            let g = badge(v).geometry.unwrap();
            let sx = g.visual.width / g.layout.width;
            let sy = g.visual.height / g.layout.height;
            assert!(
                (sx - expected).abs() < 0.015 && (sy - expected).abs() < 0.015,
                "{theme}/{preference:?} expected {expected}, got {sx} × {sy}: {g:?}"
            );
            assert!(
                (g.visual.x + g.visual.width / 2. - g.layout.x - g.layout.width / 2.).abs() < 0.5
            );
            assert!(
                (g.visual.y + g.visual.height / 2. - g.layout.y - g.layout.height / 2.).abs() < 0.5
            );
        };
        if preference != MotionPreference::Normal {
            check(&mut v, 1.);
            continue;
        }
        let identity = badge(&mut v).id;
        check(&mut v, 0.80);
        clock.advance(Duration::from_millis(252));
        ready(&mut v, &view);
        check(&mut v, 1.06);

        click(&mut v, &view, "button", "Advance progress");
        assert_eq!(badge(&mut v).id, identity);
        check(&mut v, 1.06);
        clock.advance(Duration::from_millis(180));
        ready(&mut v, &view);
        check(&mut v, 1.0);

        let (family, variant) = if theme == "modern-light" {
            ("Clean", "Light")
        } else {
            ("Modern", "Dark")
        };
        v.update(|_, cx| view.select_theme(family, variant, cx))
            .unwrap();
        ready(&mut v, &view);
        clock.advance(Duration::from_millis(252));
        ready(&mut v, &view);
        assert_eq!(badge(&mut v).id, identity);
        check(&mut v, 1.0);

        click(&mut v, &view, "button", "Show live trials");
        assert!(
            !snapshot(&mut v, &view)
                .nodes()
                .any(|n| n.role == "text" && n.name == "Best")
        );
        click(&mut v, &view, "button", "Finish trials");
        assert_ne!(badge(&mut v).id, identity);
        check(&mut v, 0.80);
    }
}

// Depends on the vendored renderer's native scene/input witness.
#[cfg(feature = "vendor-gpui")]
#[gpui::test]
fn parallel_attempts_live_fill_retargets_without_script_frames(cx: &mut TestAppContext) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "parallel_attempts".to_owned(),
        theme: "modern-light".to_owned(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::Normal)
    .runtime_clock(clock.clock())
    .prepare()
    .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("race-motion", "gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut v = VisualTestContext::from_window(*window, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1250.), px(950.)));
    clock.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    fn scene_width(scene: &gpui::Scene, offset: f32, scale: f32, top: f32, bottom: f32) -> f32 {
        let blue: gpui::Background = gpui::rgba(0xe3f3_ffff).into();
        let mut max = 0.0_f32;
        for quad in &scene.quads {
            let y = (offset + quad.bounds.origin.y.0) / scale;
            let height = quad.bounds.size.height.0 / scale;
            if quad.background == blue
                && (8.0..=12.0).contains(&height)
                && y > top
                && y + height < bottom
            {
                max = max.max(quad.bounds.size.width.0 / scale);
            }
        }
        for group in &scene.isolated_groups {
            // These cards only translate; their local paint must be placed
            // back in window coordinates before matching the visible row.
            max = max.max(scene_width(
                group.source(),
                offset + group.transform().translation[1],
                scale,
                top,
                bottom,
            ));
        }
        #[cfg(feature = "gpui-fast")]
        for layer in scene
            .layers
            .frames
            .iter()
            .filter(|layer| layer.tile_size == 0)
        {
            for child in layer.content.scenes() {
                max = max.max(scene_width(child, offset, scale, top, bottom));
            }
        }
        max
    }
    let width = |v: &mut VisualTestContext| {
        let row = bounds(v, &view, "group", "Attempt A");
        let value = v.update(|window, _| {
            window.visual_probe_with_painted_scene(|scene| {
                scene_width(
                    scene,
                    0.0,
                    window.scale_factor(),
                    row.y as f32,
                    (row.y + row.height) as f32,
                )
            })
        });
        assert!(value > 0.0, "running bar in Attempt A at {row:?}");
        value
    };
    let before = width(&mut v);
    click(&mut v, &view, "button", "Advance progress");
    assert!((before - width(&mut v)).abs() < 1.0);
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    clock.advance(Duration::from_millis(100));
    ready(&mut v, &view);
    let middle = width(&mut v);
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
    clock.advance(Duration::from_millis(620));
    ready(&mut v, &view);
    let after = width(&mut v);
    assert!(
        before + 1.0 < middle && middle + 1.0 < after,
        "{before} → {middle} → {after}"
    );
}

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
