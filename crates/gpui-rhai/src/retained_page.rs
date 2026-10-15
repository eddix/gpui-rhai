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
            for component in node.presented_components() {
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

    const FORWARDED_LEAF: &str = r#"
define_component(#{ metadata: #{ id: "test/leaf", "export": "Leaf", version: "0.2.2", runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{} },
    schema: #{ props: #{}, state: #{ fields: #{ count: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } } } }, events: #{}, slots: #{}, parts: [] }, render: Fn("leaf") });
fn Leaf(props) { render_component("test/leaf", props) }
fn increment(ctx, event) { ctx.set_state("count", ctx.get_state("count") + 1); }
fn leaf(ctx, props) { text(`${ctx.is_visible()}:${ctx.get_state("count")}`).on_click(Fn("increment")) }
"#;
    const FORWARDING_PASS: &str = r#"
define_component(#{ metadata: #{ id: "test/pass", "export": "Pass", version: "0.2.2", runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{} },
    schema: #{ props: #{ content: #{ schema: #{ type: "node" }, required: true, sensitive: false } }, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: [] }, render: Fn("pass") });
fn Pass(props) { render_component("test/pass", props) }
fn pass(ctx, props) { props.content }
"#;
    const DIRECT_OUTER: &str = r#"
import "test/leaf" as leaf;
define_component(#{ metadata: #{ id: "test/outer", "export": "Outer", version: "0.2.2", runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: ["test/leaf"], capabilities: #{} },
    schema: #{ props: #{}, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: [] }, render: Fn("outer") });
fn Outer(props) { render_component("test/outer", props) }
fn outer(ctx, props) { leaf::Leaf(#{ key: "nested" }) }
"#;
    const FORWARDING_MAIN: &str = r#"
import "test/leaf" as leaf;
import "test/pass" as pass;
import "test/outer" as outer;
fn state_schema() { #{ fields: #{ visible: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } } } } }
fn view(ctx) { column([
    pass::Pass(#{ key: "forward", content: leaf::Leaf(#{ key: "eager" }) }).with_key("forward").retained_page(ctx.get_state("visible")),
    outer::Outer(#{ key: "outer" }).with_key("outer").retained_page(ctx.get_state("visible")),
]).with_key("root") }
"#;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn forwarding_and_direct_return_keep_every_construction_owners_visibility() {
        let mut engine = RuntimeEngine::new();
        let source = crate::EmbeddedScriptSource::new(BTreeMap::from([
            (
                crate::ModuleId::parse("test/leaf").unwrap(),
                FORWARDED_LEAF.to_owned(),
            ),
            (
                crate::ModuleId::parse("test/pass").unwrap(),
                FORWARDING_PASS.to_owned(),
            ),
            (
                crate::ModuleId::parse("test/outer").unwrap(),
                DIRECT_OUTER.to_owned(),
            ),
        ]));
        engine.set_module_resolver(crate::RestrictedModuleResolver::from_source(&source).unwrap());
        let compiled = engine
            .compile_self_contained_named("forwarding", FORWARDING_MAIN)
            .unwrap();
        let schema = engine.root_state_schema(&compiled).unwrap();
        let root = ComponentInstancePath::root("View", "one");
        let eager = root.child("Leaf", "eager");
        let nested = root.child("Outer", "outer").child("Leaf", "nested");
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            runtime.clone(),
            root.clone(),
            Some("w".to_owned()),
            BTreeMap::new(),
            &schema,
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        for owner in [&eager, &nested] {
            assert!(
                runtime.borrow_mut().page_visibility.read(owner, false),
                "forwarded owner {owner} is placed, despite the outer visual marker"
            );
            assert!(runtime.borrow().dirty_components().contains(owner));
        }
        assert!(lifecycle.render_dirty(&mut engine).unwrap());
        let crate::UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
            unreachable!()
        };
        assert!(children.iter().all(
            |node| matches!(node.kind(), crate::UiNodeKind::Text { text } if text == "true:0")
        ));
        let crate::UiEventHandler::Script(callback) = children[0].handler("click").unwrap().clone()
        else {
            unreachable!()
        };
        assert_eq!(
            callback.component(),
            Some(&eager),
            "forwarding must not adopt the caller's callback/state owner"
        );
        let ids = lifecycle
            .retained()
            .nodes()
            .map(crate::RetainedNode::id)
            .collect::<Vec<_>>();
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "visible", UiValue::Bool(false))
            .unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();
        for owner in [&eager, &nested] {
            assert!(!runtime.borrow_mut().page_visibility.read(owner, false));
        }
        let _ = lifecycle
            .invoke_callback_transactional(&engine, &callback, UiValue::Null)
            .unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();
        assert_eq!(
            runtime.borrow().component_state.get(&eager, "count"),
            Some(&UiValue::Integer(1)),
            "business callbacks remain active while hidden"
        );
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "visible", UiValue::Bool(true))
            .unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();
        assert_eq!(
            lifecycle
                .retained()
                .nodes()
                .map(crate::RetainedNode::id)
                .collect::<Vec<_>>(),
            ids
        );
        for owner in [&eager, &nested] {
            assert!(runtime.borrow_mut().page_visibility.read(owner, false));
        }
        let crate::UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
            unreachable!()
        };
        assert!(matches!(children[0].kind(), crate::UiNodeKind::Text { text } if text == "true:1"));
        let rejected = engine
            .compile("fn view(ctx) { throw \"reject forwarding candidate\"; }")
            .unwrap();
        assert!(lifecycle.reload(&mut engine, rejected, &schema).is_err());
        for owner in [&eager, &nested] {
            assert!(runtime.borrow_mut().page_visibility.read(owner, false));
        }
    }

    #[test]
    fn hidden_page_motion_retains_active_time_through_view_suspend_and_resume() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine.compile(r#"
fn state_schema() { #{ fields: #{ visible: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } } } } }
fn view(ctx) { column([text("moving").with_key("moving").motion(motion_transition("opacity", 0.0, 1.0, #{ duration_ms: 1000, easing: "linear" }))]).with_key("page").retained_page(ctx.get_state("visible")) }
"#).unwrap();
        let schema = engine.root_state_schema(&compiled).unwrap();
        let root = ComponentInstancePath::root("View", "one");
        let now = Instant::now();
        let clock = crate::ManualRuntimeClock::new(now);
        let mut state = UiRuntimeState::new();
        state.clock = clock.clock();
        let runtime = Rc::new(RefCell::new(state));
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            runtime.clone(),
            root.clone(),
            Some("w".to_owned()),
            BTreeMap::new(),
            &schema,
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        let key = runtime
            .borrow()
            .motion_values
            .keys()
            .next()
            .unwrap()
            .clone();
        clock.advance(Duration::from_millis(250));
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "visible", UiValue::Bool(false))
            .unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();
        clock.advance(Duration::from_millis(250));
        lifecycle.suspend(&mut engine).unwrap();
        assert!(
            (runtime.borrow().motions.snapshot(clock.clock().now())[&key] - 0.25).abs() < 0.001,
            "whole-view suspension must not resample already frozen page motion"
        );
        clock.advance(Duration::from_secs(5));
        lifecycle.resume(&mut engine).unwrap();
        assert!(
            (runtime.borrow().motions.snapshot(clock.clock().now())[&key] - 0.25).abs() < 0.001
        );
        assert!(
            !runtime
                .borrow_mut()
                .motions
                .tick(clock.clock().now())
                .needs_frame
        );
        clock.advance(Duration::from_secs(1));
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "visible", UiValue::Bool(true))
            .unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();
        clock.advance(Duration::from_millis(100));
        assert!(
            (runtime.borrow().motions.snapshot(clock.clock().now())[&key] - 0.35).abs() < 0.001
        );
    }

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
