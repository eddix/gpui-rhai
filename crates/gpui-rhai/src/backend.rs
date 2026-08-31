use rhai::{AST, Engine, EvalAltResult, FuncArgs, Scope, Variant};

/// Static execution boundary for named functions in one compiled script.
///
/// The AST implementation is the production semantic oracle. Experimental
/// backends implement the same boundary only for characterization until their
/// parity and performance gates are independently satisfied.
pub(crate) trait ExecutionBackend {
    type Program;

    fn call_fn<T, Args>(
        engine: &Engine,
        program: &Self::Program,
        scope: &mut Scope<'_>,
        name: &str,
        args: Args,
    ) -> Result<T, Box<EvalAltResult>>
    where
        T: Variant + Clone,
        Args: FuncArgs;
}

pub(crate) struct AstInterpreter;

impl ExecutionBackend for AstInterpreter {
    type Program = AST;

    fn call_fn<T, Args>(
        engine: &Engine,
        program: &Self::Program,
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

#[cfg(all(feature = "grain-backend", test))]
pub(crate) struct GrainVm;

#[cfg(all(feature = "grain-backend", test))]
impl ExecutionBackend for GrainVm {
    type Program = rhai::grain::Program<'static>;

    fn call_fn<T, Args>(
        engine: &Engine,
        program: &Self::Program,
        scope: &mut Scope<'_>,
        name: &str,
        args: Args,
    ) -> Result<T, Box<EvalAltResult>>
    where
        T: Variant + Clone,
        Args: FuncArgs,
    {
        rhai::grain::Vm::new(engine).call_fn(scope, program, name, args)
    }
}

#[cfg(all(test, feature = "grain-backend"))]
mod tests {
    use std::collections::BTreeMap;

    use rhai::Dynamic;

    use super::*;
    use crate::{EmbeddedScriptSource, ModuleId, RestrictedModuleResolver};

    fn compare_call(
        engine: &Engine,
        ast: &AST,
        grain: &rhai::grain::Program<'static>,
        function: &str,
        args: Vec<Dynamic>,
    ) -> Result<(), String> {
        let ast_result = AstInterpreter::call_fn::<Dynamic, _>(
            engine,
            ast,
            &mut Scope::new(),
            function,
            args.clone(),
        );
        let grain_result =
            GrainVm::call_fn::<Dynamic, _>(engine, grain, &mut Scope::new(), function, args);
        match (ast_result, grain_result) {
            (Ok(ast), Ok(grain)) => {
                if ast.type_name() == grain.type_name()
                    && format!("{ast:?}") == format!("{grain:?}")
                {
                    Ok(())
                } else {
                    Err(format!("value mismatch: {ast:?} vs {grain:?}"))
                }
            }
            (Err(ast), Err(grain)) => {
                if ast.to_string() == grain.to_string() && ast.position() == grain.position() {
                    Ok(())
                } else {
                    Err(format!("error mismatch: {ast} vs {grain}"))
                }
            }
            (ast, grain) => Err(format!("result mismatch: {ast:?} vs {grain:?}")),
        }
    }

    #[test]
    fn grain_harness_checks_values_and_records_diagnostic_blocker() {
        let helper = EmbeddedScriptSource::new(BTreeMap::from([(
            ModuleId::parse("helpers/math").unwrap(),
            "fn offset(value) { value + 2 }".to_owned(),
        )]));
        let mut engine = Engine::new();
        engine.register_fn("native_double", |value: i64| value * 2);
        engine.set_module_resolver(RestrictedModuleResolver::from_source(&helper).unwrap());
        let ast = engine
            .compile_into_self_contained(
                &Scope::new(),
                r#"
                    import "helpers/math" as math;
                    fn add(base, value) { base + value }
                    fn native_and_import(value) {
                        math::offset(native_double(value))
                    }
                    fn curried(value) { Fn("add").curry(40).call(value) }
                    fn data(value) { #{ value: value, values: [value, value + 1] } }
                    fn fail() { 1 / 0 }
                "#,
            )
            .unwrap();
        let grain = rhai::grain::Compiler::new().compile(&ast);

        compare_call(
            &engine,
            &ast,
            &grain,
            "native_and_import",
            vec![Dynamic::from(5_i64)],
        )
        .unwrap();
        compare_call(&engine, &ast, &grain, "curried", vec![Dynamic::from(2_i64)]).unwrap();
        compare_call(&engine, &ast, &grain, "data", vec![Dynamic::from(7_i64)]).unwrap();
        let diagnostic_difference =
            compare_call(&engine, &ast, &grain, "fail", Vec::new()).unwrap_err();
        assert!(diagnostic_difference.contains("in call to function 'fail'"));
        let residual_count = grain.residual_count();
        assert!(residual_count <= grain.residual_nodes());
    }
}
