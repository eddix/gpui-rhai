use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, ScrollDelta, ScrollWheelEvent,
    TestAppContext, VisualTestContext, Window, WindowHandle, point, px, rgba,
};
use gpui_rhai::*;

const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const BADGE: &str = include_str!("../../../registry/components/badge.rhai");
const TABS: &str = include_str!("../../../registry/components/tabs.rhai");
const INPUT: &str = include_str!("../../../registry/components/input.rhai");
const INPUT_GROUP: &str = include_str!("../../../registry/components/input_group.rhai");
const CHECKBOX: &str = include_str!("../../../registry/components/checkbox.rhai");
const SPLIT_PANE: &str = include_str!("../../../registry/components/split_pane.rhai");
const RESIZABLE: &str = include_str!("../../../registry/components/resizable.rhai");
const DRAGGABLE: &str = include_str!("../../../registry/components/draggable.rhai");
const DRAG_SOURCE: &str = include_str!("../../../registry/components/drag_source.rhai");
const DROP_ZONE: &str = include_str!("../../../registry/components/drop_zone.rhai");
const SORTABLE: &str = include_str!("../../../registry/components/sortable.rhai");
const SCROLL_AREA: &str = include_str!("../../../registry/components/scroll_area.rhai");
const PAN_ZOOM: &str = include_str!("../../../registry/components/pan_zoom.rhai");
const RANGE_SLIDER: &str = include_str!("../../../registry/components/range_slider.rhai");
const SLIDER: &str = include_str!("../../../registry/components/slider.rhai");
const ROTATABLE: &str = include_str!("../../../registry/components/rotatable.rhai");
const SELECTION_AREA: &str = include_str!("../../../registry/components/selection_area.rhai");
const TREE: &str = include_str!("../../../registry/components/tree.rhai");
const INTERACTION_LAB: &str =
    include_str!("../../../registry/stories/workbench/interaction_lab.rhai");
const ANIMATED_TABS: &str = include_str!("../../../registry/motion/animated_tabs.rhai");
const DEFAULT_DARK: &str = include_str!("../../../registry/themes/default_dark.rhai");

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        if self.view.state() == gpui_rhai::ScriptViewState::Active {
            self.host.container(self.view.element().unwrap())
        } else {
            self.host.container(gpui::div())
        }
    }
}

fn mount(
    cx: &mut TestAppContext,
    script: &str,
    name: &str,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    mount_with_overrides(cx, script, name, ThemeTokenOverrides::default())
}

fn mount_with_overrides(
    cx: &mut TestAppContext,
    script: &str,
    name: &str,
    overrides: ThemeTokenOverrides,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (entry, script.to_owned()),
            (
                ModuleId::parse("components/button").unwrap(),
                BUTTON.to_owned(),
            ),
            (
                ModuleId::parse("components/badge").unwrap(),
                BADGE.to_owned(),
            ),
            (ModuleId::parse("components/tabs").unwrap(), TABS.to_owned()),
            (
                ModuleId::parse("components/input").unwrap(),
                INPUT.to_owned(),
            ),
            (
                ModuleId::parse("components/input_group").unwrap(),
                INPUT_GROUP.to_owned(),
            ),
            (
                ModuleId::parse("components/checkbox").unwrap(),
                CHECKBOX.to_owned(),
            ),
            (
                ModuleId::parse("components/split_pane").unwrap(),
                SPLIT_PANE.to_owned(),
            ),
            (
                ModuleId::parse("components/resizable").unwrap(),
                RESIZABLE.to_owned(),
            ),
            (
                ModuleId::parse("components/draggable").unwrap(),
                DRAGGABLE.to_owned(),
            ),
            (
                ModuleId::parse("components/drag_source").unwrap(),
                DRAG_SOURCE.to_owned(),
            ),
            (
                ModuleId::parse("components/drop_zone").unwrap(),
                DROP_ZONE.to_owned(),
            ),
            (
                ModuleId::parse("components/sortable").unwrap(),
                SORTABLE.to_owned(),
            ),
            (
                ModuleId::parse("components/scroll_area").unwrap(),
                SCROLL_AREA.to_owned(),
            ),
            (
                ModuleId::parse("components/pan_zoom").unwrap(),
                PAN_ZOOM.to_owned(),
            ),
            (
                ModuleId::parse("components/range_slider").unwrap(),
                RANGE_SLIDER.to_owned(),
            ),
            (
                ModuleId::parse("components/slider").unwrap(),
                SLIDER.to_owned(),
            ),
            (
                ModuleId::parse("components/rotatable").unwrap(),
                ROTATABLE.to_owned(),
            ),
            (
                ModuleId::parse("components/selection_area").unwrap(),
                SELECTION_AREA.to_owned(),
            ),
            (ModuleId::parse("components/tree").unwrap(), TREE.to_owned()),
            (
                ModuleId::parse("motion/animated_tabs").unwrap(),
                ANIMATED_TABS.to_owned(),
            ),
        ])),
        DEFAULT_DARK,
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .asset_sources([
        (
            "icons/chevron_down".to_owned(),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: include_bytes!("../../../registry/assets/icons/chevron_down.svg").to_vec(),
            },
        ),
        (
            "icons/chevron_right".to_owned(),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: include_bytes!("../../../registry/assets/icons/chevron_right.svg").to_vec(),
            },
        ),
        (
            "icons/check".to_owned(),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: include_bytes!("../../../registry/assets/icons/check.svg").to_vec(),
            },
        ),
        (
            "icons/minus".to_owned(),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: include_bytes!("../../../registry/assets/icons/minus.svg").to_vec(),
            },
        ),
    ])
    .theme_token_overrides(overrides)
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
fn tabs_track_and_slot_follow_theme_spacing_and_typography(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/tabs" as tabs;
fn view(ctx){tabs::Tabs(#{label:"Sections",value:"one",tabs:[
    #{value:"one",label:"One",content:text("Panel one")},
    #{value:"two",label:"Two",content:text("Panel two")}
]})}
"#;
    let overrides = ThemeTokenOverrides {
        spacing: BTreeMap::from([("xxs".to_owned(), Length::Pixels(8.0))]),
        typography: ThemeTypographyOverrides {
            // Tab labels use the control role; overriding its line box grows the slot.
            roles: BTreeMap::from([(
                "control_regular".to_owned(),
                TypographyToken::new(Length::Pixels(24.0), Length::Pixels(36.0), 400),
            )]),
            ..Default::default()
        },
        ..Default::default()
    };
    let (window, view) = mount_with_overrides(cx, script, "tabs-theme-geometry", overrides);
    let mut visual = VisualTestContext::from_window(*window, cx);
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let list = tree
        .find_by_role_and_name("tablist", "Sections")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let tab = tree
        .find_by_role_and_name("tab", "One")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    assert!(tab.height >= 36.0, "tab={tab:?}");
    assert!(
        (tab.y - list.y - 8.0).abs() < 0.01,
        "list={list:?}, tab={tab:?}"
    );
    assert!((list.y + list.height - tab.y - tab.height - 8.0).abs() < 0.01);
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
    let button = tree
        .find_by_role_and_name("button", "Density")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let badge = tree
        .find_by_role_and_name("status", "Density")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    // A md Button is metrics.control; a Badge is a 20px marker in every density.
    assert_eq!(button.height, 32.0);
    assert_eq!(badge.height, 20.0);
    assert!(
        button.width >= badge.width + 12.0,
        "button={button:?}, badge={badge:?}"
    );
    let cjk_button = tree
        .find_by_role_and_name("button", "中文")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let cjk_badge = tree
        .find_by_role_and_name("status", "中文")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    assert_eq!(cjk_button.height, 24.0);
    assert_eq!(cjk_badge.height, 18.0);
    let large_button = tree
        .find_by_role_and_name("button", "Large line")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let large_badge = tree
        .find_by_role_and_name("status", "Large line")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    // Minimum heights are lower bounds: larger type grows both markers.
    assert!(
        large_button.height >= 34.0 && large_badge.height >= 30.0,
        "large button={large_button:?}, badge={large_badge:?}"
    );
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
    let list = tree
        .find_by_role_and_name("tablist", "Sections")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let tabs = tree
        .nodes()
        .filter(|node| node.role == "tab")
        .collect::<Vec<_>>();
    assert_eq!(tabs.len(), 3);
    assert_eq!(list.height, 32.0);
    let bounds = tabs
        .iter()
        .map(|tab| tab.geometry.unwrap().visual)
        .collect::<Vec<_>>();
    assert_eq!(bounds[0].y - list.y, 2.0);
    assert_eq!(bounds[0].height, 28.0);
    assert_eq!(bounds[0].x - list.x, 2.0);
    assert!((list.x + list.width - (bounds[2].x + bounds[2].width) - 2.0).abs() < 0.01);
    assert!(
        (bounds[0].width - bounds[1].width).abs() <= 0.5,
        "list={list:?}, tabs={bounds:?}"
    );
    assert!(
        (bounds[1].width - bounds[2].width).abs() <= 0.5,
        "list={list:?}, tabs={bounds:?}"
    );
    assert_eq!(tabs[2].name, "Icon only");

    visual.simulate_click(
        point(
            px((bounds[1].x + bounds[1].width / 2.0) as f32),
            px((bounds[1].y + 13.0) as f32),
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();
    let after = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    assert_eq!(after.find_by_role_and_name("status", "1").count(), 1);
    let selected = after
        .nodes()
        .filter(|node| node.role == "tab" && node.selected == Some(true))
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 1);
    assert_eq!(
        selected[0].name, "One",
        "Host rejected the proposal, so selection must not move"
    );
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
    assert!(
        content_list.width < 420.0,
        "content layout stretched: {content_list:?}"
    );
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
    assert!(
        vertical_tabs
            .iter()
            .all(|tab| (tab.width - vertical_tabs[0].width).abs() < 0.01)
    );
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
        .filter(|node| node.role == "tab" && node.selected == Some(true))
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].name, "Two");
}

#[gpui::test]
fn single_axis_script_scroll_does_not_translate_the_other_axis(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
fn view(ctx) { box([
    text("Scroll marker").accessibility_label("Scroll marker")
        .with_style(style().width(px(1000)).height(px(40)))
]).with_style(style().width(px(200)).height(px(80)).overflow_x_scroll()) }
"#;
    let (window, view) = mount(cx, script, "single-axis-scroll");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let marker_x = |visual: &mut VisualTestContext| {
        visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("text", "Scroll marker")
                .next()
                .unwrap()
                .geometry
                .unwrap()
                .visual
                .x
        })
    };
    let before = marker_x(&mut visual);
    let position = point(px(40.), px(30.));
    visual.simulate_mouse_move(position, None, Modifiers::none());
    visual.simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Pixels(point(px(0.), px(-45.))),
        ..Default::default()
    });
    visual.run_until_parked();
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    assert_eq!(marker_x(&mut visual), before);
}

/// Quads painted with a visible border in `color`, as (width, height) in scaled pixels.
fn bordered_quads(visual: &mut VisualTestContext, color: u32) -> Vec<(f32, f32)> {
    let color: gpui::Hsla = rgba(color).into();
    visual.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .filter(|quad| quad.border_widths.left.0 > 0.0 && quad.border_color == color)
            .map(|quad| (quad.bounds.size.width.0, quad.bounds.size.height.0))
            .collect()
    })
}

#[gpui::test]
fn compound_controls_show_focus_on_their_mark_and_frame(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/checkbox" as checkbox;
import "components/input" as input;
import "components/input_group" as input_group;
fn state_schema() { #{ fields: #{ value: #{ schema: #{ type: "string" },
    "default": #{ type: "string", value: "host" } } } } }
fn changed(ctx, value) { ctx.set_state("value", value); }
fn toggled(ctx, value) {}
fn view(ctx) { column([
    checkbox::Checkbox(#{ checked: false, label: "Audit box", on_change: Fn("toggled") }),
    input_group::InputGroup(#{ label: "Audit group", prefix: text("https://"),
        control: input::Input(#{ key: "grouped", label: "Grouped input",
            value: ctx.get_state("value"), on_change: Fn("changed") }) })
        .with_style(style().width(px(260))),
]).with_style(style().gap(px(12)).padding(px(12))) }
"#;
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([("focus_ring".to_owned(), Rgba8::from_rgba_hex(0x00ff00ff))]),
        ..Default::default()
    };
    let (window, view) = mount_with_overrides(cx, script, "compound-focus", overrides);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    assert!(
        bordered_quads(&mut visual, 0x00ff00ff).is_empty(),
        "nothing is focused yet"
    );

    // Keyboard focus on the Checkbox row paints the ink frame on its 16px box only
    // (group_focus), never around the row, so the box stays on the content edge.
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let row = tree
        .find_by_role_and_name("checkbox", "Audit box")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    visual.update(|window, cx| view.focus(window, cx)).unwrap();
    visual.simulate_keystrokes("tab");
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    let scale = visual.update(|window, _| window.scale_factor());
    let focused = bordered_quads(&mut visual, 0x00ff00ff);
    assert!(!focused.is_empty(), "keyboard focus must paint a frame");
    assert!(
        focused.iter().all(|(width, height)| {
            (width / scale - 16.0).abs() < 0.5 && (height / scale - 16.0).abs() < 0.5
        }),
        "focus frame must be the box, not the row {row:?}: {focused:?}"
    );

    // Focus inside the native input paints the InputGroup frame (focus_within).
    let group = tree
        .find_by_role_and_name("group", "Audit group")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    visual.simulate_click(
        point(
            px((group.x + group.width - 30.) as f32),
            px((group.y + group.height / 2.) as f32),
        ),
        Modifiers::none(),
    );
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    let framed = bordered_quads(&mut visual, 0x00ff00ff);
    assert!(
        framed
            .iter()
            .any(|(width, _)| (width / scale - group.width as f32).abs() < 1.0),
        "the group frame must take the focus color: {framed:?}, group={group:?}"
    );
}

#[gpui::test]
fn input_wrapper_paints_focus_ring_for_the_native_editor_focus(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/input" as input;
fn state_schema() { #{ fields: #{ value: #{ schema: #{ type: "string" },
    "default": #{ type: "string", value: "initial" } } } } }
fn changed(ctx, value) { ctx.set_state("value", value); }
fn view(ctx) { input::Input(#{key:"field",label:"Audit input",value:ctx.get_state("value"),
    on_change:Fn("changed")}).with_style(style().width(px(220))) }
"#;
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([
            ("border".to_owned(), Rgba8::from_rgba_hex(0xff0000ff)),
            ("focus_ring".to_owned(), Rgba8::from_rgba_hex(0x00ff00ff)),
        ]),
        ..Default::default()
    };
    let (window, view) = mount_with_overrides(cx, script, "native-input-focus", overrides);
    let mut visual = VisualTestContext::from_window(*window, cx);
    let bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("text_field", "Audit input")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    visual.simulate_click(
        point(
            px((bounds.x + 30.) as f32),
            px((bounds.y + bounds.height / 2.) as f32),
        ),
        Modifiers::none(),
    );
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("focused and editable");
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    let focus_ring: gpui::Hsla = rgba(0x00ff00ff).into();
    let green_borders = visual.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .filter(|quad| quad.border_widths.left.0 > 0.0 && quad.border_color == focus_ring)
            .count()
    });
    assert!(
        green_borders > 0,
        "native focus did not reach wrapper style"
    );
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    assert_eq!(
        tree.find_by_role_and_name("text_field", "Audit input")
            .next()
            .unwrap()
            .value,
        Some(UiValue::String("focused and editable".to_owned()))
    );
}

#[gpui::test]
fn split_pane_keyboard_step_commits_one_controlled_ratio(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/split_pane" as split_pane;
fn state_schema(){#{fields:#{size:#{schema:#{type:"number",min:0.0,max:1.0},
    "default":#{type:"float",value:0.5}}}}}
fn resized(ctx,value){ctx.set_state("size",value);}
fn view(ctx){column([
    text(ctx.get_state("size").to_string()).accessibility_role("status"),
    split_pane::SplitPane(#{key:"layout",label:"Resize panels",size:ctx.get_state("size"),
        min_start:80.0,min_end:80.0,start:text("Start"),end:text("End"),on_resize:Fn("resized")})
        .with_style(style().width(px(420)).height(px(180)))
])}
"#;
    let (window, view) = mount(cx, script, "split-pane-keyboard");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let before = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.role == "status")
            .unwrap()
            .name
            .clone()
    });
    visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Dispatch {
                    locator: AutomationLocator::RoleName {
                        role: "separator".to_owned(),
                        name: "Resize panels".to_owned(),
                    },
                    event: "key:right".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    visual.run_until_parked();
    let after = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.role == "status")
            .unwrap()
            .name
            .clone()
    });
    assert_ne!(after, before);
    assert!(after.parse::<f64>().unwrap() > 0.5, "{after}");
}

#[gpui::test]
fn split_pane_drag_previews_natively_and_commits_only_on_release(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/split_pane" as split_pane;
fn state_schema(){#{fields:#{size:#{schema:#{type:"number",min:0.0,max:1.0},
    "default":#{type:"float",value:0.5}}}}}
fn resized(ctx,value){ctx.set_state("size",value);}
fn view(ctx){column([
    text(ctx.get_state("size").to_string()).accessibility_role("status"),
    split_pane::SplitPane(#{key:"layout",label:"Resize panels",size:ctx.get_state("size"),
        min_start:80.0,min_end:80.0,start:text("Start"),end:text("End"),on_resize:Fn("resized")})
        .with_style(style().width(px(420)).height(px(180)))
])}
"#;
    let (window, view) = mount(cx, script, "split-pane-pointer");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("separator", "Resize panels")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let status = |visual: &mut VisualTestContext| {
        visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|node| node.role == "status")
                .unwrap()
                .name
                .clone()
        })
    };
    let start = point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    );
    let end = point(start.x + px(72.0), start.y);
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert_eq!(status(&mut visual), "0.5", "pointer move reran Rhai state");
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    let committed = status(&mut visual).parse::<f64>().unwrap();
    assert!(committed > 0.5, "committed ratio={committed}");
}

fn resizable_script(handles: &str) -> String {
    format!(
        r#"
import "components/resizable" as resizable;
fn state_schema(){{#{{fields:#{{
    rect:#{{schema:#{{type:"object",allow_unknown:false,fields:#{{
        x:#{{schema:#{{type:"number"}},required:true,sensitive:false}},
        y:#{{schema:#{{type:"number"}},required:true,sensitive:false}},
        width:#{{schema:#{{type:"number",exclusive_min:0.0}},required:true,sensitive:false}},
        height:#{{schema:#{{type:"number",exclusive_min:0.0}},required:true,sensitive:false}}
    }}}},"default":#{{type:"map",value:#{{x:#{{type:"float",value:100.0}},y:#{{type:"float",value:80.0}},
        width:#{{type:"float",value:200.0}},height:#{{type:"float",value:120.0}}}}}}}},
    proposals:#{{schema:#{{type:"integer"}},"default":#{{type:"integer",value:0}}}}
}}}}}}
fn resized(ctx,value){{ctx.set_state("rect",value);ctx.set_state("proposals",ctx.get_state("proposals")+1);}}
fn view(ctx){{let rect=ctx.get_state("rect");column([
    text(`${{rect.x}},${{rect.y}},${{rect.width}},${{rect.height}},${{ctx.get_state("proposals")}}`).accessibility_role("status"),
    resizable::Resizable(#{{key:"card",label:"Demo",rect:rect,handles:{handles},
        min_width:80.0,min_height:60.0,max_width:360.0,max_height:260.0,
        content:text("Card"),on_resize:Fn("resized")}})
        .with_style(style().width(px(500)).height(px(400)))
])}}
"#
    )
}

#[gpui::test]
fn draggable_previews_natively_and_commits_once_from_the_declared_handle(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/draggable" as draggable;
fn state_schema(){
    #{ fields: #{
        position: #{
            schema: #{ type:"object", allow_unknown:false, fields: #{
                x: #{ schema:#{type:"number"}, required:true, sensitive:false },
                y: #{ schema:#{type:"number"}, required:true, sensitive:false }
            } },
            "default": #{ type:"map", value: #{
                x: #{type:"float",value:40.0}, y: #{type:"float",value:50.0}
            } }
        }
    } }
}
fn moved(ctx,value){ctx.set_state("position",value);}
fn view(ctx){
    let position=ctx.get_state("position");
    column([
        text(`${position.x},${position.y}`).accessibility_role("status"),
        draggable::Draggable(#{key:"card",label:"Move card",position:position,
            handle:text("Drag handle").with_style(style().height(px(36)).background(theme_color("surface_hover"))),
            content:text("Body").with_style(style().width(px(180)).height(px(64)).background(theme_color("surface_raised"))),
            on_move:Fn("moved")
        }).with_style(style().width(px(420)).height(px(280)))
    ])
}
"#;
    let (window, view) = mount(cx, script, "draggable-pointer");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let snapshot = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let handle = snapshot
        .nodes()
        .find(|node| node.name == "Drag handle")
        .and_then(|node| node.geometry)
        .unwrap()
        .visual;
    let surface = snapshot
        .find_by_role_and_name("group", "Move card")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let start = point(
        px((handle.x + handle.width / 2.0) as f32),
        px((handle.y + handle.height / 2.0) as f32),
    );
    let end = point(start.x + px(48.0), start.y + px(32.0));
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    let preview = visual.update(|_, cx| {
        let tree = view.accessibility_snapshot(cx).unwrap();
        assert_eq!(
            tree.nodes()
                .find(|node| node.role == "status")
                .unwrap()
                .name,
            "40.0,50.0"
        );
        tree.find_by_role_and_name("group", "Move card")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    assert!((preview.x - surface.x - 48.0).abs() < 0.01, "{preview:?}");
    assert!((preview.y - surface.y - 32.0).abs() < 0.01, "{preview:?}");
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    let status = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.role == "status")
            .unwrap()
            .name
            .clone()
    });
    assert_eq!(status, "88.0,82.0");
}

#[gpui::test]
fn typed_drag_drop_commits_one_target_and_keyboard_uses_the_same_contract(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
fn state_schema(){#{fields:#{
    owner:#{schema:#{type:"string",allowed:["a","b"]},"default":#{type:"string",value:"a"}},
    status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}
}}}
fn dropped(ctx,value){ctx.set_state("owner",value.target_id);ctx.set_state("status",`drop:${value.target_id}:${value.payload.id}`);}
fn ended(ctx,value){if value.cancelled{ctx.set_state("status","cancelled");}
    else if !value.accepted{ctx.set_state("status","keyboard rejected");}}
fn card(ctx){let other=if ctx.get_state("owner")=="a"{"b"}else{"a"};
    drag_source::DragSource(#{key:"card",label:"Move item",source_id:"item-1",payload_type:"item",
        payload:#{id:"item-1"},operation:"move",keyboard_target:other,
        content:text("ITEM ONE").with_style(style().width(px(120)).height(px(44))
            .items_center().padding_x(px(8)).background(theme_color("surface_raised"))),on_drag_end:Fn("ended")})}
fn lane(ctx,id){let children=[text(`TARGET ${id}`)];if ctx.get_state("owner")==id{children.push(card(ctx));}
    drop_zone::DropZone(#{key:`zone-${id}`,label:`Target ${id}`,target_id:id,payload_types:["item"],operations:["move"],
        content:column(children).with_style(style().width(relative(1.0)).height(relative(1.0)).padding(px(12)).gap(px(8))),
        on_drop:Fn("dropped")}).with_style(style().width(px(190)).height(px(150)).border(px(1)).border_color(theme_color("border")))}
fn view(ctx){column([text(ctx.get_state("status")).accessibility_role("status"),
    row([lane(ctx,"a"),lane(ctx,"b")]).with_style(style().gap(px(20)))])}
"#;
    let (window, view) = mount(cx, script, "typed-drag-drop");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let status = |visual: &mut VisualTestContext| {
        visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|node| node.role == "status")
                .unwrap()
                .name
                .clone()
        })
    };
    let source = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("button", "Move item")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let source_point = point(
        px((source.x + source.width / 2.0) as f32),
        px((source.y + source.height / 2.0) as f32),
    );
    visual.simulate_mouse_down(source_point, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(source_point, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    visual.simulate_keystrokes("enter");
    visual.run_until_parked();
    assert_eq!(status(&mut visual), "drop:b:item-1");

    let snapshot = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let source = snapshot
        .find_by_role_and_name("button", "Move item")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let target = snapshot
        .find_by_role_and_name("group", "Target a")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let source_point = point(
        px((source.x + source.width / 2.0) as f32),
        px((source.y + source.height / 2.0) as f32),
    );
    let target_point = point(
        px((target.x + target.width / 2.0) as f32),
        px((target.y + target.height / 2.0) as f32),
    );
    visual.simulate_mouse_down(source_point, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(target_point, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(target_point, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert_eq!(status(&mut visual), "drop:a:item-1");
}

#[gpui::test]
fn sortable_proposes_stable_anchors_for_pointer_and_keyboard(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/sortable" as sortable;
fn state_schema(){#{fields:#{
    order:#{schema:#{type:"array",max_items:8,items:#{type:"string"}},"default":#{type:"array",value:[
        #{type:"string",value:"alpha"},#{type:"string",value:"bravo"},
        #{type:"string",value:"charlie"},#{type:"string",value:"delta"}]}},
    status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}
}}}
fn index_of(values,key){for index in 0..values.len{if values[index]==key{return index;}}-1}
fn reordered(ctx,value){let order=ctx.get_state("order");let source=index_of(order,value.source_key);
    let moved=order.remove(source);let anchor=index_of(order,value.anchor_key);
    order.insert(if value.placement=="after"{anchor+1}else{anchor},moved);
    ctx.set_state("order",order);ctx.set_state("status",`${value.source_key}:${value.placement}:${value.anchor_key}`);}
fn view(ctx){let items=[];for key in ctx.get_state("order"){
    items.push(#{key:key,label:key,content:text(key).with_style(style().height(px(48)).padding(px(8)).items_center())});}
    column([text(ctx.get_state("status")).accessibility_role("status"),
        sortable::Sortable(#{key:"queue",label:"Queue",items:items,direction:"vertical",on_reorder:Fn("reordered")})
    ]).with_style(style().width(px(280)).padding(px(12)).gap(px(8)))}
"#;
    let (window, view) = mount(cx, script, "sortable");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let status = |visual: &mut VisualTestContext| {
        visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|node| node.role == "status")
                .unwrap()
                .name
                .clone()
        })
    };
    let snapshot = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let source = snapshot
        .find_by_role_and_name("button", "Reorder alpha")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let target = snapshot
        .find_by_role_and_name("listitem", "charlie")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let source_point = point(
        px((source.x + source.width / 2.0) as f32),
        px((source.y + source.height / 2.0) as f32),
    );
    let target_after = point(
        px((target.x + target.width / 2.0) as f32),
        px((target.y + target.height * 0.75) as f32),
    );
    visual.simulate_mouse_down(source_point, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(target_after, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(target_after, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert_eq!(status(&mut visual), "alpha:after:charlie");

    let source = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("button", "Reorder alpha")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let source_point = point(
        px((source.x + source.width / 2.0) as f32),
        px((source.y + source.height / 2.0) as f32),
    );
    visual.simulate_mouse_down(source_point, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(source_point, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    visual.simulate_keystrokes("alt-up");
    visual.run_until_parked();
    assert_eq!(status(&mut visual), "alpha:before:charlie");
}

#[gpui::test]
fn sortable_drag_auto_scrolls_the_active_drop_container(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/sortable" as sortable;
import "components/scroll_area" as scroll_area;
fn view(ctx){let items=[];for index in 0..12{let key=`item-${index}`;
    items.push(#{key:key,label:key,content:text(key).with_style(style().height(px(42)).padding(px(8)).items_center())});}
    let list=sortable::Sortable(#{key:"scroll-order",label:"Scroll order",items:items,direction:"vertical"});
    scroll_area::ScrollArea(#{key:"scroll",label:"Sortable viewport",height:px(180),axis:"vertical",content:list})
        .with_style(style().width(px(300)).margin(px(12)))}
"#;
    let (window, view) = mount(cx, script, "sortable-scroll");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let snapshot = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let source = snapshot
        .find_by_role_and_name("button", "Reorder item-0")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let viewport = snapshot
        .find_by_role_and_name("region", "Sortable viewport")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let before = snapshot
        .find_by_role_and_name("listitem", "item-11")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual
        .y;
    let source_point = point(
        px((source.x + source.width / 2.0) as f32),
        px((source.y + source.height / 2.0) as f32),
    );
    let bottom = viewport.y + viewport.height - 5.0;
    visual.simulate_mouse_down(source_point, MouseButton::Left, Modifiers::default());
    for step in 0..10 {
        visual.simulate_mouse_move(
            point(
                px((viewport.x + viewport.width / 2.0 + f64::from(step % 2)) as f32),
                px(bottom as f32),
            ),
            MouseButton::Left,
            Modifiers::default(),
        );
        visual.run_until_parked();
    }
    let after = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("listitem", "item-11")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
            .y
    });
    visual.simulate_mouse_up(
        point(
            px((viewport.x + viewport.width / 2.0) as f32),
            px(bottom as f32),
        ),
        MouseButton::Left,
        Modifiers::default(),
    );
    visual.run_until_parked();
    assert!(
        after < before - 24.0,
        "expected auto-scroll: {before} -> {after}"
    );
}

#[gpui::test]
fn virtual_sortable_pins_the_dragged_key_while_realization_moves(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/sortable" as sortable;
fn state_schema(){#{fields:#{status:#{schema:#{type:"string"},"default":#{type:"string",value:"ready"}}}}}
fn reordered(ctx,value){ctx.set_state("status",`${value.source_key}:${value.placement}:${value.anchor_key}`);}
fn view(ctx){let data=[];for index in 0..100{data.push(#{key:`item-${index}`,label:`Item ${index}`});}
    column([text(ctx.get_state("status")).accessibility_role("status"),
        sortable::Sortable(#{key:"virtual-order",label:"Virtual order",virtual_data:data,
            direction:"vertical",height:180.0,estimated_row_height:40.0,on_reorder:Fn("reordered")})
    ]).with_style(style().width(px(300)).padding(px(12)).gap(px(8)))}
"#;
    let (window, view) = mount(cx, script, "virtual-sortable");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let snapshot = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let source = snapshot
        .find_by_role_and_name("button", "Reorder Item 0")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let list = snapshot
        .find_by_role_and_name("list", "Virtual order")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let source_point = point(
        px((source.x + source.width / 2.0) as f32),
        px((source.y + source.height / 2.0) as f32),
    );
    let bottom = point(
        px((list.x + list.width / 2.0) as f32),
        px((list.y + list.height - 5.0) as f32),
    );
    visual.simulate_mouse_down(source_point, MouseButton::Left, Modifiers::default());
    for step in 0..30 {
        visual.simulate_mouse_move(
            point(bottom.x + px((step % 2) as f32), bottom.y),
            MouseButton::Left,
            Modifiers::default(),
        );
        visual.run_until_parked();
    }
    visual.simulate_mouse_up(bottom, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    let status = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.role == "status")
            .unwrap()
            .name
            .clone()
    });
    assert!(status.starts_with("item-0:"), "status={status}");
}

#[gpui::test]
fn pan_zoom_previews_pan_and_commits_pointer_anchored_zoom(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/pan_zoom" as pan_zoom;
fn state_schema(){#{fields:#{
    transform:#{schema:#{type:"object",allow_unknown:false,fields:#{
        x:#{schema:#{type:"number"},required:true,sensitive:false},
        y:#{schema:#{type:"number"},required:true,sensitive:false},
        scale:#{schema:#{type:"number",exclusive_min:0.0},required:true,sensitive:false}
    }},"default":#{type:"map",value:#{
        x:#{type:"float",value:0.0},y:#{type:"float",value:0.0},scale:#{type:"float",value:1.0}
    }}},
    commits:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0}}
}}}
fn changed(ctx,value){ctx.set_state("transform",value);ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){let value=ctx.get_state("transform");column([
    text(`${value.x},${value.y},${value.scale},${ctx.get_state("commits")}`).accessibility_role("status"),
    pan_zoom::PanZoom(#{key:"viewport",label:"Canvas viewport",transform:value,min_scale:0.5,max_scale:4.0,
        wheel_zoom:"always",content:canvas(canvas_scene([
            canvas_rect("target",80.0,50.0,40.0,30.0,theme_color("accent"))
        ])).with_key("canvas"),on_transform_change:Fn("changed")})
        .with_style(style().width(px(300)).height(px(180)))
]).with_style(style().padding(px(12)).gap(px(8)))}
"#;
    let (window, view) = mount(cx, script, "pan-zoom");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let status = |visual: &mut VisualTestContext| {
        visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|node| node.role == "status")
                .unwrap()
                .name
                .clone()
        })
    };
    let viewport = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("region", "Canvas viewport")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let start = point(
        px((viewport.x + viewport.width / 2.0) as f32),
        px((viewport.y + viewport.height / 2.0) as f32),
    );
    let end = point(start.x + px(40.0), start.y + px(20.0));
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert_eq!(status(&mut visual), "40.0,20.0,1.0,1");

    visual.simulate_event(ScrollWheelEvent {
        position: start,
        delta: ScrollDelta::Lines(point(0.0, -4.0)),
        ..Default::default()
    });
    visual.run_until_parked();
    let zoomed = status(&mut visual);
    let scale = zoomed.split(',').nth(2).unwrap().parse::<f64>().unwrap();
    assert!(scale > 1.0, "status={zoomed}");
    assert!(zoomed.ends_with(",2"), "status={zoomed}");

    for _ in 0..3 {
        visual.simulate_event(ScrollWheelEvent {
            position: start,
            delta: ScrollDelta::Pixels(point(px(0.0), px(-12.0))),
            ..Default::default()
        });
    }
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(100));
    visual.run_until_parked();
    cx.refresh().unwrap();
    visual.run_until_parked();
    let precise = status(&mut visual);
    assert!(precise.ends_with(",3"), "status={precise}");

    for (phase, delta) in [
        (gpui::TouchPhase::Started, -10.0),
        (gpui::TouchPhase::Moved, -10.0),
        (gpui::TouchPhase::Ended, 0.0),
    ] {
        visual.simulate_event(ScrollWheelEvent {
            position: start,
            delta: ScrollDelta::Pixels(point(px(0.0), px(delta))),
            touch_phase: phase,
            ..Default::default()
        });
    }
    visual.run_until_parked();
    let explicit = status(&mut visual);
    assert!(explicit.ends_with(",4"), "status={explicit}");
    let explicit_x = explicit.split(',').next().unwrap().parse::<f64>().unwrap();
    assert!(
        explicit_x.abs() > 1.0,
        "explicit gesture lost pan: {explicit}"
    );

    visual.simulate_keystrokes("tab right");
    visual.run_until_parked();
    let keyboard = status(&mut visual);
    assert!(keyboard.ends_with(",5"), "status={keyboard}");
}

#[gpui::test]
fn range_slider_has_two_keyboard_thumbs_and_one_pointer_commit(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/range_slider" as range_slider;
fn state_schema(){#{fields:#{range:#{schema:#{type:"object",allow_unknown:false,fields:#{
    low:#{schema:#{type:"number"},required:true,sensitive:false},
    high:#{schema:#{type:"number"},required:true,sensitive:false}}},
    "default":#{type:"map",value:#{low:#{type:"float",value:20.0},high:#{type:"float",value:80.0}}}},
    commits:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0}}}}}
fn changed(ctx,value){ctx.set_state("range",value);ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){let value=ctx.get_state("range");column([
    text(`${value.low},${value.high},${ctx.get_state("commits")}`).accessibility_role("status"),
    range_slider::RangeSlider(#{key:"range",label:"Accepted interval",low_label:"Low bound",
        high_label:"High bound",values:value,min:0.0,max:100.0,step:5.0,minimum_gap:10.0,
        on_change:Fn("changed")})
]).with_style(style().padding(px(12)).gap(px(8)))}
"#;
    let (window, view) = mount(cx, script, "range-slider");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let status = |visual: &mut VisualTestContext| {
        visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|node| node.role == "status")
                .unwrap()
                .name
                .clone()
        })
    };
    let snapshot = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let control = snapshot
        .find_by_role_and_name("group", "Accepted interval")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let start = point(
        px((control.x + control.width * 0.2) as f32),
        px((control.y + control.height * 0.7) as f32),
    );
    let target = point(
        px((control.x + control.width * 0.5) as f32),
        px((control.y + control.height * 0.7) as f32),
    );
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert!(status(&mut visual).ends_with(",1"));

    let control = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("group", "Accepted interval")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let high_point = point(
        px((control.x + control.width * 0.8) as f32),
        px((control.y + control.height * 0.7) as f32),
    );
    visual.simulate_mouse_down(high_point, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(high_point, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    visual.simulate_keystrokes("left");
    visual.run_until_parked();
    let keyboard = status(&mut visual);
    assert_eq!(keyboard, "50.0,75.0,3", "status={keyboard}");
}

#[gpui::test]
fn rotatable_keeps_an_explicit_pivot_and_commits_pointer_and_keyboard(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/rotatable" as rotatable;
fn state_schema(){#{fields:#{
    angle:#{schema:#{type:"number"},"default":#{type:"float",value:0.0}},
    commits:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0}}
}}}
fn changed(ctx,value){ctx.set_state("angle",value);ctx.set_state("commits",ctx.get_state("commits")+1);}
fn view(ctx){column([
    text(`${ctx.get_state("angle")},${ctx.get_state("commits")}`).accessibility_role("status"),
    rotatable::Rotatable(#{key:"arm",label:"Rotate arm",angle:ctx.get_state("angle"),
        pivot:#{x:100.0,y:100.0},snap:15.0,handle:text("R"),
        content:canvas(canvas_scene([canvas_rect("arm",100.0,90.0,80.0,20.0,theme_color("accent"))]))
            .with_key("arm-canvas"),on_rotate:Fn("changed")})
        .with_style(style().width(px(300)).height(px(220)))
]).with_style(style().padding(px(12)).gap(px(8)))}
"#;
    let (window, view) = mount(cx, script, "rotatable");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let status = |visual: &mut VisualTestContext| {
        visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|node| node.role == "status")
                .unwrap()
                .name
                .clone()
        })
    };
    let handle = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("slider", "Rotate arm")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let start = point(
        px((handle.x + handle.width / 2.0) as f32),
        px((handle.y + handle.height / 2.0) as f32),
    );
    let target = point(start.x - px(80.0), start.y + px(120.0));
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    let pointer = status(&mut visual);
    assert!(pointer.ends_with(",1"), "status={pointer}");

    visual.simulate_keystrokes("tab right");
    visual.run_until_parked();
    let keyboard = status(&mut visual);
    assert!(keyboard.ends_with(",2"), "status={keyboard}");
}

#[gpui::test]
fn selection_area_shares_click_range_marquee_and_keyboard_state(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/selection_area" as selection_area;
fn state_schema(){#{fields:#{
    selected:#{schema:#{type:"array",max_items:8,items:#{type:"string"}},"default":#{type:"array",value:[]}},
    active:#{schema:#{type:"optional",value:#{type:"string"}},"default":#{type:"null"}},
    anchor:#{schema:#{type:"optional",value:#{type:"string"}},"default":#{type:"null"}},
    commits:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0}}
}}}
fn changed(ctx,value){ctx.set_state("selected",value.selected_keys);ctx.set_state("active",value.active_key);
    ctx.set_state("anchor",value.anchor_key);ctx.set_state("commits",ctx.get_state("commits")+1);}
fn selected_text(values){let result="";for index in 0..values.len{if index>0{result+= "|";}result+=values[index];}result}
fn view(ctx){let targets=[#{key:"a",x:20.0,y:20.0,width:50.0,height:40.0},
    #{key:"b",x:100.0,y:20.0,width:50.0,height:40.0},#{key:"c",x:180.0,y:20.0,width:50.0,height:40.0}];
    column([text(`${selected_text(ctx.get_state("selected"))},${ctx.get_state("commits")}`).accessibility_role("status"),
        selection_area::SelectionArea(#{key:"area",label:"Node selection",targets:targets,
            selected_keys:ctx.get_state("selected"),active_key:ctx.get_state("active"),anchor_key:ctx.get_state("anchor"),
            content:canvas(canvas_scene([canvas_rect("a",20.0,20.0,50.0,40.0,theme_color("accent")),
                canvas_rect("b",100.0,20.0,50.0,40.0,theme_color("warning")),
                canvas_rect("c",180.0,20.0,50.0,40.0,theme_color("success"))])).with_key("canvas"),
            on_selection_change:Fn("changed")}).with_style(style().width(px(260)).height(px(100)))
    ]).with_style(style().padding(px(12)).gap(px(8)))}
"#;
    let (window, view) = mount(cx, script, "selection-area");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let status = |visual: &mut VisualTestContext| {
        visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|node| node.role == "status")
                .unwrap()
                .name
                .clone()
        })
    };
    let area = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("grid", "Node selection")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let local = |x: f64, y: f64| point(px((area.x + x) as f32), px((area.y + y) as f32));
    let a = local(45.0, 40.0);
    visual.simulate_mouse_down(a, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(a, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert_eq!(status(&mut visual), "a,1");

    let c = local(205.0, 40.0);
    visual.simulate_mouse_down(
        c,
        MouseButton::Left,
        Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    visual.simulate_mouse_up(
        c,
        MouseButton::Left,
        Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    visual.run_until_parked();
    assert_eq!(status(&mut visual), "a|b|c,2");

    let start = local(88.0, 8.0);
    let end = local(162.0, 72.0);
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert_eq!(status(&mut visual), "b,3");

    visual.simulate_keystrokes("tab right");
    visual.run_until_parked();
    assert!(status(&mut visual).ends_with(",4"));
}

#[gpui::test]
fn suspending_active_rotation_cleans_up_using_current_drawable_geometry(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
        import "components/rotatable" as rotatable;
        fn view(ctx) {
            rotatable::Rotatable(#{key:"rotate",label:"Rotate",angle:0.0,
                pivot:#{x:100.0,y:50.0},
                content:canvas(canvas_scene([canvas_rect("marker",95.0,45.0,10.0,10.0,rgba(0xff0000ff))]))})
                .with_style(style().width(px(300)).height(px(200)))
        }
    "#;
    let (window, view) = mount(cx, script, "rotation-suspend");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("slider", "Rotate")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let start = point(px((bounds.x + 180.0) as f32), px((bounds.y + 50.0) as f32));
    let moved = point(start.x - px(60.0), start.y + px(60.0));
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(moved, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert!(
        visual
            .update(|window, cx| view.suspend(window, cx))
            .unwrap()
    );
}

#[gpui::test]
fn tree_uses_one_controlled_outline_for_expand_navigation_and_selection(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = r#"
import "components/tree" as tree;
fn state_schema(){#{fields:#{
    expanded:#{schema:#{type:"array",max_items:8,items:#{type:"string"}},"default":#{type:"array",value:[]}},
    selected:#{schema:#{type:"array",max_items:8,items:#{type:"string"}},"default":#{type:"array",value:[]}},
    active:#{schema:#{type:"optional",value:#{type:"string"}},"default":#{type:"string",value:"root"}}
}}}
fn set_expanded(ctx,value){ctx.set_state("expanded",value);}fn set_selected(ctx,value){ctx.set_state("selected",value);}
fn set_active(ctx,value){ctx.set_state("active",value);}
fn values_text(values){let result="";for index in 0..values.len{if index>0{result+="|";}result+=values[index];}result}
fn view(ctx){let items=[#{key:"root",parent:(),label:"Root"},#{key:"a",parent:"root",label:"Alpha"},
    #{key:"b",parent:"root",label:"Bravo"},#{key:"tail",parent:(),label:"Tail",disabled:true}];
    column([text(`${values_text(ctx.get_state("expanded"))};${values_text(ctx.get_state("selected"))};${ctx.get_state("active")}`)
        .accessibility_role("status"),tree::Tree(#{key:"tree",label:"Files",items:items,
        expanded:ctx.get_state("expanded"),selected_keys:ctx.get_state("selected"),active_key:ctx.get_state("active"),
        selection_mode:"multiple",height:180.0,on_expanded_change:Fn("set_expanded"),
        on_selection_change:Fn("set_selected"),on_active_change:Fn("set_active")})
    ]).with_style(style().padding(px(12)).gap(px(8)))}
"#;
    let (window, view) = mount(cx, script, "tree");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let status = |visual: &mut VisualTestContext| {
        visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|node| node.role == "status")
                .unwrap()
                .name
                .clone()
        })
    };
    let disclosure = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("button", "Expand Root")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let point = point(
        px((disclosure.x + disclosure.width / 2.0) as f32),
        px((disclosure.y + disclosure.height / 2.0) as f32),
    );
    visual.simulate_mouse_down(point, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(point, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert!(status(&mut visual).starts_with("root;"));
    let items = visual.update(|_, cx| {
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        let mut items = snapshot
            .nodes()
            .filter(|node| node.role == "treeitem")
            .map(|node| (node.geometry.unwrap().visual.y, node.name.clone()))
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.0.total_cmp(&right.0));
        items.into_iter().map(|(_, name)| name).collect::<Vec<_>>()
    });
    assert_eq!(items, vec!["Root", "Alpha", "Bravo", "Tail"]);

    visual.simulate_keystrokes("tab down enter");
    visual.run_until_parked();
    assert_eq!(status(&mut visual), "root;a;a");
}

#[gpui::test]
fn interaction_workbench_mounts_and_closes_a_cross_component_drag_loop(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (window, view) = mount(cx, INTERACTION_LAB, "interaction-lab");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let snapshot = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    assert!(
        snapshot
            .find_by_role_and_name("tree", "Workbench files")
            .next()
            .is_some()
    );
    assert!(
        snapshot
            .find_by_role_and_name("grid", "Canvas nodes")
            .next()
            .is_some()
    );
    assert!(
        snapshot
            .find_by_role_and_name("region", "Pan zoom map")
            .next()
            .is_some()
    );
    assert!(
        snapshot
            .find_by_role_and_name("list", "Pipeline order")
            .next()
            .is_some()
    );
    let source = snapshot
        .find_by_role_and_name("button", "Move artifact")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let archive = snapshot
        .find_by_role_and_name("group", "Archive")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    let source_point = point(
        px((source.x + source.width / 2.0) as f32),
        px((source.y + source.height / 2.0) as f32),
    );
    let target_point = point(
        px((archive.x + archive.width / 2.0) as f32),
        px((archive.y + archive.height / 2.0) as f32),
    );
    visual.simulate_mouse_down(source_point, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(target_point, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(target_point, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    let status = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.role == "status")
            .unwrap()
            .name
            .clone()
    });
    assert_eq!(status, "Artifact moved to archive");
}

#[gpui::test]
fn resizable_drag_previews_natively_and_commits_opposite_corner_geometry(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = resizable_script(r#"["nw"]"#);
    let (window, view) = mount(cx, &script, "resizable-pointer");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("separator", "Demo: nw resize handle")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let status = |visual: &mut VisualTestContext| {
        visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|node| node.role == "status")
                .unwrap()
                .name
                .clone()
        })
    };
    let start = point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    );
    let end = point(start.x - px(30.0), start.y - px(20.0));
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert_eq!(
        status(&mut visual),
        "100.0,80.0,200.0,120.0,0",
        "pointer preview must not rerun Rhai"
    );
    let preview = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("separator", "Demo: nw resize handle")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    assert!((preview.x - (bounds.x - 30.0)).abs() < 0.01, "{preview:?}");
    assert!((preview.y - (bounds.y - 20.0)).abs() < 0.01, "{preview:?}");
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    assert_eq!(status(&mut visual), "70.0,60.0,230.0,140.0,1");
}

#[gpui::test]
fn resizable_keyboard_handle_uses_the_same_controlled_proposal(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = resizable_script(r#"["e"]"#);
    let (window, view) = mount(cx, &script, "resizable-keyboard");
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| window.focus_next(cx));
    visual.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("right").unwrap(),
        is_held: false,
        prefer_character_input: false,
    });
    visual.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("right").unwrap(),
    });
    visual.run_until_parked();
    let status = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.role == "status")
            .unwrap()
            .name
            .clone()
    });
    assert_eq!(status, "100.0,80.0,208.0,120.0,1");
}

#[gpui::test]
fn resizable_rejected_proposal_restores_the_controlled_rectangle(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let script = resizable_script(r#"["se"]"#).replace(r#"ctx.set_state("rect",value);"#, "");
    let (window, view) = mount(cx, &script, "resizable-rejection");
    let mut visual = VisualTestContext::from_window(*window, cx);
    let before = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("separator", "Demo: se resize handle")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let start = point(
        px((before.x + before.width / 2.0) as f32),
        px((before.y + before.height / 2.0) as f32),
    );
    let end = point(start.x + px(40.0), start.y + px(30.0));
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let after = tree
        .find_by_role_and_name("separator", "Demo: se resize handle")
        .next()
        .unwrap()
        .geometry
        .unwrap()
        .visual;
    assert!((after.x - before.x).abs() < 0.01, "{before:?} -> {after:?}");
    assert!((after.y - before.y).abs() < 0.01, "{before:?} -> {after:?}");
    assert_eq!(
        tree.nodes()
            .find(|node| node.role == "status")
            .unwrap()
            .name,
        "100.0,80.0,200.0,120.0,1"
    );
}
