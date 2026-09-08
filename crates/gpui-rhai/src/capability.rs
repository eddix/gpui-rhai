use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::mpsc::Receiver;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ModuleId, RUNTIME_API_VERSION, SchemaValidationError, UiValue, ValueSchema};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CapabilityId(String);

impl CapabilityId {
    /// Parse a namespaced capability identifier such as `app.settings`.
    ///
    /// # Errors
    ///
    /// Returns [`CapabilityError::InvalidId`] for invalid identifiers.
    pub fn parse(value: impl Into<String>) -> Result<Self, CapabilityError> {
        let value = value.into();
        if value
            .split_once('.')
            .is_some_and(|(namespace, name)| is_identifier(namespace) && is_identifier(name))
        {
            Ok(Self(value))
        } else {
            Err(CapabilityError::InvalidId(value))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CapabilityId {
    type Error = CapabilityError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<CapabilityId> for String {
    fn from(value: CapabilityId) -> Self {
        value.0
    }
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('_')
        && !value.ends_with('_')
        && !value.contains("__")
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CapabilityMethod {
    pub input: ValueSchema,
    pub output: ValueSchema,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub id: CapabilityId,
    pub version: Version,
    pub methods: BTreeMap<String, CapabilityMethod>,
}

pub trait CapabilityHandler {
    /// Execute a schema-validated capability method.
    ///
    /// # Errors
    ///
    /// Returns a human-readable handler error. The registry validates the
    /// returned value against the declared output schema.
    fn call(&mut self, method: &str, input: UiValue) -> Result<UiValue, String>;
}

pub struct TaskWork {
    work: Box<dyn FnOnce(crate::TaskCancellation) -> Result<UiValue, String> + Send + 'static>,
}

impl TaskWork {
    #[must_use]
    pub fn new(work: impl FnOnce() -> Result<UiValue, String> + Send + 'static) -> Self {
        Self {
            work: Box::new(move |_| work()),
        }
    }

    #[must_use]
    pub fn cancellable(
        work: impl FnOnce(crate::TaskCancellation) -> Result<UiValue, String> + Send + 'static,
    ) -> Self {
        Self {
            work: Box::new(work),
        }
    }

    pub(crate) fn run(self, cancellation: crate::TaskCancellation) -> Result<UiValue, String> {
        (self.work)(cancellation)
    }
}

pub trait AsyncCapabilityHandler {
    /// Build one-shot background work after input validation.
    ///
    /// # Errors
    ///
    /// Returns a human-readable setup error before a task is spawned.
    fn start(&mut self, method: &str, input: UiValue) -> Result<TaskWork, String>;
}

/// One continuous subscription producer.
///
/// The work owns the producer loop and should not return until the stream is
/// finished. Returning closes the subscription and invalidates every cloned
/// [`crate::SubscriptionEmitter`] with
/// [`crate::SubscriptionCloseReason::WorkReturned`].
pub struct SubscriptionWork {
    work: Box<dyn FnOnce(crate::SubscriptionEmitter) + Send + 'static>,
}

impl SubscriptionWork {
    /// Build a subscription from its complete blocking producer loop.
    #[must_use]
    pub fn new(work: impl FnOnce(crate::SubscriptionEmitter) + Send + 'static) -> Self {
        Self {
            work: Box::new(work),
        }
    }

    /// Forward values from a channel until its senders are dropped or the
    /// subscription is closed.
    #[must_use]
    pub fn from_receiver(receiver: Receiver<UiValue>) -> Self {
        Self::new(move |emitter| {
            loop {
                if emitter.close_reason().is_some() {
                    break;
                }
                match receiver.recv_timeout(std::time::Duration::from_millis(50)) {
                    Ok(value) => {
                        if emitter.emit_blocking(value).is_err() {
                            break;
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                }
            }
        })
    }

    pub(crate) fn run(self, emitter: crate::SubscriptionEmitter) {
        (self.work)(emitter);
    }
}

pub trait SubscriptionCapabilityHandler {
    /// Build continuous producer work after input validation.
    ///
    /// The returned [`SubscriptionWork`] is the producer loop. It must remain
    /// running for as long as external code may use the emitter; returning from
    /// it closes the subscription immediately.
    ///
    /// # Errors
    ///
    /// Returns a human-readable setup error before subscription startup.
    fn subscribe(&mut self, method: &str, input: UiValue) -> Result<SubscriptionWork, String>;
}

struct CapabilityEntry {
    descriptor: CapabilityDescriptor,
    sync: Option<Box<dyn CapabilityHandler>>,
    task: Option<Box<dyn AsyncCapabilityHandler>>,
    subscription: Option<Box<dyn SubscriptionCapabilityHandler>>,
}

#[derive(Default)]
pub struct CapabilityRegistry {
    entries: BTreeMap<CapabilityId, CapabilityEntry>,
    active: BTreeSet<CapabilityId>,
}

impl fmt::Debug for CapabilityRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CapabilityRegistry")
            .field("registered", &self.entries.keys().collect::<Vec<_>>())
            .field("active", &self.active)
            .finish()
    }
}

impl CapabilityRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a host implementation and its public schema.
    ///
    /// # Errors
    ///
    /// Returns [`CapabilityError`] for duplicate IDs, empty method sets, or
    /// invalid method names.
    pub fn register(
        &mut self,
        descriptor: CapabilityDescriptor,
        handler: impl CapabilityHandler + 'static,
    ) -> Result<(), CapabilityError> {
        validate_descriptor(&descriptor)?;
        let entry = self.entry_for_descriptor(descriptor)?;
        if entry.sync.is_some() {
            return Err(CapabilityError::DuplicateMode("sync"));
        }
        entry.sync = Some(Box::new(handler));
        Ok(())
    }

    /// Register one-shot task methods for a capability descriptor.
    ///
    /// # Errors
    ///
    /// Returns descriptor or duplicate-mode errors.
    pub fn register_async(
        &mut self,
        descriptor: CapabilityDescriptor,
        handler: impl AsyncCapabilityHandler + 'static,
    ) -> Result<(), CapabilityError> {
        validate_descriptor(&descriptor)?;
        let entry = self.entry_for_descriptor(descriptor)?;
        if entry.task.is_some() {
            return Err(CapabilityError::DuplicateMode("task"));
        }
        entry.task = Some(Box::new(handler));
        Ok(())
    }

    /// Register continuous subscription methods for a capability descriptor.
    ///
    /// # Errors
    ///
    /// Returns descriptor or duplicate-mode errors.
    pub fn register_subscription(
        &mut self,
        descriptor: CapabilityDescriptor,
        handler: impl SubscriptionCapabilityHandler + 'static,
    ) -> Result<(), CapabilityError> {
        validate_descriptor(&descriptor)?;
        let entry = self.entry_for_descriptor(descriptor)?;
        if entry.subscription.is_some() {
            return Err(CapabilityError::DuplicateMode("subscription"));
        }
        entry.subscription = Some(Box::new(handler));
        Ok(())
    }

    fn entry_for_descriptor(
        &mut self,
        descriptor: CapabilityDescriptor,
    ) -> Result<&mut CapabilityEntry, CapabilityError> {
        let id = descriptor.id.clone();
        match self.entries.entry(id.clone()) {
            Entry::Vacant(entry) => Ok(entry.insert(CapabilityEntry {
                descriptor,
                sync: None,
                task: None,
                subscription: None,
            })),
            Entry::Occupied(entry) => {
                if entry.get().descriptor == descriptor {
                    Ok(entry.into_mut())
                } else {
                    Err(CapabilityError::DescriptorMismatch(id))
                }
            }
        }
    }

    /// Validate and activate exactly the capabilities declared by an app.
    ///
    /// # Errors
    ///
    /// Returns [`CapabilityError::Missing`] or
    /// [`CapabilityError::VersionMismatch`].
    pub fn activate(
        &mut self,
        requirements: &BTreeMap<CapabilityId, VersionReq>,
    ) -> Result<(), CapabilityError> {
        let mut active = BTreeSet::new();
        for (id, requirement) in requirements {
            let entry = self
                .entries
                .get(id)
                .ok_or_else(|| CapabilityError::Missing(id.clone()))?;
            if !requirement.matches(&entry.descriptor.version) {
                return Err(CapabilityError::VersionMismatch {
                    id: id.clone(),
                    required: requirement.clone(),
                    actual: entry.descriptor.version.clone(),
                });
            }
            active.insert(id.clone());
        }
        self.active = active;
        Ok(())
    }

    /// Invoke an activated method through schema validation on both sides.
    ///
    /// # Errors
    ///
    /// Returns [`CapabilityError`] for undeclared access, unknown methods,
    /// invalid values, or a handler failure.
    pub fn call(
        &mut self,
        id: &CapabilityId,
        method: &str,
        input: UiValue,
    ) -> Result<UiValue, CapabilityError> {
        if !self.active.contains(id) {
            return Err(CapabilityError::NotDeclared(id.clone()));
        }
        let entry = self
            .entries
            .get_mut(id)
            .ok_or_else(|| CapabilityError::Missing(id.clone()))?;
        let schema = entry
            .descriptor
            .methods
            .get(method)
            .cloned()
            .ok_or_else(|| CapabilityError::UnknownMethod {
                id: id.clone(),
                method: method.to_owned(),
            })?;
        schema
            .input
            .validate_ui_value(&input)
            .map_err(|source| CapabilityError::InvalidInput {
                id: id.clone(),
                method: method.to_owned(),
                source,
            })?;
        let handler = entry
            .sync
            .as_mut()
            .ok_or(CapabilityError::UnsupportedMode("sync"))?;
        let output = handler
            .call(method, input)
            .map_err(|message| CapabilityError::Handler {
                id: id.clone(),
                method: method.to_owned(),
                message,
            })?;
        schema.output.validate_ui_value(&output).map_err(|source| {
            CapabilityError::InvalidOutput {
                id: id.clone(),
                method: method.to_owned(),
                source,
            }
        })?;
        Ok(output)
    }

    /// Build schema-validated one-shot work for the task registry.
    ///
    /// # Errors
    ///
    /// Returns capability access, schema, mode, or handler setup errors.
    pub fn start_task(
        &mut self,
        id: &CapabilityId,
        method: &str,
        input: UiValue,
    ) -> Result<(TaskWork, ValueSchema), CapabilityError> {
        let (entry, schema) = self.entry_and_method(id, method, &input)?;
        let handler = entry
            .task
            .as_mut()
            .ok_or(CapabilityError::UnsupportedMode("task"))?;
        let work = handler
            .start(method, input)
            .map_err(|message| CapabilityError::Handler {
                id: id.clone(),
                method: method.to_owned(),
                message,
            })?;
        Ok((work, schema.output))
    }

    /// Build schema-validated continuous work for the subscription registry.
    ///
    /// # Errors
    ///
    /// Returns capability access, schema, mode, or handler setup errors.
    pub fn start_subscription(
        &mut self,
        id: &CapabilityId,
        method: &str,
        input: UiValue,
    ) -> Result<(SubscriptionWork, ValueSchema), CapabilityError> {
        let (entry, schema) = self.entry_and_method(id, method, &input)?;
        let handler = entry
            .subscription
            .as_mut()
            .ok_or(CapabilityError::UnsupportedMode("subscription"))?;
        let work =
            handler
                .subscribe(method, input)
                .map_err(|message| CapabilityError::Handler {
                    id: id.clone(),
                    method: method.to_owned(),
                    message,
                })?;
        Ok((work, schema.output))
    }

    fn entry_and_method(
        &mut self,
        id: &CapabilityId,
        method: &str,
        input: &UiValue,
    ) -> Result<(&mut CapabilityEntry, CapabilityMethod), CapabilityError> {
        if !self.active.contains(id) {
            return Err(CapabilityError::NotDeclared(id.clone()));
        }
        let entry = self
            .entries
            .get_mut(id)
            .ok_or_else(|| CapabilityError::Missing(id.clone()))?;
        let schema = entry
            .descriptor
            .methods
            .get(method)
            .cloned()
            .ok_or_else(|| CapabilityError::UnknownMethod {
                id: id.clone(),
                method: method.to_owned(),
            })?;
        schema
            .input
            .validate_ui_value(input)
            .map_err(|source| CapabilityError::InvalidInput {
                id: id.clone(),
                method: method.to_owned(),
                source,
            })?;
        Ok((entry, schema))
    }
}

fn validate_descriptor(descriptor: &CapabilityDescriptor) -> Result<(), CapabilityError> {
    if descriptor.methods.is_empty() {
        return Err(CapabilityError::NoMethods(descriptor.id.clone()));
    }
    if let Some(method) = descriptor
        .methods
        .keys()
        .find(|method| !is_identifier(method))
    {
        return Err(CapabilityError::InvalidMethod(method.clone()));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppManifest {
    pub entry: ModuleId,
    pub runtime_api: u32,
    #[serde(default)]
    pub capabilities: BTreeMap<CapabilityId, VersionReq>,
}

impl AppManifest {
    #[must_use]
    pub fn new(entry: ModuleId) -> Self {
        Self {
            entry,
            runtime_api: RUNTIME_API_VERSION,
            capabilities: BTreeMap::new(),
        }
    }

    /// Add one namespaced capability requirement from generated or host code.
    ///
    /// # Errors
    ///
    /// Returns identifier or semantic-version requirement parse errors.
    pub fn with_capability(mut self, id: &str, requirement: &str) -> Result<Self, CapabilityError> {
        let id = CapabilityId::parse(id)?;
        let requirement = VersionReq::parse(requirement).map_err(|source| {
            CapabilityError::InvalidVersionRequirement {
                requirement: requirement.to_owned(),
                source,
            }
        })?;
        self.capabilities.insert(id, requirement);
        Ok(self)
    }

    /// Validate runtime compatibility and activate declared capabilities.
    ///
    /// # Errors
    ///
    /// Returns [`CapabilityError::RuntimeApiMismatch`] or an activation error.
    pub fn activate(&self, registry: &mut CapabilityRegistry) -> Result<(), CapabilityError> {
        if self.runtime_api != RUNTIME_API_VERSION {
            return Err(CapabilityError::RuntimeApiMismatch {
                required: self.runtime_api,
                actual: RUNTIME_API_VERSION,
            });
        }
        registry.activate(&self.capabilities)
    }

    /// Verify that every installed component capability is explicitly declared
    /// by the application with the same compatibility requirement.
    ///
    /// # Errors
    ///
    /// Returns missing or mismatched component capability declarations.
    pub fn validate_components(
        &self,
        components: &crate::ComponentRegistry,
    ) -> Result<(), CapabilityError> {
        for (component_id, component) in components.iter() {
            for (raw_id, required) in &component.metadata.capabilities {
                let id = CapabilityId::parse(raw_id)?;
                let Some(declared) = self.capabilities.get(&id) else {
                    return Err(CapabilityError::ComponentCapabilityMissing {
                        component: component_id.clone(),
                        capability: id,
                    });
                };
                if declared != required {
                    return Err(CapabilityError::ComponentCapabilityMismatch {
                        component: component_id.clone(),
                        capability: id,
                        component_required: required.clone(),
                        app_declared: declared.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("capability ID `{0}` must be a namespaced `snake_case` identifier")]
    InvalidId(String),
    #[error("invalid capability version requirement `{requirement}`: {source}")]
    InvalidVersionRequirement {
        requirement: String,
        #[source]
        source: semver::Error,
    },
    #[error("capability `{0:?}` was registered with conflicting descriptors")]
    DescriptorMismatch(CapabilityId),
    #[error("capability handler mode `{0}` is already registered")]
    DuplicateMode(&'static str),
    #[error("capability does not implement `{0}` mode")]
    UnsupportedMode(&'static str),
    #[error("capability `{0:?}` must declare at least one method")]
    NoMethods(CapabilityId),
    #[error("capability method `{0}` must be a `snake_case` identifier")]
    InvalidMethod(String),
    #[error("required capability `{0:?}` is not registered")]
    Missing(CapabilityId),
    #[error("capability `{id:?}` requires {required}, host provides {actual}")]
    VersionMismatch {
        id: CapabilityId,
        required: VersionReq,
        actual: Version,
    },
    #[error("capability `{0:?}` was not declared by the application manifest")]
    NotDeclared(CapabilityId),
    #[error("capability `{id:?}` has no method `{method}`")]
    UnknownMethod { id: CapabilityId, method: String },
    #[error("input for `{id:?}.{method}` is invalid: {source}")]
    InvalidInput {
        id: CapabilityId,
        method: String,
        source: SchemaValidationError,
    },
    #[error("output from `{id:?}.{method}` is invalid: {source}")]
    InvalidOutput {
        id: CapabilityId,
        method: String,
        source: SchemaValidationError,
    },
    #[error("capability `{id:?}.{method}` failed: {message}")]
    Handler {
        id: CapabilityId,
        method: String,
        message: String,
    },
    #[error("application requires runtime API {required}, current API is {actual}")]
    RuntimeApiMismatch { required: u32, actual: u32 },
    #[error(
        "component `{component}` requires capability `{capability:?}` missing from the app manifest"
    )]
    ComponentCapabilityMissing {
        component: ModuleId,
        capability: CapabilityId,
    },
    #[error(
        "component `{component}` requires capability `{capability:?}` at {component_required}, but the app declares {app_declared}"
    )]
    ComponentCapabilityMismatch {
        component: ModuleId,
        capability: CapabilityId,
        component_required: VersionReq,
        app_declared: VersionReq,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ComponentDefinition, ComponentMetadata, ComponentRegistry, ComponentSchema, RuntimeApiRange,
    };

    struct Echo;

    impl CapabilityHandler for Echo {
        fn call(&mut self, method: &str, input: UiValue) -> Result<UiValue, String> {
            if method == "echo" {
                Ok(input)
            } else {
                Err("unexpected method".to_owned())
            }
        }
    }

    struct AsyncEcho;

    impl AsyncCapabilityHandler for AsyncEcho {
        fn start(&mut self, method: &str, input: UiValue) -> Result<TaskWork, String> {
            if method == "echo" {
                Ok(TaskWork::new(move || Ok(input)))
            } else {
                Err("unexpected method".to_owned())
            }
        }
    }

    fn descriptor() -> CapabilityDescriptor {
        CapabilityDescriptor {
            id: CapabilityId::parse("app.echo").unwrap(),
            version: Version::new(1, 2, 0),
            methods: BTreeMap::from([(
                "echo".to_owned(),
                CapabilityMethod {
                    input: ValueSchema::string(),
                    output: ValueSchema::string(),
                },
            )]),
        }
    }

    #[test]
    fn manifest_controls_capability_access_and_versions() {
        let mut registry = CapabilityRegistry::new();
        registry.register(descriptor(), Echo).unwrap();
        assert!(matches!(
            registry.call(
                &CapabilityId::parse("app.echo").unwrap(),
                "echo",
                UiValue::String("no".to_owned()),
            ),
            Err(CapabilityError::NotDeclared(_))
        ));

        let manifest = AppManifest {
            entry: ModuleId::parse("main").unwrap(),
            runtime_api: RUNTIME_API_VERSION,
            capabilities: BTreeMap::from([(
                CapabilityId::parse("app.echo").unwrap(),
                VersionReq::parse("^1.0").unwrap(),
            )]),
        };
        manifest.activate(&mut registry).unwrap();
        assert_eq!(
            registry
                .call(
                    &CapabilityId::parse("app.echo").unwrap(),
                    "echo",
                    UiValue::String("yes".to_owned()),
                )
                .unwrap(),
            UiValue::String("yes".to_owned())
        );
    }

    #[test]
    fn manifest_must_cover_component_capability_requirements() {
        let component = ComponentDefinition::new(
            ComponentMetadata {
                id: ModuleId::parse("components/remote_status").unwrap(),
                export: "RemoteStatus".to_owned(),
                version: Version::new(0, 1, 0),
                runtime_api: RuntimeApiRange::new(1, 2),
                dependencies: BTreeSet::new(),
                capabilities: BTreeMap::from([(
                    "app.echo".to_owned(),
                    VersionReq::parse("^1.0").unwrap(),
                )]),
                assets: BTreeSet::new(),
            },
            ComponentSchema::default(),
        )
        .unwrap();
        let mut components = ComponentRegistry::new();
        components.register(component, RUNTIME_API_VERSION).unwrap();
        let missing = AppManifest::new(ModuleId::parse("main").unwrap());
        assert!(matches!(
            missing.validate_components(&components),
            Err(CapabilityError::ComponentCapabilityMissing { .. })
        ));
        let declared = missing.with_capability("app.echo", "^1.0").unwrap();
        declared.validate_components(&components).unwrap();
    }

    #[test]
    fn output_is_validated_even_for_rust_handlers() {
        struct BadOutput;
        impl CapabilityHandler for BadOutput {
            fn call(&mut self, _: &str, _: UiValue) -> Result<UiValue, String> {
                Ok(UiValue::Bool(true))
            }
        }
        let mut registry = CapabilityRegistry::new();
        registry.register(descriptor(), BadOutput).unwrap();
        registry
            .activate(&BTreeMap::from([(
                CapabilityId::parse("app.echo").unwrap(),
                VersionReq::STAR,
            )]))
            .unwrap();
        assert!(matches!(
            registry.call(
                &CapabilityId::parse("app.echo").unwrap(),
                "echo",
                UiValue::String("input".to_owned()),
            ),
            Err(CapabilityError::InvalidOutput { .. })
        ));
    }

    #[test]
    fn async_handler_builds_validated_worker_without_moving_rhai_callbacks() {
        let mut registry = CapabilityRegistry::new();
        registry.register_async(descriptor(), AsyncEcho).unwrap();
        registry
            .activate(&BTreeMap::from([(
                CapabilityId::parse("app.echo").unwrap(),
                VersionReq::STAR,
            )]))
            .unwrap();
        let (work, output) = registry
            .start_task(
                &CapabilityId::parse("app.echo").unwrap(),
                "echo",
                UiValue::String("async".to_owned()),
            )
            .unwrap();
        let value = work.run(crate::TaskCancellation::default()).unwrap();
        output.validate_ui_value(&value).unwrap();
        assert_eq!(value, UiValue::String("async".to_owned()));
    }
}
