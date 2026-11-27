//! Composition audit coverage and false positives found by the oh-my-byted
//! upgrade: content behind an error boundary and in virtual lists is audited
//! (#112); unrelated panes, a Badge's own gap, and content led by a view
//! switcher or a heading drawn elsewhere are not reported (#114); an absolutely
//! positioned focus frame does not stack with the text of a panel it is drawn over.
//! Each false positive case has a positive control that must still be reported.

use gpui::TestAppContext;
use gpui_rhai::MotionPreference;
use gpui_rhai_registry::BUNDLED_LAYOUT_SOURCES_BY_ID;

use crate::support::{Mount, Step, bounds, last_error, settle};

const PROFILE: &str = r#"fn profile() { #{ name: "test",
    audit: ["row-height-mismatch", "text-edge-misaligned", "spacing-not-nested", "mixed-type-in-row"] } }"#;

#[gpui::test]
fn grid_paper_changes_contrast_only_where_the_paint_order_and_geometry_allow_it(
    cx: &mut TestAppContext,
) {
    use crate::support::live_fills;
    use gpui::{px, size};
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    // covered dark text, actual low contrast against white paper, a
    // crossed edge, a rounded corner, an overlaid unknown sibling,
    // and a plate painted *after* the text.
    for (case, order, columns, indent, radius, glyph, offset, expected) in [
        ("covered", "[paper,front]", 2, 12, 0, "222222", 8, None),
        (
            "true risk",
            "[paper,front]",
            2,
            12,
            0,
            "b1b1b1",
            8,
            Some((false, Some([1.; 3]))),
        ),
        (
            "partial",
            "[paper,front]",
            1,
            108,
            0,
            "222222",
            8,
            Some((true, None)),
        ),
        (
            "corner",
            "[paper,front]",
            2,
            1,
            22,
            "222222",
            1,
            Some((true, None)),
        ),
        (
            "intervening",
            "[paper, blocker, front]",
            2,
            12,
            0,
            "222222",
            8,
            Some((true, None)),
        ),
        (
            "overflowing sibling",
            "[paper, blocker, front]",
            2,
            12,
            0,
            "222222",
            8,
            Some((true, None)),
        ),
        (
            "clipped sibling",
            "[paper, blocker, front]",
            2,
            12,
            0,
            "b1b1b1",
            8,
            Some((false, Some([1.; 3]))),
        ),
        (
            "painted after",
            "[front,paper]",
            2,
            12,
            0,
            "222222",
            8,
            Some((false, Some([0.; 3]))),
        ),
    ] {
        let source = r#"
            fn view(ctx){
                let paper=box([]).with_key("paper").accessibility_hidden(true)
                    .with_style(style().grid_cell(1,1).col_span(__COLS__)
                        .radius(px(__RAD__)).background(rgb(0xffffff)));
                let blocker=if "__CASE__"=="overflowing sibling"||"__CASE__"=="clipped sibling" {
                    let placement=style().grid_cell(2,1).relative().width(px(24)).height(px(48));
                    box([box([]).with_style(style().absolute().left(offset_px(-120)).top(px(0))
                        .width(px(180)).height(px(44)).background(rgb(0x000000)))])
                        .with_style(if "__CASE__"=="clipped sibling"{placement.clip()}else{placement})
                }else{
                    box([text("")]).with_style(style().grid_cell(1,1).col_span(2).background(rgb(0x000000)))
                };
                let blocker=blocker.with_key("blocker");
                let front=row([text("Audit sample").with_key("copy")
                    .with_style(style().font_size(px(13)).line_height(px(17)).text_color(rgb(0x__INK__)))])
                    .with_key("front").with_style(style().grid_cell(1,1).col_span(2)
                        .padding_start(px(__INSET__)).padding_top(px(__Y__)).items_start());
                box(__ORDER__).with_style(style().grid_cols(2).grid_rows(1).width(px(240)).height(px(48))
                    .background(rgb(0x000000)))
            }
        "#.replace("__CASE__",case).replace("__COLS__",&columns.to_string())
            .replace("__RAD__",&radius.to_string())
            .replace("__INK__",glyph)
            .replace("__INSET__",&indent.to_string())
            .replace("__Y__",&offset.to_string())
            .replace("__ORDER__",order);
        let (mut visual, view) = Mount::new(source).motion(MotionPreference::None).mount(cx);
        visual.simulate_resize(size(px(360.), px(180.)));
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(last_error(&mut visual, &view), None);
        assert!(
            live_fills(&mut visual, 0xffffffff)
                .iter()
                .any(|rect| rect.height >= 47. && rect.width >= 119.),
            "{case}"
        );
        if case == "overflowing sibling" {
            let copy = bounds(&mut visual, &view, "text", "Audit sample");
            assert!(
                live_fills(&mut visual, 0x000000ff).iter().any(|rect| {
                    (rect.width - 180.).abs() <= 1.
                        && (rect.height - 44.).abs() <= 1.
                        && f64::from(rect.x) < copy.x
                        && f64::from(rect.right()) > copy.x + copy.width
                }),
                "the overdraw actually reaches the text: {copy:?}"
            );
        }
        let found = visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
        let evidence = found
            .iter()
            .find(|finding| finding.path.ends_with("/copy"))
            .and_then(|finding| finding.contrast.as_ref());
        assert_eq!(
            evidence.map(|ink| (ink.conservative, ink.background_srgb)),
            expected,
            "{case}: {found:#?}"
        );
    }
}

fn audit(cx: &mut TestAppContext, view_body: &str) -> Vec<String> {
    let main = format!(
        r#"import "components/badge" as badge;
import "components/button" as button;
import "components/icon_button" as icon_button;
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
    let (mut visual, view) = Mount::components(main)
        .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .motion(MotionPreference::None)
        .with(|view| view.profile_source(PROFILE))
        .id("audit")
        .mount(cx);
    settle(&mut visual, Step::Frame, 2);
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
fn an_actionable_leaf_tree_item_owns_type_and_spacing_but_not_contrast(cx: &mut TestAppContext) {
    let script = r#"
fn activate(ctx,e){handled()}
fn cluster(key){
    row([text("Name").with_style(style().font_size(px(16)).text_color(rgb(0xb0b0b0))),
         text("42 ms").with_style(style().font_size(px(12)).text_color(rgb(0xb0b0b0)))])
        .with_key(key).with_style(style().gap_x(px(20)))
}
fn item(key) {
    row([text("›").with_style(style().font_size(px(16))),cluster("chrome")])
        .with_key(key).accessibility_role("treeitem").accessibility_label(key)
        .with_style(style().gap_x(px(4)))
}
fn view(ctx){
    column([
        item("action").on_click(Fn("activate")).tab_stop(false),
        item("key-action").on_key_value("plain+enter",Fn("activate"),()).tab_stop(true),
        item("passive"),
        row([cluster("inline"),layer(cluster("body"),#{id:"tree-popup",placement:"top_left",inset:40.0}).with_key("floating")])
            .with_key("floating-row").accessibility_role("treeitem").accessibility_label("Floating row")
            .on_click(Fn("activate")).with_style(style().gap_x(px(4))),
        row([item("branch-content"),column([item("nested")]).accessibility_role("group")])
            .with_key("branch").accessibility_role("treeitem").accessibility_label("branch")
            .on_click(Fn("activate"))
            .with_style(style().gap_x(px(8))),
    ]).accessibility_role("tree").accessibility_label("Trace")
      .with_style(style().width(px(680)).gap_y(px(50)).background(rgb(0xffffff)))
}
"#;
    let (mut visual, view) = Mount::new(script).motion(MotionPreference::None).mount(cx);
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(last_error(&mut visual, &view), None);
    let rules = gpui_rhai::AuditRules::from_ids([
        "mixed-type-in-row",
        "spacing-not-nested",
        "low-contrast-text",
    ])
    .unwrap();
    let findings = visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
    let reports = |role, path: &str| {
        findings
            .iter()
            .any(|f| f.rule == role && f.path.contains(path))
    };
    use gpui_rhai::AuditRule::{LowContrastText, MixedTypeInRow, SpacingNotNested};
    for key in ["/action/", "/key-action/"] {
        assert!(!reports(MixedTypeInRow, key), "{findings:#?}");
        assert!(!reports(SpacingNotNested, key), "{findings:#?}");
        assert!(reports(LowContrastText, key), "{findings:#?}");
    }
    for key in ["/passive/", "/branch/branch-content/"] {
        assert!(reports(MixedTypeInRow, key), "{findings:#?}");
        assert!(reports(SpacingNotNested, key), "{findings:#?}");
        assert!(reports(LowContrastText, key), "{findings:#?}");
    }
    assert!(reports(MixedTypeInRow, "/nested/"));
    assert!(!reports(MixedTypeInRow, "/floating-row/inline"));
    assert!(reports(MixedTypeInRow, "/floating-row/floating/content"));
    assert!(reports(LowContrastText, "/floating-row/floating/content"));
}

#[gpui::test]
fn tree_row_edges_follow_text_not_a_leading_icon_but_still_find_real_misalignment(
    cx: &mut TestAppContext,
) {
    for (pad, misaligned) in [(15, false), (17, true)] {
        let source = format!(
            r#"column([
                text("Summary").with_style(style().padding_start(px(0))),
                row([
                    row([box([]).with_style(style().width(px(6)).height(px(6)).background(rgb(0x335577)))])
                        .with_key("fold").accessibility_role("button").accessibility_label("Expand")
                        .with_style(style().width(px(8)).justify_center()).on_click(Fn("noop")),
                    text("Actual text")
                ]).with_key("row").accessibility_role("treeitem").accessibility_label("Actual text")
                  .on_click(Fn("noop")).with_style(style().gap_x(px(2)).padding_start(px(5))),
                row([text("Peer prose")]).with_style(style().padding_start(px({pad})))
            ]).with_style(style().gap_y(px(36)))"#
        );
        // The real row text starts 15px in. Use a two-pixel disagreement
        // only when desired; the leading glyph/button starts near the card.
        let findings = audit(cx, &source);
        let edges = findings
            .iter()
            .filter(|f| f.starts_with("text-edge-misaligned"))
            .collect::<Vec<_>>();
        assert_eq!(!edges.is_empty(), misaligned, "{edges:?}");
    }
}

#[gpui::test]
fn text_start_audit_uses_the_content_box_and_still_detects_padded_misalignment(
    cx: &mut TestAppContext,
) {
    let findings = audit(
        cx,
        r#"
        column([
            text("History").with_key("heading").with_style(style().padding_start(px(6))),
            row([text("Detail")]).with_style(style().padding_start(px(6))),
        ]).with_key("equal-content"),
        column([
            text("History").with_key("heading").with_style(style().padding_start(px(6))),
            text("Detail").with_key("body"),
        ]).with_key("unequal-content")
    "#,
    );
    let edges = findings
        .iter()
        .filter(|f| f.starts_with("text-edge-misaligned"))
        .collect::<Vec<_>>();
    assert!(
        !edges.iter().any(|f| f.contains("/equal-content:")),
        "{edges:?}"
    );
    assert!(
        edges
            .iter()
            .any(|f| f.contains("/unequal-content:") && f.contains("6.0px")),
        "{edges:?}"
    );
}

#[gpui::test]
fn text_start_audit_distinguishes_trailing_columns_and_single_edge_dividers(
    cx: &mut TestAppContext,
) {
    let findings = audit(
        cx,
        r#"
        column([
            text("Time").with_style(style().width(px(40))),
            text("Cost").with_style(style().width(px(46))),
        ]).with_key("aligned-at-end").with_style(style().items_end()),
        column([
            text("Time").with_style(style().width(px(40)).margin_end(px(4))),
            text("Cost").with_style(style().width(px(46))),
        ]).with_key("nudged-at-end").with_style(style().items_end()),
        column([
            row([text("Account")]).with_style(style().padding_start(px(6)).border_bottom(px(1))),
            row([text("Setup")]).with_style(style().margin_start(px(6)).border_top(px(1))),
        ]).with_key("section-dividers"),
        column([
            text("Summary").with_style(style().padding_start(px(6))),
            text("Step").accessibility_role("columnheader"),
        ]).with_key("column-caption-and-summary"),
        column([
            text("Steps").accessibility_role("columnheader").with_style(style().padding_start(px(6))),
            text("Rows").accessibility_role("columnheader"),
        ]).with_key("nudged-column-caption"),
        column([
            text("to").accessibility_role("label").with_style(style().padding_start(px(6))),
            text("Summary"),
        ]).with_key("label-and-prose"),
        column([
            text("to").accessibility_role("label").with_style(style().padding_start(px(6))),
            text("from").accessibility_role("label"),
        ]).with_key("nudged-field-label")
    "#,
    );
    let edges = findings
        .iter()
        .filter(|f| f.starts_with("text-edge-misaligned"))
        .collect::<Vec<_>>();
    for path in [
        "/aligned-at-end:",
        "/section-dividers:",
        "/column-caption-and-summary:",
        "/label-and-prose:",
    ] {
        assert!(!edges.iter().any(|f| f.contains(path)), "{edges:?}");
    }
    assert!(
        edges
            .iter()
            .any(|f| f.contains("/nudged-at-end:") && f.contains("ends differ by 4.0px")),
        "{edges:?}"
    );
    assert!(
        edges
            .iter()
            .any(|f| f.contains("/nudged-column-caption:") && f.contains("6.0px")),
        "{edges:?}"
    );
    assert!(
        edges
            .iter()
            .any(|f| f.contains("/nudged-field-label:") && f.contains("6.0px")),
        "{edges:?}"
    );
}

#[gpui::test]
fn text_starts_compare_like_edges_without_erasing_real_near_misses(cx: &mut TestAppContext) {
    let findings = audit(
        cx,
        r#"
        column([
            text("History"),
            row([text("All")]).with_key("filter").accessibility_role("radio")
                .with_style(style().margin_start(px(2)).padding_x(px(8)))
        ]).with_key("ink-and-filter"),
        column([
            row([text("First")]).with_key("first-tab").accessibility_role("tab")
                .with_style(style().margin_start(px(1))),
            row([text("Read report")]).with_key("tile").accessibility_role("button")
        ]).with_key("unrelated-controls"),
        column([
            text("Aligned prose").with_key("line-a"),
            row([text("Nudged prose").with_key("line-b")])
                .with_style(style().padding_start(px(2)))
        ]).with_key("prose-peers"),
        column([
            row([text("Confirm")]).with_key("primary").accessibility_role("button"),
            row([text("Dismiss")]).with_key("secondary").accessibility_role("button")
                .with_style(style().margin_start(px(2)))
        ]).with_key("button-peers"),
        column([
            row([text("Chart")]).with_key("card-a").with_style(style().background(rgb(0x335577))),
            row([text("Summary")]).with_key("card-b").with_style(style().background(rgb(0x335577))
                .margin_start(px(2)))
        ]).with_key("surface-peers")
        "#,
    );
    let edges: Vec<_> = findings
        .iter()
        .filter(|finding| finding.starts_with("text-edge-misaligned"))
        .collect();
    for path in ["/ink-and-filter:", "/unrelated-controls:"] {
        assert!(
            !edges.iter().any(|finding| finding.contains(path)),
            "{edges:?}"
        );
    }
    for (path, left, right) in [
        ("/prose-peers:", "line-a", "line-b"),
        ("/button-peers:", "primary", "secondary"),
        ("/surface-peers:", "card-a", "card-b"),
    ] {
        assert!(
            edges.iter().any(|finding| finding.contains(path)
                && finding.contains(left)
                && finding.contains(right)),
            "{edges:?}"
        );
    }
}

#[gpui::test]
fn display_text_has_its_own_peers_but_does_not_hide_an_inline_mismatch(cx: &mut TestAppContext) {
    let findings = audit(
        cx,
        r#"
        row([
            row([text("26.7")]).with_style(style().typography("display").font_size(px(28))),
            text("hours").with_style(style().typography("caption"))
        ]).with_key("metric-and-unit").with_style(style().items_center()),
        row([
            text("26.7").with_style(style().typography("display").font_size(px(28))),
            text("72.4").with_style(style().typography("display_large").font_size(px(26)))
        ]).with_key("display-peers").with_style(style().items_center()),
        row([
            text("26.7").with_style(style().typography("display").font_size(px(28))),
            text("Before").with_style(style().typography("caption").font_size(px(12))),
            text("After").with_style(style().typography("caption").font_size(px(14)))
        ]).with_key("inline-peers").with_style(style().items_center()),
        row([
            text("26.7").with_style(style().font_size(px(28))),
            text("72.4").with_style(style().font_size(px(26)))
        ]).with_key("untyped-peers").with_style(style().items_center())
        "#,
    );
    let type_findings: Vec<_> = findings
        .iter()
        .filter(|finding| finding.starts_with("mixed-type-in-row"))
        .collect();
    assert_eq!(type_findings.len(), 3, "{type_findings:?}");
    for (row, sizes) in [
        ("display-peers", "26px, 28px"),
        ("inline-peers", "12px, 14px"),
        ("untyped-peers", "26px, 28px"),
    ] {
        assert!(
            type_findings
                .iter()
                .any(|finding| finding.contains(row) && finding.ends_with(sizes)),
            "{type_findings:?}"
        );
    }
}

#[gpui::test]
fn metadata_and_monograms_keep_real_same_role_checks(cx: &mut TestAppContext) {
    let findings = audit(
        cx,
        r#"
        row([
            row([text("AR")]).with_style(style().typography("initials")),
            text("Ada Reyes").with_style(style().typography("body")),
            text("14:23").with_style(style().typography("metadata"))
        ]).with_key("authored-roles").with_style(style().items_center()),
        row([
            text("Ada Reyes").with_style(style().typography("body")),
            row([text("14:23")]).with_style(style().typography("metadata")),
            text("14:25").with_style(style().typography("metadata").font_size(px(14)))
        ]).with_key("metadata-mismatch").with_style(style().items_center()),
        row([
            text("AR").with_style(style().typography("initials")),
            text("LT").with_style(style().typography("initials").font_size(px(12)))
        ]).with_key("monogram-mismatch").with_style(style().items_center()),
        row([
            text("Unexpected role").with_style(style().typography("not_a_resolved_role").font_size(px(15))),
            text("Body text").with_style(style().typography("body").font_size(px(13)))
        ]).with_key("unresolved-is-inline").with_style(style().items_center())
        "#,
    );
    let type_findings: Vec<_> = findings
        .iter()
        .filter(|finding| finding.starts_with("mixed-type-in-row"))
        .collect();
    assert_eq!(type_findings.len(), 3, "{type_findings:?}");
    for (row, sizes) in [
        ("metadata-mismatch", "12px, 14px"),
        ("monogram-mismatch", "10px, 12px"),
        ("unresolved-is-inline", "13px, 15px"),
    ] {
        assert!(
            type_findings
                .iter()
                .any(|finding| finding.contains(row) && finding.ends_with(sizes)),
            "{type_findings:?}"
        );
    }
}

#[gpui::test]
fn wrapped_text_and_controls_only_compare_with_visual_peers(cx: &mut TestAppContext) {
    let script = r#"
        import "components/button" as button;
        fn view(ctx) { column([
            row([
                text("Compact caption").accessibility_role("text").accessibility_label("Small")
                    .with_style(style().width(px(120)).flex_shrink(false).font_size(px(12))),
                text("Headline").accessibility_role("text").accessibility_label("Large")
                    .with_style(style().width(px(120)).flex_shrink(false).font_size(px(24)))
            ]).with_key("labels").with_style(style().width(relative(1.0)).flex_wrap().gap(px(8))),
            row([
                button::Button(#{key:"short",text:"Short"}).with_style(style().width(px(120)).min_height(px(30)).height(px(30)).flex_shrink(false)),
                button::Button(#{key:"tall",text:"Tall"}).with_style(style().width(px(120)).min_height(px(56)).height(px(56)).flex_shrink(false))
            ]).with_key("buttons").with_style(style().width(relative(1.0)).flex_wrap().gap(px(8)))
        ]).with_style(style().width(relative(1.0)).gap(px(12))) }
    "#;
    let (mut visual, view) = Mount::components(script)
        .motion(MotionPreference::None)
        .with(|view| view.profile_source(PROFILE))
        .id("wrapped-audit")
        .mount(cx);
    for (width, wraps) in [(300., false), (156., true), (300., false)] {
        visual.simulate_resize(gpui::size(gpui::px(width), gpui::px(480.)));
        settle(&mut visual, Step::Tick, 3);
        assert_eq!(last_error(&mut visual, &view), None);
        for (role, small, large) in [("text", "Small", "Large"), ("button", "Short", "Tall")] {
            let first = bounds(&mut visual, &view, role, small);
            let second = bounds(&mut visual, &view, role, large);
            assert_eq!(
                second.y >= first.y + first.height,
                wraps,
                "{role}: {first:?} vs {second:?}"
            );
        }
        let findings = visual.update(|_, cx| view.composition_audit(cx)).unwrap();
        for (rule, path) in [
            (gpui_rhai::AuditRule::MixedTypeInRow, "labels"),
            (gpui_rhai::AuditRule::RowHeightMismatch, "buttons"),
        ] {
            assert_eq!(
                findings
                    .iter()
                    .any(|f| f.rule == rule && f.path.ends_with(path)),
                !wraps,
                "{width}/{rule:?}: {findings:?}"
            );
        }
    }
}

#[gpui::test]
fn grid_audit_compares_text_starts_inside_their_own_columns(cx: &mut TestAppContext) {
    for (pad, broken) in [(48, false), (46, true)] {
        let findings = audit(
            cx,
            &format!(
                r#"
                box([
                    column([text("I").with_key("upper-left")])
                        .with_style(style().grid_cell(1,1).min_width(px(0)).padding_start(px(48))),
                    column([text("I").with_key("upper-right")])
                        .with_style(style().grid_cell(2,1).min_width(px(0))),
                    column([text("I").with_key("lower-left")])
                        .with_style(style().grid_cell(1,2).min_width(px(0)).padding_start(px({pad}))),
                    column([text("I").with_key("lower-right")])
                        .with_style(style().grid_cell(2,2).min_width(px(0)))
                ]).with_key("grid").with_style(style().grid_cols(2).width(px(100)).gap_y(px(8)))
                "#
            ),
        );
        let edges = findings
            .iter()
            .filter(|f| f.starts_with("text-edge-misaligned"))
            .collect::<Vec<_>>();
        assert_eq!(!edges.is_empty(), broken, "{edges:?}");
        if broken {
            assert!(
                edges
                    .iter()
                    .any(|f| f.contains("upper-left") && f.contains("lower-left")),
                "{edges:?}"
            );
        }
    }
}

#[gpui::test]
fn grid_audit_checks_inline_type_and_control_heights_without_crossing_rows(
    cx: &mut TestAppContext,
) {
    for broken in [false, true] {
        let size = if broken { 24 } else { 12 };
        let height = if broken { 54 } else { 30 };
        let findings = audit(
            cx,
            &format!(
                r#"
                box([
                    text("A separate heading").accessibility_role("heading").accessibility_level(2)
                        .with_style(style().grid_cell(1,1).col_span(2).font_size(px(30))),
                    text("Name").with_style(style().grid_cell(1,2).font_size(px(12))),
                    text("Value").with_style(style().grid_cell(2,2).font_size(px({size}))),
                    button::Button(#{{key:"start",text:"Start"}})
                        .with_style(style().grid_cell(1,3).height(px(30)).min_height(px(0))),
                    button::Button(#{{key:"finish",text:"Finish"}})
                        .with_style(style().grid_cell(2,3).height(px({height})).min_height(px(0)))
                ]).with_key("grid").with_style(style().grid_cols(2).width(px(280)).gap(px(8)).items_start())
                "#
            ),
        );
        for rule in ["mixed-type-in-row", "row-height-mismatch"] {
            assert_eq!(
                findings.iter().any(|f| f.starts_with(rule)),
                broken,
                "{rule}: {findings:?}"
            );
        }
    }
}

#[gpui::test]
fn decorative_chrome_does_not_replace_a_content_anchor(cx: &mut TestAppContext) {
    for (child, has_text) in [
        (
            r#"box([]).with_style(style().width(px(16)).height(px(16)).background(rgb(0x335577)))"#,
            false,
        ),
        (r#"text("Visible caption")"#, true),
    ] {
        let findings = audit(
            cx,
            &format!(
                r#"
            column([
                text("Heading baseline").with_key("text"),
                row([{child}]).with_key("chrome").accessibility_hidden(true)
                    .with_style(style().padding_start(px(3)))
            ]).with_key("stack").with_style(style().gap_y(px(8)))
        "#
            ),
        );
        assert_eq!(
            findings
                .iter()
                .any(|f| f.starts_with("text-edge-misaligned")),
            has_text,
            "{findings:?}"
        );
    }
}

#[gpui::test]
fn auto_grid_audit_uses_the_presented_tracks_after_resize(cx: &mut TestAppContext) {
    let source = r#"
        fn view(ctx) { box([
            text("One").accessibility_role("text").accessibility_label("One").with_style(style().font_size(px(12))),
            text("Two").accessibility_role("text").accessibility_label("Two").with_style(style().font_size(px(24)))
        ]).with_style(style().width(relative(1.0)).auto_grid_columns(112,112,200).gap(px(8))) }
    "#;
    let (mut visual, view) = Mount::new(source)
        .motion(MotionPreference::None)
        .with(|view| view.profile_source(PROFILE))
        .mount(cx);
    let rules = gpui_rhai::AuditRules::from_ids(["mixed-type-in-row"]).unwrap();
    for (width, side_by_side) in [(300., true), (156., false), (300., true)] {
        visual.simulate_resize(gpui::size(gpui::px(width), gpui::px(240.)));
        settle(&mut visual, Step::Tick, 3);
        assert_eq!(last_error(&mut visual, &view), None);
        let first = bounds(&mut visual, &view, "text", "One");
        let second = bounds(&mut visual, &view, "text", "Two");
        assert_eq!(
            (first.y - second.y).abs() <= 0.5,
            side_by_side,
            "{first:?} {second:?}"
        );
        let findings = visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
        assert_eq!(!findings.is_empty(), side_by_side, "{width}: {findings:#?}");
    }
}

#[gpui::test]
fn icon_and_text_actions_are_distinct_but_peer_icon_heights_still_compare(cx: &mut TestAppContext) {
    for broken in [false, true] {
        let second = if broken { 72 } else { 36 };
        let findings = audit(
            cx,
            &format!(
                r#"row([
            button::Button(#{{text:"Run",on_click:Fn("noop")}}),
            icon_button::IconButton(#{{label:"First tool",icon:box([]),on_click:Fn("noop")}})
                .with_style(style().width(px(36)).height(px(36)).min_height(px(0))),
            icon_button::IconButton(#{{label:"Second tool",icon:box([]),on_click:Fn("noop")}})
                .with_style(style().width(px(36)).height(px({second})).min_height(px(0)))
        ]).with_style(style().items_center().gap(px(8)))"#
            ),
        );
        assert_eq!(
            findings
                .iter()
                .any(|finding| finding.starts_with("row-height-mismatch")),
            broken,
            "{findings:?}"
        );
    }
}

#[gpui::test]
fn wrapped_spacing_respects_individual_lines_without_losing_same_line_findings(
    cx: &mut TestAppContext,
) {
    let script = r#"
fn metrics(key,title){
    row([text("Left").with_style(style().width(px(30)).flex_shrink(false)),
         text("Right").with_style(style().width(px(30)).flex_shrink(false))])
        .with_key(key).accessibility_role("group").accessibility_label(title)
        .with_style(style().width(px(108)).height(px(32)).gap_x(px(24)).flex_shrink(false))
}
fn label(title,w){
    text(title).accessibility_role("text").accessibility_label(title)
        .with_style(style().width(px(w)).height(px(32)).flex_shrink(false))
}
fn view(ctx){column([
    row([label("Single heading",148),metrics("single","Single metrics")])
        .with_style(style().width(relative(1.0)).flex_wrap().gap_x(px(8)).gap_y(px(32))),
    row([label("Paired heading",148),metrics("paired","Paired metrics"),label("Other metrics",60)])
        .with_style(style().width(relative(1.0)).flex_wrap().gap_x(px(8)).gap_y(px(32)))
]).with_style(style().width(relative(1.0)).gap_y(px(50)))}
"#;
    let (mut v, view) = Mount::new(script)
        .motion(MotionPreference::None)
        .with(|view| view.profile_source(PROFILE))
        .mount(cx);
    for (width, wraps) in [(355., false), (182., true), (355., false)] {
        v.simulate_resize(gpui::size(gpui::px(width), gpui::px(540.)));
        settle(&mut v, Step::Tick, 3);
        assert_eq!(last_error(&mut v, &view), None);
        let a = bounds(&mut v, &view, "text", "Single heading");
        let b = bounds(&mut v, &view, "group", "Single metrics");
        let c = bounds(&mut v, &view, "text", "Paired heading");
        let d = bounds(&mut v, &view, "group", "Paired metrics");
        let e = bounds(&mut v, &view, "text", "Other metrics");
        assert_eq!(b.y >= a.y + a.height, wraps, "{a:?} {b:?}");
        assert_eq!(d.y >= c.y + c.height, wraps, "{c:?} {d:?}");
        assert!(
            (d.y - e.y).abs() < 1.,
            "the other metrics are a same-line peer: {d:?} {e:?}"
        );
        let facts = v.update(|_, cx| view.composition_audit(cx).unwrap());
        let reports = |path: &str| {
            facts
                .iter()
                .any(|f| f.rule == gpui_rhai::AuditRule::SpacingNotNested && f.path.ends_with(path))
        };
        assert_eq!(reports("/single"), !wraps, "{width}px: {facts:?}");
        assert!(
            reports("/paired"),
            "the real violation stays visible at {width}px: {facts:?}"
        );
    }
}

#[gpui::test]
fn reversed_wrapped_columns_clear_only_the_spacing_of_single_item_tracks(cx: &mut TestAppContext) {
    let script = r#"
fn metrics(key,title){
    column([text("Up").with_style(style().height(px(30)).flex_shrink(false)),
            text("Down").with_style(style().height(px(30)).flex_shrink(false))])
        .with_key(key).accessibility_role("group").accessibility_label(title)
        .with_style(style().width(px(64)).height(px(108)).gap_y(px(24)).flex_shrink(false))
}
fn label(title,h){
    text(title).accessibility_role("text").accessibility_label(title)
        .with_style(style().width(px(64)).height(px(h)).flex_shrink(false))
}
fn view(ctx){row([
    column([label("Single heading",148),metrics("single","Single metrics")])
        .with_style(style().height(px(182)).width(px(160)).flex_wrap_reverse().gap_x(px(16)).gap_y(px(8))),
    column([label("Paired heading",148),metrics("paired","Paired metrics"),label("Other metrics",60)])
        .with_style(style().height(px(182)).width(px(160)).flex_wrap_reverse().gap_x(px(16)).gap_y(px(8)))
]).with_style(style().gap_x(px(32)))}
"#;
    let (mut v, view) = Mount::new(script)
        .motion(MotionPreference::None)
        .with(|view| view.profile_source(PROFILE))
        .mount(cx);
    settle(&mut v, Step::Tick, 3);
    assert_eq!(last_error(&mut v, &view), None);
    let a = bounds(&mut v, &view, "text", "Single heading");
    let b = bounds(&mut v, &view, "group", "Single metrics");
    let c = bounds(&mut v, &view, "text", "Paired heading");
    let d = bounds(&mut v, &view, "group", "Paired metrics");
    let e = bounds(&mut v, &view, "text", "Other metrics");
    assert!(b.x + b.width <= a.x && d.x + d.width <= c.x);
    assert!((d.x - e.x).abs() < 1.0);
    let paths = v
        .update(|_, cx| view.composition_audit(cx).unwrap())
        .into_iter()
        .filter(|f| f.rule == gpui_rhai::AuditRule::SpacingNotNested)
        .map(|f| f.path)
        .collect::<Vec<_>>();
    assert_eq!(paths, ["root/1/paired"]);
}

#[gpui::test]
fn spacing_compares_content_groups_not_equal_steps_or_padded_trays(cx: &mut TestAppContext) {
    for (inner, padding, heading, broken) in [
        (8, 0, false, false),
        (12, 4, false, false),
        (24, 4, false, true),
        (24, 0, false, true),
        (24, 0, true, false),
    ] {
        let title = if heading {
            ".accessibility_role(\"heading\")"
        } else {
            ""
        };
        let source = format!(
            r#"column([
            text("Intro"){title},
            box([column([text("First"),text("Second")]).with_style(style().gap(px({inner})))])
                .with_style(style().padding_y(px({padding})))
        ]).with_style(style().gap(px(8)))"#
        );
        let findings = audit(cx, &source);
        assert_eq!(
            findings
                .iter()
                .any(|finding| finding.starts_with("spacing-not-nested")),
            broken,
            "inner={inner}, padding={padding}, heading={heading}: {findings:?}"
        );
    }
}

#[gpui::test]
fn distributed_chrome_slots_do_not_misread_a_gap_floor_as_content_proximity(
    cx: &mut TestAppContext,
) {
    let findings = audit(
        cx,
        r#"row([
        row([text("Theme"),text("Modern")]).with_style(style().gap(px(12)).flex_grow().flex_basis(relative(0))),
        row([text("Locale"),text("EN")]).with_style(style().gap(px(12)).flex_grow().flex_basis(relative(0)))
    ]).with_style(style().gap(px(8)))"#,
    );
    assert!(
        !findings
            .iter()
            .any(|finding| finding.starts_with("spacing-not-nested")),
        "{findings:?}"
    );
    let broken = audit(
        cx,
        r#"row([
        row([text("First"),text("Second")]).with_style(style().gap(px(20))),text("Next group")
    ]).with_style(style().gap(px(8)))"#,
    );
    assert!(
        broken
            .iter()
            .any(|finding| finding.starts_with("spacing-not-nested")),
        "{broken:?}"
    );
}

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
fn preset_families_can_have_distinct_heights_but_peer_controls_still_compare(
    cx: &mut TestAppContext,
) {
    for (name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for broken in [false, true] {
            let explicit_height = if broken {
                ".with_style(style().height(px(72)).min_height(px(72)))"
            } else {
                ""
            };
            let main = format!(
                r#"
                import "components/button" as button;
                import "components/input" as input;
                fn changed(ctx, value) {{ () }}
                fn view(ctx) {{ row([
                    button::Button(#{{text:"Run"}}),
                    input::Input(#{{key:"first",label:"First",value:"",on_change:Fn("changed")}}),
                    input::Input(#{{key:"second",label:"Second",value:"",on_change:Fn("changed")}}){explicit_height}
                ]).with_style(style().items_center().width(px(600))) }}
            "#
            );
            let (mut visual, view) = Mount::components(main)
                .theme(*theme)
                .motion(MotionPreference::None)
                .with(|view| view.profile_source(PROFILE))
                .id("audit-families")
                .mount(cx);
            settle(&mut visual, Step::Frame, 2);
            let findings = visual.update(|_, cx| view.composition_audit(cx)).unwrap();
            assert_eq!(
                findings
                    .iter()
                    .any(|finding| finding.rule == gpui_rhai::AuditRule::RowHeightMismatch),
                broken,
                "{name}: {findings:?}"
            );
        }
    }
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
fn an_inert_pictogram_can_precede_a_heading_but_content_cannot(cx: &mut TestAppContext) {
    let svg = r#"svg("<svg viewBox='0 0 16 16'><rect x='0' y='0' width='16' height='16'/></svg>")
        .with_style(style().width(px(16)).height(px(16)))"#;
    let cases = [
        (format!("{svg}.accessibility_hidden(true)"), true),
        (
            format!("{svg}.accessibility_role(\"image\").accessibility_label(\"Latency plot\")"),
            false,
        ),
        (
            r#"text("Body first").accessibility_hidden(true)"#.to_owned(),
            false,
        ),
        (
            format!("{svg}.accessibility_hidden(true).on_click(Fn(\"noop\"))"),
            false,
        ),
    ];
    for (leader, heading_led) in cases {
        let row = format!(
            r#"row([
              row([{leader}, text("Overview").accessibility_role("heading")
                .accessibility_level(2).with_style(style().font_size(px(16)))])
                  .with_style(style().items_center().gap(px(4))),
              row([text("First"), text("Second")]).with_style(style().gap(px(24))),
            ]).with_style(style().items_center().gap(px(8)).font_size(px(12)))"#
        );
        let findings = audit(cx, &row);
        let has = |rule: &str| findings.iter().any(|f| f.starts_with(rule));
        assert_eq!(
            has("spacing-not-nested"),
            !heading_led,
            "{leader}: {findings:?}"
        );
        assert_eq!(
            has("mixed-type-in-row"),
            !heading_led,
            "{leader}: {findings:?}"
        );
    }
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
fn absolute_items_are_audited_without_joining_the_enclosing_row(cx: &mut TestAppContext) {
    use gpui_rhai::{AuditRule, AuditRules};
    let rules = AuditRules::from_ids([
        "mixed-type-in-row",
        "row-height-mismatch",
        "low-contrast-text",
        "multiple-solid-actions",
    ])
    .unwrap();
    for (position, peers) in [("absolute()", false), ("relative()", true)] {
        let script = format!(
            r#"
import "components/button" as button;
fn view(ctx) {{
    column([
        row([
            text("Monogram").with_style(style().font_size(px(12))),
            text("Ordinal").with_key("ordinal").with_style(style().{position}.left(px(75)).top(px(0))
                .font_size(px(18)).text_color(rgb(0xbbbbbb)))
        ]).with_key("monogram").with_style(style().relative().width(px(250)).height(px(44)).items_start()),
        row([
            button::Button(#{{key:"short",text:"Short"}}).with_style(style().height(px(30)).min_height(px(0))),
            button::Button(#{{key:"tall",text:"Tall"}}).with_style(style().{position}.right(px(0)).top(px(0))
                .height(px(54)).min_height(px(0)))
        ]).with_key("actions").with_style(style().relative().width(px(250)).height(px(54)).items_start()),
        row([
            button::Button(#{{key:"apply",text:"Apply"}}),
            box([button::Button(#{{key:"save",text:"Save"}}).with_style(style().{position}.left(px(0)))])
                .with_style(style().relative().width(px(90)).height(px(40)))
        ]).with_key("floating-actions").with_style(style().width(px(250)))
    ]).with_style(style().background(rgb(0xffffff)))
}}
"#
        );
        let (mut v, view) = Mount::components(script)
            .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
            .motion(MotionPreference::None)
            .mount(cx);
        settle(&mut v, Step::Tick, 3);
        assert_eq!(last_error(&mut v, &view), None);
        let findings = v.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
        assert_eq!(
            findings
                .iter()
                .any(|f| f.rule == AuditRule::MixedTypeInRow && f.path.ends_with("/monogram")),
            peers,
            "{position}: {findings:#?}"
        );
        assert_eq!(
            findings
                .iter()
                .any(|f| f.rule == AuditRule::RowHeightMismatch && f.path.ends_with("/actions")),
            peers,
            "{position}: {findings:#?}"
        );
        assert!(
            findings
                .iter()
                .any(|f| f.rule == AuditRule::LowContrastText && f.path.ends_with("/ordinal")),
            "{position}: {findings:#?}"
        );
        assert!(
            findings
                .iter()
                .any(|f| f.rule == AuditRule::MultipleSolidActions
                    && f.path.ends_with("/floating-actions")),
            "{position}: {findings:#?}"
        );
    }
}

#[gpui::test]
fn an_absolute_frame_does_not_stack_with_the_text_under_it(cx: &mut TestAppContext) {
    // A panel's focus frame overhangs by 1px (left -1, a 2px border), near
    // a full-width surface inside the panel, as with an embedded search.
    let panel = |position: &str| {
        format!(
            r#"column([
                row([text("Type a command")]).with_style(style().height(px(32))
                    .background(rgb(0xeeeeee)).border_bottom(px(1))),
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
