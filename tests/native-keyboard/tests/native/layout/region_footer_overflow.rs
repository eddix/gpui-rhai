//! A Region / DataView footer wider than its region wraps onto more lines and never paints
//! past the region over whatever is beside it (#137). Region's footer row wraps, may shrink
//! below its content and clips; DataView's start and end groups shrink and wrap their own
//! fields too, so an item wider than the region is cut at its edge and the rest wrap.
//!
//! cargo test --manifest-path tests/native-keyboard/Cargo.toml --test native layout::region_footer_overflow -- --nocapture

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, BUNDLED_LAYOUT_SOURCES_BY_ID,
    BUNDLED_PATTERN_SOURCES_BY_ID, DEFAULT_THEME, EN_LOCALE, TOKEN_BASE_SOURCE,
};

const UPDATED: &str = "Updated 12:04:31";

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(cx: &mut TestAppContext, main: String) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let mut modules = BTreeMap::from([(entry.clone(), main)]);
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .chain(BUNDLED_PATTERN_SOURCES_BY_ID)
    {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(TOKEN_BASE_SOURCE)
            .locale_sources([("en.rhai".into(), EN_LOCALE.to_owned())])
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
        let host = ScriptViewHost::new("footer", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("footer"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    for _ in 0..3 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    (visual, view)
}

/// Where a node is laid out, in logical pixels.
#[derive(Clone, Copy, Debug)]
struct Placed {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

fn placed(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> Placed {
    visual.update(|_, cx| {
        let tree = view.accessibility_snapshot(cx).unwrap();
        let node = tree
            .find_by_role_and_name(role, name)
            .next()
            .unwrap_or_else(|| panic!("no {role} named {name}"));
        let visual = node.geometry.as_ref().unwrap().visual;
        Placed {
            left: visual.x,
            right: visual.x + visual.width,
            top: visual.y,
            bottom: visual.y + visual.height,
        }
    })
}

/// The right edges of the magenta quads painted inside `node`'s box, in logical pixels: as
/// laid out, and as drawn, cut by the content mask each quad carries.
fn quad_right_edges(visual: &mut VisualTestContext, node: Placed) -> Vec<(f64, f64)> {
    let magenta: gpui::Hsla = gpui::rgba(0xff00_ffff).into();
    visual.update(|window, _| {
        let scale = f64::from(window.scale_factor());
        window
            .painted_quads()
            .iter()
            .filter(|quad| quad.background == magenta.into())
            .filter_map(|quad| {
                let left = f64::from(quad.bounds.origin.x.0) / scale;
                let top = f64::from(quad.bounds.origin.y.0) / scale;
                let right = f64::from((quad.bounds.origin.x + quad.bounds.size.width).0) / scale;
                let bottom = f64::from((quad.bounds.origin.y + quad.bounds.size.height).0) / scale;
                let mask = quad.content_mask.bounds;
                let mask_right = f64::from((mask.origin.x + mask.size.width).0) / scale;
                (left >= node.left - 0.5 && top >= node.top - 0.5 && bottom <= node.bottom + 0.5)
                    .then_some((right, right.min(mask_right)))
            })
            .collect()
    })
}

/// `region` in a 460px column, with a 300px pane beside it.
fn beside_a_pane(imports: &str, region: &str) -> String {
    format!(
        r#"{imports}
fn view(ctx) {{ row([
    column([{region}]).with_style(style().width(px(460)).height(px(320)).flex_shrink(false)),
    column([text("Detail pane")]).with_style(style().width(px(300)).height(px(320))),
]) }}"#
    )
}

/// A DataView whose footer has the fields in `fields` and `updated`.
fn data_view(fields: &str) -> String {
    beside_a_pane(
        r#"import "patterns/data_view" as data_view;
import "components/list" as list;
import "components/pagination" as pagination;"#,
        &format!(
            r#"data_view::DataView(#{{ key: "hosts", label: "Hosts", title: "Hosts",
        body: list::List(#{{ key: "rows", label: "Rows", fill_height: true,
            items: [#{{ key: "a", title: "web-01" }}, #{{ key: "b", title: "web-02" }}] }}),
        footer: #{{ {fields} updated: "{UPDATED}" }} }})"#
        ),
    )
}

/// A Pagination with its summary, wider than the region; a magenta fill marks what it paints.
const WIDE_PAGINATION: &str = r#"status: pagination::Pagination(#{ key: "pages", label: "Hosts pagination",
    total_items: 128, current_page: 3, page_size: 20, show_summary: true, size: "xs" })
    .with_style(style().background(rgba(0xff00ffff))),"#;

fn assert_inside(what: &str, node: Placed, region: Placed, sibling: Placed) {
    assert!(
        node.right <= region.right + 0.5,
        "{what} runs to x = {:.1}, {:.1}px past the region's right edge {:.1}; the pane \
         beside it starts at {:.1} ({node:?})",
        node.right,
        node.right - region.right,
        region.right,
        sibling.left
    );
    assert!(
        node.top >= region.top - 0.5 && node.bottom <= region.bottom + 0.5,
        "{what} leaves the region vertically: {node:?} in {region:?}"
    );
}

/// `node` is wider than the region; what it paints stops at the region's right edge.
fn assert_cut(visual: &mut VisualTestContext, what: &str, node: Placed, region: Placed) {
    assert!(
        node.right > region.right + 0.5,
        "the probe needs {what} wider than the region: {node:?} in {region:?}"
    );
    let quads = quad_right_edges(visual, node);
    println!("{what}: quad right edges (laid out, drawn) {quads:?}");
    assert!(
        quads
            .iter()
            .any(|(laid_out, _)| *laid_out > region.right + 0.5),
        "{what} paints no quad past the region, so the clip is not exercised: {quads:?}"
    );
    for (laid_out, drawn) in quads {
        assert!(
            drawn <= region.right + 0.5,
            "{what} draws a quad to x = {drawn:.1} (laid out to {laid_out:.1}), past the \
             region's right edge {:.1}",
            region.right
        );
    }
}

/// Control: count and updated alone fit on one line at the end edge.
#[gpui::test]
fn control_a_short_footer_fits(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, data_view(r#"count: "2 hosts","#));
    let region = placed(&mut visual, &view, "region", "Hosts");
    let count = placed(&mut visual, &view, "text", "2 hosts");
    let updated = placed(&mut visual, &view, "text", UPDATED);
    let sibling = placed(&mut visual, &view, "text", "Detail pane");
    println!("region {region:?}, count {count:?}, updated {updated:?}");
    assert_inside("`updated`", updated, region, sibling);
    assert!(
        (count.top - updated.top).abs() < 0.5,
        "one line: {count:?} {updated:?}"
    );
    assert!(
        updated.right >= region.right - 24.0,
        "the end group sits at the end edge: {updated:?} in {region:?}"
    );
}

#[gpui::test]
fn a_pagination_footer_stays_inside_its_region(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, data_view(WIDE_PAGINATION));
    let region = placed(&mut visual, &view, "region", "Hosts");
    let pagination = placed(&mut visual, &view, "navigation", "Hosts pagination");
    let updated = placed(&mut visual, &view, "text", UPDATED);
    let sibling = placed(&mut visual, &view, "text", "Detail pane");
    println!("region {region:?}, pagination {pagination:?}, updated {updated:?}");
    assert_inside("`updated`", updated, region, sibling);
    // The Pagination alone is wider than the region: the clip cuts its painted end.
    assert_cut(&mut visual, "the Pagination", pagination, region);
    assert!(
        updated.top >= pagination.bottom - 0.5,
        "`updated` wraps to its own line: {updated:?} under {pagination:?}"
    );
}

/// The selection after an over-wide status wraps inside the start group instead of being cut.
#[gpui::test]
fn a_selection_after_a_wide_status_wraps_instead_of_being_cut(cx: &mut TestAppContext) {
    let fields = format!(r#"{WIDE_PAGINATION} selection: "3 selected", count: "128 hosts","#);
    let (mut visual, view) = mount(cx, data_view(&fields));
    let region = placed(&mut visual, &view, "region", "Hosts");
    let sibling = placed(&mut visual, &view, "text", "Detail pane");
    let pagination = placed(&mut visual, &view, "navigation", "Hosts pagination");
    let selection = placed(&mut visual, &view, "text", "3 selected");
    let count = placed(&mut visual, &view, "text", "128 hosts");
    let updated = placed(&mut visual, &view, "text", UPDATED);
    println!(
        "region {region:?}, pagination {pagination:?}, selection {selection:?}, count {count:?}, \
         updated {updated:?}"
    );
    assert_cut(&mut visual, "the Pagination", pagination, region);
    for (what, node) in [
        ("the selection", selection),
        ("the count", count),
        ("`updated`", updated),
    ] {
        assert_inside(what, node, region, sibling);
    }
    assert!(
        selection.top >= pagination.bottom - 0.5,
        "the selection wraps under the Pagination: {selection:?} under {pagination:?}"
    );
    assert!(
        (count.top - updated.top).abs() < 0.5 && count.top >= selection.bottom - 0.5,
        "count and updated share the line under the start group: {count:?} {updated:?}"
    );
}

#[gpui::test]
fn a_plain_region_footer_wraps_and_clips(cx: &mut TestAppContext) {
    let source = beside_a_pane(
        r#"import "layouts/region" as region;"#,
        r#"region::Region(#{ label: "Jobs", title: "Jobs", body: text("Body"),
        footer: [text("Status: running"), text("Selected 3 of 128"),
            text("Wide status").with_style(style().width(px(560)).flex_shrink(false)
                .background(rgba(0xff00ffff))),
            text("Last run 12:04:31")] })"#,
    );
    let (mut visual, view) = mount(cx, source);
    let region = placed(&mut visual, &view, "region", "Jobs");
    let sibling = placed(&mut visual, &view, "text", "Detail pane");
    let status = placed(&mut visual, &view, "text", "Status: running");
    let selected = placed(&mut visual, &view, "text", "Selected 3 of 128");
    let wide = placed(&mut visual, &view, "text", "Wide status");
    let last = placed(&mut visual, &view, "text", "Last run 12:04:31");
    println!(
        "region {region:?}, status {status:?}, selected {selected:?}, wide {wide:?}, last {last:?}"
    );
    for (what, node) in [
        ("the status", status),
        ("the selection", selected),
        ("the last item", last),
    ] {
        assert_inside(what, node, region, sibling);
    }
    assert_cut(&mut visual, "the wide item", wide, region);
    // The first two fit on one line; the wide item and the last one each wrap below.
    assert!(
        (status.top - selected.top).abs() < 0.5,
        "{status:?} {selected:?}"
    );
    assert!(wide.top > selected.top + 0.5, "{wide:?} under {selected:?}");
    assert!(last.top > wide.top + 0.5, "{last:?} under {wide:?}");
}
