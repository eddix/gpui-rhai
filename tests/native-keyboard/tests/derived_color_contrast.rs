//! Text on the two derived fills of the token base reaches 4.5:1 in every bundled theme
//! (#134), as the productivity profile's `low-contrast-text` audit sees it:
//!
//! - `tag.facet` fills the facet segment of a neutral Tag, whose text is `text_primary`
//!   (registry/components/tag.rhai). The fill is a tint toward that text, so it backs off
//!   toward `surface_hover` as far as 4.5:1 needs (registry/tokens.rhai).
//! - `table.selection` fills selected Table and List rows. Table cells and List titles are
//!   `text_primary`; on a selected List row the `secondary` and `meta` text take
//!   `text_primary` too (registry/components/list.rhai), as `text_muted` is tuned for the
//!   surfaces. The tint backs off toward `surface` as far as 4.5:1 for `text_primary` needs.
//!
//! `bundled_palettes_meet_the_theme_authoring_constraints` in
//! crates/gpui-rhai/tests/official_registry.rs checks the same pairs on the palettes. The
//! ratios use the runtime's own colour maths (`Rgba8::contrast_ratio`, the function the
//! audit uses) on the resolved theme tokens (token base + palette).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, BUNDLED_THEME_SOURCES,
    TOKEN_BASE_SOURCE,
};

/// (foreground, derived background, where the pair is drawn).
const PAIRS: &[(&str, &str, &str)] = &[
    ("text_primary", "tag.facet", "neutral Tag facet text"),
    (
        "text_primary",
        "table.selection",
        "text on a selected Table / List row",
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
            let ratio = color(foreground).contrast_ratio(color(background));
            let mark = if ratio < 4.5 { " FAIL" } else { "" };
            cells.push(format!(
                "{ratio:.2}{mark} (#{:06x})",
                color(background).as_rgba_hex() >> 8
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

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn audit(cx: &mut TestAppContext, theme_source: &str, body: &str) -> Vec<String> {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let mut modules = BTreeMap::from([(
        entry.clone(),
        format!(
            r#"import "components/tag" as tag;
import "components/list" as list;
import "components/table" as table;
fn view(ctx) {{ column([{body}]).with_style(style().width(px(360)).padding(px(8))) }}"#
        ),
    )]);
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let prepared = EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), theme_source)
        .token_base(TOKEN_BASE_SOURCE)
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
        let host = ScriptViewHost::new("contrast", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("contrast"), host.clone(), window, cx)
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
    let rules = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    visual.update(|_, cx| {
        assert_eq!(view.last_error(cx).unwrap(), None);
        view.composition_audit_with(&rules, cx)
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    })
}

fn audit_every_theme(cx: &mut TestAppContext, body: &str) {
    let mut failures = Vec::new();
    for &(file, source) in BUNDLED_THEME_SOURCES {
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
fn a_neutral_faceted_tag_passes_the_contrast_audit_in_every_bundled_theme(cx: &mut TestAppContext) {
    audit_every_theme(cx, r#"tag::Tag(#{ facet: "region", text: "sg" })"#);
}

#[gpui::test]
fn a_selected_list_row_passes_the_contrast_audit_in_every_bundled_theme(cx: &mut TestAppContext) {
    audit_every_theme(
        cx,
        r#"list::List(#{ key: "hosts", label: "Hosts", height: 64.0,
            selection_mode: "single", selected_keys: ["a"],
            items: [#{ key: "a", title: "web-01", secondary: "sg-central", meta: "12:04" },
                    #{ key: "b", title: "web-02", secondary: "sg-central", meta: "12:05" }] })"#,
    );
}

#[gpui::test]
fn a_selected_table_row_passes_the_contrast_audit_in_every_bundled_theme(cx: &mut TestAppContext) {
    audit_every_theme(
        cx,
        r#"table::Table(#{ key: "hosts", label: "Hosts", row_key: "id", height: 64,
            selection_mode: "single", selected_keys: ["a"],
            columns: [#{ key: "host", title: "Host", width: #{ kind: "flex", value: 1.0 } },
                #{ key: "at", title: "At", width: #{ kind: "fixed", value: 80 }, numeric: true }],
            rows: [#{ id: "a", host: "web-01", at: "12:04" }, #{ id: "b", host: "web-02", at: "12:05" }] })"#,
    );
}
