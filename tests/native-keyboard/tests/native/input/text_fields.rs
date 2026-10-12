//! Native text fields: the TextInput primitive, Unicode editing and the clipboard, TextArea
//! wrapping and limits, read-only selection, autofocus, and focus surviving a theme swap
//! while typing.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{KeyboardHost, Mount, click_palette_trigger, node_texts, palette_texts};
use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{
    ComponentInstancePath, EmbeddedScriptSource, EventPropagation, HostCallback, ModuleId,
    NodeEventDispatcher, PrimitiveNode, PrimitiveProps, PrimitiveRegistry, PrimitiveValue,
    RestrictedModuleResolver, RuntimeEngine, ScriptLifecycle, TextInputPrimitiveHandler, UiNode,
    UiRuntimeState, UiValue, init_text_area, init_text_input, text_input_primitive_descriptor,
};

#[gpui::test]
fn text_input_primitive_dispatches_host_callbacks(cx: &mut TestAppContext) {
    cx.update(init_text_input);
    let registry = PrimitiveRegistry::new();
    let descriptor = text_input_primitive_descriptor();
    let primitive = descriptor.id.clone();
    registry
        .register(descriptor, TextInputPrimitiveHandler::default())
        .unwrap();
    let received = Rc::new(RefCell::new(Vec::new()));
    let changed = Rc::clone(&received);
    let submitted = Rc::clone(&received);
    let props = PrimitiveProps::new()
        .with(
            "value",
            PrimitiveValue::Data(UiValue::String(String::new())),
        )
        .with(
            "placeholder",
            PrimitiveValue::Data(UiValue::String("Host input".to_owned())),
        )
        .with(
            "on_change",
            PrimitiveValue::Callback(
                HostCallback::new("widget.input-change", move |payload, _, _| {
                    changed.borrow_mut().push(("change".to_owned(), payload));
                    EventPropagation::Handled
                })
                .into(),
            ),
        )
        .with(
            "on_submit",
            PrimitiveValue::Callback(
                HostCallback::new("widget.input-submit", move |payload, _, _| {
                    submitted.borrow_mut().push(("submit".to_owned(), payload));
                    EventPropagation::Handled
                })
                .into(),
            ),
        );
    let root = UiNode::custom(PrimitiveNode {
        primitive,
        key: Some("host-input".to_owned()),
        props,
    });
    let dispatcher = NodeEventDispatcher::new(|_, _, _, _, _| EventPropagation::Handled);
    let window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        host_focus.focus(window, cx);
        KeyboardHost {
            root: Rc::new(RefCell::new(root)),
            tree: gpui_rhai::RetainedUiTree::new(),
            dispatcher,
            host_focus,
            primitives: registry,
        }
    });
    cx.run_until_parked();
    cx.simulate_keystrokes(*window, "tab");
    cx.simulate_input(*window, "h");
    cx.simulate_keystrokes(*window, "enter");
    // Host callbacks do not take ownership of controlled values. Without a
    // replacement root, submit still observes the Host's original empty prop.
    assert_eq!(
        *received.borrow(),
        vec![
            ("change".to_owned(), UiValue::String("h".to_owned())),
            ("submit".to_owned(), UiValue::String(String::new())),
        ]
    );
}

#[gpui::test]
fn native_input_updates_rhai_state_and_clipboard_with_unicode(cx: &mut TestAppContext) {
    cx.update(init_text_input);
    let input = gpui_rhai_registry::INPUT_SOURCE;
    let source = EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
        ModuleId::parse("components/input").unwrap(),
        input.to_owned(),
    )]));
    let mut runtime_engine = RuntimeEngine::new();
    runtime_engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = runtime_engine
        .compile_self_contained_named(
            "ui/input_lifecycle.rhai",
            r#"
                import "components/input" as input;
                fn state_schema() { #{ fields: #{
                    value: #{ schema: #{ type: "string" },
                        "default": #{ type: "string", value: "" } }
                } } }
                fn changed(ctx, value) { ctx.set_state("value", value); }
                fn view(ctx) {
                    input::Input(#{
                        key: "name", label: "Name", value: ctx.get_state("value"),
                        placeholder: "Name", on_change: Fn("changed")
                    })
                }
            "#,
        )
        .unwrap();
    let primitives = runtime_engine.primitive_registry();
    let schema = runtime_engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root_path = ComponentInstancePath::root("App", "root");
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        root_path.clone(),
        Some("main".to_owned()),
        std::collections::BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut runtime_engine).unwrap();
    let root = Rc::new(RefCell::new(lifecycle.root().unwrap().clone()));
    let lifecycle = Rc::new(RefCell::new(lifecycle));
    let runtime_engine = Rc::new(RefCell::new(runtime_engine));
    let captured_root = Rc::clone(&root);
    let captured_lifecycle = Rc::clone(&lifecycle);
    let captured_engine = Rc::clone(&runtime_engine);
    let errors = Rc::new(RefCell::new(Vec::new()));
    let captured_errors = Rc::clone(&errors);
    let dispatcher = NodeEventDispatcher::new(move |callback, payload, _, _, app| {
        let result = {
            let engine = captured_engine.borrow();
            captured_lifecycle
                .borrow()
                .invoke_callback_transactional(&engine, &callback, payload)
        };
        if let Err(error) = result {
            captured_errors.borrow_mut().push(error.to_string());
            return EventPropagation::Handled;
        }
        if let Err(error) = captured_lifecycle
            .borrow_mut()
            .render(&mut captured_engine.borrow_mut())
            .map(|root| {
                *captured_root.borrow_mut() = root.clone();
                app.refresh_windows();
            })
        {
            captured_errors.borrow_mut().push(error.to_string());
        }
        EventPropagation::Handled
    });
    let window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        host_focus.focus(window, cx);
        KeyboardHost {
            root,
            tree: gpui_rhai::RetainedUiTree::new(),
            dispatcher,
            host_focus,
            primitives,
        }
    });
    cx.run_until_parked();

    cx.simulate_keystrokes(*window, "tab");
    cx.simulate_input(*window, "中文😀é");
    assert!(errors.borrow().is_empty(), "{:?}", errors.borrow());
    assert_eq!(
        runtime.borrow().component_state.get(&root_path, "value"),
        Some(&UiValue::String("中文😀é".to_owned()))
    );

    cx.simulate_keystrokes(*window, "cmd-a cmd-c");
    assert_eq!(
        cx.read_from_clipboard().and_then(|item| item.text()),
        Some("中文😀é".to_owned())
    );
    cx.simulate_keystrokes(*window, "cmd-x");
    assert_eq!(
        runtime.borrow().component_state.get(&root_path, "value"),
        Some(&UiValue::String(String::new()))
    );
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("粘贴".to_owned()));
    cx.simulate_keystrokes(*window, "cmd-a cmd-v");
    assert_eq!(
        runtime.borrow().component_state.get(&root_path, "value"),
        Some(&UiValue::String("粘贴".to_owned()))
    );
}

#[gpui::test]
fn native_textarea_wraps_inserts_newlines_and_limits_graphemes(cx: &mut TestAppContext) {
    cx.update(init_text_area);
    let textarea = gpui_rhai_registry::TEXTAREA_SOURCE;
    let source = EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
        ModuleId::parse("components/textarea").unwrap(),
        textarea.to_owned(),
    )]));
    let mut runtime_engine = RuntimeEngine::new();
    runtime_engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = runtime_engine
        .compile_self_contained_named(
            "ui/textarea_lifecycle.rhai",
            r#"
                import "components/textarea" as textarea;
                fn state_schema() { #{ fields: #{
                    value: #{ schema: #{ type: "string" },
                        "default": #{ type: "string", value: "" } },
                    read_only_value: #{ schema: #{ type: "string" },
                        "default": #{ type: "string", value: "Read only 中文" } },
                } } }
                fn changed(ctx, value) { ctx.set_state("value", value); }
                fn changed_read_only(ctx, value) { ctx.set_state("read_only_value", value); }
                fn view(ctx) {
                    column([
                        textarea::Textarea(#{
                            key: "notes", label: "Notes", value: ctx.get_state("value"),
                            placeholder: "Notes", min_rows: 2, max_rows: 4,
                            max_length: 5, autofocus: true, on_change: Fn("changed")
                        }),
                        textarea::Textarea(#{
                            key: "reference", label: "Reference", value: ctx.get_state("read_only_value"),
                            read_only: true, rows: 2,
                            on_change: Fn("changed_read_only")
                        })
                    ])
                }
            "#,
        )
        .unwrap();
    let primitives = runtime_engine.primitive_registry();
    let schema = runtime_engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root_path = ComponentInstancePath::root("App", "root");
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        root_path.clone(),
        Some("main".to_owned()),
        std::collections::BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut runtime_engine).unwrap();
    let root = Rc::new(RefCell::new(lifecycle.root().unwrap().clone()));
    let lifecycle = Rc::new(RefCell::new(lifecycle));
    let runtime_engine = Rc::new(RefCell::new(runtime_engine));
    let captured_root = Rc::clone(&root);
    let captured_lifecycle = Rc::clone(&lifecycle);
    let captured_engine = Rc::clone(&runtime_engine);
    let errors = Rc::new(RefCell::new(Vec::new()));
    let captured_errors = Rc::clone(&errors);
    let dispatcher = NodeEventDispatcher::new(move |callback, payload, _, _, app| {
        let result = {
            let engine = captured_engine.borrow();
            captured_lifecycle
                .borrow()
                .invoke_callback_transactional(&engine, &callback, payload)
        };
        if let Err(error) = result {
            captured_errors.borrow_mut().push(error.to_string());
            return EventPropagation::Handled;
        }
        if let Err(error) = captured_lifecycle
            .borrow_mut()
            .render(&mut captured_engine.borrow_mut())
            .map(|root| {
                *captured_root.borrow_mut() = root.clone();
                app.refresh_windows();
            })
        {
            captured_errors.borrow_mut().push(error.to_string());
        }
        EventPropagation::Handled
    });
    let window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        let _ = window;
        KeyboardHost {
            root,
            tree: gpui_rhai::RetainedUiTree::new(),
            dispatcher,
            host_focus,
            primitives,
        }
    });
    cx.run_until_parked();

    cx.simulate_input(*window, "a👩‍💻b");
    cx.simulate_keystrokes(*window, "enter");
    cx.simulate_input(*window, "cd");
    cx.run_until_parked();

    assert!(errors.borrow().is_empty(), "{:?}", errors.borrow());
    assert_eq!(
        runtime.borrow().component_state.get(&root_path, "value"),
        Some(&UiValue::String("a👩‍💻b\nc".to_owned()))
    );

    cx.write_to_clipboard(gpui::ClipboardItem::new_string("👨‍👩‍👧‍👦abcdef".to_owned()));
    cx.simulate_keystrokes(*window, "cmd-a cmd-v cmd-a cmd-c");
    assert_eq!(
        cx.read_from_clipboard().and_then(|item| item.text()),
        Some("👨‍👩‍👧‍👦abcd".to_owned())
    );
    cx.simulate_keystrokes(*window, "cmd-x");
    assert_eq!(
        runtime.borrow().component_state.get(&root_path, "value"),
        Some(&UiValue::String(String::new()))
    );

    cx.simulate_keystrokes(*window, "tab cmd-a cmd-c");
    assert_eq!(
        cx.read_from_clipboard().and_then(|item| item.text()),
        Some("Read only 中文".to_owned())
    );
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("mutated".to_owned()));
    cx.simulate_keystrokes(*window, "cmd-v cmd-x");
    cx.simulate_input(*window, "typed");
    assert_eq!(
        runtime
            .borrow()
            .component_state
            .get(&root_path, "read_only_value"),
        Some(&UiValue::String("Read only 中文".to_owned()))
    );
    assert!(errors.borrow().is_empty(), "{:?}", errors.borrow());
}

#[gpui::test]
fn read_only_input_allows_selection_and_copy_but_rejects_edits(cx: &mut TestAppContext) {
    cx.update(init_text_input);
    let source = EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
        ModuleId::parse("components/input").unwrap(),
        gpui_rhai_registry::INPUT_SOURCE.to_owned(),
    )]));
    let mut runtime_engine = RuntimeEngine::new();
    runtime_engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = runtime_engine
        .compile_self_contained_named(
            "ui/read_only_input.rhai",
            r#"
                import "components/input" as input;
                fn state_schema() { #{ fields: #{
                    value: #{ schema: #{ type: "string" },
                        "default": #{ type: "string", value: "Read only 中文" } }
                } } }
                fn changed(ctx, value) { ctx.set_state("value", value); }
                fn view(ctx) {
                    input::Input(#{
                        key: "reference", label: "Reference", value: ctx.get_state("value"),
                        read_only: true, on_change: Fn("changed")
                    })
                }
            "#,
        )
        .unwrap();
    let primitives = runtime_engine.primitive_registry();
    let schema = runtime_engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root_path = ComponentInstancePath::root("App", "root");
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        root_path.clone(),
        Some("main".to_owned()),
        std::collections::BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut runtime_engine).unwrap();
    let root = Rc::new(RefCell::new(lifecycle.root().unwrap().clone()));
    let lifecycle = Rc::new(RefCell::new(lifecycle));
    let runtime_engine = Rc::new(RefCell::new(runtime_engine));
    let errors = Rc::new(RefCell::new(Vec::new()));
    let captured_root = Rc::clone(&root);
    let captured_lifecycle = Rc::clone(&lifecycle);
    let captured_engine = Rc::clone(&runtime_engine);
    let captured_errors = Rc::clone(&errors);
    let dispatcher = NodeEventDispatcher::new(move |callback, payload, _, _, app| {
        let result = {
            let engine = captured_engine.borrow();
            captured_lifecycle
                .borrow()
                .invoke_callback_transactional(&engine, &callback, payload)
        };
        if let Err(error) = result {
            captured_errors.borrow_mut().push(error.to_string());
            return EventPropagation::Handled;
        }
        if let Err(error) = captured_lifecycle
            .borrow_mut()
            .render(&mut captured_engine.borrow_mut())
            .map(|root| {
                *captured_root.borrow_mut() = root.clone();
                app.refresh_windows();
            })
        {
            captured_errors.borrow_mut().push(error.to_string());
        }
        EventPropagation::Handled
    });
    let window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        host_focus.focus(window, cx);
        KeyboardHost {
            root,
            tree: gpui_rhai::RetainedUiTree::new(),
            dispatcher,
            host_focus,
            primitives,
        }
    });
    cx.run_until_parked();

    cx.simulate_keystrokes(*window, "tab cmd-a cmd-c");
    assert_eq!(
        cx.read_from_clipboard().and_then(|item| item.text()),
        Some("Read only 中文".to_owned())
    );
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("mutated".to_owned()));
    cx.simulate_keystrokes(*window, "cmd-v cmd-x");
    cx.simulate_input(*window, "typed");

    assert!(errors.borrow().is_empty(), "{:?}", errors.borrow());
    assert_eq!(
        runtime.borrow().component_state.get(&root_path, "value"),
        Some(&UiValue::String("Read only 中文".to_owned()))
    );
}

#[gpui::test]
fn autofocus_input_receives_typing_without_any_click(cx: &mut TestAppContext) {
    let (window, _) = Mount::new(
        r#"
                fn state_schema() { #{ fields: #{
                    text: #{ schema: #{ type: "string" },
                        "default": #{ type: "string", value: "" } }
                } } }
                fn changed(ctx, value) { ctx.set_state("text", value); }
                fn view(ctx) {
                    column([
                        gpui_rhai::TextInputPrimitive(#{
                            key: "seek",
                            value: ctx.get_state("text"),
                            autofocus: true,
                            typography: "body",
                            on_change: Fn("changed"),
                        }),
                        text(`typed:${ctx.get_state("text")}`)
                    ])
                }
            "#,
    )
    .bundled_assets()
    .ids("autofocus-window", "autofocus-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    // The boundary under test: no click, no tab — typing must land in the
    // input purely because autofocus took focus on first mount.
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_input("hi");
    visual.run_until_parked();

    let texts = {
        let root = window
            .read_with(cx, |host, cx| host.view.root(cx).unwrap().unwrap())
            .unwrap();
        let mut out = Vec::new();
        node_texts(&root, &mut out);
        out
    };
    assert!(
        texts.contains(&"typed:hi".to_owned()),
        "autofocus input did not receive typing: {texts:?}"
    );
}

// Palette live-preview shape: cursor movement calls set_theme, which marks
// every window dirty (full re-render). Focus must survive that, or typing
// dies after the first character. Reuses the proven overlay keyboard path.
const THEME_SWAP_PALETTE_SCRIPT: &str = r#"
import "components/input" as input;

fn state_schema() {
    #{ fields: #{
        open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        value: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        flip: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
    } }
}

fn set_open(ctx, open) { ctx.set_state("open", open) }

fn value_changed(ctx, value) { ctx.set_state("value", value) }

// Arrow-down swaps the whole theme -- the palette live-preview shape
// (cursor moves onto a theme entry and set_theme fires).
fn theme_step(ctx, payload) {
    let flip = !ctx.get_state("flip");
    ctx.set_state("flip", flip);
    ctx.set_theme("Default", if flip { "Light" } else { "Dark" });
}

fn view(ctx) {
    column([
        text(`typed:${ctx.get_state("value")}`),
        overlay(
            text("open palette")
                .with_style(style().height(px(28)))
                .accessibility_role("button")
                .accessibility_label("open palette"),
            column([
                input::Input(#{ key: "filter", label: "Filter", value: ctx.get_state("value"),
                    placeholder: "filter", on_change: Fn("value_changed") })
            // key handlers force an interaction wrapper, and wrappers are
            // tab stops by default -- without tab_stop(false) the overlay's
            // initial_focus "first" lands on this column instead of the
            // input (the exact palette bug this test guards).
            ]).with_style(style().width(px(320)).padding(px(8)))
                .on("key:down", Fn("theme_step")).tab_stop(false),
            #{ id: "palette", kind: "dialog", placement: "center",
               open: ctx.get_state("open"), modal: true, initial_focus: "first" }
        ).with_key("palette").on_open_change(Fn("set_open"))
    ]).with_style(style().width(relative(1.0)).height(relative(1.0)).gap(px(4)))
}
"#;

#[gpui::test]
fn set_theme_during_typing_keeps_input_focus(cx: &mut TestAppContext) {
    let (window, view) = Mount::new(THEME_SWAP_PALETTE_SCRIPT)
        .registry("components/input")
        .bundled_assets()
        .ids("theme-swap-window", "theme-swap-view")
        .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();

    // Open the palette: initial_focus "first" lands focus in the input
    // (keyboard path proven by overlay_initial_focus_first_...).
    click_palette_trigger(&mut visual, &view);

    // The real palette sequence: type, arrow-down (theme swap fires),
    // type again. If the full re-render drops focus, "b" never arrives.
    visual.simulate_input("a");
    visual.run_until_parked();
    visual.simulate_keystrokes("down");
    visual.run_until_parked();
    visual.simulate_input("b");
    visual.run_until_parked();
    let texts = palette_texts(&mut visual, &view);
    assert!(
        texts.contains(&"typed:ab".to_owned()),
        "focus must survive theme swaps: {texts:?}"
    );
}
