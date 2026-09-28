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
    pub(crate) fn capture_retained(context: &NativeCallContext<'_>) -> Self {
        let mut stored = context.store_data();
        // A retained callback starts a new evaluator turn. Rhai's stored
        // context otherwise preserves the synchronous caller's stack depth,
        // so a task/event/timer chain would consume one call level per turn
        // even though those frames have already returned.
        stored.global.level = 0;
        let stored = Rc::new(stored);
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
    pub(crate) fn capture_entry_retained(context: &NativeCallContext<'_>) -> Self {
        let mut stored = context.store_data();
        stored.global.level = 0;
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use rhai::{Engine, FnPtr, NativeCallContext};

    use super::ScriptInvocationContext;

    #[test]
    fn retained_callback_chains_start_with_a_fresh_call_depth() {
        let retained = Rc::new(RefCell::new(None));
        let sink = Rc::clone(&retained);
        let mut engine = Engine::new();
        engine.set_max_call_levels(8);
        engine.register_fn(
            "retain",
            move |call: NativeCallContext<'_>, callback: FnPtr| {
                *sink.borrow_mut() =
                    Some((ScriptInvocationContext::capture_retained(&call), callback));
            },
        );
        engine
            .eval::<()>(
                r#"
                    fn next(step) {
                        retain(Fn("next"));
                        step + 1
                    }
                    retain(Fn("next"));
                "#,
            )
            .expect("seed retained callback");

        for step in 0..200_i64 {
            let (context, callback) = retained
                .borrow_mut()
                .take()
                .expect("the callback retains the next turn");
            let next = context
                .call::<i64>(&engine, &callback, (step,))
                .expect("sequential retained turns must not accumulate call depth");
            assert_eq!(next, step + 1);
        }
    }

    #[test]
    fn synchronous_recursion_still_obeys_the_engine_limit() {
        let mut engine = Engine::new();
        engine.set_max_call_levels(8);
        let error = engine
            .eval::<i64>(
                r"
                    fn recurse(value) { recurse(value + 1) }
                    recurse(0)
                ",
            )
            .expect_err("real synchronous recursion must remain bounded");

        assert!(
            matches!(*error, rhai::EvalAltResult::ErrorStackOverflow(_)),
            "unexpected recursion error: {error}"
        );
    }
}
