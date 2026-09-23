use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use gpui::{
    Context, IntoElement, Modifiers, Render, ScrollDelta, ScrollWheelEvent, TestAppContext,
    VisualTestContext, Window, WindowHandle, point, px,
};
use gpui_rhai::*;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn source(path: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
}

fn mount(cx: &mut TestAppContext, script: &str, name: &str) -> (WindowHandle<Host>, ScriptViewHandle) {
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (entry, script.to_owned()),
            (
                ModuleId::parse("charts/chart").unwrap(),
                source("registry/charts/chart.rhai"),
            ),
            (
                ModuleId::parse("charts/bar_chart").unwrap(),
                source("registry/charts/bar_chart.rhai"),
            ),
            (
                ModuleId::parse("charts/line_chart").unwrap(),
                source("registry/charts/line_chart.rhai"),
            ),
            (
                ModuleId::parse("charts/pie_chart").unwrap(),
                source("registry/charts/pie_chart.rhai"),
            ),
            (
                ModuleId::parse("charts/map_chart").unwrap(),
                source("registry/charts/map_chart.rhai"),
            ),
        ])),
        source("registry/themes/default_dark.rhai"),
    )
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let name = name.to_owned();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&name, cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new(&name), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let view = captured.borrow().as_ref().unwrap().clone();
    (window, view)
}

fn pump(cx: &mut TestAppContext, visual: &mut VisualTestContext) {
    for _ in 0..8 {
        cx.background_executor.advance_clock(Duration::from_millis(20));
        visual.run_until_parked();
        cx.refresh().unwrap();
        visual.run_until_parked();
    }
}

fn status(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.role == "status")
            .unwrap()
            .name
            .clone()
    })
}

fn chart_bounds(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> GeometryBounds {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("figure", "Control")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    })
}

#[gpui::test]
fn chart_controlled_zoom_rejects_unacknowledged_preview(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
 import "charts/chart" as chart;
 fn state_schema(){#{fields:#{n:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},first:#{schema:#{type:"float"},"default":#{type:"float",value:0.0}},last:#{schema:#{type:"float"},"default":#{type:"float",value:0.0}}}}}
 fn zoomed(ctx,p){let n=ctx.get_state("n");if n==0{ctx.set_state("first",p.zoom);}ctx.set_state("last",p.zoom);ctx.set_state("n",p.viewport_revision);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("n")}|${ctx.get_state("first")}|${ctx.get_state("last")}`),chart::Chart(#{key:"c",key_dimension:"id",zoom:1.0,pan_x:0.0,pan_y:0.0,viewport_revision:ctx.get_state("n"),data:[#{id:"a",x:0,y:1},#{id:"b",x:1,y:2}],spec:#{title:"Control",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}
 "#;
    let (window, view) = mount(cx, script, "chart-controlled-zoom");
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    let bounds = chart_bounds(&mut visual, &view);
    for _ in 0..3 {
        visual.simulate_event(ScrollWheelEvent {
            position: point(px((bounds.x + 180.0) as f32), px((bounds.y + 160.0) as f32)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(40.0))),
            ..Default::default()
        });
        visual.simulate_event(ScrollWheelEvent {
            position: point(px((bounds.x + 180.0) as f32), px((bounds.y + 160.0) as f32)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(0.0))),
            touch_phase: gpui::TouchPhase::Ended,
            ..Default::default()
        });
        visual.run_until_parked();
    }
    let result = status(&mut visual, &view);
    let values = result.split('|').collect::<Vec<_>>();
    assert_eq!(values[0], "3");
    let first = values[1].parse::<f64>().unwrap();
    let last = values[2].parse::<f64>().unwrap();
    assert!(
        (first - (-0.1_f64).exp()).abs() < 0.00001,
        "wheel input was lost before proposal: {result}"
    );
    assert!((first - last).abs() < 0.00001, "{result}");
}

const CONTROLLED_ZOOM_SOURCE: &str = r#"
 import "charts/chart" as chart;
 fn state_schema(){#{fields:#{n:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}}}}}
 fn zoomed(ctx,p){ctx.set_state("n",p.viewport_revision);ctx.set_state("z",p.zoom);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("n")}|${ctx.get_state("z")}`),chart::Chart(#{key:"c",key_dimension:"id",zoom:ctx.get_state("z"),viewport_revision:ctx.get_state("n"),data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:"Control",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}
"#;

fn chart_position(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> gpui::Point<gpui::Pixels> {
    let bounds = chart_bounds(visual, view);
    point(
        px((bounds.x + 180.0) as f32),
        px((bounds.y + 160.0) as f32),
    )
}

fn click_named(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) {
    let bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name(role, name)
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    visual.simulate_click(
        point(
            px((bounds.x + bounds.width / 2.0) as f32),
            px((bounds.y + bounds.height / 2.0) as f32),
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();
}

#[gpui::test]
fn chart_phase_less_mouse_wheel_commits(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(cx, CONTROLLED_ZOOM_SOURCE, "chart-phase-less-wheel");
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    let position = chart_position(&mut visual, &view);
    visual.simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Lines(point(0.0, 1.0)),
        touch_phase: gpui::TouchPhase::Moved,
        ..Default::default()
    });
    pump(cx, &mut visual);
    assert!(!status(&mut visual, &view).starts_with("0|"));
}

#[gpui::test]
fn chart_trackpad_preview_survives_intermediate_draw(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(cx, CONTROLLED_ZOOM_SOURCE, "chart-framed-wheel");
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    let position = chart_position(&mut visual, &view);
    visual.simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Pixels(point(px(0.0), px(40.0))),
        touch_phase: gpui::TouchPhase::Started,
        ..Default::default()
    });
    pump(cx, &mut visual);
    assert!(
        status(&mut visual, &view).starts_with("0|"),
        "an explicitly started gesture committed before Ended"
    );
    visual.simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Pixels(point(px(0.0), px(0.0))),
        touch_phase: gpui::TouchPhase::Ended,
        ..Default::default()
    });
    pump(cx, &mut visual);
    let zoom = status(&mut visual, &view)
        .split('|')
        .nth(1)
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert!((zoom - (-0.1_f64).exp()).abs() < 0.00001);
}

#[gpui::test]
fn unrelated_host_render_does_not_acknowledge_a_viewport_proposal(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
 import "charts/chart" as chart;
 fn state_schema(){#{fields:#{n:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},last:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}}}}}
 fn zoomed(ctx,p){ctx.set_state("n",ctx.get_state("n")+1);ctx.set_state("last",p.zoom);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("n")}|${ctx.get_state("last")}`),chart::Chart(#{key:"c",key_dimension:"id",zoom:1.0,viewport_revision:0,data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:"Control",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}
 "#;
    let (window, view) = mount(cx, script, "chart-delayed-viewport-ack");
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    let position = chart_position(&mut visual, &view);
    for _ in 0..2 {
        visual.simulate_event(ScrollWheelEvent {
            position,
            delta: ScrollDelta::Lines(point(0.0, 1.0)),
            touch_phase: gpui::TouchPhase::Moved,
            ..Default::default()
        });
        pump(cx, &mut visual);
    }
    let result = status(&mut visual, &view);
    let values = result.split('|').collect::<Vec<_>>();
    assert_eq!(values[0], "2");
    let zoom = values[1].parse::<f64>().unwrap();
    assert!(
        (zoom - (-0.08_f64).exp()).abs() < 0.00001,
        "unrelated script state update reset the pending preview: {result}"
    );
}

#[gpui::test]
fn acknowledging_an_old_proposal_preserves_the_current_gesture(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"import "charts/chart" as chart;
 fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},rev:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},pending_rev:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},pending_z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}}}}}
 fn zoomed(ctx,p){ctx.set_state("count",ctx.get_state("count")+1);ctx.set_state("pending_rev",p.viewport_revision);ctx.set_state("pending_z",p.zoom);}
 fn acknowledge(ctx,p){ctx.set_state("rev",ctx.get_state("pending_rev"));ctx.set_state("z",ctx.get_state("pending_z"));}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("count")}|${ctx.get_state("pending_z")}`),text("Acknowledge").accessibility_role("button").accessibility_label("Acknowledge").on_click(Fn("acknowledge")).with_style(style().width(px(140)).height(px(32))),chart::Chart(#{key:"c",key_dimension:"id",zoom:ctx.get_state("z"),viewport_revision:ctx.get_state("rev"),data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:"Control",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}"#;
    let (window, view) = mount(cx, script, "chart-overlapping-ack");
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    let position = chart_position(&mut visual, &view);
    visual.simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Lines(point(0.0, 1.0)),
        touch_phase: gpui::TouchPhase::Moved,
        ..Default::default()
    });
    visual.run_until_parked();
    assert!(status(&mut visual, &view).starts_with("1|"));

    visual.simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Pixels(point(px(0.0), px(40.0))),
        touch_phase: gpui::TouchPhase::Started,
        ..Default::default()
    });
    visual.run_until_parked();
    click_named(&mut visual, &view, "button", "Acknowledge");
    visual.simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Pixels(point(px(0.0), px(0.0))),
        touch_phase: gpui::TouchPhase::Ended,
        ..Default::default()
    });
    pump(cx, &mut visual);
    let result = status(&mut visual, &view);
    assert_eq!(result.split('|').next(), Some("2"), "{result}");
    let proposed = result.split('|').nth(1).unwrap().parse::<f64>().unwrap();
    assert!((proposed - (-0.14_f64).exp()).abs() < 0.00001, "{result}");
}

#[gpui::test]
fn chart_brush_does_not_capture_legend_control(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    for brush in ["none", "xy"] {
        let script = format!(
            r#"import "charts/chart" as chart;
 fn state_schema(){{#{{fields:#{{n:#{{schema:#{{type:"integer"}},"default":#{{type:"integer",value:0}}}}}}}}}}
 fn legend(ctx,p){{ctx.set_state("n",ctx.get_state("n")+1);}}
 fn view(ctx){{column([text("Status").accessibility_role("status").accessibility_label(ctx.get_state("n").to_string()),chart::Chart(#{{key:"c",key_dimension:"id",data:[#{{id:"a",x:0,y:1}},#{{id:"b",x:1,y:2}}],spec:#{{title:"Control",brush:"{brush}",series:[#{{key:"s",kind:"scatter",encode:#{{x:"x",y:"y"}}}}]}},on_legend_change:Fn("legend")}}).with_style(style().width(px(420)).height(px(300)))])}}"#
        );
        let (window, view) = mount(cx, &script, &format!("chart-brush-{brush}"));
        let mut visual = VisualTestContext::from_window(*window, cx);
        pump(cx, &mut visual);
        let bounds = chart_bounds(&mut visual, &view);
        visual.simulate_click(
            point(px((bounds.x + 26.0) as f32), px((bounds.y + 48.0) as f32)),
            Modifiers::default(),
        );
        visual.run_until_parked();
        assert_eq!(status(&mut visual, &view), "1");
    }
}

#[gpui::test]
fn chart_semantics_expose_active_and_selected_datum(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"import "charts/chart" as chart; fn view(ctx){chart::Chart(#{key:"c",key_dimension:"id",selected_keys:["first"],data:[#{id:"first",x:0,y:1},#{id:"second",x:1,y:2}],spec:#{title:"Control",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y",name:"id"}}]}}).with_style(style().width(px(420)).height(px(300)))}"#;
    let (window, view) = mount(cx, script, "chart-semantics");
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    let bounds = chart_bounds(&mut visual, &view);
    visual.simulate_click(
        point(px((bounds.x + 180.0) as f32), px((bounds.y + 160.0) as f32)),
        Modifiers::default(),
    );
    visual.simulate_keystrokes("home");
    visual.run_until_parked();
    let description = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("figure", "Control")
            .next()
            .unwrap()
            .description
            .clone()
    });
    assert!(description.contains("first"), "{description}");
}

#[gpui::test]
fn chart_adapter_normalizes_full_spec_series_kind(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"import "charts/chart" as chart; fn view(ctx){chart::BarChart(#{key:"c",key_dimension:"id",data:[#{id:"a",x:"A",y:1}],spec:#{series:[#{key:"s",encode:#{x:"x",y:"y"}}]}}).with_style(style().width(px(400)).height(px(300)))}"#;
    let (_, view) = mount(cx, script, "chart-adapter");
    let root = cx.update(|app| view.root(app).unwrap().unwrap());
    let UiNodeKind::Custom { primitive } = root.kind() else {
        panic!("chart must be native")
    };
    let Some(PrimitiveValue::Data(spec)) = primitive.props.get("spec") else {
        panic!("missing spec")
    };
    assert_eq!(ChartSpec::from_ui_value(spec).unwrap().series[0].kind, ChartSeriesKind::Bar);
}

#[gpui::test]
fn formal_chart_adapters_forward_the_viewport_revision(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    for (name, script) in [
        (
            "bar",
            r#"import "charts/bar_chart" as adapter;fn view(ctx){adapter::BarChart(#{key:"c",key_dimension:"id",viewport_revision:7,data:[#{id:"a",x:"A",y:1}],encode:#{x:"x",y:"y"}})}"#,
        ),
        (
            "line",
            r#"import "charts/line_chart" as adapter;fn view(ctx){adapter::LineChart(#{key:"c",key_dimension:"id",viewport_revision:7,data:[#{id:"a",x:"A",y:1}],encode:#{x:"x",y:"y"}})}"#,
        ),
        (
            "pie",
            r#"import "charts/pie_chart" as adapter;fn view(ctx){adapter::PieChart(#{key:"c",key_dimension:"id",viewport_revision:7,data:[#{id:"a",name:"A",value:1}],encode:#{name:"name",value:"value"}})}"#,
        ),
        (
            "map",
            r#"import "charts/map_chart" as adapter;fn view(ctx){adapter::MapChart(#{key:"c",key_dimension:"id",viewport_revision:7,map:"world",data:[#{id:"a",name:"A",value:1}],encode:#{name:"name",value:"value"}})}"#,
        ),
    ] {
        let (_, view) = mount(cx, script, &format!("formal-{name}-viewport"));
        let root = cx.update(|app| view.root(app).unwrap().unwrap());
        let UiNodeKind::Custom { primitive } = root.kind() else {
            panic!("formal {name} adapter must produce the chart primitive")
        };
        assert!(matches!(
            primitive.props.get("viewport_revision"),
            Some(PrimitiveValue::Data(UiValue::Integer(7)))
        ));
    }
}

#[gpui::test]
fn chart_business_key_cannot_become_a_control_role(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    for key in ["normal", "legend"] {
        let script = r#"import "charts/chart" as chart;
 fn state_schema(){#{fields:#{s:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},l:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
 fn select(ctx,p){ctx.set_state("s",ctx.get_state("s")+1);} fn legend(ctx,p){ctx.set_state("l",ctx.get_state("l")+1);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("s")}|${ctx.get_state("l")}`),chart::Chart(#{key:"c",key_dimension:"id",data:[#{id:"DATUM",x:"A",y:1}],spec:#{title:"Control",legend:#{visible:false},series:[#{key:"s",kind:"bar",encode:#{x:"x",y:"y"}}]},on_select:Fn("select"),on_legend_change:Fn("legend")}).with_style(style().width(px(420)).height(px(300)))])}"#.replace("DATUM", key);
        let (window, view) = mount(cx, &script, &format!("chart-key-{key}"));
        let mut visual = VisualTestContext::from_window(*window, cx);
        pump(cx, &mut visual);
        let bounds = chart_bounds(&mut visual, &view);
        visual.simulate_click(
            point(px((bounds.x + 200.0) as f32), px((bounds.y + 150.0) as f32)),
            Modifiers::default(),
        );
        visual.run_until_parked();
        assert_eq!(status(&mut visual, &view), "1|0");
    }
}

#[gpui::test]
fn chart_gauge_activation_returns_the_source_row_key(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"import "charts/chart" as chart;
 fn state_schema(){#{fields:#{last:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}}}}
 fn selected(ctx,p){ctx.set_state("last",p.datum_key);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(ctx.get_state("last")),chart::Chart(#{key:"c",key_dimension:"id",data:[#{id:"measurement-1",v:25}],spec:#{title:"Control",regions:[#{key:"main",kind:"polar"}],axes:[#{key:"r",position:"radial",min:0,max:100}],series:[#{key:"g",kind:"gauge",encode:#{value:"v",name:"id"}}]},on_select:Fn("selected")}).with_style(style().width(px(420)).height(px(300)))])}"#;
    let (window, view) = mount(cx, script, "chart-gauge-key");
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    let position = chart_position(&mut visual, &view);
    visual.simulate_click(position, Modifiers::default());
    visual.simulate_keystrokes("home");
    visual.simulate_keystrokes("enter");
    visual.run_until_parked();
    assert_eq!(status(&mut visual, &view), "measurement-1");
}

struct CountExtension(Arc<AtomicUsize>);

impl ScriptViewExtension for CountExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine
            .register_chart_series("count", CountRenderer(self.0.clone()))
            .map_err(|error| error.to_string())
    }
}

struct CountRenderer(Arc<AtomicUsize>);

impl HostChartSeries for CountRenderer {
    fn layout(&self, _: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
        let calls = self.0.fetch_add(1, Ordering::Relaxed) + 1;
        assert!(calls < 64, "linked charts entered a repeated layout loop");
        Ok(Vec::new())
    }
}

fn mount_with_layout_counter(
    cx: &mut TestAppContext,
    script: &str,
    name: &str,
    counter: Arc<AtomicUsize>,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (entry, script.to_owned()),
            (
                ModuleId::parse("charts/chart").unwrap(),
                source("registry/charts/chart.rhai"),
            ),
        ])),
        source("registry/themes/default_dark.rhai"),
    )
    .extension(CountExtension(counter))
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let name = name.to_owned();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&name, cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new(&name), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    let view = captured.borrow().as_ref().unwrap().clone();
    (window, view)
}

#[gpui::test]
fn idle_linked_charts_do_not_repeat_layout(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    for linked in [false, true] {
        let link = if linked {
            r#",link_group:"shared",link_domain:"time""#
        } else {
            ""
        };
        let script = r#"import "charts/chart" as chart;
 fn one(k){chart::Chart(#{key:k,key_dimension:"id",data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:kLINK,series:[#{key:"s",kind:"custom",renderer:"count",encode:#{x:"x",y:"y"}}]}}).with_style(style().width(px(300)).height(px(220)))}
 fn view(ctx){row([one("left"),one("right")])}"#
            .replace("kLINK", &format!("k{link}"));
        let counter = Arc::new(AtomicUsize::new(0));
        let (window, _) = mount_with_layout_counter(
            cx,
            &script,
            &format!("chart-idle-link-{linked}"),
            counter.clone(),
        );
        let mut visual = VisualTestContext::from_window(*window, cx);
        pump(cx, &mut visual);
        let before = counter.load(Ordering::Relaxed);
        pump(cx, &mut visual);
        assert_eq!(counter.load(Ordering::Relaxed), before);
    }
}

#[gpui::test]
fn linked_selection_keeps_each_chart_source_instead_of_last_writer_wins(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_rhai::install);
    let script = r#"import "charts/chart" as chart;
 fn one(k,selected){chart::Chart(#{key:k,key_dimension:"id",selected_keys:[selected],data:[#{id:"a",x:0,y:1},#{id:"b",x:1,y:2},#{id:"c",x:2,y:3}],spec:#{title:k,link_group:"shared",link_domain:"time",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y",name:"id"}}]}}).with_style(style().width(px(280)).height(px(220)))}
 fn view(ctx){row([one("left","a"),one("middle","b"),one("right","c")])}"#;
    let (window, view) = mount(cx, script, "chart-linked-selection-sources");
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    visual.update(|_, cx| {
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        for title in ["left", "middle", "right"] {
            let description = &snapshot
                .find_by_role_and_name("figure", title)
                .next()
                .unwrap()
                .description;
            for key in ["a", "b", "c"] {
                assert!(description.contains(key), "{title}: {description}");
            }
        }
    });
}
