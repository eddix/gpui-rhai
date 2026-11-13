//! Data-only Host capability contracts. A contract describes calls, never grants them.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

use semver::Version;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    CapabilityDescriptor, CapabilityId, CapabilityMethod, RUNTIME_API_VERSION, ValueSchema,
};

pub const CAPABILITY_CONTRACT_FORMAT: u32 = 1;
const MAX_CONTRACT_BYTES: usize = 4 * 1024 * 1024;

/// The registered entry point, independent of lifecycle permission to call it.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityCallKind {
    Sync,
    Task,
    Subscription,
}

impl CapabilityCallKind {
    pub(crate) fn from_function(name: &str) -> Option<Self> {
        match name {
            "call_capability" => Some(Self::Sync),
            "start_task" => Some(Self::Task),
            "start_subscription" => Some(Self::Subscription),
            _ => None,
        }
    }
}

/// Public schema and the modes actually registered for one capability.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityContractEntry {
    pub version: Version,
    pub methods: BTreeMap<String, CapabilityMethod>,
    pub modes: BTreeSet<CapabilityCallKind>,
}

/// A portable description of a Host's complete capability registry.
///
/// Contains declarations only: does not inspect handlers, service values,
/// file locations or activation state. Field defaults are omitted: capability
/// calls do not apply them, and they may contain business data or handles.
/// `sdk_version` is informational, not authorization
/// or the contract's semantic identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityContract {
    pub format_version: u32,
    pub runtime_api: u32,
    pub sdk_version: String,
    pub capabilities: BTreeMap<CapabilityId, CapabilityContractEntry>,
}

impl CapabilityContract {
    /// Build without constructing handlers or initializing any service.
    ///
    /// # Errors
    /// Rejects duplicate IDs, empty modes and invalid descriptors or schemas.
    pub fn from_descriptors(
        descriptors: impl IntoIterator<Item = (CapabilityDescriptor, BTreeSet<CapabilityCallKind>)>,
    ) -> Result<Self, CapabilityContractError> {
        let mut contract = Self {
            format_version: CAPABILITY_CONTRACT_FORMAT,
            runtime_api: RUNTIME_API_VERSION,
            sdk_version: env!("CARGO_PKG_VERSION").to_owned(),
            capabilities: BTreeMap::new(),
        };
        for (descriptor, modes) in descriptors {
            let id = descriptor.id;
            if contract.capabilities.contains_key(&id) {
                return Err(CapabilityContractError::Invalid(format!(
                    "duplicate ID `{id}`"
                )));
            }
            contract.capabilities.insert(
                id,
                CapabilityContractEntry {
                    version: descriptor.version,
                    methods: descriptor.methods,
                    modes,
                },
            );
        }
        contract.strip_metadata(false);
        contract.validate()?;
        Ok(contract)
    }

    /// Read tooling input without executing script or contacting services.
    ///
    /// # Errors
    /// Rejects oversized, malformed, unsupported or internally invalid contracts.
    pub fn from_json(source: &str) -> Result<Self, CapabilityContractError> {
        if source.len() > MAX_CONTRACT_BYTES {
            return Err(CapabilityContractError::Invalid(
                "contract exceeds 4 MiB".into(),
            ));
        }
        let mut contract: Self = serde_json::from_str(source)?;
        contract.strip_metadata(false);
        contract.validate()?;
        Ok(contract)
    }

    /// Read at most 4 MiB plus one byte before parsing untrusted tooling input.
    ///
    /// # Errors
    /// Reports read/UTF-8 errors or the same contract errors as `from_json`.
    pub fn from_reader(reader: impl Read) -> Result<Self, CapabilityContractError> {
        let mut source = String::new();
        reader
            .take(MAX_CONTRACT_BYTES as u64 + 1)
            .read_to_string(&mut source)?;
        Self::from_json(&source)
    }

    /// Validate an in-memory or deserialized contract before using it.
    ///
    /// # Errors
    /// Rejects unsupported versions, missing modes and invalid schemas.
    pub fn validate(&self) -> Result<(), CapabilityContractError> {
        if self.format_version != CAPABILITY_CONTRACT_FORMAT {
            return Err(CapabilityContractError::Format(self.format_version));
        }
        if self.runtime_api != RUNTIME_API_VERSION {
            return Err(CapabilityContractError::Runtime(self.runtime_api));
        }
        for (id, entry) in &self.capabilities {
            if entry.modes.is_empty() {
                return Err(CapabilityContractError::Invalid(format!(
                    "`{id}` has no entry points"
                )));
            }
            crate::capability::validate_descriptor(&CapabilityDescriptor {
                id: id.clone(),
                version: entry.version.clone(),
                methods: entry.methods.clone(),
            })
            .map_err(|error| CapabilityContractError::Invalid(error.to_string()))?;
            for method in entry.methods.values() {
                for schema in [&method.input, &method.output] {
                    schema
                        .validate_definition()
                        .map_err(|error| CapabilityContractError::Invalid(error.to_string()))?;
                }
            }
        }
        Ok(())
    }

    /// Check app requirements against declarations, without activating anything.
    ///
    /// # Errors
    /// Rejects unsupported runtime versions, missing IDs or incompatible versions.
    pub fn validate_manifest(
        &self,
        manifest: &crate::AppManifest,
    ) -> Result<(), CapabilityContractError> {
        self.validate()?;
        if manifest.runtime_api != self.runtime_api {
            return Err(CapabilityContractError::Runtime(manifest.runtime_api));
        }
        for (id, required) in &manifest.capabilities {
            let entry = self.capabilities.get(id).ok_or_else(|| {
                CapabilityContractError::Invalid(format!(
                    "manifest capability `{id}` is missing from Host contract"
                ))
            })?;
            if !required.matches(&entry.version) {
                return Err(CapabilityContractError::Invalid(format!(
                    "manifest capability `{id}` requires {required}, Host contract provides {}",
                    entry.version
                )));
            }
        }
        Ok(())
    }

    /// Deterministic diagnostic identity, not a signature or an authorization token.
    /// Documentation and SDK patch versions do not change semantic identity.
    ///
    /// # Errors
    /// Rejects invalid contracts or schemas that cannot be serialized.
    pub fn fingerprint(&self) -> Result<String, CapabilityContractError> {
        let bytes = self.semantic_bytes()?;
        let hash = bytes
            .into_iter()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
            });
        Ok(format!("cap-v{CAPABILITY_CONTRACT_FORMAT}-{hash:016x}"))
    }

    pub(crate) fn semantic_bytes(&self) -> Result<Vec<u8>, CapabilityContractError> {
        self.validate()?;
        let mut canonical = self.clone();
        canonical.sdk_version.clear();
        canonical.strip_metadata(true);
        Ok(serde_json::to_vec(&canonical)?)
    }

    fn strip_metadata(&mut self, strip_docs: bool) {
        for entry in self.capabilities.values_mut() {
            for method in entry.methods.values_mut() {
                strip_schema_metadata(&mut method.input, strip_docs);
                strip_schema_metadata(&mut method.output, strip_docs);
            }
        }
    }
}

fn strip_schema_metadata(schema: &mut ValueSchema, strip_docs: bool) {
    match schema {
        ValueSchema::Object { fields, .. } => {
            for field in fields.values_mut() {
                field.default = None;
                if strip_docs {
                    field.doc = None;
                }
                strip_schema_metadata(&mut field.schema, strip_docs);
            }
        }
        ValueSchema::Array { items, .. } => strip_schema_metadata(items, strip_docs),
        ValueSchema::Map { values } => strip_schema_metadata(values, strip_docs),
        ValueSchema::Optional { value } => strip_schema_metadata(value, strip_docs),
        ValueSchema::OneOf { variants } => variants
            .iter_mut()
            .for_each(|schema| strip_schema_metadata(schema, strip_docs)),
        _ => {}
    }
}

#[derive(Debug, Error)]
pub enum CapabilityContractError {
    #[error("could not read capability contract: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported capability contract format {0}; expected {CAPABILITY_CONTRACT_FORMAT}")]
    Format(u32),
    #[error("capability contract requires runtime API {0}; current API is {RUNTIME_API_VERSION}")]
    Runtime(u32),
    #[error("invalid capability contract: {0}")]
    Invalid(String),
    #[error(
        "stale Host capability contract: expected {expected}, registered {actual}; export it again from this Host"
    )]
    Stale { expected: String, actual: String },
    #[error("capability contract JSON: {0}")]
    Json(#[from] serde_json::Error),
}
