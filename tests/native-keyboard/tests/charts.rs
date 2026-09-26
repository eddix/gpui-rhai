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
use gpui_rhai_cli::gallery::{GalleryLaunch, prepare as prepare_gallery};

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let element = if self.view.state() == ScriptViewState::Active {
            self.view.element().unwrap()
        } else {
            gpui::div().into_any_element()
        };
        self.host.container(element)
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

fn mount(
    cx: &mut TestAppContext,
    script: &str,
    name: &str,
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

fn mount_prepared(
    cx: &mut TestAppContext,
    prepared: PreparedScriptView,
    name: &str,
) -> (WindowHandle<Host>, ScriptViewHandle) {
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
        cx.background_executor
            .advance_clock(Duration::from_millis(20));
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

fn chart_mark_count(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> i64 {
    visual.update(|_, cx| {
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        let node = snapshot
            .find_by_role_and_name("figure", "Control")
            .next()
            .unwrap();
        let Some(UiValue::Map(values)) = &node.value else {
            panic!("missing chart projection")
        };
        let UiValue::Integer(count) = values["mark_count"] else {
            panic!("missing presented mark count")
        };
        count
    })
}

#[gpui::test]
fn chart_catalog_story_uses_a_window_bounded_scroll_viewport(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let prepared = prepare_gallery(&GalleryLaunch {
        story: "charts/catalog".to_owned(),
        ..GalleryLaunch::default()
    })
    .unwrap();
    let (window, view) = mount_prepared(cx, prepared, "chart-gallery-scroll");
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    let figure_y = |visual: &mut VisualTestContext| {
        visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("figure", "pie")
                .next()
                .unwrap()
                .geometry
                .unwrap()
                .visual
                .y
        })
    };
    let before = figure_y(&mut visual);
    for _ in 0..12 {
        visual.simulate_event(ScrollWheelEvent {
            position: point(px(4.0), px(400.0)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-700.0))),
            ..ScrollWheelEvent::default()
        });
    }
    pump(cx, &mut visual);
    let after = figure_y(&mut visual);
    assert!(after < before - 300.0, "before={before}, after={after}");
}

#[gpui::test]
fn plain_wheel_over_chart_bubbles_to_scroll_container(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
        import "charts/chart" as chart;
        fn state_schema(){#{fields:#{n:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}}}}}
        fn zoomed(ctx,p){ctx.set_state("n",p.viewport_revision);ctx.set_state("z",p.zoom);}
        fn view(ctx){column([
            box([]).with_style(style().height(px(40)).flex_shrink(false)),
            chart::Chart(#{key:"c",key_dimension:"id",zoom:ctx.get_state("z"),viewport_revision:ctx.get_state("n"),data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:"Control",series:[#{key:"s",kind:"line",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(240)).flex_shrink(false)),
            text(`Status ${ctx.get_state("n")}|${ctx.get_state("z")}`).accessibility_role("status").accessibility_label(`${ctx.get_state("n")}|${ctx.get_state("z")}`),
            box([]).with_style(style().height(px(500)).flex_shrink(false))
        ]).with_style(style().width(px(460)).height(px(300)).overflow_y_scroll())}
    "#;
    let (window, view) = mount(cx, script, "chart-wheel-bubbles");
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    let before = chart_bounds(&mut visual, &view);
    visual.simulate_event(ScrollWheelEvent {
        position: point(
            px((before.x + before.width / 2.0) as f32),
            px((before.y + before.height / 2.0) as f32),
        ),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-160.0))),
        ..Default::default()
    });
    pump(cx, &mut visual);
    let after = chart_bounds(&mut visual, &view);
    assert!(
        after.y < before.y - 40.0,
        "before={before:?}, after={after:?}"
    );
    assert!(status(&mut visual, &view).starts_with("0|"));
}

#[gpui::test]
fn relative_height_chart_presents_when_parent_width_comes_from_stretch(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
        import "charts/chart" as chart;
        fn view(ctx) {
            let rows = [#{ id: "a", k: "A", v: 3 }, #{ id: "b", k: "B", v: 5 }];
            let spec = #{ title: "Control", series: [#{ key: "s", kind: "bar",
                encode: #{ x: "k", y: "v" } }] };
            let graph = chart::Chart(#{ key: "stretch-chart", key_dimension: "id",
                data: rows, spec: spec })
                .with_style(style().width(relative(1.0)).height(relative(1.0)));
            let card = box([text("card")]).with_style(style().flex_basis(relative(0.5)));
            let chart_box = box([graph]).with_style(
                style().width(px(180)).height(relative(1.0)).flex_shrink(false));
            let content = row([card, chart_box]).with_style(
                style().flex_grow().min_height(px(0)));
            column([content]).with_style(
                style().width(relative(1.0)).height(px(300)).flex_col())
        }
    "#;
    let (window, view) = mount(cx, script, "chart-stretch-parent");
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    assert!(chart_mark_count(&mut visual, &view) > 0);
    let bounds = chart_bounds(&mut visual, &view);
    assert!((bounds.width - 180.0).abs() < 1.0, "{bounds:?}");
    assert!((bounds.height - 300.0).abs() < 1.0, "{bounds:?}");
}

#[gpui::test]
fn chart_controlled_zoom_rejects_unacknowledged_preview(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
 import "charts/chart" as chart;
 fn state_schema(){#{fields:#{n:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},first:#{schema:#{type:"float"},"default":#{type:"float",value:0.0}},last:#{schema:#{type:"float"},"default":#{type:"float",value:0.0}}}}}
 fn zoomed(ctx,p){let n=ctx.get_state("n");if n==0{ctx.set_state("first",p.zoom);}ctx.set_state("last",p.zoom);ctx.set_state("n",p.viewport_revision);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("n")}|${ctx.get_state("first")}|${ctx.get_state("last")}`),chart::Chart(#{key:"c",key_dimension:"id",zoom:1.0,pan_x:0.0,pan_y:0.0,viewport_revision:ctx.get_state("n"),data:[#{id:"a",x:0,y:1},#{id:"b",x:1,y:2}],spec:#{title:"Control",interaction:#{wheel_zoom:"always"},series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}
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
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("n")}|${ctx.get_state("z")}`),chart::Chart(#{key:"c",key_dimension:"id",zoom:ctx.get_state("z"),viewport_revision:ctx.get_state("n"),data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:"Control",interaction:#{wheel_zoom:"always"},series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}
"#;

fn chart_position(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> gpui::Point<gpui::Pixels> {
    let bounds = chart_bounds(visual, view);
    point(px((bounds.x + 180.0) as f32), px((bounds.y + 160.0) as f32))
}

fn click_named(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
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
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("n")}|${ctx.get_state("last")}`),chart::Chart(#{key:"c",key_dimension:"id",zoom:1.0,viewport_revision:0,data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:"Control",interaction:#{wheel_zoom:"always"},series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}
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
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("count")}|${ctx.get_state("pending_z")}`),text("Acknowledge").accessibility_role("button").accessibility_label("Acknowledge").on_click(Fn("acknowledge")).with_style(style().width(px(140)).height(px(32))),chart::Chart(#{key:"c",key_dimension:"id",zoom:ctx.get_state("z"),viewport_revision:ctx.get_state("rev"),data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:"Control",interaction:#{wheel_zoom:"always"},series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(420)).height(px(300)))])}"#;
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
fn resumed_chart_discards_an_interrupted_explicit_wheel_gesture(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    for active_gesture in [false, true] {
        let (window, view) = mount(
            cx,
            CONTROLLED_ZOOM_SOURCE,
            &format!("chart-resume-wheel-{active_gesture}"),
        );
        let mut visual = VisualTestContext::from_window(*window, cx);
        pump(cx, &mut visual);
        if active_gesture {
            let position = chart_position(&mut visual, &view);
            visual.simulate_event(ScrollWheelEvent {
                position,
                delta: ScrollDelta::Pixels(point(px(0.0), px(40.0))),
                touch_phase: gpui::TouchPhase::Started,
                ..Default::default()
            });
            visual.run_until_parked();
        }
        visual.update(|window, cx| view.suspend(window, cx).unwrap());
        visual.run_until_parked();
        visual.update(|_, cx| view.resume(cx).unwrap());
        pump(cx, &mut visual);
        let position = chart_position(&mut visual, &view);
        visual.simulate_event(ScrollWheelEvent {
            position,
            delta: ScrollDelta::Lines(point(0.0, 1.0)),
            touch_phase: gpui::TouchPhase::Moved,
            ..Default::default()
        });
        pump(cx, &mut visual);
        assert!(
            status(&mut visual, &view).starts_with("1|"),
            "active_gesture={active_gesture}"
        );
    }
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
    let script = r#"import "charts/chart" as chart; fn view(ctx){chart::Chart(#{key:"c",key_dimension:"id",selected_keys:["first"],data:[#{id:"first",x:0,y:1},#{id:"second",x:1,y:2}],spec:#{title:"Control",interaction:#{wheel_zoom:"always"},series:[#{key:"s",kind:"scatter",encode:#{x:"x",y:"y",name:"id"}}]}}).with_style(style().width(px(420)).height(px(300)))}"#;
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
    assert_eq!(
        ChartSpec::from_ui_value(spec).unwrap().series[0].kind,
        ChartSeriesKind::Bar
    );
}

#[gpui::test]
fn formal_chart_adapters_forward_the_controlled_viewport(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    for (name, script) in [
        (
            "bar",
            r#"import "charts/bar_chart" as adapter;fn view(ctx){adapter::BarChart(#{key:"c",key_dimension:"id",viewport:#{kind:"cartesian",region:"main",y:#{key:"y",min:0,max:2}},viewport_revision:7,data:[#{id:"a",x:"A",y:1}],encode:#{x:"x",y:"y"}})}"#,
        ),
        (
            "line",
            r#"import "charts/line_chart" as adapter;fn view(ctx){adapter::LineChart(#{key:"c",key_dimension:"id",viewport:#{kind:"cartesian",region:"main",y:#{key:"y",min:0,max:2}},viewport_revision:7,data:[#{id:"a",x:"A",y:1}],encode:#{x:"x",y:"y"}})}"#,
        ),
        (
            "pie",
            r#"import "charts/pie_chart" as adapter;fn view(ctx){adapter::PieChart(#{key:"c",key_dimension:"id",viewport:#{kind:"cartesian",region:"main",y:#{key:"y",min:0,max:2}},viewport_revision:7,data:[#{id:"a",name:"A",value:1}],encode:#{name:"name",value:"value"}})}"#,
        ),
        (
            "map",
            r#"import "charts/map_chart" as adapter;fn view(ctx){adapter::MapChart(#{key:"c",key_dimension:"id",viewport:#{kind:"cartesian",region:"main",y:#{key:"y",min:0,max:2}},viewport_revision:7,map:"world",data:[#{id:"a",name:"A",value:1}],encode:#{name:"name",value:"value"}})}"#,
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
        assert!(matches!(
            primitive.props.get("viewport"),
            Some(PrimitiveValue::Data(UiValue::Map(_)))
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
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("s")}|${ctx.get_state("l")}`),chart::Chart(#{key:"c",key_dimension:"id",data:[#{id:"DATUM",x:"A",y:1}],spec:#{title:"Control",interaction:#{wheel_zoom:"always"},legend:#{visible:false},series:[#{key:"s",kind:"bar",encode:#{x:"x",y:"y"}}]},on_select:Fn("select"),on_legend_change:Fn("legend")}).with_style(style().width(px(420)).height(px(300)))])}"#.replace("DATUM", key);
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
fn invalid_chart_candidate_reaches_view_error_and_keeps_last_good_scene(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
        import "charts/chart" as chart;
        fn state_schema(){#{fields:#{kind:#{schema:#{type:"string"},"default":#{type:"string",value:"line"}}}}}
        fn break_chart(ctx,p){ctx.set_state("kind","piee");}
        fn view(ctx){column([
            text("Break chart").accessibility_role("button").accessibility_label("Break chart")
                .on_click(Fn("break_chart")).with_style(style().width(px(140)).height(px(32))),
            chart::Chart(#{key:"c",key_dimension:"id",data:[#{id:"a",x:0,y:0},#{id:"b",x:1,y:1}],spec:#{title:"Control",series:[#{key:"s",kind:ctx.get_state("kind"),encode:#{x:"x",y:"y"}}]}})
                .with_style(style().width(px(420)).height(px(300)))
        ])}
    "#;
    let (window, view) = mount(cx, script, "chart-diagnostic");
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    assert!(
        visual
            .update(|_, cx| view.last_error(cx).unwrap())
            .is_none()
    );
    click_named(&mut visual, &view, "button", "Break chart");
    let error = visual
        .update(|_, cx| view.last_error(cx).unwrap())
        .expect("invalid Chart candidate must reach the view error channel");
    assert!(error.contains("piee"), "{error}");
    assert!(
        visual.update(|_, cx| {
            let snapshot = view.accessibility_snapshot(cx).unwrap();
            snapshot
                .find_by_role_and_name("figure", "Control")
                .next()
                .is_some_and(|figure| figure.invalid && figure.description.contains("piee"))
        }),
        "last-good Chart scene must remain committed and expose the candidate error"
    );
}

#[gpui::test]
fn invalid_inline_chart_data_reaches_view_error(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
        import "charts/chart" as chart;
        fn state_schema(){#{fields:#{broken:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn break_data(ctx,p){ctx.set_state("broken",true);}
        fn view(ctx){column([
            text("Break data").accessibility_role("button").accessibility_label("Break data")
                .on_click(Fn("break_data")).with_style(style().width(px(140)).height(px(32))),
            chart::Chart(#{key:"c",key_dimension:"id",data:if ctx.get_state("broken"){[1]}else{[#{id:"a",x:0,y:0}]},spec:#{title:"Control",series:[#{key:"s",kind:"line",encode:#{x:"x",y:"y"}}]}})
                .with_style(style().width(px(420)).height(px(300)))
        ])}
    "#;
    let (window, view) = mount(cx, script, "chart-data-diagnostic");
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    click_named(&mut visual, &view, "button", "Break data");
    let error = visual
        .update(|_, cx| view.last_error(cx).unwrap())
        .expect("invalid inline Chart data must reach the view error channel");
    assert!(error.contains("inline row 0"), "{error}");
    assert!(visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .any(|node| node.invalid && node.description.contains("inline row 0"))
    }));
}

#[gpui::test]
fn chart_gauge_activation_returns_the_source_row_key(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"import "charts/chart" as chart;
 fn state_schema(){#{fields:#{last:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}}}}
 fn selected(ctx,p){ctx.set_state("last",p.datum_key);}
 fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(ctx.get_state("last")),chart::Chart(#{key:"c",key_dimension:"id",data:[#{id:"measurement-1",v:25}],spec:#{title:"Control",interaction:#{wheel_zoom:"always"},regions:[#{key:"main",kind:"polar"}],axes:[#{key:"r",position:"radial",min:0,max:100}],series:[#{key:"g",kind:"gauge",encode:#{value:"v",name:"id"}}]},on_select:Fn("selected")}).with_style(style().width(px(420)).height(px(300)))])}"#;
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

struct StreamExtension {
    data: NativeChartData,
    counter: Arc<AtomicUsize>,
}

impl ScriptViewExtension for StreamExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine
            .register_chart_series("stream_count", CountRenderer(self.counter.clone()))
            .map_err(|error| error.to_string())
    }

    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .register_native_chart_data("stream", self.data.clone())
            .map_err(|error| error.to_string())
    }
}

fn streaming_rows(offset: i64) -> ChartDataset {
    let rows = (0..2)
        .map(|index| {
            BTreeMap::from([
                ("id".to_owned(), UiValue::String(format!("r{index}"))),
                ("x".to_owned(), UiValue::Integer(index)),
                ("y".to_owned(), UiValue::Integer(index + offset)),
            ])
        })
        .collect::<Vec<_>>();
    ChartDataset::from_rows(
        "main",
        &rows,
        Some("id".to_owned()),
        ChartDataLimits::default(),
    )
    .unwrap()
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

fn mount_streaming_chart(
    cx: &mut TestAppContext,
    data: NativeChartData,
    counter: Arc<AtomicUsize>,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    let entry = ModuleId::parse("main").unwrap();
    let script = r#"import "charts/chart" as chart;fn view(ctx){chart::Chart(#{key:"c",data:ctx.get_native_chart_data("stream"),spec:#{title:"Control",interaction:#{wheel_zoom:"always"},series:[#{key:"s",kind:"custom",renderer:"stream_count",encode:#{x:"x",y:"y"}}]}}).with_style(style().width(px(420)).height(px(300)))}"#;
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
    .extension(StreamExtension { data, counter })
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("chart-suspend-stream", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("chart-suspend-stream"),
                host.clone(),
                window,
                cx,
            )
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
fn suspended_chart_coalesces_stream_updates_until_resume(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let data = NativeChartData::new([streaming_rows(0)], ChartDataLimits::default()).unwrap();
    let counter = Arc::new(AtomicUsize::new(0));
    let (window, view) = mount_streaming_chart(cx, data.clone(), counter.clone());
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    visual.update(|window, cx| view.suspend(window, cx).unwrap());
    visual.run_until_parked();
    let before = counter.load(Ordering::Relaxed);
    for revision in 1..=3 {
        data.replace([streaming_rows(revision)]).unwrap();
        visual.run_until_parked();
    }
    assert_eq!(counter.load(Ordering::Relaxed), before);

    visual.update(|_, cx| view.resume(cx).unwrap());
    pump(cx, &mut visual);
    assert_eq!(counter.load(Ordering::Relaxed), before + 1);
}

struct MotionExtension {
    data: NativeChartData,
    positions: Arc<std::sync::Mutex<Vec<ChartPoint>>>,
}

struct MovingRenderer(Arc<std::sync::Mutex<Vec<ChartPoint>>>);

impl HostChartSeries for MovingRenderer {
    fn layout(&self, context: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
        let value = context
            .dataset
            .column("y")
            .unwrap()
            .value(0)
            .unwrap()
            .as_number()
            .unwrap();
        let rect = ChartRect {
            x: context.bounds.x + 20.0 + value * 200.0,
            y: context.bounds.y + 30.0,
            width: 40.0,
            height: 40.0,
        };
        self.0.lock().unwrap().push(rect.center());
        Ok(vec![ChartMark {
            key: "moving".to_owned(),
            region_key: context.spec.coordinate.clone(),
            role: ChartMarkRole::Data,
            datum: Some(ChartDatumRef {
                dataset: context.spec.dataset.clone(),
                series: context.spec.key.clone(),
                key: "r0".to_owned(),
            }),
            series_key: context.spec.key.clone(),
            datum_key: "r0".to_owned(),
            geometry: ChartMarkGeometry::Rect(rect),
            fill: Some(context.theme.palette[0]),
            stroke: None,
            label: "moving".to_owned(),
            value: Some(value),
            interactive: true,
            selected: false,
        }])
    }
}

impl ScriptViewExtension for MotionExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine
            .register_chart_series("moving", MovingRenderer(self.positions.clone()))
            .map_err(|error| error.to_string())
    }

    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .register_native_chart_data("motion_stream", self.data.clone())
            .map_err(|error| error.to_string())
    }
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
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        self.motion.configure_engine(engine)?;
        engine
            .register_primitive(
                PrimitiveDescriptor {
                    id: PrimitiveId::parse("zz_lifecycle.clock_bomb").unwrap(),
                    export: "ClockBomb".to_owned(),
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
            .map_err(|error| error.to_string())
    }

    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        self.motion.configure_runtime(runtime)
    }
}

fn moving_data(y: i64) -> ChartDataset {
    ChartDataset::from_rows(
        "main",
        &[BTreeMap::from([
            ("id".to_owned(), UiValue::String("r0".to_owned())),
            ("x".to_owned(), UiValue::Integer(0)),
            ("y".to_owned(), UiValue::Integer(y)),
        ])],
        Some("id".to_owned()),
        ChartDataLimits::default(),
    )
    .unwrap()
}

fn mount_extended_chart(
    cx: &mut TestAppContext,
    name: &str,
    script: &str,
    extension: impl ScriptViewExtension + 'static,
    clock: RuntimeClock,
    preference: MotionPreference,
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
    .extension(extension)
    .runtime_clock(clock)
    .motion_preference(preference)
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
fn chart_motion_freezes_across_view_suspension(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let clock = ManualRuntimeClock::new(std::time::Instant::now());
    let data = NativeChartData::new([moving_data(0)], ChartDataLimits::default()).unwrap();
    let positions = Arc::new(std::sync::Mutex::new(Vec::new()));
    let script = r#"import "charts/chart" as chart;fn state_schema(){#{fields:#{hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}fn clicked(ctx,p){ctx.set_state("hits",ctx.get_state("hits")+1);}fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(ctx.get_state("hits").to_string()),chart::Chart(#{key:"c",data:ctx.get_native_chart_data("motion_stream"),spec:#{title:"Control",interaction:#{wheel_zoom:"always"},legend:#{visible:false},motion:#{duration:"slow",easing:"standard"},series:[#{key:"s",kind:"custom",renderer:"moving",encode:#{x:"x",y:"y"}}]},on_select:Fn("clicked")}).with_style(style().width(px(420)).height(px(300)))])}"#;
    let (window, view) = mount_extended_chart(
        cx,
        "chart-frozen-motion",
        script,
        MotionExtension {
            data: data.clone(),
            positions: positions.clone(),
        },
        clock.clock(),
        MotionPreference::Normal,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    clock.advance(Duration::from_secs(2));
    pump(cx, &mut visual);
    let old = positions.lock().unwrap()[0];
    data.replace([moving_data(1)]).unwrap();
    pump(cx, &mut visual);
    click_chart_coordinate(&mut visual, &view, old);
    assert_eq!(status(&mut visual, &view), "1");
    visual.update(|window, cx| view.suspend(window, cx).unwrap());
    visual.run_until_parked();
    clock.advance(Duration::from_secs(60));
    visual.update(|_, cx| view.resume(cx).unwrap());
    pump(cx, &mut visual);
    click_chart_coordinate(&mut visual, &view, old);
    assert_eq!(status(&mut visual, &view), "2");
}

#[gpui::test]
fn failed_resume_prepare_does_not_consume_chart_motion_time(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let clock = ManualRuntimeClock::new(std::time::Instant::now());
    let data = NativeChartData::new([moving_data(0)], ChartDataLimits::default()).unwrap();
    let positions = Arc::new(std::sync::Mutex::new(Vec::new()));
    let fail = Rc::new(std::cell::Cell::new(true));
    let script = r#"import "charts/chart" as chart;fn state_schema(){#{fields:#{hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}fn clicked(ctx,p){ctx.set_state("hits",ctx.get_state("hits")+1);}fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(ctx.get_state("hits").to_string()),chart::Chart(#{key:"c",data:ctx.get_native_chart_data("motion_stream"),spec:#{title:"Control",interaction:#{wheel_zoom:"always"},legend:#{visible:false},motion:#{duration:"slow",easing:"standard"},series:[#{key:"s",kind:"custom",renderer:"moving",encode:#{x:"x",y:"y"}}]},on_select:Fn("clicked")}).with_style(style().width(px(420)).height(px(300))),zz_lifecycle::ClockBomb(#{key:"bomb"}).with_key("bomb")])}"#;
    let (window, view) = mount_extended_chart(
        cx,
        "chart-failed-resume-motion",
        script,
        FailedResumeMotionExtension {
            motion: MotionExtension {
                data: data.clone(),
                positions: positions.clone(),
            },
            clock: clock.clone(),
            fail,
        },
        clock.clock(),
        MotionPreference::Normal,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    clock.advance(Duration::from_secs(2));
    pump(cx, &mut visual);
    let old = positions.lock().unwrap()[0];
    data.replace([moving_data(1)]).unwrap();
    pump(cx, &mut visual);
    click_chart_coordinate(&mut visual, &view, old);
    assert_eq!(status(&mut visual, &view), "1");
    visual.update(|window, cx| view.suspend(window, cx).unwrap());
    clock.advance(Duration::from_secs(60));
    assert!(visual.update(|_, cx| view.resume(cx)).is_err());
    assert_eq!(view.state(), ScriptViewState::Suspended);
    visual.update(|_, cx| view.resume(cx).unwrap());
    pump(cx, &mut visual);
    click_chart_coordinate(&mut visual, &view, old);
    assert_eq!(status(&mut visual, &view), "2");
}

fn presented_revision(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> u64 {
    visual.update(|_, cx| {
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        let node = snapshot
            .find_by_role_and_name("figure", "Control")
            .next()
            .unwrap();
        let Some(UiValue::Map(values)) = &node.value else {
            panic!("missing chart projection")
        };
        let UiValue::Integer(revision) = values["revision"] else {
            panic!("missing presented revision")
        };
        u64::try_from(revision).unwrap()
    })
}

#[gpui::test]
fn resume_finishes_prepared_but_unpresented_frame(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let data = NativeChartData::new([moving_data(0)], ChartDataLimits::default()).unwrap();
    let positions = Arc::new(std::sync::Mutex::new(Vec::new()));
    let script = r#"import "charts/chart" as chart;fn view(ctx){chart::Chart(#{key:"c",data:ctx.get_native_chart_data("motion_stream"),spec:#{title:"Control",interaction:#{wheel_zoom:"always"},legend:#{visible:false},series:[#{key:"s",kind:"custom",renderer:"moving",encode:#{x:"x",y:"y"}}]}}).with_style(style().width(px(420)).height(px(300)))}"#;
    let (window, view) = mount_extended_chart(
        cx,
        "chart-pending-frame",
        script,
        MotionExtension {
            data: data.clone(),
            positions: positions.clone(),
        },
        RuntimeClock::default(),
        MotionPreference::None,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    let old_revision = presented_revision(&mut visual, &view);
    let old_layouts = positions.lock().unwrap().len();
    let wanted = data.replace([moving_data(1)]).unwrap();
    let mut steps = 0;
    while positions.lock().unwrap().len() == old_layouts {
        assert!(steps < 100);
        assert!(cx.background_executor.tick());
        steps += 1;
    }
    assert_eq!(presented_revision(&mut visual, &view), old_revision);
    visual.update(|window, cx| view.suspend(window, cx).unwrap());
    visual.run_until_parked();
    visual.update(|_, cx| view.resume(cx).unwrap());
    pump(cx, &mut visual);
    assert_eq!(presented_revision(&mut visual, &view), wanted);
}

#[gpui::test]
fn failed_script_suspend_restores_committed_chart_activity(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let data = NativeChartData::new([moving_data(0)], ChartDataLimits::default()).unwrap();
    let positions = Arc::new(std::sync::Mutex::new(Vec::new()));
    let script = r#"import "charts/chart" as chart;fn suspend(ctx){throw "injected script suspend failure";}fn view(ctx){chart::Chart(#{key:"c",data:ctx.get_native_chart_data("motion_stream"),spec:#{title:"Control",interaction:#{wheel_zoom:"always"},legend:#{visible:false},series:[#{key:"s",kind:"custom",renderer:"moving",encode:#{x:"x",y:"y"}}]}}).with_style(style().width(px(420)).height(px(300)))}"#;
    let (window, view) = mount_extended_chart(
        cx,
        "chart-failed-script-suspend",
        script,
        MotionExtension {
            data: data.clone(),
            positions,
        },
        RuntimeClock::default(),
        MotionPreference::None,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    assert!(
        visual
            .update(|window, cx| view.suspend(window, cx))
            .is_err()
    );
    assert_eq!(view.state(), ScriptViewState::Active);
    let wanted = data.replace([moving_data(1)]).unwrap();
    pump(cx, &mut visual);
    assert_eq!(presented_revision(&mut visual, &view), wanted);
}

struct ViewportMarker(Arc<std::sync::Mutex<Vec<ChartPoint>>>);

impl HostChartSeries for ViewportMarker {
    fn layout(&self, context: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
        let center = ChartPoint {
            x: context.x_scale.unwrap().map_number(2.0).unwrap(),
            y: context.bounds.center().y,
        };
        self.0.lock().unwrap().push(center);
        Ok(vec![ChartMark {
            key: "viewport_marker".to_owned(),
            region_key: context.spec.coordinate.clone(),
            role: ChartMarkRole::Data,
            datum: Some(ChartDatumRef {
                dataset: context.spec.dataset.clone(),
                series: context.spec.key.clone(),
                key: "r0".to_owned(),
            }),
            series_key: context.spec.key.clone(),
            datum_key: "r0".to_owned(),
            geometry: ChartMarkGeometry::Rect(ChartRect {
                x: center.x - 6.0,
                y: center.y - 6.0,
                width: 12.0,
                height: 12.0,
            }),
            fill: Some(context.theme.palette[0]),
            stroke: None,
            label: "r0".to_owned(),
            value: Some(2.0),
            interactive: true,
            selected: false,
        }])
    }
}

struct ViewportExtension(Arc<std::sync::Mutex<Vec<ChartPoint>>>);

impl ScriptViewExtension for ViewportExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine
            .register_chart_series("viewport_marker", ViewportMarker(self.0.clone()))
            .map_err(|error| error.to_string())
    }
}

#[gpui::test]
fn canceling_preview_rebuilds_the_committed_frame(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let positions = Arc::new(std::sync::Mutex::new(Vec::new()));
    let script = r#"import "charts/chart" as chart;fn state_schema(){#{fields:#{hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}fn clicked(ctx,p){ctx.set_state("hits",ctx.get_state("hits")+1);}fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(ctx.get_state("hits").to_string()),chart::Chart(#{key:"c",key_dimension:"id",zoom:1.0,viewport_revision:0,data:[#{id:"r0",x:2,y:0},#{id:"r1",x:10,y:1}],spec:#{title:"Control",interaction:#{wheel_zoom:"always"},legend:#{visible:false},axes:[#{key:"x",position:"bottom",min:0,max:10}],series:[#{key:"s",kind:"custom",renderer:"viewport_marker",encode:#{x:"x",y:"y"}}]},on_select:Fn("clicked")}).with_style(style().width(px(420)).height(px(300)))])}"#;
    let (window, view) = mount_extended_chart(
        cx,
        "chart-cancel-preview",
        script,
        ViewportExtension(positions.clone()),
        RuntimeClock::default(),
        MotionPreference::None,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    let committed = positions.lock().unwrap()[0];
    let bounds = chart_bounds(&mut visual, &view);
    visual.simulate_event(ScrollWheelEvent {
        position: point(px((bounds.x + 180.0) as f32), px((bounds.y + 160.0) as f32)),
        delta: ScrollDelta::Pixels(point(px(0.0), px(400.0 * std::f32::consts::LN_2))),
        touch_phase: gpui::TouchPhase::Started,
        ..Default::default()
    });
    pump(cx, &mut visual);
    let preview = *positions.lock().unwrap().last().unwrap();
    assert!((preview.x - committed.x).abs() > 20.0);
    visual.update(|window, cx| view.suspend(window, cx).unwrap());
    visual.update(|_, cx| view.resume(cx).unwrap());
    pump(cx, &mut visual);
    click_chart_coordinate(&mut visual, &view, committed);
    assert_eq!(status(&mut visual, &view), "1");
}

struct GeoMarker(Arc<std::sync::Mutex<BTreeMap<String, ChartPoint>>>);

type AxisWindows = Arc<std::sync::Mutex<BTreeMap<String, ((f64, f64), (f64, f64))>>>;

struct AxisProbe(AxisWindows);

impl HostChartSeries for AxisProbe {
    fn layout(&self, context: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
        let x = context.x_scale.unwrap();
        let y = context.y_scale.unwrap();
        self.0.lock().unwrap().insert(
            context.spec.key.clone(),
            (
                (
                    x.invert(context.bounds.x).unwrap(),
                    x.invert(context.bounds.x + context.bounds.width).unwrap(),
                ),
                (
                    y.invert(context.bounds.y + context.bounds.height).unwrap(),
                    y.invert(context.bounds.y).unwrap(),
                ),
            ),
        );
        Ok(Vec::new())
    }
}

struct AxisProbeExtension(AxisWindows);

impl ScriptViewExtension for AxisProbeExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine
            .register_chart_series("axis_probe", AxisProbe(self.0.clone()))
            .map_err(|error| error.to_string())
    }
}

fn axis_window(windows: &AxisWindows, chart: &str) -> (f64, f64) {
    windows.lock().unwrap()[chart].0
}

fn wheel_figure(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    chart: &str,
    delta: f32,
    phase: gpui::TouchPhase,
) {
    let position = figure_coordinate(visual, view, chart, ChartPoint { x: 150.0, y: 140.0 });
    visual.simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Pixels(point(px(0.0), px(delta))),
        touch_phase: phase,
        ..Default::default()
    });
}

fn same_window(left: (f64, f64), right: (f64, f64)) -> bool {
    (left.0 - right.0).abs() < 0.000_001 && (left.1 - right.1).abs() < 0.000_001
}

#[gpui::test]
fn linked_target_gesture_uses_its_effective_axis_window(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let windows = AxisWindows::default();
    let script = r#"import "charts/chart" as chart;
fn state_schema(){#{fields:#{zs:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},zt:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},rs:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},rt:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn zs(ctx,p){ctx.set_state("zs",p.zoom);ctx.set_state("rs",p.viewport_revision);}fn zt(ctx,p){ctx.set_state("zt",p.zoom);ctx.set_state("rt",p.viewport_revision);}
fn one(ctx,k,z,r,cb){chart::Chart(#{key:k,key_dimension:"id",data:[#{id:"r0",x:0,y:0},#{id:"r1",x:100,y:100}],zoom:z,viewport_revision:r,spec:#{title:k,interaction:#{wheel_zoom:"always"},legend:#{visible:false},link_group:"shared",link_domain:"same_values",axes:[#{key:"x",position:"bottom",min:0,max:100},#{key:"y",position:"left",min:0,max:100}],series:[#{key:k,kind:"custom",renderer:"axis_probe",encode:#{x:"x",y:"y"}}]},on_zoom_change:cb}).with_style(style().width(px(280)).height(px(240)))}
fn view(ctx){row([one(ctx,"source",ctx.get_state("zs"),ctx.get_state("rs"),Fn("zs")),one(ctx,"target",ctx.get_state("zt"),ctx.get_state("rt"),Fn("zt"))])}"#;
    let (window, view) = mount_extended_chart(
        cx,
        "chart-linked-target-gesture",
        script,
        AxisProbeExtension(windows.clone()),
        RuntimeClock::default(),
        MotionPreference::None,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    wheel_figure(
        &mut visual,
        &view,
        "source",
        -400.0,
        gpui::TouchPhase::Started,
    );
    wheel_figure(&mut visual, &view, "source", 0.0, gpui::TouchPhase::Ended);
    pump(cx, &mut visual);
    let linked = axis_window(&windows, "target");
    wheel_figure(
        &mut visual,
        &view,
        "target",
        -200.0,
        gpui::TouchPhase::Started,
    );
    pump(cx, &mut visual);
    let preview = axis_window(&windows, "target");
    wheel_figure(&mut visual, &view, "target", 0.0, gpui::TouchPhase::Ended);
    pump(cx, &mut visual);
    let committed = axis_window(&windows, "target");
    let source = axis_window(&windows, "source");
    assert!(preview.1 - preview.0 < linked.1 - linked.0);
    assert!(same_window(preview, committed));
    assert!(same_window(committed, source));
}

#[gpui::test]
fn link_projection_identity_includes_the_group(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let windows = AxisWindows::default();
    let script = r#"import "charts/chart" as chart;
fn state_schema(){#{fields:#{group:#{schema:#{type:"string"},"default":#{type:"string",value:"group_a"}},za:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},zb:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},ra:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},rb:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn za(ctx,p){ctx.set_state("za",p.zoom);ctx.set_state("ra",p.viewport_revision);}fn zb(ctx,p){ctx.set_state("zb",p.zoom);ctx.set_state("rb",p.viewport_revision);}fn change_group(ctx,p){ctx.set_state("group","group_b");}
fn one(ctx,k,g,z,r,cb){chart::Chart(#{key:k,key_dimension:"id",data:[#{id:"r0",x:0,y:0},#{id:"r1",x:100,y:100}],zoom:z,viewport_revision:r,spec:#{title:k,interaction:#{wheel_zoom:"always"},legend:#{visible:false},link_group:g,link_domain:"same_values",axes:[#{key:"x",position:"bottom",min:0,max:100},#{key:"y",position:"left",min:0,max:100}],series:[#{key:k,kind:"custom",renderer:"axis_probe",encode:#{x:"x",y:"y"}}]},on_zoom_change:cb}).with_style(style().width(px(280)).height(px(240)))}
fn view(ctx){column([text("Switch").accessibility_role("button").accessibility_label("Switch").on_click(Fn("change_group")).with_style(style().width(px(140)).height(px(32))),row([one(ctx,"a","group_a",ctx.get_state("za"),ctx.get_state("ra"),Fn("za")),one(ctx,"b","group_b",ctx.get_state("zb"),ctx.get_state("rb"),Fn("zb")),one(ctx,"target",ctx.get_state("group"),1.0,0,())])])}"#;
    let (window, view) = mount_extended_chart(
        cx,
        "chart-link-group-identity",
        script,
        AxisProbeExtension(windows.clone()),
        RuntimeClock::default(),
        MotionPreference::None,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    for (chart, delta) in [("a", -200.0), ("b", -400.0)] {
        wheel_figure(&mut visual, &view, chart, delta, gpui::TouchPhase::Started);
        wheel_figure(&mut visual, &view, chart, 0.0, gpui::TouchPhase::Ended);
        pump(cx, &mut visual);
    }
    let before = axis_window(&windows, "target");
    assert!(same_window(before, axis_window(&windows, "a")));
    click_named(&mut visual, &view, "button", "Switch");
    pump(cx, &mut visual);
    assert!(same_window(
        axis_window(&windows, "target"),
        axis_window(&windows, "b")
    ));
}

#[gpui::test]
fn resize_reprojects_without_discarding_active_preview(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let windows = AxisWindows::default();
    let script = r#"import "charts/chart" as chart;
fn state_schema(){#{fields:#{w:#{schema:#{type:"float"},"default":#{type:"float",value:280.0}},z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},rev:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn wider(ctx,p){ctx.set_state("w",420.0);}fn zoomed(ctx,p){ctx.set_state("z",p.zoom);ctx.set_state("rev",p.viewport_revision);}
fn view(ctx){column([text("Resize").accessibility_role("button").accessibility_label("Resize").on_click(Fn("wider")).with_style(style().width(px(140)).height(px(32))),chart::Chart(#{key:"c",key_dimension:"id",data:[#{id:"r0",x:0,y:0},#{id:"r1",x:100,y:100}],zoom:ctx.get_state("z"),viewport_revision:ctx.get_state("rev"),spec:#{title:"c",interaction:#{wheel_zoom:"always"},legend:#{visible:false},axes:[#{key:"x",position:"bottom",min:0,max:100},#{key:"y",position:"left",min:0,max:100}],series:[#{key:"c",kind:"custom",renderer:"axis_probe",encode:#{x:"x",y:"y"}}]},on_zoom_change:Fn("zoomed")}).with_style(style().width(px(ctx.get_state("w"))).height(px(240)))])}"#;
    let (window, view) = mount_extended_chart(
        cx,
        "chart-resize-preview",
        script,
        AxisProbeExtension(windows.clone()),
        RuntimeClock::default(),
        MotionPreference::None,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    wheel_figure(&mut visual, &view, "c", -200.0, gpui::TouchPhase::Started);
    pump(cx, &mut visual);
    let preview = axis_window(&windows, "c");
    click_named(&mut visual, &view, "button", "Resize");
    pump(cx, &mut visual);
    assert!(same_window(preview, axis_window(&windows, "c")));
    wheel_figure(&mut visual, &view, "c", 0.0, gpui::TouchPhase::Ended);
    pump(cx, &mut visual);
    assert!(same_window(preview, axis_window(&windows, "c")));
}

#[gpui::test]
fn linked_cartesian_axes_keep_independent_logical_windows(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let windows = AxisWindows::default();
    let script = r#"import "charts/chart" as chart;fn state_schema(){#{fields:#{z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},rev:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}fn zoomed(ctx,p){ctx.set_state("z",p.zoom);ctx.set_state("rev",p.viewport_revision);}fn one(ctx,k){chart::Chart(#{key:k,key_dimension:"id",data:[#{id:"r0",x:0,y:0},#{id:"r1",x:100,y:100}],zoom:if k=="source"{ctx.get_state("z")}else{1.0},viewport_revision:if k=="source"{ctx.get_state("rev")}else{0},spec:#{title:k,interaction:#{wheel_zoom:"always"},legend:#{visible:false},link_group:"xy",link_domain:"same_values",axes:[#{key:"x",position:"bottom",min:0,max:if k=="source"{100}else{200}},#{key:"y",position:"left",min:0,max:100}],series:[#{key:k,kind:"custom",renderer:"axis_probe",encode:#{x:"x",y:"y"}}]},on_zoom_change:if k=="source"{Fn("zoomed")}else{()}}).with_style(style().width(px(280)).height(px(240)))}fn view(ctx){row([one(ctx,"source"),one(ctx,"target")])}"#;
    let (window, view) = mount_extended_chart(
        cx,
        "chart-linked-axis-windows",
        script,
        AxisProbeExtension(windows.clone()),
        RuntimeClock::default(),
        MotionPreference::None,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    let wheel = figure_coordinate(
        &mut visual,
        &view,
        "source",
        ChartPoint { x: 150.0, y: 140.0 },
    );
    visual.simulate_event(ScrollWheelEvent {
        position: wheel,
        delta: ScrollDelta::Lines(point(0.0, -25.0)),
        touch_phase: gpui::TouchPhase::Moved,
        ..Default::default()
    });
    pump(cx, &mut visual);
    let windows = windows.lock().unwrap();
    let source = windows["source"];
    let target = windows["target"];
    for (source, target) in [(source.0, target.0), (source.1, target.1)] {
        assert!((source.0 - target.0).abs() < 0.000_001);
        assert!((source.1 - target.1).abs() < 0.000_001);
    }
}

impl HostChartSeries for GeoMarker {
    fn layout(&self, context: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String> {
        let center = ChartPoint {
            x: context.bounds.x + 30.0,
            y: context.bounds.y + 30.0,
        };
        self.0
            .lock()
            .unwrap()
            .insert(context.spec.key.clone(), center);
        Ok(vec![ChartMark {
            key: format!("geo:{}", context.spec.key),
            region_key: context.spec.coordinate.clone(),
            role: ChartMarkRole::Data,
            datum: Some(ChartDatumRef {
                dataset: context.spec.dataset.clone(),
                series: context.spec.key.clone(),
                key: "r0".to_owned(),
            }),
            series_key: context.spec.key.clone(),
            datum_key: "r0".to_owned(),
            geometry: ChartMarkGeometry::Rect(ChartRect {
                x: center.x - 10.0,
                y: center.y - 10.0,
                width: 20.0,
                height: 20.0,
            }),
            fill: Some(context.theme.palette[0]),
            stroke: None,
            label: "r0".to_owned(),
            value: None,
            interactive: true,
            selected: false,
        }])
    }
}

struct GeoExtension(Arc<std::sync::Mutex<BTreeMap<String, ChartPoint>>>);

impl ScriptViewExtension for GeoExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        engine
            .register_chart_map(
                ChartGeoMap::from_geojson(
                    "map",
                    r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"square","properties":{},"geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,10],[0,10],[0,0]]]}}]}"#,
                )
                .unwrap(),
            )
            .map_err(|error| error.to_string())?;
        engine
            .register_chart_series("geo_marker", GeoMarker(self.0.clone()))
            .map_err(|error| error.to_string())
    }
}

#[gpui::test]
fn geo_link_group_synchronizes_acknowledged_camera(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let positions = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let script = r#"import "charts/chart" as chart;fn state_schema(){#{fields:#{z:#{schema:#{type:"float"},"default":#{type:"float",value:1.0}},rev:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},source_hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},target_hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}fn zoomed(ctx,p){ctx.set_state("z",p.zoom);ctx.set_state("rev",p.viewport_revision);}fn source_clicked(ctx,p){ctx.set_state("source_hits",ctx.get_state("source_hits")+1);}fn target_clicked(ctx,p){ctx.set_state("target_hits",ctx.get_state("target_hits")+1);}fn one(ctx,k){chart::Chart(#{key:k,key_dimension:"id",data:[#{id:"r0",x:0,y:0}],zoom:if k=="source"{ctx.get_state("z")}else{1.0},viewport_revision:if k=="source"{ctx.get_state("rev")}else{0},spec:#{title:k,interaction:#{wheel_zoom:"always"},legend:#{visible:false},link_group:"maps",link_domain:"location",regions:[#{key:"main",kind:"geo_2d",map:"map"}],series:[#{key:k,kind:"custom",renderer:"geo_marker",encode:#{x:"x",y:"y"}}]},on_zoom_change:if k=="source"{Fn("zoomed")}else{()},on_select:if k=="source"{Fn("source_clicked")}else{Fn("target_clicked")}}).with_style(style().width(px(280)).height(px(240)))}fn view(ctx){column([text("Status").accessibility_role("status").accessibility_label(`${ctx.get_state("source_hits")}|${ctx.get_state("target_hits")}|${ctx.get_state("z")}`),row([one(ctx,"source"),one(ctx,"target")])])}"#;
    let (window, view) = mount_extended_chart(
        cx,
        "chart-geo-link",
        script,
        GeoExtension(positions.clone()),
        RuntimeClock::default(),
        MotionPreference::None,
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    pump(cx, &mut visual);
    let source_position = positions.lock().unwrap()["source"];
    let target_position = positions.lock().unwrap()["target"];
    let source = figure_coordinate(&mut visual, &view, "source", source_position);
    let target = figure_coordinate(&mut visual, &view, "target", target_position);
    visual.simulate_click(source, Modifiers::default());
    visual.simulate_click(target, Modifiers::default());
    visual.run_until_parked();
    let wheel = figure_coordinate(
        &mut visual,
        &view,
        "source",
        ChartPoint { x: 150.0, y: 140.0 },
    );
    visual.simulate_event(ScrollWheelEvent {
        position: wheel,
        delta: ScrollDelta::Lines(point(0.0, -25.0)),
        touch_phase: gpui::TouchPhase::Moved,
        ..Default::default()
    });
    pump(cx, &mut visual);
    visual.simulate_click(source, Modifiers::default());
    visual.simulate_click(target, Modifiers::default());
    visual.run_until_parked();
    assert!(status(&mut visual, &view).starts_with("1|1|"));
    visual.update(|window, cx| view.suspend(window, cx).unwrap());
    visual.update(|_, cx| view.resume(cx).unwrap());
    pump(cx, &mut visual);
    visual.simulate_click(source, Modifiers::default());
    visual.simulate_click(target, Modifiers::default());
    visual.run_until_parked();
    assert!(status(&mut visual, &view).starts_with("1|1|"));
}

fn figure_coordinate(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    name: &str,
    coordinate: ChartPoint,
) -> gpui::Point<gpui::Pixels> {
    visual.update(|_, cx| {
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        let bounds = snapshot
            .find_by_role_and_name("figure", name)
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual;
        point(
            px((bounds.x + coordinate.x + 1.0) as f32),
            px((bounds.y + coordinate.y + 1.0) as f32),
        )
    })
}

fn click_chart_coordinate(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    coordinate: ChartPoint,
) {
    let bounds = chart_bounds(visual, view);
    visual.simulate_click(
        point(
            px((bounds.x + coordinate.x + 1.0) as f32),
            px((bounds.y + coordinate.y + 1.0) as f32),
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();
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
fn linked_selection_keeps_each_chart_source_instead_of_last_writer_wins(cx: &mut TestAppContext) {
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
