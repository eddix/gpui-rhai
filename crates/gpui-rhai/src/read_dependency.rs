use crate::{ComponentInstancePath, VirtualCollectionId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum ReadContribution {
    Component,
    VirtualItem {
        collection: VirtualCollectionId,
        key: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct ReadDependency {
    pub owner: ComponentInstancePath,
    pub contribution: ReadContribution,
}

impl From<&ComponentInstancePath> for ReadDependency {
    fn from(owner: &ComponentInstancePath) -> Self {
        Self::component(owner)
    }
}
impl From<ComponentInstancePath> for ReadDependency {
    fn from(owner: ComponentInstancePath) -> Self {
        Self::component(&owner)
    }
}
impl From<&ReadDependency> for ReadDependency {
    fn from(reader: &ReadDependency) -> Self {
        reader.clone()
    }
}

impl ReadDependency {
    pub fn component(owner: &ComponentInstancePath) -> Self {
        Self {
            owner: owner.clone(),
            contribution: ReadContribution::Component,
        }
    }
    pub fn virtual_item(
        owner: &ComponentInstancePath,
        collection: &VirtualCollectionId,
        key: &str,
    ) -> Self {
        Self {
            owner: owner.clone(),
            contribution: ReadContribution::VirtualItem {
                collection: collection.clone(),
                key: key.to_owned(),
            },
        }
    }
    pub fn retained_in_owner_scope(
        &self,
        scope: &ComponentInstancePath,
        active: &BTreeSet<ComponentInstancePath>,
    ) -> bool {
        !self.owner.is_within(scope) || self.owner == *scope || active.contains(&self.owner)
    }
    pub fn retained_in_contribution_scope(
        &self,
        scope: &ComponentInstancePath,
        active: &BTreeSet<ReadContribution>,
    ) -> bool {
        match &self.contribution {
            ReadContribution::Component => true,
            ReadContribution::VirtualItem { collection, .. } => {
                !collection
                    .component
                    .child("VirtualCollection", collection.key.clone())
                    .is_within(scope)
                    || active.contains(&self.contribution)
            }
        }
    }
}

pub(crate) fn owners(
    readers: impl IntoIterator<Item = ReadDependency>,
) -> BTreeSet<ComponentInstancePath> {
    readers.into_iter().map(|reader| reader.owner).collect()
}

pub(crate) fn retain_readers<K: Ord>(
    index: &mut BTreeMap<K, BTreeSet<ReadDependency>>,
    predicate: impl Fn(&ReadDependency) -> bool,
) {
    for readers in index.values_mut() {
        readers.retain(&predicate);
    }
    index.retain(|_, readers| !readers.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ComponentStateSchema, ElementGeometry, GeometryBounds, GeometryRegistry, RetainedUiTree,
        StateField, StoreId, StoreRegistry, UiNode, UiValue, ValueSchema,
    };

    #[test]
    fn contribution_retention_is_shared_by_store_environment_and_geometry() {
        let owner = ComponentInstancePath::root("App", "reads");
        let collection = VirtualCollectionId {
            component: owner.clone(),
            key: "rows".into(),
        };
        let scope = owner.child("VirtualCollection", "rows");
        let direct = ReadDependency::component(&owner);
        let first = ReadDependency::virtual_item(&owner, &collection, "a");
        let second = ReadDependency::virtual_item(&owner, &collection, "b");
        let active = BTreeSet::from([first.contribution.clone()]);
        let mut stores = StoreRegistry::new();
        let id = StoreId::app("store");
        stores
            .declare(
                id.clone(),
                ComponentStateSchema::new(BTreeMap::from([(
                    "value".into(),
                    StateField::new(ValueSchema::integer(), UiValue::Integer(0)),
                )]))
                .unwrap(),
            )
            .unwrap();
        for reader in [&direct, &first, &second] {
            stores.read_dependency(reader, &id, "value").unwrap();
        }
        stores.retain_contributions(&scope, &active);
        stores.reset_contribution(&direct);
        assert_eq!(
            stores.write(&id, "value", UiValue::Integer(1)).unwrap(),
            BTreeSet::from([owner.clone()])
        );
        stores.reset_contribution(&first);
        assert!(
            stores
                .write(&id, "value", UiValue::Integer(2))
                .unwrap()
                .is_empty()
        );

        let mut environment =
            crate::environment_dependency::EnvironmentDependencyRegistry::default();
        for reader in [&direct, &first, &second] {
            environment.track_locale(Some("main"), reader);
            environment.track_theme(Some("main"), reader);
            environment.track_viewport("main", reader);
        }
        environment.retain_contributions(&scope, &active);
        environment.reset_contribution(&direct);
        assert_eq!(
            environment.invalidate_theme_app(),
            BTreeSet::from([owner.clone()])
        );
        environment.reset_contribution(&first);
        assert!(environment.invalidate_locale_app().is_empty());
        assert!(environment.invalidate_viewport("main").is_empty());

        let mut tree = RetainedUiTree::new();
        tree.reconcile(UiNode::text("target")).unwrap();
        let node = tree.root_id().unwrap();
        let geometry = GeometryRegistry::new();
        for reader in [&direct, &first, &second] {
            geometry.read_tracked(node, reader);
        }
        geometry.retain_contributions(&scope, &active);
        geometry.reset_contribution(&direct);
        let bounds = GeometryBounds::new(0.0, 0.0, 100.0, 20.0).unwrap();
        geometry.update(
            node,
            ElementGeometry {
                layout: bounds,
                visual: bounds,
                clip: None,
            },
        );
        assert_eq!(geometry.take_dirty(), BTreeSet::from([owner]));
        geometry.reset_contribution(&first);
        let moved = GeometryBounds::new(1.0, 0.0, 100.0, 20.0).unwrap();
        geometry.update(
            node,
            ElementGeometry {
                layout: moved,
                visual: moved,
                clip: None,
            },
        );
        assert!(geometry.take_dirty().is_empty());
    }
}
