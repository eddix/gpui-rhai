use gpui_rhai::*;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    time::{Duration, Instant},
};
fn tween(p: MotionProperty, from: f64, to: f64, ms: u64) -> MotionSource {
    let mut s = MotionTransition::new(p, from, to, ms);
    s.easing = MotionEasing::Linear;
    MotionSource::Transition(s)
}
fn timeline(s: MotionSource) -> MotionTimeline {
    MotionTimeline::new(
        "intro",
        MotionTimelineStep::Track(MotionTrack {
            target: ".".into(),
            source: s,
        }),
    )
}
const TL: &str = r#"fn view(ctx){text("x").with_key("x").timeline(motion_timeline("intro",motion_track(".",motion_transition("opacity",0.0,1.0,#{duration_ms:1000,easing:"linear"})),#{}))}"#;
const DIRECT: &str = r#"fn view(ctx){text("x").with_key("x").motion(motion_transition("width",0.0,100.0,#{duration_ms:1000,easing:"linear"}))}"#;
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
fn main() {
    let now = Instant::now();
    let scope = ComponentInstancePath::root("UiNode", "root/card");
    let key = MotionKey::for_node("root/card", MotionProperty::Opacity);
    // Regression acceptance, use positive assertions rather than old defect expectations.
    let mut r = MotionRuntime::new(MotionPreference::Normal);
    let h = r
        .start_timeline(
            scope.clone(),
            timeline(tween(MotionProperty::Opacity, 0., 1., 1000)),
            now,
        )
        .unwrap();
    r.pause_timeline(&h, now + Duration::from_millis(300))
        .unwrap();
    assert!((r.snapshot(now)[&key] - 0.3).abs() < 1e-9);
    r.seek_timeline(&h, 700, now).unwrap();
    assert!((r.snapshot(now)[&key] - 0.7).abs() < 1e-9);
    r.cancel_node_scope("root/card");
    let new = r
        .start_timeline(
            scope.clone(),
            timeline(tween(MotionProperty::Opacity, 0., 1., 1000)),
            now,
        )
        .unwrap();
    assert_ne!(h, new);
    assert!(r.cancel_timeline(&h).is_err());
    println!("PASS pause/seek and stale instance rejection");
    let mut r = MotionRuntime::new(MotionPreference::None);
    let h = r
        .start_timeline(
            scope.clone(),
            timeline(tween(MotionProperty::Opacity, 0., 1., 1000)),
            now,
        )
        .unwrap();
    r.restart_timeline(&h, now).unwrap();
    let frame = r.tick(now + Duration::from_millis(250));
    assert!(!frame.needs_frame);
    assert_eq!(r.snapshot(now)[&key], 1.);
    println!("PASS None restart does not animate");
    // Actual lifecycle reload, not direct calls to the internal migration helper.
    let (mut e, mut l, r, clock) = lifecycle(TL, &ComponentStateSchema::default());
    let old = UiContext::new(
        r.clone(),
        ComponentInstancePath::root("View", "v"),
        Some("w".into()),
        ExecutionPhase::Event,
        BTreeMap::new(),
    )
    .with_view_id("v")
    .with_generation(l.generation())
    .motion_handle("intro")
    .unwrap();
    clock.advance(Duration::from_millis(250));
    let c = e.compile(TL).unwrap();
    println!(
        "reload generations old={:?} candidate={:?}",
        l.generation(),
        c.generation()
    );
    l.reload(&mut e, c, &ComponentStateSchema::default())
        .unwrap();
    println!(
        "reload new_context_lookup={:?} old_handle_still_valid={:?} progress={:?}",
        UiContext::new(
            r.clone(),
            ComponentInstancePath::root("View", "v"),
            Some("w".into()),
            ExecutionPhase::Event,
            BTreeMap::new()
        )
        .with_view_id("v")
        .with_generation(l.generation())
        .motion_handle("intro"),
        r.borrow().motions.timeline_state(&old),
        r.borrow()
            .motions
            .snapshot(clock.clock().now())
            .values()
            .collect::<Vec<_>>()
    );
    // The resume_reload branch must unfreeze its runtime scope.
    let (mut e, mut l, r, clock) = lifecycle(DIRECT, &ComponentStateSchema::default());
    clock.advance(Duration::from_millis(200));
    l.suspend(&mut e).unwrap();
    clock.advance(Duration::from_millis(500));
    let c = e.compile(DIRECT).unwrap();
    l.resume_reload(&mut e, c, &ComponentStateSchema::default())
        .unwrap();
    clock.advance(Duration::from_millis(200));
    let f = r.borrow_mut().motions.tick(clock.clock().now());
    println!(
        "resume_reload state={:?} expected_width=40 actual={:?} needs_frame={}",
        l.state(),
        f.values.values().collect::<Vec<_>>(),
        f.needs_frame
    );
    // Ghost is its own admitted source; unrelated live subtree rerender should not delete it.
    let schema = ComponentStateSchema::new(BTreeMap::from([
        (
            "visible".into(),
            StateField::new(ValueSchema::Bool, UiValue::Bool(true)),
        ),
        (
            "n".into(),
            StateField::new(
                ValueSchema::Integer {
                    min: None,
                    max: None,
                },
                UiValue::Integer(0),
            ),
        ),
    ]))
    .unwrap();
    let source = r#"fn view(ctx){let items=[text(ctx.get_state("n").to_string()).with_key("counter")];if ctx.get_state("visible"){items.push(text("gone").with_key("panel").exit_motion(motion_transition("opacity",1.0,0.0,#{duration_ms:1000,easing:"linear"})));}column(items)}"#;
    let (mut e, mut l, r, clock) = lifecycle(source, &schema);
    let panel = l
        .retained()
        .nodes()
        .find(|n| n.key() == Some("panel"))
        .unwrap()
        .id();
    let b = GeometryBounds::new(0., 0., 100., 20.).unwrap();
    r.borrow_mut().presentation_geometry("v").update(
        panel,
        ElementGeometry {
            layout: b,
            visual: b,
            clip: None,
        },
    );
    let root = ComponentInstancePath::root("View", "v");
    r.borrow_mut()
        .set_component_state_from_host(&root, "visible", UiValue::Bool(false))
        .unwrap();
    l.render_dirty(&mut e).unwrap();
    println!(
        "ghost after_remove active={} samples={:?}",
        r.borrow().motions.resource_usage().active,
        r.borrow().motions.snapshot(clock.clock().now())
    );
    clock.advance(Duration::from_millis(100));
    r.borrow_mut()
        .set_component_state_from_host(&root, "n", UiValue::Integer(1))
        .unwrap();
    l.render_dirty(&mut e).unwrap();
    println!(
        "ghost after_unrelated_update expected_active=1 actual_active={} samples={:?}",
        r.borrow().motions.resource_usage().active,
        r.borrow().motions.snapshot(clock.clock().now())
    );
    // Switching Reduced -> None must reproject already-settled declarations.
    let mut s = MotionTransition::new(MotionProperty::Opacity, 0., 1., 1000);
    s.iterations = Some(2);
    s.autoreverse = true;
    let mut r = MotionRuntime::new(MotionPreference::Reduced);
    r.start(scope.clone(), MotionSource::Transition(s), now)
        .unwrap();
    println!("policy reduced_before={:?}", r.snapshot(now));
    r.set_preference(MotionPreference::None);
    println!("policy None_after={:?}", r.snapshot(now));
    // Timeline-level autoreverse must participate in the static terminal projection.
    let mut spec = timeline(tween(MotionProperty::Opacity, 0., 1., 100));
    spec.autoreverse = true;
    spec.iterations = Some(2);
    let mut normal = MotionRuntime::new(MotionPreference::Normal);
    normal
        .start_timeline(scope.clone(), spec.clone(), now)
        .unwrap();
    let _ = normal.tick(now + Duration::from_millis(200));
    let mut none = MotionRuntime::new(MotionPreference::None);
    none.start_timeline(scope.clone(), spec, now).unwrap();
    println!(
        "timeline_autoreverse normal_terminal={:?} none_terminal={:?}",
        normal.snapshot(now + Duration::from_millis(200)),
        none.snapshot(now)
    );
    // Distinct legal node keys should never alias in motion path encoding.
    let mut r = MotionRuntime::new(MotionPreference::Normal);
    let node = UiNode::column(vec![
        UiNode::text("flat").with_key("a/b").with_motion(tween(
            MotionProperty::Opacity,
            0.,
            1.,
            1000,
        )),
        UiNode::column(vec![
            UiNode::text("nested").with_key("b").with_motion(tween(
                MotionProperty::Opacity,
                0.,
                1.,
                1000,
            )),
        ])
        .with_key("a"),
    ]);
    println!(
        "slash_key distinct_retained_nodes reconcile={:?}",
        motion::reconcile_node_motion(&node, &mut r, now)
    );
    let mut r = MotionRuntime::new(MotionPreference::Normal);
    let first = UiNode::text("one")
        .with_key("one")
        .with_timeline(timeline(tween(MotionProperty::Opacity, 0., 1., 1000)));
    let second = UiNode::text("two")
        .with_key("two")
        .with_timeline(timeline(tween(MotionProperty::Opacity, 0., 1., 1000)));
    let mut tree = RetainedUiTree::new();
    tree.reconcile(first.clone()).unwrap();
    motion::reconcile_node_motion(&first, &mut r, now).unwrap();
    let h = r.inspect_timelines(now)[0].handle.clone();
    let report = tree.reconcile(second.clone()).unwrap();
    motion::reconcile_node_motion(&second, &mut r, now + Duration::from_millis(500)).unwrap();
    println!(
        "root_remount unmounted={:?} old_handle_retained={} sample={:?}",
        report.unmounted,
        r.inspect_timelines(now)[0].handle == h,
        r.snapshot(now + Duration::from_millis(500))
    );

    for count in [100, 1000] {
        for inertia in [false, true] {
            let mut r = MotionRuntime::new(MotionPreference::Normal);
            for i in 0..count {
                let source = if inertia {
                    MotionSource::Inertia(MotionInertia {
                        property: MotionProperty::TranslateX,
                        from: 0.,
                        velocity: 1000.,
                        friction: 0.5,
                        min: None,
                        max: None,
                        bounce: 0.,
                        snap_points: vec![],
                        intent: MotionIntent::Decorative,
                    })
                } else {
                    tween(MotionProperty::TranslateX, 0., 1000., 10000)
                };
                r.start(
                    ComponentInstancePath::root("UiNode", format!("root/{i}")),
                    source,
                    now,
                )
                .unwrap();
            }
            let started = Instant::now();
            for _ in 0..20 {
                std::hint::black_box(r.tick(now + Duration::from_secs(8)));
                std::hint::black_box(r.snapshot(now + Duration::from_secs(8)));
            }
            println!(
                "sampling_debug count={count} inertia={inertia} avg_frame_ms={:.3}",
                started.elapsed().as_secs_f64() * 1000. / 20.
            );
        }
    }

    let schema = ComponentStateSchema::new(BTreeMap::from([(
        "go".into(),
        StateField::new(ValueSchema::Bool, UiValue::Bool(false)),
    )]))
    .unwrap();
    let source = r#"fn view(ctx){let switching=ctx.get_state("go");let node=text("card").with_key("card").timeline(motion_timeline("intro",motion_track(".",motion_transition("opacity",0.0,1.0,#{duration_ms:1000})),#{autoplay:!switching}));if switching{node.motion(motion_transition("width",0.0,100.0,#{duration_ms:1000}))}else{node}}"#;
    let (mut e, mut l, r, _clock) = lifecycle(source, &schema);
    r.borrow_mut().budgets.active_motions = 1;
    r.borrow_mut()
        .set_component_state_from_host(
            &ComponentInstancePath::root("View", "v"),
            "go",
            UiValue::Bool(true),
        )
        .unwrap();
    println!(
        "budget_replace_playing_timeline_with_idle_plus_direct expected_success_active1 actual={:?}",
        l.render_dirty(&mut e)
    );
}
