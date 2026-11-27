//! Shared activity geometry, native time and theme-driven standalone metrics.

use std::time::{Duration, Instant};

use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, MotionSource, ScriptViewHandle, UiNodeKind};

use crate::support::{
    ActiveHost, Mount, Step, bounds, by_test_id, dispatch, last_error, nodes, settle, text_of,
};

#[gpui::test]
fn spinner_design_switch_updates_geometry_and_speed_without_replacing_its_owner(
    cx: &mut TestAppContext,
) {
    let (mut visual,view)=Mount::components(r#"
        import "components/spinner" as spinner;
        fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
        fn change(ctx,p){ctx.set_state("count",ctx.get_state("count")+1);ctx.set_theme("Clean","Dark");}
        fn view(ctx){column([spinner::Spinner(#{key:"activity",label:"Activity"}),
            text(`${ctx.get_state("count")}`).test_id("count"),
            text("Switch").test_id("switch").on_click(Fn("change"))])}
    "#).theme(gpui_rhai_registry::MODERN_LIGHT_THEME).motion(MotionPreference::None)
        .with(|v|v.theme_sources([("clean_dark.rhai".to_owned(),gpui_rhai_registry::CLEAN_DARK_THEME.to_owned())])).mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let before = nodes(&mut visual, &view)
        .into_iter()
        .find(|n| n.role == "progressbar")
        .unwrap();
    for (clean, speed, top) in [(false, 750, 1.0), (true, 800, 1.1)] {
        if clean {
            dispatch(&mut visual, &view, by_test_id("switch"), "click").unwrap();
            settle(&mut visual, Step::Frame, 3);
            assert_eq!(text_of(&mut visual, &view, "count"), "1");
        }
        let current = nodes(&mut visual, &view)
            .into_iter()
            .find(|n| n.role == "progressbar")
            .unwrap();
        assert_eq!(current.id, before.id);
        assert_eq!(current.geometry, before.geometry);
        let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
        let UiNodeKind::Box { children } = root.kind() else {
            panic!("column")
        };
        let MotionSource::Transition(spec) = &children[0].motions()[0] else {
            panic!("rotation")
        };
        assert_eq!(spec.duration_ms, speed);
        let UiNodeKind::Canvas { scene } = children[0].kind() else {
            panic!("canvas")
        };
        let gpui_rhai::CanvasCommand::Path { segments, .. } = &scene.commands()[0] else {
            panic!("ring")
        };
        let gpui_rhai::CanvasPathSegment::Move { y, .. } = segments[0] else {
            panic!("top")
        };
        assert!((y - top).abs() < 0.0001);
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn spinner_diameter_inherits_size_and_scale_without_changing_action_metrics(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = Mount::components(r#"
            import "components/spinner" as spinner;
            fn view(ctx){column([
                spinner::Spinner(#{key:"default",label:"Default"}),
                column([spinner::Spinner(#{key:"inherited",label:"Inherited"}),
                    spinner::Spinner(#{key:"explicit",label:"Explicit",size:"xs"})]).env(#{size:"lg",scale:"150"}),
                spinner::Spinner(#{key:"host",label:"Host",part_styles:#{indicator:style().width(px(30)).height(px(30))}})
            ]).with_style(style().gap(px(12)).items_start())}
        "#).theme(theme).motion(MotionPreference::None).mount(cx);
        settle(&mut visual, Step::Frame, 3);
        for (label, expected) in [
            ("Default", 14.0),
            ("Inherited", 27.0),
            ("Explicit", 18.0),
            ("Host", 30.0),
        ] {
            let actual = bounds(&mut visual, &view, "progressbar", label);
            assert!(
                (actual.width - expected).abs() < 0.5 && (actual.height - expected).abs() < 0.5,
                "{name}/{label}: {actual:?}, expected {expected}"
            );
        }
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

fn query(visual: &mut VisualTestContext, view: &ScriptViewHandle, source: (f64, f64)) -> String {
    let area = bounds(visual, view, "progressbar", "Activity");
    #[allow(clippy::cast_possible_truncation)]
    visual.simulate_click(
        point(
            px((area.x + source.0 * 8.0) as f32),
            px((area.y + source.1 * 8.0) as f32),
        ),
        Modifiers::none(),
    );
    settle(visual, Step::Frame, 3);
    text_of(visual, view, "hit")
}

#[gpui::test]
fn spinner_native_rotation_retains_phase_across_commits_and_pauses_active_time(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let manual = ManualRuntimeClock::new(Instant::now());
        let clock = manual.clock();
        let (window,view)=Mount::components(r#"
            import "components/spinner" as spinner;
            fn state_schema(){#{fields:#{hit:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}}}}
            fn hit(ctx,event){ctx.set_state("hit",if event.canvas_key==(){"none"}else{event.canvas_key});}
            fn view(ctx){column([
                spinner::Spinner(#{key:"activity",label:"Activity",part_styles:#{indicator:style().width(px(128)).height(px(128))}})
                    .on("pointer_down",Fn("hit")),
                text(ctx.get_state("hit")).test_id("hit")
            ]).with_style(style().items_start())}
        "#).theme(theme).motion(MotionPreference::Normal).with(move|v|v.runtime_clock(clock))
            .mount_root(cx,|host,view|ActiveHost{host,view});
        let mut visual = VisualTestContext::from_window(*window, cx);
        settle(&mut visual, Step::Frame, 3);
        let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
        let UiNodeKind::Box { children } = root.kind() else {
            panic!("column")
        };
        let MotionSource::Transition(spec) = &children[0].motions()[0] else {
            panic!("rotation")
        };
        let duration = if name.starts_with("clean_") { 800 } else { 750 };
        assert_eq!(spec.duration_ms, duration, "{name}");
        assert_eq!(query(&mut visual, &view, (12.2, 3.8)), "indicator");
        let revision = visual.update(|_, cx| view.committed_revision(cx).unwrap());
        visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
        manual.advance(Duration::from_micros(duration * 250));
        settle(&mut visual, Step::Refresh, 3);
        assert_eq!(
            visual.update(|_, cx| view.committed_revision(cx).unwrap()),
            revision
        );
        assert!(
            visual
                .update(|_, cx| view.take_performance_snapshot(cx).unwrap())
                .timings
                .is_empty()
        );
        assert_eq!(query(&mut visual, &view, (12.2, 3.8)), "track");
        assert_eq!(query(&mut visual, &view, (12.2, 12.2)), "indicator");
        assert!(
            visual
                .update(|window, cx| view.suspend(window, cx))
                .unwrap()
        );
        settle(&mut visual, Step::Frame, 3);
        manual.advance(Duration::from_secs(60));
        assert!(visual.update(|_, cx| view.resume(cx)).unwrap());
        settle(&mut visual, Step::Refresh, 3);
        assert_eq!(query(&mut visual, &view, (12.2, 12.2)), "indicator");
        manual.advance(Duration::from_micros(duration * 250));
        settle(&mut visual, Step::Refresh, 3);
        assert_eq!(query(&mut visual, &view, (12.2, 12.2)), "track");
        assert_eq!(query(&mut visual, &view, (3.8, 12.2)), "indicator");
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn spinner_reduced_and_none_are_visible_but_do_not_keep_scheduling_frames(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for policy in [MotionPreference::Reduced, MotionPreference::None] {
            let (mut visual, view) = Mount::components(
                r#"import "components/spinner" as spinner;
                fn view(ctx){spinner::Spinner(#{key:"busy",label:"Working"})}"#,
            )
            .theme(theme)
            .motion(policy)
            .mount(cx);
            settle(&mut visual, Step::Frame, 4);
            let area = bounds(&mut visual, &view, "progressbar", "Working");
            assert!(area.width > 0.0 && area.height > 0.0);
            let mut frames = Vec::new();
            for _ in 0..3 {
                frames.push(visual.update(|window, cx| window.simulate_next_frame(cx)));
                visual.run_until_parked();
            }
            assert_eq!(frames.last(), Some(&0), "{name}/{policy:?}: {frames:?}");
            assert_eq!(last_error(&mut visual, &view), None);
        }
    }
}
