use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Modifiers, Render, TestAppContext, VisualTestContext, Window, WindowHandle, point, px};
use gpui_rhai::*;

const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const BADGE: &str = include_str!("../../../registry/components/badge.rhai");
const TABS: &str = include_str!("../../../registry/components/tabs.rhai");
const ANIMATED_TABS: &str = include_str!("../../../registry/motion/animated_tabs.rhai");
const DEFAULT_DARK: &str = include_str!("../../../registry/themes/default_dark.rhai");

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(cx: &mut TestAppContext, script: &str, name: &str) -> (WindowHandle<Host>, ScriptViewHandle) {
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (entry, script.to_owned()),
            (ModuleId::parse("components/button").unwrap(), BUTTON.to_owned()),
            (ModuleId::parse("components/badge").unwrap(), BADGE.to_owned()),
            (ModuleId::parse("components/tabs").unwrap(), TABS.to_owned()),
            (
                ModuleId::parse("motion/animated_tabs").unwrap(),
                ANIMATED_TABS.to_owned(),
            ),
        ])),
        DEFAULT_DARK,
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

#[gpui::test]
fn button_and_badge_keep_distinct_density_with_the_same_label(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/button" as button;
import "components/badge" as badge;
fn view(ctx) { row([
    button::Button(#{ text:"Density", size:"md", variant:"primary" }),
    badge::Badge(#{ text:"Density", size:"md", variant:"accent" }),
    button::Button(#{ text:"中文", size:"xs", variant:"outline" }),
    badge::Badge(#{ text:"中文", size:"sm", variant:"neutral" }),
    button::Button(#{ text:"Large line", size:"md", variant:"outline",
        part_styles: #{label:style().font_size(px(24)).line_height(px(30))} }),
    badge::Badge(#{ text:"Large line", size:"md", variant:"neutral",
        part_styles: #{label:style().font_size(px(24)).line_height(px(30))} }),
]).with_style(style().gap(px(8)).items_center()) }
"#;
    let (window, view) = mount(cx, script, "control-density");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let button = tree.find_by_role_and_name("button", "Density").next().unwrap().geometry.unwrap().visual;
    let badge = tree.find_by_role_and_name("status", "Density").next().unwrap().geometry.unwrap().visual;
    assert_eq!(button.height, 32.0);
    assert_eq!(badge.height, 20.0);
    assert!(button.width >= badge.width + 12.0, "button={button:?}, badge={badge:?}");
    let cjk_button = tree.find_by_role_and_name("button", "中文").next().unwrap().geometry.unwrap().visual;
    let cjk_badge = tree.find_by_role_and_name("status", "中文").next().unwrap().geometry.unwrap().visual;
    assert_eq!(cjk_button.height, 24.0);
    assert_eq!(cjk_badge.height, 18.0);
    let large_button = tree.find_by_role_and_name("button", "Large line").next().unwrap().geometry.unwrap().visual;
    let large_badge = tree.find_by_role_and_name("status", "Large line").next().unwrap().geometry.unwrap().visual;
    assert!(large_button.height >= 38.0 && large_badge.height >= 32.0,
        "large button={large_button:?}, badge={large_badge:?}");
    assert!(large_button.height > large_badge.height);
}

#[gpui::test]
fn tabs_equal_layout_preserves_track_inset_and_controlled_value(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/tabs" as tabs;
fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn proposed(ctx,value){ctx.set_state("count",ctx.get_state("count")+1);}
fn view(ctx){column([
    text("Count").accessibility_role("status").accessibility_label(ctx.get_state("count").to_string()),
    tabs::Tabs(#{label:"Sections",value:"one",layout:"equal",tabs:[
        #{value:"one",label:"One",content:text("Panel one")},
        #{value:"two",label:"Two wider",content:text("Panel two")},
        #{value:"three",label:"Icon only",label_visible:false,
          icon:svg("<svg width='24' height='24' viewBox='0 0 24 24'><path d='M4 12H20' stroke='currentColor' stroke-width='2'/></svg>"),content:text("Panel three")}
    ],on_change:Fn("proposed")}).with_style(style().width(px(306)))
])}
"#;
    let (window, view) = mount(cx, script, "tabs-track");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let list = tree.find_by_role_and_name("tablist", "Sections").next().unwrap().geometry.unwrap().visual;
    let tabs = tree.nodes().filter(|node| node.role == "tab").collect::<Vec<_>>();
    assert_eq!(tabs.len(), 3);
    assert_eq!(list.height, 30.0);
    let bounds = tabs.iter().map(|tab| tab.geometry.unwrap().visual).collect::<Vec<_>>();
    assert_eq!(bounds[0].y - list.y, 2.0);
    assert_eq!(bounds[0].height, 26.0);
    assert_eq!(bounds[0].x - list.x, 2.0);
    assert!((list.x + list.width - (bounds[2].x + bounds[2].width) - 2.0).abs() < 0.01);
    assert!((bounds[0].width - bounds[1].width).abs() <= 0.5, "list={list:?}, tabs={bounds:?}");
    assert!((bounds[1].width - bounds[2].width).abs() <= 0.5, "list={list:?}, tabs={bounds:?}");
    assert_eq!(tabs[2].name, "Icon only");

    visual.simulate_click(
        point(px((bounds[1].x + bounds[1].width / 2.0) as f32), px((bounds[1].y + 13.0) as f32)),
        Modifiers::default(),
    );
    visual.run_until_parked();
    let after = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    assert_eq!(after.find_by_role_and_name("status", "1").count(), 1);
    let selected = after.nodes().filter(|node| node.role == "tab" && node.checked == Some(UiValue::Bool(true))).collect::<Vec<_>>();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].name, "One", "Host rejected the proposal, so selection must not move");
}

#[gpui::test]
fn tabs_content_and_vertical_layout_use_natural_and_consistent_slots(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/tabs" as tabs;
fn items(){[
    #{value:"a",label:"A",content:text("A panel")},
    #{value:"b",label:"Much longer label",content:text("B panel")},
    #{value:"c",label:"中文项目",content:text("C panel")}
]}
fn view(ctx){column([
    tabs::Tabs(#{label:"Content Sections",value:"a",layout:"content",tabs:items()})
        .with_style(style().width(px(420))),
    tabs::Tabs(#{label:"Vertical Sections",value:"c",orientation:"vertical",tabs:items()})
        .with_style(style().width(px(420)))
]).with_style(style().gap(px(12)))}
"#;
    let (window, view) = mount(cx, script, "tabs-natural");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let content_list = tree
        .find_by_role_and_name("tablist", "Content Sections")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    assert!(content_list.width < 420.0, "content layout stretched: {content_list:?}");
    let content_tabs = tree
        .nodes()
        .filter(|node| {
            node.role == "tab"
                && node.geometry.is_some_and(|geometry| {
                    geometry.visual.y >= content_list.y
                        && geometry.visual.y < content_list.y + content_list.height
                })
        })
        .map(|node| node.geometry.unwrap().visual)
        .collect::<Vec<_>>();
    assert_eq!(content_tabs.len(), 3);
    assert!(content_tabs[1].width > content_tabs[0].width + 40.0);

    let vertical_list = tree
        .find_by_role_and_name("tablist", "Vertical Sections")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let vertical_tabs = tree
        .nodes()
        .filter(|node| {
            node.role == "tab"
                && node.geometry.is_some_and(|geometry| {
                    geometry.visual.x >= vertical_list.x
                        && geometry.visual.x < vertical_list.x + vertical_list.width
                        && geometry.visual.y >= vertical_list.y
                        && geometry.visual.y < vertical_list.y + vertical_list.height
                })
        })
        .map(|node| node.geometry.unwrap().visual)
        .collect::<Vec<_>>();
    assert_eq!(vertical_tabs.len(), 3);
    assert!(vertical_tabs.iter().all(|tab| (tab.width - vertical_tabs[0].width).abs() < 0.01));
    assert_eq!(vertical_tabs[0].y - vertical_list.y, 2.0);
    let last = vertical_tabs.last().unwrap();
    assert!((vertical_list.y + vertical_list.height - (last.y + last.height) - 2.0).abs() < 0.01);
}

#[gpui::test]
fn animated_tabs_moves_one_shared_indicator_after_controlled_commit(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "motion/animated_tabs" as animated_tabs;
fn state_schema(){#{fields:#{value:#{schema:#{type:"string"},"default":#{type:"string",value:"one"}}}}}
fn changed(ctx,value){ctx.set_state("value",value);}
fn view(ctx){animated_tabs::AnimatedTabs(#{key:"visual-tabs",label:"Animated Sections",
    value:ctx.get_state("value"),layout:"equal",tabs:[
        #{value:"one",label:"One",content:text("Panel one")},
        #{value:"two",label:"Two",content:text("Panel two")},
        #{value:"three",label:"Three",content:text("Panel three")}
    ],on_change:Fn("changed")}).with_style(style().width(px(306)))}
"#;
    let (window, view) = mount(cx, script, "animated-tabs");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let second = tree
        .find_by_role_and_name("tab", "Two")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    visual.simulate_click(
        point(
            px((second.x + second.width / 2.0) as f32),
            px((second.y + second.height / 2.0) as f32),
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    let after = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let selected = after
        .nodes()
        .filter(|node| node.role == "tab" && node.checked == Some(UiValue::Bool(true)))
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].name, "Two");
}
