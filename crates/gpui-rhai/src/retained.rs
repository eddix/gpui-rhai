use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;

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

    /// The id whose [`Self::get`] is `value`, as a motion path names it.
    pub(crate) const fn from_raw(value: u64) -> Self {
        Self(value)
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
    primitive: Option<crate::PrimitiveId>,
    component_root: Option<crate::ComponentInstancePath>,
    forwarded_components: Option<std::rc::Rc<Vec<crate::ComponentInstancePath>>>,
    element_ref: Option<crate::ElementRef>,
    attributes: BTreeMap<String, crate::UiValue>,
    text: Option<String>,
    handlers: BTreeMap<String, Vec<crate::UiEventBinding>>,
    handler_payloads: BTreeMap<String, crate::UiValue>,
    scrollable: bool,
    focus_styled: bool,
    canvas_commands: usize,
    canvas_scene: Option<crate::CanvasScene>,
    virtual_data_items: usize,
    virtual_realized_items: usize,
    children: Vec<RetainedChildLink>,
    summary: SubtreeSummary,
}

/// What a retained subtree holds, kept on its root for the render path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SubtreeSummary {
    /// Nodes in the subtree, its root included.
    pub nodes: u32,
    /// Nodes that a cached island cannot hold
    /// ([`crate::renderer::island_hot`]).
    pub hot: u32,
    /// Nodes with a `group_hover` style, painted by an ancestor's hover.
    pub group_hover: u32,
    /// Nodes a commit's motion phase has to visit
    /// ([`crate::motion::node_takes_motion`]).
    pub motion: u32,
    /// Nodes with exit motions, which leave a ghost when removed.
    pub exit: u32,
    /// Nodes that read a native signal: a binding, a signal style or a
    /// signal-triggered progress motion.
    pub signals: u32,
    /// Virtual collections.
    pub collections: u32,
    /// What the commit's resource budgets count.
    pub resources: ResourceTotals,
    /// The root itself lays out or moves its children in ways an island
    /// below it cannot follow: Table layout or a scroll container.
    pub opaque: bool,
}

/// The amounts of a subtree that a commit checks against its resource
/// budgets, each summed with saturation as the check sums them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ResourceTotals {
    pub handlers: usize,
    pub layers: usize,
    pub canvas_scenes: usize,
    pub canvas_commands: usize,
    pub virtual_data_items: usize,
    pub virtual_realized_items: usize,
}

impl ResourceTotals {
    fn of(node: &RetainedNode) -> Self {
        Self {
            handlers: node.handler_count(),
            layers: usize::from(node.kind == UiNodeKindTag::Layer),
            canvas_scenes: usize::from(node.kind == UiNodeKindTag::Canvas),
            canvas_commands: node.canvas_commands,
            virtual_data_items: node.virtual_data_items,
            virtual_realized_items: node.virtual_realized_items,
        }
    }

    const fn add(self, other: Self) -> Self {
        Self {
            handlers: self.handlers.saturating_add(other.handlers),
            layers: self.layers.saturating_add(other.layers),
            canvas_scenes: self.canvas_scenes.saturating_add(other.canvas_scenes),
            canvas_commands: self.canvas_commands.saturating_add(other.canvas_commands),
            virtual_data_items: self
                .virtual_data_items
                .saturating_add(other.virtual_data_items),
            virtual_realized_items: self
                .virtual_realized_items
                .saturating_add(other.virtual_realized_items),
        }
    }
}

impl SubtreeSummary {
    fn of(node: &UiNode) -> Self {
        Self {
            nodes: 1,
            hot: u32::from(crate::renderer::island_hot(node)),
            group_hover: u32::from(node.style().group_hover.is_some()),
            motion: u32::from(crate::motion::node_takes_motion(node)),
            exit: u32::from(!node.exit_motions().is_empty()),
            signals: u32::from(
                node.signal_bindings().next().is_some()
                    || node.signal_style().is_some()
                    || node
                        .progress_motions()
                        .iter()
                        .any(|binding| binding.signal_trigger.is_some()),
            ),
            collections: u32::from(matches!(
                node.kind(),
                crate::UiNodeKind::VirtualCollection { .. }
            )),
            resources: ResourceTotals::default(),
            opaque: node.table_layout().is_some() || crate::renderer::node_scrollable(node),
        }
    }

    fn add(self, other: Self) -> Self {
        Self {
            nodes: self.nodes.saturating_add(other.nodes),
            hot: self.hot.saturating_add(other.hot),
            group_hover: self.group_hover.saturating_add(other.group_hover),
            motion: self.motion.saturating_add(other.motion),
            exit: self.exit.saturating_add(other.exit),
            signals: self.signals.saturating_add(other.signals),
            collections: self.collections.saturating_add(other.collections),
            resources: self.resources.add(other.resources),
            opaque: self.opaque,
        }
    }
}

impl RetainedNode {
    #[must_use]
    pub const fn id(&self) -> NodeId {
        self.id
    }

    /// What this node's subtree holds.
    pub(crate) const fn summary(&self) -> SubtreeSummary {
        self.summary
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
    pub const fn primitive(&self) -> Option<&crate::PrimitiveId> {
        self.primitive.as_ref()
    }

    #[must_use]
    pub fn component_root(&self) -> Option<&crate::ComponentInstancePath> {
        self.component_root.as_ref()
    }

    pub(crate) fn presented_components(
        &self,
    ) -> impl Iterator<Item = &crate::ComponentInstancePath> {
        self.component_root.iter().chain(
            self.forwarded_components
                .iter()
                .flat_map(|owners| owners.iter()),
        )
    }

    #[must_use]
    pub const fn element_ref(&self) -> Option<&crate::ElementRef> {
        self.element_ref.as_ref()
    }

    #[must_use]
    pub fn attributes(&self) -> &BTreeMap<String, crate::UiValue> {
        &self.attributes
    }

    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    #[must_use]
    pub fn event_handlers(&self, event: &str) -> &[crate::UiEventBinding] {
        self.handlers.get(event).map_or(&[], Vec::as_slice)
    }

    /// The payload set for `event` on the node, else the value of its last
    /// handler declared with one.
    #[must_use]
    pub fn handler_payload(&self, event: &str) -> Option<&crate::UiValue> {
        self.handler_payloads.get(event).or_else(|| {
            self.event_handlers(event)
                .iter()
                .rev()
                .find_map(crate::UiEventBinding::value)
        })
    }

    /// The payload set for `event` on the node itself, which handlers without
    /// their own value receive.
    #[must_use]
    pub(crate) fn node_payload(&self, event: &str) -> Option<&crate::UiValue> {
        self.handler_payloads.get(event)
    }

    #[must_use]
    pub fn handler_count(&self) -> usize {
        self.handlers.values().map(Vec::len).sum()
    }

    #[must_use]
    pub const fn scrollable(&self) -> bool {
        self.scrollable
    }

    #[must_use]
    pub const fn focus_styled(&self) -> bool {
        self.focus_styled
    }

    #[must_use]
    pub const fn canvas_command_count(&self) -> usize {
        self.canvas_commands
    }

    #[must_use]
    pub const fn canvas_scene(&self) -> Option<&crate::CanvasScene> {
        self.canvas_scene.as_ref()
    }

    #[must_use]
    pub const fn virtual_data_item_count(&self) -> usize {
        self.virtual_data_items
    }

    #[must_use]
    pub const fn virtual_realized_item_count(&self) -> usize {
        self.virtual_realized_items
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReconcileMetrics {
    pub mounted: usize,
    pub preserved: usize,
    pub moved: usize,
    pub unmounted: usize,
}

impl ReconcileReport {
    fn sort(&mut self) {
        self.mounted.sort_unstable();
        self.preserved.sort_unstable();
        self.moved.sort_unstable();
        self.unmounted.sort_unstable();
    }

    #[must_use]
    pub fn metrics(&self) -> ReconcileMetrics {
        ReconcileMetrics {
            mounted: self.mounted.len(),
            preserved: self.preserved.len(),
            moved: self.moved.len(),
            unmounted: self.unmounted.len(),
        }
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
    #[error("fragment nodes may contain only children, source, and an optional key")]
    FragmentDecoration,
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
    /// Shared with the trees an unchanged subtree was kept from, and with
    /// their clones.
    nodes: BTreeMap<NodeId, Rc<RetainedNode>>,
    next_id: u64,
    last_report: ReconcileReport,
}

impl Default for RetainedUiTree {
    fn default() -> Self {
        Self {
            root: None,
            root_id: None,
            nodes: BTreeMap::new(),
            next_id: 1,
            last_report: ReconcileReport::default(),
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
        self.nodes.get(&id).map(|node| &**node)
    }

    /// Read one accepted declarative snapshot by retained identity. Walking
    /// just its ancestor path avoids scanning unrelated subtrees when an
    /// active native gesture validates its contract after a script commit.
    pub(crate) fn snapshot_node(&self, id: NodeId) -> Option<&UiNode> {
        let mut path = Vec::new();
        let mut current = id;
        while let Some(parent) = self.node(current)?.parent() {
            path.push(current);
            current = parent;
        }
        if Some(current) != self.root_id {
            return None;
        }
        let mut snapshot = self.root.as_ref()?;
        for child in path.into_iter().rev() {
            let retained = self.node(current)?;
            snapshot = snapshot
                .retained_child_groups()
                .into_iter()
                .flat_map(|(_, children)| children)
                .zip(retained.children())
                .find_map(|(snapshot, link)| (link.node() == child).then_some(snapshot))?;
            current = child;
        }
        Some(snapshot)
    }

    #[must_use]
    pub fn component_node(&self, component: &crate::ComponentInstancePath) -> Option<NodeId> {
        self.nodes
            .values()
            .find_map(|node| (node.component_root.as_ref() == Some(component)).then_some(node.id))
    }

    pub fn nodes(&self) -> impl ExactSizeIterator<Item = &RetainedNode> {
        self.nodes.values().map(|node| &**node)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// The resource totals summed over every node, as the summaries keep
    /// them.
    pub(crate) fn recounted_resources(&self) -> ResourceTotals {
        self.nodes()
            .fold(ResourceTotals::default(), |totals, node| {
                totals.add(ResourceTotals::of(node))
            })
    }

    /// The summary of the whole tree, empty without a root.
    pub(crate) fn summary(&self) -> SubtreeSummary {
        self.root_id
            .and_then(|id| self.node(id))
            .map(RetainedNode::summary)
            .unwrap_or_default()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[must_use]
    pub const fn last_report(&self) -> &ReconcileReport {
        &self.last_report
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
            unmounted: BTreeSet::new(),
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
            if !transaction.reused.contains(&old_id) && transaction.unmounted.insert(old_id) {
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
        self.last_report = report.clone();
        Ok(report)
    }
}

struct ReconcileTransaction<'a> {
    old_nodes: &'a BTreeMap<NodeId, Rc<RetainedNode>>,
    new_nodes: BTreeMap<NodeId, Rc<RetainedNode>>,
    next_id: u64,
    report: ReconcileReport,
    reused: BTreeSet<NodeId>,
    /// The ids in `report.unmounted`.
    unmounted: BTreeSet<NodeId>,
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
        // The same snapshot accepted again keeps its subtree: reconciling it
        // would match every node to its old id and rebuild what it holds.
        if let (Some(old_snapshot), Some(old_id)) = (old_snapshot, old_id)
            && old_snapshot.ptr_eq(candidate)
        {
            self.keep_subtree(old_id, moved)?;
            return Ok(old_id);
        }
        validate_fragment(candidate)?;
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
        self.insert_candidate(id, parent, candidate, child_links);
        Ok(id)
    }

    fn insert_candidate(
        &mut self,
        id: NodeId,
        parent: Option<NodeId>,
        candidate: &UiNode,
        children: Vec<RetainedChildLink>,
    ) {
        let mut node = RetainedNode {
            id,
            parent,
            key: candidate.key().map(|key| key.as_str().to_owned()),
            kind: candidate.kind_tag(),
            primitive: match candidate.kind() {
                crate::UiNodeKind::Custom { primitive } => Some(primitive.primitive.clone()),
                _ => None,
            },
            component_root: candidate.component_root().cloned(),
            forwarded_components: candidate.forwarded_components().cloned(),
            element_ref: candidate.element_ref().cloned(),
            attributes: candidate.attributes().clone(),
            text: match candidate.kind() {
                crate::UiNodeKind::Text { text } | crate::UiNodeKind::RichText { text, .. } => {
                    Some(text.to_string())
                }
                _ => None,
            },
            handlers: candidate.handlers().clone(),
            handler_payloads: candidate.handler_payloads().clone(),
            scrollable: snapshot_scrollable(candidate),
            focus_styled: candidate.style().focus.is_some(),
            canvas_commands: match candidate.kind() {
                crate::UiNodeKind::Canvas { scene } => scene.complexity(),
                _ => 0,
            },
            canvas_scene: match candidate.kind() {
                crate::UiNodeKind::Canvas { scene } => Some(scene.clone()),
                _ => None,
            },
            virtual_data_items: match candidate.kind() {
                crate::UiNodeKind::VirtualCollection { spec } => spec.data.len(),
                _ => 0,
            },
            virtual_realized_items: match candidate.kind() {
                crate::UiNodeKind::VirtualCollection { spec } => spec.realized.len(),
                _ => 0,
            },
            children,
            summary: SubtreeSummary::default(),
        };
        // Children are inserted before their parent.
        node.summary = node
            .children
            .iter()
            .filter_map(|link| self.new_nodes.get(&link.node))
            .fold(
                SubtreeSummary {
                    resources: ResourceTotals::of(&node),
                    ..SubtreeSummary::of(candidate)
                },
                |summary, child| summary.add(child.summary),
            );
        self.new_nodes.insert(id, Rc::new(node));
    }

    /// Keeps the old subtree at `root` as it is, as a reconcile of the same
    /// snapshot would.
    fn keep_subtree(&mut self, root: NodeId, moved: bool) -> Result<(), ReconcileError> {
        if moved {
            self.report.moved.push(root);
        }
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            let node = self
                .old_nodes
                .get(&id)
                .ok_or(ReconcileError::MissingNode(id))?;
            self.reused.insert(id);
            self.report.preserved.push(id);
            stack.extend(node.children.iter().map(|child| child.node));
            self.new_nodes.insert(id, Rc::clone(node));
        }
        Ok(())
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
        if self.unmounted.contains(&id) {
            return Ok(());
        }
        let node = self
            .old_nodes
            .get(&id)
            .ok_or(ReconcileError::MissingNode(id))?;
        for child in &node.children {
            self.collect_unmounted(child.node)?;
        }
        self.unmounted.insert(id);
        self.report.unmounted.push(id);
        Ok(())
    }
}

fn snapshot_scrollable(node: &UiNode) -> bool {
    matches!(
        node.style().base.overflow_x,
        Some(crate::OverflowMode::Scroll)
    ) || matches!(
        node.style().base.overflow_y,
        Some(crate::OverflowMode::Scroll)
    )
}

fn validate_fragment(node: &UiNode) -> Result<(), ReconcileError> {
    if matches!(node.kind(), crate::UiNodeKind::Fragment { .. })
        && (node.style() != &crate::Style::new()
            || node.part_styles().next().is_some()
            || !node.attributes().is_empty()
            || !node.handlers().is_empty()
            || !node.motions().is_empty()
            || !node.exit_motions().is_empty()
            || !node.progress_motions().is_empty()
            || !node.timelines().is_empty()
            || node.signal_bindings().next().is_some()
            || node.element_ref().is_some())
    {
        Err(ReconcileError::FragmentDecoration)
    } else {
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
        assert_eq!(report.mounted, [] as [crate::retained::NodeId; 0]);
        assert_eq!(report.unmounted, [] as [crate::retained::NodeId; 0]);
        assert_eq!(report.moved, before);
    }

    #[test]
    fn a_shared_subtree_is_kept_as_a_rebuilt_one_would_be() {
        let block = |label: &str| {
            UiNode::column(
                (0..4)
                    .map(|index| {
                        UiNode::row(vec![
                            UiNode::text(format!("{label} {index}")),
                            keyed_text("detail", "detail"),
                        ])
                        .with_key(format!("r{index}"))
                    })
                    .collect(),
            )
        };
        let page = |shared: &UiNode, moved: bool, extra: &str| {
            let mut children = vec![shared.clone(), keyed_text("extra", extra)];
            if moved {
                children.reverse();
            }
            UiNode::column(children)
        };
        let shared = block("Row").with_key("shared");
        let mut kept = RetainedUiTree::new();
        let mut rebuilt = RetainedUiTree::new();
        kept.reconcile(page(&shared, false, "one")).unwrap();
        rebuilt
            .reconcile(page(&block("Row").with_key("shared"), false, "one"))
            .unwrap();
        for (moved, extra) in [(false, "two"), (true, "three"), (true, "three")] {
            let kept_report = kept.reconcile(page(&shared, moved, extra)).unwrap();
            let rebuilt_report = rebuilt
                .reconcile(page(&block("Row").with_key("shared"), moved, extra))
                .unwrap();
            assert_eq!(kept_report, rebuilt_report);
            assert!(kept.nodes().eq(rebuilt.nodes()));
        }
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
        let old_report = tree.last_report().clone();

        let error = tree
            .reconcile(UiNode::column(vec![
                keyed_text("same", "A"),
                keyed_text("same", "B"),
            ]))
            .unwrap_err();
        assert!(matches!(error, ReconcileError::DuplicateKey { .. }));
        assert_eq!(tree.root(), Some(&old_root));
        assert_eq!(tree.last_report(), &old_report);
        assert_eq!(
            tree.nodes().map(RetainedNode::id).collect::<Vec<_>>(),
            old_ids
        );

        let report = tree
            .reconcile(UiNode::column(vec![keyed_text("next", "B")]))
            .unwrap();
        assert_eq!(report.mounted.len(), 1);
        assert_eq!(report.mounted[0].get(), 3);
        assert_eq!(tree.last_report().metrics(), report.metrics());
    }

    #[test]
    fn named_child_groups_isolate_duplicate_slot_keys() {
        let mut tree = RetainedUiTree::new();
        let overlay = UiNode::overlay(
            keyed_text("shared", "trigger"),
            keyed_text("shared", "content"),
            crate::OverlayNodeSpec {
                id: crate::OverlayId::new("overlay"),
                owner: None,
                parent: None,
                kind: crate::OverlayKind::Popover,
                initial_focus: crate::OverlayInitialFocus::Panel,
                placement: crate::OverlayPlacement::Bottom,
                align: crate::OverlayAlign::Center,
                anchor: None,
                open: true,
                gap: 0.0,
                modal: false,
                dismiss: crate::OverlayDismissPolicy {
                    escape: true,
                    outside: true,
                },
                tooltip_delays: None,
                activate_on_trigger: true,
                width_policy: crate::OverlayWidthPolicy::Content,
            },
        );
        tree.reconcile(overlay).unwrap();
        assert_eq!(tree.len(), 3);
    }

    #[test]
    fn fragment_rejects_layout_or_interaction_decoration() {
        let mut tree = RetainedUiTree::new();
        let decorated = UiNode::fragment(vec![UiNode::text("child")])
            .with_style(&crate::Style::new().flex_row());
        assert_eq!(
            tree.reconcile(decorated).unwrap_err(),
            ReconcileError::FragmentDecoration
        );
        assert!(tree.is_empty());
    }
}
