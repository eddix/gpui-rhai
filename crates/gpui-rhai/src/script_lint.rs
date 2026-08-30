use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use rhai::{AST, ASTNode, Engine, Expr, FnAccess, OptimizationLevel, Stmt};
use serde::Deserialize;
use thiserror::Error;

use crate::ModuleId;

#[derive(Clone, Debug, Eq, PartialEq)]
/// One type-agnostic Rhai function-name or arity diagnostic.
pub struct KnownCallDiagnostic {
    /// Diagnostic source name supplied by the Host.
    pub source: String,
    /// One-based source line, when Rhai retained it in the AST.
    pub line: Option<usize>,
    /// One-based source column, when Rhai retained it in the AST.
    pub column: Option<usize>,
    /// Qualified call namespace, or `None` for direct and method calls.
    pub namespace: Option<String>,
    /// Called function or method name.
    pub function: String,
    /// Number of explicit arguments at the call site.
    pub arity: usize,
    /// Known valid explicit arities; empty means the name is unknown.
    pub expected_arities: Vec<usize>,
}

impl fmt::Display for KnownCallDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.source)?;
        if let Some(line) = self.line {
            write!(formatter, ":{line}")?;
            if let Some(column) = self.column {
                write!(formatter, ":{column}")?;
            }
        }
        let qualified = self.namespace.as_ref().map_or_else(
            || self.function.clone(),
            |namespace| format!("{namespace}::{}", self.function),
        );
        if self.expected_arities.is_empty() {
            write!(
                formatter,
                ": unknown function call `{qualified}/{}`",
                self.arity
            )
        } else {
            let expected = self
                .expected_arities
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            write!(
                formatter,
                ": call `{qualified}/{}` has the wrong arity; expected {expected}",
                self.arity
            )
        }
    }
}

#[derive(Debug, Error)]
/// Failure to construct or parse inputs for known-call validation.
pub enum KnownCallLintError {
    #[error("failed to read registered Rhai function metadata: {0}")]
    Metadata(#[from] serde_json::Error),
    #[error("failed to parse `{source_name}` for known-call validation: {message}")]
    Parse {
        source_name: String,
        message: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
enum MetadataFunctionType {
    Native,
    Script,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
enum MetadataNamespace {
    Internal,
    Global,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetadataFunction {
    name: String,
    namespace: MetadataNamespace,
    #[serde(rename = "type")]
    function_type: MetadataFunctionType,
    num_params: usize,
}

#[derive(Debug, Default, Deserialize)]
struct MetadataModule {
    #[serde(default)]
    modules: BTreeMap<String, Self>,
    #[serde(default)]
    functions: Vec<MetadataFunction>,
}

#[derive(Clone, Debug, Default)]
struct FunctionSet {
    direct: BTreeMap<String, BTreeSet<usize>>,
    method: BTreeMap<String, BTreeSet<usize>>,
}

impl FunctionSet {
    fn insert_direct(&mut self, name: impl Into<String>, arity: usize) {
        self.direct.entry(name.into()).or_default().insert(arity);
    }

    fn insert_method(&mut self, name: impl Into<String>, arity: usize) {
        self.method.entry(name.into()).or_default().insert(arity);
    }
}

#[derive(Clone, Debug, Default)]
struct KnownCallCatalog {
    global: FunctionSet,
    modules: BTreeMap<String, FunctionSet>,
}

impl KnownCallCatalog {
    fn from_engine(engine: &Engine) -> Result<Self, KnownCallLintError> {
        let metadata: MetadataModule =
            serde_json::from_str(&engine.gen_fn_metadata_to_json(true)?)?;
        let mut catalog = Self::default();
        catalog.ingest_metadata_module("", &metadata);
        catalog.insert_language_intrinsics();
        Ok(catalog)
    }

    fn insert_language_intrinsics(&mut self) {
        // Rhai 1.26 implements these in evaluator control flow and emits them
        // in builtin-functions.d.rhai, so they are intentionally absent from
        // Engine function metadata. Keep this list characterized with the
        // pinned Rhai version.
        for (name, arities) in [
            ("print", &[1][..]),
            ("debug", &[1][..]),
            ("type_of", &[1][..]),
            ("Fn", &[1][..]),
            ("call", &[1][..]),
            ("curry", &[2][..]),
            ("is_def_fn", &[2, 3][..]),
            ("is_def_var", &[1][..]),
            ("is_shared", &[1][..]),
            ("eval", &[1][..]),
        ] {
            for arity in arities {
                self.global.insert_direct(name, *arity);
            }
        }
        for (name, arity) in [
            ("print", 0),
            ("debug", 0),
            ("type_of", 0),
            ("call", 0),
            ("curry", 1),
            ("is_shared", 0),
        ] {
            self.global.insert_method(name, arity);
        }
    }

    fn ingest_metadata_module(&mut self, path: &str, module: &MetadataModule) {
        for function in &module.functions {
            if path.is_empty() {
                if function.namespace == MetadataNamespace::Global {
                    self.global
                        .insert_direct(function.name.clone(), function.num_params);
                }
                match function.function_type {
                    MetadataFunctionType::Native if function.num_params > 0 => self
                        .global
                        .insert_method(function.name.clone(), function.num_params - 1),
                    MetadataFunctionType::Script => self
                        .global
                        .insert_method(function.name.clone(), function.num_params),
                    MetadataFunctionType::Native => {}
                }
            } else {
                self.modules
                    .entry(path.to_owned())
                    .or_default()
                    .insert_direct(function.name.clone(), function.num_params);
            }
        }
        for (name, child) in &module.modules {
            let child_path = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}::{name}")
            };
            self.ingest_metadata_module(&child_path, child);
        }
    }
}

struct ParsedSource {
    name: String,
    module: Option<String>,
    ast: AST,
}

pub(crate) fn lint_known_calls(
    engine: &mut Engine,
    entry_name: &str,
    entry_source: &str,
    modules: &BTreeMap<ModuleId, String>,
) -> Result<Vec<KnownCallDiagnostic>, KnownCallLintError> {
    let catalog = KnownCallCatalog::from_engine(engine)?;
    let previous_optimization = engine.optimization_level();
    engine.set_optimization_level(OptimizationLevel::None);
    let parsed = parse_sources(engine, entry_name, entry_source, modules);
    engine.set_optimization_level(previous_optimization);
    let parsed = parsed?;

    let mut module_functions = BTreeMap::<String, FunctionSet>::new();
    for source in &parsed {
        if let Some(module) = &source.module {
            let functions = module_functions.entry(module.clone()).or_default();
            for function in source.ast.iter_functions() {
                if function.access == FnAccess::Public {
                    functions.insert_direct(function.name, function.params.len());
                }
            }
        }
    }

    let mut diagnostics = Vec::new();
    for source in &parsed {
        lint_source(source, &catalog, &module_functions, &mut diagnostics);
    }
    diagnostics.sort_by(|left, right| {
        (
            &left.source,
            left.line,
            left.column,
            &left.namespace,
            &left.function,
            left.arity,
        )
            .cmp(&(
                &right.source,
                right.line,
                right.column,
                &right.namespace,
                &right.function,
                right.arity,
            ))
    });
    diagnostics.dedup();
    Ok(diagnostics)
}

fn parse_sources(
    engine: &Engine,
    entry_name: &str,
    entry_source: &str,
    modules: &BTreeMap<ModuleId, String>,
) -> Result<Vec<ParsedSource>, KnownCallLintError> {
    let mut parsed = Vec::with_capacity(modules.len() + 1);
    parsed.push(parse_source(engine, entry_name, None, entry_source)?);
    for (module, source) in modules {
        parsed.push(parse_source(
            engine,
            module.as_str(),
            Some(module.as_str()),
            source,
        )?);
    }
    Ok(parsed)
}

fn parse_source(
    engine: &Engine,
    name: &str,
    module: Option<&str>,
    source: &str,
) -> Result<ParsedSource, KnownCallLintError> {
    let mut ast = engine
        .compile(source)
        .map_err(|error| KnownCallLintError::Parse {
            source_name: name.to_owned(),
            message: error.to_string(),
        })?;
    ast.set_source(name);
    Ok(ParsedSource {
        name: name.to_owned(),
        module: module.map(ToOwned::to_owned),
        ast,
    })
}

fn lint_source(
    source: &ParsedSource,
    catalog: &KnownCallCatalog,
    module_functions: &BTreeMap<String, FunctionSet>,
    diagnostics: &mut Vec<KnownCallDiagnostic>,
) {
    let mut local = FunctionSet::default();
    for function in source.ast.iter_functions() {
        local.insert_direct(function.name, function.params.len());
        local.insert_method(function.name, function.params.len());
    }
    let imports = collect_imports(&source.ast);
    source.ast.walk(&mut |path| {
        match path.last() {
            Some(
                ASTNode::Expr(Expr::FnCall(call, position))
                | ASTNode::Stmt(Stmt::FnCall(call, position)),
            ) if !call.is_operator_call() => {
                let namespace = (!call.namespace.is_empty()).then(|| call.namespace.to_string());
                if namespace.is_none()
                    && language_variadic_accepts(&call.name, false, call.args.len())
                {
                    return true;
                }
                let expected = direct_arities(
                    catalog,
                    module_functions,
                    &local,
                    &imports,
                    namespace.as_deref(),
                    &call.name,
                );
                push_if_invalid(
                    diagnostics,
                    source,
                    *position,
                    namespace,
                    &call.name,
                    call.args.len(),
                    expected,
                );
            }
            Some(ASTNode::Expr(Expr::MethodCall(call, position))) => {
                if language_variadic_accepts(&call.name, true, call.args.len()) {
                    return true;
                }
                let expected = merged_arities(
                    catalog.global.method.get(call.name.as_str()),
                    local.method.get(call.name.as_str()),
                );
                push_if_invalid(
                    diagnostics,
                    source,
                    *position,
                    None,
                    &call.name,
                    call.args.len(),
                    expected,
                );
            }
            _ => {}
        }
        true
    });
}

fn language_variadic_accepts(function: &str, method: bool, arity: usize) -> bool {
    match (function, method) {
        ("call", true) => true,
        ("curry", false) => arity >= 2,
        ("call", false) | ("curry", true) => arity >= 1,
        _ => false,
    }
}

fn collect_imports(ast: &AST) -> BTreeMap<String, String> {
    let mut imports = BTreeMap::new();
    ast.walk(&mut |path| {
        if let Some(ASTNode::Stmt(Stmt::Import(import, ..))) = path.last()
            && let (Expr::StringConstant(module, ..), alias) = &**import
            && !alias.is_empty()
        {
            imports.insert(alias.as_str().to_owned(), module.to_string());
        }
        true
    });
    imports
}

fn direct_arities(
    catalog: &KnownCallCatalog,
    module_functions: &BTreeMap<String, FunctionSet>,
    local: &FunctionSet,
    imports: &BTreeMap<String, String>,
    namespace: Option<&str>,
    function: &str,
) -> BTreeSet<usize> {
    let Some(namespace) = namespace else {
        return merged_arities(
            catalog.global.direct.get(function),
            local.direct.get(function),
        );
    };
    if namespace == "global" {
        return catalog
            .global
            .direct
            .get(function)
            .cloned()
            .unwrap_or_default();
    }
    let mut segments = namespace.split("::");
    let root = segments.next().unwrap_or_default();
    if let Some(module) = imports.get(root) {
        let suffix = segments.collect::<Vec<_>>().join("::");
        let module = if suffix.is_empty() {
            module.clone()
        } else {
            format!("{module}::{suffix}")
        };
        return module_functions
            .get(&module)
            .and_then(|functions| functions.direct.get(function))
            .cloned()
            .unwrap_or_default();
    }
    catalog
        .modules
        .get(namespace)
        .and_then(|functions| functions.direct.get(function))
        .cloned()
        .unwrap_or_default()
}

fn merged_arities(
    first: Option<&BTreeSet<usize>>,
    second: Option<&BTreeSet<usize>>,
) -> BTreeSet<usize> {
    first.into_iter().chain(second).flatten().copied().collect()
}

fn push_if_invalid(
    diagnostics: &mut Vec<KnownCallDiagnostic>,
    source: &ParsedSource,
    position: rhai::Position,
    namespace: Option<String>,
    function: &str,
    arity: usize,
    expected: BTreeSet<usize>,
) {
    if expected.contains(&arity) {
        return;
    }
    diagnostics.push(KnownCallDiagnostic {
        source: source.name.clone(),
        line: position.line(),
        column: position.position(),
        namespace,
        function: function.to_owned(),
        arity,
        expected_arities: expected.into_iter().collect(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeEngine;

    #[test]
    fn known_call_lint_accepts_local_module_core_and_standard_calls() {
        let mut runtime = RuntimeEngine::new();
        let modules = BTreeMap::from([(
            ModuleId::parse("components/probe").unwrap(),
            "fn Probe(value) { text(value) }".to_owned(),
        )]);
        let diagnostics = runtime
            .lint_known_calls(
                "ui/main.rhai",
                r#"
                    import "components/probe" as probe;
                    fn helper(value) { value.to_upper() }
                    fn view(ctx) { probe::Probe(helper("ready")) }
                "#,
                &modules,
            )
            .unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn known_call_lint_checks_unexecuted_branches_and_arities() {
        let mut runtime = RuntimeEngine::new();
        let diagnostics = runtime
            .lint_known_calls(
                "ui/main.rhai",
                r#"
                    fn view(ctx) {
                        if false {
                            texxt("unreachable");
                            text();
                        }
                        text("ok")
                    }
                "#,
                &BTreeMap::new(),
            )
            .unwrap();
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].function, "texxt");
        assert!(diagnostics[0].expected_arities.is_empty());
        assert_eq!(diagnostics[1].function, "text");
        assert_eq!(diagnostics[1].expected_arities, vec![1]);
    }

    #[test]
    fn known_call_lint_normalizes_native_method_receiver_arity() {
        let mut runtime = RuntimeEngine::new();
        let diagnostics = runtime
            .lint_known_calls(
                "ui/main.rhai",
                r#"fn view(ctx) { text("probe").with_key() }"#,
                &BTreeMap::new(),
            )
            .unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].function, "with_key");
        assert_eq!(diagnostics[0].expected_arities, vec![1]);
    }

    #[test]
    fn known_call_lint_checks_qualified_installed_modules() {
        let mut runtime = RuntimeEngine::new();
        let modules = BTreeMap::from([(
            ModuleId::parse("components/probe").unwrap(),
            "fn Probe(value) { text(value) }".to_owned(),
        )]);
        let diagnostics = runtime
            .lint_known_calls(
                "ui/main.rhai",
                r#"
                    import "components/probe" as probe;
                    fn view(ctx) { probe::Probe() }
                "#,
                &modules,
            )
            .unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].namespace.as_deref(), Some("probe"));
        assert_eq!(diagnostics[0].expected_arities, vec![1]);
    }

    #[test]
    fn known_call_lint_characterizes_rhai_language_intrinsics() {
        let mut runtime = RuntimeEngine::new();
        let diagnostics = runtime
            .lint_known_calls(
                "ui/main.rhai",
                r#"
                    fn callback(first, second) { first + second }
                    fn view(ctx) {
                        let callback = Fn("callback");
                        callback.call(1, 2);
                        callback.curry(1).call(2);
                        text(type_of(is_def_fn("callback", 2)))
                    }
                "#,
                &BTreeMap::new(),
            )
            .unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn known_call_lint_restores_engine_optimization_after_parse_failure() {
        let mut runtime = RuntimeEngine::new();
        let previous = runtime.engine().optimization_level();
        assert!(
            runtime
                .lint_known_calls("ui/main.rhai", "fn view(", &BTreeMap::new())
                .is_err()
        );
        assert_eq!(runtime.engine().optimization_level(), previous);
    }
}
