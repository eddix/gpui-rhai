//! Identifiers in the code role (#95): a Table column's `typography` reaches
//! only that column's cells, for array rows and a NativeCollection alike; group
//! headers use the label voice (#118); a span takes a role's face and a
//! background without leaving its line.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, rgba};
use gpui_rhai::*;

const ROWS: &[(&str, &str, &str)] = &[
    ("r0", "dp-09aa6aec4c-kbzgl", "A"),
    ("r1", "dp-09aa6aec4c-0lO1I", "B"),
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
        let rows = ROWS.iter().map(|(id, pod, group)| {
            BTreeMap::from([
                ("id".into(), UiValue::String((*id).into())),
                ("pod".into(), UiValue::String((*pod).into())),
                ("group".into(), UiValue::String((*group).into())),
            ])
        });
        runtime
            .native_collections
            .register("rows", NativeCollection::new("id", rows).unwrap())
            .map_err(|error| error.to_string())
    }
}

fn mount(cx: &mut TestAppContext, view_body: &str) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let source = format!(
        r#"import "components/table" as table;
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(480))) }}"#
    );
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
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("code", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("code"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    for _ in 0..2 {
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
    (visual, view)
}

/// Every node of the rendered tree, realized virtual rows included.
fn walk<'a>(node: &'a UiNode, out: &mut Vec<&'a UiNode>) {
    out.push(node);
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for child in children {
                walk(child, out);
            }
        }
        UiNodeKind::VirtualCollection { spec } => {
            for child in spec.realized.values() {
                walk(child, out);
            }
        }
        _ => {}
    }
}

/// The typography role of every node keyed `key`.
fn roles(visual: &mut VisualTestContext, view: &ScriptViewHandle, key: &str) -> Vec<String> {
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    nodes
        .into_iter()
        .filter(|node| node.key().is_some_and(|node_key| node_key.as_str() == key))
        .map(|node| node.style().base.typography.clone().unwrap_or_default())
        .collect()
}

#[gpui::test]
fn a_code_column_reaches_only_its_cells(cx: &mut TestAppContext) {
    for rows in [
        r#"[#{ id: "r0", pod: "dp-09aa6aec4c-kbzgl", group: "A" },
            #{ id: "r1", pod: "dp-09aa6aec4c-0lO1I", group: "B" }]"#,
        r#"ctx.get_native_collection("rows")"#,
    ] {
        let (mut visual, view) = mount(
            cx,
            &format!(
                r#"table::Table(#{{ key: "pods", label: "Pods", row_key: "id", height: 200, rows: {rows},
                    columns: [#{{ key: "pod", title: "Pod", typography: "code", width: #{{ kind: "fixed", value: 240 }} }},
                        #{{ key: "group", title: "Group", width: #{{ kind: "fixed", value: 80 }} }}] }})"#
            ),
        );
        let pod = roles(&mut visual, &view, "pod");
        let group = roles(&mut visual, &view, "group");
        println!("{rows:.10}: pod {pod:?}, group {group:?}");
        // The header cell carries the key too; it keeps the label voice.
        assert!(
            pod.iter().filter(|role| *role == "code").count() == 2,
            "{pod:?}"
        );
        assert!(group.iter().all(|role| role != "code"), "{group:?}");
        assert!(
            group.iter().filter(|role| *role == "body").count() == 2,
            "{group:?}"
        );
    }
}

#[gpui::test]
fn a_group_header_uses_the_label_voice(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"table::Table(#{ key: "pods", label: "Pods", row_key: "id", height: 200,
            rows: ctx.get_native_collection("rows"), group_by: "group",
            columns: [#{ key: "pod", title: "Pod", width: #{ kind: "fixed", value: 240 } },
                #{ key: "group", title: "Group", width: #{ kind: "fixed", value: 80 } }] })"#,
    );
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    let labels = nodes
        .iter()
        .filter(|node| {
            node.key()
                .is_some_and(|key| key.as_str().ends_with("-label"))
        })
        .map(|node| {
            let text = match node.kind() {
                UiNodeKind::Text { text } => text.to_string(),
                _ => String::new(),
            };
            (
                text,
                node.style().base.typography.clone().unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    println!("{labels:?}");
    assert_eq!(
        labels,
        [("A".into(), "label".into()), ("B".into(), "label".into())]
    );
}

#[gpui::test]
fn a_span_takes_a_face_and_a_background(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"text([span("Run "), span("cargo run").typography("code").background(rgba(0x12345678)),
            span(" now.")]).with_style(style().typography("body"))"#,
    );
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    let spans = nodes
        .iter()
        .find_map(|node| match node.kind() {
            UiNodeKind::RichText { spans, .. } => Some(spans.clone()),
            _ => None,
        })
        .expect("rich text");
    assert_eq!(spans[1].typography_role(), Some("code"));
    assert!(spans[1].background_value().is_some());
    assert!(spans[0].typography_role().is_none() && spans[0].background_value().is_none());
    // The background is painted behind the span's glyphs.
    let color: gpui::Hsla = rgba(0x1234_5678).into();
    let painted = visual.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .any(|quad| quad.background == color.into())
    });
    assert!(painted, "the span background is painted");
}
