use std::fmt;
use std::rc::Rc;

use rhai::{FnPtr, FuncArgs, NativeCallContext};

pub(crate) const NAMED_FUNCTION: &str = "named_function";

/// Lexical provenance is independent of a callback's UI state owner. It travels
/// with a named function, including through maps, helper modules and curry.
#[derive(Clone, Debug)]
pub(crate) struct FunctionEnvironment(pub(crate) ScriptInvocationContext);

/// Rhai's builtin `Fn` is only a name; it does not retain its declaration
/// environment. Map that one token to an equivalent native constructor. Unlike
/// source rewriting this preserves strings, comments and diagnostic positions.
/// The trampoline also supports ordinary synchronous `call`/`curry` semantics.
#[allow(deprecated)]
pub(crate) fn register_named_functions(engine: &mut rhai::Engine) {
    engine.on_parse_token(|token, _, _| match token {
        rhai::Token::Identifier(name) | rhai::Token::Reserved(name) if name.as_str() == "Fn" => {
            rhai::Token::Identifier(Box::new(NAMED_FUNCTION.into()))
        }
        token => token,
    });
    rhai::FuncRegistration::new(NAMED_FUNCTION)
        .in_global_namespace()
        .with_purity(false)
        .with_volatility(true)
        .register_into_engine(
            engine,
            |call: NativeCallContext<'_>, name: rhai::ImmutableString| {
                let function = FnPtr::new(name.clone())?;
                let environment = ScriptInvocationContext::capture_retained(&call);
                let retained = environment.clone();
                let mut bound = FnPtr::from_fn(name, move |call, args| {
                    // Rhai puts an object's mutable `this` before curry for
                    // obj.call(fn, ...); ordinary call has the marker first.
                    let object_call = args
                        .get(1)
                        .is_some_and(|value| value.is::<FunctionEnvironment>())
                        && args
                            .first()
                            .is_some_and(|value| !value.is::<FunctionEnvironment>());
                    let (object, args) = if object_call {
                        let (object, args) = args.split_first_mut().expect("object and marker");
                        (Some(&mut **object), args)
                    } else {
                        (None, &mut args[..])
                    };
                    let values: Vec<_> = args
                        .iter_mut()
                        .filter(|value| !value.is::<FunctionEnvironment>())
                        .map(|value| value.take())
                        .collect();
                    retained.with_synchronous_context(&call, |context| {
                        function.call_raw(context, object, values)
                    })
                })?;
                bound.add_curry(rhai::Dynamic::from(FunctionEnvironment(environment)));
                Ok::<_, Box<rhai::EvalAltResult>>(bound)
            },
        );
}

pub(crate) fn take_function_environment(function: &mut FnPtr) -> Option<ScriptInvocationContext> {
    let mut environment = None;
    let curry: Vec<_> = function
        .iter_curry()
        .filter_map(|value| {
            if value.is::<FunctionEnvironment>() {
                environment = Some(value.clone_cast::<FunctionEnvironment>().0);
                None
            } else {
                Some(value.clone())
            }
        })
        .collect();
    if environment.is_some() {
        // Retained SDK invocations already re-enter this environment. Do not
        // retain a second trampoline or charge an extra script/Rust call per
        // event, component render or virtual row.
        *function = FnPtr::new(function.fn_name()).expect("a previously validated function name");
    }
    function.set_curry(curry);
    environment
}

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

    /// Compare the lexical environment of a retained function.
    /// Shared module identities survive repeated captures of the same program;
    /// evaluator counters and the stored-context allocation do not.
    #[allow(deprecated)]
    pub(crate) fn same_environment(&self, other: &Self) -> bool {
        self.stored.source == other.stored.source
            && self.stored.global.source == other.stored.global.source
            && self.stored.global.lib.len() == other.stored.global.lib.len()
            && self
                .stored
                .global
                .lib
                .iter()
                .zip(other.stored.global.lib.iter())
                .all(|(left, right)| std::ptr::eq(left.as_ref(), right.as_ref()))
            && self.stored.global.num_imports() == other.stored.global.num_imports()
            && self
                .stored
                .global
                .iter_imports_raw()
                .zip(other.stored.global.iter_imports_raw())
                .all(|((left_name, left), (right_name, right))| {
                    left_name == right_name && std::ptr::eq(left.as_ref(), right.as_ref())
                })
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

    /// Re-enter a retained module environment synchronously, not as a new
    /// event turn. Rhai 1.26 clones the evaluator: preserve the current depth
    /// and operation counter so render factories cannot reset safety limits.
    #[allow(deprecated)]
    pub(crate) fn call_synchronous<T: rhai::Variant + Clone>(
        &self,
        caller: &NativeCallContext<'_>,
        function: &FnPtr,
        args: impl FuncArgs,
    ) -> Result<T, Box<rhai::EvalAltResult>> {
        self.with_synchronous_context(caller, |context| {
            function.call_within_context(context, args)
        })
    }

    #[allow(deprecated)]
    fn with_synchronous_context<T>(
        &self,
        caller: &NativeCallContext<'_>,
        operation: impl FnOnce(&NativeCallContext<'_>) -> T,
    ) -> T {
        let mut stored = (*self.stored).clone();
        let current = caller.global_runtime_state();
        stored.global.level = current.level;
        stored.global.num_operations = current.num_operations;
        stored.global.num_modules_loaded = current.num_modules_loaded;
        stored.global.tag = current.tag.clone();
        let context = stored.create_context(caller.engine());
        operation(&context)
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
    #[allow(deprecated)]
    fn factory_environment_uses_module_identity_not_capture_counters() {
        let captures = Rc::new(RefCell::new(Vec::new()));
        let sink = captures.clone();
        let mut engine = Engine::new();
        engine.register_fn("capture", move |call: NativeCallContext<'_>| {
            sink.borrow_mut()
                .push(ScriptInvocationContext::capture_retained(&call));
        });
        engine.eval::<()>("capture(); capture();").unwrap();
        let captures = captures.borrow();
        let first = &captures[0];
        assert!(first.same_environment(&captures[1]));

        #[allow(deprecated)]
        let different_capture = |change: fn(&mut rhai::NativeCallContextStore)| {
            let mut stored = (*first.stored).clone();
            change(&mut stored);
            ScriptInvocationContext {
                stored: Rc::new(stored),
                operation_base: 999,
            }
        };
        let counters = different_capture(|stored| {
            stored.global.level = 60;
            stored.global.num_operations = 999;
            stored.global.num_modules_loaded = 42;
            stored.global.tag = "another invocation".into();
        });
        assert!(first.same_environment(&counters));
        let source = different_capture(|stored| stored.source = Some("other/module".into()));
        assert!(!first.same_environment(&source));
        let library = different_capture(|stored| {
            stored.global.lib.push(rhai::Module::new().into());
        });
        assert!(!first.same_environment(&library));
        let import = different_capture(|stored| {
            stored.global.push_import("palette", rhai::Module::new());
        });
        assert!(!first.same_environment(&import));
    }

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

    #[test]
    fn synchronous_retained_reentry_does_not_reset_call_depth() {
        let retained = Rc::new(RefCell::new(None));
        let sink = Rc::clone(&retained);
        let mut engine = Engine::new();
        engine.set_max_call_levels(8);
        engine.register_fn(
            "retain",
            move |call: NativeCallContext<'_>, function: FnPtr| {
                *sink.borrow_mut() =
                    Some((ScriptInvocationContext::capture_retained(&call), function));
            },
        );
        engine.register_fn(
            "bridge",
            move |call: NativeCallContext<'_>,
                  value: i64|
                  -> Result<i64, Box<rhai::EvalAltResult>> {
                let (context, function) = retained.borrow().clone().unwrap();
                context.call_synchronous(&call, &function, (value,))
            },
        );
        let error = engine
            .eval::<i64>(r#"fn recurse(value){bridge(value+1)} retain(Fn("recurse")); bridge(0)"#)
            .unwrap_err();
        assert!(
            error.to_string().to_lowercase().contains("stack"),
            "{error}"
        );
    }
}
