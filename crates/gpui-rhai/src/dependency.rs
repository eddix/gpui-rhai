use std::collections::{BTreeMap, BTreeSet};

use rhai::{AST, ASTNode, Engine, Expr, OptimizationLevel, Stmt};
use thiserror::Error;

use crate::{ModuleId, ScriptSource, ScriptSourceError};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModuleDependencyGraph {
    dependencies: BTreeMap<ModuleId, BTreeSet<ModuleId>>,
    dependents: BTreeMap<ModuleId, BTreeSet<ModuleId>>,
}

impl ModuleDependencyGraph {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_dependencies(&mut self, module: &ModuleId, dependencies: BTreeSet<ModuleId>) {
        if let Some(previous) = self
            .dependencies
            .insert(module.clone(), dependencies.clone())
        {
            for dependency in previous {
                if let Some(dependents) = self.dependents.get_mut(&dependency) {
                    dependents.remove(module);
                }
            }
        }
        for dependency in dependencies {
            self.dependents
                .entry(dependency)
                .or_default()
                .insert(module.clone());
        }
        self.dependents
            .retain(|_, dependents| !dependents.is_empty());
    }

    #[must_use]
    pub fn dependencies_of(&self, module: &ModuleId) -> BTreeSet<ModuleId> {
        self.dependencies.get(module).cloned().unwrap_or_default()
    }

    #[must_use]
    pub fn affected_by(&self, changed: impl IntoIterator<Item = ModuleId>) -> BTreeSet<ModuleId> {
        let mut affected = changed.into_iter().collect::<BTreeSet<_>>();
        let mut pending = affected.iter().cloned().collect::<Vec<_>>();
        while let Some(module) = pending.pop() {
            for dependent in self.dependents.get(&module).into_iter().flatten() {
                if affected.insert(dependent.clone()) {
                    pending.push(dependent.clone());
                }
            }
        }
        affected
    }

    /// Rebuild a graph from the current contents of a script source.
    ///
    /// # Errors
    ///
    /// Returns source or static-import parsing errors.
    pub fn from_source(source: &impl ScriptSource) -> Result<Self, DependencyError> {
        let mut graph = Self::new();
        for id in source.module_ids() {
            let asset = source.load(&id)?;
            graph.set_dependencies(&id, extract_imports(&asset.source)?);
        }
        Ok(graph)
    }
}

/// Extract literal Rhai imports from an unoptimized Rhai AST.
///
/// # Errors
///
/// Returns [`DependencyError`] for invalid Rhai syntax, a dynamic import, or an
/// invalid module ID.
pub fn extract_imports(source: &str) -> Result<BTreeSet<ModuleId>, DependencyError> {
    let mut parser = Engine::new_raw();
    parser.set_optimization_level(OptimizationLevel::None);
    parser.set_max_expr_depths(64, 32);
    let ast = parser
        .compile(source)
        .map_err(|error| DependencyError::Parse {
            position: error.position(),
            message: error.to_string(),
        })?;
    extract_imports_from_ast(&ast)
}

fn extract_imports_from_ast(ast: &AST) -> Result<BTreeSet<ModuleId>, DependencyError> {
    let mut imports = BTreeSet::new();
    let mut error = None;
    ast.walk(&mut |path| {
        let Some(ASTNode::Stmt(Stmt::Import(import, _))) = path.last() else {
            return true;
        };
        if let Expr::StringConstant(module, ..) = &import.0 {
            match ModuleId::parse(module.to_string()) {
                Ok(module) => {
                    imports.insert(module);
                    true
                }
                Err(source) => {
                    error = Some(DependencyError::ModuleId(source));
                    false
                }
            }
        } else {
            error = Some(DependencyError::DynamicImport);
            false
        }
    });
    error.map_or(Ok(imports), Err)
}

#[derive(Clone)]
struct CachedModule {
    content_hash: u64,
    ast: AST,
}

#[derive(Clone, Default)]
pub struct ModuleCompileCache {
    modules: BTreeMap<ModuleId, CachedModule>,
    graph: ModuleDependencyGraph,
}

impl ModuleCompileCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Recompile changed modules and their transitive dependants transactionally.
    ///
    /// # Errors
    ///
    /// Returns [`DependencyError`] without modifying the current cache when any
    /// affected source cannot load or compile.
    pub fn refresh(
        &mut self,
        engine: &Engine,
        source: &impl ScriptSource,
        changed: impl IntoIterator<Item = ModuleId>,
    ) -> Result<ModuleRefreshReport, DependencyError> {
        let graph = ModuleDependencyGraph::from_source(source)?;
        let changed = changed.into_iter().collect::<BTreeSet<_>>();
        let mut affected = self.graph.affected_by(changed.iter().cloned());
        affected.extend(graph.affected_by(changed));
        let mut staged = self.modules.clone();
        let existing = source.module_ids().into_iter().collect::<BTreeSet<_>>();
        staged.retain(|id, _| existing.contains(id));
        let mut compiled = Vec::new();
        for id in &affected {
            if !existing.contains(id) {
                continue;
            }
            let asset = source.load(id)?;
            let mut ast =
                engine
                    .compile(&asset.source)
                    .map_err(|source| DependencyError::Compile {
                        module: id.clone(),
                        source: source.into(),
                    })?;
            crate::engine::validate_assignment_targets(&ast).map_err(|error| {
                DependencyError::UnsafeAst {
                    module: id.clone(),
                    message: error.to_string(),
                }
            })?;
            ast.set_source(id.as_str());
            staged.insert(
                id.clone(),
                CachedModule {
                    content_hash: asset.content_hash,
                    ast,
                },
            );
            compiled.push(id.clone());
        }
        self.modules = staged;
        self.graph = graph;
        Ok(ModuleRefreshReport { affected, compiled })
    }

    #[must_use]
    pub fn contains(&self, module: &ModuleId) -> bool {
        self.modules.contains_key(module)
    }

    #[must_use]
    pub fn content_hash(&self, module: &ModuleId) -> Option<u64> {
        self.modules.get(module).map(|cached| cached.content_hash)
    }

    #[must_use]
    pub fn ast(&self, module: &ModuleId) -> Option<&AST> {
        self.modules.get(module).map(|cached| &cached.ast)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleRefreshReport {
    pub affected: BTreeSet<ModuleId>,
    pub compiled: Vec<ModuleId>,
}

#[derive(Debug, Error)]
pub enum DependencyError {
    #[error("Rhai imports must use a literal module string")]
    DynamicImport,
    #[error("Rhai source failed to parse while extracting imports at {position}: {message}")]
    Parse {
        position: rhai::Position,
        message: String,
    },
    #[error("module `{module}` failed to compile: {source}")]
    Compile {
        module: ModuleId,
        #[source]
        source: Box<rhai::EvalAltResult>,
    },
    #[error("module `{module}` contains an unsafe Rhai AST: {message}")]
    UnsafeAst { module: ModuleId, message: String },
    #[error(transparent)]
    ModuleId(#[from] crate::ModuleIdError),
    #[error(transparent)]
    Source(#[from] ScriptSourceError),
}

#[cfg(feature = "dev-reload")]
mod watcher {
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::{Receiver, TryRecvError, channel};
    use std::time::Duration;

    use notify::{Config, Event, PollWatcher, RecursiveMode, Watcher};
    use thiserror::Error;

    pub struct FileWatcher {
        root: PathBuf,
        receiver: Receiver<notify::Result<Event>>,
        _watcher: PollWatcher,
    }

    impl FileWatcher {
        /// Watch a UI source tree recursively.
        ///
        /// # Errors
        ///
        /// Returns [`WatcherError`] when the platform watcher cannot initialize.
        pub fn new(root: impl AsRef<Path>) -> Result<Self, WatcherError> {
            let root = root.as_ref().canonicalize().map_err(WatcherError::Io)?;
            let (sender, receiver) = channel();
            let mut watcher = PollWatcher::new(
                move |event| {
                    let _ = sender.send(event);
                },
                Config::default()
                    .with_poll_interval(Duration::from_millis(100))
                    .with_compare_contents(true),
            )?;
            watcher.watch(&root, RecursiveMode::Recursive)?;
            Ok(Self {
                root,
                receiver,
                _watcher: watcher,
            })
        }

        /// Drain currently available relevant file changes without blocking.
        ///
        /// # Errors
        ///
        /// Returns [`WatcherError`] for platform watcher failures or a closed
        /// event channel.
        pub fn poll(&self) -> Result<FileChangeBatch, WatcherError> {
            let mut paths = std::collections::BTreeSet::new();
            loop {
                match self.receiver.try_recv() {
                    Ok(Ok(event)) => {
                        paths.extend(event.paths.into_iter().filter(|path| relevant(path)));
                    }
                    Ok(Err(error)) => return Err(error.into()),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        return Err(WatcherError::Disconnected);
                    }
                }
            }
            Ok(FileChangeBatch {
                root: self.root.clone(),
                paths,
            })
        }
    }

    pub(super) fn relevant(path: &Path) -> bool {
        matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some(
                "rhai"
                    | "toml"
                    | "svg"
                    | "png"
                    | "jpg"
                    | "jpeg"
                    | "gif"
                    | "webp"
                    | "bmp"
                    | "tif"
                    | "tiff"
            )
        )
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct FileChangeBatch {
        pub root: PathBuf,
        pub paths: std::collections::BTreeSet<PathBuf>,
    }

    #[derive(Debug, Error)]
    pub enum WatcherError {
        #[error("file watcher I/O failed: {0}")]
        Io(std::io::Error),
        #[error("file watcher failed: {0}")]
        Notify(#[from] notify::Error),
        #[error("file watcher event channel disconnected")]
        Disconnected,
    }
}

#[cfg(feature = "dev-reload")]
pub use watcher::{FileChangeBatch, FileWatcher, WatcherError};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EmbeddedScriptSource;

    fn id(value: &str) -> ModuleId {
        ModuleId::parse(value).unwrap()
    }

    #[test]
    fn import_scanner_ignores_comments_and_string_contents() {
        let imports = extract_imports(
            r#"
                // import "ignored/line" as ignored;
                /* import "ignored/block" as ignored; */
                let message = "import \"ignored/string\"";
                import "components/button" as button;
            "#,
        )
        .unwrap();
        assert_eq!(imports, BTreeSet::from([id("components/button")]));
    }

    #[test]
    fn import_extraction_uses_rhai_parser_for_nested_comments_and_templates() {
        for source in [
            r#"/* outer /* inner */ import "../ignored"; */ fn view() { text("ok") }"#,
            r#"fn view() { text(`example: import "../ignored" as demo;`) }"#,
        ] {
            assert!(extract_imports(source).unwrap().is_empty());
        }
        assert_eq!(
            extract_imports(
                r#"fn view() { text(`before ${#{ value: 1 }.value} after`) }
                    import "components/real" as real;"#,
            )
            .unwrap(),
            BTreeSet::from([id("components/real")])
        );
    }

    #[test]
    fn import_extraction_uses_unoptimized_ast_for_nested_templates() {
        let cases = [
            (
                r#"let x = `a${if true { "" } else { `b${1}` }}`;"#,
                BTreeSet::new(),
            ),
            (
                r#"let x = `a${`b${1}`}`; import "components/real" as real;"#,
                BTreeSet::from([id("components/real")]),
            ),
            (r"let x = `a${`b${1}c${2}`}d${3}`;", BTreeSet::new()),
            (r"let x = `a${`b${`c${1}`}`}`;", BTreeSet::new()),
            (
                r#"let x = `a${{ import "components/real" as real; `b${1}` }}`;"#,
                BTreeSet::from([id("components/real")]),
            ),
            (r#"let x = `a${`import "../fake" ${1}`}`;"#, BTreeSet::new()),
            (
                r"let x = `a${#{ value: `b${#{ nested: 1 }.nested}` }.value}`;",
                BTreeSet::new(),
            ),
            (
                r#"/* outer /* inner */ import "../fake"; */ let x = `a${1}`;"#,
                BTreeSet::new(),
            ),
            (
                r#"if false { import "components/dead" as dead; }"#,
                BTreeSet::from([id("components/dead")]),
            ),
        ];
        for (source, expected) in cases {
            assert_eq!(extract_imports(source).unwrap(), expected, "{source}");
        }
    }

    #[test]
    fn import_extraction_rejects_dynamic_and_invalid_imports() {
        assert!(
            extract_imports(r#"let module = "components/real"; import module as real;"#).is_err()
        );
        assert!(extract_imports(r#"import "../outside" as outside;"#).is_err());
        assert!(matches!(
            extract_imports("let broken = `unterminated${1}"),
            Err(DependencyError::Parse { .. })
        ));
    }

    #[test]
    fn changes_invalidate_transitive_dependants() {
        let mut graph = ModuleDependencyGraph::new();
        graph.set_dependencies(&id("button"), BTreeSet::new());
        graph.set_dependencies(&id("toolbar"), BTreeSet::from([id("button")]));
        graph.set_dependencies(&id("main"), BTreeSet::from([id("toolbar")]));
        assert_eq!(
            graph.affected_by([id("button")]),
            BTreeSet::from([id("button"), id("toolbar"), id("main")])
        );
    }

    #[test]
    fn failed_refresh_does_not_commit_partial_cache() {
        let button = id("button");
        let toolbar = id("toolbar");
        let mut source = EmbeddedScriptSource::new(BTreeMap::from([
            (button.clone(), "fn Button() { 1 }".to_owned()),
            (
                toolbar.clone(),
                "import \"button\" as button; fn Toolbar() { 1 }".to_owned(),
            ),
        ]));
        let engine = Engine::new();
        let mut cache = ModuleCompileCache::new();
        cache
            .refresh(&engine, &source, [button.clone(), toolbar.clone()])
            .unwrap();
        let old_hash = cache.content_hash(&button).unwrap();

        source = EmbeddedScriptSource::new(BTreeMap::from([
            (button.clone(), "fn Button() { 2 }".to_owned()),
            (toolbar.clone(), "fn Toolbar( {".to_owned()),
        ]));
        assert!(cache.refresh(&engine, &source, [button.clone()]).is_err());
        assert_eq!(cache.content_hash(&button), Some(old_hash));
    }

    #[cfg(feature = "dev-reload")]
    #[test]
    fn file_watcher_reports_rhai_changes() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("main.rhai");
        std::fs::write(&file, "fn view(ctx) { text(\"before\") }").unwrap();
        let watcher = FileWatcher::new(directory.path()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        std::fs::write(&file, "fn view(ctx) { text(\"after\") }").unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let batch = watcher.poll().unwrap();
            if batch
                .paths
                .iter()
                .any(|changed| changed.ends_with("main.rhai"))
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "watcher event timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }

    #[cfg(feature = "dev-reload")]
    #[test]
    fn file_watcher_accepts_all_supported_image_formats() {
        for extension in [
            "svg", "png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff",
        ] {
            assert!(watcher::relevant(std::path::Path::new(&format!(
                "asset.{extension}"
            ))));
        }
        assert!(!watcher::relevant(std::path::Path::new("asset.txt")));
    }
}
