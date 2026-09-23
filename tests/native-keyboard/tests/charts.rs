use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
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
 fn zoomed(ctx,p){let n=ctx.get_state("n");if n==0{ctx.set_state("first",p.zoom);}ctx.set_state("last",p.zoom);ctx.set_state("n",n+1);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("n")}|${ctx.get_state("first")}|${ctx.get_state("last")}`),chart::Chart(#{key:"c",key_dimension:"id",zoom:1.0,pan_x:0.0,pan_y:0.0,data:[#{id:"a",x:0,y:1},#{id:"b",x:1,y:2}],spec:#{title:"Control",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}
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
    assert!((first - last).abs() < 0.00001, "{result}");
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
