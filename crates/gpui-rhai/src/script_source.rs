use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::ModuleId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptAsset {
    pub id: ModuleId,
    pub source: String,
    pub content_hash: u64,
}

impl ScriptAsset {
    #[must_use]
    pub fn new(id: ModuleId, source: String) -> Self {
        let content_hash = fnv1a(source.as_bytes());
        Self {
            id,
            source,
            content_hash,
        }
    }
}

pub trait ScriptSource {
    fn module_ids(&self) -> Vec<ModuleId>;

    /// Load one declared module.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptSourceError`] when the module is absent, escapes a file
    /// root, or cannot be read.
    fn load(&self, id: &ModuleId) -> Result<ScriptAsset, ScriptSourceError>;
}

#[derive(Clone, Debug, Default)]
pub struct EmbeddedScriptSource {
    modules: BTreeMap<ModuleId, String>,
}

impl EmbeddedScriptSource {
    #[must_use]
    pub fn new(modules: BTreeMap<ModuleId, String>) -> Self {
        Self { modules }
    }
}

impl ScriptSource for EmbeddedScriptSource {
    fn module_ids(&self) -> Vec<ModuleId> {
        self.modules.keys().cloned().collect()
    }

    fn load(&self, id: &ModuleId) -> Result<ScriptAsset, ScriptSourceError> {
        let source = self
            .modules
            .get(id)
            .cloned()
            .ok_or_else(|| ScriptSourceError::Missing(id.clone()))?;
        Ok(ScriptAsset::new(id.clone(), source))
    }
}

#[derive(Clone, Debug)]
pub struct FileScriptSource {
    root: PathBuf,
    modules: Vec<ModuleId>,
}

impl FileScriptSource {
    /// Create a file source rooted at an existing canonical directory.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptSourceError`] if the root cannot be canonicalized.
    pub fn new(
        root: impl AsRef<Path>,
        modules: impl IntoIterator<Item = ModuleId>,
    ) -> Result<Self, ScriptSourceError> {
        let root = root
            .as_ref()
            .canonicalize()
            .map_err(|source| ScriptSourceError::Io {
                path: root.as_ref().to_path_buf(),
                source,
            })?;
        Ok(Self {
            root,
            modules: modules.into_iter().collect(),
        })
    }

    fn path_for(&self, id: &ModuleId) -> PathBuf {
        self.root.join(id.as_str()).with_extension("rhai")
    }
}

impl ScriptSource for FileScriptSource {
    fn module_ids(&self) -> Vec<ModuleId> {
        self.modules.clone()
    }

    fn load(&self, id: &ModuleId) -> Result<ScriptAsset, ScriptSourceError> {
        if !self.modules.contains(id) {
            return Err(ScriptSourceError::Missing(id.clone()));
        }
        let requested = self.path_for(id);
        let canonical = requested
            .canonicalize()
            .map_err(|source| ScriptSourceError::Io {
                path: requested.clone(),
                source,
            })?;
        if !canonical.starts_with(&self.root) {
            return Err(ScriptSourceError::EscapedRoot(canonical));
        }
        let source = fs::read_to_string(&canonical).map_err(|source| ScriptSourceError::Io {
            path: canonical,
            source,
        })?;
        Ok(ScriptAsset::new(id.clone(), source))
    }
}

#[derive(Debug, Error)]
pub enum ScriptSourceError {
    #[error("script module `{0}` is not present in this source")]
    Missing(ModuleId),
    #[error("script path `{0}` escaped its configured root")]
    EscapedRoot(PathBuf),
    #[error("script source I/O failed for `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

const fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        index += 1;
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RestrictedModuleResolver;
    use rhai::Engine;

    #[test]
    fn embedded_source_is_content_addressed() {
        let id = ModuleId::parse("components/button").unwrap();
        let source = EmbeddedScriptSource::new(BTreeMap::from([(
            id.clone(),
            "fn Button(props) { props }".to_owned(),
        )]));
        let first = source.load(&id).unwrap();
        let second = source.load(&id).unwrap();
        assert_eq!(first.content_hash, second.content_hash);
    }

    #[test]
    fn missing_embedded_module_is_explicit() {
        let source = EmbeddedScriptSource::default();
        assert!(matches!(
            source.load(&ModuleId::parse("components/missing").unwrap()),
            Err(ScriptSourceError::Missing(_))
        ));
    }

    #[test]
    fn file_and_embedded_sources_have_identical_resolution() {
        let directory = tempfile::tempdir().unwrap();
        let component_directory = directory.path().join("components");
        fs::create_dir_all(&component_directory).unwrap();
        let script = "fn greeting() { \"hello\" }";
        fs::write(component_directory.join("greeting.rhai"), script).unwrap();
        let id = ModuleId::parse("components/greeting").unwrap();
        let file = FileScriptSource::new(directory.path(), [id.clone()]).unwrap();
        let embedded = EmbeddedScriptSource::new(BTreeMap::from([(id.clone(), script.to_owned())]));

        assert_eq!(
            file.load(&id).unwrap().content_hash,
            embedded.load(&id).unwrap().content_hash
        );

        for resolver in [
            RestrictedModuleResolver::from_source(&file).unwrap(),
            RestrictedModuleResolver::from_source(&embedded).unwrap(),
        ] {
            let mut engine = Engine::new();
            engine.set_module_resolver(resolver);
            let value: String = engine
                .eval(
                    r#"
                        import "components/greeting" as greeting;
                        greeting::greeting()
                    "#,
                )
                .unwrap();
            assert_eq!(value, "hello");
        }
    }
}
