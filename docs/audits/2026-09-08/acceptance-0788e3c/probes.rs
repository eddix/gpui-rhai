use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
fn main() {
    // #42 and neighboring lexical constructs, judged against pinned Rhai.
    for (name, src) in [
        (
            "issue42",
            r#"let x = `a${if true { "" } else { `b${1}` }}`;"#,
        ),
        (
            "nested_no_inner_interpolation",
            r#"let x = `a${if true { "" } else { `b` }}`;"#,
        ),
        ("sequential_interpolation", r#"let x = `a${1}b${2}`;"#),
        ("three_levels", r#"let x = `a${`b${`c${1}`}`}`;"#),
        (
            "after_nested_real_import",
            r#"let x = `a${`b${1}`}`; import "components/real" as real;"#,
        ),
        (
            "import_in_expression",
            r#"let x = `a${{ import "components/real" as r; 1 }}`;"#,
        ),
        (
            "nested_comment",
            "/* a /* b */ import \"../fake\"; */ let x = 1;",
        ),
    ] {
        let plain = rhai::Engine::new().compile(src);
        println!(
            "lex {name}: rhai_ok={} extracted={:?}",
            plain.is_ok(),
            extract_imports(src)
        );
    }
    // Incremental component work must not re-charge the parent's historical operations.
    for parent in [0, 250_000] {
        let module = r#"
define_component(#{metadata:#{id:"components/probe", "export":"Probe",version:"0.1.1",runtime_api:#{min_inclusive:1,max_exclusive:2},dependencies:[],capabilities:#{}},schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{heavy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}},events:#{},slots:#{},parts:["root"]},render:Fn("render_Probe")});
fn Probe(props) { render_component("components/probe",props) }
fn render_Probe(ctx,props) { let n=0; if ctx.get_state("heavy") { for i in 0..100000 { n+=1; } } text(`${n}`) }
"#;
        let mut e = RuntimeEngine::new();
        let mut resolver = RestrictedModuleResolver::new();
        resolver.insert("components/probe", module).unwrap();
        e.set_module_resolver(resolver);
        let src = format!(
            "import \"components/probe\" as p; fn view(ctx) {{ let n=0; for i in 0..{parent} {{n+=1;}} p::Probe(#{{key:\"probe\"}}) }}"
        );
        let c = e
            .compile_self_contained_named("incremental-budget", &src)
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let mut l = ScriptLifecycle::new(
            c,
            runtime.clone(),
            ComponentInstancePath::root("App", "budget"),
            None,
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap();
        l.start(&mut e).unwrap();
        let owner = e.component_invocations().next().unwrap().path().clone();
        let _ = e.take_timings();
        runtime
            .borrow_mut()
            .set_component_state_from_host(&owner, "heavy", UiValue::Bool(true))
            .unwrap();
        let result = l.render_dirty(&mut e);
        println!(
            "incremental parent={parent} result={result:?} timings={:?}",
            e.take_timings()
                .iter()
                .map(|x| (&x.operation, x.operations))
                .collect::<Vec<_>>()
        );
    }
    // A stale closed stream with buffered data must actually leave the registry.
    let mut e = RuntimeEngine::new();
    let a = e
        .compile("fn view() {text(\"a\")} fn done(ctx,x) {()}")
        .unwrap();
    let ca = e.callback(&a, "done").unwrap();
    let b = e
        .compile("fn view() {text(\"b\")} fn done(ctx,x) {()}")
        .unwrap();
    let mut subs = SubscriptionRegistry::new();
    let (_, emitter) = subs.subscribe(SubscriptionRegistration::new(
        "old",
        AsyncScope::App,
        a.generation(),
        ca.clone(),
        ca,
        ValueSchema::integer(),
    ));
    emitter.emit(UiValue::Integer(1)).unwrap();
    println!(
        "stale_sub first={} second={} active={} reason={:?}",
        subs.drain(b.generation()).len(),
        subs.drain(b.generation()).len(),
        subs.active_count(),
        emitter.close_reason()
    );
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let path = ComponentInstancePath::root("SecretInput", "audit");
    let secret = "AUDIT_ONLY_SYNTHETIC_TOKEN";
    let schema = ComponentStateSchema::new(BTreeMap::from([(
        "token".into(),
        StateField::new(ValueSchema::string(), UiValue::String(secret.into())).sensitive(true),
    )]))
    .unwrap();
    runtime
        .borrow_mut()
        .component_state
        .mount_instance(path.clone(), &schema)
        .unwrap();
    let ctx = UiContext::new(
        runtime.clone(),
        path,
        None,
        ExecutionPhase::Event,
        BTreeMap::from([(
            "change".into(),
            EventSchema {
                payload: ValueSchema::string(),
            },
        )]),
    );
    ctx.emit("change", ctx.get_state("token").unwrap().into_dynamic())
        .unwrap();
    println!(
        "sensitive_emit state={:?} trace={:?}",
        runtime.borrow().component_state.inspect()[0].fields["token"].value,
        runtime
            .borrow()
            .traces
            .snapshot()
            .last()
            .map(|trace| (&trace.payload, trace.sensitive))
    );
}
