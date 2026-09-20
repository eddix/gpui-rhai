use std::collections::BTreeMap;
use std::fmt;

use rhai::EvalAltResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ComponentInstancePath, ExecutionOperation, ExecutionTiming, MAX_SCRIPT_OPERATIONS,
    RuntimeError, StateInstanceSnapshot, UiContext,
};

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
    pub execution_timing: Option<ExecutionTiming>,
    pub component_state: Vec<DiagnosticStateSnapshot>,
}

impl DiagnosticContext {
    /// Capture the runtime details associated with a failed script evaluation.
    ///
    /// # Errors
    ///
    /// Returns an error when component state is currently mutably borrowed.
    pub fn capture(
        engine: &crate::RuntimeEngine,
        context: &UiContext,
        source: Option<String>,
        key: Option<String>,
    ) -> Result<Self, DiagnosticContextError> {
        let component = context.component_path().clone();
        let component_state = context
            .runtime()
            .try_borrow()
            .map_err(|_| DiagnosticContextError::StateBorrowed)?
            .component_state
            .inspect();
        Ok(Self {
            source,
            component: Some(component.clone()),
            key,
            execution_timing: engine.last_failed_timing(),
            component_state: redact_component_state(&component_state, &component),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DiagnosticContextError {
    #[error("component state is already borrowed")]
    StateBorrowed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticExecutionTiming {
    pub operation: String,
    pub source: String,
    pub duration_micros: u64,
    pub succeeded: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticPosition {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticStateSnapshot {
    pub path: String,
    pub fields: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: DiagnosticCode,
    pub message: String,
    pub source: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub position: Option<DiagnosticPosition>,
    pub component: Option<String>,
    pub key: Option<String>,
    pub stack: Vec<DiagnosticFrame>,
    pub error_kind: Option<String>,
    pub token: Option<Value>,
    pub execution: Option<DiagnosticExecutionTiming>,
    pub operations: Option<u64>,
    pub max_operations: Option<u64>,
    pub component_state: Vec<DiagnosticStateSnapshot>,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        formatter.write_str(&json)
    }
}

impl Diagnostic {
    #[must_use]
    #[allow(clippy::too_many_lines)]
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
                position: None,
                component: context.component.as_ref().map(ToString::to_string),
                key: context.key.clone(),
                stack: Vec::new(),
                ..runtime_context(context)
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
                position: None,
                component: Some(component.to_string()),
                key: context.key.clone(),
                stack: Vec::new(),
                ..runtime_context(context)
            },
            RuntimeError::RetainedCallback { source, .. } => Self {
                severity: DiagnosticSeverity::Error,
                code: DiagnosticCode::ScriptEvaluate,
                message: source.to_string(),
                source: context.source.clone(),
                line: None,
                column: None,
                position: None,
                component: context.component.as_ref().map(ToString::to_string),
                key: context.key.clone(),
                stack: Vec::new(),
                ..runtime_context(context)
            },
            RuntimeError::ComponentRuntime(message) => Self {
                severity: DiagnosticSeverity::Error,
                code: DiagnosticCode::ScriptEvaluate,
                message: message.clone(),
                source: context.source.clone(),
                line: None,
                column: None,
                position: None,
                component: context.component.as_ref().map(ToString::to_string),
                key: context.key.clone(),
                stack: Vec::new(),
                ..runtime_context(context)
            },
            RuntimeError::MissingComponentInvocation(component) => Self {
                severity: DiagnosticSeverity::Error,
                code: DiagnosticCode::ScriptEvaluate,
                message: format!("component invocation `{component}` is unavailable"),
                source: context.source.clone(),
                line: None,
                column: None,
                position: None,
                component: Some(component.to_string()),
                key: context.key.clone(),
                stack: Vec::new(),
                ..runtime_context(context)
            },
            RuntimeError::Import(message) => Self {
                severity: DiagnosticSeverity::Error,
                code: DiagnosticCode::ScriptCompile,
                message: message.clone(),
                source: context.source.clone(),
                line: None,
                column: None,
                position: None,
                component: context.component.as_ref().map(ToString::to_string),
                key: context.key.clone(),
                stack: Vec::new(),
                ..runtime_context(context)
            },
            RuntimeError::InvalidAssignmentTarget(position) => Self {
                severity: DiagnosticSeverity::Error,
                code: DiagnosticCode::ScriptCompile,
                message: error.to_string(),
                source: context.source.clone(),
                line: position.line(),
                column: position.position(),
                position: position
                    .line()
                    .zip(position.position())
                    .map(|(line, column)| DiagnosticPosition { line, column }),
                component: context.component.as_ref().map(ToString::to_string),
                key: context.key.clone(),
                stack: Vec::new(),
                ..runtime_context(context)
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
    let (error_kind, token) = eval_details(leaf);
    let line = position.line();
    let column = position.position();
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        code,
        message: error.to_string(),
        source: source_for(error).or_else(|| context.source.clone()),
        line,
        column,
        position: line
            .zip(column)
            .map(|(line, column)| DiagnosticPosition { line, column }),
        component: context.component.as_ref().map(ToString::to_string),
        key: context.key.clone(),
        stack,
        error_kind,
        token,
        ..runtime_context(context)
    }
}

fn eval_details(error: &EvalAltResult) -> (Option<String>, Option<Value>) {
    match error {
        EvalAltResult::ErrorTerminated(token, _) => (
            Some("ErrorTerminated".to_owned()),
            Some(
                rhai::serde::from_dynamic::<Value>(token)
                    .unwrap_or_else(|_| Value::String(token.to_string())),
            ),
        ),
        _ => (None, None),
    }
}

fn runtime_context(context: &DiagnosticContext) -> Diagnostic {
    let execution = context
        .execution_timing
        .as_ref()
        .map(DiagnosticExecutionTiming::from);
    let operations = context
        .execution_timing
        .as_ref()
        .map(|timing| timing.operations);
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        code: DiagnosticCode::ScriptEvaluate,
        message: String::new(),
        source: None,
        line: None,
        column: None,
        position: None,
        component: None,
        key: None,
        stack: Vec::new(),
        error_kind: None,
        token: None,
        execution,
        operations,
        max_operations: operations.map(|_| MAX_SCRIPT_OPERATIONS),
        component_state: context.component_state.clone(),
    }
}

impl From<&ExecutionTiming> for DiagnosticExecutionTiming {
    fn from(timing: &ExecutionTiming) -> Self {
        Self {
            operation: match &timing.operation {
                ExecutionOperation::Compile => "compile".to_owned(),
                ExecutionOperation::Render => "render".to_owned(),
                ExecutionOperation::ComponentReuse(count) => format!("component_reuse:{count}"),
                ExecutionOperation::VirtualCollection(name) => {
                    format!("virtual_collection:{name}")
                }
                ExecutionOperation::Lifecycle(name) => format!("lifecycle:{name}"),
                ExecutionOperation::Callback(name) => format!("callback:{name}"),
            },
            source: timing.source.clone(),
            duration_micros: timing.duration.as_micros().try_into().unwrap_or(u64::MAX),
            succeeded: timing.succeeded,
        }
    }
}

fn redact_component_state(
    snapshots: &[StateInstanceSnapshot],
    component: &ComponentInstancePath,
) -> Vec<DiagnosticStateSnapshot> {
    snapshots
        .iter()
        .filter(|snapshot| snapshot.path == *component)
        .map(|snapshot| DiagnosticStateSnapshot {
            path: snapshot.path.to_string(),
            fields: snapshot
                .fields
                .iter()
                .map(|(name, field)| {
                    let value = if field.sensitive {
                        Value::String("<redacted>".to_owned())
                    } else {
                        serde_json::to_value(&field.value).unwrap_or(Value::Null)
                    };
                    (name.clone(), value)
                })
                .collect(),
        })
        .collect()
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
    use std::collections::BTreeMap;
    use std::time::Duration;

    use rhai::{Dynamic, Position};

    use crate::{
        ExecutionOperation, ExecutionTiming, MAX_SCRIPT_OPERATIONS, RuntimeEngine,
        StateInstanceSnapshot, StateValueSnapshot, UiValue,
    };

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
                ..DiagnosticContext::default()
            },
        );
        assert_eq!(diagnostic.code, DiagnosticCode::ScriptEvaluate);
        assert_eq!(diagnostic.source.as_deref(), Some("ui/main.rhai"));
        assert!(diagnostic.line.is_some());
        assert_eq!(diagnostic.component.as_deref(), Some("/App[root]"));
        assert!(diagnostic.stack.iter().any(|frame| frame.name == "fail"));
    }

    #[test]
    fn terminated_evaluation_preserves_structured_runtime_context() {
        let component = ComponentInstancePath::root("App", "root").child("LoginForm", "primary");
        let other_component =
            ComponentInstancePath::root("App", "root").child("Sidebar", "primary");
        let error = RuntimeError::Evaluate(Box::new(EvalAltResult::ErrorInFunctionCall(
            "submit".to_owned(),
            "ui/login.rhai".to_owned(),
            Box::new(EvalAltResult::ErrorTerminated(
                Dynamic::from("operation-limit"),
                Position::new(17, 9),
            )),
            Position::new(20, 5),
        )));
        let timing = ExecutionTiming {
            operation: ExecutionOperation::Callback("submit".to_owned()),
            source: "ui/login.rhai".to_owned(),
            duration: Duration::from_micros(2_500),
            operations: 1_000_001,
            operation_semantics: crate::OPERATION_SEMANTICS_VERSION,
            slow: false,
            succeeded: false,
        };
        let state = vec![
            StateInstanceSnapshot {
                path: component.clone(),
                fields: BTreeMap::from([
                    (
                        "attempts".to_owned(),
                        StateValueSnapshot {
                            value: UiValue::Integer(3),
                            sensitive: false,
                        },
                    ),
                    (
                        "password".to_owned(),
                        StateValueSnapshot {
                            value: UiValue::String("hunter2".to_owned()),
                            sensitive: true,
                        },
                    ),
                ]),
            },
            StateInstanceSnapshot {
                path: other_component,
                fields: BTreeMap::from([(
                    "expanded".to_owned(),
                    StateValueSnapshot {
                        value: UiValue::Bool(true),
                        sensitive: false,
                    },
                )]),
            },
        ];

        let component_state = redact_component_state(&state, &component);
        let diagnostic = Diagnostic::from_runtime(
            &error,
            &DiagnosticContext {
                source: Some("ui/login.rhai".to_owned()),
                component: Some(component),
                key: Some("primary".to_owned()),
                execution_timing: Some(timing),
                component_state,
            },
        );

        assert_eq!(diagnostic.error_kind.as_deref(), Some("ErrorTerminated"));
        assert_eq!(diagnostic.token, Some(serde_json::json!("operation-limit")));
        assert_eq!(diagnostic.line, Some(17));
        assert_eq!(diagnostic.column, Some(9));
        assert_eq!(
            diagnostic.position,
            Some(DiagnosticPosition {
                line: 17,
                column: 9
            })
        );
        assert_eq!(diagnostic.operations, Some(1_000_001));
        assert_eq!(diagnostic.max_operations, Some(MAX_SCRIPT_OPERATIONS));
        assert_eq!(
            diagnostic.component.as_deref(),
            Some("/App[root]/LoginForm[primary]")
        );
        let execution = diagnostic.execution.as_ref().unwrap();
        assert_eq!(execution.operation, "callback:submit");
        assert_eq!(execution.source, "ui/login.rhai");
        assert_eq!(execution.duration_micros, 2_500);
        assert!(!execution.succeeded);
        assert_eq!(diagnostic.component_state.len(), 1);
        assert_eq!(
            diagnostic.component_state[0].fields["attempts"],
            serde_json::json!({ "type": "integer", "value": 3 })
        );
        assert_eq!(
            diagnostic.component_state[0].fields["password"],
            serde_json::json!("<redacted>")
        );
        assert!(
            !serde_json::to_string(&diagnostic)
                .unwrap()
                .contains("hunter2")
        );
    }
}
