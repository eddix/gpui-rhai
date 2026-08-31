use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, Modifiers, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestAppContext, VisualTestContext, Window, div, point, px,
};
use gpui_rhai::{
    ActionId, ComponentInstancePath, EmbeddedScriptSource, EmbeddedScriptView, EventPropagation,
    GpuiNodeRenderer, HostCallback, InteractionState, KeyBindingSpec, LiteralColorResolver,
    ModuleId, NodeEventDispatcher, OverlayDismissPolicy, OverlayId, OverlayKind, OverlayNodeSpec,
    OverlayPlacement, PrimitiveEventEmitter, PrimitiveHandler, PrimitiveInstance, PrimitiveNode,
    PrimitiveProps, PrimitiveRegistry, PrimitiveTheme, PrimitiveValue, RestrictedModuleResolver,
    RuntimeEngine, ScriptLifecycle, ScriptViewConfig, ScriptViewHandle, ScriptViewHost,
    TextInputPrimitiveHandler, UiNode, UiRuntimeState, UiValue, init_text_area, init_text_input,
    text_input_primitive_descriptor,
};

struct KeyboardHost {
    root: Rc<RefCell<UiNode>>,
    tree: gpui_rhai::RetainedUiTree,
    dispatcher: NodeEventDispatcher,
    host_focus: FocusHandle,
    primitives: PrimitiveRegistry,
}

impl Render for KeyboardHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let root = self.root.borrow().clone();
        let content = self.tree.reconcile(root).map_or_else(
            |error| div().child(error.to_string()).into_any_element(),
            |_| {
                self.primitives.retain_tree(&self.tree).map_or_else(
                    |error| div().child(error.to_string()).into_any_element(),
                    |_| {
                        GpuiNodeRenderer::render_retained_with_dispatcher(
                            &self.tree,
                            &LiteralColorResolver,
                            &InteractionState::default(),
                            &self.primitives,
                            &self.dispatcher,
                        )
                    },
                )
            },
        );
        div()
            .track_focus(&self.host_focus)
            .on_key_down(|event, window, cx| {
                if event.keystroke.key.as_str() == "tab" {
                    if event.keystroke.modifiers.shift {
                        window.focus_prev();
                    } else {
                        window.focus_next();
                    }
                    cx.stop_propagation();
                }
            })
            .child(content)
    }
}

struct HostCallbackTestHost {
    root: UiNode,
    host_focus: FocusHandle,
    parent_clicks: Rc<RefCell<usize>>,
    bubbled_keys: Rc<RefCell<Vec<String>>>,
}

impl Render for HostCallbackTestHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let parent_clicks = Rc::clone(&self.parent_clicks);
        let bubbled_keys = Rc::clone(&self.bubbled_keys);
        div()
            .id("host-callback-parent")
            .size_full()
            .track_focus(&self.host_focus)
            .on_click(move |_, _, _| *parent_clicks.borrow_mut() += 1)
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key.as_str() == "tab" {
                    window.focus_next();
                    cx.stop_propagation();
                } else {
                    bubbled_keys
                        .borrow_mut()
                        .push(event.keystroke.key.to_string());
                }
            })
            .child(GpuiNodeRenderer::render(&self.root))
    }
}

#[gpui::test]
fn tab_order_skips_disabled_nodes_and_enter_activates_focus(cx: &mut TestAppContext) {
    let mut engine = RuntimeEngine::new();
    let compiled = engine
        .compile(
            r#"
                fn view(ctx) { text("keyboard test") }
                fn activated(ctx, payload) { () }
            "#,
        )
        .expect("compile callback fixture");
    let callback = engine
        .callback(&compiled, "activated")
        .expect("resolve callback fixture");
    let button = |label: &str, payload: i64| {
        UiNode::text(label)
            .with_key(label)
            .with_handler("click", callback.clone())
            .with_handler_payload("click", UiValue::Integer(payload))
    };
    let root = UiNode::column(vec![
        button("disabled", 0).with_attribute("disabled", UiValue::Bool(true)),
        UiNode::overlay(
            UiNode::text("overlay trigger"),
            UiNode::text("overlay content"),
            OverlayNodeSpec {
                id: OverlayId::new("keyboard-overlay"),
                parent: None,
                kind: OverlayKind::Popover,
                placement: OverlayPlacement::Bottom,
                open: false,
                gap: 4.0,
                modal: false,
                dismiss: OverlayDismissPolicy {
                    escape: true,
                    outside: true,
                },
                tooltip_delays: None,
            },
        )
        .with_key("overlay")
        .with_handler("open_change", callback.clone()),
        button("first", 1),
        button("second", 2),
    ]);
    let activations = Rc::new(RefCell::new(Vec::new()));
    let captured = Rc::clone(&activations);
    let dispatcher = NodeEventDispatcher::new(move |_, payload, _, _| {
        if let UiValue::Integer(value) = payload {
            captured.borrow_mut().push(value);
        }
        EventPropagation::Handled
    });
    let window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        host_focus.focus(window);
        KeyboardHost {
            root: Rc::new(RefCell::new(root)),
            tree: gpui_rhai::RetainedUiTree::new(),
            dispatcher,
            host_focus,
            primitives: PrimitiveRegistry::new(),
        }
    });
    cx.run_until_parked();

    cx.simulate_keystrokes(*window, "tab tab enter tab enter shift-tab enter");

    assert_eq!(*activations.borrow(), vec![1, 2, 1]);
}

#[gpui::test]
fn host_callbacks_dispatch_without_a_script_runtime(cx: &mut TestAppContext) {
    let received = Rc::new(RefCell::new(Vec::new()));
    let pointer_received = Rc::clone(&received);
    let keyboard_received = Rc::clone(&received);
    let root = UiNode::text("Host action")
        .with_host_handler(
            "click",
            HostCallback::new("widget.pointer", move |payload, _, _| {
                pointer_received.borrow_mut().push(payload);
                EventPropagation::Propagate
            }),
        )
        .with_handler_payload("click", UiValue::Integer(7))
        .with_host_handler(
            "key:enter",
            HostCallback::new("widget.keyboard", move |payload, _, _| {
                keyboard_received.borrow_mut().push(payload);
                EventPropagation::Handled
            }),
        )
        .with_handler_payload("key:enter", UiValue::String("enter".to_owned()));
    let parent_clicks = Rc::new(RefCell::new(0));
    let bubbled_keys = Rc::new(RefCell::new(Vec::new()));
    let parent_clicks_for_window = Rc::clone(&parent_clicks);
    let bubbled_keys_for_window = Rc::clone(&bubbled_keys);
    let window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        host_focus.focus(window);
        HostCallbackTestHost {
            root,
            host_focus,
            parent_clicks: parent_clicks_for_window,
            bubbled_keys: bubbled_keys_for_window,
        }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    let bounds = visual.debug_bounds("root").expect("host node bounds");
    visual.simulate_click(
        point(
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + bounds.size.height / 2.0,
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();
    assert_eq!(*parent_clicks.borrow(), 1);
    assert_eq!(*received.borrow(), vec![UiValue::Integer(7)]);

    visual.simulate_keystrokes("tab enter");
    visual.run_until_parked();
    assert_eq!(
        *received.borrow(),
        vec![UiValue::Integer(7), UiValue::String("enter".to_owned())]
    );
    assert!(bubbled_keys.borrow().is_empty());
}

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
    let dispatcher = NodeEventDispatcher::new(|_, _, _, _| EventPropagation::Handled);
    let window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        host_focus.focus(window);
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
fn custom_primitive_rejects_invalid_payload_before_host_callback(cx: &mut TestAppContext) {
    struct InvalidPayloadHandler {
        rejected: Rc<RefCell<bool>>,
    }

    impl PrimitiveHandler for InvalidPayloadHandler {
        fn render(
            &mut self,
            _: &PrimitiveInstance,
            events: &PrimitiveEventEmitter,
            _: &PrimitiveTheme,
            window: &mut Window,
            app: &mut gpui::App,
        ) -> Result<gpui::AnyElement, String> {
            *self.rejected.borrow_mut() = events
                .emit("change", UiValue::Bool(true), window, app)
                .is_err();
            Ok(div().into_any_element())
        }
    }

    let registry = PrimitiveRegistry::new();
    let descriptor = text_input_primitive_descriptor();
    let primitive = descriptor.id.clone();
    let rejected = Rc::new(RefCell::new(false));
    registry
        .register(
            descriptor,
            InvalidPayloadHandler {
                rejected: Rc::clone(&rejected),
            },
        )
        .unwrap();
    let invoked = Rc::new(RefCell::new(0));
    let invoked_by_callback = Rc::clone(&invoked);
    let props = PrimitiveProps::new().with(
        "on_change",
        PrimitiveValue::Callback(
            HostCallback::new("widget.invalid", move |_, _, _| {
                *invoked_by_callback.borrow_mut() += 1;
                EventPropagation::Handled
            })
            .into(),
        ),
    );
    let root = UiNode::custom(PrimitiveNode {
        primitive,
        key: Some("invalid-payload".to_owned()),
        props,
    });
    let dispatcher = NodeEventDispatcher::new(|_, _, _, _| EventPropagation::Handled);
    let _window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        host_focus.focus(window);
        KeyboardHost {
            root: Rc::new(RefCell::new(root)),
            tree: gpui_rhai::RetainedUiTree::new(),
            dispatcher,
            host_focus,
            primitives: registry,
        }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    assert!(*rejected.borrow());
    assert_eq!(*invoked.borrow(), 0);
}

#[gpui::test]
fn nested_overlay_renders_inside_parent_deferred_subtree(cx: &mut TestAppContext) {
    let child = UiNode::overlay(
        UiNode::text("Nested popover"),
        UiNode::text("Nested content"),
        OverlayNodeSpec {
            id: OverlayId::new("child-popover"),
            parent: Some(OverlayId::new("parent-dialog")),
            kind: OverlayKind::Popover,
            placement: OverlayPlacement::Right,
            open: true,
            gap: 4.0,
            modal: false,
            dismiss: OverlayDismissPolicy {
                escape: true,
                outside: true,
            },
            tooltip_delays: None,
        },
    )
    .with_key("child-popover");
    let root = UiNode::overlay(
        UiNode::text("Open dialog"),
        child,
        OverlayNodeSpec {
            id: OverlayId::new("parent-dialog"),
            parent: None,
            kind: OverlayKind::Dialog,
            placement: OverlayPlacement::Bottom,
            open: true,
            gap: 0.0,
            modal: true,
            dismiss: OverlayDismissPolicy {
                escape: true,
                outside: true,
            },
            tooltip_delays: None,
        },
    )
    .with_key("parent-dialog");
    let window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        host_focus.focus(window);
        KeyboardHost {
            root: Rc::new(RefCell::new(root)),
            tree: gpui_rhai::RetainedUiTree::new(),
            dispatcher: NodeEventDispatcher::new(|_, _, _, _| EventPropagation::Handled),
            host_focus,
            primitives: PrimitiveRegistry::new(),
        }
    });

    cx.run_until_parked();
    assert!(cx.windows().contains(&*window));
}

struct SingleEmbeddedHost {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for SingleEmbeddedHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

#[gpui::test]
fn dropdown_pointer_updates_transactional_rhai_caller_state(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([
            (
                entry,
                r#"
                    import "components/dropdown" as dropdown;
                    fn state_schema() { #{ fields: #{
                        open: #{ schema: #{ type: "bool" },
                            "default": #{ type: "bool", value: false } },
                        selected: #{ schema: #{ type: "array", max_items: 1,
                            items: #{ type: "string" } }, "default": #{ type: "array",
                            value: [#{ type: "string", value: "default-dark" }] } }
                    } } }
                    fn changed(ctx, values) {
                        ctx.set_state("selected", values);
                        ctx.set_state("open", false);
                    }
                    fn opened(ctx, open) { ctx.set_state("open", open); }
                    fn view(ctx) {
                        let selected = ctx.get_state("selected");
                        column([
                            text(`Selected: ${selected[0]}`),
                            dropdown::Dropdown(#{
                                key: "theme",
                                options: [
                                    #{ value: "default-dark", label: "Default Dark" },
                                    #{ value: "tokyo-night", label: "Tokyo Night" },
                                    #{ value: "tokyo-storm", label: "Tokyo Storm" }
                                ],
                                selected: selected, open: ctx.get_state("open"),
                                searchable: false,
                                on_change: Fn("changed"), on_open_change: Fn("opened")
                            })
                        ])
                    }
                "#
                .to_owned(),
            ),
            (
                ModuleId::parse("components/dropdown").unwrap(),
                include_str!("../../../registry/components/dropdown.rhai").to_owned(),
            ),
            (
                ModuleId::parse("components/input").unwrap(),
                include_str!("../../../registry/components/input.rhai").to_owned(),
            ),
        ])),
        include_str!("../../../registry/themes/default_dark.rhai"),
    )
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("dropdown-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("dropdown-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some((host.clone(), view.clone()));
        SingleEmbeddedHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let (host, view) = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    let trigger = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("combobox", "")
            .next()
            .unwrap()
            .geometry
            .unwrap()
    });
    visual.simulate_click(
        point(
            px((trigger.visual.x + trigger.visual.width / 2.0) as f32),
            px((trigger.visual.y + trigger.visual.height / 2.0) as f32),
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();
    let placement = host.overlay_placement("dropdown-view", "theme").unwrap();
    visual.simulate_click(
        point(
            px((placement.bounds.x + placement.bounds.width / 2.0) as f32),
            px((placement.bounds.y + 4.0 + 32.0 + 16.0) as f32),
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut texts = Vec::new();
    node_texts(&root, &mut texts);
    assert!(
        texts.contains(&"Selected: tokyo-night".to_owned()),
        "{texts:?}; placement={placement:?}"
    );
}

#[gpui::test]
fn native_input_updates_rhai_state_and_clipboard_with_unicode(cx: &mut TestAppContext) {
    cx.update(init_text_input);
    let input = include_str!("../../../registry/components/input.rhai");
    let source = EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
        ModuleId::parse("components/input").unwrap(),
        input.to_owned(),
    )]));
    let mut runtime_engine = RuntimeEngine::new();
    runtime_engine
        .set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
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
                        key: "name", value: ctx.get_state("value"),
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
    let dispatcher = NodeEventDispatcher::new(move |callback, payload, _, app| {
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
        host_focus.focus(window);
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
    let textarea = include_str!("../../../registry/components/textarea.rhai");
    let source = EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
        ModuleId::parse("components/textarea").unwrap(),
        textarea.to_owned(),
    )]));
    let mut runtime_engine = RuntimeEngine::new();
    runtime_engine
        .set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
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
                            key: "notes", value: ctx.get_state("value"),
                            placeholder: "Notes", min_rows: 2, max_rows: 4,
                            max_length: 5, autofocus: true, on_change: Fn("changed")
                        }),
                        textarea::Textarea(#{
                            key: "reference", value: ctx.get_state("read_only_value"),
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
    let dispatcher = NodeEventDispatcher::new(move |callback, payload, _, app| {
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

    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
        "👨‍👩‍👧‍👦abcdef".to_owned(),
    ));
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
        include_str!("../../../registry/components/input.rhai").to_owned(),
    )]));
    let mut runtime_engine = RuntimeEngine::new();
    runtime_engine
        .set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
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
                        key: "reference", value: ctx.get_state("value"),
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
    let dispatcher = NodeEventDispatcher::new(move |callback, payload, _, app| {
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
        host_focus.focus(window);
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
fn menu_trigger_routes_roving_and_enter_keys_through_current_rhai_state(
    cx: &mut TestAppContext,
) {
    let source = EmbeddedScriptSource::new(std::collections::BTreeMap::from([
        (
            ModuleId::parse("components/menu").unwrap(),
            include_str!("../../../registry/components/menu.rhai").to_owned(),
        ),
        (
            ModuleId::parse("components/divider").unwrap(),
            include_str!("../../../registry/components/divider.rhai").to_owned(),
        ),
    ]));
    let mut runtime_engine = RuntimeEngine::new();
    runtime_engine
        .set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = runtime_engine
        .compile_self_contained_named(
            "ui/menu_lifecycle.rhai",
            r#"
                import "components/menu" as menu;
                fn state_schema() { #{ fields: #{
                    open: #{ schema: #{ type: "bool" },
                        "default": #{ type: "bool", value: false } },
                    active: #{ schema: #{ type: "string" },
                        "default": #{ type: "string", value: "new" } },
                    action: #{ schema: #{ type: "string" },
                        "default": #{ type: "string", value: "" } }
                } } }
                fn opened(ctx, open) { ctx.set_state("open", open); }
                fn activated(ctx, value) { ctx.set_state("active", value); }
                fn acted(ctx, value) {
                    ctx.set_state("action", value);
                    ctx.set_state("open", false);
                }
                fn view(ctx) {
                    menu::Menu(#{
                        key: "file", trigger: text("File"),
                        open: ctx.get_state("open"), active_value: ctx.get_state("active"),
                        items: [
                            #{ kind: "item", value: "new", label: "New" },
                            #{ kind: "separator" },
                            #{ kind: "item", value: "open", label: "Open" }
                        ],
                        on_action: Fn("acted"), on_active_change: Fn("activated"),
                        on_open_change: Fn("opened")
                    })
                }
            "#,
        )
        .unwrap();
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
    let events = Rc::new(RefCell::new(Vec::new()));
    let captured_root = Rc::clone(&root);
    let captured_lifecycle = Rc::clone(&lifecycle);
    let captured_engine = Rc::clone(&runtime_engine);
    let captured_errors = Rc::clone(&errors);
    let captured_events = Rc::clone(&events);
    let dispatcher = NodeEventDispatcher::new(move |callback, payload, _, app| {
        captured_events
            .borrow_mut()
            .push((callback.name().to_owned(), payload.clone()));
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
        host_focus.focus(window);
        KeyboardHost {
            root,
            tree: gpui_rhai::RetainedUiTree::new(),
            dispatcher,
            host_focus,
            primitives: PrimitiveRegistry::new(),
        }
    });
    cx.run_until_parked();

    cx.simulate_keystrokes(*window, "tab enter");
    cx.simulate_keystrokes(*window, "down n o enter");

    assert!(errors.borrow().is_empty(), "{:?}", errors.borrow());
    let runtime = runtime.borrow();
    assert_eq!(
        runtime.component_state.get(&root_path, "open"),
        Some(&UiValue::Bool(false)),
        "events: {:?}",
        events.borrow()
    );
    assert_eq!(
        runtime.component_state.get(&root_path, "active"),
        Some(&UiValue::String("open".to_owned()))
    );
    assert_eq!(
        runtime.component_state.get(&root_path, "action"),
        Some(&UiValue::String("open".to_owned()))
    );
}

const EMBEDDED_VIEW_SCRIPT: &str = r#"
import "components/toast" as toast;
fn state_schema() {
    #{ fields: #{
        count: #{ schema: #{ type: "integer" },
            "default": #{ type: "integer", value: 0 } },
        open: #{ schema: #{ type: "bool" },
            "default": #{ type: "bool", value: __OPEN__ } },
    } }
}
fn increment(ctx, payload) { ctx.set_state("count", ctx.get_state("count") + 1); }
fn set_open(ctx, open) { ctx.set_state("open", open); }
fn dismissed(ctx, id) { () }
fn view(ctx) {
    column([
        toast::Toast(#{
            key: "toasts",
            items: [#{ id: "shared-toast", title: `Toast ${ctx.view_id()}`,
                duration_ms: 60000 }],
            on_dismiss: Fn("dismissed")
        }),
        text(`View: ${ctx.view_id()}`),
        text(`Responsive: ${ctx.viewport_class()}`),
        text(`Count: ${ctx.get_state("count")}`),
        text("Increment").with_style(
            style().height(px(32)).padding(px(6)).background(theme_color("accent"))
        ).on_click(Fn("increment")),
        overlay(
            text("Shared overlay trigger").with_style(style().height(px(28))),
            row([text("A 280 point overlay")]).with_style(
                style().width(px(280)).height(px(64)).padding(px(8))
                    .background(theme_color("surface_raised"))
            ),
            #{ id: "shared-overlay", kind: "dropdown", open: ctx.get_state("open"),
                placement: "bottom", dismiss_on_escape: true, dismiss_on_outside: true }
        ).with_key("shared-overlay").on_open_change(Fn("set_open"))
    ]).with_style(style().width(relative(1.0)).height(relative(1.0)).gap(px(4)))
}
"#;

fn prepared_embedded_test_view(open: bool) -> gpui_rhai::PreparedScriptView {
    let entry = ModuleId::parse("main").unwrap();
    EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([
            (
                entry,
                EMBEDDED_VIEW_SCRIPT.replace("__OPEN__", if open { "true" } else { "false" }),
            ),
            (
                ModuleId::parse("components/toast").unwrap(),
                include_str!("../../../registry/components/toast.rhai").to_owned(),
            ),
        ])),
        include_str!("../../../registry/themes/default_dark.rhai"),
    )
    .prepare()
    .unwrap()
}

struct EmbeddedIntegrationHost {
    host: ScriptViewHost,
    first: ScriptViewHandle,
    second: ScriptViewHandle,
    third: ScriptViewHandle,
}

impl EmbeddedIntegrationHost {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_with_second_open(window, cx, false)
    }

    fn new_with_second_open(
        window: &mut Window,
        cx: &mut Context<Self>,
        second_open: bool,
    ) -> Self {
        let host = ScriptViewHost::new("integration-window", cx).unwrap();
        let first = prepared_embedded_test_view(true)
            .mount(
                ScriptViewConfig::new("first"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        let second = prepared_embedded_test_view(second_open)
            .mount(
                ScriptViewConfig::new("second"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        let third = prepared_embedded_test_view(false)
            .mount(
                ScriptViewConfig::new("third"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        Self {
            host,
            first,
            second,
            third,
        }
    }

    fn dispose_and_remount_third(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.third.dispose(cx).unwrap();
        self.third = prepared_embedded_test_view(false)
            .mount(
                ScriptViewConfig::new("third"),
                self.host.clone(),
                window,
                cx,
            )
            .unwrap();
        cx.notify();
    }
}

impl Render for EmbeddedIntegrationHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(
            div()
                .size_full()
                .flex()
                .gap(px(12.0))
                .items_start()
                .child(
                    div()
                        .w(px(200.0))
                        .h(px(240.0))
                        .child(self.first.element().unwrap()),
                )
                .child(
                    div()
                        .w(px(240.0))
                        .h(px(240.0))
                        .child(self.second.element().unwrap()),
                )
                .child(
                    div()
                        .w(px(280.0))
                        .h(px(240.0))
                        .child(self.third.element().unwrap()),
                ),
        )
    }
}

fn node_texts(node: &UiNode, output: &mut Vec<String>) {
    match node.kind() {
        gpui_rhai::UiNodeKind::Text { text } => output.push(text.to_string()),
        gpui_rhai::UiNodeKind::Box { children }
        | gpui_rhai::UiNodeKind::Fragment { children } => {
            for child in children {
                node_texts(child, output);
            }
        }
        gpui_rhai::UiNodeKind::Overlay {
            trigger, content, ..
        } => {
            node_texts(trigger, output);
            node_texts(content, output);
        }
        gpui_rhai::UiNodeKind::Layer { content, .. } => node_texts(content, output),
        gpui_rhai::UiNodeKind::ErrorBoundary { child, fallback } => {
            node_texts(child, output);
            node_texts(fallback, output);
        }
        gpui_rhai::UiNodeKind::VirtualCollection { spec } => {
            for child in spec.realized.values() {
                node_texts(child, output);
            }
        }
        _ => {}
    }
}

fn overlay_open(node: &UiNode) -> Option<bool> {
    match node.kind() {
        gpui_rhai::UiNodeKind::Overlay { spec, .. } => Some(spec.open),
        gpui_rhai::UiNodeKind::Box { children }
        | gpui_rhai::UiNodeKind::Fragment { children } => children.iter().find_map(overlay_open),
        gpui_rhai::UiNodeKind::Layer { content, .. } => overlay_open(content),
        gpui_rhai::UiNodeKind::ErrorBoundary { child, fallback } => {
            overlay_open(child).or_else(|| overlay_open(fallback))
        }
        gpui_rhai::UiNodeKind::VirtualCollection { spec } => {
            spec.realized.values().find_map(overlay_open)
        }
        _ => None,
    }
}

#[gpui::test]
fn multiple_embedded_views_share_host_mechanics_but_isolate_runtime_state(
    cx: &mut TestAppContext,
) {
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = EmbeddedIntegrationHost::new(window, cx);
        *captured_for_window.borrow_mut() = Some((
            host.host.clone(),
            host.first.clone(),
            host.second.clone(),
            host.third.clone(),
        ));
        host
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let (host, first, second, third) = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();

    let first_root = visual
        .update(|_, cx| first.root(cx).unwrap().unwrap());
    let mut first_text = Vec::new();
    node_texts(&first_root, &mut first_text);
    assert!(first_text.contains(&"View: first".to_owned()));
    assert!(first_text.contains(&"Responsive: compact".to_owned()));

    let first_placement = host.overlay_placement("first", "shared-overlay").unwrap();
    assert!(first_placement.bounds.width > 200.0);
    let toast_count = [&first, &second, &third]
        .into_iter()
        .map(|view| {
            visual.update(|_, cx| {
                view.accessibility_snapshot(cx)
                    .unwrap()
                    .nodes()
                    .filter(|node| node.role == "status")
                    .count()
            })
        })
        .sum::<usize>();
    assert_eq!(toast_count, 3);

    let increment = visual
        .debug_bounds("window:integration-window/view:third/root/4")
        .expect("third increment debug bounds");
    let increment_center = point(
        increment.origin.x + increment.size.width / 2.0,
        increment.origin.y + increment.size.height / 2.0,
    );
    visual.simulate_click(increment_center, Modifiers::default());
    visual.run_until_parked();
    let third_root = visual
        .update(|_, cx| third.root(cx).unwrap().unwrap());
    let mut third_text = Vec::new();
    node_texts(&third_root, &mut third_text);
    assert!(
        third_text.contains(&"Count: 1".to_owned()),
        "{third_text:?}"
    );
    assert_eq!(overlay_open(&third_root), Some(false));

    let first_root = visual
        .update(|_, cx| first.root(cx).unwrap().unwrap());
    assert_eq!(overlay_open(&first_root), Some(false));

    let second_root = visual
        .update(|_, cx| second.root(cx).unwrap().unwrap());
    assert_eq!(overlay_open(&second_root), Some(false));

    window
        .update(&mut visual, |root, window, cx| {
            root.dispose_and_remount_third(window, cx)
        })
        .unwrap();
    assert!(matches!(
        third.element(),
        Err(gpui_rhai::ScriptViewError::DisposedView(id)) if id == "third"
    ));
    visual.run_until_parked();
}

#[gpui::test]
fn duplicate_local_overlay_ids_are_namespaced_per_embedded_view(cx: &mut TestAppContext) {
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let _window = cx.add_window(move |window, cx| {
        let root = EmbeddedIntegrationHost::new_with_second_open(window, cx, true);
        *captured_for_window.borrow_mut() = Some(root.host.clone());
        root
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let host = captured.borrow().as_ref().unwrap().clone();
    let first = host.overlay_placement("first", "shared-overlay").unwrap();
    let second = host.overlay_placement("second", "shared-overlay").unwrap();
    assert!(first.bounds.width > 200.0);
    assert!(second.bounds.width > 200.0);
    assert_ne!(first.bounds.x, second.bounds.x);
}

#[gpui::test]
fn script_runtime_install_is_idempotent_and_key_conflicts_are_explicit(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        gpui_rhai::install(cx);
        gpui_rhai::install(cx);
        let host = ScriptViewHost::new("keys-window", cx).unwrap();
        let save = KeyBindingSpec::new(
            "cmd-s",
            ActionId::parse("document.save").unwrap(),
            None,
        )
        .unwrap();
        host.bind_keys([save.clone()], cx).unwrap();
        host.bind_keys([save], cx).unwrap();
        let conflicting = KeyBindingSpec::new(
            "cmd-s",
            ActionId::parse("document.sync").unwrap(),
            None,
        )
        .unwrap();
        assert!(matches!(
            host.bind_keys([conflicting], cx),
            Err(gpui_rhai::ScriptViewError::KeyBindingConflict { .. })
        ));
    });
}

struct DualHostRoot {
    left_host: ScriptViewHost,
    left: ScriptViewHandle,
    right_host: ScriptViewHost,
    right: ScriptViewHandle,
}

impl DualHostRoot {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let left_host = ScriptViewHost::new("left-domain", cx).unwrap();
        let left = prepared_embedded_test_view(true)
            .mount(
                ScriptViewConfig::new("left-view"),
                left_host.clone(),
                window,
                cx,
            )
            .unwrap();
        let right_host = ScriptViewHost::new("right-domain", cx).unwrap();
        let right = prepared_embedded_test_view(false)
            .mount(
                ScriptViewConfig::new("right-view"),
                right_host.clone(),
                window,
                cx,
            )
            .unwrap();
        Self {
            left_host,
            left,
            right_host,
            right,
        }
    }
}

impl Render for DualHostRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .gap(px(20.0))
            .child(
                div().w(px(320.0)).h(px(240.0)).child(
                    self.left_host
                        .container(self.left.element().expect("left live")),
                ),
            )
            .child(
                div().w(px(320.0)).h(px(240.0)).child(
                    self.right_host
                        .container(self.right.element().expect("right live")),
                ),
            )
    }
}

#[gpui::test]
fn separate_hosts_in_one_window_keep_overlay_domains_isolated(cx: &mut TestAppContext) {
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let root = DualHostRoot::new(window, cx);
        *captured_for_window.borrow_mut() = Some((root.left.clone(), root.right.clone()));
        root
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let (left, right) = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    let increment = visual
        .debug_bounds("window:right-domain/view:right-view/root/4")
        .expect("right increment bounds");
    visual.simulate_click(
        point(
            increment.origin.x + increment.size.width / 2.0,
            increment.origin.y + increment.size.height / 2.0,
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();

    let left_root = visual.update(|_, cx| left.root(cx).unwrap().unwrap());
    assert_eq!(overlay_open(&left_root), Some(true));
    let right_root = visual.update(|_, cx| right.root(cx).unwrap().unwrap());
    let mut right_text = Vec::new();
    node_texts(&right_root, &mut right_text);
    assert!(right_text.contains(&"Count: 1".to_owned()));
}
