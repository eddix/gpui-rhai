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

    /// An opaque, deterministic, whitespace-free accessibility ID for this ref.
    ///
    /// Encodes the typed component path and local key losslessly. Unlike the
    /// diagnostic `scope` string, embedded path delimiters cannot alias another
    /// path. No mounted node, generation, random value or process counter enters
    /// the ID, so it stays stable across reconciliation and retained hiding.
    #[must_use]
    pub fn semantic_id(&self) -> String {
        fn append_hex(output: &mut String, value: &str) {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            for byte in value.bytes() {
                output.push(char::from(HEX[usize::from(byte >> 4)]));
                output.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
        }

        let mut output = String::from("element-ref");
        // Hex alphabet excludes every structural separator. Segment boundaries,
        // component names, keys and the final ref key are therefore unambiguous.
        for (component, key) in self.id.component.segments() {
            output.push('/');
            append_hex(&mut output, component);
            output.push('.');
            append_hex(&mut output, key);
        }
        output.push(':');
        append_hex(&mut output, &self.id.key);
        output
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
            })
            .with_get("semantic_id", |reference: &mut Self| {
                ImmutableString::from(reference.semantic_id())
            });
    }
}

#[derive(Clone, Debug, Default)]
pub struct ElementRefRegistry {
    active: BTreeMap<ElementRefId, NodeId>,
    // The logical subscription survives absence and replacement of its node.
    geometry_readers: BTreeMap<ElementRefId, ElementRefGeometryReaders>,
    dirty: BTreeSet<ComponentInstancePath>,
}

/// The geometry projection observed by one executable contribution. Bounds is
/// stronger than size, so reading both coalesces to a full observation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ElementRefGeometryRead {
    LayoutSize,
    Bounds,
}

pub(crate) type ElementRefGeometryReaders =
    BTreeMap<crate::read_dependency::ReadDependency, ElementRefGeometryRead>;
pub(crate) type ElementRefGeometryBindings =
    BTreeMap<NodeId, BTreeMap<ElementRefId, ElementRefGeometryReaders>>;

pub(crate) fn retain_geometry_ref_readers<K: Ord>(
    index: &mut BTreeMap<K, ElementRefGeometryReaders>,
    predicate: impl Fn(&crate::read_dependency::ReadDependency) -> bool,
) {
    index.retain(|_, readers| {
        readers.retain(|reader, _| predicate(reader));
        !readers.is_empty()
    });
}

pub(crate) fn track_geometry_ref_reader(
    readers: &mut ElementRefGeometryReaders,
    reader: crate::read_dependency::ReadDependency,
    observation: ElementRefGeometryRead,
) {
    readers
        .entry(reader)
        .and_modify(|previous| *previous = (*previous).max(observation))
        .or_insert(observation);
}

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
        self.resolve_and_track_geometry_read(reference, reader, ElementRefGeometryRead::Bounds)
    }

    pub(crate) fn resolve_and_track_geometry_read(
        &mut self,
        reference: &ElementRef,
        reader: impl Into<crate::read_dependency::ReadDependency>,
        observation: ElementRefGeometryRead,
    ) -> Option<NodeId> {
        track_geometry_ref_reader(
            self.geometry_readers
                .entry(reference.id().clone())
                .or_default(),
            reader.into(),
            observation,
        );
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
                    .extend(readers.keys().map(|reader| reader.owner.clone()));
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
                        .keys()
                        .filter(|reader| !reader.owner.is_within(root))
                        .map(|reader| reader.owner.clone()),
                );
                return false;
            }
            readers.retain(|reader, _| !reader.owner.is_within(root));
            !readers.is_empty()
        });
        self.dirty.retain(|owner| !owner.is_within(root));
    }

    pub(crate) fn reset_contribution(&mut self, reader: &crate::read_dependency::ReadDependency) {
        retain_geometry_ref_readers(&mut self.geometry_readers, |existing| existing != reader);
    }
    pub(crate) fn retain_contributions(
        &mut self,
        scope: &ComponentInstancePath,
        active: &BTreeSet<crate::read_dependency::ReadContribution>,
    ) {
        retain_geometry_ref_readers(&mut self.geometry_readers, |reader| {
            reader.retained_in_contribution_scope(scope, active)
        });
    }

    pub(crate) fn retain_reader_scope(
        &mut self,
        scope: &ComponentInstancePath,
        active: &BTreeSet<ComponentInstancePath>,
    ) {
        retain_geometry_ref_readers(&mut self.geometry_readers, |reader| {
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
                    .extend(readers.keys().map(|reader| reader.owner.clone()));
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
    CopyText {
        window: String,
        text: String,
    },
    ScrollCollectionBy {
        window: String,
        id: crate::VirtualCollectionId,
        distance: f64,
    },
    ScrollCollection {
        window: String,
        id: crate::VirtualCollectionId,
        end: bool,
    },
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
            Self::CopyText { window, .. }
            | Self::ScrollCollectionBy { window, .. }
            | Self::ScrollCollection { window, .. }
            | Self::Focus { window, .. }
            | Self::ScrollTo { window, .. }
            | Self::ScrollIntoView { window, .. } => window,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_ids_encode_typed_paths_instead_of_ambiguous_display_strings() {
        let flat = ComponentInstancePath::root("View", "one").child("Pane", "left]/Pane[right");
        let nested = ComponentInstancePath::root("View", "one")
            .child("Pane", "left")
            .child("Pane", "right");
        assert_ne!(flat, nested);
        assert_eq!(flat.to_string(), nested.to_string());
        let flat = ElementRef::new(ElementRefId::new(flat, "choice-0").unwrap());
        let nested = ElementRef::new(ElementRefId::new(nested, "choice-0").unwrap());
        assert_ne!(flat.semantic_id(), nested.semantic_id());
        assert_eq!(flat.semantic_id(), flat.clone().semantic_id());
        let different_key =
            ElementRef::new(ElementRefId::new(flat.id.component.clone(), "choice-1").unwrap());
        assert_ne!(flat.semantic_id(), different_key.semantic_id());
        let unicode = ElementRef::new(
            ElementRefId::new(
                ComponentInstancePath::root("View", "one").child("Pane", "北 美\t\n%20"),
                "choice-0",
            )
            .unwrap(),
        );
        assert!(unicode.semantic_id().is_ascii());
        assert!(!unicode.semantic_id().chars().any(char::is_whitespace));
    }

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
    fn size_readers_ignore_position_visual_and_clip_but_full_reads_still_notify() {
        let provider = ComponentInstancePath::root("Provider", "main");
        let size_owner = provider.child("Reader", "size");
        let full_owner = provider.child("Reader", "full");
        let direct_owner = provider.child("Reader", "direct");
        let both_owner = provider.child("Reader", "both");
        let reference = ElementRef::new(ElementRefId::new(provider.clone(), "field").unwrap());
        let (node, _) = nodes();
        let geometry = crate::GeometryRegistry::new();
        let mut refs = ElementRefRegistry::new();
        for owner in [&size_owner, &both_owner] {
            refs.resolve_and_track_geometry_read(
                &reference,
                owner,
                ElementRefGeometryRead::LayoutSize,
            );
        }
        for owner in [&full_owner, &both_owner] {
            refs.resolve_and_track_geometry(&reference, owner);
        }
        refs.reconcile(&provider, BTreeMap::from([(reference.id().clone(), node)]));
        geometry.sync_ref_readers(&provider, refs.geometry_bindings(&provider));
        geometry.read_tracked(node, &direct_owner);
        refs.take_dirty();
        let mut current = measured(120.0);
        geometry.update(node, current);
        assert_eq!(
            geometry.take_dirty(),
            BTreeSet::from([
                size_owner.clone(),
                full_owner.clone(),
                direct_owner.clone(),
                both_owner.clone(),
            ])
        );

        let full_owners = BTreeSet::from([full_owner, direct_owner, both_owner]);
        current.layout.x = 70.0;
        current.layout.y = 30.0;
        geometry.update(node, current);
        assert_eq!(geometry.take_dirty(), full_owners);
        current.visual.width = 240.0;
        geometry.update(node, current);
        assert_eq!(geometry.take_dirty(), full_owners);
        current.clip = Some(crate::GeometryBounds::new(0.0, 0.0, 10.0, 10.0).unwrap());
        geometry.update(node, current);
        assert_eq!(geometry.take_dirty(), full_owners);
        current.layout.height = 32.0;
        geometry.update(node, current);
        assert_eq!(
            geometry.take_dirty(),
            full_owners.into_iter().chain([size_owner]).collect()
        );
    }

    #[test]
    fn size_subscription_survives_pending_rebind_unmount_and_remount() {
        let provider = ComponentInstancePath::root("Provider", "main");
        let owner = provider.child("Reader", "size");
        let reference = ElementRef::new(ElementRefId::new(provider.clone(), "field").unwrap());
        let (first, second) = nodes();
        let geometry = crate::GeometryRegistry::new();
        let mut refs = ElementRefRegistry::new();
        assert_eq!(
            refs.resolve_and_track_geometry_read(
                &reference,
                &owner,
                ElementRefGeometryRead::LayoutSize,
            ),
            None
        );
        for node in [first, second, first] {
            refs.reconcile(&provider, BTreeMap::from([(reference.id().clone(), node)]));
            geometry.sync_ref_readers(&provider, refs.geometry_bindings(&provider));
            assert_eq!(refs.take_dirty(), BTreeSet::from([owner.clone()]));
            geometry.update(node, measured(120.0));
            geometry.take_dirty();
            let mut moved = measured(120.0);
            moved.layout.x = 200.0;
            geometry.update(node, moved);
            assert!(geometry.take_dirty().is_empty());

            refs.reconcile(&provider, BTreeMap::new());
            geometry.sync_ref_readers(&provider, refs.geometry_bindings(&provider));
            assert_eq!(refs.take_dirty(), BTreeSet::from([owner.clone()]));
            geometry.take_dirty();
            geometry.update(node, measured(140.0));
            assert!(
                geometry.take_dirty().is_empty(),
                "unbound target is detached"
            );
        }
    }

    #[test]
    fn size_subscriptions_share_contribution_cleanup_and_rollback_with_bounds() {
        let provider = ComponentInstancePath::root("Provider", "main");
        let owner = provider.child("Reader", "shared");
        let reference = ElementRef::new(ElementRefId::new(provider.clone(), "field").unwrap());
        let collection = crate::VirtualCollectionId {
            component: owner.clone(),
            key: "rows".into(),
        };
        let scope = owner.child("VirtualCollection", "rows");
        let direct = crate::read_dependency::ReadDependency::component(&owner);
        let row = crate::read_dependency::ReadDependency::virtual_item(&owner, &collection, "a");
        let (node, _) = nodes();
        let geometry = crate::GeometryRegistry::new();
        let mut refs = ElementRefRegistry::new();
        refs.resolve_and_track_geometry_read(
            &reference,
            &direct,
            ElementRefGeometryRead::LayoutSize,
        );
        refs.resolve_and_track_geometry(&reference, &row);
        refs.reconcile(&provider, BTreeMap::from([(reference.id().clone(), node)]));
        geometry.sync_ref_readers(&provider, refs.geometry_bindings(&provider));
        refs.take_dirty();
        geometry.update(node, measured(120.0));
        geometry.take_dirty();
        let saved_refs = refs.clone();
        let saved_geometry = geometry.snapshot();
        refs.retain_contributions(&scope, &BTreeSet::new());
        geometry.retain_contributions(&scope, &BTreeSet::new());
        let mut moved = measured(120.0);
        moved.layout.x = 200.0;
        geometry.update(node, moved);
        assert!(
            geometry.take_dirty().is_empty(),
            "row bounds read was pruned, size read remains"
        );
        geometry.update(node, measured(140.0));
        assert_eq!(geometry.take_dirty(), BTreeSet::from([owner.clone()]));

        refs = saved_refs;
        geometry.restore(saved_geometry);
        geometry.update(node, moved);
        assert_eq!(geometry.take_dirty(), BTreeSet::from([owner.clone()]));
        refs.reset_contribution(&row);
        geometry.reset_contribution(&row);
        refs.reset_contribution(&direct);
        geometry.reset_contribution(&direct);
        geometry.sync_ref_readers(&provider, refs.geometry_bindings(&provider));
        geometry.update(node, measured(160.0));
        assert!(geometry.take_dirty().is_empty());
        refs.reconcile(&provider, BTreeMap::new());
        assert!(refs.take_dirty().is_empty());
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
