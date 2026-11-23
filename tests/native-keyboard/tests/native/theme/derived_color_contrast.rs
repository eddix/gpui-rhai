//! Text on the two derived fills of the token base reaches 4.5:1 in every bundled theme
//! (#134), as the productivity profile's `low-contrast-text` audit sees it:
//!
//! - `tag.facet` fills the facet segment of a neutral Tag, whose text is `text_primary`
//!   (registry/components/tag.rhai). The fill is a tint toward that text, so it backs off
//!   toward `surface_hover` as far as 4.5:1 needs (registry/tokens.rhai).
//! - `collection.selection` fills selected List rows. Titles, secondary and meta
//!   text use `text_primary` there. Table owns `table.surface`, `table.text` and
//!   `table.selection` independently; its authored tint is not a generic list wash.
//!
//! `bundled_palettes_meet_the_theme_authoring_constraints` in
//! crates/gpui-rhai/tests/official_registry.rs checks the same pairs on the palettes. The
//! ratios use the runtime's own colour maths (`Rgba8::contrast_ratio`, the function the
//! audit uses) on the resolved theme tokens (token base + palette).

use gpui::TestAppContext;
use gpui_rhai::*;
use gpui_rhai_registry::{BUNDLED_PALETTE_SOURCES, BUNDLED_THEME_SOURCES, TOKEN_BASE_SOURCE};

use crate::support::{Mount, ReplaceOnce, Step, last_error, settle};

/// (foreground, derived background, where the pair is drawn).
const PAIRS: &[(&str, &str, &str)] = &[
    ("text_primary", "tag.facet", "neutral Tag facet text"),
    (
        "text_primary",
        "table.selection",
        "text on a selected Table row",
    ),
    (
        "text_primary",
        "collection.selection",
        "text on a selected List row",
    ),
];

fn theme(name: &str, source: &str) -> ThemeVariant {
    let engine = RuntimeEngine::new();
    let base = load_token_base(engine.engine(), "tokens.rhai", TOKEN_BASE_SOURCE).unwrap();
    load_theme_with_layers(
        engine.engine(),
        Some(&base),
        name,
        source,
        &ThemeTokenOverrides::default(),
    )
    .unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn text_on_derived_backgrounds_meets_4_5_in_every_bundled_theme() {
    let mut failures = Vec::new();
    println!(
        "| theme | {} |",
        PAIRS
            .iter()
            .map(|(fg, bg, _)| format!("{fg} on {bg}"))
            .collect::<Vec<_>>()
            .join(" | ")
    );
    for &(file, source) in BUNDLED_THEME_SOURCES {
        let theme = theme(file, source);
        let color = |token: &str| theme.tokens.color(token).unwrap();
        let mut cells = Vec::new();
        for &(foreground, background, place) in PAIRS {
            let painted_background = color(background).over(color("surface"));
            let ratio = color(foreground)
                .over(painted_background)
                .contrast_ratio(painted_background);
            let mark = if ratio < 4.5 { " FAIL" } else { "" };
            cells.push(format!(
                "{ratio:.2}{mark} (#{:06x})",
                painted_background.as_rgba_hex() >> 8
            ));
            if ratio < 4.5 {
                failures.push(format!(
                    "{} {}: {foreground} on {background} ({place}) is {ratio:.2}:1",
                    theme.family, theme.name
                ));
            }
        }
        println!(
            "| {} {} | {} |",
            theme.family,
            theme.name,
            cells.join(" | ")
        );
    }
    assert!(
        failures.is_empty(),
        "text on derived backgrounds is below 4.5:1:\n  {}",
        failures.join("\n  ")
    );
}

// The same thing as the productivity profile's audit sees it.

fn audit(cx: &mut TestAppContext, theme_source: &str, body: &str) -> Vec<String> {
    let main = format!(
        r#"import "components/tag" as tag;
import "components/list" as list;
import "components/table" as table;
import "components/button" as button;
import "components/badge" as badge;
import "components/input" as input;
import "components/checkbox" as checkbox;
import "components/radio" as radio;
fn noop(ctx, value) {{}}
fn view(ctx) {{ column([{body}]).with_style(style().width(px(360)).padding(px(8))) }}"#
    );
    let (mut visual, view) = Mount::components(main)
        .theme(theme_source)
        .motion(MotionPreference::None)
        .id("contrast")
        .mount(cx);
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(last_error(&mut visual, &view), None);
    let rules = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    visual.update(|_, cx| {
        view.composition_audit_with(&rules, cx)
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    })
}

fn audit_every_palette(cx: &mut TestAppContext, body: &str) {
    let mut failures = Vec::new();
    for &(file, source) in BUNDLED_PALETTE_SOURCES {
        let findings = audit(cx, source, body);
        println!("{file}: {findings:?}");
        if !findings.is_empty() {
            failures.push(format!("{file}: {findings:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "low-contrast-text findings:\n  {}",
        failures.join("\n  ")
    );
}

#[gpui::test]
fn exact_design_risks_and_broken_palette_controls_still_produce_findings(cx: &mut TestAppContext) {
    // Exact appearance and accessibility certification are different contracts.
    // A known small-text risk must remain visible, not be waived or recolored.
    let source = gpui_rhai_registry::MODERN_LIGHT_THEME;
    let found = audit(
        cx,
        source,
        r#"badge::Badge(#{text:"Ready",variant:"success"})"#,
    );
    assert!(
        !found.is_empty(),
        "exact design risk must remain reportable"
    );
    // A genuinely broken alternate palette must also fail the same native rule.
    let broken = format!(
        "{}\nfn theme(){{let result=palette_theme();result.tokens.colors.text_primary=result.tokens.colors.surface;result}}",
        gpui_rhai_registry::DEFAULT_LIGHT_THEME.replace_once("fn theme()", "fn palette_theme()")
    );
    let found = audit(
        cx,
        &broken,
        r#"text("Unreadable").with_style(style().typography("body").text_color(theme_color("text_primary")))"#,
    );
    assert!(
        !found.is_empty(),
        "palette calibration must not become an audit exemption"
    );
}

#[gpui::test]
fn a_neutral_faceted_tag_passes_the_contrast_audit_in_every_bundled_palette(
    cx: &mut TestAppContext,
) {
    audit_every_palette(cx, r#"tag::Tag(#{ facet: "region", text: "sg" })"#);
}

#[gpui::test]
fn every_tag_tone_and_facet_passes_the_native_audit_in_every_bundled_palette(
    cx: &mut TestAppContext,
) {
    let body = ["neutral", "accent", "success", "warning", "danger"]
        .into_iter()
        .flat_map(|tone| {
            [
                format!(r#"tag::Tag(#{{text:"{tone}",variant:"{tone}"}})"#),
                format!(r#"tag::Tag(#{{text:"{tone}",variant:"{tone}",facet:"kind"}})"#),
            ]
        })
        .collect::<Vec<_>>()
        .join(",");
    audit_every_palette(cx, &body);
}

#[gpui::test]
fn enabled_controls_and_status_fills_pass_in_every_bundled_palette(cx: &mut TestAppContext) {
    audit_every_palette(
        cx,
        r#"
        input::Input(#{key:"name",label:"Host",value:"edge-01",placeholder:"Enter a host"}),
        checkbox::Checkbox(#{checked:false,label:"Include archived",on_change:Fn("noop")}),
        radio::Radio(#{value:"current",selected:false,label:"Current revision",on_select:Fn("noop")}),
        button::Button(#{text:"Deploy",variant:"primary",on_click:Fn("noop")}),
        button::Button(#{text:"Delete",variant:"danger",on_click:Fn("noop")}),
        button::Button(#{text:"Review",variant:"warning",on_click:Fn("noop")}),
        button::Button(#{text:"Approve",variant:"success",on_click:Fn("noop")})
    "#,
    );
}

#[gpui::test]
fn status_pairs_stay_readable_on_selected_and_colored_containers(cx: &mut TestAppContext) {
    let badges = ["neutral", "info", "success", "warning", "danger", "purple"]
        .into_iter()
        .map(|tone| format!(r#"badge::Badge(#{{text:"{tone}",variant:"{tone}"}})"#))
        .collect::<Vec<_>>()
        .join(",");
    audit_every_palette(
        cx,
        &format!(
            r#"column([{badges},button::Button(#{{text:"Delete",variant:"danger",on_click:Fn("noop")}})])
                .with_style(style().background(theme_color("table.selection"))),
            column([{badges}]).with_style(style().background(theme_color("accent")))"#
        ),
    );
}

#[gpui::test]
fn a_selected_list_row_passes_the_contrast_audit_in_every_bundled_palette(cx: &mut TestAppContext) {
    audit_every_palette(
        cx,
        r#"list::List(#{ key: "hosts", label: "Hosts", height: 64.0,
            selection_mode: "single", selected_keys: ["a"],
            items: [#{ key: "a", title: "web-01", secondary: "sg-central", meta: "12:04" },
                    #{ key: "b", title: "web-02", secondary: "sg-central", meta: "12:05" }] })"#,
    );
}

#[gpui::test]
fn a_selected_table_row_passes_the_contrast_audit_in_every_bundled_palette(
    cx: &mut TestAppContext,
) {
    audit_every_palette(
        cx,
        r#"table::Table(#{ key: "hosts", label: "Hosts", row_key: "id", height: 64,
            selection_mode: "single", selected_keys: ["a"],
            columns: [#{ key: "host", title: "Host", width: #{ kind: "flex", value: 1.0 } },
                #{ key: "at", title: "At", width: #{ kind: "fixed", value: 80 }, numeric: true }],
            rows: [#{ id: "a", host: "web-01", at: "12:04" }, #{ id: "b", host: "web-02", at: "12:05" }] })"#,
    );
}
