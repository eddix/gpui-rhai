use gpui_rhai::*;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    time::{Duration, Instant},
};
fn tween(p: MotionProperty, from: f64, to: f64, ms: u64) -> MotionSource {
    let mut t = MotionTransition::new(p, from, to, ms);
    t.easing = MotionEasing::Linear;
    MotionSource::Transition(t)
}
fn lifecycle(
    source: &str,
    schema: &ComponentStateSchema,
) -> (
    RuntimeEngine,
    ScriptLifecycle,
    Rc<RefCell<UiRuntimeState>>,
    ManualRuntimeClock,
) {
    let mut e = RuntimeEngine::new();
    let c = e.compile(source).unwrap();
    let r = Rc::new(RefCell::new(UiRuntimeState::new()));
    let clock = ManualRuntimeClock::new(Instant::now());
    r.borrow_mut().clock = clock.clock();
    let mut l = ScriptLifecycle::new(
        c,
        r.clone(),
        ComponentInstancePath::root("View", "v"),
        Some("w".into()),
        BTreeMap::new(),
        schema,
    )
    .unwrap()
    .with_view_id("v");
    l.start(&mut e).unwrap();
    (e, l, r, clock)
}

fn context(r: &Rc<RefCell<UiRuntimeState>>, l: &ScriptLifecycle) -> UiContext {
    UiContext::new(
        r.clone(),
        ComponentInstancePath::root("View", "v"),
        Some("w".into()),
        ExecutionPhase::Event,
        BTreeMap::new(),
    )
    .with_view_id("v")
    .with_generation(l.generation())
}
fn main() {
    // Rebinding must preserve all playback states, and a failed candidate must not leak a binding.
    for playback in ["idle", "paused", "completed"] {
        let schema = ComponentStateSchema::new(BTreeMap::from([(
            "switched".into(),
            StateField::new(ValueSchema::Bool, UiValue::Bool(false)),
        )]))
        .unwrap();
        let source = format!(
            r#"fn view(ctx){{let child=if ctx.get_state("switched"){{box([])}}else{{text("Title")}};column([child.with_key("title")]).with_key("panel").timeline(motion_timeline("intro",motion_track("title",motion_transition("width",100.0,200.0,#{{duration_ms:1000,easing:"linear"}})),#{{autoplay:{}}}))}}"#,
            playback != "idle"
        );
        let (mut e, mut l, r, clock) = lifecycle(&source, &schema);
        let old = l
            .retained()
            .nodes()
            .find(|n| n.key() == Some("title"))
            .unwrap()
            .id();
        let h = context(&r, &l).motion_handle("intro").unwrap();
        let expected = match playback {
            "paused" => {
                clock.advance(Duration::from_millis(250));
                r.borrow_mut()
                    .motions
                    .pause_timeline(&h, clock.clock().now())
                    .unwrap();
                125.
            }
            "completed" => {
                clock.advance(Duration::from_secs(1));
                let _ = r.borrow_mut().motions.tick(clock.clock().now());
                200.
            }
            _ => 100.,
        };
        let state = r.borrow().motions.timeline_state(&h);
        let _ = r.borrow_mut().motions.drain_timeline_events();
        r.borrow_mut()
            .set_component_state_from_host(
                &ComponentInstancePath::root("View", "v"),
                "switched",
                UiValue::Bool(true),
            )
            .unwrap();
        l.render_dirty(&mut e).unwrap();
        let new = l
            .retained()
            .nodes()
            .find(|n| n.key() == Some("title"))
            .unwrap()
            .id();
        assert_ne!(old, new);
        assert_eq!(context(&r, &l).motion_handle("intro").unwrap(), h);
        assert_eq!(r.borrow().motions.timeline_state(&h), state);
        let v = r.borrow().motions.snapshot(clock.clock().now());
        assert!(!v.contains_key(&MotionKey::for_node(
            &format!("window:w/view:v/root/node:{}", old.get()),
            MotionProperty::Width
        )));
        assert_eq!(
            v[&MotionKey::for_node(
                &format!("window:w/view:v/root/node:{}", new.get()),
                MotionProperty::Width
            )],
            expected
        );
        assert!(r.borrow_mut().motions.drain_timeline_events().is_empty());
        let candidate = e
            .compile(&source.replace("box([])", "canvas(canvas_scene([]))"))
            .unwrap();
        l.reload(&mut e, candidate, &schema).unwrap();
        let migrated = context(&r, &l).motion_handle("intro").unwrap();
        assert_ne!(migrated, h);
        assert_eq!(r.borrow().motions.timeline_state(&h), None);
        assert_eq!(r.borrow().motions.timeline_state(&migrated), state);
        let latest = l
            .retained()
            .nodes()
            .find(|n| n.key() == Some("title"))
            .unwrap()
            .id();
        assert_ne!(latest, new);
        let v = r.borrow().motions.snapshot(clock.clock().now());
        assert_eq!(
            v[&MotionKey::for_node(
                &format!("window:w/view:v/root/node:{}", latest.get()),
                MotionProperty::Width
            )],
            expected
        );
        assert_eq!(v.len(), 1);
        println!(
            "PASS target rebind + generation migration preserves {playback}, sample={expected}, old targets removed"
        );
    }
    // Every permutation installs the same final plan. Rejection must preserve handles/events/limits.
    for order in ["abc", "acb", "bac", "bca", "cab", "cba"] {
        let schema = ComponentStateSchema::new(BTreeMap::from([(
            "mode".into(),
            StateField::new(ValueSchema::integer(), UiValue::Integer(0)),
        )]))
        .unwrap();
        let source = format!(
            r#"fn view(ctx){{let m=ctx.get_state("mode");let a=motion_timeline("a",motion_track(".",motion_transition("opacity",0.0,1.0,#{{duration_ms:1000}})),#{{autoplay:m==0||m==2}});let b=motion_timeline("b",motion_track(".",motion_transition("width",100.0,200.0,#{{duration_ms:1000}})),#{{autoplay:m==1||m==2}});let c=motion_timeline("c",motion_track(".",motion_transition("height",30.0,60.0,#{{duration_ms:1000}})),#{{autoplay:m==3||m==2}});text("x").with_key("x"){} }}"#,
            order
                .chars()
                .map(|c| format!(".timeline({c})"))
                .collect::<String>()
        );
        let (mut e, mut l, r, clock) = lifecycle(&source, &schema);
        r.borrow_mut().budgets.active_motions = 1;
        let path = ComponentInstancePath::root("View", "v");
        r.borrow_mut()
            .set_component_state_from_host(&path, "mode", UiValue::Integer(1))
            .unwrap();
        l.render_dirty(&mut e).unwrap();
        assert_eq!(r.borrow().motions.resource_usage().active, 1);
        let handles = r.borrow().motions.inspect_timelines(clock.clock().now());
        let values = r.borrow().motions.snapshot(clock.clock().now());
        let _ = r.borrow_mut().motions.drain_timeline_events();
        r.borrow_mut()
            .set_component_state_from_host(&path, "mode", UiValue::Integer(2))
            .unwrap();
        assert!(l.render_dirty(&mut e).is_err());
        assert_eq!(
            r.borrow().motions.inspect_timelines(clock.clock().now()),
            handles
        );
        assert_eq!(r.borrow().motions.snapshot(clock.clock().now()), values);
        assert!(r.borrow_mut().motions.drain_timeline_events().is_empty());
        assert!(
            r.borrow_mut()
                .motions
                .start(
                    ComponentInstancePath::root("UiNode", "external"),
                    tween(MotionProperty::Opacity, 0., 1., 1000),
                    clock.clock().now()
                )
                .is_err()
        );
        r.borrow_mut()
            .set_component_state_from_host(&path, "mode", UiValue::Integer(3))
            .unwrap();
        l.render_dirty(&mut e).unwrap();
        assert_eq!(r.borrow().motions.resource_usage().active, 1);
        println!(
            "PASS budget permutation={order}, failed candidate restores handles/events/limit, next valid candidate accepted"
        );
    }
    // The protection horizon must preserve real position on either side of the cap.
    let now = Instant::now();
    let mut cases = 0;
    for friction in [0.001, 0.01, 0.1, 0.5, 1., 8., 100.] {
        for velocity in [-100., 100.] {
            for bounded in [false, true] {
                for bounce in [0., 0.75] {
                    let source = MotionSource::Inertia(MotionInertia {
                        property: MotionProperty::TranslateX,
                        from: 0.,
                        velocity,
                        friction,
                        min: bounded.then_some(-50.),
                        max: bounded.then_some(50.),
                        bounce,
                        snap_points: vec![],
                        intent: MotionIntent::Decorative,
                    });
                    let path = ComponentInstancePath::root("UiNode", "root");
                    let mut direct = MotionRuntime::new(MotionPreference::Normal);
                    let key = direct.start(path.clone(), source.clone(), now).unwrap();
                    let mut track = MotionRuntime::new(MotionPreference::Normal);
                    track
                        .start_timeline(
                            path,
                            MotionTimeline::new(
                                "test",
                                MotionTimelineStep::Track(MotionTrack {
                                    target: ".".into(),
                                    source,
                                }),
                            ),
                            now,
                        )
                        .unwrap();
                    let sample = |ms| {
                        direct
                            .sample(&key, now + Duration::from_millis(ms))
                            .unwrap()
                    };
                    assert!((sample(10000) - sample(9999)).abs() <= 0.101);
                    assert!((sample(11000) - sample(10000)).abs() < 1e-8);
                    for ms in [0, 1, 100, 1000, 9999, 10000, 11000] {
                        let x = sample(ms);
                        assert!(x.is_finite());
                        if bounded {
                            assert!((-50.000001..=50.000001).contains(&x));
                        }
                        let y = track.snapshot(now + Duration::from_millis(ms))[&key];
                        assert!((x - y).abs() < 1e-8);
                    }
                    cases += 1;
                }
            }
        }
    }
    println!("PASS inertia continuity/direct-timeline parity over {cases} parameter combinations");
    // Repeated close/reopen must not inherit any prior suspended scope.
    let source = r#"fn view(ctx){text("x").with_key("x").motion(motion_transition("width",0.0,100.0,#{duration_ms:1000,easing:"linear"}))}"#;
    let (mut e, mut l, r, clock) = lifecycle(source, &ComponentStateSchema::default());
    for cycle in 0..3 {
        clock.advance(Duration::from_millis(200));
        let value = r
            .borrow()
            .motions
            .snapshot(clock.clock().now())
            .values()
            .next()
            .copied()
            .unwrap();
        assert!((value - 20.).abs() < 1e-8);
        l.suspend(&mut e).unwrap();
        l.dispose(&mut e).unwrap();
        r.borrow_mut()
            .release_window("w", &ComponentInstancePath::root("View", "v"))
            .unwrap();
        assert_eq!(r.borrow().motions.resource_usage().active, 0);
        assert!(r.borrow().motions.snapshot(clock.clock().now()).is_empty());
        assert!(r.borrow_mut().motions.drain_timeline_events().is_empty());
        let c = e.compile(source).unwrap();
        l = ScriptLifecycle::new(
            c,
            r.clone(),
            ComponentInstancePath::root("View", "v"),
            Some("w".into()),
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap()
        .with_view_id("v");
        l.start(&mut e).unwrap();
        println!("PASS paused teardown/reopen cycle={cycle}");
    }
}
