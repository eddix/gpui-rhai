use std::collections::{BTreeMap, BTreeSet};

use rhai::{CustomType, ImmutableString, TypeBuilder};
use thiserror::Error;

use crate::{ComponentInstancePath, NodeId};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ElementRefId {
    component: ComponentInstancePath,
    key: String,
}

impl ElementRefId {
    /// Construct a stable component-local reference identity.
    ///
    /// # Errors
    ///
    /// Returns [`ElementRefError::InvalidKey`] for unsafe keys.
    pub fn new(
        component: ComponentInstancePath,
        key: impl Into<String>,
    ) -> Result<Self, ElementRefError> {
        let key = key.into();
        if key.is_empty()
            || key.len() > 128
            || !key.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':')
            })
        {
            return Err(ElementRefError::InvalidKey(key));
        }
        Ok(Self { component, key })
    }

    #[must_use]
    pub const fn component(&self) -> &ComponentInstancePath {
        &self.component
    }

    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
}

/// Non-owning reference to a retained node in one formal component scope.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ElementRef {
    id: ElementRefId,
}

impl ElementRef {
    #[must_use]
    pub const fn new(id: ElementRefId) -> Self {
        Self { id }
    }

    #[must_use]
    pub const fn id(&self) -> &ElementRefId {
        &self.id
    }
}

impl CustomType for ElementRef {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("ElementRef")
            .with_get("key", |reference: &mut Self| {
                ImmutableString::from(reference.id.key.as_str())
            })
            .with_get("scope", |reference: &mut Self| {
                ImmutableString::from(format!(
                    "{}:{}",
                    reference.id.component(),
                    reference.id.key()
                ))
            });
    }
}

#[derive(Clone, Debug, Default)]
pub struct ElementRefRegistry {
    active: BTreeMap<ElementRefId, NodeId>,
    // The logical subscription survives absence and replacement of its node.
    geometry_readers: BTreeMap<ElementRefId, BTreeSet<crate::read_dependency::ReadDependency>>,
    dirty: BTreeSet<ComponentInstancePath>,
}

pub(crate) type ElementRefGeometryBindings =
    BTreeMap<NodeId, BTreeMap<ElementRefId, BTreeSet<crate::read_dependency::ReadDependency>>>;

impl ElementRefRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.active.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&ElementRefId, &NodeId)> {
        self.active.iter()
    }

    /// Resolve the last committed retained identity.
    ///
    /// # Errors
    ///
    /// Returns [`ElementRefError::Stale`] after unmount or before commit.
    pub fn resolve(&self, reference: &ElementRef) -> Result<NodeId, ElementRefError> {
        self.active
            .get(reference.id())
            .copied()
            .ok_or_else(|| ElementRefError::Stale(reference.id().clone()))
    }

    /// Resolve a mounted component-local ref key.
    ///
    /// # Errors
    ///
    /// Returns [`ElementRefError::UnknownKey`] when no binding is committed.
    pub fn resolve_key(
        &self,
        component: &ComponentInstancePath,
        key: &str,
    ) -> Result<ElementRef, ElementRefError> {
        self.active
            .keys()
            .find(|id| id.component() == component && id.key() == key)
            .cloned()
            .map(ElementRef::new)
            .ok_or_else(|| ElementRefError::UnknownKey {
                component: component.clone(),
                key: key.to_owned(),
            })
    }

    pub(crate) fn resolve_and_track_geometry(
        &mut self,
        reference: &ElementRef,
        reader: impl Into<crate::read_dependency::ReadDependency>,
    ) -> Option<NodeId> {
        self.geometry_readers
            .entry(reference.id().clone())
            .or_default()
            .insert(reader.into());
        self.active.get(reference.id()).copied()
    }

    pub(crate) fn reconcile(
        &mut self,
        root: &ComponentInstancePath,
        candidate: BTreeMap<ElementRefId, NodeId>,
    ) {
        let previous = self.active.clone();
        self.active.retain(|id, _| !id.component.is_within(root));
        self.active.extend(candidate);
        for (id, readers) in &self.geometry_readers {
            if id.component.is_within(root) && previous.get(id) != self.active.get(id) {
                self.dirty
                    .extend(readers.iter().map(|reader| reader.owner.clone()));
            }
        }
    }

    /// The committed logical bindings, grouped by their current geometry node.
    /// Unbound references remain subscribed here but have no native target yet.
    pub(crate) fn geometry_bindings(
        &self,
        scope: &ComponentInstancePath,
    ) -> ElementRefGeometryBindings {
        let mut bindings = ElementRefGeometryBindings::new();
        for (id, readers) in &self.geometry_readers {
            if id.component.is_within(scope)
                && let Some(node) = self.active.get(id)
            {
                bindings
                    .entry(*node)
                    .or_default()
                    .insert(id.clone(), readers.clone());
            }
        }
        bindings
    }

    pub(crate) fn take_dirty(&mut self) -> BTreeSet<ComponentInstancePath> {
        std::mem::take(&mut self.dirty)
    }

    pub(crate) fn remove_scope(&mut self, root: &ComponentInstancePath) {
        self.active.retain(|id, _| !id.component.is_within(root));
        self.geometry_readers.retain(|id, readers| {
            if id.component.is_within(root) {
                self.dirty.extend(
                    readers
                        .iter()
                        .filter(|reader| !reader.owner.is_within(root))
                        .map(|reader| reader.owner.clone()),
                );
                return false;
            }
            readers.retain(|reader| !reader.owner.is_within(root));
            !readers.is_empty()
        });
        self.dirty.retain(|owner| !owner.is_within(root));
    }

    pub(crate) fn reset_contribution(&mut self, reader: &crate::read_dependency::ReadDependency) {
        crate::read_dependency::retain_readers(&mut self.geometry_readers, |existing| {
            existing != reader
        });
    }
    pub(crate) fn retain_contributions(
        &mut self,
        scope: &ComponentInstancePath,
        active: &BTreeSet<crate::read_dependency::ReadContribution>,
    ) {
        crate::read_dependency::retain_readers(&mut self.geometry_readers, |reader| {
            reader.retained_in_contribution_scope(scope, active)
        });
    }

    pub(crate) fn retain_reader_scope(
        &mut self,
        scope: &ComponentInstancePath,
        active: &BTreeSet<ComponentInstancePath>,
    ) {
        crate::read_dependency::retain_readers(&mut self.geometry_readers, |reader| {
            reader.retained_in_owner_scope(scope, active)
        });
        self.dirty
            .retain(|owner| !owner.is_within(scope) || owner == scope || active.contains(owner));
    }

    /// End subscriptions whose provider or executable reader left the final
    /// invocation manifest. A surviving external reader receives removal once.
    pub(crate) fn retain_scope(
        &mut self,
        scope: &ComponentInstancePath,
        active: &BTreeSet<ComponentInstancePath>,
    ) {
        self.retain_reader_scope(scope, active);
        let retained = |path: &ComponentInstancePath| {
            !path.is_within(scope) || path == scope || active.contains(path)
        };
        self.active.retain(|id, _| retained(id.component()));
        self.geometry_readers.retain(|id, readers| {
            if retained(id.component()) {
                true
            } else {
                self.dirty
                    .extend(readers.iter().map(|reader| reader.owner.clone()));
                false
            }
        });
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ElementRefError {
    #[error("element-ref key `{0}` must be 1-128 ASCII letters, digits, `_`, `-`, `.`, or `:`")]
    InvalidKey(String),
    #[error("element ref `{0:?}` is stale or unmounted")]
    Stale(ElementRefId),
    #[error("element ref `{0:?}` was not declared by the active formal component render")]
    Undeclared(ElementRefId),
    #[error("element ref `{0:?}` is bound to more than one node")]
    DuplicateBinding(ElementRefId),
    #[error("element ref `{0:?}` requires a stable node key")]
    MissingNodeKey(ElementRefId),
    #[error("component `{component}` has no mounted element ref `{key}`")]
    UnknownKey {
        component: ComponentInstancePath,
        key: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ElementCommand {
    Focus {
        window: String,
        node: NodeId,
    },
    ScrollTo {
        window: String,
        node: NodeId,
        x: f64,
        y: f64,
    },
    ScrollIntoView {
        window: String,
        node: NodeId,
    },
}

impl ElementCommand {
    pub(crate) fn window(&self) -> &str {
        match self {
            Self::Focus { window, .. }
            | Self::ScrollTo { window, .. }
            | Self::ScrollIntoView { window, .. } => window,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes() -> (NodeId, NodeId) {
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(crate::UiNode::text("first")).unwrap();
        let first = tree.root_id().unwrap();
        tree.reconcile(crate::UiNode::box_node(Vec::new())).unwrap();
        (first, tree.root_id().unwrap())
    }

    fn measured(width: f64) -> crate::ElementGeometry {
        let bounds = crate::GeometryBounds::new(0.0, 0.0, width, 24.0).unwrap();
        crate::ElementGeometry {
            layout: bounds,
            visual: bounds,
            clip: None,
        }
    }

    #[test]
    fn logical_ref_subscription_survives_all_binding_transitions_without_repeats() {
        let provider = ComponentInstancePath::root("Provider", "main");
        let first_reader = provider.child("Reader", "first");
        let second_reader = provider.child("Reader", "second");
        let reference = ElementRef::new(ElementRefId::new(provider.clone(), "field").unwrap());
        let readers = BTreeSet::from([first_reader.clone(), second_reader.clone()]);
        let (first, second) = nodes();
        let mut refs = ElementRefRegistry::new();
        let geometry = crate::GeometryRegistry::new();
        for reader in &readers {
            assert_eq!(refs.resolve_and_track_geometry(&reference, reader), None);
        }
        refs.reconcile(&provider, BTreeMap::new());
        assert!(refs.take_dirty().is_empty());
        for node in [first, first, second] {
            refs.reconcile(&provider, BTreeMap::from([(reference.id().clone(), node)]));
            geometry.sync_ref_readers(&provider, refs.geometry_bindings(&provider));
            let dirty = refs.take_dirty();
            if node == first && geometry.get(first).is_some() {
                assert!(dirty.is_empty(), "unchanged binding must stay idle");
            } else {
                assert_eq!(dirty, readers);
            }
            geometry.update(node, measured(if node == first { 120.0 } else { 180.0 }));
            geometry.take_dirty();
        }
        refs.reconcile(&provider, BTreeMap::new());
        geometry.sync_ref_readers(&provider, refs.geometry_bindings(&provider));
        assert_eq!(refs.take_dirty(), readers);
        assert!(geometry.take_dirty().is_empty());
        geometry.update(first, measured(200.0));
        geometry.update(second, measured(220.0));
        assert!(
            geometry.take_dirty().is_empty(),
            "neither old node stays subscribed"
        );
        refs.reconcile(&provider, BTreeMap::new());
        assert!(refs.take_dirty().is_empty());
    }

    #[test]
    fn rebinding_does_not_remove_an_independent_direct_geometry_read() {
        let provider = ComponentInstancePath::root("Provider", "main");
        let owner = provider.child("Reader", "shared");
        let reference = ElementRef::new(ElementRefId::new(provider.clone(), "field").unwrap());
        let (first, second) = nodes();
        let geometry = crate::GeometryRegistry::new();
        let mut refs = ElementRefRegistry::new();
        refs.resolve_and_track_geometry(&reference, &owner);
        refs.reconcile(&provider, BTreeMap::from([(reference.id().clone(), first)]));
        geometry.sync_ref_readers(&provider, refs.geometry_bindings(&provider));
        geometry.read_tracked(first, &owner);
        geometry.update(first, measured(120.0));
        geometry.take_dirty();
        refs.reconcile(
            &provider,
            BTreeMap::from([(reference.id().clone(), second)]),
        );
        geometry.sync_ref_readers(&provider, refs.geometry_bindings(&provider));
        geometry.update(first, measured(140.0));
        assert_eq!(geometry.take_dirty(), BTreeSet::from([owner.clone()]));
        let dependency = crate::read_dependency::ReadDependency::component(&owner);
        geometry.reset_contribution(&dependency);
        geometry.sync_ref_readers(&provider, refs.geometry_bindings(&provider));
        geometry.take_dirty();
        geometry.update(first, measured(160.0));
        assert!(geometry.take_dirty().is_empty());
        geometry.update(second, measured(180.0));
        assert_eq!(geometry.take_dirty(), BTreeSet::from([owner]));
    }

    #[test]
    fn contribution_retarget_and_last_item_prune_detach_native_and_logical_reads() {
        let provider = ComponentInstancePath::root("Provider", "main");
        let owner = provider.child("Reader", "shared");
        let collection = crate::VirtualCollectionId {
            component: owner.clone(),
            key: "rows".into(),
        };
        let scope = owner.child("VirtualCollection", "rows");
        let direct = crate::read_dependency::ReadDependency::component(&owner);
        let item = crate::read_dependency::ReadDependency::virtual_item(&owner, &collection, "a");
        let first_ref = ElementRef::new(ElementRefId::new(provider.clone(), "first").unwrap());
        let second_ref = ElementRef::new(ElementRefId::new(provider.clone(), "second").unwrap());
        let (first, second) = nodes();
        let geometry = crate::GeometryRegistry::new();
        let mut refs = ElementRefRegistry::new();
        refs.reconcile(
            &provider,
            BTreeMap::from([
                (first_ref.id().clone(), first),
                (second_ref.id().clone(), second),
            ]),
        );
        for reader in [&direct, &item] {
            refs.resolve_and_track_geometry(&first_ref, reader);
            geometry.read_ref_tracked(first_ref.id(), first, reader);
        }
        geometry.update(first, measured(100.0));
        geometry.update(second, measured(120.0));
        geometry.take_dirty();
        refs.reset_contribution(&direct);
        geometry.reset_contribution(&direct);
        refs.resolve_and_track_geometry(&second_ref, &direct);
        geometry.read_ref_tracked(second_ref.id(), second, &direct);
        refs.retain_contributions(&scope, &BTreeSet::new());
        geometry.retain_contributions(&scope, &BTreeSet::new());
        geometry.update(first, measured(140.0));
        assert!(geometry.take_dirty().is_empty());
        geometry.update(second, measured(160.0));
        assert_eq!(geometry.take_dirty(), BTreeSet::from([owner.clone()]));
        refs.reset_contribution(&direct);
        geometry.reset_contribution(&direct);
        refs.reconcile(&provider, BTreeMap::new());
        assert!(refs.take_dirty().is_empty());
        assert!(refs.geometry_readers.is_empty());
        geometry.update(second, measured(180.0));
        assert!(geometry.take_dirty().is_empty());
    }

    #[test]
    fn reader_teardown_and_provider_removal_notify_only_surviving_owners() {
        let root = ComponentInstancePath::root("App", "main");
        let provider = root.child("Provider", "source");
        let internal = provider.child("Reader", "internal");
        let external = root.child("Reader", "external");
        let removed = root.child("Reader", "removed");
        let reference = ElementRef::new(ElementRefId::new(provider.clone(), "field").unwrap());
        let (node, _) = nodes();
        let mut refs = ElementRefRegistry::new();
        let geometry = crate::GeometryRegistry::new();
        for reader in [&internal, &external, &removed] {
            refs.resolve_and_track_geometry(&reference, reader);
        }
        refs.reconcile(&root, BTreeMap::from([(reference.id().clone(), node)]));
        geometry.sync_ref_readers(&root, refs.geometry_bindings(&root));
        refs.take_dirty();
        geometry.update(node, measured(100.0));
        geometry.take_dirty();
        let active = BTreeSet::from([provider.clone(), internal.clone(), external.clone()]);
        refs.retain_reader_scope(&root, &active);
        geometry.retain_reader_scope(&root, &active);
        refs.remove_scope(&provider);
        geometry.remove_scope(&provider);
        assert_eq!(refs.take_dirty(), BTreeSet::from([external]));
        assert!(refs.geometry_readers.is_empty());
        geometry.update(node, measured(120.0));
        assert!(geometry.take_dirty().is_empty());
    }

    #[test]
    fn final_manifest_prunes_provider_subscription_without_reviving_removed_readers() {
        let root = ComponentInstancePath::root("App", "main");
        let provider = root.child("Provider", "source");
        let reader = root.child("Reader", "external");
        let removed = root.child("Reader", "removed");
        let reference = ElementRef::new(ElementRefId::new(provider.clone(), "field").unwrap());
        let (node, _) = nodes();
        let mut refs = ElementRefRegistry::new();
        let geometry = crate::GeometryRegistry::new();
        for owner in [&reader, &removed] {
            refs.resolve_and_track_geometry(&reference, owner);
        }
        refs.reconcile(&root, BTreeMap::from([(reference.id().clone(), node)]));
        geometry.sync_ref_readers(&root, refs.geometry_bindings(&root));
        refs.take_dirty();
        geometry.update(node, measured(100.0));
        geometry.take_dirty();
        let active = BTreeSet::from([reader.clone()]);
        refs.retain_scope(&root, &active);
        geometry.retain_reader_scope(&root, &active);
        assert_eq!(refs.take_dirty(), BTreeSet::from([reader]));
        assert!(refs.geometry_readers.is_empty());
        assert!(refs.resolve(&reference).is_err());
        geometry.update(node, measured(120.0));
        assert!(geometry.take_dirty().is_empty());
    }

    #[test]
    fn failed_candidate_restores_last_good_binding_and_subscriptions() {
        let provider = ComponentInstancePath::root("Provider", "main");
        let owner = provider.child("Reader", "reader");
        let dependency = crate::read_dependency::ReadDependency::component(&owner);
        let reference = ElementRef::new(ElementRefId::new(provider.clone(), "field").unwrap());
        let (first, second) = nodes();
        let mut refs = ElementRefRegistry::new();
        let geometry = crate::GeometryRegistry::new();
        refs.resolve_and_track_geometry(&reference, &owner);
        refs.reconcile(&provider, BTreeMap::from([(reference.id().clone(), first)]));
        geometry.sync_ref_readers(&provider, refs.geometry_bindings(&provider));
        refs.take_dirty();
        geometry.update(first, measured(100.0));
        geometry.take_dirty();
        let saved_refs = refs.clone();
        let saved_geometry = geometry.snapshot();
        refs.reset_contribution(&dependency);
        geometry.reset_contribution(&dependency);
        refs.resolve_and_track_geometry(&reference, &owner);
        refs.reconcile(
            &provider,
            BTreeMap::from([(reference.id().clone(), second)]),
        );
        geometry.sync_ref_readers(&provider, refs.geometry_bindings(&provider));
        refs = saved_refs;
        geometry.restore(saved_geometry);
        assert_eq!(refs.resolve(&reference), Ok(first));
        assert!(refs.take_dirty().is_empty());
        geometry.update(second, measured(180.0));
        assert!(geometry.take_dirty().is_empty());
        geometry.update(first, measured(120.0));
        assert_eq!(geometry.take_dirty(), BTreeSet::from([owner]));
    }

    #[test]
    fn stale_refs_do_not_rebind_after_scope_removal() {
        let component = ComponentInstancePath::root("Panel", "main");
        let reference = ElementRef::new(ElementRefId::new(component.clone(), "field").unwrap());
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(crate::UiNode::text("field")).unwrap();
        let mut registry = ElementRefRegistry::new();
        registry.reconcile(
            &component,
            BTreeMap::from([(reference.id().clone(), tree.root_id().unwrap())]),
        );
        assert!(registry.resolve(&reference).is_ok());
        registry.remove_scope(&component);
        assert!(matches!(
            registry.resolve(&reference),
            Err(ElementRefError::Stale(_))
        ));
    }

    #[test]
    fn pending_geometry_reader_binds_after_the_ref_commits() {
        let component = ComponentInstancePath::root("Panel", "main");
        let reader = component.child("Inspector", "reader");
        let reference = ElementRef::new(ElementRefId::new(component.clone(), "field").unwrap());
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(crate::UiNode::text("field")).unwrap();
        let node = tree.root_id().unwrap();
        let mut refs = ElementRefRegistry::new();
        assert_eq!(refs.resolve_and_track_geometry(&reference, &reader), None);

        refs.reconcile(&component, BTreeMap::from([(reference.id().clone(), node)]));
        let geometry = crate::GeometryRegistry::new();
        geometry.sync_ref_readers(&component, refs.geometry_bindings(&component));
        geometry.update(
            node,
            crate::ElementGeometry {
                layout: crate::GeometryBounds::new(0.0, 0.0, 120.0, 24.0).unwrap(),
                visual: crate::GeometryBounds::new(0.0, 0.0, 120.0, 24.0).unwrap(),
                clip: None,
            },
        );
        assert_eq!(geometry.take_dirty(), BTreeSet::from([reader.clone()]));

        assert_eq!(
            refs.resolve_and_track_geometry(&reference, &reader),
            Some(node)
        );
        tree.reconcile(crate::UiNode::box_node(Vec::new())).unwrap();
        let replacement = tree.root_id().unwrap();
        assert_ne!(replacement, node);
        refs.reconcile(
            &component,
            BTreeMap::from([(reference.id().clone(), replacement)]),
        );
        geometry.sync_ref_readers(&component, refs.geometry_bindings(&component));
        geometry.retain_nodes(&BTreeSet::from([replacement]));
        geometry.update(
            replacement,
            crate::ElementGeometry {
                layout: crate::GeometryBounds::new(0.0, 0.0, 160.0, 24.0).unwrap(),
                visual: crate::GeometryBounds::new(0.0, 0.0, 160.0, 24.0).unwrap(),
                clip: None,
            },
        );
        assert_eq!(geometry.take_dirty(), BTreeSet::from([reader]));
    }
}
