//! Composition audit coverage and false positives found by the oh-my-byted
//! upgrade: content behind an error boundary and in virtual lists is audited
//! (#112); unrelated panes, a Badge's own gap, and content led by a view
//! switcher or a heading drawn elsewhere are not reported (#114); an absolutely
//! positioned focus frame does not stack with the text of a panel it is drawn over.
//! Each false positive case has a positive control that must still be reported.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, BUNDLED_LAYOUT_SOURCES_BY_ID,
    DEFAULT_THEME,
};

const PROFILE: &str = r#"fn profile() { #{ name: "test",
    audit: ["row-height-mismatch", "text-edge-misaligned", "spacing-not-nested", "mixed-type-in-row"] } }"#;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn audit(cx: &mut TestAppContext, view_body: &str) -> Vec<String> {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(BUNDLED_LAYOUT_SOURCES_BY_ID)
    {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let main = format!(
        r#"import "components/badge" as badge;
import "components/button" as button;
import "components/split_pane" as split;
import "components/tabs" as tabs;
import "layouts/region" as region;
fn noop(ctx, payload) {{ () }}
fn item(ctx, payload) {{
    row([text(payload.item.label).with_style(style().typography("caption")),
        text("big").with_style(style().typography("title"))])
        .with_key(payload.item.key).with_style(style().gap(px(8)).height(px(32)))
}}
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(800))) }}"#
    );
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(entry.clone(), main);
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .profile_source(PROFILE)
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
        let host = ScriptViewHost::new("audit", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("audit"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    for _ in 0..2 {
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
    visual
        .update(|_, cx| view.composition_audit(cx).unwrap())
        .into_iter()
        .map(|finding| {
            format!(
                "{} at {}: {}",
                finding.rule.id(),
                finding.path,
                finding.message
            )
        })
        .collect()
}

/// A row that mixes type sizes: a real finding wherever it sits.
const MIXED_ROW: &str = r#"row([text("small").with_style(style().typography("caption")),
    text("big").with_style(style().typography("title"))]).with_style(style().gap(px(8)))"#;

#[gpui::test]
fn content_behind_an_error_boundary_is_audited(cx: &mut TestAppContext) {
    let direct = audit(cx, MIXED_ROW);
    assert!(
        direct.iter().any(|f| f.starts_with("mixed-type-in-row")),
        "{direct:?}"
    );
    let bounded = audit(
        cx,
        &format!(r#"error_boundary({MIXED_ROW}, text("fallback"))"#),
    );
    assert!(
        bounded
            .iter()
            .any(|f| f.starts_with("mixed-type-in-row") && f.contains("/child")),
        "{bounded:?}"
    );
}

#[gpui::test]
fn realized_virtual_rows_are_audited(cx: &mut TestAppContext) {
    let found = audit(
        cx,
        r#"virtual_collection(#{ key: "list", label: "Rows", height: 200.0, estimated_height: 32.0,
            data: [#{ key: "a", label: "Alpha" }, #{ key: "b", label: "Bravo" }] }, Fn("item"))"#,
    );
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("mixed-type-in-row") && f.contains("/item/")),
        "{found:?}"
    );
}

#[gpui::test]
fn unrelated_panes_side_by_side_are_not_one_row_of_controls(cx: &mut TestAppContext) {
    let panes = audit(
        cx,
        r#"row([
            column([button::Button(#{ text: "Run", on_click: Fn("noop") }), text("left pane")])
                .with_style(style().flex_grow().gap(px(8))),
            column([button::Button(#{ text: "Filter", size: "sm", on_click: Fn("noop") }), text("right pane")])
                .with_style(style().flex_grow().gap(px(8))),
        ]).with_style(style().gap(px(16)))"#,
    );
    assert!(panes.is_empty(), "{panes:?}");
    // The positive control: the same two controls in one row are a mismatch (the
    // row centers them; a stretching row would make them the same height).
    let row = audit(
        cx,
        r#"row([button::Button(#{ text: "Run", on_click: Fn("noop") }),
            button::Button(#{ text: "Filter", size: "sm", on_click: Fn("noop") })])
            .with_style(style().items_center().gap(px(8)))"#,
    );
    assert!(
        row.iter().any(|f| f.starts_with("row-height-mismatch")),
        "{row:?}"
    );
}

#[gpui::test]
fn a_badge_keeps_its_internal_gap_out_of_the_nesting_rule(cx: &mut TestAppContext) {
    let found = audit(
        cx,
        r#"row([
            badge::Badge(#{ text: "Healthy", variant: "neutral", dot: true, size: "sm" }),
            text("since 10:00"),
        ]).with_style(style().items_center().gap(theme_length("space.unit")))"#,
    );
    assert!(found.is_empty(), "{found:?}");
}

fn led_by(leader: &str, extra: &str) -> String {
    format!(
        r#"column([
            {leader},
            column([text("Section one"), text("Section two")]).with_style(style().gap(px(24))),
        ]){extra}.with_style(style().gap(px(8)))"#
    )
}

#[gpui::test]
fn content_led_by_a_heading_or_a_view_switcher_is_not_reported(cx: &mut TestAppContext) {
    let heading = audit(
        cx,
        &led_by(r#"text("Overview").accessibility_role("heading")"#, ""),
    );
    assert!(heading.is_empty(), "{heading:?}");
    let switcher = audit(
        cx,
        &led_by(
            r#"tabs::Tabs(#{ value: "a", label: "View", panel: false,
                tabs: [#{ value: "a", label: "A", content: box([]) }, #{ value: "b", label: "B", content: box([]) }],
                on_change: Fn("noop") })"#,
            "",
        ),
    );
    assert!(switcher.is_empty(), "{switcher:?}");
    // A container whose heading is drawn elsewhere (a host's panel header).
    let elsewhere = audit(cx, &led_by(r#"text("Body")"#, ".heading_elsewhere()"));
    assert!(elsewhere.is_empty(), "{elsewhere:?}");
    // A Region whose title is in a TitleBar or a tab says so with `external_title`.
    let region = |external: bool| {
        format!(
            r#"region::Region(#{{ label: "Hosts", fill: false, external_title: {external},
                body: column([text("Section one"), text("Section two")]).with_style(style().gap(px(48))),
                footer: [text("3 hosts")] }})"#
        )
    };
    let external = audit(cx, &region(true));
    assert!(external.is_empty(), "{external:?}");
    let untitled = audit(cx, &region(false));
    assert!(
        untitled.iter().any(|f| f.starts_with("spacing-not-nested")),
        "{untitled:?}"
    );
    // The positive control: plain text leading the same content is reported.
    let plain = audit(cx, &led_by(r#"text("Body")"#, ""));
    assert!(
        plain.iter().any(|f| f.starts_with("spacing-not-nested")),
        "{plain:?}"
    );
}

#[gpui::test]
fn the_panes_of_a_split_are_not_one_stack(cx: &mut TestAppContext) {
    // Two panes stacked by a vertical SplitPane whose first texts start 4px apart (#114
    // follow-up: a floated cell's placeholder above the next cell's title).
    let pane = |inset: u32, label: &str| {
        format!(r#"column([text("{label}")]).with_style(style().padding_start(px({inset})))"#)
    };
    let split = audit(
        cx,
        &format!(
            r#"split::SplitPane(#{{ key: "cells", label: "Cells", orientation: "vertical",
                size: 0.5, start: {}, end: {} }}).with_style(style().width(px(400)).height(px(300)))"#,
            pane(12, "placeholder"),
            pane(16, "Next cell")
        ),
    );
    assert!(split.is_empty(), "{split:?}");
    // The positive control: the same two blocks stacked in one column are misaligned.
    let column = audit(
        cx,
        &format!(
            "column([{}, {}]).with_style(style().gap(px(8)))",
            pane(12, "placeholder"),
            pane(16, "Next cell")
        ),
    );
    assert!(
        column.iter().any(|f| f.starts_with("text-edge-misaligned")),
        "{column:?}"
    );
}

#[gpui::test]
fn an_absolute_frame_does_not_stack_with_the_text_under_it(cx: &mut TestAppContext) {
    // A panel's focus frame overhangs its hairline by 1px (left -1, a 2px border), so its
    // edge sits 1px from a full-width line inside the panel, as with an embedded search.
    let panel = |position: &str| {
        format!(
            r#"column([
                row([text("Type a command")]).with_style(style().height(px(32)).border_bottom(px(1))),
                row([]).with_style(style().{position}.left(offset_px(-1)).top(px(0))
                    .width(px(40)).height(px(40)).border(px(2)).border_color(rgba(0x00000000))),
            ]).with_style(style().relative().width(px(300)).border(px(1)))"#
        )
    };
    let framed = audit(cx, &panel("absolute()"));
    assert!(framed.is_empty(), "{framed:?}");
    // The positive control: the same box in flow is a 1px near miss.
    let in_flow = audit(cx, &panel("relative()"));
    assert!(
        in_flow
            .iter()
            .any(|f| f.starts_with("text-edge-misaligned")),
        "{in_flow:?}"
    );
}
