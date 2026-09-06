//! Opt-in gpui-rhai adapter for the experimental Grain backend.

use std::rc::Rc;

use gpui_rhai::{
    ExperimentalCompiledScript, ExperimentalOperationObserver, ExperimentalScriptBackend,
};
use rhai::grain::{CallbackBindings, SharedFunctionAccelerator, Vm};
use rhai::{AST, CallFnOptions, Dynamic, Engine, EvalAltResult, Scope, Shared};

/// Compiles gpui-rhai generations to Grain and optionally attaches a function
/// accelerator. This crate remains unpublished while it requires a Rhai fork.
#[derive(Clone)]
pub struct GrainScriptBackend {
    accelerator: Option<SharedFunctionAccelerator>,
}

impl GrainScriptBackend {
    /// Build a Grain backend with an optional profiler or adaptive tier.
    #[must_use]
    pub const fn new(accelerator: Option<SharedFunctionAccelerator>) -> Self {
        Self { accelerator }
    }
}

struct CompiledGrainScript {
    bindings: CallbackBindings,
    accelerator: Option<SharedFunctionAccelerator>,
}

impl ExperimentalScriptBackend for GrainScriptBackend {
    fn configure_engine(&self, engine: &mut Engine, observer: ExperimentalOperationObserver) {
        engine.on_progress_batch(move |operations| {
            observer.record(operations);
            None
        });
    }

    fn compile(&self, _engine: &Engine, ast: &AST) -> Rc<dyn ExperimentalCompiledScript> {
        Rc::new(CompiledGrainScript {
            bindings: CallbackBindings::new(Shared::new(rhai::grain::Compiler::new().compile(ast))),
            accelerator: self.accelerator.clone(),
        })
    }
}

impl ExperimentalCompiledScript for CompiledGrainScript {
    fn call_fn(
        &self,
        engine: &Engine,
        name: &str,
        args: Vec<Dynamic>,
    ) -> Result<Dynamic, Box<EvalAltResult>> {
        let mut vm = Vm::new(engine);
        if let Some(accelerator) = &self.accelerator {
            vm = vm.with_function_accelerator(accelerator.clone());
        }
        vm.call_fn_with_bindings(
            CallFnOptions::new(),
            &mut Scope::new(),
            &self.bindings,
            name,
            args,
        )
    }
}
