use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::{ElementGeometry, GeometryRegistry, NodeId, RetainedNode, RetainedUiTree, UiValue};

#[derive(Clone, Debug, PartialEq)]
pub struct AccessibilityNode {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub role: String,
    pub name: String,
    pub description: String,
    pub semantic_id: Option<String>,
    pub value: Option<UiValue>,
    pub checked: Option<UiValue>,
    pub disabled: bool,
    pub invalid: bool,
    pub required: bool,
    pub geometry: Option<ElementGeometry>,
    pub children: Vec<NodeId>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AccessibilityTree {
    roots: Vec<NodeId>,
    nodes: BTreeMap<NodeId, AccessibilityNode>,
    semantic_ids: BTreeMap<String, NodeId>,
}

impl AccessibilityTree {
    /// Build a stable semantic tree from retained identity and committed geometry.
    ///
    /// Nodes without semantics are flattened; their semantic descendants attach
    /// to the nearest semantic ancestor.
    ///
    /// # Errors
    ///
    /// Returns [`AccessibilityError::DuplicateSemanticId`] for ambiguous IDs.
    pub fn from_retained(
        tree: &RetainedUiTree,
        geometry: &GeometryRegistry,
    ) -> Result<Self, AccessibilityError> {
        let labels = semantic_labels(tree)?;
        let mut output = Self::default();
        if let Some(root) = tree.root_id() {
            visit_retained(tree, geometry, root, None, &labels, &mut output)?;
        }
        Ok(output)
    }

    #[must_use]
    pub fn root_ids(&self) -> &[NodeId] {
        &self.roots
    }

    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&AccessibilityNode> {
        self.nodes.get(&id)
    }

    pub fn nodes(&self) -> impl ExactSizeIterator<Item = &AccessibilityNode> {
        self.nodes.values()
    }

    #[must_use]
    pub fn find_by_semantic_id(&self, id: &str) -> Option<&AccessibilityNode> {
        self.semantic_ids
            .get(id)
            .and_then(|node| self.nodes.get(node))
    }

    pub fn find_by_role_and_name<'a>(
        &'a self,
        role: &'a str,
        name: &'a str,
    ) -> impl Iterator<Item = &'a AccessibilityNode> + 'a {
        self.nodes
            .values()
            .filter(move |node| node.role == role && node.name == name)
    }
}

fn semantic_labels(tree: &RetainedUiTree) -> Result<BTreeMap<String, String>, AccessibilityError> {
    let mut labels = BTreeMap::new();
    for node in tree.nodes() {
        let Some(id) = string_attribute(node, "semantic_id") else {
            continue;
        };
        let label = string_attribute(node, "label")
            .or_else(|| node.text().map(ToOwned::to_owned))
            .unwrap_or_default();
        if labels.insert(id.clone(), label).is_some() {
            return Err(AccessibilityError::DuplicateSemanticId(id));
        }
    }
    Ok(labels)
}

fn visit_retained(
    tree: &RetainedUiTree,
    geometry: &GeometryRegistry,
    id: NodeId,
    semantic_parent: Option<NodeId>,
    labels: &BTreeMap<String, String>,
    output: &mut AccessibilityTree,
) -> Result<(), AccessibilityError> {
    let retained = tree
        .node(id)
        .ok_or(AccessibilityError::MissingRetainedNode(id))?;
    let semantic = semantic_node(retained, semantic_parent, geometry, labels);
    let next_parent = if let Some(node) = semantic {
        if semantic_parent.is_none() {
            output.roots.push(id);
        }
        if let Some(parent) = semantic_parent
            && let Some(parent) = output.nodes.get_mut(&parent)
        {
            parent.children.push(id);
        }
        if let Some(semantic_id) = &node.semantic_id
            && output
                .semantic_ids
                .insert(semantic_id.clone(), id)
                .is_some()
        {
            return Err(AccessibilityError::DuplicateSemanticId(semantic_id.clone()));
        }
        output.nodes.insert(id, node);
        Some(id)
    } else {
        semantic_parent
    };
    for child in retained.children() {
        visit_retained(tree, geometry, child.node(), next_parent, labels, output)?;
    }
    Ok(())
}

fn semantic_node(
    node: &RetainedNode,
    parent: Option<NodeId>,
    geometry: &GeometryRegistry,
    labels: &BTreeMap<String, String>,
) -> Option<AccessibilityNode> {
    let role = string_attribute(node, "role").or_else(|| node.text().map(|_| "text".to_owned()))?;
    let explicit_name = string_attribute(node, "label");
    let name = explicit_name
        .or_else(|| referenced_text(node, "labelled_by", labels))
        .or_else(|| node.text().map(ToOwned::to_owned))
        .unwrap_or_default();
    Some(AccessibilityNode {
        id: node.id(),
        parent,
        role,
        name,
        description: referenced_text(node, "described_by", labels).unwrap_or_default(),
        semantic_id: string_attribute(node, "semantic_id"),
        value: node.attributes().get("value").cloned(),
        checked: node.attributes().get("checked").cloned(),
        disabled: bool_attribute(node, "disabled"),
        invalid: bool_attribute(node, "invalid"),
        required: bool_attribute(node, "required"),
        geometry: geometry.get(node.id()),
        children: Vec::new(),
    })
}

fn referenced_text(
    node: &RetainedNode,
    attribute: &str,
    labels: &BTreeMap<String, String>,
) -> Option<String> {
    let ids = string_attribute(node, attribute)?;
    let mut seen = BTreeSet::new();
    let text = ids
        .split_whitespace()
        .filter(|id| seen.insert((*id).to_owned()))
        .filter_map(|id| labels.get(id))
        .filter(|label| !label.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    (!text.is_empty()).then_some(text)
}

fn string_attribute(node: &RetainedNode, name: &str) -> Option<String> {
    match node.attributes().get(name) {
        Some(UiValue::String(value)) => Some(value.clone()),
        _ => None,
    }
}

fn bool_attribute(node: &RetainedNode, name: &str) -> bool {
    node.attributes().get(name) == Some(&UiValue::Bool(true))
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum AccessibilityError {
    #[error("semantic ID `{0}` is declared more than once")]
    DuplicateSemanticId(String),
    #[error("retained accessibility traversal lost node {0}")]
    MissingRetainedNode(NodeId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_tree_flattens_layout_and_resolves_label_relationships() {
        let label = crate::UiNode::text("Project name")
            .with_key("label")
            .with_attribute("semantic_id", UiValue::String("project-label".to_owned()))
            .with_attribute("role", UiValue::String("label".to_owned()));
        let input = crate::UiNode::text("value")
            .with_key("input")
            .with_attribute("role", UiValue::String("text_field".to_owned()))
            .with_attribute("labelled_by", UiValue::String("project-label".to_owned()))
            .with_attribute("required", UiValue::Bool(true));
        let mut retained = RetainedUiTree::new();
        retained
            .reconcile(crate::UiNode::box_node(vec![label, input]))
            .unwrap();
        let tree = AccessibilityTree::from_retained(&retained, &GeometryRegistry::new()).unwrap();
        let field = tree
            .find_by_role_and_name("text_field", "Project name")
            .next()
            .unwrap();
        assert!(field.required);
        assert_eq!(tree.nodes().len(), 2);
        assert_eq!(
            tree.find_by_semantic_id("project-label").unwrap().name,
            "Project name"
        );
    }
}
