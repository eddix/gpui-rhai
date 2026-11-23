use std::time::{Duration, Instant};

use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{
    Host, Mount, bounds, center, filled_including_rounded_clips, glyphs_with_ink, snapshot,
};

#[gpui::test]
fn plan_vs_actual_finish_marker_uses_body_ink_on_its_wash(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("plan_finish_marker_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    let (mut v, view) = gallery(&mut real, "plan_vs_actual", "modern-light");
    v.simulate_resize(size(px(1450.), px(980.)));
    for (design, appearance, ink, wash) in [
        ("Modern", "Light", 0x0000009c, 0x0000000b),
        ("Modern", "Dark", 0xffffff9e, 0xffffff12),
        ("Clean", "Light", 0x0000009c, 0x0000000b),
        ("Clean", "Dark", 0xffffff9e, 0xffffff12),
    ] {
        v.update(|_, cx| view.select_theme(design, appearance, cx))
            .unwrap();
        ready(&mut v, &view);
        let paper = bounds(&mut v, &view, "text", "Plan 2:00");
        assert!(
            glyphs_with_ink(&mut v, paper, ink) >= 6,
            "{design}/{appearance}: finish text must use body ink"
        );
        assert!(
            crate::support::live_fills(&mut v, wash)
                .iter()
                .any(|rect| (f64::from(rect.x) - paper.x).abs() <= 1.
                    && (f64::from(rect.y) - paper.y).abs() <= 1.
                    && (f64::from(rect.width) - paper.width).abs() <= 1.
                    && (f64::from(rect.height) - paper.height).abs() <= 1.),
            "{design}/{appearance}: plan marker is painted on its own wash"
        );
    }
}

#[gpui::test]
fn plan_vs_actual_authored_task_risk_stays_in_the_raw_audit(cx: &mut TestAppContext) {
    use crate::support::PresetRisks;
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let mut risks = PresetRisks::new();
    for (theme, light) in [
        ("modern-light", true),
        ("modern-dark", false),
        ("clean-light", true),
        ("clean-dark", false),
    ] {
        let (mut v, view) = gallery(cx, "plan_vs_actual", theme);
        v.simulate_resize(size(px(1450.), px(980.)));
        ready(&mut v, &view);
        let (variant, findings) = v.update(|_, cx| {
            (
                view.theme_snapshot(cx).unwrap().variant,
                view.composition_audit_with(&rules, cx).unwrap(),
            )
        });
        let task = findings
            .iter()
            .find(|f| f.path.ends_with("/plan-vs-actual/head/title/task"))
            .unwrap_or_else(|| {
                panic!("{theme}: the raw audit lost the task warning: {findings:?}")
            });
        let ratio = task.contrast.as_ref().unwrap().ratio;
        assert!(
            (ratio - if light { 3.362683 } else { 4.454726 }).abs() < 0.01,
            "{theme}: {ratio}"
        );
        assert!(risks.record(&variant, task), "{theme}: {task:?}");
        let mut unrelated = task.clone();
        unrelated.path = unrelated.path.replace("/title/task", "/title/other-label");
        assert!(
            !risks.record(&variant, &unrelated),
            "{theme}: same ink does not exempt another slot"
        );
        let mut wrong_paint = task.clone();
        wrong_paint.contrast.as_mut().unwrap().element_opacity = 0.8;
        assert!(
            !risks.record(&variant, &wrong_paint),
            "{theme}: same slot with changed opacity is not an original risk"
        );
        if light {
            let late = findings
                .iter()
                .find(|f| f.path.ends_with("/plan-vs-actual/head/verdict"))
                .unwrap();
            assert!(risks.record(&variant, late), "{theme}: {late:?}");
        } else {
            let late = bounds(&mut v, &view, "status", "+18s behind");
            assert!(
                crate::support::live_fills(&mut v, 0xffffff12)
                    .into_iter()
                    .any(|rect| (f64::from(rect.x) - late.x).abs() <= 1.
                        && (f64::from(rect.y) - late.y).abs() <= 1.
                        && (f64::from(rect.width) - late.width).abs() <= 1.
                        && (f64::from(rect.height) - late.height).abs() <= 1.),
                "{theme}: the dark verdict uses the authored wash"
            );
        }
        for density in ["comfortable", "compact"] {
            super::acceptance::action(
                &mut v,
                &view,
                "gallery.set_density",
                gpui_rhai::UiValue::String(density.to_owned()),
            );
            ready(&mut v, &view);
            let findings = v.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
            let specimen = findings
                .iter()
                .filter(|f| f.path.contains("/sizing/plan-vs-actual/"))
                .collect::<Vec<_>>();
            assert!(
                specimen.len() > 8,
                "{theme}/{density}: the raw audit must still report authored risks"
            );
            let unexplained = specimen
                .into_iter()
                .filter(|f| !risks.record(&variant, f))
                .collect::<Vec<_>>();
            assert!(
                unexplained.is_empty(),
                "{theme}/{density}: {unexplained:#?}"
            );
        }
    }
    risks.assert_reported();
}

#[gpui::test]
fn plan_vs_actual_waiting_title_paints_body_ink_until_the_host_finishes_it(
    cx: &mut TestAppContext,
) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("plan_waiting_title_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, waiting, done) in [
        ("modern-dark", 0xffffff9e, 0xedededff),
        ("clean-light", 0x0000009c, 0x000000e0),
    ] {
        let (mut v, view) = gallery(&mut real, "plan_vs_actual", theme);
        v.simulate_resize(size(px(1300.), px(960.)));
        ready(&mut v, &view);
        let before = node(&mut v, &view, "button", "Review draft");
        assert!(before.description.starts_with("Waiting"), "{before:?}");
        assert!(
            glyphs_with_ink(&mut v, before.geometry.unwrap().visual, waiting) >= 9,
            "{theme}: waiting"
        );
        click(&mut v, &view, "button", "Finish earlier");
        let after = node(&mut v, &view, "button", "Review draft");
        assert_eq!(after.id, before.id);
        assert!(after.description.starts_with("Done"), "{after:?}");
        assert!(
            glyphs_with_ink(&mut v, after.geometry.unwrap().visual, done) >= 9,
            "{theme}: done"
        );
    }
}

#[gpui::test]
fn plan_vs_actual_reports_the_run_and_direct_dependencies_in_the_real_gallery(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "plan_vs_actual", "modern-light");
    v.simulate_resize(size(px(1300.), px(960.)));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "+18s behind");
    let build = node(&mut v, &view, "button", "Build the layout");
    assert!(build.description.contains("Elapsed 0:59"));
    assert!(build.description.contains("Waits for: Read the brief"));
    assert!(build.description.contains("Unblocks: Review draft"));
    let b = build.geometry.unwrap().visual;
    v.simulate_mouse_move(center(b), None, Modifiers::none());
    ready(&mut v, &view);
    assert!(
        snapshot(&mut v, &view)
            .nodes()
            .any(|n| n.role == "status" && n.name.starts_with("Build the layout · Running"))
    );
    let chart = bounds(&mut v, &view, "group", "Schedule timeline");
    let wires = filled_including_rounded_clips(&mut v, 0x0000_0080)
        .into_iter()
        .filter(|q| {
            q.bounds.height > 10.0 && q.bounds.width <= 2.0 && f64::from(q.bounds.y) >= chart.y
        })
        .count();
    assert!(wires >= 2, "dependency stems must be painted");
    click(&mut v, &view, "button", "Advance clock");
    node(&mut v, &view, "status", "+1:33 behind");
    assert!(
        node(&mut v, &view, "button", "Build the layout")
            .description
            .contains("Elapsed 2:14")
    );
    click(&mut v, &view, "button", "Finish earlier");
    node(&mut v, &view, "status", "11s early");
    assert!(
        node(&mut v, &view, "button", "Handoff")
            .description
            .contains("Done · Elapsed 0:26")
    );
}

#[gpui::test]
fn plan_vs_actual_pin_rejection_and_narrow_reflow_keep_the_same_native_target(
    cx: &mut TestAppContext,
) {
    for preset in ["modern-dark", "clean-light", "clean-dark"] {
        let (mut v, view) = gallery(cx, "plan_vs_actual", preset);
        v.simulate_resize(size(px(1280.), px(940.)));
        ready(&mut v, &view);
        click(&mut v, &view, "button", "Host-controlled inspection");
        click(&mut v, &view, "button", "Refuse next pin");
        click(&mut v, &view, "button", "Build the layout");
        node(
            &mut v,
            &view,
            "status",
            "The Host kept the previous inspection",
        );
        let step = node(&mut v, &view, "button", "Build the layout");
        assert_eq!(step.pressed, Some(false));
        let focus = v.update(|win, cx| win.focused(cx)).unwrap();
        let wide = bounds(&mut v, &view, "group", "Schedule timeline");
        v.simulate_resize(size(px(720.), px(940.)));
        ready(&mut v, &view);
        assert!(bounds(&mut v, &view, "group", "Schedule timeline").width < wide.width);
        assert_eq!(
            node(&mut v, &view, "button", "Build the layout").id,
            step.id
        );
        assert_eq!(v.update(|win, cx| win.focused(cx)), Some(focus));
        v.simulate_keystrokes("enter");
        ready(&mut v, &view);
        node(&mut v, &view, "status", "Pin accepted: ui/构建");
        assert_eq!(
            node(&mut v, &view, "button", "Build the layout").pressed,
            Some(true)
        );
        v.simulate_keystrokes("escape down");
        ready(&mut v, &view);
        node(&mut v, &view, "status", "Inspection cleared");
        assert!(
            snapshot(&mut v, &view)
                .nodes()
                .any(|n| n.role == "status" && n.name.starts_with("Prepare assets · Done"))
        );
    }
}

#[gpui::test]
fn plan_vs_actual_forecast_moves_in_native_frames_without_rhai_sampling(cx: &mut TestAppContext) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "plan_vs_actual".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .runtime_clock(clock.clock())
    .motion_preference(MotionPreference::Normal)
    .prepare()
    .unwrap();
    let (win, view) = super::mount_prepared(
        cx,
        prepared,
        ("plan-motion", "gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut v = VisualTestContext::from_window(*win, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1290.), px(940.)));
    clock.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    let x = |v: &mut VisualTestContext| {
        let target = bounds(v, &view, "button", "Handoff");
        filled_including_rounded_clips(v, 0x0000_0009)
            .into_iter()
            .filter(|q| {
                (12.0..=16.0).contains(&q.bounds.height)
                    && f64::from(q.bounds.y) > target.y
                    && f64::from(q.bounds.y + q.bounds.height) < target.y + target.height
            })
            .map(|q| q.bounds.x)
            .reduce(f32::max)
            .expect("painted forecast for Handoff")
    };
    let start = x(&mut v);
    click(&mut v, &view, "button", "Advance clock");
    assert!((start - x(&mut v)).abs() <= 1.0);
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    clock.advance(Duration::from_millis(125));
    ready(&mut v, &view);
    let middle = x(&mut v);
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
    clock.advance(Duration::from_millis(800));
    ready(&mut v, &view);
    let end = x(&mut v);
    assert!(
        start + 1.0 < middle && middle + 1.0 < end,
        "{start} → {middle} → {end}"
    );
}

#[gpui::test]
fn plan_vs_actual_header_keeps_metrics_whole_and_uses_the_narrow_gaps(cx: &mut TestAppContext) {
    use gpui_rhai::{AuditRule, AuditRules, AutomationCommand, UiValue};
    for theme in [
        gpui_rhai_registry::MODERN_LIGHT_THEME,
        gpui_rhai_registry::MODERN_DARK_THEME,
        gpui_rhai_registry::CLEAN_LIGHT_THEME,
        gpui_rhai_registry::CLEAN_DARK_THEME,
    ] {
        let (mut v, view) = Mount::new(
        r#"
import "patterns/plan_vs_actual" as plan;
fn state_schema(){#{fields:#{scale:#{schema:#{type:"string"},"default":#{type:"string",value:"100"}}}}}
fn init(ctx){ctx.register_action("probe.scale",Fn("set_scale"));}
fn set_scale(ctx,scale){ctx.set_state("scale",scale);}
fn view(ctx){
    plan::PlanVsActual(#{key:"plan",run:"run",title:"Deploy site",
        task:"Prepare a very long rollout description for all the storefronts before the next review date and report back to the whole team",
        now_ms:20000,steps:[#{id:"build",title:"Build",planned_ms:60000,
            start_ms:0,spent_micros:24000,planned_micros:48000}]}).env(#{scale:ctx.get_state("scale")})
}
"#,
    )
    .registry("patterns/plan_vs_actual")
    .theme(theme)
    .motion(MotionPreference::None)
    .mount(cx);
        let text = |v: &mut VisualTestContext, name: &str| bounds(v, &view, "text", name);
        let heading = |v: &mut VisualTestContext| node(v, &view, "heading", "Deploy site");
        v.simulate_resize(size(px(1050.), px(500.)));
        ready(&mut v, &view);
        let full_header = heading(&mut v);
        assert_eq!(full_header.level, Some(2));
        let full_title = full_header.geometry.unwrap().visual.width;

        v.simulate_resize(size(px(636.), px(500.)));
        ready(&mut v, &view);
        let mounted_header = heading(&mut v);
        assert_eq!(mounted_header.id, full_header.id);
        let title = mounted_header.geometry.unwrap().visual;
        let elapsed = text(&mut v, "Time");
        let cost = text(&mut v, "Cost");
        assert!(
            (title.width - full_title).abs() <= 1.0,
            "the long task gives way before the title"
        );
        let row_overlap =
            (title.y + title.height).min(elapsed.y + elapsed.height) - title.y.max(elapsed.y);
        assert!(
            row_overlap >= title.height.min(elapsed.height) / 2.0
                && (elapsed.y - cost.y).abs() < 1.0,
            "the wide metrics stay on the title line: {title:?}, {elapsed:?}, {cost:?}"
        );
        let wide_gap = cost.x - {
            let budget = text(&mut v, "/ 1:00");
            budget.x + budget.width
        };
        assert!((wide_gap - 14.).abs() <= 0.75, "wide gap: {wide_gap}");
        let findings = v.update(|_, cx| {
            let rules = AuditRules::from_ids([
                AuditRule::SpacingNotNested.id(),
                AuditRule::MixedTypeInRow.id(),
            ])
            .unwrap();
            view.composition_audit_with(&rules, cx).unwrap()
        });
        assert!(
            findings
                .iter()
                .all(|f| !f.path.ends_with("/head") && !f.path.ends_with("/head/totals")),
            "{findings:?}"
        );

        let set_scale = |v: &mut VisualTestContext, scale: &str| {
            v.update(|window, cx| {
                view.automate(
                    AutomationCommand::Action {
                        id: "probe.scale".to_owned(),
                        payload: Some(UiValue::String(scale.to_owned())),
                    },
                    window,
                    cx,
                )
            })
            .unwrap();
            ready(v, &view);
        };
        let step = node(&mut v, &view, "button", "Build").id;
        let before = bounds(&mut v, &view, "group", "Schedule timeline").width;
        set_scale(&mut v, "150");
        let title = heading(&mut v);
        assert_eq!(title.id, full_header.id);
        let title = title.geometry.unwrap().visual;
        let elapsed = text(&mut v, "Time");
        let after = bounds(&mut v, &view, "group", "Schedule timeline").width;
        assert!(
            (before - after).abs() <= 1.,
            "reading scale does not resize this specimen: {before} to {after}"
        );
        assert!(
            elapsed.y >= title.y + title.height,
            "150% reflows at the same width"
        );
        assert_eq!(node(&mut v, &view, "button", "Build").id, step);
        set_scale(&mut v, "100");

        v.simulate_resize(size(px(430.), px(500.)));
        ready(&mut v, &view);
        let title = heading(&mut v).geometry.unwrap().visual;
        let elapsed = text(&mut v, "Time");
        let cost = text(&mut v, "Cost");
        let time_budget = text(&mut v, "/ 1:00");
        let gap = cost.x - (time_budget.x + time_budget.width);
        assert!(
            elapsed.y >= title.y + title.height,
            "narrow totals must use their own row"
        );
        assert!(
            (gap - 16.0).abs() <= 0.75,
            "reference narrow spacing is 16px, got {gap}: budget={time_budget:?}, cost={cost:?}"
        );
    }
}

#[gpui::test]
fn plan_vs_actual_96_steps_fit_the_default_budget_of_an_embedded_view(cx: &mut TestAppContext) {
    let (mut v, view) = Mount::new(
        r#"
        import "patterns/plan_vs_actual" as plan;
        fn view(ctx){
            let steps=[];
            for i in 0..96{
                steps.push(#{id:"work/"+i.to_string(),title:"Work "+i.to_string(),planned_ms:30000,
                    depends_on:if i==0{[]}else{["work/"+(i-1).to_string()]}});
            }
            plan::PlanVsActual(#{key:"ceiling",run:"many",now_ms:0,steps:steps})
        }
    "#,
    )
    .registry("patterns/plan_vs_actual")
    .motion(MotionPreference::None)
    .mount(cx);
    ready(&mut v, &view);
    assert_eq!(
        snapshot(&mut v, &view)
            .nodes()
            .filter(|n| n.role == "button")
            .count(),
        96
    );
    assert!(
        node(&mut v, &view, "button", "Work 95")
            .description
            .contains("Waits for: Work 94")
    );
    node(&mut v, &view, "status", "On plan");
}
