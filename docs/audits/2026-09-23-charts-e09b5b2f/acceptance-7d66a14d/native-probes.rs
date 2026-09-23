use gpui::{
    Context, IntoElement, Render, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext,
    Window, WindowHandle, point, px,
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
fn mount(cx: &mut TestAppContext, src: &str, name: &str) -> (WindowHandle<Host>, ScriptViewHandle) {
    let repo = std::env::var("GPUI_RHAI_REVIEW_REPO").unwrap();
    let id = ModuleId::parse("main").unwrap();
    let p = EmbeddedScriptView::new(
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
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
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

fn mount_stream(
    cx: &mut TestAppContext,
    src: &str,
    name: &str,
    data: NativeChartData,
    counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    let repo = std::env::var("GPUI_RHAI_REVIEW_REPO").unwrap();
    let id = ModuleId::parse("main").unwrap();
    let p = EmbeddedScriptView::new(
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
    .extension(StreamExtension { data, counter })
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
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
struct StreamExtension {
    data: NativeChartData,
    counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}
impl ScriptViewExtension for StreamExtension {
    fn configure_engine(&self, e: &mut RuntimeEngine) -> Result<(), String> {
        e.register_chart_series("counter", Counter(self.counter.clone()))
            .map_err(|e| e.to_string())
    }
    fn configure_runtime(&self, r: &mut UiRuntimeState) -> Result<(), String> {
        r.register_native_chart_data("stream", self.data.clone())
            .map_err(|e| e.to_string())
    }
}
struct Counter(std::sync::Arc<std::sync::atomic::AtomicUsize>);
impl HostChartSeries for Counter {
    fn layout(&self, _: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(vec![])
    }
}
fn rows(offset: i64) -> ChartDataset {
    let rows = (0..2)
        .map(|i| {
            BTreeMap::from([
                ("id".into(), UiValue::String(format!("r{i}"))),
                ("x".into(), UiValue::Integer(i)),
                ("y".into(), UiValue::Integer(i + offset)),
            ])
        })
        .collect::<Vec<_>>();
    ChartDataset::from_rows("main", &rows, Some("id".into()), ChartDataLimits::default()).unwrap()
}
#[gpui::test]
fn suspended_chart_does_not_layout_stream_updates(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let data = NativeChartData::new([rows(0)], ChartDataLimits::default()).unwrap();
    let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let src = r#"import "charts/chart" as chart;fn view(ctx){chart::Chart(#{key:"c",data:ctx.get_native_chart_data("stream"),spec:#{title:"Control",series:[#{key:"s",kind:"custom",renderer:"counter",encode:#{x:"x",y:"y"}}]}}).with_style(style().width(px(420)).height(px(300)))}"#;
    let (w, view) = mount_stream(cx, src, "suspend-stream", data.clone(), count.clone());
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    v.update(|window, cx| view.suspend(window, cx).unwrap());
    v.run_until_parked();
    let before = count.load(std::sync::atomic::Ordering::Relaxed);
    for i in 1..=3 {
        data.replace([rows(i)]).unwrap();
        v.run_until_parked();
    }
    let after = count.load(std::sync::atomic::Ordering::Relaxed);
    println!(
        "SUSPENDED_STREAM before={before} after={after} state={:?}",
        view.state()
    );
    assert_eq!(
        before, after,
        "suspended chart kept doing geometry work for each new revision"
    );
}
const WHEEL_SOURCE: &str = r#"import "charts/chart" as chart;
 fn state_schema(){#{fields:#{n:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}}}}}
 fn zoomed(ctx,p){ctx.set_state("n",p.viewport_revision);ctx.set_state("z",p.zoom);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("n")}|${ctx.get_state("z")}`),chart::Chart(#{key:"c",key_dimension:"id",zoom:ctx.get_state("z"),viewport_revision:ctx.get_state("n"),data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:"Control",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}"#;
fn position(v: &mut VisualTestContext, view: &ScriptViewHandle) -> gpui::Point<gpui::Pixels> {
    v.update(|_, cx| {
        let t = view.accessibility_snapshot(cx).unwrap();
        let b = t
            .find_by_role_and_name("figure", "Control")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual;
        point(px((b.x + 180.) as f32), px((b.y + 160.) as f32))
    })
}
#[gpui::test]
fn resumed_chart_accepts_new_mouse_wheel(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut results = Vec::new();
    for active_gesture in [false, true] {
        let (w, view) = mount(cx, WHEEL_SOURCE, &format!("resume-wheel-{active_gesture}"));
        let mut v = VisualTestContext::from_window(*w, cx);
        pump(cx, &mut v);
        if active_gesture {
            let pos = position(&mut v, &view);
            v.simulate_event(ScrollWheelEvent {
                position: pos,
                delta: ScrollDelta::Pixels(point(px(0.), px(40.))),
                touch_phase: gpui::TouchPhase::Started,
                ..Default::default()
            });
            v.run_until_parked();
        }
        v.update(|window, cx| view.suspend(window, cx).unwrap());
        v.run_until_parked();
        v.update(|_, cx| view.resume(cx).unwrap());
        pump(cx, &mut v);
        let pos = position(&mut v, &view);
        v.simulate_event(ScrollWheelEvent {
            position: pos,
            delta: ScrollDelta::Lines(point(0., 1.)),
            touch_phase: gpui::TouchPhase::Moved,
            ..Default::default()
        });
        pump(cx, &mut v);
        let result = status(&mut v, &view);
        println!(
            "RESUMED_WHEEL active_before_suspend={active_gesture} callbacks_zoom={result} state={:?}",
            view.state()
        );
        results.push(result);
    }
    assert!(
        results[0].starts_with("1|"),
        "control must commit wheel input after ordinary suspend/resume"
    );
    assert!(
        results[1].starts_with("1|"),
        "stale explicit gesture prevents ordinary wheel commit after resume"
    );
}
