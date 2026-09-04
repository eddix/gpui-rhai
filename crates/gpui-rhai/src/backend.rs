use rhai::{AST, Engine, EvalAltResult, FuncArgs, Scope, Variant};

#[cfg(feature = "experimental-backend")]
use rhai::Dynamic;
#[cfg(feature = "experimental-backend")]
use std::rc::Rc;
#[cfg(feature = "experimental-backend")]
use std::sync::Arc;
#[cfg(feature = "experimental-backend")]
use std::sync::atomic::{AtomicU64, Ordering};

/// Static execution boundary for the production AST interpreter.
pub(crate) struct AstInterpreter;

impl AstInterpreter {
    pub(crate) fn call_fn<T, Args>(
        engine: &Engine,
        program: &AST,
        scope: &mut Scope<'_>,
        name: &str,
        args: Args,
    ) -> Result<T, Box<EvalAltResult>>
    where
        T: Variant + Clone,
        Args: FuncArgs,
    {
        engine.call_fn(scope, program, name, args)
    }
}

/// Experimental, fork-independent factory for an alternate script executor.
///
/// This feature-gated API uses only types available from official Rhai. An
/// implementation may require a patched Rhai in the final application, but the
/// published gpui-rhai package does not name or depend on those extra APIs.
#[cfg(feature = "experimental-backend")]
pub trait ExperimentalScriptBackend {
    /// Configure Rhai before any script is compiled with this backend.
    ///
    /// `operations` is the observer used by gpui-rhai timings. A backend
    /// that replaces Rhai's progress callback must continue updating it.
    fn configure_engine(&self, engine: &mut Engine, operations: ExperimentalOperationObserver);

    /// Compile one immutable script generation into backend-owned state.
    fn compile(&self, engine: &Engine, ast: &AST) -> Rc<dyn ExperimentalCompiledScript>;
}

/// Opaque operation observer supplied to an experimental backend.
///
/// Backends that replace Rhai's progress callback record the latest absolute
/// operation count here so ordinary gpui-rhai timing remains valid.
#[cfg(feature = "experimental-backend")]
#[derive(Clone)]
pub struct ExperimentalOperationObserver(pub(crate) Arc<AtomicU64>);

#[cfg(feature = "experimental-backend")]
impl ExperimentalOperationObserver {
    /// Record the latest absolute Rhai operation count.
    pub fn record(&self, operations: u64) {
        self.0.store(operations, Ordering::Relaxed);
    }
}

/// One generation of an experimental script executor.
#[cfg(feature = "experimental-backend")]
pub trait ExperimentalCompiledScript {
    /// Invoke a named function, consuming the supplied dynamic arguments.
    ///
    /// The implementation is responsible for ordinary fallback and must not
    /// replay an invocation after it has started performing effects.
    ///
    /// # Errors
    ///
    /// Returns the backend's script/runtime error with its original Rhai
    /// position and function boundary.
    fn call_fn(
        &self,
        engine: &Engine,
        name: &str,
        args: Vec<Dynamic>,
    ) -> Result<Dynamic, Box<EvalAltResult>>;
}
