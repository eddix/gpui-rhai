//! Keyboard contract: every interactive component is one tab stop, unless it
//! is a set of independent targets (a closable Tag, Pagination, RangeSlider caps,
//! Accordion headers). Phantom stops (wrappers that take focus without showing it)
//! make Tab feel broken, so the census below is exact.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, BUNDLED_LAYOUT_SOURCES_BY_ID,
    BUNDLED_PATTERN_SOURCES_BY_ID, DEFAULT_THEME, EN_LOCALE, TOKEN_BASE_SOURCE,
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

/// Mount `body` (a Rhai expression using `noop`) and count distinct tab stops.
fn tab_stops(cx: &mut TestAppContext, imports: &str, body: &str) -> usize {
    cx.update(gpui_rhai::install);
    let main =
        format!("{imports}\nfn noop(ctx, payload) {{}}\nfn view(ctx) {{ column([{body}]) }}\n");
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .chain(BUNDLED_PATTERN_SOURCES_BY_ID)
    {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    modules.insert(ModuleId::parse("main").unwrap(), main);
    let prepared = EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(modules),
        DEFAULT_THEME,
    )
    .token_base(TOKEN_BASE_SOURCE)
    .locale_sources([("en.rhai".to_owned(), EN_LOCALE.to_owned())])
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
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("tab-stops", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("tab-stops"), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    // Frames realize virtual rows and overlay content, so stops inside them count.
    for _ in 0..2 {
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
    let mut seen: Vec<gpui::FocusHandle> = Vec::new();
    for _ in 0..32 {
        visual.update(|window, cx| window.focus_next(cx));
        visual.run_until_parked();
        let Some(focused) = visual.update(|window, cx| window.focused(cx)) else {
            break;
        };
        if seen.contains(&focused) {
            break;
        }
        seen.push(focused);
    }
    seen.len()
}

#[gpui::test]
fn interactive_components_are_one_tab_stop_each(cx: &mut TestAppContext) {
    let cases: &[(&str, &str, &str, usize)] = &[
        (
            "button",
            r#"import "components/button" as c;"#,
            r#"c::Button(#{ text: "Deploy", on_click: Fn("noop") })"#,
            1,
        ),
        (
            "icon_button",
            r#"import "components/icon_button" as c;"#,
            r#"c::IconButton(#{ label: "Help", icon: image_source(asset("app/icons/help")), on_click: Fn("noop") })"#,
            1,
        ),
        (
            "toggle",
            r#"import "components/toggle" as c;"#,
            r#"c::Toggle(#{ text: "Bold", pressed: false, on_pressed_change: Fn("noop") })"#,
            1,
        ),
        (
            "toggle_group",
            r#"import "components/toggle_group" as c;"#,
            r#"c::ToggleGroup(#{ key: "g", label: "View", values: ["a"], items: [#{ value: "a", label: "A" }, #{ value: "b", label: "B" }], on_change: Fn("noop") })"#,
            1,
        ),
        (
            "input",
            r#"import "components/input" as c;"#,
            r#"c::Input(#{ key: "i", label: "Name", value: "", on_change: Fn("noop") })"#,
            1,
        ),
        (
            "select",
            r#"import "components/select" as c;"#,
            r#"c::Select(#{ key: "s", label: "Region", value: "a", open: false, query: "", options: [#{ value: "a", label: "A" }], on_change: Fn("noop"), on_open_change: Fn("noop") })"#,
            1,
        ),
        (
            "combobox",
            r#"import "components/combobox" as c;"#,
            r#"c::Combobox(#{ key: "c", label: "Theme", options: [#{ value: "a", label: "A" }], selected: [], open: false, query: "", on_change: Fn("noop"), on_open_change: Fn("noop") })"#,
            1,
        ),
        (
            "date_picker",
            r#"import "components/date_picker" as c;"#,
            r#"c::DatePicker(#{ key: "d", label: "Date", value: (), on_change: Fn("noop") })"#,
            1,
        ),
        (
            "checkbox",
            r#"import "components/checkbox" as c;"#,
            r#"c::Checkbox(#{ checked: false, label: "Accept", on_change: Fn("noop") })"#,
            1,
        ),
        (
            "radio_group",
            r#"import "components/radio_group" as c;"#,
            r#"c::RadioGroup(#{ value: "a", label: "Plan", options: [#{ value: "a", label: "A" }, #{ value: "b", label: "B" }], on_change: Fn("noop") })"#,
            1,
        ),
        (
            "switch",
            r#"import "components/switch" as c;"#,
            r#"c::Switch(#{ checked: false, label: "Alerts", on_change: Fn("noop") })"#,
            1,
        ),
        (
            "slider",
            r#"import "components/slider" as c;"#,
            r#"c::Slider(#{ key: "s", label: "Volume", value: 4.0, min: 0.0, max: 10.0, on_change: Fn("noop") })"#,
            1,
        ),
        (
            "tabs",
            r#"import "components/tabs" as c;"#,
            r#"c::Tabs(#{ value: "a", label: "Views", panel: false, tabs: [#{ value: "a", label: "A" }, #{ value: "b", label: "B" }], on_change: Fn("noop") })"#,
            1,
        ),
        (
            "popover",
            r#"import "components/popover" as c; import "components/button" as b;"#,
            r#"c::Popover(#{ key: "p", label: "Filters", open: false, trigger: b::Button(#{ text: "Filters", on_click: Fn("noop") }), content: text("x"), on_open_change: Fn("noop") })"#,
            1,
        ),
        (
            "tooltip",
            r#"import "components/tooltip" as c; import "components/button" as b;"#,
            r#"c::Tooltip(#{ key: "t", label: "Deploy", trigger: b::Button(#{ text: "Deploy", on_click: Fn("noop") }), content: text("x") })"#,
            1,
        ),
        (
            "dialog_closed",
            r#"import "components/dialog" as c;"#,
            r#"c::Dialog(#{ key: "d", open: false, title: "Rename", content: text("x"), on_open_change: Fn("noop") })"#,
            0,
        ),
        (
            "table",
            r#"import "components/table" as c;"#,
            r#"c::Table(#{ key: "t", label: "Hosts", row_key: "id", height: 120, selection_mode: "single", selected_keys: [], columns: [#{ key: "h", title: "Host", width: #{ kind: "flex", value: 1.0 } }], rows: [#{ id: "a", h: "a" }, #{ id: "b", h: "b" }], on_selection_change: Fn("noop") })"#,
            1,
        ),
        (
            "accordion",
            r#"import "components/accordion" as c;"#,
            r#"c::Accordion(#{ key: "a", expanded: [], items: [#{ key: "x", title: "X", content: text("x"), content_height: 20 }, #{ key: "y", title: "Y", content: text("y"), content_height: 20 }], on_change: Fn("noop") })"#,
            2,
        ),
        (
            // Each region is a stop before its control: it owns the focus its frame shows.
            "app_shell",
            r#"import "patterns/app_shell" as c; import "components/button" as b;"#,
            r#"c::AppShell(#{ key: "shell", label: "Tool",
                sidebar: b::Button(#{ text: "Nav", on_click: Fn("noop") }),
                main: b::Button(#{ text: "Main", on_click: Fn("noop") }),
                inspector: b::Button(#{ text: "Info", on_click: Fn("noop") }) })"#,
            6,
        ),
        (
            "tag_closable",
            r#"import "components/tag" as c;"#,
            r#"c::Tag(#{ text: "rust", closable: true, on_close: Fn("noop") })"#,
            1,
        ),
    ];
    let mut report = Vec::new();
    for (name, imports, body, expected) in cases {
        let count = tab_stops(cx, imports, body);
        report.push(format!("{name}: {count} (expected {expected})"));
        if count != *expected {
            report.push("  ^ MISMATCH".to_owned());
        }
    }
    let text = report.join("\n");
    println!("{text}");
    assert!(!text.contains("MISMATCH"), "{text}");
}
