use rhai::EvalAltResult;
use serde::{Deserialize, Serialize};

use crate::{ComponentInstancePath, RuntimeError};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    ScriptCompile,
    ScriptEvaluate,
    StaleCallback,
    SlowExecution,
    PrimitiveRender,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticFrame {
    pub kind: String,
    pub name: String,
    pub source: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct DiagnosticContext {
    pub source: Option<String>,
    pub component: Option<ComponentInstancePath>,
    pub key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: DiagnosticCode,
    pub message: String,
    pub source: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub component: Option<String>,
    pub key: Option<String>,
    pub stack: Vec<DiagnosticFrame>,
}

impl Diagnostic {
    #[must_use]
    pub fn from_runtime(error: &RuntimeError, context: &DiagnosticContext) -> Self {
        match error {
            RuntimeError::Compile(error) => {
                from_eval(DiagnosticCode::ScriptCompile, error, context)
            }
            RuntimeError::Evaluate(error)
            | RuntimeError::CallbackDefinition { source: error, .. } => {
                from_eval(DiagnosticCode::ScriptEvaluate, error, context)
            }
            RuntimeError::StaleCallback {
                name,
                callback_generation,
                current_generation,
            } => Self {
                severity: DiagnosticSeverity::Error,
                code: DiagnosticCode::StaleCallback,
                message: format!(
                    "callback `{name}` belongs to generation {callback_generation}; current generation is {current_generation}"
                ),
                source: context.source.clone(),
                line: None,
                column: None,
                component: context.component.as_ref().map(ToString::to_string),
                key: context.key.clone(),
                stack: Vec::new(),
            },
            RuntimeError::StaleComponentCallback { name, component } => Self {
                severity: DiagnosticSeverity::Error,
                code: DiagnosticCode::StaleCallback,
                message: format!(
                    "callback `{name}` belongs to an unmounted incarnation of `{component}`"
                ),
                source: context.source.clone(),
                line: None,
                column: None,
                component: Some(component.to_string()),
                key: context.key.clone(),
                stack: Vec::new(),
            },
            RuntimeError::RetainedCallback { source, .. } => Self {
                severity: DiagnosticSeverity::Error,
                code: DiagnosticCode::ScriptEvaluate,
                message: source.to_string(),
                source: context.source.clone(),
                line: None,
                column: None,
                component: context.component.as_ref().map(ToString::to_string),
                key: context.key.clone(),
                stack: Vec::new(),
            },
            RuntimeError::ComponentRuntime(message) => Self {
                severity: DiagnosticSeverity::Error,
                code: DiagnosticCode::ScriptEvaluate,
                message: message.clone(),
                source: context.source.clone(),
                line: None,
                column: None,
                component: context.component.as_ref().map(ToString::to_string),
                key: context.key.clone(),
                stack: Vec::new(),
            },
            RuntimeError::MissingComponentInvocation(component) => Self {
                severity: DiagnosticSeverity::Error,
                code: DiagnosticCode::ScriptEvaluate,
                message: format!("component invocation `{component}` is unavailable"),
                source: context.source.clone(),
                line: None,
                column: None,
                component: Some(component.to_string()),
                key: context.key.clone(),
                stack: Vec::new(),
            },
            RuntimeError::Import(message) => Self {
                severity: DiagnosticSeverity::Error,
                code: DiagnosticCode::ScriptCompile,
                message: message.clone(),
                source: context.source.clone(),
                line: None,
                column: None,
                component: context.component.as_ref().map(ToString::to_string),
                key: context.key.clone(),
                stack: Vec::new(),
            },
            RuntimeError::InvalidAssignmentTarget(position) => Self {
                severity: DiagnosticSeverity::Error,
                code: DiagnosticCode::ScriptCompile,
                message: error.to_string(),
                source: context.source.clone(),
                line: position.line(),
                column: position.position(),
                component: context.component.as_ref().map(ToString::to_string),
                key: context.key.clone(),
                stack: Vec::new(),
            },
        }
    }
}

fn from_eval(
    code: DiagnosticCode,
    error: &EvalAltResult,
    context: &DiagnosticContext,
) -> Diagnostic {
    let leaf = leaf_error(error);
    let position = leaf.position();
    let mut stack = Vec::new();
    collect_frames(error, &mut stack);
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        code,
        message: error.to_string(),
        source: source_for(error).or_else(|| context.source.clone()),
        line: position.line(),
        column: position.position(),
        component: context.component.as_ref().map(ToString::to_string),
        key: context.key.clone(),
        stack,
    }
}

fn leaf_error(mut error: &EvalAltResult) -> &EvalAltResult {
    loop {
        error = match error {
            EvalAltResult::ErrorInFunctionCall(_, _, inner, _)
            | EvalAltResult::ErrorInModule(_, inner, _) => inner,
            _ => return error,
        };
    }
}

fn source_for(error: &EvalAltResult) -> Option<String> {
    match error {
        EvalAltResult::ErrorInFunctionCall(_, source, inner, _) => {
            source_for(inner).or_else(|| (!source.is_empty()).then(|| source.clone()))
        }
        EvalAltResult::ErrorInModule(module, inner, _) => {
            source_for(inner).or_else(|| (!module.is_empty()).then(|| module.clone()))
        }
        _ => None,
    }
}

fn collect_frames(error: &EvalAltResult, frames: &mut Vec<DiagnosticFrame>) {
    match error {
        EvalAltResult::ErrorInFunctionCall(function, source, inner, _) => {
            frames.push(DiagnosticFrame {
                kind: "function".to_owned(),
                name: function.clone(),
                source: (!source.is_empty()).then(|| source.clone()),
            });
            collect_frames(inner, frames);
        }
        EvalAltResult::ErrorInModule(module, inner, _) => {
            frames.push(DiagnosticFrame {
                kind: "module".to_owned(),
                name: module.clone(),
                source: Some(module.clone()),
            });
            collect_frames(inner, frames);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeEngine;

    #[test]
    fn runtime_diagnostic_contains_location_component_and_stack() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile_named(
                "ui/main.rhai",
                r#"
                    fn fail() { throw "boom"; }
                    fn view() { fail(); }
                "#,
            )
            .unwrap();
        let error = runtime.render(&compiled).unwrap_err();
        let diagnostic = Diagnostic::from_runtime(
            &error,
            &DiagnosticContext {
                source: Some("ui/main.rhai".to_owned()),
                component: Some(ComponentInstancePath::root("App", "root")),
                key: Some("root".to_owned()),
            },
        );
        assert_eq!(diagnostic.code, DiagnosticCode::ScriptEvaluate);
        assert_eq!(diagnostic.source.as_deref(), Some("ui/main.rhai"));
        assert!(diagnostic.line.is_some());
        assert_eq!(diagnostic.component.as_deref(), Some("/App[root]"));
        assert!(diagnostic.stack.iter().any(|frame| frame.name == "fail"));
    }
}
