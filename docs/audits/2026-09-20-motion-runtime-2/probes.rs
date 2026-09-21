use gpui_rhai::*;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    time::{Duration, Instant},
};
fn tween(property: MotionProperty, from: f64, to: f64, ms: u64) -> MotionSource {
    let mut t = MotionTransition::new(property, from, to, ms);
    t.easing = MotionEasing::Linear;
    MotionSource::Transition(t)
}
fn tl(name: &str, source: MotionSource) -> MotionTimeline {
    MotionTimeline::new(
        name,
        MotionTimelineStep::Track(MotionTrack {
            target: ".".into(),
            source,
        }),
    )
}
fn main() {
    let now = Instant::now();
    let scope = ComponentInstancePath::root("UiNode", "root/card");
    let key = MotionKey::for_node("root/card", MotionProperty::Opacity);
    let mut r = MotionRuntime::new(MotionPreference::Normal);
    let h = r
        .start_timeline(
            scope.clone(),
            tl("intro", tween(MotionProperty::Opacity, 0.0, 1.0, 1000)),
            now,
        )
        .unwrap();
    let _ = r.tick(now + Duration::from_millis(100));
    r.pause_timeline(&h, now + Duration::from_millis(300))
        .unwrap();
    let paused = r
        .snapshot(now + Duration::from_millis(300))
        .get(&key)
        .copied();
    r.seek_timeline(&h, 700, now + Duration::from_millis(300))
        .unwrap();
    println!(
        "paused_seek expected_pause=.3 expected_seek=.7 actual_pause={paused:?} actual_seek={:?}",
        r.snapshot(now + Duration::from_millis(300)).get(&key)
    );
    r.cancel_node_scope("root/card");
    let newer = r
        .start_timeline(
            scope.clone(),
            tl("intro", tween(MotionProperty::Opacity, 0.0, 1.0, 1000)),
            now,
        )
        .unwrap();
    r.cancel_timeline(&h).unwrap();
    println!(
        "old_handle_reused={} new_state={:?}",
        h == newer,
        r.timeline_state(&newer)
    );

    let mut reduced = MotionRuntime::new(MotionPreference::None);
    let h = reduced
        .start_timeline(
            scope.clone(),
            tl("intro", tween(MotionProperty::Opacity, 0.0, 1.0, 1000)),
            now,
        )
        .unwrap();
    reduced.restart_timeline(&h, now).unwrap();
    let frame = reduced.tick(now + Duration::from_millis(250));
    println!(
        "none_policy_restart state={:?} sample={:?} needs_frame={}",
        reduced.timeline_state(&h),
        frame.values.get(&key),
        frame.needs_frame
    );

    let mut reversed = MotionTransition::new(MotionProperty::Opacity, 0.0, 1.0, 100);
    reversed.easing = MotionEasing::Linear;
    reversed.iterations = Some(2);
    reversed.autoreverse = true;
    let mut r = MotionRuntime::new(MotionPreference::Normal);
    let key = r
        .start(scope.clone(), MotionSource::Transition(reversed), now)
        .unwrap();
    let f = r.tick(now + Duration::from_millis(200));
    println!(
        "autoreverse final_tick={:?} final_snapshot={:?}",
        f.values.get(&key),
        r.snapshot(now + Duration::from_millis(200)).get(&key)
    );

    let spring = MotionSpring::new(MotionProperty::TranslateX, 0.0, 1.0);
    let mut direct = MotionRuntime::new(MotionPreference::Normal);
    let dk = direct
        .start(scope.clone(), MotionSource::Spring(spring.clone()), now)
        .unwrap();
    let mut timeline = MotionRuntime::new(MotionPreference::Normal);
    timeline
        .start_timeline(
            scope.clone(),
            tl("spring", MotionSource::Spring(spring.clone())),
            now,
        )
        .unwrap();
    let direct_value = direct.tick(now + Duration::from_secs(1)).values[&dk];
    let timeline_value = timeline.tick(now + Duration::from_secs(1)).values[&dk];
    println!("same_spring_at_1s direct={direct_value} timeline={timeline_value}");
    let mut r = MotionRuntime::new(MotionPreference::Normal);
    r.start(scope.clone(), MotionSource::Spring(spring.clone()), now)
        .unwrap();
    let _ = r.tick(now + Duration::from_millis(16));
    let velocity = r.inspect(now)[0].velocity;
    let mut retarget = spring.clone();
    retarget.to = 2.0;
    r.start(
        scope.clone(),
        MotionSource::Spring(retarget),
        now + Duration::from_millis(16),
    )
    .unwrap();
    println!(
        "retarget_velocity before={velocity:?} after={:?}",
        r.inspect(now)[0].velocity
    );
    let mut r = MotionRuntime::new(MotionPreference::Normal);
    let mut unstable = spring;
    unstable.stiffness = 1e308;
    unstable.mass = 1e-308;
    let result = r.start(scope.clone(), MotionSource::Spring(unstable), now);
    println!("extreme_physics_accepted={}", result.is_ok());
    if let Ok(key) = result {
        let _ = r.tick(now + Duration::from_millis(16));
        let _ = r.tick(now + Duration::from_millis(32));
        println!(
            "extreme_physics_value={:?}",
            r.sample(&key, now + Duration::from_millis(32))
        );
    }

    let mut r = MotionRuntime::new(MotionPreference::Normal);
    let node = UiNode::text("conflict")
        .with_key("conflict")
        .with_motion(tween(MotionProperty::Opacity, 0.0, 1.0, 1000))
        .with_timeline(tl("second", tween(MotionProperty::Opacity, 1.0, 0.0, 1000)));
    println!(
        "two_property_owners_accepted={}",
        motion::reconcile_node_motion(&node, &mut r, now).is_ok()
    );
    let missing = MotionTimeline::new(
        "missing",
        MotionTimelineStep::Track(MotionTrack {
            target: "nonexistent".into(),
            source: tween(MotionProperty::Rotate, 0.0, 90.0, 1000),
        }),
    );
    let node = UiNode::text("no child")
        .with_key("root")
        .with_timeline(missing);
    println!(
        "missing_unsupported_target_accepted={}",
        motion::reconcile_node_motion(
            &node,
            &mut MotionRuntime::new(MotionPreference::Normal),
            now
        )
        .is_ok()
    );

    // A mounted view commits geometry to its per-view domain, not the legacy global map.
    let clock = ManualRuntimeClock::new(now);
    let state = Rc::new(RefCell::new(UiRuntimeState::new()));
    state.borrow_mut().clock = clock.clock();
    let schema = ComponentStateSchema::new(BTreeMap::from([(
        "visible".into(),
        StateField::new(ValueSchema::Bool, UiValue::Bool(true)),
    )]))
    .unwrap();
    let mut engine = RuntimeEngine::new();
    let compiled=engine.compile(r#"fn view(ctx){ if ctx.get_state("visible") { column([text("gone").with_key("panel").exit_motion(motion_transition("opacity",1.0,0.0,#{duration_ms:500}))]) } else {column([])} }"#).unwrap();
    let root = ComponentInstancePath::root("View", "v");
    let mut life = ScriptLifecycle::new(
        compiled,
        state.clone(),
        root.clone(),
        Some("w".into()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap()
    .with_view_id("v");
    life.start(&mut engine).unwrap();
    let id = life
        .retained()
        .nodes()
        .find(|n| n.key() == Some("panel"))
        .unwrap()
        .id();
    let bounds = GeometryBounds::new(0.0, 0.0, 100.0, 20.0).unwrap();
    state.borrow_mut().presentation_geometry("v").update(
        id,
        ElementGeometry {
            layout: bounds,
            visual: bounds,
            clip: None,
        },
    );
    state
        .borrow_mut()
        .set_component_state_from_host(&root, "visible", UiValue::Bool(false))
        .unwrap();
    life.render_dirty(&mut engine).unwrap();
    println!(
        "exit_with_committed_view_geometry active_motion_sources={}",
        state.borrow().motions.resource_usage().active
    );

    // Mirrors a second active window ticking the shared runtime during another view's suspension.
    let mut engine = RuntimeEngine::new();
    let state = Rc::new(RefCell::new(UiRuntimeState::new()));
    let clock = ManualRuntimeClock::new(now);
    state.borrow_mut().clock = clock.clock();
    let compiled=engine.compile(r#"fn view(ctx){text("x").with_key("x").motion(motion_transition("width",0.0,100.0,#{duration_ms:100,easing:"linear"}))}"#).unwrap();
    let mut life = ScriptLifecycle::new(
        compiled,
        state.clone(),
        ComponentInstancePath::root("View", "v"),
        Some("w".into()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap()
    .with_view_id("v");
    life.start(&mut engine).unwrap();
    clock.advance(Duration::from_millis(20));
    let _ = state.borrow_mut().motions.tick(clock.clock().now());
    life.suspend(&mut engine).unwrap();
    let before = state.borrow().motion_values.values().next().copied();
    clock.advance(Duration::from_millis(200));
    let _ = state.borrow_mut().motions.tick(clock.clock().now());
    life.resume(&mut engine).unwrap();
    println!(
        "suspended_shared_tick before={before:?} after={:?}",
        state.borrow().motion_values.values().next()
    );
    let state = Rc::new(RefCell::new(UiRuntimeState::new()));
    state.borrow_mut().budgets.active_motions = 1;
    let mut engine = RuntimeEngine::new();
    let compiled=engine.compile(r#"fn view(ctx){text("x").with_key("x").timeline(motion_timeline("intro",motion_parallel([
      motion_track(".",motion_transition("opacity",0.0,1.0,#{duration_ms:1000})),
      motion_track(".",motion_transition("translate_x",0.0,100.0,#{duration_ms:1000}))
    ]),#{autoplay:false}))}"#).unwrap();
    let path = ComponentInstancePath::root("View", "budget");
    let mut life = ScriptLifecycle::new(
        compiled,
        state.clone(),
        path.clone(),
        None,
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    life.start(&mut engine).unwrap();
    let ctx = UiContext::new(
        state.clone(),
        path,
        None,
        ExecutionPhase::Event,
        BTreeMap::new(),
    );
    let handle = ctx.motion_handle("intro").unwrap();
    let play = ctx.play_motion(&handle);
    println!(
        "play_bypasses_active_budget limit=1 accepted={} active={}",
        play.is_ok(),
        state.borrow().motions.resource_usage().active
    );

    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let foreign = runtime
        .borrow_mut()
        .motions
        .start_timeline(
            ComponentInstancePath::root("UiNode", "window:a/view:a/root"),
            tl("foreign", tween(MotionProperty::Opacity, 0.0, 1.0, 1000)),
            now,
        )
        .unwrap();
    let other = UiContext::new(
        runtime.clone(),
        ComponentInstancePath::root("View", "b"),
        Some("b".into()),
        ExecutionPhase::Event,
        BTreeMap::new(),
    )
    .with_view_id("b");
    println!(
        "foreign_view_handle_cancel_accepted={}",
        other.cancel_motion(&foreign).is_ok()
    );

    let mut runtime = MotionRuntime::new(MotionPreference::Normal);
    let overlapping = MotionTimeline::new(
        "invalid",
        MotionTimelineStep::Parallel(vec![
            MotionTimelineStep::Track(MotionTrack {
                target: ".".into(),
                source: tween(MotionProperty::Opacity, 0.0, 1.0, 100),
            }),
            MotionTimelineStep::Track(MotionTrack {
                target: ".".into(),
                source: tween(MotionProperty::Opacity, 1.0, 0.0, 100),
            }),
        ]),
    );
    let root = UiNode::text("x")
        .with_key("x")
        .with_motion(tween(MotionProperty::TranslateX, 0.0, 100.0, 100))
        .with_timeline(overlapping);
    let result = motion::reconcile_node_motion(&root, &mut runtime, now);
    println!(
        "failed_host_reconcile error={} leaked_active_sources={}",
        result.is_err(),
        runtime.resource_usage().active
    );

    let mut r = MotionRuntime::new(MotionPreference::Normal);
    let key = MotionKey::for_node("root/card", MotionProperty::Opacity);
    let h = r
        .start_timeline(
            scope.clone(),
            tl("play", tween(MotionProperty::Opacity, 0.0, 1.0, 1000)),
            now,
        )
        .unwrap();
    let before = r
        .snapshot(now + Duration::from_millis(400))
        .get(&key)
        .copied();
    r.play_timeline(&h, now + Duration::from_millis(400))
        .unwrap();
    println!(
        "repeated_play before={before:?} after={:?}",
        r.snapshot(now + Duration::from_millis(400)).get(&key)
    );
    let mut r = MotionRuntime::new(MotionPreference::Normal);
    let timeline = MotionTimeline::new(
        "sequence",
        MotionTimelineStep::Sequence(vec![
            MotionTimelineStep::Track(MotionTrack {
                target: ".".into(),
                source: tween(MotionProperty::Opacity, 0.0, 1.0, 100),
            }),
            MotionTimelineStep::Track(MotionTrack {
                target: ".".into(),
                source: tween(MotionProperty::TranslateX, 0.0, 100.0, 100),
            }),
        ]),
    );
    let h = r.start_timeline(scope.clone(), timeline, now).unwrap();
    let _ = r.tick(now + Duration::from_millis(150));
    r.seek_timeline(&h, 0, now + Duration::from_millis(150))
        .unwrap();
    println!(
        "seek_back_future_track expected_x=0 actual_x={:?}",
        r.snapshot(now + Duration::from_millis(150))
            .get(&MotionKey::for_node(
                "root/card",
                MotionProperty::TranslateX
            ))
    );

    let state = Rc::new(RefCell::new(UiRuntimeState::new()));
    let mut engine = RuntimeEngine::new();
    let compiled=engine.compile(r#"fn render_row(ctx,p){text("A").with_key(p.key).motion(motion_transition("opacity",0.0,1.0,#{duration_ms:1000}))}
      fn view(ctx){virtual_collection(#{key:"list",data:[#{key:"alpha"}],height:100,estimated_height:20},Fn("render_row"))}"#).unwrap();
    let mut life = ScriptLifecycle::new(
        compiled,
        state.clone(),
        ComponentInstancePath::root("View", "virtual"),
        None,
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    life.start(&mut engine).unwrap();
    let values = state.borrow().motions.snapshot(Instant::now());
    println!(
        "virtual_row_motion renderer_key_present={} stored_keys={:?}",
        values.contains_key(&MotionKey::for_node(
            "root/item:alpha",
            MotionProperty::Opacity
        )),
        values.keys().collect::<Vec<_>>()
    );
}
