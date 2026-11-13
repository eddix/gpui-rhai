use rhai::{ASTNode, Expr, Stmt};

use super::{KnownCallCatalog, ParsedSource};
use crate::{AppManifest, CapabilityCallKind, CapabilityContract, CapabilityId, HostFunction};
use std::collections::BTreeSet;

/// A source location and a payload-free capability preflight result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityCallDiagnostic {
    pub source: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub message: String,
}

impl std::fmt::Display for CapabilityCallDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source)?;
        if let Some(line) = self.line {
            write!(f, ":{line}")?;
            if let Some(column) = self.column {
                write!(f, ":{column}")?;
            }
        }
        write!(f, ": {}", self.message)
    }
}

/// Static coverage, not proof of authorization or lifecycle correctness.
#[derive(Clone, Debug, Default)]
pub struct CapabilityCallReport {
    /// Calls whose literal ID/method, registered mode and manifest requirement match.
    pub checked_calls: usize,
    /// Checked calls with fully literal, schema-validated input.
    pub checked_inputs: usize,
    /// Calls left to the Host: missing contract, dynamic name, or ambiguous dispatch.
    pub unchecked: Vec<CapabilityCallDiagnostic>,
    /// Definite errors only. No diagnostic includes an input payload.
    pub diagnostics: Vec<CapabilityCallDiagnostic>,
}

pub(super) fn check(
    sources: &[ParsedSource],
    catalog: &KnownCallCatalog,
    host: &BTreeSet<HostFunction>,
    manifest: &AppManifest,
    contract: Option<&CapabilityContract>,
) -> CapabilityCallReport {
    let script_functions = sources
        .iter()
        .flat_map(|source| source.ast.iter_functions())
        .map(|function| (function.name.to_owned(), function.params.len()))
        .collect::<BTreeSet<_>>();
    let mut report = CapabilityCallReport::default();
    for source in sources {
        source.ast.walk(&mut |path| {
            let (call, position, method) = match path.last() {
                Some(ASTNode::Expr(Expr::MethodCall(call, position))) => (call, *position, true),
                Some(ASTNode::Expr(Expr::FnCall(call, position)) | ASTNode::Stmt(Stmt::FnCall(call, position)))
                    if call.namespace.is_empty() || call.namespace.to_string() == "global" => (call, *position, false),
                _ => return true,
            };
            let Some(kind) = CapabilityCallKind::from_function(&call.name) else { return true; };
            let offset = usize::from(!method);
            let native_arity = call.args.len() + usize::from(method);
            // Arity and receiver come from real registration, not another API
            // signature table. Ordinary known-call lint handles wrong arities.
            let Some(overloads) = catalog.native_direct_types.get(call.name.as_str()) else { return true; };
            if call.args.len() < offset + 3 || !overloads.iter().any(|types|
                types.len() == native_arity && types[0] == "UiContext") { return true; }
            let diagnostic = |message: String| CapabilityCallDiagnostic {
                source: source.name.clone(), line: position.line(), column: position.position(), message,
            };
            // Rhai dispatch is dynamic. A script function or another native
            // receiver can take over this name: never reject that Host API as
            // if it necessarily called UiContext.
            let ambiguous = script_functions.contains(&(call.name.to_string(), call.args.len()))
                || host.iter().any(|function| function.namespace.is_none()
                    && function.name == call.name && function.arity == native_arity)
                || overloads.iter().any(|types| types.len() == native_arity && types[0] != "UiContext");
            if ambiguous {
                report.unchecked.push(diagnostic(format!("`{}` has script/Host overloads; capability dispatch is unverified", call.name)));
                return true;
            }
            let [Expr::StringConstant(raw_id, _), method_expression, input, ..] = &call.args[offset..] else {
                report.unchecked.push(diagnostic(format!("`{}` uses a dynamic capability ID or method; validate in the Host", call.name)));
                return true;
            };
            let Ok(id) = CapabilityId::parse(raw_id.to_string()) else {
                report.diagnostics.push(diagnostic("invalid literal capability ID".into()));
                return true;
            };
            let Some(requirement) = manifest.capabilities.get(&id) else {
                report.diagnostics.push(diagnostic(format!("capability `{id}` is not declared by the app manifest")));
                return true;
            };
            let Expr::StringConstant(method_name, _) = method_expression else {
                report.unchecked.push(diagnostic(format!("`{id}` uses a dynamic method; validate in the Host")));
                return true;
            };
            let Some(contract) = contract else {
                report.unchecked.push(diagnostic(format!("`{id}.{method_name}` has no Host contract; method, entry point and schemas are unverified")));
                return true;
            };
            let Some(entry) = contract.capabilities.get(&id) else {
                report.diagnostics.push(diagnostic(format!("Host contract has no capability `{id}`")));
                return true;
            };
            if !requirement.matches(&entry.version) {
                report.diagnostics.push(diagnostic(format!("`{id}` requires {requirement}, Host contract provides {}", entry.version)));
                return true;
            }
            let Some(schema) = entry.methods.get(method_name.as_str()) else {
                report.diagnostics.push(diagnostic(format!("Host contract has no method `{id}.{method_name}`")));
                return true;
            };
            if !entry.modes.contains(&kind) {
                report.diagnostics.push(diagnostic(format!("`{id}.{method_name}` has no {kind:?} entry point")));
                return true;
            }
            report.checked_calls += 1;
            if is_literal(input) && let Some(value) = input.get_literal_value(None) {
                if crate::UiValue::from_dynamic(value)
                    .is_ok_and(|value| schema.input.validate_ui_value(&value).is_ok()) {
                    report.checked_inputs += 1;
                } else {
                    report.diagnostics.push(diagnostic(format!("literal input for `{id}.{method_name}` does not match its declared schema")));
                }
            }
            true
        });
    }
    report
}

// No function calls, constant propagation, interpolation, or native evaluation.
fn is_literal(expression: &Expr) -> bool {
    match expression {
        Expr::Unit(_)
        | Expr::BoolConstant(..)
        | Expr::IntegerConstant(..)
        | Expr::FloatConstant(..)
        | Expr::StringConstant(..) => true,
        Expr::Array(items, _) => items.iter().all(is_literal),
        Expr::Map(map, _) => map.0.iter().all(|(_, value)| is_literal(value)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, collections::BTreeMap, rc::Rc};

    #[test]
    fn preflight_disables_full_optimizer_and_restores_it_without_running_native_code() {
        let calls = Rc::new(Cell::new(0));
        let observed = Rc::clone(&calls);
        let mut engine = rhai::Engine::new();
        engine.register_fn("probe", move || {
            observed.set(observed.get() + 1);
            7_i64
        });
        engine.set_optimization_level(rhai::OptimizationLevel::Full);
        engine.compile("probe()").unwrap();
        assert_eq!(
            calls.get(),
            1,
            "positive control: Full executes the native function"
        );
        calls.set(0);
        let manifest = crate::AppManifest::new(crate::ModuleId::parse("main").unwrap());
        super::super::preflight_calls(
            &mut engine,
            "source",
            "probe(); fn unused() { if false { probe(); } }",
            &BTreeMap::new(),
            &super::BTreeSet::new(),
            Some((&manifest, None)),
        )
        .unwrap();
        assert_eq!(calls.get(), 0);
        assert_eq!(engine.optimization_level(), rhai::OptimizationLevel::Full);
        // Failure must restore the optimizer too.
        assert!(
            super::super::preflight_calls(
                &mut engine,
                "source",
                "fn broken(",
                &BTreeMap::new(),
                &super::BTreeSet::new(),
                Some((&manifest, None))
            )
            .is_err()
        );
        assert_eq!(engine.optimization_level(), rhai::OptimizationLevel::Full);
    }

    #[test]
    fn a_real_native_receiver_overload_is_left_to_dynamic_dispatch() {
        let mut engine = rhai::Engine::new();
        crate::context::register_ui_context_api(&mut engine);
        engine.register_fn(
            "call_capability",
            |_: &mut String, _: String, _: String, _: rhai::Dynamic| 42_i64,
        );
        let manifest = crate::AppManifest::new(crate::ModuleId::parse("main").unwrap());
        let report = super::super::preflight_calls(
            &mut engine,
            "source",
            r#"fn event() { "other receiver".call_capability("other.api", "fine", ()); }"#,
            &BTreeMap::new(),
            &super::BTreeSet::new(),
            Some((&manifest, None)),
        )
        .unwrap();
        assert_eq!(report.capabilities.checked_calls, 0);
        assert_eq!(report.capabilities.unchecked.len(), 1);
        assert!(
            report.capabilities.diagnostics.is_empty(),
            "{:?}",
            report.capabilities.diagnostics
        );
    }
}
