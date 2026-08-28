use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use rhai::{Dynamic, Engine, EvalAltResult, FuncRegistration, Map, Position};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    ComponentStateSchema, ModuleId, ObjectField, RUNTIME_API_VERSION, SchemaValidationError,
    ValueSchema,
};

const HEADER_START: &str = "/* gpui-rhai\n";
const HEADER_END: &str = "\n*/";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeApiRange {
    pub min_inclusive: u32,
    pub max_exclusive: u32,
}

impl RuntimeApiRange {
    #[must_use]
    pub const fn new(min_inclusive: u32, max_exclusive: u32) -> Self {
        Self {
            min_inclusive,
            max_exclusive,
        }
    }

    #[must_use]
    pub const fn contains(self, version: u32) -> bool {
        self.min_inclusive <= version && version < self.max_exclusive
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.min_inclusive < self.max_exclusive
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComponentMetadata {
    pub id: ModuleId,
    pub export: String,
    pub version: Version,
    pub runtime_api: RuntimeApiRange,
    #[serde(default)]
    pub dependencies: BTreeSet<ModuleId>,
    #[serde(default)]
    pub capabilities: BTreeMap<String, VersionReq>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComponentSchema {
    #[serde(default)]
    pub props: BTreeMap<String, crate::ObjectField>,
    #[serde(default)]
    pub state: ComponentStateSchema,
    #[serde(default)]
    pub events: BTreeMap<String, EventSchema>,
    #[serde(default)]
    pub slots: BTreeMap<String, SlotSchema>,
    #[serde(default)]
    pub parts: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventSchema {
    pub payload: ValueSchema,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SlotSchema {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub multiple: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentDefinition {
    pub metadata: ComponentMetadata,
    pub schema: ComponentSchema,
}

impl ComponentDefinition {
    /// Construct and validate a formal component definition.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentError`] when names, compatibility ranges, defaults,
    /// event callbacks, slots, parts, or capabilities violate the component
    /// contract.
    pub fn new(
        metadata: ComponentMetadata,
        mut schema: ComponentSchema,
    ) -> Result<Self, ComponentError> {
        install_standard_style_props(&mut schema)?;
        validate_metadata(&metadata)?;
        validate_schema(&schema)?;
        Ok(Self { metadata, schema })
    }

    /// Validate and normalize invocation props, applying declared defaults.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentError::MissingKey`] for an unkeyed stateful instance,
    /// or [`ComponentError::InvalidProps`] when props violate the schema.
    pub fn invoke(
        &self,
        key: Option<String>,
        mut props: Map,
    ) -> Result<ComponentInvocation, ComponentError> {
        if !self.schema.state.is_empty() && key.is_none() {
            return Err(ComponentError::MissingKey {
                component: self.metadata.id.clone(),
            });
        }

        for (name, field) in &self.schema.props {
            if !props.contains_key(name.as_str())
                && let Some(default) = &field.default
            {
                props.insert(name.clone().into(), default.clone().into_dynamic());
            }
        }

        ValueSchema::object(self.schema.props.clone())
            .validate(&Dynamic::from_map(props.clone()))
            .map_err(|source| ComponentError::InvalidProps {
                component: self.metadata.id.clone(),
                source,
            })?;

        if let Some(part_styles) = props.get("part_styles") {
            let part_styles = part_styles.clone_cast::<Map>();
            if let Some(part) = part_styles
                .keys()
                .find(|part| !self.schema.parts.contains(part.as_str()))
            {
                return Err(ComponentError::UnknownStylePart {
                    component: self.metadata.id.clone(),
                    part: part.to_string(),
                });
            }
        }

        Ok(ComponentInvocation {
            component: self.metadata.id.clone(),
            export: self.metadata.export.clone(),
            key,
            props,
        })
    }

    /// Validate a declared semantic event payload.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentError::UnknownEvent`] or
    /// [`ComponentError::InvalidEventPayload`].
    pub fn validate_event(&self, event: &str, payload: &Dynamic) -> Result<(), ComponentError> {
        let event_schema =
            self.schema
                .events
                .get(event)
                .ok_or_else(|| ComponentError::UnknownEvent {
                    component: self.metadata.id.clone(),
                    event: event.to_owned(),
                })?;
        event_schema.payload.validate(payload).map_err(|source| {
            ComponentError::InvalidEventPayload {
                component: self.metadata.id.clone(),
                event: event.to_owned(),
                source,
            }
        })
    }

    /// Confirm that the source header describes this exported definition.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentError::HeaderMismatch`] when any metadata differs.
    pub fn validate_header(&self, header: &ComponentMetadata) -> Result<(), ComponentError> {
        if &self.metadata == header {
            Ok(())
        } else {
            Err(ComponentError::HeaderMismatch {
                header: Box::new(header.clone()),
                exported: Box::new(self.metadata.clone()),
            })
        }
    }
}

fn install_standard_style_props(schema: &mut ComponentSchema) -> Result<(), ComponentError> {
    match schema.props.get("key") {
        Some(field) if !matches!(field.schema, ValueSchema::String { .. }) => {
            return Err(ComponentError::InvalidStandardKeyProp);
        }
        Some(_) => {}
        None => {
            schema.props.insert(
                "key".to_owned(),
                ObjectField::optional(ValueSchema::string()),
            );
        }
    }
    let standard = [
        ("style", ObjectField::optional(ValueSchema::Style)),
        (
            "part_styles",
            ObjectField::optional(ValueSchema::Map {
                values: Box::new(ValueSchema::Style),
            }),
        ),
    ];
    for (name, expected) in standard {
        match schema.props.get(name) {
            Some(actual) if actual != &expected => {
                return Err(ComponentError::InvalidStandardStyleProp(name.to_owned()));
            }
            Some(_) => {}
            None => {
                schema.props.insert(name.to_owned(), expected);
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ComponentInvocation {
    pub component: ModuleId,
    pub export: String,
    pub key: Option<String>,
    pub props: Map,
}

#[derive(Clone, Debug, Default)]
pub struct ComponentRegistry {
    components: BTreeMap<ModuleId, ComponentDefinition>,
}

/// Installs `export_component` into Rhai and accumulates validated definitions.
#[derive(Clone, Debug, Default)]
pub struct ComponentExportCollector {
    registry: Arc<Mutex<ComponentRegistry>>,
}

impl ComponentExportCollector {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_into(&self, engine: &mut Engine) {
        let registry = Arc::clone(&self.registry);
        FuncRegistration::new("export_component")
            .in_global_namespace()
            .register_into_engine(
                engine,
                move |raw: Dynamic| -> Result<(), Box<EvalAltResult>> {
                    let decoded: ComponentDefinition = rhai::serde::from_dynamic(&raw)?;
                    let definition = ComponentDefinition::new(decoded.metadata, decoded.schema)
                        .map_err(|error| Box::new(export_runtime_error(error.to_string())))?;
                    registry
                        .lock()
                        .map_err(|_| {
                            Box::new(export_runtime_error(
                                "component export registry lock is poisoned".to_owned(),
                            ))
                        })?
                        .register(definition, RUNTIME_API_VERSION)
                        .map_err(|error| Box::new(export_runtime_error(error.to_string())))
                },
            );
    }

    /// Clone the current exported-component registry.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentExportError::Poisoned`] if a prior panic poisoned the
    /// collector lock.
    pub fn snapshot(&self) -> Result<ComponentRegistry, ComponentExportError> {
        self.registry
            .lock()
            .map(|registry| registry.clone())
            .map_err(|_| ComponentExportError::Poisoned)
    }

    /// Clear all collected component definitions.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentExportError::Poisoned`] if a prior panic poisoned the
    /// collector lock.
    pub fn clear(&self) -> Result<(), ComponentExportError> {
        *self
            .registry
            .lock()
            .map_err(|_| ComponentExportError::Poisoned)? = ComponentRegistry::new();
        Ok(())
    }

    /// Replace collected exports, used to roll back a failed reload candidate.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentExportError::Poisoned`] if the collector lock is poisoned.
    pub fn replace(&self, registry: ComponentRegistry) -> Result<(), ComponentExportError> {
        *self
            .registry
            .lock()
            .map_err(|_| ComponentExportError::Poisoned)? = registry;
        Ok(())
    }
}

impl ComponentRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a component compatible with the current runtime API.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentRegistryError`] for duplicate component IDs or an
    /// incompatible runtime API range.
    pub fn register(
        &mut self,
        definition: ComponentDefinition,
        runtime_api: u32,
    ) -> Result<(), ComponentRegistryError> {
        let id = definition.metadata.id.clone();
        if self.components.contains_key(&id) {
            return Err(ComponentRegistryError::Duplicate(id));
        }
        if !definition.metadata.runtime_api.contains(runtime_api) {
            return Err(ComponentRegistryError::IncompatibleRuntime {
                component: id,
                required: definition.metadata.runtime_api,
                actual: runtime_api,
            });
        }
        self.components.insert(id, definition);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, id: &ModuleId) -> Option<&ComponentDefinition> {
        self.components.get(id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.components.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ModuleId, &ComponentDefinition)> {
        self.components.iter()
    }

    /// Resolve dependencies in installation order, dependencies first.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentRegistryError::Missing`] or
    /// [`ComponentRegistryError::DependencyCycle`].
    pub fn resolve(
        &self,
        requested: impl IntoIterator<Item = ModuleId>,
    ) -> Result<Vec<ModuleId>, ComponentRegistryError> {
        let mut ordered = Vec::new();
        let mut complete = BTreeSet::new();
        let mut stack = Vec::new();
        for id in requested {
            self.visit(&id, &mut stack, &mut complete, &mut ordered)?;
        }
        Ok(ordered)
    }

    fn visit(
        &self,
        id: &ModuleId,
        stack: &mut Vec<ModuleId>,
        complete: &mut BTreeSet<ModuleId>,
        ordered: &mut Vec<ModuleId>,
    ) -> Result<(), ComponentRegistryError> {
        if complete.contains(id) {
            return Ok(());
        }
        if let Some(start) = stack.iter().position(|active| active == id) {
            let mut cycle = stack[start..].to_vec();
            cycle.push(id.clone());
            return Err(ComponentRegistryError::DependencyCycle(cycle));
        }
        let component = self
            .components
            .get(id)
            .ok_or_else(|| ComponentRegistryError::Missing(id.clone()))?;
        stack.push(id.clone());
        for dependency in &component.metadata.dependencies {
            self.visit(dependency, stack, complete, ordered)?;
        }
        stack.pop();
        complete.insert(id.clone());
        ordered.push(id.clone());
        Ok(())
    }
}

/// Parse the required JSON metadata block at the start of a component script.
///
/// # Errors
///
/// Returns [`ComponentHeaderError`] when the sentinel is absent, the block is
/// unterminated, or the JSON metadata is invalid.
pub fn parse_component_header(source: &str) -> Result<ComponentMetadata, ComponentHeaderError> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let body = source
        .strip_prefix(HEADER_START)
        .ok_or(ComponentHeaderError::Missing)?;
    let end = body
        .find(HEADER_END)
        .ok_or(ComponentHeaderError::Unterminated)?;
    serde_json::from_str(&body[..end]).map_err(ComponentHeaderError::InvalidJson)
}

fn validate_metadata(metadata: &ComponentMetadata) -> Result<(), ComponentError> {
    if !is_pascal_case_identifier(&metadata.export) {
        return Err(ComponentError::InvalidExport(metadata.export.clone()));
    }
    if !metadata.runtime_api.is_valid() {
        return Err(ComponentError::InvalidRuntimeRange(metadata.runtime_api));
    }
    if metadata.dependencies.contains(&metadata.id) {
        return Err(ComponentError::SelfDependency(metadata.id.clone()));
    }
    for name in metadata.capabilities.keys() {
        if !is_namespaced_identifier(name) {
            return Err(ComponentError::InvalidCapability(name.clone()));
        }
    }
    Ok(())
}

fn validate_schema(schema: &ComponentSchema) -> Result<(), ComponentError> {
    for (name, field) in &schema.props {
        if !is_snake_case_identifier(name) {
            return Err(ComponentError::InvalidSchemaName(name.clone()));
        }
        if field.required && field.default.is_some() {
            return Err(ComponentError::RequiredPropHasDefault(name.clone()));
        }
        if let Some(default) = &field.default {
            field
                .schema
                .validate(&default.clone().into_dynamic())
                .map_err(|source| ComponentError::InvalidPropDefault {
                    prop: name.clone(),
                    source,
                })?;
        }
    }
    for (name, event) in &schema.events {
        if !is_snake_case_identifier(name) {
            return Err(ComponentError::InvalidSchemaName(name.clone()));
        }
        let callback_name = format!("on_{name}");
        if !schema
            .props
            .get(&callback_name)
            .is_some_and(|field| schema_accepts_callback(&field.schema))
        {
            return Err(ComponentError::MissingEventCallback {
                event: name.clone(),
                prop: callback_name,
            });
        }
        let _ = event;
    }
    for (name, slot) in &schema.slots {
        if !is_snake_case_identifier(name) {
            return Err(ComponentError::InvalidSchemaName(name.clone()));
        }
        let valid = schema.props.get(name).is_some_and(|field| {
            if slot.multiple {
                matches!(
                    &field.schema,
                    ValueSchema::Array { items, .. } if matches!(items.as_ref(), ValueSchema::Node)
                )
            } else {
                schema_accepts_node(&field.schema)
            }
        });
        if !valid {
            return Err(ComponentError::MissingSlotProp(name.clone()));
        }
    }
    for part in &schema.parts {
        if !is_snake_case_identifier(part) {
            return Err(ComponentError::InvalidSchemaName(part.clone()));
        }
    }
    Ok(())
}

fn schema_accepts_callback(schema: &ValueSchema) -> bool {
    matches!(schema, ValueSchema::Callback)
        || matches!(schema, ValueSchema::Optional { value } if matches!(value.as_ref(), ValueSchema::Callback))
}

fn schema_accepts_node(schema: &ValueSchema) -> bool {
    matches!(schema, ValueSchema::Node)
        || matches!(schema, ValueSchema::Optional { value } if matches!(value.as_ref(), ValueSchema::Node))
}

fn is_pascal_case_identifier(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_uppercase())
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

fn is_snake_case_identifier(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('_')
        && !value.ends_with('_')
        && !value.contains("__")
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn is_namespaced_identifier(value: &str) -> bool {
    value.split_once('.').is_some_and(|(namespace, name)| {
        is_snake_case_identifier(namespace) && is_snake_case_identifier(name)
    })
}

fn export_runtime_error(error: String) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(Dynamic::from(error), Position::NONE)
}

#[derive(Debug, Error)]
pub enum ComponentError {
    #[error("component export `{0}` must be a PascalCase identifier")]
    InvalidExport(String),
    #[error("runtime API range {0:?} is empty")]
    InvalidRuntimeRange(RuntimeApiRange),
    #[error("component `{0}` cannot depend on itself")]
    SelfDependency(ModuleId),
    #[error("capability `{0}` must be a namespaced snake_case identifier")]
    InvalidCapability(String),
    #[error("schema name `{0}` must be a snake_case identifier")]
    InvalidSchemaName(String),
    #[error("required prop `{0}` cannot also declare a default")]
    RequiredPropHasDefault(String),
    #[error("standard style prop `{0}` has an incompatible schema")]
    InvalidStandardStyleProp(String),
    #[error("standard component key prop must be a string")]
    InvalidStandardKeyProp,
    #[error("default for prop `{prop}` is invalid: {source}")]
    InvalidPropDefault {
        prop: String,
        #[source]
        source: SchemaValidationError,
    },
    #[error("event `{event}` requires callback prop `{prop}`")]
    MissingEventCallback { event: String, prop: String },
    #[error("slot `{0}` requires a compatible node prop with the same name")]
    MissingSlotProp(String),
    #[error("stateful component `{component}` requires a stable key")]
    MissingKey { component: ModuleId },
    #[error("props for component `{component}` are invalid: {source}")]
    InvalidProps {
        component: ModuleId,
        #[source]
        source: SchemaValidationError,
    },
    #[error("component `{component}` does not declare style part `{part}`")]
    UnknownStylePart { component: ModuleId, part: String },
    #[error("component `{component}` does not declare event `{event}`")]
    UnknownEvent { component: ModuleId, event: String },
    #[error("payload for `{component}` event `{event}` is invalid: {source}")]
    InvalidEventPayload {
        component: ModuleId,
        event: String,
        #[source]
        source: SchemaValidationError,
    },
    #[error("component header metadata does not match exported metadata")]
    HeaderMismatch {
        header: Box<ComponentMetadata>,
        exported: Box<ComponentMetadata>,
    },
}

#[derive(Debug, Error)]
pub enum ComponentRegistryError {
    #[error("component `{0}` is already registered")]
    Duplicate(ModuleId),
    #[error("component `{component}` requires runtime API {required:?}, current API is {actual}")]
    IncompatibleRuntime {
        component: ModuleId,
        required: RuntimeApiRange,
        actual: u32,
    },
    #[error("component `{0}` is not registered")]
    Missing(ModuleId),
    #[error("component dependency cycle: {0:?}")]
    DependencyCycle(Vec<ModuleId>),
}

#[derive(Debug, Error)]
pub enum ComponentHeaderError {
    #[error("component source must start with `/* gpui-rhai` metadata")]
    Missing,
    #[error("component metadata block is missing its closing `*/`")]
    Unterminated,
    #[error("component metadata JSON is invalid: {0}")]
    InvalidJson(#[source] serde_json::Error),
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ComponentExportError {
    #[error("component export registry lock is poisoned")]
    Poisoned,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ObjectField, StateField, Style, UiValue};

    fn metadata(id: &str, export: &str) -> ComponentMetadata {
        ComponentMetadata {
            id: ModuleId::parse(id).unwrap(),
            export: export.to_owned(),
            version: Version::new(0, 1, 0),
            runtime_api: RuntimeApiRange::new(1, 2),
            dependencies: BTreeSet::new(),
            capabilities: BTreeMap::new(),
        }
    }

    fn button() -> ComponentDefinition {
        ComponentDefinition::new(
            metadata("components/button", "Button"),
            ComponentSchema {
                props: BTreeMap::from([
                    (
                        "text".to_owned(),
                        ObjectField::required(ValueSchema::string()),
                    ),
                    (
                        "variant".to_owned(),
                        ObjectField::optional(ValueSchema::enumeration(["primary", "secondary"]))
                            .with_default(UiValue::String("primary".to_owned())),
                    ),
                    (
                        "on_click".to_owned(),
                        ObjectField::optional(ValueSchema::optional(ValueSchema::Callback)),
                    ),
                ]),
                events: BTreeMap::from([(
                    "click".to_owned(),
                    EventSchema {
                        payload: ValueSchema::Null,
                    },
                )]),
                parts: BTreeSet::from(["root".to_owned(), "label".to_owned()]),
                ..ComponentSchema::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn invocation_applies_defaults_and_rejects_unknown_props() {
        let button = button();
        let invocation = button
            .invoke(
                None,
                Map::from_iter([("text".into(), Dynamic::from("Save"))]),
            )
            .unwrap();
        assert_eq!(
            invocation
                .props
                .get("variant")
                .unwrap()
                .clone_cast::<String>(),
            "primary"
        );

        let error = button
            .invoke(
                None,
                Map::from_iter([
                    ("text".into(), Dynamic::from("Save")),
                    ("lable".into(), Dynamic::from("typo")),
                ]),
            )
            .unwrap_err();
        assert!(matches!(error, ComponentError::InvalidProps { .. }));
    }

    #[test]
    fn standard_style_props_are_typed_and_reject_unknown_parts() {
        let component = button();
        assert!(matches!(
            component.schema.props["style"].schema,
            ValueSchema::Style
        ));
        let props = Map::from_iter([
            ("text".into(), Dynamic::from("Save")),
            (
                "part_styles".into(),
                Dynamic::from_map(Map::from_iter([(
                    "missing".into(),
                    Dynamic::from(Style::new()),
                )])),
            ),
        ]);
        assert!(matches!(
            component.invoke(None, props),
            Err(ComponentError::UnknownStylePart { part, .. }) if part == "missing"
        ));
    }

    #[test]
    fn stateful_components_require_keys() {
        let state = ComponentStateSchema::new(BTreeMap::from([(
            "open".to_owned(),
            StateField::new(ValueSchema::Bool, UiValue::Bool(false)),
        )]))
        .unwrap();
        let definition = ComponentDefinition::new(
            metadata("components/popover", "Popover"),
            ComponentSchema {
                state,
                ..ComponentSchema::default()
            },
        )
        .unwrap();
        assert!(matches!(
            definition.invoke(None, Map::new()),
            Err(ComponentError::MissingKey { .. })
        ));
        definition
            .invoke(Some("settings".to_owned()), Map::new())
            .unwrap();
    }

    #[test]
    fn registry_resolves_dependencies_first() {
        let mut registry = ComponentRegistry::new();
        registry.register(button(), 1).unwrap();
        let mut popover_metadata = metadata("components/popover", "Popover");
        popover_metadata
            .dependencies
            .insert(ModuleId::parse("components/button").unwrap());
        registry
            .register(
                ComponentDefinition::new(popover_metadata, ComponentSchema::default()).unwrap(),
                1,
            )
            .unwrap();

        assert_eq!(
            registry
                .resolve([ModuleId::parse("components/popover").unwrap()])
                .unwrap(),
            vec![
                ModuleId::parse("components/button").unwrap(),
                ModuleId::parse("components/popover").unwrap()
            ]
        );
    }

    #[test]
    fn component_header_is_machine_readable() {
        let header = r#"/* gpui-rhai
{
  "id": "components/button",
  "export": "Button",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Human-facing Button documentation follows.
fn render_button(props) { text(props.text) }
"#;
        assert_eq!(
            parse_component_header(header).unwrap(),
            metadata("components/button", "Button")
        );
    }

    #[test]
    fn export_component_registers_validated_schema() {
        let collector = ComponentExportCollector::new();
        let mut engine = Engine::new();
        collector.register_into(&mut engine);
        let definition = button();
        let mut scope = rhai::Scope::new();
        scope.push_dynamic(
            "definition",
            rhai::serde::to_dynamic(definition.clone()).unwrap(),
        );

        engine
            .eval_with_scope::<()>(&mut scope, "export_component(definition)")
            .unwrap();
        assert_eq!(collector.snapshot().unwrap().len(), 1);

        let error = engine
            .eval_with_scope::<()>(&mut scope, "export_component(definition)")
            .unwrap_err();
        assert!(error.to_string().contains("already registered"));
    }

    #[test]
    fn header_and_exported_metadata_must_match() {
        let definition = button();
        let mut header = definition.metadata.clone();
        header.version = Version::new(0, 2, 0);
        assert!(matches!(
            definition.validate_header(&header),
            Err(ComponentError::HeaderMismatch { .. })
        ));
    }

    #[test]
    fn component_schema_round_trips_for_tooling() {
        let definition = button();
        let json = serde_json::to_string_pretty(&definition).unwrap();
        let decoded: ComponentDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, definition);
    }
}
