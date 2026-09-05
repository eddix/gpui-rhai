use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{SchemaDefinitionError, SchemaValidationError, UiValue, ValueSchema};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ComponentInstancePath(Vec<ComponentInstanceSegment>);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
struct ComponentInstanceSegment {
    component: String,
    key: String,
}

impl ComponentInstancePath {
    #[must_use]
    pub fn root(component: impl Into<String>, key: impl Into<String>) -> Self {
        Self(vec![ComponentInstanceSegment {
            component: component.into(),
            key: key.into(),
        }])
    }

    #[must_use]
    pub fn child(&self, component: impl Into<String>, key: impl Into<String>) -> Self {
        let mut segments = self.0.clone();
        segments.push(ComponentInstanceSegment {
            component: component.into(),
            key: key.into(),
        });
        Self(segments)
    }

    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        if self.0.len() <= 1 {
            return None;
        }
        let mut segments = self.0.clone();
        segments.pop();
        Some(Self(segments))
    }

    #[must_use]
    pub fn is_within(&self, ancestor: &Self) -> bool {
        self.0.starts_with(&ancestor.0)
    }

    pub(crate) fn single_root_key(&self, component: &str) -> Option<&str> {
        (self.0.len() == 1 && self.0[0].component == component).then_some(self.0[0].key.as_str())
    }
}

impl fmt::Display for ComponentInstancePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for segment in &self.0 {
            write!(formatter, "/{}[{}]", segment.component, segment.key)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StateField {
    pub schema: ValueSchema,
    pub default: UiValue,
    #[serde(default)]
    pub sensitive: bool,
}

impl StateField {
    #[must_use]
    pub fn new(schema: ValueSchema, default: UiValue) -> Self {
        Self {
            schema,
            default,
            sensitive: false,
        }
    }

    #[must_use]
    pub const fn sensitive(mut self, sensitive: bool) -> Self {
        self.sensitive = sensitive;
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComponentStateSchema {
    fields: BTreeMap<String, StateField>,
}

impl ComponentStateSchema {
    /// Create a state schema after validating every declared default.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::InvalidDefault`] when a default does not satisfy
    /// its field schema.
    pub fn new(fields: BTreeMap<String, StateField>) -> Result<Self, StateError> {
        for (name, field) in &fields {
            field
                .schema
                .validate_definition()
                .map_err(|source| StateError::InvalidSchema {
                    field: name.clone(),
                    source,
                })?;
            field
                .schema
                .validate(&field.default.clone().into_dynamic())
                .map_err(|source| StateError::InvalidDefault {
                    field: name.clone(),
                    source,
                })?;
        }
        Ok(Self { fields })
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    #[must_use]
    pub fn field(&self, name: &str) -> Option<&StateField> {
        self.fields.get(name)
    }

    #[must_use]
    pub fn fields(&self) -> &BTreeMap<String, StateField> {
        &self.fields
    }

    fn defaults(&self) -> BTreeMap<String, UiValue> {
        self.fields
            .iter()
            .map(|(name, field)| (name.clone(), field.default.clone()))
            .collect()
    }
}

#[derive(Clone, Debug, Default)]
pub struct StateStore {
    instances: BTreeMap<ComponentInstancePath, ComponentState>,
}

#[derive(Clone, Debug)]
struct ComponentState {
    schema: ComponentStateSchema,
    values: BTreeMap<String, UiValue>,
}

impl StateStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Start an isolated render-state transaction.
    ///
    /// Dropping the transaction leaves the committed store unchanged. Passing
    /// it to [`StateStore::commit_render`] installs compatible state and removes
    /// instances not reached during the successful render.
    #[must_use]
    pub fn begin_render(&self) -> RenderStateTransaction {
        RenderStateTransaction {
            instances: self.instances.clone(),
            seen: BTreeSet::new(),
            scope: None,
        }
    }

    /// Start a transaction that reconciles only one component subtree.
    /// State belonging to other window roots remains untouched on commit.
    #[must_use]
    pub fn begin_render_scope(&self, scope: ComponentInstancePath) -> RenderStateTransaction {
        RenderStateTransaction {
            instances: self.instances.clone(),
            seen: BTreeSet::new(),
            scope: Some(scope),
        }
    }

    pub fn commit_render(&mut self, mut transaction: RenderStateTransaction) {
        transaction.instances.retain(|path, _| {
            transaction.seen.contains(path)
                || transaction
                    .scope
                    .as_ref()
                    .is_some_and(|scope| !path.is_within(scope))
        });
        self.instances = transaction.instances;
    }

    /// Remove one component subtree, including every descendant instance.
    pub fn remove_scope(&mut self, scope: &ComponentInstancePath) {
        self.instances.retain(|path, _| !path.is_within(scope));
    }

    /// Mount or reconcile one instance without cleaning any other path.
    /// Used to make candidate component defaults readable during `view`.
    ///
    /// # Errors
    ///
    /// Returns invalid-schema/default errors.
    pub fn mount_instance(
        &mut self,
        path: ComponentInstancePath,
        schema: &ComponentStateSchema,
    ) -> Result<StateReconcileReport, StateError> {
        let mut transaction = self.begin_render();
        let report = transaction.mount(path, schema)?;
        self.instances = transaction.instances;
        Ok(report)
    }

    #[must_use]
    pub fn get(&self, path: &ComponentInstancePath, field: &str) -> Option<&UiValue> {
        self.instances.get(path)?.values.get(field)
    }

    /// Update a declared field on a committed component instance.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::UnknownInstance`], [`StateError::UnknownField`], or
    /// [`StateError::InvalidValue`] when the update violates committed state.
    pub fn set(
        &mut self,
        path: &ComponentInstancePath,
        field: &str,
        value: UiValue,
    ) -> Result<bool, StateError> {
        let state = self
            .instances
            .get_mut(path)
            .ok_or_else(|| StateError::UnknownInstance(path.clone()))?;
        let state_field = state
            .schema
            .field(field)
            .ok_or_else(|| StateError::UnknownField {
                path: path.clone(),
                field: field.to_owned(),
            })?;
        state_field
            .schema
            .validate(&value.clone().into_dynamic())
            .map_err(|source| StateError::InvalidValue {
                path: path.clone(),
                field: field.to_owned(),
                source,
            })?;
        if state.values.get(field) == Some(&value) {
            return Ok(false);
        }
        state.values.insert(field.to_owned(), value);
        Ok(true)
    }

    #[must_use]
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    pub(crate) fn contains_instance(&self, path: &ComponentInstancePath) -> bool {
        self.instances.contains_key(path)
    }

    #[must_use]
    pub fn paths(&self) -> BTreeSet<ComponentInstancePath> {
        self.instances.keys().cloned().collect()
    }

    #[must_use]
    pub fn is_sensitive(&self, path: &ComponentInstancePath, field: &str) -> bool {
        self.instances
            .get(path)
            .and_then(|state| state.schema.field(field))
            .is_some_and(|field| field.sensitive)
    }

    #[must_use]
    pub fn inspect(&self) -> Vec<StateInstanceSnapshot> {
        self.instances
            .iter()
            .map(|(path, state)| StateInstanceSnapshot {
                path: path.clone(),
                fields: state
                    .values
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.clone(),
                            StateValueSnapshot {
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
pub struct StateInstanceSnapshot {
    pub path: ComponentInstancePath,
    pub fields: BTreeMap<String, StateValueSnapshot>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateValueSnapshot {
    pub value: UiValue,
    pub sensitive: bool,
}

#[derive(Clone, Debug)]
pub struct RenderStateTransaction {
    instances: BTreeMap<ComponentInstancePath, ComponentState>,
    seen: BTreeSet<ComponentInstancePath>,
    scope: Option<ComponentInstancePath>,
}

impl RenderStateTransaction {
    /// Retain a previously mounted instance without changing its schema.
    #[must_use]
    pub fn retain_existing(&mut self, path: &ComponentInstancePath) -> bool {
        if self.instances.contains_key(path) {
            self.seen.insert(path.clone());
            true
        } else {
            false
        }
    }

    /// Mount or reconcile an instance inside this prospective render.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::InvalidDefault`] if a programmatically constructed
    /// schema contains an invalid default.
    pub fn mount(
        &mut self,
        path: ComponentInstancePath,
        schema: &ComponentStateSchema,
    ) -> Result<StateReconcileReport, StateError> {
        // Revalidate here because deserialized schemas can bypass `new`.
        ComponentStateSchema::new(schema.fields.clone())?;

        let report = match self.instances.get(&path) {
            None => {
                self.instances.insert(
                    path.clone(),
                    ComponentState {
                        schema: schema.clone(),
                        values: schema.defaults(),
                    },
                );
                StateReconcileReport {
                    created: true,
                    reset_fields: Vec::new(),
                }
            }
            Some(previous) => {
                let mut reset_fields = Vec::new();
                let values = schema
                    .fields
                    .iter()
                    .map(|(name, field)| {
                        let value = previous
                            .values
                            .get(name)
                            .filter(|value| {
                                field
                                    .schema
                                    .validate(&(*value).clone().into_dynamic())
                                    .is_ok()
                            })
                            .cloned()
                            .unwrap_or_else(|| {
                                if previous.values.contains_key(name) {
                                    reset_fields.push(name.clone());
                                }
                                field.default.clone()
                            });
                        (name.clone(), value)
                    })
                    .collect();
                self.instances.insert(
                    path.clone(),
                    ComponentState {
                        schema: schema.clone(),
                        values,
                    },
                );
                StateReconcileReport {
                    created: false,
                    reset_fields,
                }
            }
        };
        self.seen.insert(path);
        Ok(report)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StateReconcileReport {
    pub created: bool,
    pub reset_fields: Vec<String>,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum StateError {
    #[error("schema for state field `{field}` is invalid: {source}")]
    InvalidSchema {
        field: String,
        source: SchemaDefinitionError,
    },
    #[error("default for state field `{field}` is invalid: {source}")]
    InvalidDefault {
        field: String,
        source: SchemaValidationError,
    },
    #[error("component state instance `{0}` does not exist")]
    UnknownInstance(ComponentInstancePath),
    #[error("state field `{field}` is not declared for component `{path}`")]
    UnknownField {
        path: ComponentInstancePath,
        field: String,
    },
    #[error("state value for `{path}.{field}` is invalid: {source}")]
    InvalidValue {
        path: ComponentInstancePath,
        field: String,
        source: SchemaValidationError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(value_schema: ValueSchema, default: UiValue) -> ComponentStateSchema {
        ComponentStateSchema::new(BTreeMap::from([(
            "value".to_owned(),
            StateField::new(value_schema, default),
        )]))
        .unwrap()
    }

    #[test]
    fn failed_render_does_not_change_committed_state() {
        let path = ComponentInstancePath::root("App", "root").child("Combobox", "country");
        let bool_schema = schema(ValueSchema::Bool, UiValue::Bool(false));
        let mut store = StateStore::new();

        let mut initial = store.begin_render();
        initial.mount(path.clone(), &bool_schema).unwrap();
        store.commit_render(initial);
        store.set(&path, "value", UiValue::Bool(true)).unwrap();

        let mut failed = store.begin_render();
        failed
            .mount(
                path.clone(),
                &schema(
                    ValueSchema::enumeration(["new"]),
                    UiValue::String("new".to_owned()),
                ),
            )
            .unwrap();
        drop(failed);

        assert_eq!(store.get(&path, "value"), Some(&UiValue::Bool(true)));
    }

    #[test]
    fn successful_render_resets_only_incompatible_fields() {
        let path = ComponentInstancePath::root("App", "root");
        let mut store = StateStore::new();
        let bool_schema = schema(ValueSchema::Bool, UiValue::Bool(false));
        let mut initial = store.begin_render();
        initial.mount(path.clone(), &bool_schema).unwrap();
        store.commit_render(initial);
        store.set(&path, "value", UiValue::Bool(true)).unwrap();

        let string_schema = schema(ValueSchema::string(), UiValue::String("reset".to_owned()));
        let mut reload = store.begin_render();
        let report = reload.mount(path.clone(), &string_schema).unwrap();
        store.commit_render(reload);

        assert_eq!(report.reset_fields, vec!["value"]);
        assert_eq!(
            store.get(&path, "value"),
            Some(&UiValue::String("reset".to_owned()))
        );
    }

    #[test]
    fn successful_render_cleans_unreachable_instances() {
        let first = ComponentInstancePath::root("App", "root").child("Row", "first");
        let second = ComponentInstancePath::root("App", "root").child("Row", "second");
        let state_schema = schema(ValueSchema::integer(), UiValue::Integer(0));
        let mut store = StateStore::new();
        let mut initial = store.begin_render();
        initial.mount(first.clone(), &state_schema).unwrap();
        initial.mount(second, &state_schema).unwrap();
        store.commit_render(initial);
        assert_eq!(store.instance_count(), 2);

        let mut next = store.begin_render();
        next.mount(first, &state_schema).unwrap();
        store.commit_render(next);
        assert_eq!(store.instance_count(), 1);
    }

    #[test]
    fn updates_are_schema_checked() {
        let path = ComponentInstancePath::root("App", "root");
        let state_schema = schema(ValueSchema::Bool, UiValue::Bool(false));
        let mut store = StateStore::new();
        let mut render = store.begin_render();
        render.mount(path.clone(), &state_schema).unwrap();
        store.commit_render(render);

        assert!(matches!(
            store.set(&path, "value", UiValue::String("no".to_owned())),
            Err(StateError::InvalidValue { .. })
        ));
    }

    #[test]
    fn scoped_commit_preserves_other_window_roots_and_remove_scope_cleans_one() {
        let state_schema = schema(ValueSchema::string(), UiValue::String("ready".to_owned()));
        let main = ComponentInstancePath::root("App", "main");
        let settings = ComponentInstancePath::root("App", "settings");
        let mut store = StateStore::new();
        let mut initial = store.begin_render();
        initial.mount(main.clone(), &state_schema).unwrap();
        initial.mount(settings.clone(), &state_schema).unwrap();
        store.commit_render(initial);

        let mut main_render = store.begin_render_scope(main.clone());
        main_render.mount(main.clone(), &state_schema).unwrap();
        store.commit_render(main_render);
        assert!(store.get(&settings, "value").is_some());

        store.remove_scope(&main);
        assert!(store.get(&main, "value").is_none());
        assert!(store.get(&settings, "value").is_some());
    }
}
