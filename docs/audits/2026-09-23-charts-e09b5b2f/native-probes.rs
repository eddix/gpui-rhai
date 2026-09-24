use gpui::{
    Context, IntoElement, Modifiers, Render, ScrollDelta, ScrollWheelEvent, TestAppContext,
    VisualTestContext, Window, WindowHandle, point, px,
};
use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Duration};
struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
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
        Host { host, view }
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
#[gpui::test]
fn controlled_zoom_does_not_drift_when_host_keeps_value(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let src = r#"
 import "charts/chart" as chart;
 fn state_schema(){#{fields:#{n:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},first:#{schema:#{type:"float"},"default":#{type:"float",value:0.0}},last:#{schema:#{type:"float"},"default":#{type:"float",value:0.0}}}}}
 fn zoomed(ctx,p){let n=ctx.get_state("n");if n==0{ctx.set_state("first",p.zoom);}ctx.set_state("last",p.zoom);ctx.set_state("n",n+1);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("n")}|${ctx.get_state("first")}|${ctx.get_state("last")}`),chart::Chart(#{key:"c",key_dimension:"id",zoom:1.0,pan_x:0.0,pan_y:0.0,data:[#{id:"a",x:0,y:1},#{id:"b",x:1,y:2}],spec:#{title:"Control",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}
 "#;
    let (w, view) = mount(cx, src, "zoom");
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
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
    for _ in 0..3 {
        v.simulate_event(ScrollWheelEvent {
            position: point(px((b.x + 180.) as f32), px((b.y + 160.) as f32)),
            delta: ScrollDelta::Pixels(point(px(0.), px(40.))),
            ..Default::default()
        });
        v.simulate_event(ScrollWheelEvent {
            position: point(px((b.x + 180.) as f32), px((b.y + 160.) as f32)),
            delta: ScrollDelta::Pixels(point(px(0.), px(0.))),
            touch_phase: gpui::TouchPhase::Ended,
            ..Default::default()
        });
        v.run_until_parked();
    }
    let result = status(&mut v, &view);
    println!("controlled zoom=1, wheel proposals={result}");
    let p = result.split('|').collect::<Vec<_>>();
    let first = p[1].parse::<f64>().unwrap();
    let last = p[2].parse::<f64>().unwrap();
    assert_eq!(p[0], "3", "fixture must receive events");
    assert!(
        (first - last).abs() < 0.00001,
        "controlled zoom ignored unchanged model value and compounded local state"
    );
}
#[gpui::test]
fn brush_does_not_disable_legend_controls(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut counts = vec![];
    for brush in ["none", "xy"] {
        let src = format!(
            r#"
 import "charts/chart" as chart;
 fn state_schema(){{#{{fields:#{{n:#{{schema:#{{type:"integer"}},"default":#{{type:"integer",value:0}}}}}}}}}}
 fn legend(ctx,p){{ctx.set_state("n",ctx.get_state("n")+1);}}
 fn view(ctx){{column([text("Status").accessibility_role("status").accessibility_label(ctx.get_state("n").to_string()),chart::Chart(#{{key:"c",key_dimension:"id",data:[#{{id:"a",x:0,y:1}},#{{id:"b",x:1,y:2}}],spec:#{{title:"Control",brush:"{brush}",series:[#{{key:"s",kind:"scatter",encode:#{{x:"x",y:"y"}}}}]}},on_legend_change:Fn("legend")}}).with_style(style().width(px(420)).height(px(300)))])}}
 "#
        );
        let (w, view) = mount(cx, &src, &format!("brush-{brush}"));
        let mut v = VisualTestContext::from_window(*w, cx);
        pump(cx, &mut v);
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
            point(px((b.x + 26.) as f32), px((b.y + 48.) as f32)),
            Modifiers::default(),
        );
        v.run_until_parked();
        let n = status(&mut v, &view);
        println!("brush={brush} legend_callbacks={n}");
        counts.push(n);
    }
    assert_eq!(counts[0], "1");
    assert_eq!(
        counts[1], "1",
        "enabling brush swallowed legend click outside plot"
    );
}
#[gpui::test]
fn focused_chart_data_is_exposed_to_semantics(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let src = r#"import "charts/chart" as chart; fn view(ctx){chart::Chart(#{key:"c",key_dimension:"id",selected_keys:["first"],data:[#{id:"first",x:0,y:1},#{id:"second",x:1,y:2}],spec:#{title:"Control",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y",name:"id"}}]}}).with_style(style().width(px(420)).height(px(300)))}"#;
    let (w, view) = mount(cx, src, "semantics");
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
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
        point(px((b.x + 180.) as f32), px((b.y + 160.) as f32)),
        Modifiers::default(),
    );
    v.simulate_keystrokes("home");
    v.run_until_parked();
    let nodes = v.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .map(|n| {
                (
                    n.role.clone(),
                    n.name.clone(),
                    n.description.clone(),
                    n.value.clone(),
                )
            })
            .collect::<Vec<_>>()
    });
    println!("chart semantic nodes after keyboard focus={nodes:?}");
    assert!(
        nodes
            .iter()
            .any(|(_, n, d, _)| n.contains("first") || d.contains("first")),
        "focused/selected datum is absent from the public semantic tree"
    );
}
#[gpui::test]
fn specialized_adapter_populates_series_kind(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let src = r#"import "charts/chart" as chart; fn view(ctx){chart::BarChart(#{key:"c",key_dimension:"id",data:[#{id:"a",x:"A",y:1}],spec:#{series:[#{key:"s",encode:#{x:"x",y:"y"}}]}}).with_style(style().width(px(400)).height(px(300)))}"#;
    let (w, view) = mount(cx, src, "adapter");
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
    let UiNodeKind::Custom { primitive } = root.kind() else {
        panic!("chart must use primitive")
    };
    let Some(PrimitiveValue::Data(spec)) = primitive.props.get("spec") else {
        panic!()
    };
    let parsed = ChartSpec::from_ui_value(spec);
    println!("BarChart(spec.series without kind) parsed={parsed:?}");
    assert!(
        parsed.is_ok(),
        "adapter failed to fill kind in supplied spec"
    );
    assert_eq!(parsed.unwrap().series[0].kind, ChartSeriesKind::Bar);
}
#[gpui::test]
fn datum_key_cannot_turn_data_into_a_legend_control(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut results = vec![];
    for key in ["normal", "legend"] {
        let src=r#"import "charts/chart" as chart;
 fn state_schema(){#{fields:#{s:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},l:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
 fn select(ctx,p){ctx.set_state("s",ctx.get_state("s")+1);}fn legend(ctx,p){ctx.set_state("l",ctx.get_state("l")+1);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("s")}|${ctx.get_state("l")}`),chart::Chart(#{key:"c",key_dimension:"id",data:[#{id:"DATUM_KEY",x:"A",y:1}],spec:#{title:"Control",legend:#{visible:false},series:[#{key:"s",kind:"bar",encode:#{x:"x",y:"y"}}]},on_select:Fn("select"),on_legend_change:Fn("legend")}).with_style(style().width(px(420)).height(px(300)))])}"#.replace("DATUM_KEY",key);
        let (w, view) = mount(cx, &src, &format!("key-{key}"));
        let mut v = VisualTestContext::from_window(*w, cx);
        pump(cx, &mut v);
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
            point(px((b.x + 200.) as f32), px((b.y + 150.) as f32)),
            Modifiers::default(),
        );
        v.run_until_parked();
        let out = status(&mut v, &view);
        println!("data key={key}: select|legend={out}");
        results.push(out);
    }
    assert_eq!(results[0], "1|0");
    assert_eq!(
        results[1], "1|0",
        "a valid datum key was interpreted as a UI control tag"
    );
}
