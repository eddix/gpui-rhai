//! Menus: a menu trigger routes roving keys and Enter through the current state, and a
//! ContextMenu anchors at the right click and closes with Escape.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{KeyboardHost, Mount, bounds, palette_texts, registry_source};
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{
    ComponentInstancePath, EmbeddedScriptSource, EventPropagation, ModuleId, NodeEventDispatcher,
    PrimitiveRegistry, RestrictedModuleResolver, RuntimeEngine, ScriptLifecycle, UiNodeKind,
    UiRuntimeState, UiValue,
};

#[gpui::test]
fn menu_trigger_routes_roving_and_enter_keys_through_current_rhai_state(cx: &mut TestAppContext) {
    let source = EmbeddedScriptSource::new(std::collections::BTreeMap::from([
        (
            ModuleId::parse("components/menu").unwrap(),
            registry_source("components/menu").to_owned(),
        ),
        (
            ModuleId::parse("components/divider").unwrap(),
            registry_source("components/divider").to_owned(),
        ),
        (
            ModuleId::parse("components/kbd").unwrap(),
            registry_source("components/kbd").to_owned(),
        ),
    ]));
    let mut runtime_engine = RuntimeEngine::new();
    runtime_engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
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
                // The menu asks to close after the action, through `opened`.
                fn acted(ctx, value) { ctx.set_state("action", value); }
                fn view(ctx) {
                    menu::Menu(#{
                        key: "file", label: "File menu", trigger: text("File"),
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
    let captured_runtime = Rc::clone(&runtime);
    let dispatcher = NodeEventDispatcher::new(move |callback, payload, _, _, app| {
        captured_events
            .borrow_mut()
            .push((callback.name().to_owned(), payload.clone()));
        let result = {
            let engine = captured_engine.borrow();
            let lifecycle = captured_lifecycle.borrow();
            // Menu reports row activations as component events, which the host delivers
            // to the caller after the handler returns.
            lifecycle
                .invoke_callback_transactional(&engine, &callback, payload)
                .and_then(|_| {
                    let pending = captured_runtime.borrow_mut().drain_batch().events;
                    for event in pending {
                        lifecycle.invoke_component_event_transactional(&engine, event)?;
                    }
                    Ok(())
                })
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

#[gpui::test]
fn context_menu_anchors_at_the_right_click_and_closes_with_escape(cx: &mut TestAppContext) {
    let (window, view) = Mount::new(
        r#"
                    import "components/context_menu" as context_menu;
                    fn state_schema() { #{ fields: #{
                        open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
                        active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "copy" } }
                    } } }
                    fn opened(ctx, open) { ctx.set_state("open", open); }
                    fn active(ctx, value) { ctx.set_state("active", value); }
                    fn action(ctx, value) { () }
                    fn view(ctx) {
                        column([
                            text(`context-open:${ctx.get_state("open")}`),
                            context_menu::ContextMenu(#{ key: "row-menu", label: "Row actions",
                                trigger: text("Context target").with_style(style()
                                    .width(px(220)).height(px(80)).padding(px(12))
                                    .background(theme_color("surface_raised"))),
                                open: ctx.get_state("open"), active_value: ctx.get_state("active"),
                                items: [#{ kind: "item", value: "copy", label: "Copy" },
                                    #{ kind: "item", value: "delete", label: "Delete" }],
                                on_open_change: Fn("opened"), on_active_change: Fn("active"),
                                on_action: Fn("action") })
                        ]).with_style(style().padding(px(24)).gap(px(8)))
                    }
        "#,
    )
    .registry("components/context_menu")
    .registry("components/menu")
    .registry("components/divider")
    .registry("components/kbd")
    .bundled_assets()
    .ids("context-menu-window", "context-menu-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    let target = bounds(&mut visual, &view, "text", "Context target");
    let click = point(
        px((target.x + target.width * 0.7) as f32),
        px((target.y + target.height * 0.6) as f32),
    );
    visual.simulate_mouse_down(click, MouseButton::Right, Modifiers::default());
    visual.simulate_mouse_up(click, MouseButton::Right, Modifiers::default());
    visual.run_until_parked();
    let texts = palette_texts(&mut visual, &view);
    let error = visual.update(|_, cx| view.last_error(cx));
    assert!(
        texts.contains(&"context-open:true".to_owned()),
        "context menu did not open: texts={texts:?} error={error:?}"
    );
    let anchor = visual.update(|_, cx| {
        let root = view.root(cx).unwrap().unwrap();
        let UiNodeKind::Box { children } = root.kind() else {
            unreachable!()
        };
        let UiNodeKind::Overlay { spec, .. } = children[1].kind() else {
            unreachable!()
        };
        spec.anchor.unwrap()
    });
    assert!((anchor.x - f64::from(click.x)).abs() < 0.5);
    assert!((anchor.y - f64::from(click.y)).abs() < 0.5);

    visual.simulate_keystrokes("escape");
    visual.run_until_parked();
    assert!(palette_texts(&mut visual, &view).contains(&"context-open:false".to_owned()));
}
