//! A caller-written shortcut shows the platform legend, the same as a bound
//! action's: `cmd-p` is `⌘P` on macOS. Text that is already a legend (`⌘K`)
//! stays as written. Button draws it in the label voice that Command and Menu
//! use for their inline Kbd.

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

fn texts(cx: &mut TestAppContext, body: &str) -> Vec<String> {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(
        entry.clone(),
        format!(
            r#"import "components/button" as button;
import "components/command" as command;
fn noop(ctx, payload) {{ () }}
fn view(ctx) {{ column([{body}]).with_style(style().width(px(600))) }}"#
        ),
    );
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(TOKEN_BASE_SOURCE)
            .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
                (
                    path.strip_suffix(".svg").unwrap_or(path).to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".to_owned(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }))
            .prepare()
            .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("legend", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("legend"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, cx| window.simulate_next_frame(cx));
    visual.run_until_parked();
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    tree.nodes()
        .filter(|node| node.role == "text")
        .map(|node| node.name.clone())
        .collect()
}

#[gpui::test]
fn caller_written_shortcuts_show_the_platform_legend(cx: &mut TestAppContext) {
    let found = texts(
        cx,
        r#"button::Button(#{ text: "Menu", shortcut: "cmd-p", on_click: Fn("noop") }),
        button::Button(#{ text: "Find", shortcut: "⌘F", on_click: Fn("noop") }),
        command::Command(#{ key: "palette", label: "Commands", query: "", active_value: "",
            items: [#{ value: "deploy", label: "Deploy", shortcut: "cmd-shift-d" }] })"#,
    );
    println!("{found:?}");
    for raw in ["cmd-p", "cmd-shift-d"] {
        assert!(
            !found.iter().any(|text| text == raw),
            "raw `{raw}` shown: {found:?}"
        );
    }
    assert!(
        found.iter().any(|text| text == "⌘F"),
        "a legend stays: {found:?}"
    );
    if cfg!(target_os = "macos") {
        assert!(found.iter().any(|text| text == "⌘P"), "{found:?}");
        assert!(found.iter().any(|text| text == "⇧⌘D"), "{found:?}");
    }
}
