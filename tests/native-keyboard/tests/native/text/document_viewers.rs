//! CodeViewer and DiffViewer mount native document surfaces, and a text document revision
//! invalidates exactly the viewer that reads it.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{SingleEmbeddedHost, palette_texts, wait_for_view_text};
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{
    EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptViewConfig, ScriptViewHost,
    UiRuntimeState,
};

#[gpui::test]
fn code_and_diff_viewers_mount_native_document_surfaces(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([
            (
                entry,
                r#"
                    import "components/code_viewer" as code_viewer;
                    import "components/diff_viewer" as diff_viewer;
                    fn activated(ctx, payload) { () }
                    fn view(ctx) {
                        row([
                            code_viewer::CodeViewer(#{ key: "rhai", label: "Rhai source",
                                language: "rhai", source: "fn view() {\n    text(\"Hello\")\n}\n",
                                on_location_activate: Fn("activated") }),
                            diff_viewer::DiffViewer(#{ key: "servers", mode: "split",
                                left: #{ source: "port = 80\ncity = \"东京\"\n", label: "Server A", language: "rhai" },
                                right: #{ source: "port = 443\ncity = \"上海\"\n", label: "Server B", language: "rhai" },
                                on_location_activate: Fn("activated") })
                        ]).with_style(style().width(relative(1)).height(relative(1)))
                    }
                "#
                .to_owned(),
            ),
            (
                ModuleId::parse("components/code_viewer").unwrap(),
                include_str!("../../../../../registry/components/code_viewer.rhai").to_owned(),
            ),
            (
                ModuleId::parse("components/diff_viewer").unwrap(),
                include_str!("../../../../../registry/components/diff_viewer.rhai").to_owned(),
            ),
        ])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("document-viewer-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("document-viewer-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some(view.clone());
        SingleEmbeddedHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let view = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    visual.update(|_, cx| {
        assert_eq!(view.last_error(cx).unwrap(), None);
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        for label in ["Rhai source", "Server A compared with Server B"] {
            let node = snapshot
                .find_by_role_and_name("document", label)
                .next()
                .unwrap_or_else(|| panic!("missing native document {label}"));
            let bounds = node.geometry.unwrap().visual;
            assert!(
                bounds.width > 100.0 && bounds.height > 100.0,
                "{label}: {bounds:?}"
            );
        }
    });
    let diff_bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("document", "Server A compared with Server B")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let diff_line = point(
        px((diff_bounds.x + 72.0) as f32),
        px((diff_bounds.y + 42.0) as f32),
    );
    visual.simulate_mouse_down(diff_line, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(diff_line, MouseButton::Left, Modifiers::default());
    visual.simulate_keystrokes("alt-down");
    visual.simulate_keystrokes("alt-cmd-c");
    visual.run_until_parked();
    let patch = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert!(
        patch.as_deref().is_some_and(|patch| {
            patch.contains("--- Server A")
                && patch.contains("+++ Server B")
                && patch.contains("-port = 80")
                && patch.contains("+port = 443")
        }),
        "explicit left-to-right patch copy failed: {patch:?}"
    );
}

#[gpui::test]
fn native_text_document_revision_invalidates_exact_viewer_reader(cx: &mut TestAppContext) {
    #[derive(Clone)]
    struct Documents;

    impl gpui_rhai::ScriptViewExtension for Documents {
        fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
            runtime
                .native_documents
                .register(
                    "source",
                    gpui_rhai::NativeTextDocument::new(
                        "source",
                        1,
                        "let port = 80;\nlet enabled = true;\n",
                    )
                    .unwrap(),
                )
                .map_err(|error| error.to_string())
        }
    }

    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([
            (
                entry,
                r#"
                    import "components/code_viewer" as code_viewer;
                    fn view(ctx) {
                        let document = ctx.get_native_text_document("source");
                        column([
                            text(`revision:${document.revision}`),
                            code_viewer::CodeViewer(#{ key: "source", label: "Source",
                                source: document, language: "rhai" })
                        ]).with_style(style().width(relative(1)).height(relative(1)))
                    }
                "#
                .to_owned(),
            ),
            (
                ModuleId::parse("components/code_viewer").unwrap(),
                include_str!("../../../../../registry/components/code_viewer.rhai").to_owned(),
            ),
        ])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .extension(Documents)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("native-document-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("native-document-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some(view.clone());
        SingleEmbeddedHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let view = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    assert!(palette_texts(&mut visual, &view).contains(&"revision:1".to_owned()));
    let document_bounds = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("document", "Source")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    let start = point(
        px((document_bounds.x + 48.0) as f32),
        px((document_bounds.y + 8.0) as f32),
    );
    let end = point(
        px((document_bounds.x + 90.0) as f32),
        px((document_bounds.y + 24.0) as f32),
    );
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.simulate_keystrokes("cmd-c");
    visual.run_until_parked();
    let copied = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert!(
        copied.as_deref().is_some_and(|text| text.contains('\n')),
        "cross-line source selection did not copy original text: {copied:?}"
    );
    visual.simulate_keystrokes("cmd-f");
    visual.run_until_parked();
    visual.simulate_input("enabled");
    visual.run_until_parked();
    visual.simulate_keystrokes("cmd-g");
    visual.simulate_keystrokes("escape");
    visual.run_until_parked();
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    let invalidated = visual
        .update(|_, cx| {
            view.replace_native_text_document(
                "source",
                gpui_rhai::NativeTextDocument::new("source", 2, "let port = 443;\n").unwrap(),
                cx,
            )
        })
        .unwrap();
    assert!(invalidated, "native document reader was not tracked");
    for _ in 0..4 {
        cx.background_executor
            .advance_clock(std::time::Duration::from_millis(16));
        visual.run_until_parked();
    }
    wait_for_view_text(
        &mut visual,
        &view,
        "revision:2",
        "native document replacement should invalidate its exact reader",
    );
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
}
