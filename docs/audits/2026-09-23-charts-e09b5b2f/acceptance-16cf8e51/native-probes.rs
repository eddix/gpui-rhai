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
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("n")}|${ctx.get_state("z")}`),chart::Chart(#{key:"c",key_dimension:"id",zoom:ctx.get_state("z"),data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:"Control",series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}
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
