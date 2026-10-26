mod common;

use common::{compile, engine_with, mount, new_runtime, official_modules, root_path};
use gpui_rhai::{UiNode, UiNodeKind, UiValue};

const PAGE: &str = r#"
import "components/toggle_group" as toggle_group;
fn set_filter(prefix, ctx, values) { ctx.set_state("filter", prefix + values[0]); }
fn handler() { Fn("set_filter").curry("") }
fn view(ctx) {
    toggle_group::ToggleGroup(#{key:"filter", label:"Filter",
        values:[ctx.get_state("filter")],
        items:[#{value:"all",label:"All"},#{value:"other",label:"Other"}],
        on_change: handler()})
}
"#;

#[test]
fn lexical_constructor_cannot_be_shadowed_in_entry_or_module_init() {
    let mut engine = gpui_rhai::RuntimeEngine::new();
    let error = engine
        .compile(r#"fn named_function(name){throw "executed";} fn view(ctx){text("ok")}"#)
        .err()
        .expect("reserved constructor");
    assert!(error.to_string().contains("reserved by the SDK"));
    let mut engine = engine_with([(
        "pages/bad",
        r#"
        private fn named_function(name){throw "executed";}
        fn view(ctx){text("ok")}
    "#,
    )]);
    let error = engine
        .compile_self_contained_named(
            "main",
            r#"import "pages/bad" as bad; fn view(ctx){bad::view(ctx)}"#,
        )
        .err()
        .expect("reserved module constructor");
    assert!(error.to_string().contains("reserved by the SDK"));
    assert!(!error.to_string().contains("executed"));
}

#[test]
fn synchronous_object_calls_preserve_the_mutable_this_pointer() {
    let mut engine = gpui_rhai::RuntimeEngine::new();
    let compiled = common::compile(
        &mut engine,
        "object-calls",
        r#"
        fn bump(delta){this.count+=delta;}
        fn view(ctx){let value=#{count:1};value.call(Fn("bump").curry(2));text(value.count.to_string())}
    "#,
    );
    let root = common::render(&mut engine, &compiled);
    assert!(
        matches!(root.kind(),UiNodeKind::Text{text} if text.as_str()=="3"),
        "{root:?}"
    );
}

#[test]
fn constructor_mapping_does_not_change_quoted_map_keys_or_string_data() {
    // Fn is reserved in bare Rhai: data keys use strings, not `#{Fn:...}`.
    for source in [
        r#"/* Fn("ignored") */ let value=#{"Fn":42};value["Fn"]"#,
        r"let value=#{named_function:42};value.named_function",
        r#""Fn(\"callback\")""#,
    ] {
        let expected = gpui_rhai::rhai::Engine::new()
            .eval::<gpui_rhai::rhai::Dynamic>(source)
            .expect("valid Rhai data expression");
        let runtime = gpui_rhai::RuntimeEngine::new();
        let actual = runtime.engine().eval::<gpui_rhai::rhai::Dynamic>(source);
        assert_eq!(
            format!("{expected:?}"),
            format!("{:?}", actual.expect("SDK data expression")),
            "{source}"
        );
    }
}

#[test]
fn synchronous_function_pointer_reentry_cannot_reset_operation_or_depth_budgets() {
    let mut engine = gpui_rhai::RuntimeEngine::new();
    engine.set_operation_limit(5000);
    let program = engine
        .compile(
            r#"
        fn work(){let total=0;for index in 0..100000 {total+=index;}total}
        fn view(ctx){text(Fn("work").call().to_string())}
    "#,
        )
        .unwrap();
    let error =
        common::try_render_in(&mut engine, &program, gpui_rhai::UiRuntimeState::new()).unwrap_err();
    assert!(
        matches!(error,gpui_rhai::RuntimeError::Evaluate(ref cause) if matches!(**cause,gpui_rhai::rhai::EvalAltResult::ErrorTerminated(..))),
        "{error}"
    );
    assert!(engine.last_failed_timing().unwrap().round_operations > 5000);
    let mut engine = gpui_rhai::RuntimeEngine::new();
    let program = engine
        .compile(r#"fn recurse(){Fn("recurse").call()} fn view(ctx){text(recurse().to_string())}"#)
        .unwrap();
    let error =
        common::try_render_in(&mut engine, &program, gpui_rhai::UiRuntimeState::new()).unwrap_err();
    assert!(error.to_string().contains("Stack overflow"), "{error}");
}

fn click(node: &UiNode) -> Option<gpui_rhai::ScriptCallback> {
    if node.attributes().get("label") == Some(&UiValue::String("Other".into())) {
        return node
            .handler("click")
            .and_then(gpui_rhai::UiEventHandler::as_script)
            .cloned();
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(click)
        }
        _ => None,
    }
}

#[test]
fn ordinary_module_callback_retains_lexical_scope_and_entry_state_owner() {
    let mut engine = engine_with(official_modules().chain([("pages/filter", PAGE)]));
    let compiled = compile(
        &mut engine,
        "main",
        r#"
        import "pages/filter" as page;
        fn state_schema(){#{fields:#{filter:#{schema:#{type:"string"},
            "default":#{type:"string",value:"all"}}}}}
        fn set_filter(prefix, ctx, values) { throw "wrong entry helper"; }
        fn view(ctx){page::view(ctx)}
    "#,
    );
    let state = new_runtime();
    let lifecycle = mount(&mut engine, compiled, &state);
    let callback = click(lifecycle.root().unwrap()).unwrap();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &callback, UiValue::String("other".into()))
        .unwrap();
    let events = state.borrow_mut().drain_batch().events;
    for event in events {
        lifecycle
            .invoke_component_event_transactional(&engine, event)
            .unwrap();
    }
    assert_eq!(
        state.borrow().component_state.get(&root_path(), "filter"),
        Some(&UiValue::String("other".into()))
    );
}

#[test]
fn forwarded_entry_callback_is_not_rebound_to_the_forwarding_module() {
    let page = PAGE
        .replace("on_change: handler()", "on_change: supplied")
        .replace("fn view(ctx)", "fn view(ctx, supplied)");
    let mut engine = engine_with([
        (
            "components/toggle_group",
            common::official_source("components/toggle_group"),
        ),
        ("pages/filter", page.as_str()),
    ]);
    let compiled = compile(
        &mut engine,
        "main",
        r#"
        import "pages/filter" as page;
        fn state_schema(){#{fields:#{filter:#{schema:#{type:"string"},
            "default":#{type:"string",value:"all"}}}}}
        fn set_filter(prefix, ctx, values) { ctx.set_state("filter", prefix + values[0]); }
        fn view(ctx){page::view(ctx, Fn("set_filter").curry("entry:"))}
    "#,
    );
    let state = new_runtime();
    let lifecycle = mount(&mut engine, compiled, &state);
    let callback = click(lifecycle.root().unwrap()).unwrap();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &callback, UiValue::String("other".into()))
        .unwrap();
    let events = state.borrow_mut().drain_batch().events;
    for event in events {
        lifecycle
            .invoke_component_event_transactional(&engine, event)
            .unwrap();
    }
    assert_eq!(
        state.borrow().component_state.get(&root_path(), "filter"),
        Some(&UiValue::String("entry:other".into()))
    );
}

#[test]
fn same_named_callbacks_have_distinct_environments_and_synchronous_call_works() {
    let page = r#"
        const PREFIX = "a:";
        fn same(ctx, value) { ctx.set_state("filter", global::PREFIX + value); }
        fn callback() { Fn("same") }
        fn add(prefix, value) { prefix + value }
        fn synchronous() { Fn("add").curry(global::PREFIX).call("sync") }
    "#;
    let other = page.replace("a:", "b:");
    let mut engine = engine_with([("pages/a", page), ("pages/b", other.as_str())]);
    let compiled = compile(
        &mut engine,
        "main",
        r#"
        import "pages/a" as a;
        import "pages/b" as b;
        fn state_schema(){#{fields:#{filter:#{schema:#{type:"string"},
            "default":#{type:"string",value:"all"}}}}}
        fn view(ctx){row([text(a::synchronous()).on_click(a::callback()),
                         text(b::synchronous()).on_click(b::callback())])}
    "#,
    );
    let state = new_runtime();
    let lifecycle = mount(&mut engine, compiled, &state);
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("row")
    };
    let callbacks: Vec<_> = children
        .iter()
        .map(|node| node.handler("click").unwrap().as_script().unwrap().clone())
        .collect();
    assert_ne!(
        callbacks[0], callbacks[1],
        "module identity participates in event equality"
    );
    for (callback, expected) in callbacks.iter().zip(["a:value", "b:value"]) {
        let _ = lifecycle
            .invoke_callback_transactional(&engine, callback, UiValue::String("value".into()))
            .unwrap();
        assert_eq!(
            state.borrow().component_state.get(&root_path(), "filter"),
            Some(&UiValue::String(expected.into()))
        );
    }
    assert!(format!("{:?}", lifecycle.root()).contains("a:sync"));
    assert!(format!("{:?}", lifecycle.root()).contains("b:sync"));
}
