//! Table keyboard contract on both data sources: Up/Down move one row and stop
//! at the ends, Home/End go to the first and last row, Enter activates the
//! current row; filtering, paging, sorting and collapsed groups decide which
//! rows those are. A NativeCollection finds its targets in native code.

use std::collections::BTreeMap;
use std::time::Duration;

use gpui::{TestAppContext, VisualTestContext, WindowHandle};
use gpui_rhai::*;

use crate::support::{Host, Mount, last_error, text_of};

const ROWS: [(&str, &str, &str); 4] = [
    ("r0", "Alpha", "A"),
    ("r1", "Bravo", "A"),
    ("r2", "Charlie", "B"),
    ("r3", "Delta", "B"),
];

#[derive(Clone)]
struct Rows;

impl ScriptViewExtension for Rows {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        let rows = ROWS.iter().map(|(id, label, group)| {
            BTreeMap::from([
                ("id".into(), UiValue::String((*id).into())),
                ("label".into(), UiValue::String((*label).into())),
                ("group".into(), UiValue::String((*group).into())),
            ])
        });
        runtime
            .native_collections
            .register("rows", NativeCollection::new("id", rows).unwrap())
            .map_err(|error| error.to_string())
    }
}

struct Case {
    name: &'static str,
    /// Extra Table props.
    props: &'static str,
    /// Array rows in display order (an array Table does not sort them itself).
    array_order: [usize; 4],
    selected: &'static str,
    /// Keys pressed in turn, each with the selection it must leave.
    steps: &'static [(&'static str, &'static str)],
    /// The row Enter activates afterwards, if any.
    activates: Option<&'static str>,
}

const CASES: &[Case] = &[
    Case {
        name: "plain",
        props: "",
        array_order: [0, 1, 2, 3],
        selected: "r0",
        steps: &[
            ("down", "r1"),
            ("down", "r2"),
            ("end", "r3"),
            ("down", "r3"),
            ("home", "r0"),
            ("up", "r0"),
        ],
        activates: Some("r0"),
    },
    Case {
        name: "filtered",
        props: r#"query: "e", search_fields: ["label"],"#,
        array_order: [0, 1, 2, 3],
        selected: "",
        steps: &[("down", "r2"), ("end", "r3"), ("home", "r2")],
        activates: Some("r2"),
    },
    Case {
        name: "paged",
        props: "page: 2, page_size: 2,",
        array_order: [0, 1, 2, 3],
        selected: "r2",
        steps: &[("down", "r3"), ("home", "r2"), ("up", "r2")],
        activates: None,
    },
    Case {
        name: "sorted",
        props: r#"sort: #{ key: "label", direction: "descending" },"#,
        array_order: [3, 2, 1, 0],
        selected: "r3",
        steps: &[("down", "r2"), ("end", "r0")],
        activates: Some("r0"),
    },
    Case {
        name: "collapsed",
        props: r#"group_by: "group", collapsed_groups: ["A"],"#,
        array_order: [0, 1, 2, 3],
        selected: "",
        steps: &[("down", "r2"), ("end", "r3"), ("up", "r2")],
        activates: None,
    },
];

fn script(case: &Case, native: bool) -> String {
    let rows = if native {
        "ctx.get_native_collection(\"rows\")".to_owned()
    } else {
        let items = case
            .array_order
            .iter()
            .map(|index| {
                let (id, label, group) = ROWS[*index];
                format!(r#"#{{id:"{id}",label:"{label}",group:"{group}"}}"#)
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("[{items}]")
    };
    format!(
        r#"import "components/table" as table;
fn state_schema(){{#{{fields:#{{
    selected:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"{selected}"}}}},
    clicked:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn changed(ctx,keys){{ctx.set_state("selected",keys[0]);}}
fn clicked(ctx,key){{ctx.set_state("clicked",key);}}
fn view(ctx){{column([
    table::Table(#{{key:"rows",label:"Rows",row_key:"id",height:200,rows:{rows},{props}
        columns:[#{{key:"label",title:"Label",sortable:true,width:#{{kind:"fixed",value:160}}}},
            #{{key:"group",title:"Group",width:#{{kind:"fixed",value:80}}}}],
        selection_mode:"single",selected_keys:[ctx.get_state("selected")],
        on_selection_change:Fn("changed"),on_row_click:Fn("clicked")}}),
    text(`${{ctx.get_state("selected")}}|${{ctx.get_state("clicked")}}`).test_id("state"),
]).with_style(style().width(px(400)))}}"#,
        selected = case.selected,
        props = case.props,
    )
}

fn mount(
    cx: &mut TestAppContext,
    source: String,
    name: &str,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    Mount::components(source)
        .with(|view| view.extension(Rows))
        .id(name)
        .mount_window(cx)
}

fn state(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> (String, String) {
    let text = text_of(visual, view, "state");
    let (selected, clicked) = text.split_once('|').unwrap();
    (selected.to_owned(), clicked.to_owned())
}

fn press(visual: &mut VisualTestContext, key: &str) {
    visual.simulate_keystrokes(key);
    visual.run_until_parked();
    visual.executor().advance_clock(Duration::from_millis(32));
    visual.run_until_parked();
}

#[gpui::test]
fn both_data_sources_follow_the_same_keyboard_contract(cx: &mut TestAppContext) {
    let mut report = Vec::new();
    for case in CASES {
        for (source, native) in [("array", false), ("native", true)] {
            let (window, view) =
                mount(cx, script(case, native), &format!("{}-{source}", case.name));
            let mut visual = VisualTestContext::from_window(*window, cx);
            visual.update(|window, cx| {
                window.simulate_next_frame(cx);
                window.focus_next(cx);
            });
            visual.run_until_parked();
            assert_eq!(
                last_error(&mut visual, &view),
                None,
                "{} {source}",
                case.name
            );
            for (key, expected) in case.steps {
                press(&mut visual, key);
                let (selected, _) = state(&mut visual, &view);
                let ok = selected == *expected;
                report.push(format!(
                    "{} {source} {key}: {selected} (expected {expected}){}",
                    case.name,
                    if ok { "" } else { "  <- MISMATCH" }
                ));
            }
            if let Some(expected) = case.activates {
                press(&mut visual, "enter");
                let (_, clicked) = state(&mut visual, &view);
                report.push(format!(
                    "{} {source} enter: {clicked} (expected {expected}){}",
                    case.name,
                    if clicked == expected {
                        ""
                    } else {
                        "  <- MISMATCH"
                    }
                ));
            }
        }
    }
    let text = report.join("\n");
    println!("{text}");
    assert!(!text.contains("MISMATCH"), "{text}");
}
