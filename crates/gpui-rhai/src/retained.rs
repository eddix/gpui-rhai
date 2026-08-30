use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use thiserror::Error;

use crate::{NodeKey, UiNode, UiNodeKindTag};

/// Stable runtime identity for one accepted declarative node.
///
/// IDs are monotonic within one [`RetainedUiTree`] and are never reused, so a
/// stale reference cannot silently bind to a later node.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(u64);

impl NodeId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedChildLink {
    group: String,
    node: NodeId,
}

impl RetainedChildLink {
    #[must_use]
    pub fn group(&self) -> &str {
        &self.group
    }

    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.node
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetainedNode {
    id: NodeId,
    parent: Option<NodeId>,
    key: Option<String>,
    kind: UiNodeKindTag,
    component_root: Option<crate::ComponentInstancePath>,
    element_ref: Option<crate::ElementRef>,
    handlers: BTreeMap<String, Vec<crate::UiEventBinding>>,
    handler_payloads: BTreeMap<String, crate::UiValue>,
    children: Vec<RetainedChildLink>,
}

impl RetainedNode {
    #[must_use]
    pub const fn id(&self) -> NodeId {
        self.id
    }

    #[must_use]
    pub const fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    #[must_use]
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    #[must_use]
    pub const fn kind(&self) -> UiNodeKindTag {
        self.kind
    }

    #[must_use]
    pub fn component_root(&self) -> Option<&crate::ComponentInstancePath> {
        self.component_root.as_ref()
    }

    #[must_use]
    pub const fn element_ref(&self) -> Option<&crate::ElementRef> {
        self.element_ref.as_ref()
    }

    #[must_use]
    pub fn event_handlers(&self, event: &str) -> &[crate::UiEventBinding] {
        self.handlers.get(event).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn handler_payload(&self, event: &str) -> Option<&crate::UiValue> {
        self.handler_payloads.get(event)
    }

    pub fn children(&self) -> impl ExactSizeIterator<Item = &RetainedChildLink> {
        self.children.iter()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReconcileReport {
    pub mounted: Vec<NodeId>,
    pub preserved: Vec<NodeId>,
    pub moved: Vec<NodeId>,
    pub unmounted: Vec<NodeId>,
}

impl ReconcileReport {
    fn sort(&mut self) {
        self.mounted.sort_unstable();
        self.preserved.sort_unstable();
        self.moved.sort_unstable();
        self.unmounted.sort_unstable();
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ReconcileError {
    #[error("node ID space is exhausted")]
    NodeIdExhausted,
    #[error("retained root state is inconsistent")]
    RootStateMismatch,
    #[error("retained node {0} is missing")]
    MissingNode(NodeId),
    #[error("retained child structure for node {0} is inconsistent with its snapshot")]
    InconsistentSnapshot(NodeId),
    #[error("duplicate key `{key}` in child group `{group}` below node {parent}")]
    DuplicateKey {
        parent: NodeId,
        group: String,
        key: String,
    },
}

/// One accepted declarative snapshot plus its stable runtime identity graph.
///
/// Reconciliation builds a complete candidate map and only replaces live state
/// after validation succeeds. The snapshot owns node payloads once; retained
/// entries contain identity/edge metadata rather than duplicated subtrees.
#[derive(Clone, Debug)]
pub struct RetainedUiTree {
    root: Option<UiNode>,
    root_id: Option<NodeId>,
    nodes: BTreeMap<NodeId, RetainedNode>,
    next_id: u64,
}

impl Default for RetainedUiTree {
    fn default() -> Self {
        Self {
            root: None,
            root_id: None,
            nodes: BTreeMap::new(),
            next_id: 1,
        }
    }
}

impl RetainedUiTree {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn root(&self) -> Option<&UiNode> {
        self.root.as_ref()
    }

    #[must_use]
    pub const fn root_id(&self) -> Option<NodeId> {
        self.root_id
    }

    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&RetainedNode> {
        self.nodes.get(&id)
    }

    #[must_use]
    pub fn component_node(&self, component: &crate::ComponentInstancePath) -> Option<NodeId> {
        self.nodes
            .values()
            .find_map(|node| (node.component_root.as_ref() == Some(component)).then_some(node.id))
    }

    pub fn nodes(&self) -> impl ExactSizeIterator<Item = &RetainedNode> {
        self.nodes.values()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Reconcile and atomically accept one complete candidate snapshot.
    ///
    /// # Errors
    ///
    /// Returns a structural error without changing the current snapshot, IDs,
    /// or allocation cursor.
    pub fn reconcile(&mut self, candidate: UiNode) -> Result<ReconcileReport, ReconcileError> {
        let mut transaction = ReconcileTransaction {
            old_nodes: &self.nodes,
            new_nodes: BTreeMap::new(),
            next_id: self.next_id,
            report: ReconcileReport::default(),
            reused: BTreeSet::new(),
        };
        let root_id = match (self.root.as_ref(), self.root_id) {
            (Some(old), Some(old_id)) if reusable(old, &candidate) => {
                transaction.reconcile_node(Some(old), Some(old_id), &candidate, None, false)?
            }
            (Some(_), Some(old_id)) => {
                transaction.collect_unmounted(old_id)?;
                transaction.reconcile_node(None, None, &candidate, None, false)?
            }
            (None, None) => transaction.reconcile_node(None, None, &candidate, None, false)?,
            _ => return Err(ReconcileError::RootStateMismatch),
        };
        for old_id in self.nodes.keys().copied() {
            if !transaction.reused.contains(&old_id)
                && !transaction.report.unmounted.contains(&old_id)
            {
                transaction.report.unmounted.push(old_id);
            }
        }
        transaction.report.sort();

        let ReconcileTransaction {
            new_nodes,
            next_id,
            report,
            ..
        } = transaction;

        self.root = Some(candidate);
        self.root_id = Some(root_id);
        self.nodes = new_nodes;
        self.next_id = next_id;
        Ok(report)
    }
}

struct ReconcileTransaction<'a> {
    old_nodes: &'a BTreeMap<NodeId, RetainedNode>,
    new_nodes: BTreeMap<NodeId, RetainedNode>,
    next_id: u64,
    report: ReconcileReport,
    reused: BTreeSet<NodeId>,
}

impl ReconcileTransaction<'_> {
    fn reconcile_node(
        &mut self,
        old_snapshot: Option<&UiNode>,
        old_id: Option<NodeId>,
        candidate: &UiNode,
        parent: Option<NodeId>,
        moved: bool,
    ) -> Result<NodeId, ReconcileError> {
        let id = if let Some(id) = old_id {
            self.reused.insert(id);
            self.report.preserved.push(id);
            if moved {
                self.report.moved.push(id);
            }
            id
        } else {
            self.allocate()?
        };
        let old_node = old_id
            .map(|id| {
                self.old_nodes
                    .get(&id)
                    .ok_or(ReconcileError::MissingNode(id))
            })
            .transpose()?;
        let old_groups = match (old_snapshot, old_node) {
            (Some(snapshot), Some(node)) => group_old_children(snapshot, node)?,
            (None, None) => BTreeMap::new(),
            _ => return Err(ReconcileError::InconsistentSnapshot(id)),
        };
        let mut child_links = Vec::new();
        let mut seen_groups = BTreeSet::new();
        for (group, candidates) in candidate.retained_child_groups() {
            if !seen_groups.insert(group.clone()) {
                return Err(ReconcileError::InconsistentSnapshot(id));
            }
            validate_unique_keys(id, &group, &candidates)?;
            let old = old_groups.get(group.as_str()).cloned().unwrap_or_default();
            let mut used_old = BTreeSet::new();
            let keyed = old
                .iter()
                .filter_map(|child| {
                    child
                        .snapshot
                        .key()
                        .map(|key| (key.as_str().to_owned(), child))
                })
                .collect::<BTreeMap<_, _>>();
            for (index, child) in candidates.into_iter().enumerate() {
                let selected = child.key().and_then(|key| {
                    keyed
                        .get(key.as_str())
                        .filter(|old| old.snapshot.kind_tag() == child.kind_tag())
                        .copied()
                });
                let selected = selected.or_else(|| {
                    child
                        .key()
                        .is_none()
                        .then(|| old.get(index))
                        .flatten()
                        .filter(|old| old.snapshot.key().is_none() && reusable(old.snapshot, child))
                });
                let (old_snapshot, old_child_id, was_moved) =
                    selected.map_or((None, None, false), |selected| {
                        used_old.insert(selected.node);
                        (
                            Some(selected.snapshot),
                            Some(selected.node),
                            selected.index != index,
                        )
                    });
                let child_id =
                    self.reconcile_node(old_snapshot, old_child_id, child, Some(id), was_moved)?;
                child_links.push(RetainedChildLink {
                    group: group.clone(),
                    node: child_id,
                });
            }
            for old_child in old {
                if !used_old.contains(&old_child.node) {
                    self.collect_unmounted(old_child.node)?;
                }
            }
        }
        for (group, old) in old_groups {
            if !seen_groups.contains(&group) {
                for old_child in old {
                    self.collect_unmounted(old_child.node)?;
                }
            }
        }
        self.new_nodes.insert(
            id,
            RetainedNode {
                id,
                parent,
                key: candidate.key().map(|key| key.as_str().to_owned()),
                kind: candidate.kind_tag(),
                component_root: candidate.component_root().cloned(),
                element_ref: candidate.element_ref().cloned(),
                handlers: candidate.handlers().clone(),
                handler_payloads: candidate.handler_payloads().clone(),
                children: child_links,
            },
        );
        Ok(id)
    }

    fn allocate(&mut self) -> Result<NodeId, ReconcileError> {
        let id = NodeId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(ReconcileError::NodeIdExhausted)?;
        self.report.mounted.push(id);
        Ok(id)
    }

    fn collect_unmounted(&mut self, id: NodeId) -> Result<(), ReconcileError> {
        if self.report.unmounted.contains(&id) {
            return Ok(());
        }
        let node = self
            .old_nodes
            .get(&id)
            .ok_or(ReconcileError::MissingNode(id))?;
        for child in &node.children {
            self.collect_unmounted(child.node)?;
        }
        self.report.unmounted.push(id);
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct OldChild<'a> {
    index: usize,
    node: NodeId,
    snapshot: &'a UiNode,
}

fn group_old_children<'a>(
    snapshot: &'a UiNode,
    node: &RetainedNode,
) -> Result<BTreeMap<String, Vec<OldChild<'a>>>, ReconcileError> {
    let mut links = node.children.iter();
    let mut result = BTreeMap::new();
    for (group, snapshots) in snapshot.retained_child_groups() {
        let mut children = Vec::with_capacity(snapshots.len());
        for (index, snapshot) in snapshots.into_iter().enumerate() {
            let link = links
                .next()
                .ok_or(ReconcileError::InconsistentSnapshot(node.id))?;
            if link.group != group {
                return Err(ReconcileError::InconsistentSnapshot(node.id));
            }
            children.push(OldChild {
                index,
                node: link.node,
                snapshot,
            });
        }
        result.insert(group, children);
    }
    if links.next().is_some() {
        return Err(ReconcileError::InconsistentSnapshot(node.id));
    }
    Ok(result)
}

fn validate_unique_keys(
    parent: NodeId,
    group: &str,
    children: &[&UiNode],
) -> Result<(), ReconcileError> {
    let mut keys = BTreeSet::new();
    if let Some(key) = children
        .iter()
        .filter_map(|node| node.key().map(NodeKey::as_str))
        .find(|key| !keys.insert((*key).to_owned()))
    {
        return Err(ReconcileError::DuplicateKey {
            parent,
            group: group.to_owned(),
            key: key.to_owned(),
        });
    }
    Ok(())
}

fn reusable(old: &UiNode, new: &UiNode) -> bool {
    old.kind_tag() == new.kind_tag()
        && old.key().map(NodeKey::as_str) == new.key().map(NodeKey::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyed_text(key: &str, text: &str) -> UiNode {
        UiNode::text(text).with_key(key)
    }

    #[test]
    fn keyed_reorder_preserves_ids_and_reports_moves() {
        let mut tree = RetainedUiTree::new();
        tree.reconcile(UiNode::column(vec![
            keyed_text("a", "A"),
            keyed_text("b", "B"),
        ]))
        .unwrap();
        let root = tree.root_id().unwrap();
        let before = tree
            .node(root)
            .unwrap()
            .children()
            .map(RetainedChildLink::node)
            .collect::<Vec<_>>();

        let report = tree
            .reconcile(UiNode::column(vec![
                keyed_text("b", "B2"),
                keyed_text("a", "A2"),
            ]))
            .unwrap();
        let after = tree
            .node(root)
            .unwrap()
            .children()
            .map(RetainedChildLink::node)
            .collect::<Vec<_>>();

        assert_eq!(after, vec![before[1], before[0]]);
        assert!(report.mounted.is_empty());
        assert!(report.unmounted.is_empty());
        assert_eq!(report.moved, before);
    }

    #[test]
    fn kind_change_replaces_only_the_changed_subtree() {
        let mut tree = RetainedUiTree::new();
        tree.reconcile(UiNode::column(vec![keyed_text("item", "A")]))
            .unwrap();
        let root = tree.root_id().unwrap();
        let old_child = tree.node(root).unwrap().children().next().unwrap().node();

        let report = tree
            .reconcile(UiNode::column(vec![
                UiNode::row(Vec::new()).with_key("item"),
            ]))
            .unwrap();
        let new_child = tree.node(root).unwrap().children().next().unwrap().node();

        assert_ne!(old_child, new_child);
        assert_eq!(report.mounted, vec![new_child]);
        assert_eq!(report.unmounted, vec![old_child]);
        assert!(report.preserved.contains(&root));
    }

    #[test]
    fn duplicate_key_rejects_candidate_without_consuming_ids() {
        let mut tree = RetainedUiTree::new();
        tree.reconcile(UiNode::column(vec![keyed_text("good", "A")]))
            .unwrap();
        let old_root = tree.root().unwrap().clone();
        let old_ids = tree.nodes().map(RetainedNode::id).collect::<Vec<_>>();

        let error = tree
            .reconcile(UiNode::column(vec![
                keyed_text("same", "A"),
                keyed_text("same", "B"),
            ]))
            .unwrap_err();
        assert!(matches!(error, ReconcileError::DuplicateKey { .. }));
        assert_eq!(tree.root(), Some(&old_root));
        assert_eq!(
            tree.nodes().map(RetainedNode::id).collect::<Vec<_>>(),
            old_ids
        );

        let report = tree
            .reconcile(UiNode::column(vec![keyed_text("next", "B")]))
            .unwrap();
        assert_eq!(report.mounted.len(), 1);
        assert_eq!(report.mounted[0].get(), 3);
    }

    #[test]
    fn named_child_groups_isolate_duplicate_slot_keys() {
        let mut tree = RetainedUiTree::new();
        let overlay = UiNode::overlay(
            keyed_text("shared", "trigger"),
            keyed_text("shared", "content"),
            crate::OverlayNodeSpec {
                id: crate::OverlayId::new("overlay"),
                parent: None,
                kind: crate::OverlayKind::Popover,
                placement: crate::OverlayPlacement::Bottom,
                open: true,
                gap: 0.0,
                modal: false,
                dismiss: crate::OverlayDismissPolicy {
                    escape: true,
                    outside: true,
                },
                tooltip_delays: None,
            },
        );
        tree.reconcile(overlay).unwrap();
        assert_eq!(tree.len(), 3);
    }
}
