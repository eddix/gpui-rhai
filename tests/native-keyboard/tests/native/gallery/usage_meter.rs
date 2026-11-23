use std::time::{Duration, Instant};

use gpui::{Modifiers, TestAppContext, point, px, size};
use gpui_rhai::{AutomationCommand, ManualRuntimeClock, MotionPreference, UiValue};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Mount, bounds, glyphs_with_ink, live_fills, snapshot};

fn advance(visual: &mut gpui::VisualTestContext, view: &gpui_rhai::ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(350));
    ready(visual, view);
}

#[gpui::test]
fn usage_meter_hint_follows_the_native_strip_with_no_focus_or_script_side_effects(
    cx: &mut TestAppContext,
) {
    let script = r#"
import "patterns/usage_meter" as meter;
fn state_schema(){#{fields:#{changed:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
fn init(ctx){ctx.register_action("probe.retarget",Fn("retarget"));}
fn retarget(ctx,event){ctx.set_state("changed",true);}
fn noop(ctx,event){}
fn view(ctx){
    let changed=ctx.get_state("changed");
    let values=if changed{[50,30,20]}else{[8,84,8]};
    column([
        text("Focus guard").with_key("guard").accessibility_role("button").tab_stop(true).on_click(Fn("noop")),
        meter::UsageMeter(#{key:"sample",series:"sample",revision:if changed{1}else{0},total:100,unit:"credits",
            used:[#{id:"first",label:"First",value:values[0],color:0x4971c9},
                  #{id:"mid",label:"Middle",value:values[1],color:0x43a575},
                  #{id:"last",label:"Trailing category with an intentionally long title",value:values[2],color:0xc37049}]})
    ]).with_style(style().width(relative(1.0)))
}
"#;
    for rtl in [false, true] {
        let manual = ManualRuntimeClock::new(Instant::now());
        let clock = manual.clock();
        let (mut v, view) = Mount::new(script)
            .registry("patterns/usage_meter")
            .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
            .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
            .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
            .motion(MotionPreference::Normal)
            .with(move |view| view.runtime_clock(clock))
            .mount(cx);
        v.update(|window, cx| {
            window.activate_window();
            view.select_locale(if rtl { "ar" } else { "en" }, cx)
                .unwrap();
        });
        v.simulate_resize(size(px(350.), px(700.)));
        ready(&mut v, &view);
        manual.advance(Duration::from_millis(750));
        ready(&mut v, &view);
        let track = |v: &mut gpui::VisualTestContext| {
            snapshot(v, &view)
                .nodes()
                .find(|n| n.role == "progressbar")
                .unwrap()
                .clone()
        };
        let lane = |v: &mut gpui::VisualTestContext, paint| {
            let bar = track(v).geometry.unwrap().visual;
            live_fills(v, paint)
                .into_iter()
                .find(|q| {
                    (f64::from(q.y) - bar.y).abs() <= 0.5
                        && (f64::from(q.height) - bar.height).abs() <= 0.5
                })
                .expect("bar lane is painted")
        };
        let hint = |v: &mut gpui::VisualTestContext, name: &str, paint| {
            let bar = track(v).geometry.unwrap().visual;
            let slice = lane(v, paint);
            let tree = snapshot(v, &view);
            let tip = tree
                .nodes()
                .find(|n| n.role == "status" && n.name.contains(name))
                .unwrap_or_else(|| {
                    let statuses: Vec<_> = tree
                        .nodes()
                        .filter(|n| n.role == "status")
                        .map(|n| &n.name)
                        .collect();
                    panic!("rtl={rtl}: no hint for {name:?}; statuses={statuses:?}")
                })
                .geometry
                .unwrap()
                .visual;
            let fraction = (f64::from(slice.x + slice.width / 2.) - bar.x) / bar.width;
            let expected = bar.x + fraction * (bar.width - tip.width);
            assert!(
                tip.width > 20.
                    && tip.x >= bar.x - 1.
                    && tip.x + tip.width <= bar.x + bar.width + 1.,
                "rtl={rtl}: {tip:?} inside {bar:?}"
            );
            assert!(
                (tip.x - expected).abs() <= 1.,
                "rtl={rtl}: {tip:?}, {bar:?}, {slice:?}: {expected}"
            );
            assert!(tip.y + tip.height < bar.y, "{tip:?} above {bar:?}");
            tip
        };
        let focus = |v: &mut gpui::VisualTestContext| {
            v.update(|window, cx| {
                view.debug_frame_digest(window, cx)
                    .unwrap()
                    .focus_path
                    .first()
                    .copied()
            })
        };
        click(&mut v, &view, "button", "Focus guard");
        let guard = focus(&mut v).expect("guard has focus");
        let trailing = lane(&mut v, 0xc37049ff);
        let start = lane(&mut v, 0x4971c9ff);
        assert_eq!(trailing.x < start.x, rtl, "{start:?}, {trailing:?}");
        v.simulate_mouse_move(
            point(
                px(trailing.x + trailing.width / 2.),
                px(trailing.y + trailing.height / 2.),
            ),
            None,
            Modifiers::none(),
        );
        ready(&mut v, &view);
        let end = hint(&mut v, "Trailing category", 0xc37049ff);
        assert!(
            end.width > 130.,
            "hint is still readable at the end: {end:?}"
        );
        assert_eq!(focus(&mut v), Some(guard), "hover must not steal focus");

        v.simulate_click(
            point(
                px(trailing.x + trailing.width / 2.),
                px(trailing.y + trailing.height / 2.),
            ),
            Modifiers::none(),
        );
        v.simulate_mouse_move(point(px(4.), px(650.)), None, Modifiers::none());
        ready(&mut v, &view);
        let track_id = track(&mut v).id;
        assert_eq!(focus(&mut v), Some(track_id));
        hint(&mut v, "Trailing category", 0xc37049ff);
        v.simulate_keystrokes(if rtl { "right" } else { "left" });
        ready(&mut v, &view);
        let before = hint(&mut v, "Middle", 0x43a575ff);
        v.update(|window, cx| {
            view.automate(
                AutomationCommand::Action {
                    id: "probe.retarget".into(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
        ready(&mut v, &view);
        let accepted = v
            .update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap();
        assert!(
            !accepted.timings.is_empty(),
            "positive script timing control"
        );
        manual.advance(Duration::from_millis(250));
        ready(&mut v, &view);
        let after = hint(&mut v, "Middle", 0x43a575ff);
        assert!((after.x - before.x).abs() > 8., "{before:?} -> {after:?}");
        assert_eq!(focus(&mut v), Some(track_id));
        assert!(
            v.update(|_, cx| view.take_performance_snapshot(cx))
                .unwrap()
                .timings
                .is_empty()
        );
        v.simulate_resize(size(px(220.), px(700.)));
        ready(&mut v, &view);
        hint(&mut v, "Middle", 0x43a575ff);
        v.simulate_keystrokes("escape");
        ready(&mut v, &view);
        assert!(
            !snapshot(&mut v, &view)
                .nodes()
                .any(|n| n.role == "status" && n.name.contains("Middle"))
        );
        assert_eq!(focus(&mut v), Some(track_id));
    }
}

#[gpui::test]
fn usage_meter_native_quota_number_inherits_its_status_ink(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("usage_quota_ink"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, warning, ordinary, limit) in [
        ("modern-light", 0xb25e00ff, 0x404040ff, 0x171717ff),
        ("modern-dark", 0xf2a15cff, 0xc8c8ccff, 0xedededff),
        ("clean-light", 0x8f6c00ff, 0x0000009c, 0xf04438ff),
        ("clean-dark", 0xe6c24dff, 0xffffff9e, 0xff8a7aff),
    ] {
        let (mut v, view) = gallery(&mut real, "usage_meter", theme);
        v.simulate_resize(size(px(1240.), px(1100.)));
        ready(&mut v, &view);
        let pct = bounds(&mut v, &view, "text", "87%");
        assert!(
            glyphs_with_ink(&mut v, pct, warning) >= 3,
            "{theme}: {pct:?}"
        );
        for (period, amount, ink) in [("Week", "50%", ordinary), ("Quarter", "134%", limit)] {
            click(&mut v, &view, "radio", period);
            advance(&mut v, &view);
            let pct = bounds(&mut v, &view, "text", amount);
            assert!(
                glyphs_with_ink(&mut v, pct, ink) >= 3,
                "{theme}/{period}: {pct:?}"
            );
        }
    }
}

#[gpui::test]
fn usage_meter_footer_owns_the_action_surface_in_each_state(cx: &mut TestAppContext) {
    #[cfg(feature = "gpui-fast")]
    use apple_fast as apple;
    #[cfg(feature = "gpui-pre")]
    use apple_pre as apple;
    let mut real = TestAppContext::build_with_text_system(
        cx.dispatcher.clone(),
        Some("usage_footer_surface"),
        std::sync::Arc::new(apple::AppleTextSystem::new()),
    );
    for (theme, height, normal_bg, normal_ink, strong_bg, strong_ink) in [
        (
            "modern-light",
            32.,
            0xffffffff,
            0x171717ff,
            0x171717ff,
            0xffffffff,
        ),
        (
            "modern-dark",
            32.,
            0x19191bff,
            0xedededff,
            0xedededff,
            0x19191bff,
        ),
        (
            "clean-light",
            28.,
            0xffffffff,
            0x000000ff,
            0x000000e0,
            0xfcfcfcff,
        ),
        (
            "clean-dark",
            28.,
            0x1c1c1eff,
            0xffffffff,
            0xffff_ffe6,
            0x111113ff,
        ),
    ] {
        let (mut v, view) = gallery(&mut real, "usage_meter", theme);
        v.simulate_resize(size(px(1240.), px(1100.)));
        ready(&mut v, &view);
        let target = node(&mut v, &view, "button", "Add credits");
        for (title, paper, ink) in [
            ("Add credits", strong_bg, strong_ink),
            ("Upgrade", normal_bg, normal_ink),
        ] {
            if title == "Upgrade" {
                click(&mut v, &view, "radio", "Week");
                advance(&mut v, &view);
            }
            let action = node(&mut v, &view, "button", title);
            assert_eq!(
                action.id, target.id,
                "{theme}: same action through Host changes"
            );
            let rect = action.geometry.unwrap().visual;
            assert!(
                (rect.height - height).abs() <= 1.,
                "{theme}/{title}: {rect:?}"
            );
            assert!(
                glyphs_with_ink(&mut v, rect, ink) >= 5,
                "{theme}/{title}: incorrect ink"
            );
            assert!(
                crate::support::live_fills(&mut v, paper)
                    .iter()
                    .any(|paint| (f64::from(paint.x) - rect.x).abs() <= 1.
                        && (f64::from(paint.y) - rect.y).abs() <= 1.
                        && (f64::from(paint.width) - rect.width).abs() <= 1.
                        && (f64::from(paint.height) - rect.height).abs() <= 1.),
                "{theme}/{title}: missing action surface {paper:08x}"
            );
        }
    }
}

#[gpui::test]
fn usage_meter_zero_capacity_is_not_unlimited_and_never_displays_infinity(cx: &mut TestAppContext) {
    let patterns = gpui_rhai_registry::BUNDLED_PATTERN_SOURCES_BY_ID
        .iter()
        .copied()
        .filter(|(id, _)| {
            matches!(
                *id,
                "patterns/usage_meter"
                    | "patterns/compact_breakdown"
                    | "patterns/agent_tree"
                    | "patterns/approval_card"
            )
        })
        .collect::<Vec<_>>();
    let (mut visual, view) = Mount::components(
        r#"
        import "patterns/usage_meter" as meter;
        fn view(ctx){
            column([
                meter::UsageMeter(#{key:"no-usage",series:"inactive",revision:0,total:0,used:[]}),
                meter::UsageMeter(#{key:"over",series:"no-plan",revision:0,total:0,
                    used:[#{id:"api",label:"API",value:3}],
                    limits:[#{id:"workspaces",label:"Workspaces",used:1,total:0},
                        #{id:"projects",label:"Projects",used:1}]}),
            ])
        }
    "#,
    )
    .modules(&patterns)
    .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
    .motion(MotionPreference::None)
    .mount(cx);
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Within limit");
    node(&mut visual, &view, "status", "Limit reached");
    node(&mut visual, &view, "text", "Over quota");
    let capped = node(&mut visual, &view, "progressbar", "Workspaces");
    assert_eq!(capped.value_max, Some(0.0));
    assert_eq!(capped.description, "Limit reached");
    assert!(
        node(&mut visual, &view, "listitem", "Projects")
            .description
            .contains("Unlimited")
    );
}

#[gpui::test]
fn usage_meter_uses_host_accepted_quota_and_versioned_actions(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "usage_meter", "modern-light");
    visual.simulate_resize(size(px(1240.), px(1100.)));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Running low");
    let metric = node(&mut visual, &view, "progressbar", "Credits used");
    assert_eq!(metric.value, Some(UiValue::Float(868.)));
    assert_eq!(metric.value_max, Some(1000.));
    click(&mut visual, &view, "button", "Refuse next snapshot");
    click(&mut visual, &view, "radio", "Week");
    node(&mut visual, &view, "status", "Waiting for usage snapshot…");
    assert_eq!(
        node(&mut visual, &view, "progressbar", "Credits used").value,
        metric.value
    );
    assert_eq!(
        node(&mut visual, &view, "radio", "Month").checked,
        Some(UiValue::Bool(true))
    );
    advance(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The Host kept the current usage snapshot.",
    );
    visual.simulate_keystrokes("space");
    ready(&mut visual, &view);
    advance(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted usage: week");
    node(&mut visual, &view, "status", "Within limit");
    assert_eq!(
        node(&mut visual, &view, "progressbar", "Credits used").value,
        Some(UiValue::Float(500.))
    );
    click(&mut visual, &view, "button", "Upgrade");
    node(&mut visual, &view, "status", "Upgrade requested");
    click(&mut visual, &view, "radio", "Quarter");
    advance(&mut visual, &view);
    node(&mut visual, &view, "status", "Limit reached");
    let over = node(&mut visual, &view, "progressbar", "Credits used");
    assert_eq!(over.value, Some(UiValue::Float(1000.)));
    assert!(over.description.contains("1,340 of 1,000"));
    click(&mut visual, &view, "button", "Add credits");
    node(&mut visual, &view, "status", "Top-up requested");
    assert!(
        node(&mut visual, &view, "listitem", "Projects")
            .description
            .contains("Unlimited")
    );
}

#[gpui::test]
fn usage_meter_bar_hit_is_stable_and_daily_distinguishes_zero_from_future(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "usage_meter", "clean-dark");
    visual.simulate_resize(size(px(1240.), px(1100.)));
    ready(&mut visual, &view);
    let target = |visual: &mut gpui::VisualTestContext| {
        let fills = live_fills(visual, 0x4fb0_feff);
        fills
            .iter()
            .copied()
            .find(|r| r.width > 140. && (r.height - 8.).abs() < 0.5)
            .unwrap_or_else(|| panic!("the main usage segment must be painted: {fills:?}"))
    };
    let bar = target(&mut visual);
    let track = bounds(&mut visual, &view, "progressbar", "Credits used");
    assert!((f64::from(bar.width) / track.width - 0.46).abs() < 0.015);
    assert!(
        live_fills(&mut visual, 0x1c1c1eff)
            .into_iter()
            .any(|q| (q.width - 2.).abs() < 0.5
                && (q.y - bar.y).abs() < 0.5
                && (q.height - bar.height).abs() < 0.5
                && ((f64::from(q.right()) - track.x) / track.width - 0.46).abs() < 0.015),
        "the separator uses card paper without changing the quota fraction"
    );
    visual.simulate_mouse_move(
        point(px(bar.x + bar.width / 2.), px(bar.y + 4.)),
        None,
        Modifiers::none(),
    );
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Compute, 460 credits, 53%");
    assert_eq!(
        bounds(&mut visual, &view, "progressbar", "Credits used"),
        track
    );
    let spark = bounds(&mut visual, &view, "list", "Per day");
    crate::support::click_center(&mut visual, spark);
    ready(&mut visual, &view);
    visual.simulate_keystrokes("home right right");
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Day 3 · 0 credits");
    visual.simulate_keystrokes("end");
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Day 30 · Not yet recorded");
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("status", "Day 30 · Not yet recorded")
            .next()
            .is_none()
    );
    let before = bounds(&mut visual, &view, "progressbar", "Storage");
    click(&mut visual, &view, "button", "Narrow usage card");
    assert!(bounds(&mut visual, &view, "progressbar", "Credits used").width < track.width);
    assert!(bounds(&mut visual, &view, "progressbar", "Storage").width > before.width);
    node(&mut visual, &view, "listitem", "Compute, 460 credits, 53%");
}
