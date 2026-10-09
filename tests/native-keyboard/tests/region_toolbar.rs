//! Toolbar's `fill` slot takes the width the groups leave (#115): the field no
//! longer collapses to its padding, it can sit first, between or last, and a
//! long placeholder does not push the actions to a second line. DataView passes
//! `fill`, `size` and `inset` through. Region's body scrolls only on `scroll`
//! and otherwise clips: it neither draws nor takes clicks outside the region.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle, point,
    px,
};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, BUNDLED_LAYOUT_SOURCES_BY_ID,
    BUNDLED_PATTERN_SOURCES_BY_ID, DEFAULT_THEME,
};

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

/// A mounted view: `view_body` sits in a column of `width` x `height` pixels.
fn mount(
    cx: &mut TestAppContext,
    view_body: &str,
    width: u32,
    height: u32,
) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::new();
    let tables = [
        BUNDLED_COMPONENT_SOURCES_BY_ID,
        BUNDLED_LAYOUT_SOURCES_BY_ID,
        BUNDLED_PATTERN_SOURCES_BY_ID,
    ];
    for (id, source) in tables.iter().flat_map(|table| table.iter()) {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let main = format!(
        r#"import "components/button" as button;
import "components/input" as input;
import "layouts/region" as region;
import "layouts/toolbar" as toolbar;
import "patterns/data_view" as data_view;
fn state_schema() {{ #{{ fields: #{{ clicks: #{{ schema: #{{ type: "integer" }},
    "default": #{{ type: "integer", value: 0 }} }} }} }} }}
fn noop(ctx, payload) {{ () }}
fn clicked(ctx, payload) {{ ctx.set_state("clicks", ctx.get_state("clicks") + 1); }}
fn rows() {{
    let rows = [];
    for index in 0..20 {{
        rows.push(box([text(`Row ${{index}}`)]).accessibility_role("group").test_id(`row-${{index}}`)
            .with_style(style().height(px(40)).flex_shrink(false)));
    }}
    column(rows)
}}
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px({width})).height(px({height}))) }}"#
    );
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(entry.clone(), main);
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .motion_preference(MotionPreference::None)
            .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
                (
                    path.strip_suffix(".svg").unwrap_or(path).to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".to_owned(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }))
            .prepare()
            .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window: WindowHandle<Host> = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("toolbar", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("toolbar"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| window.simulate_next_frame(cx));
    visual.run_until_parked();
    (visual, view)
}

fn bounds(visual: &mut VisualTestContext, view: &ScriptViewHandle, id: &str) -> GeometryBounds {
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    tree.nodes()
        .find(|node| {
            node.test_id.as_deref() == Some(id)
                || (node.role == "text_field" && node.name == id)
                || (node.role == "button" && node.name == id)
        })
        .and_then(|node| node.geometry)
        .map(|geometry| geometry.visual)
        .unwrap_or_else(|| panic!("no node {id}"))
}

const SEARCH: &str = r#"input::Input(#{ key: "q", label: "Search", value: "",
    placeholder: "Search hosts, owners or tags", on_change: Fn("noop") })"#;
const LONG_SEARCH: &str = r#"input::Input(#{ key: "q", label: "Search", value: "",
    placeholder: "Search hosts, owners, tags, regions, clusters, services and deployment notes",
    on_change: Fn("noop") })"#;
const FILTER: &str =
    r#"button::Button(#{ text: "Filter", variant: "ghost", on_click: Fn("noop") })"#;
const REFRESH: &str =
    r#"button::Button(#{ text: "Refresh", variant: "ghost", on_click: Fn("noop") })"#;

fn toolbar(fields: &str) -> String {
    format!(r#"toolbar::Toolbar(#{{ label: "Hosts toolbar", {fields} }})"#)
}

#[gpui::test]
fn a_fill_takes_the_width_between_the_groups(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        &toolbar(&format!(
            "filters: [{FILTER}], fill: {SEARCH}, actions: [{REFRESH}]"
        )),
        800,
        200,
    );
    let filter = bounds(&mut visual, &view, "Filter");
    let field = bounds(&mut visual, &view, "Search");
    let refresh = bounds(&mut visual, &view, "Refresh");
    println!("filter {filter:?}\nfield {field:?}\nrefresh {refresh:?}");
    assert!(field.width > 500.0, "the field is {}px wide", field.width);
    assert!(filter.x + filter.width < field.x && field.x + field.width < refresh.x);
    assert!(
        (field.y - refresh.y).abs() < 8.0,
        "one line: {field:?} {refresh:?}"
    );
    assert!((refresh.x + refresh.width - 800.0).abs() < 0.5);
}

#[gpui::test]
fn a_fill_sits_first_or_last_when_a_group_is_empty(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        &toolbar(&format!("fill: {SEARCH}, actions: [{REFRESH}]")),
        800,
        200,
    );
    let field = bounds(&mut visual, &view, "Search");
    let refresh = bounds(&mut visual, &view, "Refresh");
    assert!(field.x.abs() < 0.5, "first: {field:?}");
    assert!(field.x + field.width < refresh.x);

    let (mut visual, view) = mount(
        cx,
        &toolbar(&format!("filters: [{FILTER}], fill: {SEARCH}")),
        800,
        200,
    );
    let field = bounds(&mut visual, &view, "Search");
    let filter = bounds(&mut visual, &view, "Filter");
    assert!(filter.x + filter.width < field.x);
    assert!(
        (field.x + field.width - 800.0).abs() < 0.5,
        "last: {field:?}"
    );

    let (mut visual, view) = mount(cx, &toolbar(&format!("fill: {SEARCH}")), 800, 200);
    let field = bounds(&mut visual, &view, "Search");
    assert!(
        field.x.abs() < 0.5 && (field.width - 800.0).abs() < 0.5,
        "alone: {field:?}"
    );
}

#[gpui::test]
fn a_long_placeholder_does_not_wrap_the_bar(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        &toolbar(&format!(
            "filters: [{FILTER}], fill: {LONG_SEARCH}, actions: [{REFRESH}]"
        )),
        480,
        200,
    );
    let field = bounds(&mut visual, &view, "Search");
    let refresh = bounds(&mut visual, &view, "Refresh");
    assert!(
        (field.y - refresh.y).abs() < 8.0,
        "one line: {field:?} {refresh:?}"
    );
    assert!(field.x + field.width < refresh.x);
}

#[gpui::test]
fn a_bar_too_narrow_for_the_minimum_wraps_instead_of_shrinking(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        &toolbar(&format!(
            "filters: [{FILTER}], fill: {SEARCH}, actions: [{REFRESH}]"
        )),
        240,
        200,
    );
    let field = bounds(&mut visual, &view, "Search");
    let refresh = bounds(&mut visual, &view, "Refresh");
    println!("field {field:?}\nrefresh {refresh:?}");
    // metrics.label_column is the floor.
    assert!(
        field.width >= 128.0 - 0.5,
        "the field is {}px wide",
        field.width
    );
    assert!(
        refresh.y > field.y + 8.0,
        "the actions wrap: {field:?} {refresh:?}"
    );
}

#[gpui::test]
fn data_view_passes_fill_size_and_inset_through(cx: &mut TestAppContext) {
    let data_view = |extra: &str| {
        format!(
            r#"data_view::DataView(#{{ key: "hosts", label: "Hosts", body: text("rows"), {extra}
                toolbar: #{{ fill: {SEARCH}, actions: [{REFRESH}] }} }})"#
        )
    };
    let (mut visual, view) = mount(cx, &data_view(""), 800, 400);
    let field = bounds(&mut visual, &view, "Search");
    let refresh = bounds(&mut visual, &view, "Refresh");
    println!("default: field {field:?} refresh {refresh:?}");
    assert!(field.width > 500.0, "the field is {}px wide", field.width);
    assert!(field.x > 0.5, "the region's inset applies: {field:?}");

    let (mut visual, view) = mount(cx, &data_view("inset: false,"), 800, 400);
    let flush = bounds(&mut visual, &view, "Search");
    assert!(flush.x.abs() < 0.5, "no inset: {flush:?}");

    let small = data_view("").replace("actions: [", r#"size: "xs", actions: ["#);
    let (mut visual, view) = mount(cx, &small, 800, 400);
    let small_refresh = bounds(&mut visual, &view, "Refresh");
    println!("xs: refresh {small_refresh:?}");
    assert!(
        small_refresh.height < refresh.height - 0.5,
        "size reaches the controls: {} vs {}",
        small_refresh.height,
        refresh.height
    );
}

#[gpui::test]
fn data_view_insets_a_body_that_does_not_bleed(cx: &mut TestAppContext) {
    // By default the body reaches the sides (rows carry their inset); `bleed: false`
    // keeps the region inset for a body that has none of its own (#119).
    let data_view = |extra: &str| {
        format!(
            r#"data_view::DataView(#{{ key: "hosts", label: "Hosts", {extra}
                body: box([text("detail")]).accessibility_role("group").test_id("body") }})"#
        )
    };
    let (mut visual, view) = mount(cx, &data_view(""), 800, 400);
    let bled = bounds(&mut visual, &view, "body");
    assert!(bled.x.abs() < 0.5, "the body bleeds: {bled:?}");
    let (mut visual, view) = mount(cx, &data_view("bleed: false,"), 800, 400);
    let inset = bounds(&mut visual, &view, "body");
    assert!(
        (inset.x - 12.0).abs() < 0.5,
        "the body keeps the inset: {inset:?}"
    );
}

fn scroll_body(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> (f64, f64) {
    let before = bounds(visual, view, "row-0");
    #[allow(clippy::cast_possible_truncation)]
    visual.simulate_event(gpui::ScrollWheelEvent {
        position: point(px((before.x + 20.0) as f32), px((before.y + 60.0) as f32)),
        delta: gpui::ScrollDelta::Pixels(point(px(0.0), px(-120.0))),
        ..gpui::ScrollWheelEvent::default()
    });
    visual.run_until_parked();
    visual.update(|window, cx| window.simulate_next_frame(cx));
    visual.run_until_parked();
    (before.y, bounds(visual, view, "row-0").y)
}

#[gpui::test]
fn a_region_body_scrolls_only_when_asked(cx: &mut TestAppContext) {
    let region = |scroll: bool| {
        format!(
            r#"region::Region(#{{ label: "Hosts", title: "Hosts", scroll: {scroll},
                body: rows(), footer: [box([text("20 hosts")]).accessibility_role("group").test_id("footer")] }})"#
        )
    };
    let (mut visual, view) = mount(cx, &region(true), 800, 320);
    let footer = bounds(&mut visual, &view, "footer");
    assert!(
        footer.y + footer.height <= 320.5,
        "the footer stays in view: {footer:?}"
    );
    let (before, after) = scroll_body(&mut visual, &view);
    println!("scroll: {before} -> {after}");
    assert!(
        after < before - 60.0,
        "the body scrolls: {before} -> {after}"
    );
    let footer_after = bounds(&mut visual, &view, "footer");
    assert!(
        (footer_after.y - footer.y).abs() < 0.5,
        "the footer does not move"
    );

    let (mut visual, view) = mount(cx, &region(false), 800, 320);
    let (before, after) = scroll_body(&mut visual, &view);
    assert!(
        (after - before).abs() < 0.5,
        "no scroll by default: {before} -> {after}"
    );
}

#[gpui::test]
fn a_region_body_clips_unless_it_scrolls(cx: &mut TestAppContext) {
    // A 500px body in a 200px Region: a press 100px under the region must not
    // reach it, and its fill must be masked to the region; a press inside does.
    for scroll in [false, true] {
        let body = format!(
            r#"region::Region(#{{ label: "Long", fill: false, inset: false, scroll: {scroll},
                body: box([text("Long body")]).accessibility_role("button").accessibility_label("Long body")
                    .on_click(Fn("clicked"))
                    .with_style(style().height(px(500)).flex_shrink(false).background(rgba(0xff00ffff))) }})
                .with_style(style().height(px(200)).flex_shrink(false)),
            text(`${{ctx.get_state("clicks")}}`).test_id("clicks")"#
        );
        let (mut visual, view) = mount(cx, &body, 300, 600);
        let clicks = |visual: &mut VisualTestContext| {
            visual.update(|_, cx| {
                view.accessibility_snapshot(cx)
                    .unwrap()
                    .nodes()
                    .find(|node| node.test_id.as_deref() == Some("clicks"))
                    .unwrap()
                    .name
                    .clone()
            })
        };
        let magenta: gpui::Hsla = gpui::rgba(0xff00_ffff).into();
        // Scene quads are in device pixels.
        let mask_bottom = visual.update(|window, _| {
            let scale = window.scale_factor();
            window
                .painted_quads()
                .iter()
                .filter(|quad| quad.background == magenta.into())
                .map(|quad| {
                    let mask = quad.content_mask.bounds;
                    let bottom = (mask.origin.y + mask.size.height).0;
                    bottom.min((quad.bounds.origin.y + quad.bounds.size.height).0) / scale
                })
                .fold(0.0_f32, f32::max)
        });
        assert!(
            mask_bottom > 150.0 && mask_bottom <= 200.5,
            "scroll {scroll}: the body paints down to {mask_bottom}"
        );
        visual.simulate_click(point(px(50.0), px(300.0)), gpui::Modifiers::none());
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
        assert_eq!(clicks(&mut visual), "0", "scroll {scroll}: a press below");
        visual.simulate_click(point(px(50.0), px(100.0)), gpui::Modifiers::none());
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
        assert_eq!(clicks(&mut visual), "1", "scroll {scroll}: a press inside");
    }
}
