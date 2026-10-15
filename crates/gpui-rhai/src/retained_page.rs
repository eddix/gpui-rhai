//! Presentation policy shared by retained tab panels and navigation visits.

use std::collections::{BTreeMap, BTreeSet};

use crate::{ComponentInstancePath, NodeId, RetainedUiTree, UiValue};

pub(crate) const PAGE_VISIBLE: &str = "retained_page_visible";
pub(crate) const NAVIGATION_DEPTH: &str = "navigation_depth";
pub(crate) const NAVIGATION_BACK: &str = "navigation_back";

/// One controlled navigation scope. Entry keys identify visits, not records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationStackInfo {
    /// Owning mounted view; local stack keys are not globally unique.
    pub view_id: String,
    /// Stable retained identity for this stack scope.
    pub node: NodeId,
    /// Accessible name supplied by its author.
    pub label: String,
    /// Controlled visit count, including the root.
    pub depth: usize,
    /// Whether a non-root pop can be proposed to a mounted handler.
    pub can_pop: bool,
    /// Whether this scope contains current keyboard focus.
    pub focused: bool,
}

/// Transactional committed placement and the components that observe it.
/// Construction is eager, so the first render reads false until placement.
#[derive(Clone, Debug, Default)]
pub(crate) struct PageVisibility {
    values: BTreeMap<ComponentInstancePath, bool>,
    readers: BTreeSet<ComponentInstancePath>,
    motion_scopes: BTreeMap<String, BTreeSet<String>>,
}

impl PageVisibility {
    pub fn read(&mut self, component: &ComponentInstancePath, track: bool) -> bool {
        if track {
            self.readers.insert(component.clone());
        }
        self.values.get(component).copied().unwrap_or(false)
    }

    pub fn reset_reader(&mut self, component: &ComponentInstancePath) {
        self.readers.remove(component);
    }

    pub fn retain_scope(
        &mut self,
        root: &ComponentInstancePath,
        active: &BTreeSet<ComponentInstancePath>,
    ) {
        self.readers
            .retain(|path| !path.is_within(root) || path == root || active.contains(path));
        self.values
            .retain(|path, _| !path.is_within(root) || path == root || active.contains(path));
    }

    pub fn remove_scope(&mut self, root: &ComponentInstancePath, domain: &str) {
        self.readers.retain(|path| !path.is_within(root));
        self.values.retain(|path, _| !path.is_within(root));
        self.motion_scopes
            .retain(|path, _| !path.starts_with(domain));
    }

    pub fn reconcile(
        &mut self,
        tree: &RetainedUiTree,
        root: &ComponentInstancePath,
        domain: &str,
        motions: &mut crate::MotionRuntime,
        now: std::time::Instant,
    ) -> BTreeSet<ComponentInstancePath> {
        let mut next = BTreeMap::from([(
            root.clone(),
            tree.root_id()
                .is_some_and(|node| tree.is_page_visible(node)),
        )]);
        let mut hidden = BTreeSet::new();
        for node in tree.nodes() {
            let visible = tree.is_page_visible(node.id());
            if let Some(component) = node.component_root() {
                next.entry(component.clone())
                    .and_modify(|current| *current |= visible)
                    .or_insert(visible);
            }
            if !visible {
                hidden.insert(crate::motion::retained_node_path(domain, node.id()));
            }
        }
        let previous = self.motion_scopes.entry(domain.to_owned()).or_default();
        for scope in previous.difference(&hidden) {
            motions.resume_node_scope(scope, now);
        }
        for scope in hidden.difference(previous) {
            motions.suspend_node_scope(scope, now);
        }
        *previous = hidden;
        let dirty = self
            .readers
            .iter()
            .filter(|path| path.is_within(root))
            .filter(|path| {
                self.values.get(*path).copied().unwrap_or(false)
                    != next.get(*path).copied().unwrap_or(false)
            })
            .cloned()
            .collect();
        self.values.retain(|path, _| !path.is_within(root));
        self.values.extend(next);
        dirty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ComponentStateSchema, MotionKey, MotionPreference, MotionProperty, MotionRuntime,
        MotionSource, MotionTransition, RuntimeEngine, ScriptLifecycle, UiNode, UiRuntimeState,
    };
    use std::{
        cell::RefCell,
        rc::Rc,
        time::{Duration, Instant},
    };

    #[test]
    fn nested_committed_visibility_tracks_readers_and_preserves_construction_owner() {
        let root = ComponentInstancePath::root("View", "one");
        let child = root.child("Panel", "caller-owned");
        let mut visibility = PageVisibility::default();
        let mut tree = RetainedUiTree::new();
        let make = |outer, inner| {
            UiNode::column(vec![
                UiNode::column(vec![
                    UiNode::text("child").with_component_root(child.clone()),
                ])
                .with_key("inner")
                .retained_page(inner),
            ])
            .with_key("outer")
            .retained_page(outer)
        };
        assert!(
            !visibility.read(&child, true),
            "placement is unknown during eager construction"
        );
        let now = Instant::now();
        let mut motions = MotionRuntime::new(MotionPreference::Normal);
        tree.reconcile(make(false, true)).unwrap();
        assert!(
            visibility
                .reconcile(&tree, &root, "window:w/view:one/root", &mut motions, now)
                .is_empty()
        );
        tree.reconcile(make(true, true)).unwrap();
        assert!(
            visibility
                .reconcile(&tree, &root, "window:w/view:one/root", &mut motions, now)
                .contains(&child)
        );
        assert!(visibility.read(&child, true));
        tree.reconcile(make(true, false)).unwrap();
        assert!(
            visibility
                .reconcile(&tree, &root, "window:w/view:one/root", &mut motions, now)
                .contains(&child)
        );
        assert!(!visibility.read(&child, false));
        assert_eq!(
            tree.nodes().find_map(|node| node.component_root()),
            Some(&child)
        );
    }

    #[test]
    fn invisible_motion_freezes_and_never_requests_frames_or_catches_up() {
        let root = ComponentInstancePath::root("View", "one");
        let mut visibility = PageVisibility::default();
        let mut tree = RetainedUiTree::new();
        tree.reconcile(UiNode::text("motion").with_key("page").retained_page(true))
            .unwrap();
        let domain = "window:w/view:one/root";
        let path = crate::motion::retained_node_path(domain, tree.root_id().unwrap());
        let key = MotionKey::for_node(&path, MotionProperty::Opacity);
        let mut motions = MotionRuntime::new(MotionPreference::Normal);
        let now = Instant::now();
        let mut transition = MotionTransition::new(MotionProperty::Opacity, 0.0, 1.0, 1000);
        transition.easing = crate::MotionEasing::Linear;
        motions
            .start(
                ComponentInstancePath::root("UiNode", &path),
                MotionSource::Transition(transition),
                now,
            )
            .unwrap();
        let hide = now + Duration::from_millis(250);
        tree.reconcile(UiNode::text("motion").with_key("page").retained_page(false))
            .unwrap();
        visibility.reconcile(&tree, &root, domain, &mut motions, hide);
        let frame = motions.tick_scope(now + Duration::from_secs(20), domain);
        assert!(!frame.needs_frame);
        assert!((frame.values[&key] - 0.25).abs() < 0.001);
        let show = now + Duration::from_secs(25);
        tree.reconcile(UiNode::text("motion").with_key("page").retained_page(true))
            .unwrap();
        visibility.reconcile(&tree, &root, domain, &mut motions, show);
        assert!(
            (motions
                .sample(&key, show + Duration::from_millis(100))
                .unwrap()
                - 0.35)
                .abs()
                < 0.001
        );
    }

    #[test]
    fn root_visibility_invalidates_after_first_commit_and_rejected_reload_keeps_it() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile("fn view(ctx) { text(`${ctx.is_visible()}`).with_key(\"root\") }")
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            runtime.clone(),
            ComponentInstancePath::root("View", "one"),
            Some("w".to_owned()),
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        assert!(
            matches!(lifecycle.root().unwrap().kind(), crate::UiNodeKind::Text {text} if text == "false")
        );
        assert!(lifecycle.render_dirty(&mut engine).unwrap());
        assert!(
            matches!(lifecycle.root().unwrap().kind(), crate::UiNodeKind::Text {text} if text == "true")
        );
        let id = lifecycle.retained().root_id();
        let candidate = engine
            .compile("fn view(ctx) { throw \"rejected\"; text(\"bad\").retained_page(false) }")
            .unwrap();
        assert!(
            lifecycle
                .reload(&mut engine, candidate, &ComponentStateSchema::default())
                .is_err()
        );
        assert_eq!(lifecycle.retained().root_id(), id);
        assert!(
            runtime
                .borrow_mut()
                .page_visibility
                .read(lifecycle.root_path(), false)
        );
        assert!(
            matches!(lifecycle.root().unwrap().kind(), crate::UiNodeKind::Text {text} if text == "true")
        );
    }
}

impl RetainedUiTree {
    /// Whether every retained-page ancestor permits presentation. This is not
    /// a layout/paint query: ordinary clipping and display styles are separate.
    #[must_use]
    pub fn is_page_visible(&self, node: NodeId) -> bool {
        let mut cursor = Some(node);
        while let Some(id) = cursor {
            let Some(retained) = self.node(id) else {
                return false;
            };
            if retained.attributes().get(PAGE_VISIBLE) == Some(&UiValue::Bool(false)) {
                return false;
            }
            cursor = retained.parent();
        }
        true
    }
}
