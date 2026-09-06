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
    pub test_id: Option<String>,
    pub value: Option<UiValue>,
    pub checked: Option<UiValue>,
    pub pressed: Option<bool>,
    pub expanded: Option<bool>,
    pub orientation: Option<String>,
    pub value_min: Option<f64>,
    pub value_max: Option<f64>,
    pub current: Option<String>,
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
        Self::build(tree, geometry, false)
    }

    /// Build the semantic tree from nodes that participated in the latest
    /// committed GPUI presentation frame.
    ///
    /// # Errors
    ///
    /// Returns [`AccessibilityError::DuplicateSemanticId`] for ambiguous IDs
    /// among currently presented nodes.
    pub fn from_presented(
        tree: &RetainedUiTree,
        geometry: &GeometryRegistry,
    ) -> Result<Self, AccessibilityError> {
        Self::build(tree, geometry, true)
    }

    fn build(
        tree: &RetainedUiTree,
        geometry: &GeometryRegistry,
        presented_only: bool,
    ) -> Result<Self, AccessibilityError> {
        let labels = semantic_labels(tree, geometry, presented_only)?;
        let mut output = Self::default();
        if let Some(root) = tree.root_id() {
            visit_retained(
                tree,
                geometry,
                root,
                None,
                &labels,
                presented_only,
                &mut output,
            )?;
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

    pub fn find_by_test_id<'a>(
        &'a self,
        id: &'a str,
    ) -> impl Iterator<Item = &'a AccessibilityNode> + 'a {
        self.nodes
            .values()
            .filter(move |node| node.test_id.as_deref() == Some(id))
    }
}

fn semantic_labels(
    tree: &RetainedUiTree,
    geometry: &GeometryRegistry,
    presented_only: bool,
) -> Result<BTreeMap<String, String>, AccessibilityError> {
    let mut labels = BTreeMap::new();
    for node in tree.nodes() {
        if presented_only && !geometry.is_presented(node.id()) {
            continue;
        }
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
    presented_only: bool,
    output: &mut AccessibilityTree,
) -> Result<(), AccessibilityError> {
    let retained = tree
        .node(id)
        .ok_or(AccessibilityError::MissingRetainedNode(id))?;
    if presented_only && !geometry.is_presented(id) {
        return Ok(());
    }
    let semantic = semantic_node(tree, retained, semantic_parent, geometry, labels);
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
        visit_retained(
            tree,
            geometry,
            child.node(),
            next_parent,
            labels,
            presented_only,
            output,
        )?;
    }
    Ok(())
}

fn semantic_node(
    tree: &RetainedUiTree,
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
        test_id: string_attribute(node, "test_id"),
        value: node.attributes().get("value").cloned(),
        checked: node.attributes().get("checked").cloned(),
        pressed: optional_bool_attribute(node, "pressed"),
        expanded: optional_bool_attribute(node, "expanded"),
        orientation: string_attribute(node, "orientation"),
        value_min: float_attribute(node, "value_min"),
        value_max: float_attribute(node, "value_max"),
        current: string_attribute(node, "current"),
        disabled: bool_attribute(node, "disabled"),
        invalid: bool_attribute(node, "invalid"),
        required: bool_attribute(node, "required"),
        geometry: retained_geometry(tree, node, geometry),
        children: Vec::new(),
    })
}

fn retained_geometry(
    tree: &RetainedUiTree,
    node: &RetainedNode,
    geometry: &GeometryRegistry,
) -> Option<ElementGeometry> {
    let mut current = Some(node.id());
    while let Some(id) = current {
        if let Some(bounds) = geometry.get(id) {
            return Some(bounds);
        }
        current = tree.node(id).and_then(RetainedNode::parent);
    }
    None
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

fn optional_bool_attribute(node: &RetainedNode, name: &str) -> Option<bool> {
    match node.attributes().get(name) {
        Some(UiValue::Bool(value)) => Some(*value),
        _ => None,
    }
}

fn float_attribute(node: &RetainedNode, name: &str) -> Option<f64> {
    match node.attributes().get(name) {
        Some(UiValue::Float(value)) => Some(*value),
        Some(UiValue::Integer(value)) => value.to_string().parse().ok(),
        _ => None,
    }
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

    #[test]
    fn semantic_descendant_inherits_nearest_realized_geometry() {
        let root = crate::UiNode::box_node(vec![
            crate::UiNode::text("Virtual row")
                .with_key("row")
                .with_attribute("role", UiValue::String("option".to_owned())),
        ])
        .with_key("realized-root");
        let mut retained = RetainedUiTree::new();
        retained.reconcile(root).unwrap();
        let root_id = retained.root_id().unwrap();
        let bounds = crate::GeometryBounds::new(4.0, 8.0, 100.0, 24.0).unwrap();
        let geometry = GeometryRegistry::new();
        geometry.update(
            root_id,
            ElementGeometry {
                layout: bounds,
                visual: bounds,
                clip: None,
            },
        );

        let tree = AccessibilityTree::from_retained(&retained, &geometry).unwrap();
        assert_eq!(
            tree.find_by_role_and_name("option", "Virtual row")
                .next()
                .unwrap()
                .geometry,
            Some(ElementGeometry {
                layout: bounds,
                visual: bounds,
                clip: None,
            })
        );
    }

    #[test]
    fn presented_tree_excludes_retained_but_unpainted_subtrees() {
        let visible = crate::UiNode::text("Visible")
            .with_key("visible")
            .with_attribute("role", UiValue::String("button".to_owned()));
        let hidden = crate::UiNode::text("Hidden")
            .with_key("hidden")
            .with_attribute("role", UiValue::String("button".to_owned()));
        let mut retained = RetainedUiTree::new();
        retained
            .reconcile(crate::UiNode::box_node(vec![visible, hidden]))
            .unwrap();
        let root = retained.root_id().unwrap();
        let children = retained
            .node(root)
            .unwrap()
            .children()
            .map(crate::RetainedChildLink::node)
            .collect::<Vec<_>>();
        let geometry = GeometryRegistry::new();
        let bounds = crate::GeometryBounds::new(0.0, 0.0, 100.0, 24.0).unwrap();
        let presentation = ElementGeometry {
            layout: bounds,
            visual: bounds,
            clip: None,
        };
        geometry.update(root, presentation);
        geometry.update(children[0], presentation);

        let structural = AccessibilityTree::from_retained(&retained, &geometry).unwrap();
        assert_eq!(structural.nodes().len(), 2);
        let presented = AccessibilityTree::from_presented(&retained, &geometry).unwrap();
        assert_eq!(presented.nodes().len(), 1);
        assert!(
            presented
                .find_by_role_and_name("button", "Visible")
                .next()
                .is_some()
        );
        assert!(
            presented
                .find_by_role_and_name("button", "Hidden")
                .next()
                .is_none()
        );
    }

    #[test]
    fn semantic_tree_preserves_pressed_expanded_orientation_and_range_values() {
        let node = crate::UiNode::text("Volume")
            .with_attribute("role", UiValue::String("slider".to_owned()))
            .with_attribute("pressed", UiValue::Bool(false))
            .with_attribute("expanded", UiValue::Bool(true))
            .with_attribute("orientation", UiValue::String("horizontal".to_owned()))
            .with_attribute("value", UiValue::Float(40.0))
            .with_attribute("value_min", UiValue::Float(0.0))
            .with_attribute("value_max", UiValue::Float(100.0));
        let mut retained = RetainedUiTree::new();
        retained.reconcile(node).unwrap();
        let tree = AccessibilityTree::from_retained(&retained, &GeometryRegistry::new()).unwrap();
        let node = tree
            .find_by_role_and_name("slider", "Volume")
            .next()
            .unwrap();
        assert_eq!(node.pressed, Some(false));
        assert_eq!(node.expanded, Some(true));
        assert_eq!(node.orientation.as_deref(), Some("horizontal"));
        assert_eq!(node.value, Some(UiValue::Float(40.0)));
        assert_eq!(node.value_min, Some(0.0));
        assert_eq!(node.value_max, Some(100.0));
    }
}
