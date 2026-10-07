//! Table keyboard contract on both data sources: Up/Down move one row and stop
//! at the ends, Home/End go to the first and last row, Enter activates the
//! current row; filtering, paging, sorting and collapsed groups decide which
//! rows those are. A NativeCollection finds its targets in native code.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
use gpui_rhai::*;

const ROWS: [(&str, &str, &str); 4] = [
    ("r0", "Alpha", "A"),
    ("r1", "Bravo", "A"),
    ("r2", "Charlie", "B"),
    ("r3", "Delta", "B"),
];

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

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
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::from([(ModuleId::parse("main").unwrap(), source)]);
    for (id, source) in gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let prepared = EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(modules),
        gpui_rhai_registry::DEFAULT_THEME,
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .asset_sources(
        gpui_rhai_registry::BUNDLED_ASSET_SOURCES
            .iter()
            .map(|(path, source)| {
                (
                    path.trim_end_matches(".svg").to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".into(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }),
    )
    .extension(Rows)
    .prepare()
    .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let name = name.to_owned();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(&name, cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new(&name), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    (window, view)
}

fn state(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> (String, String) {
    let text = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.test_id.as_deref() == Some("state"))
            .unwrap()
            .name
            .clone()
    });
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
                view.last_error_with(&mut visual),
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

trait LastError {
    fn last_error_with(&self, visual: &mut VisualTestContext) -> Option<String>;
}

impl LastError for ScriptViewHandle {
    fn last_error_with(&self, visual: &mut VisualTestContext) -> Option<String> {
        visual.update(|_, cx| self.last_error(cx).unwrap())
    }
}
