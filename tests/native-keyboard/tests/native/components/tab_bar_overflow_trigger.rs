//! TabBar's all-tabs menu trigger fills the bar's height (#128): it asked for
//! `height(relative(1.0))` inside the Menu root, which has no definite height, so it
//! collapsed to the 16px chevron at the bar's top edge. It now takes `metrics.control`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window};
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

/// A bar `width` wide over `count` tabs (t0..), the selected value in state; `extra` adds
/// TabBar props. The log line records events.
fn script(count: usize, width: u32, extra: &str) -> String {
    format!(
        r#"import "components/tab_bar" as tab_bar;
fn state_schema(){{#{{fields:#{{
    value:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"t1"}}}},
    log:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn note(ctx,line){{ctx.set_state("log",`${{ctx.get_state("log")}} ${{line}}`);}}
fn changed(ctx,value){{ctx.set_state("value",value);note(ctx,`change:${{value}}`);}}
fn closed(ctx,value){{note(ctx,`close:${{value}}`);}}
fn asked(ctx,request){{note(ctx,`ctx:${{request.value}}:${{request.source}}`);}}
fn moved(ctx,request){{note(ctx,`move:${{request.value}}:${{request.placement}}:${{request.anchor}}`);}}
fn tabs(){{let result=[];for index in 0..{count}{{
    result.push(#{{value:`t${{index}}`,label:`Tab number ${{index}}`,closable:index<3,dirty:index==2}});}}result}}
fn view(ctx){{column([
    tab_bar::TabBar(#{{key:"files",label:"Open files",value:ctx.get_state("value"),tabs:tabs(),
        on_change:Fn("changed"),on_close:Fn("closed"),on_context_request:Fn("asked"),on_reorder:Fn("moved")
        {extra}}}).with_style(style().width(px({width}))),
    text(ctx.get_state("log")).test_id("log"),
])}}"#
    )
}

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::from([(ModuleId::parse("main").unwrap(), source)]);
    for (id, source) in gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let prepared = EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(modules),
        gpui_rhai_registry::DEFAULT_THEME,
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .motion_preference(MotionPreference::None)
    .asset_sources(
        gpui_rhai_registry::BUNDLED_ASSET_SOURCES
            .iter()
            .map(|(path, source)| {
                (
                    path.trim_end_matches(".svg").to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".into(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }),
    )
    .prepare()
    .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("tabs", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("tabs"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    for _ in 0..3 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn bounds(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name(role, name)
            .next()
            .and_then(|node| node.geometry)
            .unwrap_or_else(|| panic!("no {role} {name:?}"))
            .visual
    })
}

#[gpui::test]
fn the_overflow_menu_trigger_fills_the_bar(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(3, 360, ",overflow_menu:true"));
    let tab = bounds(&mut visual, &view, "tab", "Tab number 0");
    let trigger = bounds(&mut visual, &view, "button", "All tabs");
    assert!(
        (trigger.height - tab.height).abs() < 0.5 && (trigger.y - tab.y).abs() < 0.5,
        "the trigger should span the bar like the tabs: tab {tab:?}, trigger {trigger:?}"
    );
}
