//! Official components keep the promises of their schema docs and the design rules: optional
//! props accept `()`, declared parts reach the node they name, wrappers keep the defaults of
//! the component they wrap, and disabled or status looks follow docs/design.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, px, rgba};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_CHART_SOURCES_BY_ID, BUNDLED_COMPONENT_SOURCES_BY_ID,
    BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_MOTION_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID,
    DEFAULT_THEME, EN_LOCALE, TOKEN_BASE_SOURCE,
};

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

fn mount(cx: &mut TestAppContext, main: &str) -> (VisualTestContext, ScriptViewHandle) {
    mount_with(
        cx,
        main,
        "fn component_styles() { #{} }",
        ThemeTokenOverrides::default(),
    )
}

fn mount_with(
    cx: &mut TestAppContext,
    main: &str,
    styles: &str,
    overrides: ThemeTokenOverrides,
) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let mut modules = BTreeMap::from([(entry.clone(), main.to_owned())]);
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .chain(BUNDLED_PATTERN_SOURCES_BY_ID)
        .chain(BUNDLED_CHART_SOURCES_BY_ID)
        .chain(BUNDLED_MOTION_SOURCES_BY_ID)
    {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(TOKEN_BASE_SOURCE)
            .locale_sources([("en.rhai".into(), EN_LOCALE.to_owned())])
            .component_styles(styles)
            .theme_token_overrides(overrides)
            .motion_preference(MotionPreference::None)
            .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
                (
                    path.trim_end_matches(".svg").to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".into(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }))
            .prepare()
            .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("contracts", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("contracts"), host.clone(), window, cx)
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
    visual.run_until_parked();
    visual.executor().advance_clock(Duration::from_millis(32));
    visual.update(|window, cx| window.simulate_next_frame(cx));
    visual.run_until_parked();
}

fn last_error(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Option<String> {
    visual.update(|_, cx| view.last_error(cx).unwrap())
}

fn nodes(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<AccessibilityNode> {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .cloned()
            .collect()
    })
}

fn bounds(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    nodes(visual, view)
        .into_iter()
        .find(|node| node.role == role && node.name == name)
        .unwrap_or_else(|| panic!("no {role} named {name}"))
        .geometry
        .unwrap()
        .visual
}

fn color(hex: u32) -> Rgba8 {
    Rgba8::from_rgba_hex(hex)
}

fn colors(tokens: &[(&str, u32)]) -> ThemeTokenOverrides {
    ThemeTokenOverrides {
        colors: tokens
            .iter()
            .map(|(token, hex)| ((*token).to_owned(), color(*hex)))
            .collect(),
        ..Default::default()
    }
}

/// (background, border color) of every painted quad.
fn quads(visual: &mut VisualTestContext) -> Vec<(gpui::Background, gpui::Hsla, f32)> {
    visual.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .map(|quad| {
                (
                    quad.background,
                    quad.border_color,
                    quad.border_widths.left.0,
                )
            })
            .collect()
    })
}

fn hsla(hex: u32) -> gpui::Hsla {
    rgba(hex).into()
}

fn press(visual: &mut VisualTestContext, at: GeometryBounds) {
    #[allow(clippy::cast_possible_truncation)]
    let position = gpui::point(
        px((at.x + at.width / 2.0) as f32),
        px((at.y + at.height / 2.0) as f32),
    );
    visual.simulate_click(position, gpui::Modifiers::none());
    settle(visual);
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    nodes(visual, view)
        .into_iter()
        .find(|node| node.test_id.as_deref() == Some("log"))
        .unwrap()
        .name
        .trim()
        .to_owned()
}

const TABLE_ROWS: &str = r#"row_key: "id", rows: [#{ id: "a", name: "Alpha", count: "1" }]"#;

#[gpui::test]
fn a_table_takes_an_unset_height_beside_fill_height(cx: &mut TestAppContext) {
    // `height` is optional: `()` is the same as leaving it out, as in List and Tree.
    let (mut visual, view) = mount(
        cx,
        &format!(
            r#"import "components/table" as table;
fn view(ctx) {{ column([table::Table(#{{ key: "t", label: "Items", {TABLE_ROWS},
    columns: [#{{ key: "name", title: "Name", width: #{{ kind: "flex", value: 1.0 }} }}],
    height: (), fill_height: true }})]).with_style(style().width(px(400)).height(px(300))) }}"#
        ),
    );
    assert_eq!(last_error(&mut visual, &view), None);
    let table = bounds(&mut visual, &view, "table", "Items");
    assert!((table.height - 300.0).abs() < 0.5, "{table:?}");
}

#[gpui::test]
fn a_centered_table_column_centers_its_header(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        &format!(
            r#"import "components/table" as table;
fn view(ctx) {{ column([table::Table(#{{ key: "t", label: "Items", {TABLE_ROWS},
    columns: [#{{ key: "name", title: "Name", width: #{{ kind: "fixed", value: 200.0 }}, align: "center" }},
        #{{ key: "count", title: "Count", width: #{{ kind: "fixed", value: 200.0 }}, align: "end" }}],
    height: 200.0 }})]).with_style(style().width(px(600)).height(px(300))) }}"#
        ),
    );
    let center = |bounds: GeometryBounds| bounds.x + bounds.width / 2.0;
    let end = |bounds: GeometryBounds| bounds.x + bounds.width;
    let header = bounds(&mut visual, &view, "columnheader", "Name");
    let title = bounds(&mut visual, &view, "text", "NAME");
    let cell = bounds(&mut visual, &view, "text", "Alpha");
    assert!(
        (center(title) - center(header)).abs() < 1.0,
        "header {header:?}, title {title:?}"
    );
    assert!((center(title) - center(cell)).abs() < 1.0, "cell {cell:?}");
    // An end column already followed its alignment; it keeps doing so.
    let count = bounds(&mut visual, &view, "text", "COUNT");
    let number = bounds(&mut visual, &view, "text", "1");
    assert!(
        (end(count) - end(number)).abs() < 1.0,
        "{count:?} {number:?}"
    );
}

#[gpui::test]
fn toggle_part_styles_reach_its_label_and_slots(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"import "components/toggle" as toggle;
fn mark(name) { box([]).accessibility_role("image").accessibility_label(name) }
fn view(ctx) { column([row([toggle::Toggle(#{ text: "Bold", pressed: false,
    prefix: mark("before"), suffix: mark("after"),
    part_styles: #{ label: style().width(px(90)), prefix: style().width(px(17)).height(px(5)),
        suffix: style().width(px(23)).height(px(5)) } })])]) }"#,
    );
    assert_eq!(last_error(&mut visual, &view), None);
    assert!((bounds(&mut visual, &view, "text", "Bold").width - 90.0).abs() < 0.5);
    assert!((bounds(&mut visual, &view, "image", "before").width - 17.0).abs() < 0.5);
    assert!((bounds(&mut visual, &view, "image", "after").width - 23.0).abs() < 0.5);
}

#[gpui::test]
fn a_searchable_select_keeps_the_combobox_search_placeholder(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"import "components/select" as select;
fn view(ctx) { column([select::Select(#{ key: "fruit", label: "Fruit",
    options: [#{ value: "a", label: "Apple" }], open: true, query: "", searchable: true })])
    .with_style(style().width(px(400)).height(px(400))) }"#,
    );
    let placeholders = nodes(&mut visual, &view)
        .into_iter()
        .filter_map(|node| node.placeholder)
        .collect::<Vec<_>>();
    assert_eq!(placeholders, ["Search"]);
}

#[gpui::test]
fn toggle_group_space_and_enter_press_the_focused_segment(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"import "components/toggle_group" as group;
fn state_schema() { #{ fields: #{
    values: #{ schema: #{ type: "array", items: #{ type: "string" } }, "default": #{ type: "array", value: [] } },
    log: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } } } } }
fn changed(ctx, values) { ctx.set_state("values", values);
    ctx.set_state("log", `${ctx.get_state("log")} ${values}`); }
fn view(ctx) { column([group::ToggleGroup(#{ key: "format", label: "Format", mode: "multiple",
    values: ctx.get_state("values"), on_change: Fn("changed"),
    items: [#{ value: "b", label: "Bold" }, #{ value: "i", label: "Italic" }] }),
    text(ctx.get_state("log")).test_id("log")]) }"#,
    );
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    visual.simulate_keystrokes("space");
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), r#"["b"]"#);
    visual.simulate_keystrokes("enter");
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), r#"["b"] []"#);
    visual.simulate_keystrokes("right space");
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), r#"["b"] [] ["i"]"#);
}

#[gpui::test]
fn the_label_required_part_styles_only_the_mark(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"import "components/label" as label;
fn view(ctx) { column([
    row([label::Label(#{ text: "Plain", part_styles: #{ required: style().width(px(300)) } })]),
    row([label::Label(#{ text: "Needed", required: true,
        part_styles: #{ required: style().width(px(40)) } })]),
]) }"#,
    );
    // Without `required` there is no mark, so the part has nothing to style.
    let plain = bounds(&mut visual, &view, "text", "Plain");
    assert!(plain.width < 100.0, "{plain:?}");
    let needed = bounds(&mut visual, &view, "text", "Needed");
    let mark = bounds(&mut visual, &view, "text", " *");
    assert!(needed.width < 100.0, "{needed:?}");
    assert!((mark.width - 40.0).abs() < 0.5, "{mark:?}");
    assert!(
        (mark.x - (needed.x + needed.width)).abs() < 0.5,
        "{needed:?} {mark:?}"
    );
    // A long required label still wraps inside its width, the mark beside it.
    let (mut visual, view) = mount(
        cx,
        r#"import "components/label" as label;
fn view(ctx) { column([label::Label(#{ text: "A rather long label that has to wrap", required: true })])
    .with_style(style().width(px(120))) }"#,
    );
    let long = bounds(
        &mut visual,
        &view,
        "text",
        "A rather long label that has to wrap",
    );
    let mark = bounds(&mut visual, &view, "text", " *");
    assert!(long.height > 30.0, "{long:?}");
    assert!(mark.x + mark.width <= 120.5, "{long:?} {mark:?}");
}

const ACCENT: u32 = 0x1020_30ff;
const SURFACE_HOVER: u32 = 0x4050_60ff;
const BORDER: u32 = 0x7080_90ff;
const SELECTION: u32 = 0xa0b0_c0ff;
const TEXT_MUTED: u32 = 0xc0d0_e0ff;

fn icon_button_quads(
    cx: &mut TestAppContext,
    props: &str,
) -> Vec<(gpui::Background, gpui::Hsla, f32)> {
    let (mut visual, view) = mount_with(
        cx,
        &format!(
            r#"import "components/icon_button" as icon_button;
fn view(ctx) {{ row([icon_button::IconButton(#{{ icon: box([]), label: "Tool", disabled: true, {props} }})]) }}"#
        ),
        "fn component_styles() { #{} }",
        colors(&[
            ("accent", ACCENT),
            ("surface_hover", SURFACE_HOVER),
            ("border", BORDER),
            ("selection", SELECTION),
        ]),
    );
    assert_eq!(last_error(&mut visual, &view), None);
    quads(&mut visual)
}

#[gpui::test]
fn a_disabled_icon_button_keeps_the_variant_silhouette(cx: &mut TestAppContext) {
    // D27, as on Button: blocks become a neutral block, outline keeps only its frame and
    // ghost stays bare, whatever the enabled fill and frame were.
    let fill = |hex: u32| -> gpui::Background { hsla(hex).into() };
    let primary = icon_button_quads(cx, r#"variant: "primary""#);
    assert!(
        primary.iter().any(|(bg, ..)| *bg == fill(SURFACE_HOVER)),
        "{primary:?}"
    );
    assert!(
        primary
            .iter()
            .any(|(_, border, width)| *border == hsla(SURFACE_HOVER) && *width > 0.0),
        "{primary:?}"
    );
    assert!(
        !primary.iter().any(|(_, border, _)| *border == hsla(ACCENT)),
        "{primary:?}"
    );
    let outline = icon_button_quads(cx, r#"variant: "outline""#);
    assert!(
        outline
            .iter()
            .any(|(_, border, width)| *border == hsla(BORDER) && *width > 0.0),
        "{outline:?}"
    );
    assert!(
        !outline.iter().any(|(bg, ..)| *bg == fill(SURFACE_HOVER)),
        "{outline:?}"
    );
    let selected = icon_button_quads(cx, r#"variant: "ghost", selected: true"#);
    assert!(
        !selected
            .iter()
            .any(|(bg, border, _)| *border == hsla(SELECTION) || *bg == fill(SELECTION)),
        "{selected:?}"
    );
}

#[gpui::test]
fn a_lone_button_group_item_is_first_and_last(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"import "components/button" as button;
import "components/button_group" as button_group;
fn view(ctx) { column([row([button_group::ButtonGroup(#{ label: "Only",
    buttons: [button::Button(#{ text: "One" })],
    part_styles: #{ last: style().width(px(150)) } })])]) }"#,
    );
    assert!((bounds(&mut visual, &view, "button", "One").width - 150.0).abs() < 0.5);
}

#[gpui::test]
fn an_icon_with_an_unset_label_is_decorative(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"import "components/icon" as icon;
fn view(ctx) { row([icon::Icon(#{ source: asset("app/icons/check"), label: () })]) }"#,
    );
    assert_eq!(last_error(&mut visual, &view), None);
    assert!(
        nodes(&mut visual, &view)
            .iter()
            .any(|node| node.role == "presentation")
    );
}

#[gpui::test]
fn an_info_alert_marks_itself_with_a_neutral_lamp(cx: &mut TestAppContext) {
    // composition.md §5: accent is not an "info" status color; a neutral lamp is muted ink.
    let (mut visual, view) = mount_with(
        cx,
        r#"import "components/alert" as alert;
fn view(ctx) { column([alert::Alert(#{ title: "Heads up", variant: "info" })])
    .with_style(style().width(px(400))) }"#,
        "fn component_styles() { #{} }",
        colors(&[("accent", ACCENT), ("text_muted", TEXT_MUTED)]),
    );
    assert_eq!(last_error(&mut visual, &view), None);
    let painted = quads(&mut visual);
    let fill = |hex: u32| -> gpui::Background { hsla(hex).into() };
    assert!(
        !painted.iter().any(|(bg, ..)| *bg == fill(ACCENT)),
        "{painted:?}"
    );
    assert!(
        painted.iter().any(|(bg, ..)| *bg == fill(TEXT_MUTED)),
        "{painted:?}"
    );
}

#[gpui::test]
fn a_chart_adapter_root_follows_the_component_stylesheet(cx: &mut TestAppContext) {
    let (mut visual, view) = mount_with(
        cx,
        r#"import "charts/bar_chart" as bar;
fn view(ctx) { column([bar::BarChart(#{ key: "sales", title: "Sales",
    data: [#{ id: "a", x: "a", y: 1 }], key_dimension: "id", encode: #{ x: "x", y: "y" } })])
    .with_style(style().width(px(600)).height(px(400))) }"#,
        r#"fn component_styles() { #{ "charts/bar_chart": #{ root: style().width(px(321)) } } }"#,
        ThemeTokenOverrides::default(),
    );
    assert_eq!(last_error(&mut visual, &view), None);
    let figure = bounds(&mut visual, &view, "figure", "Sales");
    assert!((figure.width - 321.0).abs() < 0.5, "{figure:?}");
}

#[gpui::test]
fn a_number_ticker_value_change_keeps_its_text_mounted(cx: &mut TestAppContext) {
    // Exit motion only plays for nodes that leave the tree; a new value replays the enter
    // fade on the same node, so there is no fading-out old value.
    let (mut visual, view) = mount(
        cx,
        r#"import "motion/number_ticker" as ticker;
fn state_schema() { #{ fields: #{ value: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 1 } } } } }
fn bump(ctx, payload) { ctx.set_state("value", ctx.get_state("value") + 1); }
fn view(ctx) { column([
    text("Bump").accessibility_role("button").accessibility_label("Bump").on_click(Fn("bump"))
        .with_style(style().width(px(80)).height(px(32))),
    ticker::NumberTicker(#{ key: "count", value: ctx.get_state("value") }),
]) }"#,
    );
    let button = bounds(&mut visual, &view, "button", "Bump");
    press(&mut visual, button);
    let _ = bounds(&mut visual, &view, "text", "2");
    let report = visual.update(|_, cx| view.reconcile_report(cx).unwrap());
    assert!(report.unmounted.is_empty(), "{report:?}");
}
