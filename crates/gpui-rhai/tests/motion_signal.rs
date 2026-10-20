use std::collections::BTreeSet;

use gpui_rhai::{
    ComponentInstancePath, MotionProgressBinding, MotionProgressDriver, MotionProperty,
    MotionSource, MotionTransition, NativeSignal, SignalId, SignalKind,
};

fn signal(kind: SignalKind) -> NativeSignal {
    NativeSignal::new(
        SignalId::new(ComponentInstancePath::root("Test", "test"), "state", kind).unwrap(),
    )
}

fn source() -> MotionSource {
    MotionSource::Transition(MotionTransition::new(
        MotionProperty::Height,
        24.0,
        40.0,
        120,
    ))
}

fn render_context(phase: gpui_rhai::ExecutionPhase) -> gpui_rhai::UiContext {
    gpui_rhai::UiContext::new(
        std::rc::Rc::new(std::cell::RefCell::new(gpui_rhai::UiRuntimeState::new())),
        ComponentInstancePath::root("View", "factory"),
        None,
        phase,
        std::collections::BTreeMap::new(),
    )
}

#[test]
fn node_factory_rejects_unbound_functions_wrong_results_and_nonrender_invocation() {
    for (body, message) in [
        (r#"ctx.render_node(Fn("factory"),())"#, "ctx.node_factory"),
        (r"ctx.render_node(42,())", "requires a named Rhai Fn"),
        (
            r#"ctx.render_node(ctx.node_factory(Fn("factory")),())"#,
            "Output type incorrect",
        ),
    ] {
        let mut engine = gpui_rhai::RuntimeEngine::new();
        let compiled = engine
            .compile(&format!(
                "fn factory(ctx,detail){{42}} fn view(ctx){{{body}}}"
            ))
            .unwrap();
        let error = engine
            .render_with_context(&compiled, render_context(gpui_rhai::ExecutionPhase::Render))
            .unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
    }
    let mut engine = gpui_rhai::RuntimeEngine::new();
    let compiled = engine
        .compile(
            r#"fn factory(ctx,p){text("x")}
fn clicked(ctx){ctx.node_factory(Fn("factory"))} fn view(ctx){text("test")}"#,
        )
        .unwrap();
    let callback = engine.callback(&compiled, "clicked").unwrap();
    engine
        .render_with_context(&compiled, render_context(gpui_rhai::ExecutionPhase::Render))
        .unwrap();
    let error = engine
        .invoke_callback(
            &compiled,
            &callback,
            (render_context(gpui_rhai::ExecutionPhase::Event),),
        )
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("only available during component rendering"),
        "{error}"
    );
}

#[test]
fn native_trigger_and_factory_api_metadata_has_matching_documentation() {
    let engine = gpui_rhai::RuntimeEngine::new();
    let api = gpui_rhai::script_docs::ScriptApi::from_engine(engine.engine()).unwrap();
    let functions = api
        .functions
        .iter()
        .filter(|f| {
            matches!(
                f.name.as_str(),
                "motion_signal" | "render_node" | "node_factory"
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(functions.len(), 3);
    assert!(
        functions.iter().all(|f| f.doc.is_some()),
        "{:#?}",
        functions.iter().map(|f| &f.signature).collect::<Vec<_>>()
    );
}

#[test]
fn many_small_node_factories_share_one_render_operation_budget() {
    fn program(count: usize) -> String {
        format!(
            r#"fn decoration(ctx,n){{let sum=0;for i in 0..80{{sum+=i+n;}}text(`${{sum}}`)}}
fn view(ctx){{let children=[];for n in 0..{count}{{children.push(ctx.render_node(ctx.node_factory(Fn("decoration")),n));}}column(children)}}"#
        )
    }
    let mut engine = gpui_rhai::RuntimeEngine::new();
    engine.set_operation_limit(2_000);
    let context = || {
        gpui_rhai::UiContext::new(
            std::rc::Rc::new(std::cell::RefCell::new(gpui_rhai::UiRuntimeState::new())),
            ComponentInstancePath::root("View", "budget"),
            None,
            gpui_rhai::ExecutionPhase::Render,
            std::collections::BTreeMap::new(),
        )
    };
    let compiled = engine.compile(&program(1)).unwrap();
    engine
        .render_with_context(&compiled, context())
        .expect("one small factory is below the quota");
    let compiled = engine.compile(&program(10)).unwrap();
    let error = engine
        .render_with_context(&compiled, context())
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("operation budget exceeded"),
        "{error:?}"
    );
}

#[test]
fn signal_motion_requires_a_string_predicate_and_bounded_nonempty_state_set() {
    for kind in [
        SignalKind::Bool,
        SignalKind::Float,
        SignalKind::OptionalFloat,
    ] {
        assert!(
            MotionProgressBinding::from_signal(
                signal(kind),
                BTreeSet::from(["drag".to_owned()]),
                source()
            )
            .is_err()
        );
    }
    for states in [
        BTreeSet::new(),
        BTreeSet::from([String::new()]),
        BTreeSet::from(["x".repeat(129)]),
        (0..65).map(|n| format!("state{n}")).collect(),
    ] {
        assert!(
            MotionProgressBinding::from_signal(signal(SignalKind::String), states, source())
                .is_err()
        );
    }
    assert!(MotionProgressBinding::new(MotionProgressDriver::Signal, source()).is_err());
    let binding = MotionProgressBinding::from_signal(
        signal(SignalKind::String),
        BTreeSet::from(["drag".to_owned(), "hover".to_owned()]),
        source(),
    )
    .unwrap();
    assert_eq!(binding.property(), MotionProperty::Height);
    assert_eq!(binding.driver, MotionProgressDriver::Signal);
}
