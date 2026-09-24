use gpui::{
    Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle, point,
    px,
};
use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Duration};
struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
    render_count: usize,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.render_count += 1;
        if self.render_count >= 64 {
            eprintln!(
                "PROBE_GUARD: idle linked chart host reached 64 renders without parking; stopping test process"
            );
            std::process::exit(70);
        }
        let element = if self.view.state() == ScriptViewState::Active {
            self.view.element().unwrap()
        } else {
            gpui::div().into_any_element()
        };
        self.host.container(element)
    }
}
fn pump(cx: &mut TestAppContext, v: &mut VisualTestContext) {
    for _ in 0..8 {
        cx.background_executor
            .advance_clock(Duration::from_millis(20));
        v.run_until_parked();
        cx.refresh().unwrap();
        v.run_until_parked();
    }
}
fn status(v: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|n| n.role == "status")
            .unwrap()
            .name
            .clone()
    })
}

fn mount_extension(
    cx: &mut TestAppContext,
    src: &str,
    name: &str,
    extension: impl ScriptViewExtension + 'static,
    clock: Option<RuntimeClock>,
    preference: MotionPreference,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    let repo = std::env::var("GPUI_RHAI_REVIEW_REPO").unwrap();
    let id = ModuleId::parse("main").unwrap();
    let mut p = EmbeddedScriptView::new(
        id.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (id, src.to_owned()),
            (
                ModuleId::parse("charts/bar_chart").unwrap(),
                std::fs::read_to_string(format!("{repo}/registry/charts/bar_chart.rhai")).unwrap(),
            ),
            (
                ModuleId::parse("charts/chart").unwrap(),
                std::fs::read_to_string(format!("{repo}/registry/charts/chart.rhai")).unwrap(),
            ),
        ])),
        std::fs::read_to_string(format!("{repo}/registry/themes/default_dark.rhai")).unwrap(),
    )
    .extension(extension)
    .motion_preference(preference);
    if let Some(clock) = clock {
        p = p.runtime_clock(clock);
    }
    let p = p.prepare().unwrap();
    let name = name.to_owned();
    let cap = Rc::new(RefCell::new(None));
    let take = cap.clone();
    let w = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&name, cx).unwrap();
        let view = p
            .mount(ScriptViewConfig::new(&name), host.clone(), window, cx)
            .unwrap();
        *take.borrow_mut() = Some(view.clone());
        Host {
            host,
            view,
            render_count: 0,
        }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let v = cap.borrow().as_ref().unwrap().clone();
    (w, v)
}
#[derive(Default)]
struct NativeState {
    active: std::cell::Cell<bool>,
    fail_resume: std::cell::Cell<bool>,
    fail_suspend: std::cell::Cell<bool>,
    resumes: std::cell::Cell<usize>,
    suspends: std::cell::Cell<usize>,
}
struct HookPrimitive(Rc<NativeState>);
impl PrimitiveHandler for HookPrimitive {
    fn unmount(&mut self, _: &PrimitiveInstanceId) {
        self.0.active.set(false);
    }
    fn mount(&mut self, _: &PrimitiveInstance) -> Result<(), String> {
        self.0.active.set(true);
        Ok(())
    }
    fn suspend(&mut self, _: &PrimitiveInstanceId, _: &mut gpui::App) {
        self.0.suspends.set(self.0.suspends.get() + 1);
        if self.0.fail_suspend.replace(false) {
            panic!("injected native suspend failure");
        }
        self.0.active.set(false);
    }
    fn resume(&mut self, _: &PrimitiveInstanceId, _: &mut gpui::App) {
        self.0.resumes.set(self.0.resumes.get() + 1);
        if self.0.fail_resume.replace(false) {
            panic!("injected native resume failure");
        }
        self.0.active.set(true);
    }
    fn render(
        &mut self,
        _: &PrimitiveInstance,
        _: &PrimitiveEventEmitter,
        _: &PrimitiveTheme,
        _: &mut Window,
        _: &mut gpui::App,
    ) -> Result<gpui::AnyElement, String> {
        Ok(gpui::div().into_any_element())
    }
}
struct HookExtension(Vec<Rc<NativeState>>);
impl ScriptViewExtension for HookExtension {
    fn configure_engine(&self, e: &mut RuntimeEngine) -> Result<(), String> {
        for (i, state) in self.0.iter().enumerate() {
            e.register_primitive(
                PrimitiveDescriptor {
                    id: PrimitiveId::parse(&format!("audit.p{i}")).unwrap(),
                    export: format!("P{i}"),
                    props: BTreeMap::new(),
                    events: BTreeMap::new(),
                    state: ComponentStateSchema::default(),
                    lifecycle: true,
                    effect: None,
                },
                HookPrimitive(state.clone()),
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
const HOOK_SOURCE: &str = r#"fn view(ctx){row([audit::P0(#{key:"a"}).with_key("a"),audit::P1(#{key:"b"}).with_key("b"),audit::P2(#{key:"c"}).with_key("c")])}"#;
#[gpui::test]
fn native_resume_failure_remains_suspended_and_retryable(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let states = (0..3)
        .map(|_| Rc::new(NativeState::default()))
        .collect::<Vec<_>>();
    states[1].fail_resume.set(true);
    let (w, view) = mount_extension(
        cx,
        HOOK_SOURCE,
        "resume-hook-failure",
        HookExtension(states.clone()),
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    v.update(|window, cx| view.suspend(window, cx).unwrap());
    let result = v.update(|_, cx| view.resume(cx));
    let state_after = view.state();
    let active_after = states.iter().map(|s| s.active.get()).collect::<Vec<_>>();
    let retry = v.update(|_, cx| view.resume(cx));
    println!(
        "RESUME_HOOK_FAILURE result={result:?} state_after={state_after:?} active_after={active_after:?} retry={retry:?} resume_calls={:?}",
        states.iter().map(|s| s.resumes.get()).collect::<Vec<_>>()
    );
    assert!(result.is_err());
    assert_eq!(
        state_after,
        ScriptViewState::Suspended,
        "failed native resume exposed a partially resumed view as Active"
    );
    assert!(
        active_after.iter().all(|active| !active),
        "resume failure must quiesce already-resumed peers"
    );
    assert!(
        matches!(retry, Ok(true)),
        "failed resume must permit a real retry"
    );
    assert!(
        states.iter().all(|s| s.active.get()),
        "successful retry must resume every primitive"
    );
}
#[gpui::test]
fn native_suspend_failure_does_not_skip_later_resources(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let states = (0..3)
        .map(|_| Rc::new(NativeState::default()))
        .collect::<Vec<_>>();
    states[1].fail_suspend.set(true);
    let (w, view) = mount_extension(
        cx,
        HOOK_SOURCE,
        "suspend-hook-failure",
        HookExtension(states.clone()),
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let result = v.update(|window, cx| view.suspend(window, cx));
    let state = view.state();
    let active = states.iter().map(|s| s.active.get()).collect::<Vec<_>>();
    let retry = v.update(|window, cx| view.suspend(window, cx));
    println!(
        "SUSPEND_HOOK_FAILURE result={result:?} state={state:?} active={active:?} retry={retry:?} suspend_calls={:?}",
        states.iter().map(|s| s.suspends.get()).collect::<Vec<_>>()
    );
    assert!(result.is_err());
    assert!(
        match state {
            ScriptViewState::Active => active.iter().all(|value| *value),
            ScriptViewState::Suspended | ScriptViewState::Disposed =>
                active.iter().all(|value| !*value),
        },
        "suspend failure left native instances split across active and quiescent states"
    );
}
struct MotionExtension {
    data: NativeChartData,
    positions: std::sync::Arc<std::sync::Mutex<Vec<ChartPoint>>>,
}
struct MovingRenderer(std::sync::Arc<std::sync::Mutex<Vec<ChartPoint>>>);
impl HostChartSeries for MovingRenderer {
    fn layout(&self, c: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
        let val = c
            .dataset
            .column("y")
            .unwrap()
            .value(0)
            .unwrap()
            .as_number()
            .unwrap();
        let rect = ChartRect {
            x: c.bounds.x + 20. + val * 200.,
            y: c.bounds.y + 30.,
            width: 40.,
            height: 40.,
        };
        self.0.lock().unwrap().push(rect.center());
        Ok(vec![ChartMark {
            key: "moving".into(),
            region_key: c.spec.coordinate.clone(),
            role: ChartMarkRole::Data,
            datum: Some(ChartDatumRef {
                dataset: c.spec.dataset.clone(),
                series: c.spec.key.clone(),
                key: "r0".into(),
            }),
            series_key: c.spec.key.clone(),
            datum_key: "r0".into(),
            geometry: ChartMarkGeometry::Rect(rect),
            fill: Some(c.theme.palette[0]),
            stroke: None,
            label: "moving".into(),
            value: Some(val),
            interactive: true,
            selected: false,
        }])
    }
}
impl ScriptViewExtension for MotionExtension {
    fn configure_engine(&self, e: &mut RuntimeEngine) -> Result<(), String> {
        e.register_chart_series("moving", MovingRenderer(self.positions.clone()))
            .map_err(|e| e.to_string())
    }
    fn configure_runtime(&self, r: &mut UiRuntimeState) -> Result<(), String> {
        r.register_native_chart_data("stream", self.data.clone())
            .map_err(|e| e.to_string())
    }
}
fn moving_data(y: i64) -> ChartDataset {
    ChartDataset::from_rows(
        "main",
        &[BTreeMap::from([
            ("id".into(), UiValue::String("r0".into())),
            ("x".into(), UiValue::Integer(0)),
            ("y".into(), UiValue::Integer(y)),
        ])],
        Some("id".into()),
        ChartDataLimits::default(),
    )
    .unwrap()
}
fn click_chart_point(v: &mut VisualTestContext, view: &ScriptViewHandle, p: ChartPoint) {
    let b = v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("figure", "Control")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    v.simulate_click(
        point(px((b.x + p.x + 1.) as f32), px((b.y + p.y + 1.) as f32)),
        gpui::Modifiers::default(),
    );
    v.run_until_parked();
}
#[gpui::test]
fn chart_motion_freezes_across_suspension(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let clock = ManualRuntimeClock::new(std::time::Instant::now());
    let data = NativeChartData::new([moving_data(0)], ChartDataLimits::default()).unwrap();
    let positions = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let src = r#"import "charts/chart" as chart;fn state_schema(){#{fields:#{hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}fn clicked(ctx,p){ctx.set_state("hits",ctx.get_state("hits")+1);}fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(ctx.get_state("hits").to_string()),chart::Chart(#{key:"c",data:ctx.get_native_chart_data("stream"),spec:#{title:"Control",legend:#{visible:false},motion:#{duration:"slow",easing:"standard"},series:[#{key:"s",kind:"custom",renderer:"moving",encode:#{x:"x",y:"y"}}]},on_select:Fn("clicked")}).with_style(style().width(px(420)).height(px(300)))])}"#;
    let (w, view) = mount_extension(
        cx,
        src,
        "frozen-chart-motion",
        MotionExtension {
            data: data.clone(),
            positions: positions.clone(),
        },
        Some(clock.clock()),
        MotionPreference::Normal,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    clock.advance(Duration::from_secs(2));
    pump(cx, &mut v);
    let old = positions.lock().unwrap()[0];
    data.replace([moving_data(1)]).unwrap();
    pump(cx, &mut v);
    click_chart_point(&mut v, &view, old);
    let before = status(&mut v, &view);
    assert_eq!(
        before, "1",
        "fixture must hit the start of the transition before suspension"
    );
    v.update(|window, cx| view.suspend(window, cx).unwrap());
    v.run_until_parked();
    clock.advance(Duration::from_secs(60));
    v.update(|_, cx| view.resume(cx).unwrap());
    pump(cx, &mut v);
    click_chart_point(&mut v, &view, old);
    let after_old = status(&mut v, &view);
    let terminal = *positions.lock().unwrap().last().unwrap();
    click_chart_point(&mut v, &view, terminal);
    let after_terminal = status(&mut v, &view);
    println!(
        "FROZEN_MOTION start={old:?} terminal={terminal:?} hits_before={before} hits_after_old={after_old} hits_after_terminal={after_terminal}"
    );
    assert_eq!(
        after_old, "2",
        "chart animation caught up suspended wall time instead of resuming the frozen sample"
    );
}
fn presented_revision(v: &mut VisualTestContext, view: &ScriptViewHandle) -> u64 {
    v.update(|_, cx| {
        let tree = view.accessibility_snapshot(cx).unwrap();
        let node = tree
            .find_by_role_and_name("figure", "Control")
            .next()
            .unwrap();
        let Some(UiValue::Map(values)) = &node.value else {
            panic!("missing chart projection")
        };
        let UiValue::Integer(rev) = values["revision"] else {
            panic!()
        };
        rev as u64
    })
}
const STREAM_SOURCE: &str = r#"import "charts/chart" as chart;fn view(ctx){chart::Chart(#{key:"c",data:ctx.get_native_chart_data("stream"),spec:#{title:"Control",legend:#{visible:false},series:[#{key:"s",kind:"custom",renderer:"moving",encode:#{x:"x",y:"y"}}]}}).with_style(style().width(px(420)).height(px(300)))}"#;
#[gpui::test]
fn resume_finishes_a_prepared_but_unpresented_revision(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let data = NativeChartData::new([moving_data(0)], ChartDataLimits::default()).unwrap();
    let positions = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let (w, view) = mount_extension(
        cx,
        STREAM_SOURCE,
        "pending-layout",
        MotionExtension {
            data: data.clone(),
            positions: positions.clone(),
        },
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let old = presented_revision(&mut v, &view);
    let old_layouts = positions.lock().unwrap().len();
    let wanted = data.replace([moving_data(1)]).unwrap();
    let mut steps = 0;
    while positions.lock().unwrap().len() == old_layouts {
        assert!(steps < 100, "layout boundary not reached");
        assert!(cx.background_executor.tick(), "no scheduled work");
        steps += 1;
    }
    let before = presented_revision(&mut v, &view);
    assert_eq!(
        before, old,
        "fixture must pause after geometry calculation but before foreground installation"
    );
    v.update(|w, cx| view.suspend(w, cx).unwrap());
    v.run_until_parked();
    v.update(|_, cx| view.resume(cx).unwrap());
    pump(cx, &mut v);
    let after = presented_revision(&mut v, &view);
    println!(
        "PREPARED_NOT_PRESENTED ticks={steps} old={old} wanted={wanted} before_suspend={before} after_resume={after} layouts={}",
        positions.lock().unwrap().len()
    );
    assert_eq!(
        after, wanted,
        "prepared revision was mistaken for a committed scene after layout cancellation"
    );
}
#[gpui::test]
fn failed_compensation_does_not_claim_quiescent_view(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let states = (0..3)
        .map(|_| Rc::new(NativeState::default()))
        .collect::<Vec<_>>();
    let (w, view) = mount_extension(
        cx,
        HOOK_SOURCE,
        "failed-compensation",
        HookExtension(states.clone()),
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    v.update(|w, cx| view.suspend(w, cx).unwrap());
    states[1].fail_resume.set(true);
    states[0].fail_suspend.set(true);
    let result = v.update(|_, cx| view.resume(cx));
    let active = states.iter().map(|s| s.active.get()).collect::<Vec<_>>();
    println!(
        "COMPENSATION_FAILURE result={result:?} view_state={:?} active={active:?}",
        view.state()
    );
    assert!(result.is_err());
    assert!(
        active.iter().all(|a| !*a),
        "failed compensation leaves a native instance active under a Suspended view"
    );
}
struct ClockBomb {
    clock: ManualRuntimeClock,
    fail: Rc<std::cell::Cell<bool>>,
}
impl PrimitiveHandler for ClockBomb {
    fn resume(&mut self, _: &PrimitiveInstanceId, _: &mut gpui::App) {
        if self.fail.replace(false) {
            self.clock.advance(Duration::from_secs(60));
            panic!("injected slow native resume failure");
        }
    }
    fn render(
        &mut self,
        _: &PrimitiveInstance,
        _: &PrimitiveEventEmitter,
        _: &PrimitiveTheme,
        _: &mut Window,
        _: &mut gpui::App,
    ) -> Result<gpui::AnyElement, String> {
        Ok(gpui::div().into_any_element())
    }
}
struct FailedResumeMotionExtension {
    motion: MotionExtension,
    clock: ManualRuntimeClock,
    fail: Rc<std::cell::Cell<bool>>,
}
impl ScriptViewExtension for FailedResumeMotionExtension {
    fn configure_engine(&self, e: &mut RuntimeEngine) -> Result<(), String> {
        self.motion.configure_engine(e)?;
        e.register_primitive(
            PrimitiveDescriptor {
                id: PrimitiveId::parse("zz_lifecycle.clock_bomb").unwrap(),
                export: "ClockBomb".into(),
                props: BTreeMap::new(),
                events: BTreeMap::new(),
                state: ComponentStateSchema::default(),
                lifecycle: true,
                effect: None,
            },
            ClockBomb {
                clock: self.clock.clone(),
                fail: self.fail.clone(),
            },
        )
        .map_err(|e| e.to_string())
    }
    fn configure_runtime(&self, r: &mut UiRuntimeState) -> Result<(), String> {
        self.motion.configure_runtime(r)
    }
}
#[gpui::test]
fn failed_resume_does_not_consume_chart_motion_time(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let clock = ManualRuntimeClock::new(std::time::Instant::now());
    let data = NativeChartData::new([moving_data(0)], ChartDataLimits::default()).unwrap();
    let positions = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let fail = Rc::new(std::cell::Cell::new(true));
    let src = r#"import "charts/chart" as chart;fn state_schema(){#{fields:#{hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}fn clicked(ctx,p){ctx.set_state("hits",ctx.get_state("hits")+1);}fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(ctx.get_state("hits").to_string()),chart::Chart(#{key:"c",data:ctx.get_native_chart_data("stream"),spec:#{title:"Control",legend:#{visible:false},motion:#{duration:"slow",easing:"standard"},series:[#{key:"s",kind:"custom",renderer:"moving",encode:#{x:"x",y:"y"}}]},on_select:Fn("clicked")}).with_style(style().width(px(420)).height(px(300))),zz_lifecycle::ClockBomb(#{key:"bomb"}).with_key("bomb")])}"#;
    let (w, view) = mount_extension(
        cx,
        src,
        "failed-resume-motion",
        FailedResumeMotionExtension {
            motion: MotionExtension {
                data: data.clone(),
                positions: positions.clone(),
            },
            clock: clock.clone(),
            fail,
        },
        Some(clock.clock()),
        MotionPreference::Normal,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    clock.advance(Duration::from_secs(2));
    pump(cx, &mut v);
    let old = positions.lock().unwrap()[0];
    data.replace([moving_data(1)]).unwrap();
    pump(cx, &mut v);
    click_chart_point(&mut v, &view, old);
    assert_eq!(status(&mut v, &view), "1");
    v.update(|w, cx| view.suspend(w, cx).unwrap());
    clock.advance(Duration::from_secs(60));
    let failed = v.update(|_, cx| view.resume(cx));
    assert!(failed.is_err());
    assert_eq!(view.state(), ScriptViewState::Suspended);
    v.update(|_, cx| view.resume(cx).unwrap());
    pump(cx, &mut v);
    click_chart_point(&mut v, &view, old);
    let old_hits = status(&mut v, &view);
    let target = *positions.lock().unwrap().last().unwrap();
    click_chart_point(&mut v, &view, target);
    println!(
        "FAILED_RESUME_MOTION failed={failed:?} hits_at_frozen={old_hits} hits_after_target={}",
        status(&mut v, &view)
    );
    assert_eq!(
        old_hits, "2",
        "time spent preparing a failed resume advanced an animation whose view never became Active"
    );
}
struct ViewportMarker(std::sync::Arc<std::sync::Mutex<Vec<ChartPoint>>>);
impl HostChartSeries for ViewportMarker {
    fn layout(&self, c: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
        let p = ChartPoint {
            x: c.x_scale.unwrap().map_number(2.).unwrap(),
            y: c.bounds.center().y,
        };
        self.0.lock().unwrap().push(p);
        Ok(vec![ChartMark {
            key: "viewport_marker".into(),
            region_key: c.spec.coordinate.clone(),
            role: ChartMarkRole::Data,
            datum: Some(ChartDatumRef {
                dataset: c.spec.dataset.clone(),
                series: c.spec.key.clone(),
                key: "r0".into(),
            }),
            series_key: c.spec.key.clone(),
            datum_key: "r0".into(),
            geometry: ChartMarkGeometry::Rect(ChartRect {
                x: p.x - 6.,
                y: p.y - 6.,
                width: 12.,
                height: 12.,
            }),
            fill: Some(c.theme.palette[0]),
            stroke: None,
            label: "r0".into(),
            value: Some(2.),
            interactive: true,
            selected: false,
        }])
    }
}
struct ViewportExtension(std::sync::Arc<std::sync::Mutex<Vec<ChartPoint>>>);
impl ScriptViewExtension for ViewportExtension {
    fn configure_engine(&self, e: &mut RuntimeEngine) -> Result<(), String> {
        e.register_chart_series("viewport_marker", ViewportMarker(self.0.clone()))
            .map_err(|e| e.to_string())
    }
}
#[gpui::test]
fn canceling_preview_invalidates_presented_viewport(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let positions = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let src = r#"import "charts/chart" as chart;fn state_schema(){#{fields:#{hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}fn clicked(ctx,p){ctx.set_state("hits",ctx.get_state("hits")+1);}fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(ctx.get_state("hits").to_string()),chart::Chart(#{key:"c",key_dimension:"id",zoom:1.0,viewport_revision:0,data:[#{id:"r0",x:2,y:0},#{id:"r1",x:10,y:1}],spec:#{title:"Control",legend:#{visible:false},axes:[#{key:"x",position:"bottom",min:0,max:10}],series:[#{key:"s",kind:"custom",renderer:"viewport_marker",encode:#{x:"x",y:"y"}}]},on_select:Fn("clicked")}).with_style(style().width(px(420)).height(px(300)))])}"#;
    let (w, view) = mount_extension(
        cx,
        src,
        "cancel-preview",
        ViewportExtension(positions.clone()),
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let committed = positions.lock().unwrap()[0];
    let b = v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("figure", "Control")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    v.simulate_event(gpui::ScrollWheelEvent {
        position: point(px((b.x + 180.) as f32), px((b.y + 160.) as f32)),
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(400. * std::f32::consts::LN_2))),
        touch_phase: gpui::TouchPhase::Started,
        ..Default::default()
    });
    pump(cx, &mut v);
    let preview = *positions.lock().unwrap().last().unwrap();
    assert!((preview.x - committed.x).abs() > 20.);
    click_chart_point(&mut v, &view, preview);
    assert_eq!(status(&mut v, &view), "1");
    v.update(|w, cx| view.suspend(w, cx).unwrap());
    v.update(|_, cx| view.resume(cx).unwrap());
    pump(cx, &mut v);
    click_chart_point(&mut v, &view, committed);
    let committed_hits = status(&mut v, &view);
    click_chart_point(&mut v, &view, preview);
    println!(
        "CANCELLED_PREVIEW committed={committed:?} preview={preview:?} committed_hits={committed_hits} preview_hits={} layouts={}",
        status(&mut v, &view),
        positions.lock().unwrap().len()
    );
    assert_eq!(
        committed_hits, "2",
        "controller reset the viewport but kept presenting the cancelled preview"
    );
}
struct GeoMarker(std::sync::Arc<std::sync::Mutex<BTreeMap<String, ChartPoint>>>);
impl HostChartSeries for GeoMarker {
    fn layout(&self, c: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
        let p = ChartPoint {
            x: c.bounds.x + 30.,
            y: c.bounds.y + 30.,
        };
        self.0.lock().unwrap().insert(c.spec.key.clone(), p);
        Ok(vec![ChartMark {
            key: format!("geo:{}", c.spec.key),
            region_key: c.spec.coordinate.clone(),
            role: ChartMarkRole::Data,
            datum: Some(ChartDatumRef {
                dataset: c.spec.dataset.clone(),
                series: c.spec.key.clone(),
                key: "r0".into(),
            }),
            series_key: c.spec.key.clone(),
            datum_key: "r0".into(),
            geometry: ChartMarkGeometry::Rect(ChartRect {
                x: p.x - 10.,
                y: p.y - 10.,
                width: 20.,
                height: 20.,
            }),
            fill: Some(c.theme.palette[0]),
            stroke: None,
            label: "r0".into(),
            value: None,
            interactive: true,
            selected: false,
        }])
    }
}
struct GeoExtension(std::sync::Arc<std::sync::Mutex<BTreeMap<String, ChartPoint>>>);
impl ScriptViewExtension for GeoExtension {
    fn configure_engine(&self, e: &mut RuntimeEngine) -> Result<(), String> {
        e.register_chart_map(ChartGeoMap::from_geojson("map",r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"square","properties":{},"geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,10],[0,10],[0,0]]]}}]}"#).unwrap()).map_err(|e|e.to_string())?;
        e.register_chart_series("geo_marker", GeoMarker(self.0.clone()))
            .map_err(|e| e.to_string())
    }
}
fn figure_point(
    v: &mut VisualTestContext,
    view: &ScriptViewHandle,
    name: &str,
    p: ChartPoint,
) -> gpui::Point<gpui::Pixels> {
    v.update(|_, cx| {
        let tree = view.accessibility_snapshot(cx).unwrap();
        let b = tree
            .find_by_role_and_name("figure", name)
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual;
        point(px((b.x + p.x + 1.) as f32), px((b.y + p.y + 1.) as f32))
    })
}
#[gpui::test]
fn geo_link_group_synchronizes_acknowledged_viewport(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let positions = std::sync::Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let src = r#"import "charts/chart" as chart;
 fn state_schema(){#{fields:#{z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},rev:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},source_hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},target_hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
 fn zoomed(ctx,p){ctx.set_state("z",p.zoom);ctx.set_state("rev",p.viewport_revision);}fn source_clicked(ctx,p){ctx.set_state("source_hits",ctx.get_state("source_hits")+1);}fn target_clicked(ctx,p){ctx.set_state("target_hits",ctx.get_state("target_hits")+1);}
 fn one(ctx,k){chart::Chart(#{key:k,key_dimension:"id",data:[#{id:"r0",x:0,y:0}],zoom:if k=="source"{ctx.get_state("z")}else{1.0},viewport_revision:if k=="source"{ctx.get_state("rev")}else{0},spec:#{title:k,legend:#{visible:false},link_group:"maps",link_domain:"location",regions:[#{key:"main",kind:"geo_2d",map:"map"}],series:[#{key:k,kind:"custom",renderer:"geo_marker",encode:#{x:"x",y:"y"}}]},on_zoom_change:if k=="source"{Fn("zoomed")}else{()},on_select:if k=="source"{Fn("source_clicked")}else{Fn("target_clicked")}}).with_style(style().width(px(280)).height(px(240)))}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("source_hits")}|${ctx.get_state("target_hits")}|${ctx.get_state("z")}`),row([one(ctx,"source"),one(ctx,"target")])])}"#;
    let (w, view) = mount_extension(
        cx,
        src,
        "geo-link",
        GeoExtension(positions.clone()),
        None,
        MotionPreference::None,
    );
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let ps = positions.lock().unwrap()["source"];
    let pt = positions.lock().unwrap()["target"];
    let source = figure_point(&mut v, &view, "source", ps);
    let target = figure_point(&mut v, &view, "target", pt);
    v.simulate_click(source, gpui::Modifiers::default());
    v.simulate_click(target, gpui::Modifiers::default());
    v.run_until_parked();
    assert!(status(&mut v, &view).starts_with("1|1|"));
    let wheel = figure_point(&mut v, &view, "source", ChartPoint { x: 150., y: 140. });
    v.simulate_event(gpui::ScrollWheelEvent {
        position: wheel,
        delta: gpui::ScrollDelta::Lines(point(0., -25.)),
        touch_phase: gpui::TouchPhase::Moved,
        ..Default::default()
    });
    pump(cx, &mut v);
    v.simulate_click(source, gpui::Modifiers::default());
    v.simulate_click(target, gpui::Modifiers::default());
    v.run_until_parked();
    let result = status(&mut v, &view);
    println!("GEO_LINK source_hits|target_hits|zoom={result}");
    let zoom = result.split('|').nth(2).unwrap().parse::<f64>().unwrap();
    assert!(zoom > 2.0, "fixture must commit a real source zoom");
    assert!(
        result.starts_with("1|1|"),
        "source viewport changed but linked target remained at its old geometry"
    );
}
