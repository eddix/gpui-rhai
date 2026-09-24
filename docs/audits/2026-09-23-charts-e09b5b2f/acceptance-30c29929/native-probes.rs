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
