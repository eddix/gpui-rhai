//! A Resizable grip covers its native handle's hitbox, so the handle's resize cursor does
//! not reach the pointer over it; an edge grip carries the resize cursor of its direction
//! itself, as SplitPane's grip does (#143). The Style API has no diagonal cursor, so a corner
//! grip carries none.
//!
//! cargo test --manifest-path tests/native-keyboard/Cargo.toml --test native manipulation::resizable_grip_cursor

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME, TOKEN_BASE_SOURCE,
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

const HANDLES: [&str; 8] = ["n", "s", "e", "w", "ne", "nw", "se", "sw"];

/// A Resizable with a grip on every handle.
fn mount(cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let grips = HANDLES
        .iter()
        .map(|handle| format!("{handle}:box([]).with_style(style().width(px(12)).height(px(12)))"))
        .collect::<Vec<_>>()
        .join(",");
    let main = format!(
        r#"import "components/resizable" as resizable;
fn view(ctx) {{ column([
    resizable::Resizable(#{{ key: "card", label: "Card", content: text("Card"),
        rect: #{{ x: 100.0, y: 80.0, width: 200.0, height: 120.0 }}, grips: #{{ {grips} }} }})
        .with_style(style().width(px(500)).height(px(400)))
]) }}"#
    );
    let entry = ModuleId::parse("main").unwrap();
    let mut modules = BTreeMap::from([(entry.clone(), main)]);
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(TOKEN_BASE_SOURCE)
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
        let host = ScriptViewHost::new("resizable", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("resizable"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| window.simulate_next_frame(cx));
    visual.run_until_parked();
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    (visual, view)
}

fn find_keyed<'a>(node: &'a UiNode, key: &str) -> Option<&'a UiNode> {
    if node.key().is_some_and(|node_key| node_key.as_str() == key) {
        return Some(node);
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(|child| find_keyed(child, key))
        }
        _ => None,
    }
}

#[gpui::test]
fn each_grip_carries_the_cursor_of_its_handle(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx);
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let cursors = HANDLES
        .iter()
        .map(|handle| {
            let grip = find_keyed(&root, &format!("card-grip-{handle}"))
                .unwrap_or_else(|| panic!("no grip for {handle}"));
            (*handle, grip.style().base.cursor)
        })
        .collect::<Vec<_>>();
    println!("{cursors:?}");
    assert_eq!(
        cursors,
        [
            ("n", Some(CursorKind::ResizeVertical)),
            ("s", Some(CursorKind::ResizeVertical)),
            ("e", Some(CursorKind::ResizeHorizontal)),
            ("w", Some(CursorKind::ResizeHorizontal)),
            ("ne", None),
            ("nw", None),
            ("se", None),
            ("sw", None),
        ]
    );
}
