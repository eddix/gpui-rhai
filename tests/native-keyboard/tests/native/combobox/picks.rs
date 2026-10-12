//! Combobox and Select picks: a click on an option and Enter on the active option report the
//! same `change` payload (the full next selection, built when the pick happens), `single`
//! closes and resets the query, `multiple` toggles and stays open, a searchable panel keeps
//! values hidden by the query in the selection, and Select reports scalar values.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::EN_LOCALE;

use crate::support::{Mount, ReplaceOnce, Step, bounds, click_center, last_error, text_of};

fn mount(cx: &mut TestAppContext, main: String) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(main)
        .locale("en.rhai", EN_LOCALE)
        .motion(MotionPreference::None)
        .id("picks")
        .mount(cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Tick, 2);
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    text_of(visual, view, "log").trim().to_owned()
}

fn no_error(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    let error = last_error(visual, view);
    assert!(error.is_none(), "{error:?}");
}

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let bounds = bounds(visual, view, role, name);
    click_center(visual, bounds);
    settle(visual);
}

fn keys(visual: &mut VisualTestContext, keystrokes: &str) {
    visual.simulate_keystrokes(keystrokes);
    settle(visual);
}

/// A Combobox whose selection, open state and query live in state (`selected` starts at
/// `initial`, `open` at `open`); `component` is the Combobox call, with `options`, `selected`,
/// `open` and `query` in scope. The log records `change` (as `show(values)`), `open_change`
/// and `query_change`.
fn script(options: &str, initial: &[&str], open: bool, show: &str, component: &str) -> String {
    let initial = initial
        .iter()
        .map(|value| format!(r#"#{{ type: "string", value: "{value}" }}"#))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"import "components/combobox" as combobox;
import "components/select" as select;
fn state_schema() {{ #{{ fields: #{{
    selected: #{{ schema: #{{ type: "array", max_items: 10000, items: #{{ type: "string" }} }},
        "default": #{{ type: "array", value: [{initial}] }} }},
    open: #{{ schema: #{{ type: "bool" }}, "default": #{{ type: "bool", value: {open} }} }},
    query: #{{ schema: #{{ type: "string" }}, "default": #{{ type: "string", value: "" }} }},
    log: #{{ schema: #{{ type: "string" }}, "default": #{{ type: "string", value: "" }} }} }} }} }}
fn note(ctx, line) {{ ctx.set_state("log", `${{ctx.get_state("log")}} ${{line}}`); }}
fn show(values) {{ {show} }}
fn changed(ctx, values) {{ ctx.set_state("selected", values); note(ctx, `change:${{show(values)}}`); }}
fn opened(ctx, open) {{ ctx.set_state("open", open); note(ctx, `open:${{open}}`); }}
fn queried(ctx, query) {{ ctx.set_state("query", query); note(ctx, `query:${{query}}`); }}
fn picked(ctx, value) {{
    ctx.set_state("selected", if value == () {{ [] }} else {{ [value] }});
    note(ctx, `change:${{if value == () {{ "()" }} else {{ value }}}}`);
}}
fn options() {{ {options} }}
fn view(ctx) {{
    let options = options();
    let selected = ctx.get_state("selected");
    let open = ctx.get_state("open");
    let query = ctx.get_state("query");
    column([
        {component},
        text(ctx.get_state("log")).test_id("log"),
    ]).with_style(style().width(px(400)).height(px(480)))
}}"#
    )
}

const ABC: &str = r#"[#{ value: "a", label: "Alpha" }, #{ value: "b", label: "Bravo" },
    #{ value: "c", label: "Charlie" }]"#;
const COMBOBOX: &str = r#"combobox::Combobox(#{ key: "region", label: "Region", mode: "__MODE__",
            options: options, selected: selected, open: open, query: query,
            reset_query_on_close: true, searchable: __SEARCHABLE__,
            on_change: Fn("changed"), on_open_change: Fn("opened"), on_query_change: Fn("queried") })"#;

fn combobox(mode: &str, searchable: bool) -> String {
    COMBOBOX
        .replace_once("__MODE__", mode)
        .replace_once("__SEARCHABLE__", &searchable.to_string())
}

/// `single`: a click and Enter both replace the selection, close the panel and reset the
/// query; picking the selected option again reports no change.
#[gpui::test]
fn single_mode_click_and_enter_report_the_same_payloads(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        script(ABC, &["a"], false, "values", &combobox("single", false)),
    );
    // The keys open the panel on the first option, move to Charlie and pick it.
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    keys(&mut visual, "down");
    keys(&mut visual, "down");
    keys(&mut visual, "down");
    keys(&mut visual, "enter");
    assert_eq!(
        log(&mut visual, &view),
        r#"open:true change:["c"] open:false query:"#
    );
    // A click opens the panel again; Bravo replaces Charlie.
    click(&mut visual, &view, "combobox", "Region");
    click(&mut visual, &view, "option", "Bravo");
    // Picking the selected option closes the panel without a change.
    click(&mut visual, &view, "combobox", "Region");
    click(&mut visual, &view, "option", "Bravo");
    no_error(&mut visual, &view);
    assert_eq!(
        log(&mut visual, &view),
        r#"open:true change:["c"] open:false query: open:true change:["b"] open:false query: open:true open:false query:"#
    );
}

/// `multiple`: a click and Enter toggle the value and keep the panel open, appending a newly
/// picked value after the ones already selected.
#[gpui::test]
fn multiple_mode_click_and_enter_toggle_and_stay_open(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        script(ABC, &["c"], true, "values", &combobox("multiple", false)),
    );
    click(&mut visual, &view, "option", "Alpha");
    // The click made Alpha active and left focus in the panel: Enter drops it again.
    keys(&mut visual, "enter");
    click(&mut visual, &view, "option", "Bravo");
    click(&mut visual, &view, "option", "Charlie");
    no_error(&mut visual, &view);
    assert_eq!(
        log(&mut visual, &view),
        r#"change:["c", "a"] change:["c"] change:["c", "b"] change:["b"]"#
    );
}

/// A searchable `multiple` panel over 500 options with 20 selected: picks from the filtered
/// list keep the selected values the query hides.
#[gpui::test]
fn a_searchable_multiple_panel_over_500_options_keeps_hidden_values(cx: &mut TestAppContext) {
    let options = r#"let result = []; for i in 0..500 { result.push(#{ value: `o${i}`, label: `Option ${i}` }); } result"#;
    let initial = (0..20).map(|i| format!("o{i}")).collect::<Vec<_>>();
    let initial = initial.iter().map(String::as_str).collect::<Vec<_>>();
    let show = r#"let kept = 0; for i in 0..20 { if `o${i}` in values { kept += 1; } }
        `${values.len}:${kept}:${if values.len == 0 { "-" } else { values[values.len - 1] }}`"#;
    // Twelve rows show the eleven matches at once.
    let component = combobox("multiple", true)
        .replace_once("searchable: true,", "searchable: true, max_visible: 12,");
    assert!(component.contains("max_visible: 12"));
    let (mut visual, view) = mount(cx, script(options, &initial, true, show, &component));
    no_error(&mut visual, &view);
    click(&mut visual, &view, "text_field", "Region");
    visual.simulate_input("Option 49");
    settle(&mut visual);
    let shown = crate::support::nodes(&mut visual, &view)
        .iter()
        .filter(|node| node.role == "option")
        .count();
    assert_eq!(shown, 11, "Option 49 and Option 490..499");
    click(&mut visual, &view, "option", "Option 49");
    keys(&mut visual, "enter");
    click(&mut visual, &view, "option", "Option 495");
    no_error(&mut visual, &view);
    // The search field reports the query as each character lands.
    let typed = (1..="Option 49".len())
        .map(|end| format!("query:{}", &"Option 49"[..end]))
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        log(&mut visual, &view),
        format!("{typed} change:21:20:o49 change:20:20:o19 change:21:20:o495")
    );
}

/// Select reports the picked value, or `()` when cleared, and resets the query on close.
#[gpui::test]
fn select_reports_scalar_values(cx: &mut TestAppContext) {
    let component = r#"select::Select(#{ key: "region", label: "Region", options: options,
            value: if selected.len == 0 { () } else { selected[0] }, open: open, query: query,
            clearable: true, clear_label: "Clear region",
            on_change: Fn("picked"), on_open_change: Fn("opened"), on_query_change: Fn("queried") })"#;
    let (mut visual, view) = mount(cx, script(ABC, &["a"], false, "values", component));
    click(&mut visual, &view, "combobox", "Region");
    click(&mut visual, &view, "option", "Charlie");
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    click(&mut visual, &view, "button", "Clear region");
    no_error(&mut visual, &view);
    assert_eq!(
        log(&mut visual, &view),
        "open:true change:c open:false query: change:() query:"
    );
}
