use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ComponentInstancePath, ComponentStateSchema, SchemaValidationError, UiValue};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "scope", content = "id", rename_all = "snake_case")]
pub enum StoreScope {
    App,
    Window(String),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct StoreId {
    pub scope: StoreScope,
    pub name: String,
}

impl StoreId {
    #[must_use]
    pub fn app(name: impl Into<String>) -> Self {
        Self {
            scope: StoreScope::App,
            name: name.into(),
        }
    }

    #[must_use]
    pub fn window(window: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            scope: StoreScope::Window(window.into()),
            name: name.into(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct StoreRegistry {
    stores: BTreeMap<StoreId, StoreState>,
    readers: BTreeMap<StoreField, BTreeSet<ComponentInstancePath>>,
}

#[derive(Clone, Debug)]
struct StoreState {
    schema: ComponentStateSchema,
    values: BTreeMap<String, UiValue>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct StoreField {
    store: StoreId,
    field: String,
}

impl StoreRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare an application- or window-scoped typed store.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::DuplicateStore`] when the ID already exists.
    pub fn declare(&mut self, id: StoreId, schema: ComponentStateSchema) -> Result<(), StoreError> {
        if self.stores.contains_key(&id) {
            return Err(StoreError::DuplicateStore(id));
        }
        let values = schema
            .fields()
            .iter()
            .map(|(name, field)| (name.clone(), field.default.clone()))
            .collect();
        self.stores.insert(id, StoreState { schema, values });
        Ok(())
    }

    /// Begin tracking reads for one component render.
    ///
    /// Previous subscriptions for the component are removed first so
    /// conditional reads cannot leave stale invalidation edges.
    pub fn begin_read<'a>(&'a mut self, reader: &'a ComponentInstancePath) -> StoreReadSession<'a> {
        self.reset_reader(reader);
        StoreReadSession {
            registry: self,
            reader,
        }
    }

    pub fn reset_reader(&mut self, reader: &ComponentInstancePath) {
        for readers in self.readers.values_mut() {
            readers.remove(reader);
        }
        self.readers.retain(|_, readers| !readers.is_empty());
    }

    /// Remove stale reader edges inside one successfully reconciled subtree.
    pub fn retain_reader_scope(
        &mut self,
        root: &ComponentInstancePath,
        active: &BTreeSet<ComponentInstancePath>,
    ) {
        for readers in self.readers.values_mut() {
            readers.retain(|reader| {
                !reader.is_within(root) || reader == root || active.contains(reader)
            });
        }
        self.readers.retain(|_, readers| !readers.is_empty());
    }

    /// Read a field and add one dependency without clearing earlier reads.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::UnknownStore`] or [`StoreError::UnknownField`].
    pub fn read_tracked(
        &mut self,
        reader: &ComponentInstancePath,
        store: &StoreId,
        field: &str,
    ) -> Result<UiValue, StoreError> {
        let value = self
            .stores
            .get(store)
            .ok_or_else(|| StoreError::UnknownStore(store.clone()))?
            .values
            .get(field)
            .cloned()
            .ok_or_else(|| StoreError::UnknownField {
                store: store.clone(),
                field: field.to_owned(),
            })?;
        self.readers
            .entry(StoreField {
                store: store.clone(),
                field: field.to_owned(),
            })
            .or_default()
            .insert(reader.clone());
        Ok(value)
    }

    /// Validate and update a store field, returning only subscribed components.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::UnknownStore`], [`StoreError::UnknownField`], or
    /// [`StoreError::InvalidValue`].
    pub fn write(
        &mut self,
        store: &StoreId,
        field: &str,
        value: UiValue,
    ) -> Result<BTreeSet<ComponentInstancePath>, StoreError> {
        let state = self
            .stores
            .get_mut(store)
            .ok_or_else(|| StoreError::UnknownStore(store.clone()))?;
        let declared = state
            .schema
            .field(field)
            .ok_or_else(|| StoreError::UnknownField {
                store: store.clone(),
                field: field.to_owned(),
            })?;
        declared
            .schema
            .validate(&value.clone().into_dynamic())
            .map_err(|source| StoreError::InvalidValue {
                store: store.clone(),
                field: field.to_owned(),
                source,
            })?;

        if state.values.get(field) == Some(&value) {
            return Ok(BTreeSet::new());
        }
        state.values.insert(field.to_owned(), value);
        Ok(self
            .readers
            .get(&StoreField {
                store: store.clone(),
                field: field.to_owned(),
            })
            .cloned()
            .unwrap_or_default())
    }

    /// Remove a window and all stores/read dependencies scoped to it.
    pub fn remove_window(&mut self, window: &str) {
        self.stores
            .retain(|id, _| !matches!(&id.scope, StoreScope::Window(id) if id == window));
        self.readers.retain(
            |field, _| !matches!(&field.store.scope, StoreScope::Window(id) if id == window),
        );
    }

    #[must_use]
    pub fn is_sensitive(&self, store: &StoreId, field: &str) -> bool {
        self.stores
            .get(store)
            .and_then(|state| state.schema.field(field))
            .is_some_and(|field| field.sensitive)
    }

    #[must_use]
    pub fn inspect(&self) -> Vec<StoreSnapshot> {
        self.stores
            .iter()
            .map(|(id, state)| StoreSnapshot {
                id: id.clone(),
                fields: state
                    .values
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.clone(),
                            crate::StateValueSnapshot {
                                value: value.clone(),
                                sensitive: state
                                    .schema
                                    .field(name)
                                    .is_some_and(|field| field.sensitive),
                            },
                        )
                    })
                    .collect(),
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoreSnapshot {
    pub id: StoreId,
    pub fields: BTreeMap<String, crate::StateValueSnapshot>,
}

pub struct StoreReadSession<'a> {
    registry: &'a mut StoreRegistry,
    reader: &'a ComponentInstancePath,
}

impl StoreReadSession<'_> {
    /// Read a field and subscribe the current component to that exact field.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::UnknownStore`] or [`StoreError::UnknownField`].
    pub fn read(&mut self, store: &StoreId, field: &str) -> Result<UiValue, StoreError> {
        self.registry.read_tracked(self.reader, store, field)
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum StoreError {
    #[error("store `{0:?}` is already declared")]
    DuplicateStore(StoreId),
    #[error("store `{0:?}` is not declared")]
    UnknownStore(StoreId),
    #[error("field `{field}` is not declared in store `{store:?}`")]
    UnknownField { store: StoreId, field: String },
    #[error("value for `{store:?}.{field}` is invalid: {source}")]
    InvalidValue {
        store: StoreId,
        field: String,
        source: SchemaValidationError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StateField, ValueSchema};

    fn session_schema() -> ComponentStateSchema {
        ComponentStateSchema::new(BTreeMap::from([
            (
                "project".to_owned(),
                StateField::new(ValueSchema::string(), UiValue::String("alpha".to_owned())),
            ),
            (
                "sidebar_open".to_owned(),
                StateField::new(ValueSchema::Bool, UiValue::Bool(true)),
            ),
        ]))
        .unwrap()
    }

    #[test]
    fn writes_invalidate_only_exact_field_readers() {
        let store_id = StoreId::app("session");
        let project = ComponentInstancePath::root("ProjectView", "project");
        let sidebar = ComponentInstancePath::root("Sidebar", "sidebar");
        let mut stores = StoreRegistry::new();
        stores.declare(store_id.clone(), session_schema()).unwrap();

        stores
            .begin_read(&project)
            .read(&store_id, "project")
            .unwrap();
        stores
            .begin_read(&sidebar)
            .read(&store_id, "sidebar_open")
            .unwrap();

        let invalidated = stores
            .write(&store_id, "project", UiValue::String("beta".to_owned()))
            .unwrap();
        assert_eq!(invalidated, BTreeSet::from([project]));
    }

    #[test]
    fn repeated_value_does_not_invalidate() {
        let store_id = StoreId::app("session");
        let reader = ComponentInstancePath::root("App", "root");
        let mut stores = StoreRegistry::new();
        stores.declare(store_id.clone(), session_schema()).unwrap();
        stores
            .begin_read(&reader)
            .read(&store_id, "project")
            .unwrap();

        assert!(
            stores
                .write(&store_id, "project", UiValue::String("alpha".to_owned()),)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn removing_window_clears_window_store() {
        let store_id = StoreId::window("settings", "draft");
        let mut stores = StoreRegistry::new();
        stores.declare(store_id.clone(), session_schema()).unwrap();
        stores.remove_window("settings");
        assert!(matches!(
            stores.write(&store_id, "sidebar_open", UiValue::Bool(false)),
            Err(StoreError::UnknownStore(_))
        ));
    }

    #[test]
    fn successful_subtree_reconcile_removes_stale_app_store_readers() {
        let id = StoreId::app("shared");
        let root = ComponentInstancePath::root("App", "main");
        let removed = root.child("Panel", "removed");
        let mut stores = StoreRegistry::new();
        stores.declare(id.clone(), session_schema()).unwrap();
        stores.read_tracked(&removed, &id, "sidebar_open").unwrap();
        stores.retain_reader_scope(&root, &BTreeSet::new());
        assert!(
            stores
                .write(&id, "sidebar_open", UiValue::Bool(false))
                .unwrap()
                .is_empty()
        );
    }
}
