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
#[gpui::test]
fn wheel_proposal_must_contain_the_requested_zoom(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let src = r#"
 import "charts/chart" as chart;
 fn state_schema(){#{fields:#{n:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},first:#{schema:#{type:"float"},"default":#{type:"float",value:0.0}},last:#{schema:#{type:"float"},"default":#{type:"float",value:0.0}}}}}
 fn zoomed(ctx,p){let n=ctx.get_state("n");if n==0{ctx.set_state("first",p.zoom);}ctx.set_state("last",p.zoom);ctx.set_state("n",n+1);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("n")}|${ctx.get_state("first")}|${ctx.get_state("last")}`),chart::Chart(#{key:"c",key_dimension:"id",zoom:1.0,pan_x:0.0,pan_y:0.0,viewport_revision:ctx.get_state("n"),data:[#{id:"a",x:0,y:1},#{id:"b",x:1,y:2}],spec:#{title:"Control",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}
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
        (first - (-0.1_f64).exp()).abs() < 0.00001,
        "wheel input was lost before proposal: {result}"
    );
    assert!(
        (first - last).abs() < 0.00001,
        "controlled zoom ignored unchanged model value and compounded local state"
    );
}
const ZOOM_SOURCE: &str = r#"
 import "charts/chart" as chart;
 fn state_schema(){#{fields:#{n:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}}}}}
 fn zoomed(ctx,p){ctx.set_state("n",ctx.get_state("n")+1);ctx.set_state("z",p.zoom);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("n")}|${ctx.get_state("z")}`),chart::Chart(#{key:"c",key_dimension:"id",zoom:ctx.get_state("z"),viewport_revision:ctx.get_state("n"),data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:"Control",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}
"#;
fn chart_pos(v: &mut VisualTestContext, view: &ScriptViewHandle) -> gpui::Point<gpui::Pixels> {
    v.update(|_, cx| {
        let b = view
            .accessibility_snapshot(cx)
            .unwrap()
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
fn phase_less_mouse_wheel_commits(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (w, view) = mount(cx, ZOOM_SOURCE, "phase-less");
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let pos = chart_pos(&mut v, &view);
    v.simulate_event(ScrollWheelEvent {
        position: pos,
        delta: ScrollDelta::Lines(point(0., 1.)),
        touch_phase: gpui::TouchPhase::Moved,
        ..Default::default()
    });
    for _ in 0..3 {
        pump(cx, &mut v);
    }
    let s = status(&mut v, &view);
    println!("PHASELESS_WHEEL callbacks|zoom={s}");
    assert!(
        !s.starts_with("0|"),
        "ordinary phase-less wheel never commits zoom"
    );
}
#[gpui::test]
fn framed_gesture_survives_preview_draw(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (w, view) = mount(cx, ZOOM_SOURCE, "framed");
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let pos = chart_pos(&mut v, &view);
    v.simulate_event(ScrollWheelEvent {
        position: pos,
        delta: ScrollDelta::Pixels(point(px(0.), px(40.))),
        touch_phase: gpui::TouchPhase::Started,
        ..Default::default()
    });
    pump(cx, &mut v);
    v.simulate_event(ScrollWheelEvent {
        position: pos,
        delta: ScrollDelta::Pixels(point(px(0.), px(0.))),
        touch_phase: gpui::TouchPhase::Ended,
        ..Default::default()
    });
    pump(cx, &mut v);
    let s = status(&mut v, &view);
    println!("FRAMED_GESTURE callbacks|zoom={s}");
    let z = s.split('|').nth(1).unwrap().parse::<f64>().unwrap();
    assert!(
        (z - (-0.1_f64).exp()).abs() < 0.00001,
        "native preview was reset by drawing before commit"
    );
}

fn mount_counter(
    cx: &mut TestAppContext,
    src: &str,
    name: &str,
    count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
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
    .extension(CountExtension(count))
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
struct CountExtension(std::sync::Arc<std::sync::atomic::AtomicUsize>);
impl ScriptViewExtension for CountExtension {
    fn configure_engine(&self, e: &mut RuntimeEngine) -> Result<(), String> {
        e.register_chart_series("count", CountRenderer(self.0.clone()))
            .map_err(|e| e.to_string())
    }
}
struct CountRenderer(std::sync::Arc<std::sync::atomic::AtomicUsize>);
impl HostChartSeries for CountRenderer {
    fn layout(&self, _: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
        let calls = self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        if calls >= 64 {
            eprintln!(
                "PROBE_GUARD: linked idle charts reached {calls} custom layout calls without parking; stopping this test process"
            );
            std::process::exit(70);
        }
        Ok(vec![])
    }
}
#[gpui::test]
fn idle_linked_charts_do_not_repeat_layout(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let mut deltas = vec![];
    for link in [false, true] {
        let linktext = if link {
            ",link_group:\"shared\",link_domain:\"time\""
        } else {
            ""
        };
        let script=r#"import "charts/chart" as chart;
 fn one(k){chart::Chart(#{key:k,key_dimension:"id",data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:kLINK,series:[#{key:"s",kind:"custom",renderer:"count",encode:#{x:"x",y:"y"}}]}}).with_style(style().width(px(300)).height(px(220)))}
 fn view(ctx){row([one("left"),one("right")])}"#.replace("kLINK",&format!("k{linktext}"));
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (w, _view) = mount_counter(cx, &script, &format!("idle-{link}"), counter.clone());
        let mut v = VisualTestContext::from_window(*w, cx);
        pump(cx, &mut v);
        let before = counter.load(std::sync::atomic::Ordering::Relaxed);
        pump(cx, &mut v);
        let after = counter.load(std::sync::atomic::Ordering::Relaxed);
        println!(
            "IDLE_LINK link={link} layouts_before={before} after={after} delta={}",
            after - before
        );
        deltas.push(after - before);
    }
    assert_eq!(
        deltas[0], 0,
        "control should not relayout on unchanged redraw"
    );
    assert_eq!(
        deltas[1], 0,
        "linked charts continuously relayout without data, selection or viewport changes"
    );
}
#[gpui::test]
fn gauge_activation_returns_business_key(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let src = r#"import "charts/chart" as chart;
 fn state_schema(){#{fields:#{last:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}}}}
 fn selected(ctx,p){ctx.set_state("last",p.datum_key);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(ctx.get_state("last")),chart::Chart(#{key:"c",key_dimension:"id",data:[#{id:"measurement-1",v:25}],spec:#{title:"Control",regions:[#{key:"main",kind:"polar"}],axes:[#{key:"r",position:"radial",min:0,max:100}],series:[#{key:"g",kind:"gauge",encode:#{value:"v",name:"id"}}]},on_select:Fn("selected")}).with_style(style().width(px(420)).height(px(300)))])}"#;
    let (w, view) = mount(cx, src, "gauge-key");
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let pos = chart_pos(&mut v, &view);
    v.simulate_click(pos, gpui::Modifiers::default());
    v.simulate_keystrokes("home");
    v.simulate_keystrokes("enter");
    v.run_until_parked();
    let result = status(&mut v, &view);
    println!("GAUGE_CALLBACK_KEY={result}");
    assert_eq!(
        result, "measurement-1",
        "activation must identify the source row"
    );
}
fn click_named(v: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) {
    let p = v.update(|_, cx| {
        let t = view.accessibility_snapshot(cx).unwrap();
        let b = t
            .find_by_role_and_name("button", name)
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual;
        point(
            px((b.x + b.width / 2.) as f32),
            px((b.y + b.height / 2.) as f32),
        )
    });
    v.simulate_click(p, gpui::Modifiers::default());
    v.run_until_parked();
}
#[gpui::test]
fn formal_bar_adapter_accepts_viewport_revision(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let src = r#"import "charts/bar_chart" as bar;fn view(ctx){bar::BarChart(#{key:"c",key_dimension:"id",viewport_revision:1,zoom:1.0,data:[#{id:"a",x:"A",y:1}],encode:#{x:"x",y:"y"}})}"#;
    let (w, view) = mount(cx, src, "formal-bar-revision");
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let root = v.update(|_, cx| view.root(cx).unwrap());
    assert!(
        root.is_some(),
        "formal adapter should accept documented viewport protocol"
    );
    let root = root.unwrap();
    let UiNodeKind::Custom { primitive } = root.kind() else {
        panic!("formal adapter must produce the chart primitive");
    };
    assert!(matches!(
        primitive.props.get("viewport_revision"),
        Some(PrimitiveValue::Data(UiValue::Integer(1)))
    ));
}
#[gpui::test]
fn previous_ack_does_not_erase_current_gesture(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let src = r#"import "charts/chart" as chart;
 fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},rev:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},pending_rev:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},pending_z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}}}}}
 fn zoomed(ctx,p){ctx.set_state("count",ctx.get_state("count")+1);ctx.set_state("pending_rev",p.viewport_revision);ctx.set_state("pending_z",p.zoom);}
 fn acknowledge(ctx,p){ctx.set_state("rev",ctx.get_state("pending_rev"));ctx.set_state("z",ctx.get_state("pending_z"));}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("count")}|${ctx.get_state("pending_z")}`),text("Acknowledge").accessibility_role("button").accessibility_label("Acknowledge").on_click(Fn("acknowledge")).with_style(style().width(px(140)).height(px(32))),chart::Chart(#{key:"c",key_dimension:"id",zoom:ctx.get_state("z"),viewport_revision:ctx.get_state("rev"),data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:"Control",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}"#;
    let (w, view) = mount(cx, src, "delayed-overlap");
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let pos = chart_pos(&mut v, &view);
    v.simulate_event(ScrollWheelEvent {
        position: pos,
        delta: ScrollDelta::Lines(point(0., 1.)),
        touch_phase: gpui::TouchPhase::Moved,
        ..Default::default()
    });
    v.run_until_parked();
    let first = status(&mut v, &view);
    assert!(first.starts_with("1|"));
    v.simulate_event(ScrollWheelEvent {
        position: pos,
        delta: ScrollDelta::Pixels(point(px(0.), px(40.))),
        touch_phase: gpui::TouchPhase::Started,
        ..Default::default()
    });
    v.run_until_parked();
    click_named(&mut v, &view, "Acknowledge");
    v.simulate_event(ScrollWheelEvent {
        position: pos,
        delta: ScrollDelta::Pixels(point(px(0.), px(0.))),
        touch_phase: gpui::TouchPhase::Ended,
        ..Default::default()
    });
    pump(cx, &mut v);
    let result = status(&mut v, &view);
    println!("OVERLAPPING_ACK first={first} final={result}");
    assert_eq!(
        result.split('|').next().unwrap(),
        "2",
        "ack of proposal 1 erased the not-yet-committed second gesture"
    );
    let proposed = result.split('|').nth(1).unwrap().parse::<f64>().unwrap();
    assert!(
        (proposed - (-0.14_f64).exp()).abs() < 0.00001,
        "the second proposal must include both accepted and newly previewed deltas"
    );
}
#[gpui::test]
fn linked_source_unmount_retracts_highlight(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let src = r#"import "charts/chart" as chart;
 fn state_schema(){#{fields:#{show:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}}}}}
 fn remove(ctx,p){ctx.set_state("show",false);}
 fn one(k,keys){chart::Chart(#{key:k,key_dimension:"id",selected_keys:keys,data:[#{id:"ALPHA_SELECTED_DATUM",x:0,y:1},#{id:"other",x:1,y:2}],spec:#{title:k,link_group:"g",link_domain:"time",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y",name:"id"}}]}}).with_style(style().width(px(280)).height(px(220)))}
 fn view(ctx){let charts=[];if ctx.get_state("show"){charts.push(one("source",["ALPHA_SELECTED_DATUM"]));}charts.push(one("target",[]));column([text("Remove").accessibility_role("button").accessibility_label("Remove").on_click(Fn("remove")).with_style(style().width(px(120)).height(px(32))),row(charts)])}"#;
    let (w, view) = mount(cx, src, "linked-remove");
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let get = |v: &mut VisualTestContext| {
        v.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("figure", "target")
                .next()
                .unwrap()
                .description
                .clone()
        })
    };
    let before = get(&mut v);
    assert!(before.contains("ALPHA_SELECTED_DATUM"));
    click_named(&mut v, &view, "Remove");
    v.run_until_parked();
    let after = get(&mut v);
    println!("LINKED_UNMOUNT before={before:?} after={after:?}");
    assert!(
        !after.contains("ALPHA_SELECTED_DATUM"),
        "unmounted source leaves stale linked selection"
    );
}
#[gpui::test]
fn started_gesture_waits_for_ended(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (w, view) = mount(cx, ZOOM_SOURCE, "phased-pause");
    let mut v = VisualTestContext::from_window(*w, cx);
    pump(cx, &mut v);
    let pos = chart_pos(&mut v, &view);
    v.simulate_event(ScrollWheelEvent {
        position: pos,
        delta: ScrollDelta::Pixels(point(px(0.), px(40.))),
        touch_phase: gpui::TouchPhase::Started,
        ..Default::default()
    });
    pump(cx, &mut v);
    let before_end = status(&mut v, &view);
    println!("PHASED_GESTURE_BEFORE_END={before_end}");
    assert!(
        before_end.starts_with("0|"),
        "explicitly active gesture committed before Ended during a brief pause"
    );
}
