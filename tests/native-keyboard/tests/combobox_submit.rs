//! Combobox free-text submit (#139a): with `on_submit`, Enter in the search field submits the
//! query when no enabled option matches it, and the panel stays open; while an option is
//! active Enter still picks it.

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
        let host = ScriptViewHost::new("submit", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("submit"), host.clone(), window, cx)
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
    for _ in 0..2 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    visual.update(|_, cx| {
        assert_eq!(view.last_error(cx).unwrap(), None);
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_test_id("log")
            .next()
            .unwrap()
            .name
            .trim()
            .to_owned()
    })
}

/// An open, searchable `single` Combobox over Alpha, Bravo and a disabled Zulu, its query and
/// selection in state; `extra` adds props. The log records `change`, `open_change` and
/// `submit`, not `query_change`.
fn script(extra: &str) -> String {
    format!(
        r#"import "components/combobox" as combobox;
fn state_schema() {{ #{{ fields: #{{
    selected: #{{ schema: #{{ type: "array", items: #{{ type: "string" }} }},
        "default": #{{ type: "array", value: [] }} }},
    open: #{{ schema: #{{ type: "bool" }}, "default": #{{ type: "bool", value: true }} }},
    query: #{{ schema: #{{ type: "string" }}, "default": #{{ type: "string", value: "" }} }},
    log: #{{ schema: #{{ type: "string" }}, "default": #{{ type: "string", value: "" }} }} }} }} }}
fn note(ctx, line) {{ ctx.set_state("log", `${{ctx.get_state("log")}} ${{line}}`); }}
fn changed(ctx, values) {{ ctx.set_state("selected", values); note(ctx, `change:${{values}}`); }}
fn opened(ctx, open) {{ ctx.set_state("open", open); note(ctx, `open:${{open}}`); }}
fn queried(ctx, query) {{ ctx.set_state("query", query); }}
fn submitted(ctx, query) {{ note(ctx, `submit:${{query}}`); }}
fn view(ctx) {{
    column([
        combobox::Combobox(#{{ key: "service", label: "Service", searchable: true,
            options: [#{{ value: "a", label: "Alpha" }}, #{{ value: "b", label: "Bravo" }},
                #{{ value: "z", label: "Zulu", disabled: true }}],
            selected: ctx.get_state("selected"), open: ctx.get_state("open"),
            query: ctx.get_state("query"),
            on_change: Fn("changed"), on_open_change: Fn("opened"),
            on_query_change: Fn("queried") {extra} }}),
        text(ctx.get_state("log")).test_id("log"),
    ]).with_style(style().width(px(400)).height(px(480)))
}}"#
    )
}

/// Focus the search field, type `query` and press Enter.
fn type_and_enter(visual: &mut VisualTestContext, view: &ScriptViewHandle, query: &str) {
    let bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("text_field", "Service")
            .next()
            .expect("no search field")
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
    visual.simulate_input(query);
    settle(visual);
    visual.simulate_keystrokes("enter");
    settle(visual);
}

#[gpui::test]
fn enter_with_no_matching_option_submits_the_query(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(r#", on_submit: Fn("submitted")"#));
    type_and_enter(&mut visual, &view, "svc-new");
    assert_eq!(
        log(&mut visual, &view),
        "submit:svc-new",
        "the query is submitted, nothing is picked and the panel stays open"
    );
}

#[gpui::test]
fn a_query_that_matches_only_a_disabled_option_submits(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(r#", on_submit: Fn("submitted")"#));
    type_and_enter(&mut visual, &view, "zu");
    assert_eq!(log(&mut visual, &view), "submit:zu");
}

#[gpui::test]
fn enter_with_an_active_option_still_picks_it(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(r#", on_submit: Fn("submitted")"#));
    type_and_enter(&mut visual, &view, "br");
    assert_eq!(log(&mut visual, &view), r#"change:["b"] open:false"#);
}
