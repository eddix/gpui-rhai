use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{AccessibilityNode, AccessibilityTree, EventPhase, NodeId, UiEventHandler, UiValue};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutomationLocator {
    SemanticId { id: String },
    TestId { id: String },
    RoleName { role: String, name: String },
    Text { text: String },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AutomationBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationNode {
    pub id: u64,
    pub parent: Option<u64>,
    pub role: String,
    pub name: String,
    pub description: String,
    pub semantic_id: Option<String>,
    pub test_id: Option<String>,
    pub value: Option<UiValue>,
    pub checked: Option<UiValue>,
    pub disabled: bool,
    pub invalid: bool,
    pub required: bool,
    pub bounds: Option<AutomationBounds>,
    pub children: Vec<u64>,
}

impl From<&AccessibilityNode> for AutomationNode {
    fn from(node: &AccessibilityNode) -> Self {
        Self {
            id: node.id.get(),
            parent: node.parent.map(NodeId::get),
            role: node.role.clone(),
            name: node.name.clone(),
            description: node.description.clone(),
            semantic_id: node.semantic_id.clone(),
            test_id: node.test_id.clone(),
            value: node.value.clone(),
            checked: node.checked.clone(),
            disabled: node.disabled,
            invalid: node.invalid,
            required: node.required,
            bounds: node.geometry.map(|geometry| AutomationBounds {
                x: geometry.visual.x,
                y: geometry.visual.y,
                width: geometry.visual.width,
                height: geometry.visual.height,
            }),
            children: node.children.iter().copied().map(NodeId::get).collect(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AutomationSnapshot {
    pub roots: Vec<u64>,
    pub nodes: Vec<AutomationNode>,
}

impl AutomationSnapshot {
    #[must_use]
    pub fn from_accessibility(tree: &AccessibilityTree) -> Self {
        Self {
            roots: tree.root_ids().iter().copied().map(NodeId::get).collect(),
            nodes: tree.nodes().map(AutomationNode::from).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum AutomationCommand {
    Snapshot,
    Query {
        locator: AutomationLocator,
    },
    Dispatch {
        locator: AutomationLocator,
        event: String,
        #[serde(default)]
        payload: Option<UiValue>,
    },
    Action {
        id: String,
        #[serde(default)]
        payload: Option<UiValue>,
    },
    AdvanceTime {
        millis: u64,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AutomationDispatchReport {
    pub target: u64,
    pub visited: Vec<u64>,
    pub invoked: usize,
    pub default_prevented: bool,
    pub stopped: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum AutomationResult {
    Snapshot { snapshot: AutomationSnapshot },
    Node { node: Box<AutomationNode> },
    Dispatch { report: AutomationDispatchReport },
    Action { id: String },
    Advanced { millis: u64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationRequest {
    #[serde(default)]
    pub id: serde_json::Value,
    #[serde(flatten)]
    pub command: AutomationCommand,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutomationResponse {
    pub id: serde_json::Value,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<AutomationResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum AutomationError {
    #[error("automation locator matched no retained semantic node: {0}")]
    NoMatch(String),
    #[error("automation locator is ambiguous ({count} matches): {locator}")]
    Ambiguous { locator: String, count: usize },
    #[error("automation target node {0} is no longer retained")]
    StaleTarget(u64),
    #[error("automation target node {0} is disabled")]
    Disabled(u64),
    #[error("automation event name must be a non-empty safe identifier")]
    InvalidEvent,
    #[error("runtime clock does not support deterministic advance")]
    ClockNotControllable,
    #[error("automation command failed: {0}")]
    Command(String),
}

#[derive(Clone)]
pub(crate) struct AutomationDispatchStep {
    pub node: NodeId,
    pub phase: EventPhase,
    pub handler: UiEventHandler,
}

pub(crate) fn resolve_locator(
    tree: &AccessibilityTree,
    locator: &AutomationLocator,
) -> Result<NodeId, AutomationError> {
    let matches = match locator {
        AutomationLocator::SemanticId { id } => tree
            .find_by_semantic_id(id)
            .into_iter()
            .map(|node| node.id)
            .collect::<Vec<_>>(),
        AutomationLocator::TestId { id } => tree.find_by_test_id(id).map(|node| node.id).collect(),
        AutomationLocator::RoleName { role, name } => tree
            .find_by_role_and_name(role, name)
            .map(|node| node.id)
            .collect(),
        AutomationLocator::Text { text } => tree
            .nodes()
            .filter(|node| node.name == *text)
            .map(|node| node.id)
            .collect(),
    };
    match matches.as_slice() {
        [node] => Ok(*node),
        [] => Err(AutomationError::NoMatch(format!("{locator:?}"))),
        _ => Err(AutomationError::Ambiguous {
            locator: format!("{locator:?}"),
            count: matches.len(),
        }),
    }
}

pub(crate) fn dispatch_plan(
    tree: &crate::RetainedUiTree,
    target: NodeId,
    event: &str,
) -> Result<Vec<AutomationDispatchStep>, AutomationError> {
    if event.is_empty()
        || !event
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | ':'))
    {
        return Err(AutomationError::InvalidEvent);
    }
    let target_node = tree
        .node(target)
        .ok_or_else(|| AutomationError::StaleTarget(target.get()))?;
    if target_node.attributes().get("disabled") == Some(&UiValue::Bool(true)) {
        return Err(AutomationError::Disabled(target.get()));
    }
    let mut route = Vec::new();
    let mut current = Some(target);
    while let Some(node) = current {
        route.push(node);
        current = tree.node(node).and_then(crate::RetainedNode::parent);
    }
    route.reverse();
    let mut steps = Vec::new();
    for node in &route {
        append_phase(tree, *node, event, EventPhase::Capture, &mut steps)?;
    }
    append_phase(tree, target, event, EventPhase::Target, &mut steps)?;
    for node in route.iter().rev() {
        append_phase(tree, *node, event, EventPhase::Bubble, &mut steps)?;
    }
    Ok(steps)
}

fn append_phase(
    tree: &crate::RetainedUiTree,
    node: NodeId,
    event: &str,
    phase: EventPhase,
    output: &mut Vec<AutomationDispatchStep>,
) -> Result<(), AutomationError> {
    let retained = tree
        .node(node)
        .ok_or_else(|| AutomationError::StaleTarget(node.get()))?;
    output.extend(
        retained
            .event_handlers(event)
            .iter()
            .filter(|binding| binding.phase() == phase)
            .map(|binding| AutomationDispatchStep {
                node,
                phase,
                handler: binding.handler().clone(),
            }),
    );
    Ok(())
}

/// Decode one request, invoke the foreground handler, and encode one response.
///
/// # Errors
///
/// Returns a JSON serialization error if the response cannot be encoded.
pub fn handle_automation_json_line(
    line: &str,
    mut handle: impl FnMut(AutomationCommand) -> Result<AutomationResult, AutomationError>,
) -> Result<String, serde_json::Error> {
    let request = serde_json::from_str::<AutomationRequest>(line);
    let response = match request {
        Ok(request) => match handle(request.command) {
            Ok(value) => AutomationResponse {
                id: request.id,
                ok: true,
                value: Some(value),
                error: None,
            },
            Err(error) => AutomationResponse {
                id: request.id,
                ok: false,
                value: None,
                error: Some(error.to_string()),
            },
        },
        Err(error) => AutomationResponse {
            id: serde_json::Value::Null,
            ok: false,
            value: None,
            error: Some(error.to_string()),
        },
    };
    serde_json::to_string(&response)
}

/// Serve newline-delimited JSON requests synchronously through a Host handler.
///
/// # Errors
///
/// Returns reader/writer I/O errors or response serialization errors wrapped as
/// [`std::io::Error`].
pub fn run_automation_json_lines(
    reader: impl BufRead,
    mut writer: impl Write,
    mut handle: impl FnMut(AutomationCommand) -> Result<AutomationResult, AutomationError>,
) -> std::io::Result<()> {
    for line in reader.lines() {
        let response =
            handle_automation_json_line(&line?, &mut handle).map_err(std::io::Error::other)?;
        writeln!(writer, "{response}")?;
        writer.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locators_require_one_unambiguous_semantic_node() {
        let mut retained = crate::RetainedUiTree::new();
        retained
            .reconcile(crate::UiNode::box_node(vec![
                crate::UiNode::text("Save")
                    .with_key("save")
                    .with_attribute("role", UiValue::String("button".to_owned()))
                    .with_attribute("test_id", UiValue::String("save".to_owned())),
                crate::UiNode::text("Save")
                    .with_key("save-copy")
                    .with_attribute("role", UiValue::String("button".to_owned())),
            ]))
            .unwrap();
        let tree =
            AccessibilityTree::from_retained(&retained, &crate::GeometryRegistry::new()).unwrap();
        assert!(
            resolve_locator(
                &tree,
                &AutomationLocator::TestId {
                    id: "save".to_owned()
                }
            )
            .is_ok()
        );
        assert!(matches!(
            resolve_locator(
                &tree,
                &AutomationLocator::Text {
                    text: "Save".to_owned()
                }
            ),
            Err(AutomationError::Ambiguous { count: 2, .. })
        ));
    }

    #[test]
    fn json_lines_protocol_correlates_success_and_errors() {
        let line = r#"{"id":"probe","command":"advance_time","millis":16}"#;
        let response = handle_automation_json_line(line, |command| match command {
            AutomationCommand::AdvanceTime { millis } => Ok(AutomationResult::Advanced { millis }),
            _ => unreachable!(),
        })
        .unwrap();
        let decoded: AutomationResponse = serde_json::from_str(&response).unwrap();
        assert!(decoded.ok);
        assert_eq!(decoded.id, serde_json::Value::String("probe".to_owned()));

        let malformed = handle_automation_json_line("not-json", |_| unreachable!()).unwrap();
        let decoded: AutomationResponse = serde_json::from_str(&malformed).unwrap();
        assert!(!decoded.ok);
        assert_eq!(decoded.id, serde_json::Value::Null);
    }
}
