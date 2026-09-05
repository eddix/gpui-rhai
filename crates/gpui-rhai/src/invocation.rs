use std::fmt;
use std::rc::Rc;

use rhai::{FnPtr, FuncArgs, NativeCallContext};

/// The only adapter around Rhai's volatile stored native call context.
///
/// Imported module callbacks need the evaluator's global/module environment in
/// order to resolve private helpers after the original render returns. Keeping
/// this wrapper crate-private prevents volatile Rhai internals from spreading
/// through the retained runtime.
#[derive(Clone)]
pub(crate) struct ScriptInvocationContext {
    #[allow(deprecated)]
    stored: Rc<rhai::NativeCallContextStore>,
    operation_base: u64,
}

impl ScriptInvocationContext {
    #[allow(deprecated)]
    pub(crate) fn capture(context: &NativeCallContext<'_>) -> Self {
        let stored = Rc::new(context.store_data());
        Self {
            // Rhai clones this absolute counter into every later
            // `call_within_context`. Timings must subtract it to report the
            // delayed invocation rather than repeatedly charging the parent.
            operation_base: stored.global.num_operations,
            stored,
        }
    }

    /// Capture the entry-module frame from a formal component constructor
    /// call. Rhai keeps the entry library first and appends the current imported
    /// module while evaluating that constructor.
    #[allow(deprecated)]
    pub(crate) fn capture_entry(context: &NativeCallContext<'_>) -> Self {
        let mut stored = context.store_data();
        stored.global.lib.truncate(1);
        if let Some(source) = stored
            .global
            .lib
            .first()
            .and_then(|module| module.id())
            .map(str::to_owned)
        {
            stored.source = Some(source.clone());
            stored.global.source = Some(source.into());
        }
        Self {
            operation_base: stored.global.num_operations,
            stored: Rc::new(stored),
        }
    }

    pub(crate) const fn operation_base(&self) -> u64 {
        self.operation_base
    }

    #[allow(deprecated)]
    pub(crate) fn call<T: rhai::Variant + Clone>(
        &self,
        engine: &rhai::Engine,
        function: &FnPtr,
        args: impl FuncArgs,
    ) -> Result<T, Box<rhai::EvalAltResult>> {
        let context = self.stored.create_context(engine);
        function.call_within_context(&context, args)
    }
}

impl fmt::Debug for ScriptInvocationContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ScriptInvocationContext")
            .finish_non_exhaustive()
    }
}
