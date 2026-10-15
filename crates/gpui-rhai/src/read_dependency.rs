use crate::{ComponentInstancePath, VirtualCollectionId};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MAX_MISSING_NAMES_PER_READER: usize = 64;
pub(crate) const MAX_MISSING_READER_PAIRS: usize = 4_096;
pub(crate) const MAX_MISSING_NAME_BYTES: usize = 64 * 1024;

pub(crate) struct MissingReadBudget {
    pub kind: &'static str,
    pub limit: usize,
}

/// Shared bounded policy for caught lookups of not-yet-registered Host data.
pub(crate) fn track_missing_reader(
    index: &mut BTreeMap<String, BTreeSet<ReadDependency>>,
    reader: &ReadDependency,
    name: &str,
) -> Result<(), MissingReadBudget> {
    if index
        .get(name)
        .is_some_and(|readers| readers.contains(reader))
    {
        return Ok(());
    }
    if index
        .values()
        .filter(|readers| readers.contains(reader))
        .count()
        >= MAX_MISSING_NAMES_PER_READER
    {
        return Err(MissingReadBudget {
            kind: "names per component",
            limit: MAX_MISSING_NAMES_PER_READER,
        });
    }
    if index.values().map(BTreeSet::len).sum::<usize>() >= MAX_MISSING_READER_PAIRS {
        return Err(MissingReadBudget {
            kind: "component/name pairs",
            limit: MAX_MISSING_READER_PAIRS,
        });
    }
    if !index.contains_key(name)
        && index
            .keys()
            .map(String::len)
            .sum::<usize>()
            .saturating_add(name.len())
            > MAX_MISSING_NAME_BYTES
    {
        return Err(MissingReadBudget {
            kind: "name bytes",
            limit: MAX_MISSING_NAME_BYTES,
        });
    }
    index
        .entry(name.to_owned())
        .or_default()
        .insert(reader.clone());
    Ok(())
}

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
    fn shared_missing_read_index_bounds_global_pairs_and_name_bytes() {
        let mut index = BTreeMap::new();
        for reader in 0..64 {
            let owner = ReadDependency::component(&ComponentInstancePath::root(
                "View",
                format!("reader-{reader}"),
            ));
            for name in 0..64 {
                track_missing_reader(&mut index, &owner, &format!("name-{name}"))
                    .unwrap_or_else(|_| panic!("within budget"));
            }
        }
        let overflow = ReadDependency::component(&ComponentInstancePath::root("View", "overflow"));
        assert!(matches!(
            track_missing_reader(&mut index, &overflow, "name-0"),
            Err(MissingReadBudget {
                kind: "component/name pairs",
                ..
            })
        ));
        assert_eq!(
            index.values().map(BTreeSet::len).sum::<usize>(),
            MAX_MISSING_READER_PAIRS
        );
        let mut index = BTreeMap::new();
        for number in 0..256 {
            let owner = ReadDependency::component(&ComponentInstancePath::root(
                "View",
                format!("byte-reader-{number}"),
            ));
            let name = format!("{number:04}-{}", "x".repeat(251));
            track_missing_reader(&mut index, &owner, &name)
                .unwrap_or_else(|_| panic!("within name budget"));
        }
        assert_eq!(
            index.keys().map(String::len).sum::<usize>(),
            MAX_MISSING_NAME_BYTES
        );
        assert!(matches!(
            track_missing_reader(&mut index, &overflow, "more-bytes"),
            Err(MissingReadBudget {
                kind: "name bytes",
                ..
            })
        ));
    }

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
            stores.read_dependency(Some(reader), &id, "value").unwrap();
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
