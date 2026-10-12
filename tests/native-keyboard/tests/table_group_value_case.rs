//! A Table group header shows the group value as the rows carry it. The value
//! comes from a data column (`group_by`) and is often a case-sensitive
//! identifier (a cluster, a pod, a path), so the header must not change it.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window};
use gpui_rhai::*;

const GROUPS: [&str; 2] = [
    "default · 583444 · Singapore-Central · TikTok/default",
    "canary · 583445 · US-East · TikTok/canary",
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

fn mount(cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let source = format!(
        r#"import "components/table" as table;
fn view(ctx) {{
    column([table::Table(#{{ key: "instances", label: "Instances", row_key: "id", height: 240,
        group_by: "cluster",
        rows: [#{{ id: "i0", pod: "dp-09aa6aec4c-kbzgl", cluster: "{a}" }},
               #{{ id: "i1", pod: "dp-09aa6aec4c-0lO1I", cluster: "{b}" }}],
        columns: [#{{ key: "pod", title: "Pod", width: #{{ kind: "fixed", value: 240 }} }},
                  #{{ key: "cluster", title: "Cluster", width: #{{ kind: "fixed", value: 360 }} }}] }})])
        .with_style(style().width(px(640)))
}}"#,
        a = GROUPS[0],
        b = GROUPS[1],
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
    .prepare()
    .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("groups", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("groups"), host.clone(), window, cx)
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
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
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

#[gpui::test]
fn a_group_header_shows_the_group_value_unchanged(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx);
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    // The visible group label text (key `<header key>-label`).
    let shown = nodes
        .iter()
        .filter(|node| {
            node.key()
                .is_some_and(|key| key.as_str().ends_with("-label"))
        })
        .filter_map(|node| match node.kind() {
            UiNodeKind::Text { text } => Some(text.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    // What assistive technology reads for the same header row.
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let spoken = tree
        .nodes()
        .filter(|node| node.role == "rowheader")
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();
    println!("shown {shown:?}");
    println!("spoken {spoken:?}");
    assert_eq!(
        shown, GROUPS,
        "the group header shows the value as the rows carry it"
    );
}
