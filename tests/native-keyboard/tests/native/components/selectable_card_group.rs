//! Rich choice surface focus/selection and independent actions under real input.
use gpui::TestAppContext;
use gpui_rhai::UiValue;
use gpui_rhai_registry::{AR_LOCALE, DEFAULT_LIGHT_THEME, DEFAULT_THEME, EN_LOCALE};

use crate::support::{
    Mount, ReplaceOnce, bounds, by_test_id, click_center, dispatch, last_error, nodes,
    text_of,
};

const SOURCE: &str = r#"
import "components/selectable_card_group" as cards;
import "components/button" as button;
fn state_schema() { #{ fields: #{
    values: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "string" } }, "default": #{ type: "array", value: [#{ type: "string", value: "a" }] } },
    changes: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
    actions: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
} } }
fn changed(ctx, values) { ctx.set_state("values", values); ctx.set_state("changes", ctx.get_state("changes")+1); }
fn action(ctx, payload) { ctx.set_state("actions", ctx.get_state("actions")+1); }
fn view(ctx) { column([
    cards::SelectableCardGroup(#{ key: "regions", label: "Regions", values: ctx.get_state("values"),
        layout: "grid", columns: 2, select_on_focus: false, on_change: Fn("changed"),
        items: [
            #{ value: "a", label: "Alpha", content: text("Alpha description"), actions: button::Button(#{ key: "inspect", text: "Inspect", on_click: Fn("action") }) },
            #{ value: "b", label: "Beta", disabled: true, content: text("Maintenance") },
            #{ value: "c", label: "Charlie", content: text("Charlie description") },
            #{ value: "d", label: "Delta", content: text("Delta description") },
        ] }),
    text(`${ctx.get_state("values")}:${ctx.get_state("changes")}:${ctx.get_state("actions")}`).test_id("state"),
]).with_style(style().width(px(600))) }
"#;

const SCOPED_PANES: &str = r#"
import "components/selectable_card_group" as cards;
define_component(#{
    metadata: #{id:"test/scoped_choice_pane", "export":"ScopedChoicePane", version:"0.2.2",
        runtime_api:#{min_inclusive:3,max_exclusive:4}, dependencies:["components/selectable_card_group"], capabilities:#{}},
    schema:#{props:#{label:#{schema:#{type:"string"},required:true,sensitive:false},
        nested:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false}}},
        state:#{fields:#{values:#{schema:#{type:"array",items:#{type:"string"}},"default":#{type:"array",value:[]}}}},
        events:#{},slots:#{},parts:[]},render:Fn("render_scoped_choice_pane")
});
fn scoped_choice_changed(ctx,values){ctx.set_state("values",values);}
fn render_scoped_choice_pane(ctx,props){
    if props.nested {return render_component("test/scoped_choice_pane",#{key:"right",label:props.label});}
    cards::SelectableCardGroup(#{key:"choices",label:`${props.label} choices`,values:ctx.get_state("values"),
        items:[#{value:"a",label:`${props.label} choice`,content:text(`${props.label} description`)}],
        on_change:Fn("scoped_choice_changed")}).with_style(style().width(px(260)))
}
fn choice_pane(key,label){render_component("test/scoped_choice_pane",#{key:key,label:label})}
fn state_schema(){#{fields:#{show:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}}}}}
fn toggle_scoped_pages(ctx,payload){ctx.set_state("show",!ctx.get_state("show"));}
fn view(ctx){column([
    text("Toggle").with_key("toggle").test_id("toggle").on_click(Fn("toggle_scoped_pages")),
    SCOPED_CONTENT,
]).with_style(style().width(px(600)))}
"#;

#[gpui::test]
fn cards_same_local_keys_in_sibling_component_scopes_keep_distinct_descriptions(
    cx: &mut TestAppContext,
) {
    // Neither parent nor group keys are restricted to safe ASCII test-ID tokens.
    let source = SCOPED_PANES.replace_once(
        "SCOPED_CONTENT",
        r#"row([choice_pane("left pane","Left"),choice_pane("left%20;pane","Right")])"#,
    );
    let (mut visual, view) = Mount::components(source).mount(cx);
    let snapshot = nodes(&mut visual, &view);
    for name in ["Left", "Right"] {
        let radio = snapshot
            .iter()
            .find(|node| node.role == "radio" && node.name == format!("{name} choice"))
            .unwrap();
        assert_eq!(radio.description, format!("{name} description"));
    }
    let ids = snapshot
        .iter()
        .filter_map(|node| node.semantic_id.as_ref())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        ids.len(),
        2,
        "local choices keys may not alias semantic IDs"
    );
    assert!(ids.iter().all(|id| !id.chars().any(char::is_whitespace)));
    dispatch(&mut visual, &view, by_test_id("toggle"), "click").unwrap();
    visual.run_until_parked();
    let rerendered_ids = nodes(&mut visual, &view)
        .into_iter()
        .filter_map(|node| node.semantic_id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(rerendered_ids, ids, "scoped IDs must be deterministic");
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn cards_distinct_typed_paths_with_equal_display_scopes_do_not_alias(cx: &mut TestAppContext) {
    // These paths stringify identically:
    // ScopedChoicePane[left]/ScopedChoicePane[right]/SelectableCardGroup[choices]
    // One has a delimiter-containing parent key, the other two typed segments.
    let source = SCOPED_PANES.replace_once(
        "SCOPED_CONTENT",
        r#"row([
            choice_pane("left]/ScopedChoicePane[right","Flat"),
            render_component("test/scoped_choice_pane",#{key:"left",label:"Nested",nested:true}),
        ])"#,
    );
    let (mut visual, view) = Mount::components(source).mount(cx);
    let snapshot = nodes(&mut visual, &view);
    for name in ["Flat", "Nested"] {
        let radio = snapshot
            .iter()
            .find(|node| node.role == "radio" && node.name == format!("{name} choice"))
            .unwrap();
        assert_eq!(radio.description, format!("{name} description"));
    }
    let ids = snapshot
        .iter()
        .filter_map(|node| node.semantic_id.as_ref())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        ids.len(),
        2,
        "Display cannot identify a typed component path"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn cards_accept_unicode_and_whitespace_group_keys_without_automatic_test_ids(
    cx: &mut TestAppContext,
) {
    let source = SCOPED_PANES
        .replace_once("key:\"choices\",label:", "key:\"本 地 choices / %\",label:")
        .replace_once("SCOPED_CONTENT", r#"choice_pane("pane","Unicode")"#);
    let (mut visual, view) = Mount::components(source).mount(cx);
    let radio = nodes(&mut visual, &view)
        .into_iter()
        .find(|node| node.role == "radio")
        .unwrap();
    assert_eq!(radio.description, "Unicode description");
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn cards_focus_grid_disabled_home_end_and_independent_actions(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(SOURCE).mount(cx);
    visual.update(|window, cx| view.focus(window, cx).unwrap());
    visual.simulate_keystrokes("tab down");
    visual.run_until_parked();
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(
        text_of(&mut visual, &view, "state"),
        "[\"a\"]:0:0",
        "focus-only must not select"
    );
    visual.simulate_keystrokes("space");
    assert_eq!(
        last_error(&mut visual, &view),
        None,
        "selection callback must resolve in the owning component"
    );
    assert_eq!(text_of(&mut visual, &view, "state"), "[\"c\"]:1:0");
    visual.simulate_keystrokes("end enter home right enter");
    assert_eq!(
        text_of(&mut visual, &view, "state"),
        "[\"c\"]:3:0",
        "right skips disabled Beta"
    );
    let inspect = bounds(&mut visual, &view, "button", "Inspect");
    click_center(&mut visual, inspect);
    visual.simulate_keystrokes("right");
    assert_eq!(
        text_of(&mut visual, &view, "state"),
        "[\"c\"]:3:1",
        "action focus must not use group arrow handlers"
    );
    visual.simulate_keystrokes("enter");
    assert_eq!(text_of(&mut visual, &view, "state"), "[\"c\"]:3:2");
    let alpha = bounds(&mut visual, &view, "radio", "Alpha");
    click_center(&mut visual, alpha);
    let state = nodes(&mut visual, &view);
    let alpha = state
        .iter()
        .find(|node| node.role == "radio" && node.name == "Alpha")
        .unwrap();
    assert_eq!(alpha.checked, Some(UiValue::Bool(true)));
    assert_eq!(alpha.description, "Alpha description");
    let action = state
        .iter()
        .find(|node| node.role == "button" && node.name == "Inspect")
        .unwrap();
    assert_ne!(
        action.parent,
        Some(alpha.id),
        "interactive action must not nest inside radio"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn cards_multiple_selection_and_nonempty_policy(cx: &mut TestAppContext) {
    let source = SOURCE.replace_once(
        "select_on_focus: false",
        "mode: \"multiple\", allow_empty: false, indicator: \"check\"",
    );
    let (mut visual, view) = Mount::components(source).mount(cx);
    visual.update(|window, cx| view.focus(window, cx).unwrap());
    visual.simulate_keystrokes("tab space down space home space");
    visual.run_until_parked();
    assert_eq!(text_of(&mut visual, &view, "state"), "[\"c\"]:2:0");
    visual.simulate_keystrokes("end space");
    assert_eq!(text_of(&mut visual, &view, "state"), "[\"c\", \"d\"]:3:0");
    assert_eq!(
        nodes(&mut visual, &view)
            .iter()
            .filter(|node| node.role == "checkbox")
            .count(),
        4
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn cards_select_on_focus_rtl_and_token_geometry_across_modes_and_densities(
    cx: &mut TestAppContext,
) {
    for theme in [DEFAULT_THEME, DEFAULT_LIGHT_THEME] {
        for density in ["comfortable", "compact"] {
            for locale in ["en", "ar"] {
                let source = SOURCE
                    .replace_once("select_on_focus: false", "select_on_focus: true")
                    .replace_once(
                        "fn view(ctx)",
                        &format!("fn init(ctx) {{ ctx.set_locale(\"{locale}\"); }}\nfn view(ctx)"),
                    )
                    .replace_once(
                        "style().width(px(600)))",
                        &format!("style().width(px(600))).env(#{{ density: \"{density}\" }})"),
                    );
                let (mut visual, view) = Mount::components(source)
                    .theme(theme)
                    .locale("en.rhai", EN_LOCALE)
                    .locale("ar.rhai", AR_LOCALE)
                    .mount(cx);
                let alpha = bounds(&mut visual, &view, "group", "Alpha");
                let beta = bounds(&mut visual, &view, "group", "Beta");
                assert!(alpha.width > 150.0 && alpha.height > 30.0);
                assert!(
                    (alpha.height - beta.height).abs() < 1.0,
                    "grid peers share a height"
                );
                visual.update(|window, cx| view.focus(window, cx).unwrap());
                visual.simulate_keystrokes("tab");
                visual.simulate_keystrokes(if locale == "ar" { "left" } else { "right" });
                assert_eq!(
                    text_of(&mut visual, &view, "state"),
                    "[\"c\"]:1:0",
                    "logical next skips disabled Beta"
                );
                visual.simulate_keystrokes("end");
                assert_eq!(text_of(&mut visual, &view, "state"), "[\"d\"]:2:0");
                assert_eq!(last_error(&mut visual, &view), None);
            }
        }
    }
}

#[gpui::test]
fn cards_empty_all_disabled_and_unicode_values_are_valid(cx: &mut TestAppContext) {
    for items in [
        "[]",
        "[#{value: \"日本/SG\", label: \"Disabled\", disabled: true, content: text(\"CJK\")}]",
    ] {
        let source = format!(
            r#"
            import "components/selectable_card_group" as cards;
            fn changed(ctx, payload) {{}}
            fn view(ctx) {{ cards::SelectableCardGroup(#{{key: "empty", label: "Choices", values: [],
                items: {items}, on_change: Fn("changed") }}) }}
        "#
        );
        let (mut visual, view) = Mount::components(source).mount(cx);
        visual.simulate_keystrokes("tab right down home end enter space");
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn cards_vertical_grid_skips_disabled_and_missing_cells_in_the_same_column(
    cx: &mut TestAppContext,
) {
    let source = SOURCE
        .replace_once("select_on_focus: false", "select_on_focus: true")
        .replace_once("value: \"c\", label: \"Charlie\"", "value: \"c\", label: \"Charlie\", disabled: true")
        .replace_once("#{ value: \"d\", label: \"Delta\", content: text(\"Delta description\") },",
            "#{ value: \"d\", label: \"Delta\", content: text(\"Delta description\") }, #{value: \"e\", label: \"Echo\", content: text(\"Partial row\")},");
    let (mut visual, view) = Mount::components(source).mount(cx);
    visual.update(|window, cx| view.focus(window, cx).unwrap());
    visual.simulate_keystrokes("tab down");
    visual.run_until_parked();
    assert_eq!(text_of(&mut visual, &view, "state"), "[\"e\"]:1:0");
    visual.simulate_keystrokes("down");
    assert_eq!(text_of(&mut visual, &view, "state"), "[\"a\"]:2:0");
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn cards_row_and_column_follow_their_keyboard_axis(cx: &mut TestAppContext) {
    for layout in ["row", "column"] {
        let source = SOURCE
            .replace_once("layout: \"grid\"", &format!("layout: \"{layout}\""))
            .replace_once("select_on_focus: false", "select_on_focus: true");
        let (mut visual, view) = Mount::components(source).mount(cx);
        visual.update(|window, cx| view.focus(window, cx).unwrap());
        visual.simulate_keystrokes("tab");
        visual.simulate_keystrokes(if layout == "row" { "right" } else { "down" });
        assert_eq!(text_of(&mut visual, &view, "state"), "[\"c\"]:1:0");
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn cards_reorder_preserves_the_focused_value_identity(cx: &mut TestAppContext) {
    let source = r#"
        import "components/selectable_card_group" as cards;
        fn state_schema() { #{ fields: #{
            values: #{ schema: #{type:"array",items:#{type:"string"}}, "default":#{type:"array",value:[]} },
            reverse: #{ schema: #{type:"bool"}, "default":#{type:"bool",value:false} },
        } } }
        fn changed(ctx, values) { ctx.set_state("values", values); }
        fn flip(ctx, payload) { ctx.set_state("reverse", !ctx.get_state("reverse")); }
        fn view(ctx) {
            let items = [#{value:"a",label:"Alpha",content:text("A")},
                #{value:"b",label:"Beta",content:text("B")}, #{value:"c",label:"Charlie",content:text("C")}];
            if ctx.get_state("reverse") { items.reverse(); }
            column([
                cards::SelectableCardGroup(#{key:"order",label:"Choices",layout:"column",mode:"multiple",allow_empty:true,
                    values:ctx.get_state("values"),items:items,on_change:Fn("changed")}),
                text(`${ctx.get_state("values")}`).test_id("values"),
            ]).on_key_value("r",Fn("flip"),()).tab_stop(false).with_style(style().width(px(320)))
        }
    "#;
    let (mut visual, view) = Mount::components(source).mount(cx);
    visual.update(|window, cx| view.focus(window, cx).unwrap());
    visual.simulate_keystrokes("tab down down r space");
    visual.run_until_parked();
    assert_eq!(text_of(&mut visual, &view, "values"), "[\"c\"]");
    assert_eq!(last_error(&mut visual, &view), None);
}
