//! Combobox free-text submit (#139a): with `on_submit`, Enter in the search field submits the
//! query when no enabled option matches it, and the panel stays open; while an option is
//! active Enter still picks it.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::EN_LOCALE;

use crate::support::{Mount, Step, bounds, click_center, last_error, text_of};

fn mount(cx: &mut TestAppContext, main: String) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(main)
        .locale("en.rhai", EN_LOCALE)
        .motion(MotionPreference::None)
        .id("submit")
        .mount(cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Tick, 2);
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    assert_eq!(last_error(visual, view), None);
    text_of(visual, view, "log").trim().to_owned()
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
    let field = bounds(visual, view, "text_field", "Service");
    click_center(visual, field);
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

/// Without `on_submit`, Enter with nothing to pick reaches the search field's
/// input handler as a line break; a single-line field inserts nothing.
#[gpui::test]
fn without_on_submit_enter_with_nothing_to_pick_does_nothing(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(""));
    type_and_enter(&mut visual, &view, "svc-new");
    assert_eq!(log(&mut visual, &view), "");
}
