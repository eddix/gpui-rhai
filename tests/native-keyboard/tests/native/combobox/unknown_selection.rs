//! Repro: Combobox (and Select, which wraps it) throws when `selected` / `value` holds a
//! value that is not among `options`. `validate_combobox` in registry/components/combobox.rhai
//! ends with
//!
//!     for value in selected {
//!         if !(value in values) { throw `combobox selection '${value}' does not match an option`; }
//!     }
//!
//! so the whole view's render fails: the first render cannot mount, and a later render rolls
//! back to the last committed tree (other state changes in the same transaction are lost).
//! Transient states hit it: options loaded by an effect or task after the first render while
//! the selection is restored from settings, or options cleared while they reload.
//!
//! cargo test --manifest-path tests/native-keyboard/Cargo.toml --test native combobox::unknown_selection -- --nocapture

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, px};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME, EN_LOCALE,
    TOKEN_BASE_SOURCE,
};

use crate::support::ReplaceOnce;

struct Host {
    host: ScriptViewHost,
    view: Option<ScriptViewHandle>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let element = match &self.view {
            Some(view) if view.state() == ScriptViewState::Active => view.element().unwrap(),
            _ => gpui::div().into_any_element(),
        };
        self.host.container(element)
    }
}

/// Mounts `main`; returns the view, or the mount error.
fn mount(
    cx: &mut TestAppContext,
    main: &str,
) -> (VisualTestContext, Result<ScriptViewHandle, String>) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let mut modules = BTreeMap::from([(entry.clone(), main.to_owned())]);
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
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
        let host = ScriptViewHost::new("combo", cx).unwrap();
        let mounted = prepared.mount(ScriptViewConfig::new("combo"), host.clone(), window, cx);
        let view = mounted.as_ref().ok().cloned();
        *save.borrow_mut() = Some(mounted.map_err(|error| error.to_string()));
        Host { host, view }
    });
    cx.run_until_parked();
    let result = saved.borrow_mut().take().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    (visual, result)
}

fn settle(visual: &mut VisualTestContext) {
    for _ in 0..2 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn text_with_test_id(visual: &mut VisualTestContext, view: &ScriptViewHandle, id: &str) -> String {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_test_id(id)
            .next()
            .unwrap_or_else(|| panic!("no node with test id {id}"))
            .name
            .clone()
    })
}

fn press(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) {
    let bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("button", name)
            .next()
            .unwrap_or_else(|| panic!("no button {name}"))
            .geometry
            .unwrap()
            .visual
    });
    #[allow(clippy::cast_possible_truncation)]
    let at = gpui::point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    );
    visual.simulate_click(at, gpui::Modifiers::none());
    settle(visual);
}

/// `component` is a Combobox or Select whose options are `[a, b]` while `loaded` and `[]`
/// otherwise.
fn script(loaded: bool, component: &str) -> String {
    format!(
        r#"import "components/combobox" as combobox;
import "components/select" as select;
fn state_schema() {{ #{{ fields: #{{
    loaded: #{{ schema: #{{ type: "bool" }}, "default": #{{ type: "bool", value: {loaded} }} }},
    status: #{{ schema: #{{ type: "string" }}, "default": #{{ type: "string", value: "idle" }} }} }} }} }}
// Options are cleared while they reload; the selection the user made stays.
fn reload(ctx, payload) {{ ctx.set_state("loaded", false); ctx.set_state("status", "reloading"); }}
fn view(ctx) {{
    let options = if ctx.get_state("loaded") {{
        [#{{ value: "a", label: "Alpha" }}, #{{ value: "b", label: "Bravo" }}]
    }} else {{ [] }};
    column([
        text("Reload").accessibility_role("button").accessibility_label("Reload").on_click(Fn("reload"))
            .with_style(style().width(px(80)).height(px(32))),
        text(ctx.get_state("status")).test_id("status"),
        {component},
    ]).with_style(style().width(px(400)).height(px(300)))
}}"#
    )
}

const COMBOBOX: &str = r#"combobox::Combobox(#{ key: "region", label: "Region",
            options: options, selected: ["a"], open: false, query: "" })"#;
const SELECT: &str = r#"select::Select(#{ key: "region", label: "Region",
            options: options, value: "a", open: false, query: "" })"#;

/// Control: with its options present the view mounts without an error.
#[gpui::test]
fn control_a_combobox_with_its_options_mounts(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, &script(true, COMBOBOX));
    let view = view.expect("mount");
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    assert_eq!(text_with_test_id(&mut visual, &view, "status"), "idle");
}

/// Options that arrive after the first render (loaded by an effect or a task), while the
/// selection is restored from settings: the first render has `options: []`, `selected: ["a"]`.
#[gpui::test]
fn a_combobox_whose_options_arrive_after_the_first_render_mounts(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, &script(false, COMBOBOX));
    let view = view.unwrap_or_else(|error| panic!("the view did not mount: {error}"));
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
}

#[gpui::test]
fn a_select_whose_options_arrive_after_the_first_render_mounts(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, &script(false, SELECT));
    let view = view.unwrap_or_else(|error| panic!("the view did not mount: {error}"));
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
}

/// Options cleared while they reload: the render throws and the whole transaction rolls
/// back, so the `status` change made in the same callback is lost too.
#[gpui::test]
fn clearing_a_comboboxs_options_does_not_roll_back_the_view(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, &script(true, COMBOBOX));
    let view = view.expect("mount");
    press(&mut visual, &view, "Reload");
    let error = visual.update(|_, cx| view.last_error(cx).unwrap());
    let status = text_with_test_id(&mut visual, &view, "status");
    assert!(
        error.is_none() && status == "reloading",
        "status {status:?} (expected \"reloading\"), last_error {error:?}"
    );
}

#[gpui::test]
fn clearing_a_selects_options_does_not_roll_back_the_view(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, &script(true, SELECT));
    let view = view.expect("mount");
    press(&mut visual, &view, "Reload");
    let error = visual.update(|_, cx| view.last_error(cx).unwrap());
    let status = text_with_test_id(&mut visual, &view, "status");
    assert!(
        error.is_none() && status == "reloading",
        "status {status:?} (expected \"reloading\"), last_error {error:?}"
    );
}

// What an unknown selection shows and does once it no longer throws.

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name(role, name)
            .next()
            .unwrap_or_else(|| panic!("no {role} {name}"))
            .geometry
            .unwrap()
            .visual
    });
    #[allow(clippy::cast_possible_truncation)]
    let at = gpui::point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    );
    visual.simulate_click(at, gpui::Modifiers::none());
    settle(visual);
}

fn accessibility_value(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> Option<UiValue> {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name(role, name)
            .next()
            .unwrap_or_else(|| panic!("no {role} {name}"))
            .value
            .clone()
    })
}

fn has_text(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) -> bool {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("text", name)
            .next()
            .is_some()
    })
}

/// An open Combobox over `[a: Alpha, b: Bravo]` in `mode`, its selection in state starting at
/// `selected`; `extra` adds props. The log records each `change` payload.
fn interactive(mode: &str, selected: &[&str], extra: &str) -> String {
    let initial = selected
        .iter()
        .map(|value| format!(r#"#{{ type: "string", value: "{value}" }}"#))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"import "components/combobox" as combobox;
fn state_schema() {{ #{{ fields: #{{
    selected: #{{ schema: #{{ type: "array", items: #{{ type: "string" }} }},
        "default": #{{ type: "array", value: [{initial}] }} }},
    log: #{{ schema: #{{ type: "string" }}, "default": #{{ type: "string", value: "" }} }} }} }} }}
fn changed(ctx, values) {{
    ctx.set_state("selected", values);
    ctx.set_state("log", `${{ctx.get_state("log")}} change:${{values}}`);
}}
fn view(ctx) {{
    column([
        combobox::Combobox(#{{ key: "region", label: "Region", mode: "{mode}",
            options: [#{{ value: "a", label: "Alpha" }}, #{{ value: "b", label: "Bravo" }}],
            selected: ctx.get_state("selected"), open: true, query: "",
            on_change: Fn("changed"){extra} }}),
        text(ctx.get_state("log")).test_id("log"),
    ]).with_style(style().width(px(400)).height(px(300)))
}}"#
    )
}

/// The trigger shows an unknown value as its own label, so the user sees the stored
/// selection; the list has no row for it, and picking an option replaces it.
#[gpui::test]
fn an_unknown_single_selection_shows_its_raw_value(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, &interactive("single", &["zz"], ""));
    let view = view.expect("mount");
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    assert!(
        has_text(&mut visual, &view, "zz"),
        "the trigger shows the raw value"
    );
    assert_eq!(
        accessibility_value(&mut visual, &view, "combobox", "Region"),
        Some(UiValue::String("zz".to_owned()))
    );
    let options = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .filter(|node| node.role == "option")
            .map(|node| (node.name.clone(), node.selected))
            .collect::<Vec<_>>()
    });
    assert_eq!(
        options,
        [
            ("Alpha".to_owned(), Some(false)),
            ("Bravo".to_owned(), Some(false))
        ]
    );
    click(&mut visual, &view, "option", "Bravo");
    assert_eq!(
        text_with_test_id(&mut visual, &view, "log"),
        r#" change:["b"]"#
    );
    assert_eq!(
        accessibility_value(&mut visual, &view, "combobox", "Region"),
        Some(UiValue::String("Bravo".to_owned()))
    );
}

/// In `multiple` mode an unknown value stays in the selection the caller passed and in
/// every next selection the component reports.
#[gpui::test]
fn an_unknown_value_stays_in_a_multiple_selection(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, &interactive("multiple", &["a", "zz"], ""));
    let view = view.expect("mount");
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    assert_eq!(
        accessibility_value(&mut visual, &view, "combobox", "Region"),
        Some(UiValue::String("Alpha, zz".to_owned()))
    );
    click(&mut visual, &view, "option", "Bravo");
    click(&mut visual, &view, "option", "Alpha");
    assert_eq!(
        text_with_test_id(&mut visual, &view, "log"),
        r#" change:["a", "zz", "b"] change:["zz", "b"]"#
    );
    assert_eq!(
        accessibility_value(&mut visual, &view, "combobox", "Region"),
        Some(UiValue::String("Bravo, zz".to_owned()))
    );
}

/// `clearable` counts an unknown value as a selection, so it can be cleared.
#[gpui::test]
fn a_clearable_combobox_clears_an_unknown_selection(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        &interactive(
            "single",
            &["zz"],
            r#", clearable: true, clear_label: "Clear region""#,
        ),
    );
    let view = view.expect("mount");
    click(&mut visual, &view, "button", "Clear region");
    assert_eq!(text_with_test_id(&mut visual, &view, "log"), " change:[]");
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
}

/// Select reports an unknown value as its accessibility value and shows it in the trigger.
#[gpui::test]
fn a_select_shows_an_unknown_value(cx: &mut TestAppContext) {
    let select = SELECT.replace_once(r#"value: "a""#, r#"value: "zz""#);
    assert!(select.contains(r#"value: "zz""#));
    let source = script(true, &select);
    let (mut visual, view) = mount(cx, &source);
    let view = view.expect("mount");
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    assert!(
        has_text(&mut visual, &view, "zz"),
        "the trigger shows the raw value"
    );
    assert_eq!(
        accessibility_value(&mut visual, &view, "combobox", "Region"),
        Some(UiValue::String("zz".to_owned()))
    );
}

/// The real contract errors still throw: duplicate values and blank labels.
#[gpui::test]
fn duplicate_values_and_blank_labels_still_fail(cx: &mut TestAppContext) {
    for (options, message) in [
        (
            r#"[#{ value: "a", label: "Alpha" }, #{ value: "a", label: "Again" }]"#,
            "combobox option value 'a' is duplicated",
        ),
        (
            r#"[#{ value: "a", label: "  " }]"#,
            "combobox option 'a' has an empty label",
        ),
    ] {
        let source = script(true, COMBOBOX).replace_once(
            r#"[#{ value: "a", label: "Alpha" }, #{ value: "b", label: "Bravo" }]"#,
            options,
        );
        assert!(source.contains(options));
        let (_, view) = mount(cx, &source);
        let error = view.err().expect("the view does not mount");
        assert!(error.contains(message), "{error}");
    }
}
