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

#[derive(Clone, Debug, Eq, PartialEq)]
/// An entry-script function that takes over calls meant for a native function.
///
/// Rhai looks for a script function before a native one. A method call
/// `x.f(a, b)` looks for a script `fn f(a, b)` (the receiver is not a
/// parameter), and a direct call `f(a, b)` for `fn f(a, b)`. Entry-script
/// functions are visible inside imported modules, so `fn index_of(values,
/// key)` in an application captured `text.index_of(character, start)` in the
/// official Command component.
pub struct ShadowedBuiltin {
    /// Diagnostic source name supplied by the Host.
    pub source: String,
    /// One-based line of the `fn` declaration, when found in the source text.
    pub line: Option<usize>,
    /// The shadowing function name.
    pub function: String,
    /// Its parameter count.
    pub arity: usize,
    /// A native method takes `arity` explicit arguments after its receiver.
    pub captures_method_calls: bool,
    /// A native function takes exactly `arity` arguments.
    pub captures_direct_calls: bool,
}

impl fmt::Display for ShadowedBuiltin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.source)?;
        if let Some(line) = self.line {
            write!(formatter, ":{line}")?;
        }
        let arguments = (0..self.arity)
            .map(|index| char::from(b'a' + u8::try_from(index % 26).unwrap_or(0)).to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let name = &self.function;
        let captured = match (self.captures_method_calls, self.captures_direct_calls) {
            (true, true) => format!(
                "method calls `x.{name}({arguments})` and direct calls `{name}({arguments})`"
            ),
            (true, false) => format!("method calls `x.{name}({arguments})`"),
            _ => format!("direct calls `{name}({arguments})`"),
        };
        write!(
            formatter,
            ": `fn {name}` takes over built-in {captured}, also inside imported components; rename it"
        )
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
/// A function the Host registers on its own engine, which known-call
/// validation cannot see in this one: `name/arity` called directly or as a
/// method, or `module::name/arity` in a static module (a custom primitive
/// constructor is `namespace::Export/1`).
pub struct HostFunction {
    /// Static module path for `module::name` calls, or `None` for a global
    /// function.
    pub namespace: Option<String>,
    /// Function name.
    pub name: String,
    /// Number of parameters.
    pub arity: usize,
}

impl HostFunction {
    /// Parse `name/arity` or `module::name/arity`, such as `open_file/1` or
    /// `omb::status/0`.
    ///
    /// # Errors
    ///
    /// Returns [`HostFunctionError`] naming the declaration when the arity
    /// is missing or not a number, a segment is not a Rhai identifier, or the
    /// module is `global`.
    pub fn parse(value: &str) -> Result<Self, HostFunctionError> {
        let (path, arity) = value
            .rsplit_once('/')
            .ok_or_else(|| HostFunctionError::MissingArity(value.to_owned()))?;
        let arity = arity
            .parse::<usize>()
            .ok()
            .filter(|_| arity.bytes().all(|byte| byte.is_ascii_digit()))
            .ok_or_else(|| HostFunctionError::InvalidArity(value.to_owned()))?;
        let mut segments = path.split("::").collect::<Vec<_>>();
        if !segments.iter().all(|segment| is_rhai_identifier(segment)) {
            return Err(HostFunctionError::InvalidName(value.to_owned()));
        }
        let name = segments.pop().unwrap_or_default().to_owned();
        let namespace = (!segments.is_empty()).then(|| segments.join("::"));
        if namespace.as_deref() == Some("global") {
            return Err(HostFunctionError::GlobalNamespace(value.to_owned()));
        }
        Ok(Self {
            namespace,
            name,
            arity,
        })
    }

    /// The constructor of a custom primitive from its export: `OmbChart` for
    /// `gpui_rhai::OmbChart`, or `namespace::Export` for the primitive
    /// namespace `namespace`. It takes one argument, the props.
    ///
    /// # Errors
    ///
    /// Returns [`HostFunctionError::InvalidPrimitive`] when the export is not
    /// `PascalCase` or the namespace is not a primitive namespace.
    pub fn primitive(export: &str) -> Result<Self, HostFunctionError> {
        let (namespace, name) = export.split_once("::").unwrap_or(("gpui_rhai", export));
        let pascal_case = name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
            && name
                .chars()
                .all(|character| character.is_ascii_alphanumeric());
        // A primitive namespace is the part of a `PrimitiveId` before the dot.
        if !pascal_case || crate::PrimitiveId::parse(format!("{namespace}.export")).is_err() {
            return Err(HostFunctionError::InvalidPrimitive(export.to_owned()));
        }
        Ok(Self {
            namespace: Some(namespace.to_owned()),
            name: name.to_owned(),
            arity: 1,
        })
    }
}

/// A Rhai identifier: an ASCII letter or `_`, then letters, digits or `_`.
fn is_rhai_identifier(value: &str) -> bool {
    value != "_"
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
/// A malformed [`HostFunction`] declaration; each variant holds it.
pub enum HostFunctionError {
    #[error(
        "host function `{0}` has no arity; write `name/arity` or `module::name/arity`, such as `open_file/1`"
    )]
    MissingArity(String),
    #[error(
        "host function `{0}` has an invalid arity; write `name/arity` or `module::name/arity`, such as `open_file/1`"
    )]
    InvalidArity(String),
    #[error(
        "host function `{0}` has an invalid name; write `name/arity` or `module::name/arity`, such as `open_file/1`"
    )]
    InvalidName(String),
    #[error(
        "host function `{0}` names the `global` namespace; write `name/arity` for a global function"
    )]
    GlobalNamespace(String),
    #[error(
        "host primitive `{0}` must be a PascalCase export, `OmbChart` for `gpui_rhai::OmbChart` or `namespace::Export` for another primitive namespace"
    )]
    InvalidPrimitive(String),
}

impl fmt::Display for HostFunction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(namespace) = &self.namespace {
            write!(formatter, "{namespace}::")?;
        }
        write!(formatter, "{}/{}", self.name, self.arity)
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
    /// Native functions callable directly, by name and parameter count.
    native_direct: BTreeMap<String, BTreeSet<usize>>,
    /// Native functions by name and explicit argument count after the receiver.
    native_method: BTreeMap<String, BTreeSet<usize>>,
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

    fn insert_host_functions(&mut self, host: &BTreeSet<HostFunction>) {
        for function in host {
            if let Some(namespace) = &function.namespace {
                self.modules
                    .entry(namespace.clone())
                    .or_default()
                    .insert_direct(function.name.clone(), function.arity);
            } else {
                self.global
                    .insert_direct(function.name.clone(), function.arity);
                if function.arity > 0 {
                    self.global
                        .insert_method(function.name.clone(), function.arity - 1);
                }
            }
        }
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
                if function.function_type == MetadataFunctionType::Native {
                    if function.namespace == MetadataNamespace::Global {
                        self.native_direct
                            .entry(function.name.clone())
                            .or_default()
                            .insert(function.num_params);
                    }
                    if function.num_params > 0 {
                        self.native_method
                            .entry(function.name.clone())
                            .or_default()
                            .insert(function.num_params - 1);
                    }
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
    host: &BTreeSet<HostFunction>,
) -> Result<(Vec<KnownCallDiagnostic>, BTreeSet<HostFunction>), KnownCallLintError> {
    let mut catalog = KnownCallCatalog::from_engine(engine)?;
    catalog.insert_host_functions(host);
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
    let mut host_calls = HostCalls {
        declared: host,
        called: BTreeSet::new(),
    };
    for source in &parsed {
        lint_source(
            source,
            &catalog,
            &module_functions,
            &mut host_calls,
            &mut diagnostics,
        );
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
    Ok((diagnostics, host_calls.called))
}

/// The Host functions a lint was given and those the sources call.
struct HostCalls<'a> {
    declared: &'a BTreeSet<HostFunction>,
    called: BTreeSet<HostFunction>,
}

impl HostCalls<'_> {
    /// Record a call that resolves to a declared Host function.
    fn record(&mut self, namespace: Option<&str>, name: &str, arity: usize) {
        let function = HostFunction {
            namespace: namespace.map(ToOwned::to_owned),
            name: name.to_owned(),
            arity,
        };
        if self.declared.contains(&function) {
            self.called.insert(function);
        }
    }
}

pub(crate) fn lint_shadowed_builtins(
    engine: &mut Engine,
    entry_name: &str,
    entry_source: &str,
) -> Result<Vec<ShadowedBuiltin>, KnownCallLintError> {
    let catalog = KnownCallCatalog::from_engine(engine)?;
    let previous_optimization = engine.optimization_level();
    engine.set_optimization_level(OptimizationLevel::None);
    let parsed = parse_source(engine, entry_name, None, entry_source);
    engine.set_optimization_level(previous_optimization);
    let parsed = parsed?;
    let mut diagnostics = parsed
        .ast
        .iter_functions()
        .filter(|function| function.this_type.is_none())
        .filter_map(|function| {
            let arity = function.params.len();
            let captures_method_calls = takes(&catalog.native_method, function.name, arity);
            let captures_direct_calls = takes(&catalog.native_direct, function.name, arity);
            (captures_method_calls || captures_direct_calls).then(|| ShadowedBuiltin {
                source: entry_name.to_owned(),
                line: declaration_line(entry_source, function.name),
                function: function.name.to_owned(),
                arity,
                captures_method_calls,
                captures_direct_calls,
            })
        })
        .collect::<Vec<_>>();
    diagnostics
        .sort_by(|left, right| (left.line, &left.function).cmp(&(right.line, &right.function)));
    Ok(diagnostics)
}

/// The line of `fn name(` (or `private fn name(`) outside line comments.
fn declaration_line(source: &str, name: &str) -> Option<usize> {
    source
        .lines()
        .position(|line| {
            let code = line.split("//").next().unwrap_or(line);
            code.match_indices("fn ").any(|(index, _)| {
                let starts_word = code[..index]
                    .chars()
                    .next_back()
                    .is_none_or(|character| !character.is_alphanumeric() && character != '_');
                let rest = code[index + 3..].trim_start();
                starts_word
                    && rest
                        .strip_prefix(name)
                        .is_some_and(|after| after.trim_start().starts_with('('))
            })
        })
        .map(|index| index + 1)
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
    host: &mut HostCalls<'_>,
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
                let arity = call.args.len();
                match namespace.as_deref() {
                    None if !takes(&local.direct, &call.name, arity) => {
                        host.record(None, &call.name, arity);
                    }
                    Some("global") => host.record(None, &call.name, arity),
                    Some(namespace)
                        if !imports.contains_key(namespace.split("::").next().unwrap_or("")) =>
                    {
                        host.record(Some(namespace), &call.name, arity);
                    }
                    _ => {}
                }
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
                if !takes(&local.method, &call.name, call.args.len()) {
                    host.record(None, &call.name, call.args.len() + 1);
                }
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

fn takes(set: &BTreeMap<String, BTreeSet<usize>>, name: &str, arity: usize) -> bool {
    set.get(name)
        .is_some_and(|arities| arities.contains(&arity))
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
        assert_eq!(diagnostics[0].expected_arities, [] as [usize; 0]);
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
    fn host_functions_parse_from_declarations() {
        let parsed = |value: &str| HostFunction::parse(value).map(|function| function.to_string());
        assert_eq!(parsed("open_file/1").as_deref(), Ok("open_file/1"));
        assert_eq!(parsed("omb::status/0").as_deref(), Ok("omb::status/0"));
        assert_eq!(parsed("a::b::_c/12").as_deref(), Ok("a::b::_c/12"));
        let status = HostFunction::parse("omb::status/0").unwrap();
        assert_eq!(
            (
                status.namespace.as_deref(),
                status.name.as_str(),
                status.arity
            ),
            (Some("omb"), "status", 0)
        );
        for (value, error) in [
            (
                "open_file",
                HostFunctionError::MissingArity("open_file".into()),
            ),
            (
                "open_file/",
                HostFunctionError::InvalidArity("open_file/".into()),
            ),
            (
                "open_file/+1",
                HostFunctionError::InvalidArity("open_file/+1".into()),
            ),
            (
                "open file/1",
                HostFunctionError::InvalidName("open file/1".into()),
            ),
            ("omb::/0", HostFunctionError::InvalidName("omb::/0".into())),
            ("1open/0", HostFunctionError::InvalidName("1open/0".into())),
            ("_/0", HostFunctionError::InvalidName("_/0".into())),
            (
                "global::open/1",
                HostFunctionError::GlobalNamespace("global::open/1".into()),
            ),
        ] {
            assert_eq!(HostFunction::parse(value), Err(error), "{value}");
        }

        let primitive =
            |value: &str| HostFunction::primitive(value).map(|function| function.to_string());
        assert_eq!(
            primitive("OmbChart").as_deref(),
            Ok("gpui_rhai::OmbChart/1")
        );
        assert_eq!(primitive("omb::Chart").as_deref(), Ok("omb::Chart/1"));
        for value in [
            "ombChart",
            "Omb::Chart",
            "omb::chart",
            "omb__x::Chart",
            "Omb-Chart",
            "",
        ] {
            assert_eq!(
                HostFunction::primitive(value),
                Err(HostFunctionError::InvalidPrimitive(value.into())),
                "{value}"
            );
        }
    }

    #[test]
    fn known_call_lint_accepts_and_reports_declared_host_functions() {
        let open = HostFunction::parse("open_file/1").unwrap();
        let status = HostFunction::parse("omb::status/0").unwrap();
        let chart = HostFunction::primitive("OmbChart").unwrap();
        let unused = HostFunction::parse("close_file/1").unwrap();
        let host = BTreeSet::from([open.clone(), status.clone(), chart.clone(), unused]);
        let source = r#"
            fn clicked(ctx, payload) { "a.txt".open_file(); }
            fn view(ctx) {
                let state = omb::status();
                column([gpui_rhai::OmbChart(#{ key: "chart" }), text(state)])
            }
        "#;
        let mut runtime = RuntimeEngine::new();
        assert_eq!(
            runtime
                .lint_known_calls("ui/main.rhai", source, &BTreeMap::new())
                .unwrap()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            [
                "ui/main.rhai:2:48: unknown function call `open_file/0`",
                "ui/main.rhai:4:34: unknown function call `omb::status/0`",
                "ui/main.rhai:5:36: unknown function call `gpui_rhai::OmbChart/1`",
            ]
        );
        let (diagnostics, called) = runtime
            .lint_known_calls_with_host("ui/main.rhai", source, &BTreeMap::new(), &host)
            .unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        assert_eq!(called, BTreeSet::from([open.clone(), status, chart]));
        assert_eq!(open.to_string(), "open_file/1");

        // A wrong arity is still wrong; a script function of the same name
        // and a module import are not Host calls.
        let (diagnostics, called) = runtime
            .lint_known_calls_with_host(
                "ui/main.rhai",
                r#"
                    import "components/probe" as omb;
                    fn open_file(path) { path }
                    fn view(ctx) { omb::status(); open_file("a"); text(open_file()) }
                "#,
                &BTreeMap::from([(
                    ModuleId::parse("components/probe").unwrap(),
                    "fn status() { 0 }".to_owned(),
                )]),
                &host,
            )
            .unwrap();
        assert_eq!(
            diagnostics
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            ["ui/main.rhai:4:72: call `open_file/0` has the wrong arity; expected 1"]
        );
        assert!(called.is_empty(), "{called:?}");
    }

    #[test]
    fn shadowed_builtin_lint_reports_entry_functions_named_like_natives() {
        let mut runtime = RuntimeEngine::new();
        let source = "fn helper(value) { value }\n\
                      // fn index_of(values, key) in a comment\n\
                      fn index_of(values, key) { 0 }\n\
                      private fn contains(a) { false }\n\
                      fn len(a, b, c, d, e, f) { 0 }\n\
                      fn text(value) { value }\n\
                      fn view(ctx) { text(helper(\"x\")) }\n";
        let diagnostics = runtime
            .lint_shadowed_builtins("ui/main.rhai", source)
            .unwrap();
        let found = diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.function.as_str(),
                    diagnostic.line,
                    diagnostic.captures_method_calls,
                    diagnostic.captures_direct_calls,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            found,
            [
                ("index_of", Some(3), true, true),
                ("contains", Some(4), true, false),
                ("text", Some(6), false, true),
            ]
        );
        assert_eq!(
            diagnostics[1].to_string(),
            "ui/main.rhai:4: `fn contains` takes over built-in method calls `x.contains(a)`, \
             also inside imported components; rename it"
        );
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
