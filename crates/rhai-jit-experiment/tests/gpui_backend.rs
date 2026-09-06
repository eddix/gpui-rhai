use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui_rhai::{
    EmbeddedScriptSource, ModuleId, RestrictedModuleResolver, RuntimeEngine, UiNodeKind,
};
use rhai::grain::{AcceleratedCall, FunctionAccelerator, FunctionCall};
use rhai_jit_experiment::gpui::GrainScriptBackend;

struct RecordingAccelerator {
    calls: Rc<Cell<u64>>,
}

impl FunctionAccelerator for RecordingAccelerator {
    fn call(&mut self, call: FunctionCall<'_, '_>) -> AcceleratedCall {
        if call.name == "helper" {
            self.calls.set(self.calls.get().saturating_add(1));
        }
        AcceleratedCall::Declined
    }
}

#[test]
fn gpui_adapter_routes_nested_calls_through_grain() {
    let calls = Rc::new(Cell::new(0));
    let accelerator = rhai::grain::shared_function_accelerator(RecordingAccelerator {
        calls: Rc::clone(&calls),
    });
    let mut engine = RuntimeEngine::new();
    engine
        .set_experimental_script_backend(Some(Rc::new(GrainScriptBackend::new(Some(accelerator)))));
    let compiled = engine
        .compile_self_contained_named(
            "backend/adapter.rhai",
            "fn helper(value) { value + 1 } fn view() { text(`${helper(41)}`) }",
        )
        .expect("script must compile");

    let node = engine.render(&compiled).expect("script must render");

    assert!(matches!(node.kind(), UiNodeKind::Text { text } if text == "42") && calls.get() == 1);
}

#[test]
#[cfg(target_arch = "aarch64")]
fn gpui_adapter_executes_an_imported_component_render_natively() {
    use std::time::Duration;

    use rhai_jit_experiment::adaptive::{AdaptiveFunctionTier, AdaptiveTierConfig};

    const COMPONENT: &str = r#"
/* gpui-rhai
{
  "id": "components/jit_probe",
  "export": "JitProbe",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
define_component(#{
    metadata: #{
        id: "components/jit_probe", "export": "JitProbe", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{}
    },
    schema: #{
        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: []
    },
    render: Fn("render_JitProbe")
});
fn plus_one(value) { value + 1 }
fn JitProbe(props) { render_component("components/jit_probe", props) }
fn render_JitProbe(ctx, props) { text(`${plus_one(41)}`) }
"#;
    const ENTRY: &str = r#"
import "components/jit_probe" as probe;
fn view() { probe::JitProbe(#{ key: "probe" }) }
"#;

    let tier = AdaptiveFunctionTier::new(AdaptiveTierConfig {
        min_calls: 1,
        min_self_grain_time: Duration::ZERO,
        check_profitability: false,
        ..Default::default()
    });
    let mut engine = RuntimeEngine::new();
    engine.set_experimental_script_backend(Some(Rc::new(GrainScriptBackend::new(Some(
        tier.accelerator(),
    )))));
    engine.set_module_resolver(
        RestrictedModuleResolver::from_source(&EmbeddedScriptSource::new(BTreeMap::from([(
            ModuleId::parse("components/jit_probe").expect("valid module id"),
            COMPONENT.to_owned(),
        )])))
        .expect("resolver must build"),
    );
    let compiled = engine
        .compile_self_contained_named("ui/main.rhai", ENTRY)
        .expect("entry must compile");

    let first = engine.render(&compiled).expect("first render must run");
    let second = engine.render(&compiled).expect("second render must run");
    let stats = tier.stats();

    assert!(
        matches!(first.kind(), UiNodeKind::Text { text } if text == "42")
            && matches!(second.kind(), UiNodeKind::Text { text } if text == "42")
            && stats
                .iter()
                .any(|entry| entry.name == "render_JitProbe" && entry.jit_calls > 0),
        "{stats:#?}"
    );
}
