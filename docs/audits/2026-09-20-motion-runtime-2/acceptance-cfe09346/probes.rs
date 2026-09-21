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
fn timeline(name: &str, target: &str, p: MotionProperty, autoplay: bool) -> MotionTimeline {
    let mut t = MotionTimeline::new(
        name,
        MotionTimelineStep::Track(MotionTrack {
            target: target.into(),
            source: tween(p, 0., 1., 1000),
        }),
    );
    t.autoplay = autoplay;
    t
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
fn main() {
    let schema = ComponentStateSchema::new(BTreeMap::from([(
        "switched".into(),
        StateField::new(ValueSchema::Bool, UiValue::Bool(false)),
    )]))
    .unwrap();
    let source = r#"fn view(ctx){let child=if ctx.get_state("switched"){box([]).with_key("title")}else{text("Title").with_key("title")};column([child]).with_key("panel").timeline(motion_timeline("intro",motion_track("title",motion_transition("width",100.0,200.0,#{duration_ms:1000,easing:"linear"})),#{}))}"#;
    let (mut e, mut l, r, clock) = lifecycle(source, &schema);
    let old = l
        .retained()
        .nodes()
        .find(|n| n.key() == Some("title"))
        .unwrap()
        .id();
    clock.advance(Duration::from_millis(250));
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
    let newkey = MotionKey::for_node(
        &format!("window:w/view:v/root/node:{}", new.get()),
        MotionProperty::Width,
    );
    let oldkey = MotionKey::for_node(
        &format!("window:w/view:v/root/node:{}", old.get()),
        MotionProperty::Width,
    );
    let values = r.borrow().motions.snapshot(clock.clock().now());
    println!(
        "retained_target_replacement old={old:?} new={new:?} old_unmounted={} old_sample={:?} new_sample={:?}",
        l.retained().node(old).is_none(),
        values.get(&oldkey),
        values.get(&newkey)
    );
    // The headless declaration compiler must use the same path encoding for direct and timeline tracks.
    let now = Instant::now();
    let direct = UiNode::column(vec![
        UiNode::text("Title").with_key("title").with_motion(tween(
            MotionProperty::Opacity,
            0.,
            1.,
            1000,
        )),
    ])
    .with_key("panel");
    let via_timeline = UiNode::column(vec![UiNode::text("Title").with_key("title")])
        .with_key("panel")
        .with_timeline(timeline("intro", "title", MotionProperty::Opacity, true));
    let mut a = MotionRuntime::new(MotionPreference::Normal);
    let mut b = MotionRuntime::new(MotionPreference::Normal);
    let direct = motion::reconcile_node_motion(&direct, &mut a, now).unwrap();
    let tracks = motion::reconcile_node_motion(&via_timeline, &mut b, now).unwrap();
    println!(
        "headless_same_property direct_keys={:?} timeline_keys={:?} keys_equal={}",
        direct.keys().collect::<Vec<_>>(),
        tracks.keys().collect::<Vec<_>>(),
        direct.keys().eq(tracks.keys())
    );
    // Reverse traversal order compared to the last report's one-timeline case.
    for order in ["ba", "ab"] {
        let source = format!(
            r#"fn view(ctx){{let switching=ctx.get_state("switched");let a=motion_timeline("a",motion_track(".",motion_transition("opacity",0.0,1.0,#{{duration_ms:1000}})),#{{autoplay:!switching}});let b=motion_timeline("b",motion_track(".",motion_transition("width",100.0,200.0,#{{duration_ms:1000}})),#{{autoplay:switching}});let n=text("x").with_key("x");{} }}"#,
            if order == "ba" {
                "n.timeline(b).timeline(a)"
            } else {
                "n.timeline(a).timeline(b)"
            }
        );
        let (mut e, mut l, r, _clock) = lifecycle(&source, &schema);
        r.borrow_mut().budgets.active_motions = 1;
        r.borrow_mut()
            .set_component_state_from_host(
                &ComponentInstancePath::root("View", "v"),
                "switched",
                UiValue::Bool(true),
            )
            .unwrap();
        println!(
            "two_timeline_budget order={order} expected_active1 actual={:?}",
            l.render_dirty(&mut e)
        );
    }
    // Analytic inertia should not jump to the asymptote at the safety duration cap.
    let now = Instant::now();
    let spec = MotionSource::Inertia(MotionInertia {
        property: MotionProperty::TranslateX,
        from: 0.,
        velocity: 100.,
        friction: 0.1,
        min: None,
        max: None,
        bounce: 0.,
        snap_points: vec![],
        intent: MotionIntent::Decorative,
    });
    let mut r = MotionRuntime::new(MotionPreference::Normal);
    let k = r
        .start(ComponentInstancePath::root("UiNode", "root"), spec, now)
        .unwrap();
    let a = r.sample(&k, now + Duration::from_millis(9999)).unwrap();
    let b = r.sample(&k, now + Duration::from_millis(10000)).unwrap();
    println!(
        "inertia_no_snap_points t9999={a} t10000={b} one_ms_jump={}",
        b - a
    );
    // Window release is a teardown boundary, not a pause that future mounts should inherit.
    let src = r#"fn view(ctx){text("x").with_key("x").motion(motion_transition("width",0.0,100.0,#{duration_ms:1000,easing:"linear"}))}"#;
    let (mut e, mut l, r, clock) = lifecycle(src, &ComponentStateSchema::default());
    clock.advance(Duration::from_millis(200));
    l.suspend(&mut e).unwrap();
    l.dispose(&mut e).unwrap();
    r.borrow_mut()
        .release_window("w", &ComponentInstancePath::root("View", "v"))
        .unwrap();
    let c = e.compile(src).unwrap();
    let mut reopened = ScriptLifecycle::new(
        c,
        r.clone(),
        ComponentInstancePath::root("View", "v"),
        Some("w".into()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap()
    .with_view_id("v");
    reopened.start(&mut e).unwrap();
    clock.advance(Duration::from_millis(200));
    let f = r.borrow_mut().motions.tick(clock.clock().now());
    println!(
        "reopen_after_suspended_release expected_width20 actual={:?} needs_frame={}",
        f.values.values().collect::<Vec<_>>(),
        f.needs_frame
    );
    // Closing w must not reclaim an exit owned by w2.
    let schema = ComponentStateSchema::new(BTreeMap::from([(
        "visible".into(),
        StateField::new(ValueSchema::Bool, UiValue::Bool(true)),
    )]))
    .unwrap();
    let source = r#"fn view(ctx){if ctx.get_state("visible"){column([text("gone").with_key("panel").exit_motion(motion_transition("opacity",1.0,0.0,#{duration_ms:1000}))])}else{column([])}}"#;
    let (mut e, mut w, r, clock) = lifecycle(source, &schema);
    let c = e.compile(source).unwrap();
    let root2 = ComponentInstancePath::root("View", "v2");
    let mut w2 = ScriptLifecycle::new(
        c,
        r.clone(),
        root2.clone(),
        Some("w2".into()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap()
    .with_view_id("v2");
    w2.start(&mut e).unwrap();
    let panel = w2
        .retained()
        .nodes()
        .find(|n| n.key() == Some("panel"))
        .unwrap()
        .id();
    let b = GeometryBounds::new(0., 0., 100., 20.).unwrap();
    r.borrow_mut().presentation_geometry("v2").update(
        panel,
        ElementGeometry {
            layout: b,
            visual: b,
            clip: None,
        },
    );
    r.borrow_mut()
        .set_component_state_from_host(&root2, "visible", UiValue::Bool(false))
        .unwrap();
    w2.render_dirty(&mut e).unwrap();
    let before = r.borrow().motions.resource_usage().active;
    w.dispose(&mut e).unwrap();
    r.borrow_mut()
        .release_window("w", &ComponentInstancePath::root("View", "v"))
        .unwrap();
    println!(
        "release_w_affects_w2_ghost before_active={before} after_active={} remaining={:?}",
        r.borrow().motions.resource_usage().active,
        r.borrow().motions.snapshot(clock.clock().now())
    );
}
