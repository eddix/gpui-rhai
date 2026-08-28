use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

#[cfg(feature = "dev-reload")]
use gpui::actions;
use gpui::{
    AnyWindowHandle, App, AppContext, Application, Bounds, Context, FocusHandle,
    InteractiveElement, IntoElement, KeyBinding, ParentElement, Render, SharedString, Styled, Task,
    Timer, TitlebarOptions, Window, WindowAppearance, WindowBounds, WindowOptions, div, px, rgba,
    size,
};
use thiserror::Error;

#[cfg(feature = "dev-reload")]
use crate::FileWatcher;
use crate::overlay_element::WindowOverlayCoordinator;
use crate::{
    ActionError, ActionId, AnimationRuntime, AppManifest, AssetData, CapabilityError, CompiledUi,
    ComponentExportError, ComponentInstancePath, ComponentRegistry, ComponentStateSchema,
    DependencyError, DirectoryAssetProvider, DispatchScriptAction, EmbeddedScriptSource,
    FileScriptSource, GpuiNodeRenderer, InMemoryAssetProvider, InteractionState, KeyBindingSpec,
    LocaleBundle, LocaleManager, ModuleCompileCache, ModuleId, MotionPreference,
    NodeEventDispatcher, PrimitiveRegistry, ResponsiveError, ResponsiveRuntime,
    RestrictedModuleResolver, RuntimeEngine, RuntimeError, ScriptCallback, ScriptLifecycle,
    ScriptSource, ScriptWindowSpec, SystemAppearance, TextDirection, ThemeManager, ThemeSelection,
    ThemeVariant, UiRuntimeState, UiValue, ViewportBreakpoints, WindowCommand, init_text_input,
    load_locale_source, load_theme_source,
};

#[cfg(feature = "dev-reload")]
actions!(gpui_rhai_devtools, [ToggleInspector]);

const HOST_KEY_CONTEXT: &str = "GPUIRhaiHost";

pub trait ScriptAppExtension {
    /// Register custom primitives or engine APIs before scripts compile.
    ///
    /// # Errors
    ///
    /// Returns an application-facing extension diagnostic.
    fn configure_engine(&self, _engine: &mut RuntimeEngine) -> Result<(), String> {
        Ok(())
    }

    /// Register and activate capabilities or stores before lifecycle init.
    ///
    /// # Errors
    ///
    /// Returns an application-facing extension diagnostic.
    fn configure_runtime(&self, _runtime: &mut UiRuntimeState) -> Result<(), String> {
        Ok(())
    }

    /// Declare resources owned by each script window, such as window stores.
    /// This runs once before that window's lifecycle `init`.
    ///
    /// # Errors
    ///
    /// Returns an application-facing extension diagnostic.
    fn configure_window(
        &self,
        _window_id: &str,
        _runtime: &mut UiRuntimeState,
    ) -> Result<(), String> {
        Ok(())
    }
}

fn motion_preference_from_env() -> MotionPreference {
    std::env::var("GPUI_RHAI_REDUCED_MOTION")
        .ok()
        .as_deref()
        .map_or(MotionPreference::Normal, parse_motion_preference)
}

fn parse_motion_preference(value: &str) -> MotionPreference {
    if matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "reduce" | "reduced"
    ) {
        MotionPreference::Reduced
    } else {
        MotionPreference::Normal
    }
}

pub struct ScriptApp {
    entry: PathBuf,
    window_size: (f32, f32),
    development: bool,
    motion_preference: MotionPreference,
    extensions: Vec<Box<dyn ScriptAppExtension>>,
    key_bindings: Vec<KeyBindingSpec>,
    viewport_breakpoints: ViewportBreakpoints,
}

impl ScriptApp {
    #[must_use]
    pub fn new(entry: impl Into<PathBuf>) -> Self {
        Self {
            entry: entry.into(),
            window_size: (720.0, 480.0),
            development: cfg!(feature = "dev-reload") && cfg!(debug_assertions),
            motion_preference: motion_preference_from_env(),
            extensions: Vec::new(),
            key_bindings: Vec::new(),
            viewport_breakpoints: ViewportBreakpoints::default(),
        }
    }

    #[must_use]
    pub fn window_size(mut self, width: f32, height: f32) -> Self {
        self.window_size = (width, height);
        self
    }

    #[must_use]
    pub fn development(mut self, enabled: bool) -> Self {
        self.development = enabled;
        self
    }

    #[must_use]
    pub const fn motion_preference(mut self, preference: MotionPreference) -> Self {
        self.motion_preference = preference;
        self
    }

    #[must_use]
    pub fn extension(mut self, extension: impl ScriptAppExtension + 'static) -> Self {
        self.extensions.push(Box::new(extension));
        self
    }

    #[must_use]
    pub fn key_binding(mut self, binding: KeyBindingSpec) -> Self {
        self.key_bindings.push(binding);
        self
    }

    #[must_use]
    pub const fn viewport_breakpoints(mut self, breakpoints: ViewportBreakpoints) -> Self {
        self.viewport_breakpoints = breakpoints;
        self
    }

    /// Read, compile, initialize, and render the app before opening GPUI.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptAppError`] for source I/O, compilation, lifecycle, or
    /// initial rendering failures.
    pub fn prepare(self) -> Result<PreparedScriptApp, ScriptAppError> {
        let source = fs::read_to_string(&self.entry).map_err(|source| ScriptAppError::Io {
            path: self.entry.clone(),
            source,
        })?;
        let extensions = Rc::new(self.extensions);
        let mut engine = RuntimeEngine::new();
        for extension in extensions.iter() {
            extension
                .configure_engine(&mut engine)
                .map_err(ScriptAppError::Extension)?;
        }
        let (ui_root, theme_path, theme) = load_primary_file_theme(&engine, &self.entry)?;
        let manifest = load_file_manifest(&ui_root, &self.entry)?;
        let module_cache = configure_file_modules(&mut engine, &ui_root, &self.entry, &theme_path)?;
        #[cfg(not(feature = "dev-reload"))]
        let _ = &module_cache;
        let compiled =
            engine.compile_self_contained_named(&self.entry.to_string_lossy(), &source)?;
        let component_exports = engine.component_exports()?;
        manifest.validate_components(&component_exports)?;
        let state_schema = engine.root_state_schema(&compiled)?;
        let mut runtime_state = UiRuntimeState::new();
        runtime_state.animations = AnimationRuntime::new(self.motion_preference);
        runtime_state.responsive = ResponsiveRuntime::new(self.viewport_breakpoints);
        runtime_state.locale = load_locale_directory(engine.engine(), &ui_root.join("locales"))?;
        runtime_state.theme = Some(load_theme_directory(
            engine.engine(),
            &ui_root.join("themes"),
            &theme,
        )?);
        register_file_assets(&runtime_state, &ui_root)?;
        runtime_state
            .windows
            .register_open("main")
            .map_err(|error| ScriptAppError::Extension(error.to_string()))?;
        for extension in extensions.iter() {
            extension
                .configure_runtime(&mut runtime_state)
                .map_err(ScriptAppError::Extension)?;
        }
        manifest.activate(&mut runtime_state.capabilities)?;
        for extension in extensions.iter() {
            extension
                .configure_window("main", &mut runtime_state)
                .map_err(ScriptAppError::Extension)?;
        }
        let runtime = Rc::new(RefCell::new(runtime_state));
        let program = WindowProgram {
            compiled: compiled.clone(),
            state_schema: state_schema.clone(),
            component_exports,
        };
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            ComponentInstancePath::root("App", "main"),
            Some("main".to_owned()),
            BTreeMap::default(),
            &state_schema,
        )?;
        lifecycle.start(&mut engine)?;
        let factory = Rc::new(ScriptWindowFactory {
            program: RefCell::new(program),
            runtime,
            extensions,
            theme: theme.clone(),
            #[cfg(feature = "dev-reload")]
            development: self.development,
        });
        Ok(PreparedScriptApp {
            engine,
            lifecycle,
            window_size: self.window_size,
            theme,
            entry: self.entry,
            ui_root,
            theme_path,
            development: self.development,
            factory,
            key_bindings: self.key_bindings,
            #[cfg(feature = "dev-reload")]
            module_cache,
        })
    }

    /// Prepare and run the app on GPUI's foreground event loop.
    ///
    /// # Errors
    ///
    /// Returns preparation or window-opening errors.
    pub fn run(self) -> Result<(), ScriptAppError> {
        self.prepare()?.run()
    }
}

pub struct EmbeddedScriptApp {
    entry: ModuleId,
    scripts: EmbeddedScriptSource,
    theme_source: String,
    window_size: (f32, f32),
    locales: Vec<(String, String)>,
    themes: Vec<(String, String)>,
    development: bool,
    motion_preference: MotionPreference,
    extensions: Vec<Box<dyn ScriptAppExtension>>,
    manifest: AppManifest,
    key_bindings: Vec<KeyBindingSpec>,
    assets: BTreeMap<String, AssetData>,
    viewport_breakpoints: ViewportBreakpoints,
}

impl EmbeddedScriptApp {
    #[must_use]
    pub fn new(
        entry: ModuleId,
        scripts: EmbeddedScriptSource,
        theme_source: impl Into<String>,
    ) -> Self {
        let manifest = AppManifest::new(entry.clone());
        Self {
            entry,
            scripts,
            theme_source: theme_source.into(),
            window_size: (720.0, 480.0),
            locales: Vec::new(),
            themes: Vec::new(),
            development: false,
            motion_preference: motion_preference_from_env(),
            extensions: Vec::new(),
            manifest,
            key_bindings: Vec::new(),
            assets: BTreeMap::new(),
            viewport_breakpoints: ViewportBreakpoints::default(),
        }
    }

    #[must_use]
    pub fn window_size(mut self, width: f32, height: f32) -> Self {
        self.window_size = (width, height);
        self
    }

    #[must_use]
    pub fn locale_sources(mut self, locales: impl IntoIterator<Item = (String, String)>) -> Self {
        self.locales = locales.into_iter().collect();
        self
    }

    #[must_use]
    pub fn theme_sources(mut self, themes: impl IntoIterator<Item = (String, String)>) -> Self {
        self.themes = themes.into_iter().collect();
        self
    }

    #[must_use]
    pub const fn development(mut self, enabled: bool) -> Self {
        self.development = enabled;
        self
    }

    #[must_use]
    pub const fn motion_preference(mut self, preference: MotionPreference) -> Self {
        self.motion_preference = preference;
        self
    }

    #[must_use]
    pub fn extension(mut self, extension: impl ScriptAppExtension + 'static) -> Self {
        self.extensions.push(Box::new(extension));
        self
    }

    #[must_use]
    pub fn manifest(mut self, manifest: AppManifest) -> Self {
        self.manifest = manifest;
        self
    }

    #[must_use]
    pub fn key_binding(mut self, binding: KeyBindingSpec) -> Self {
        self.key_bindings.push(binding);
        self
    }

    #[must_use]
    pub fn asset_sources(mut self, assets: impl IntoIterator<Item = (String, AssetData)>) -> Self {
        self.assets = assets.into_iter().collect();
        self
    }

    #[must_use]
    pub const fn viewport_breakpoints(mut self, breakpoints: ViewportBreakpoints) -> Self {
        self.viewport_breakpoints = breakpoints;
        self
    }

    /// Compile and initialize a fully embedded application.
    ///
    /// # Errors
    ///
    /// Returns source, theme, compile, or lifecycle errors.
    pub fn prepare(self) -> Result<PreparedScriptApp, ScriptAppError> {
        let entry = self.scripts.load(&self.entry)?;
        validate_manifest_entry(&self.manifest, &self.entry)?;
        let extensions = Rc::new(self.extensions);
        let mut engine = RuntimeEngine::new();
        for extension in extensions.iter() {
            extension
                .configure_engine(&mut engine)
                .map_err(ScriptAppError::Extension)?;
        }
        let theme = load_theme_source(engine.engine(), "<embedded-theme>", &self.theme_source)
            .map_err(|error| ScriptAppError::Theme(error.to_string()))?;
        engine.set_module_resolver(RestrictedModuleResolver::from_source(&self.scripts)?);
        let compiled = engine.compile_self_contained_named(self.entry.as_str(), &entry.source)?;
        let component_exports = engine.component_exports()?;
        self.manifest.validate_components(&component_exports)?;
        let state_schema = engine.root_state_schema(&compiled)?;
        let mut runtime_state = UiRuntimeState::new();
        runtime_state.animations = AnimationRuntime::new(self.motion_preference);
        runtime_state.responsive = ResponsiveRuntime::new(self.viewport_breakpoints);
        runtime_state.locale = load_embedded_locales(engine.engine(), self.locales)?;
        runtime_state.theme = Some(load_embedded_themes(engine.engine(), self.themes, &theme)?);
        if !self.assets.is_empty() {
            runtime_state
                .assets
                .register("app", InMemoryAssetProvider::new(self.assets))?;
        }
        runtime_state
            .windows
            .register_open("main")
            .map_err(|error| ScriptAppError::Extension(error.to_string()))?;
        for extension in extensions.iter() {
            extension
                .configure_runtime(&mut runtime_state)
                .map_err(ScriptAppError::Extension)?;
        }
        self.manifest.activate(&mut runtime_state.capabilities)?;
        for extension in extensions.iter() {
            extension
                .configure_window("main", &mut runtime_state)
                .map_err(ScriptAppError::Extension)?;
        }
        let runtime = Rc::new(RefCell::new(runtime_state));
        let program = WindowProgram {
            compiled: compiled.clone(),
            state_schema: state_schema.clone(),
            component_exports,
        };
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            ComponentInstancePath::root("App", "main"),
            Some("main".to_owned()),
            BTreeMap::default(),
            &state_schema,
        )?;
        lifecycle.start(&mut engine)?;
        let factory = Rc::new(ScriptWindowFactory {
            program: RefCell::new(program),
            runtime,
            extensions,
            theme: theme.clone(),
            #[cfg(feature = "dev-reload")]
            development: self.development,
        });
        Ok(PreparedScriptApp {
            engine,
            lifecycle,
            window_size: self.window_size,
            theme,
            entry: PathBuf::new(),
            ui_root: PathBuf::new(),
            theme_path: PathBuf::new(),
            development: self.development,
            factory,
            key_bindings: self.key_bindings,
            #[cfg(feature = "dev-reload")]
            module_cache: ModuleCompileCache::new(),
        })
    }

    /// Prepare and run the embedded application.
    ///
    /// # Errors
    ///
    /// Returns preparation or GPUI window errors.
    pub fn run(self) -> Result<(), ScriptAppError> {
        self.prepare()?.run()
    }
}

fn load_embedded_locales(
    engine: &rhai::Engine,
    sources: Vec<(String, String)>,
) -> Result<Option<LocaleManager>, ScriptAppError> {
    let mut bundles = sources
        .into_iter()
        .map(|(name, source)| {
            load_locale_source(engine, &name, &source)
                .map_err(|error| ScriptAppError::Locale(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    bundles.sort_by(|left, right| left.locale.cmp(&right.locale));
    if bundles.is_empty() {
        return Ok(None);
    }
    let fallback = bundles
        .iter()
        .find(|bundle| bundle.locale == "en")
        .unwrap_or(&bundles[0])
        .locale
        .clone();
    Ok(Some(
        LocaleManager::new(bundles, fallback.clone(), fallback)
            .map_err(|error| ScriptAppError::Locale(error.to_string()))?,
    ))
}

fn load_embedded_themes(
    engine: &rhai::Engine,
    sources: Vec<(String, String)>,
    primary: &ThemeVariant,
) -> Result<ThemeManager, ScriptAppError> {
    let mut variants = BTreeMap::from([(
        (primary.family.clone(), primary.name.clone()),
        primary.clone(),
    )]);
    for (name, source) in sources {
        let variant = load_theme_source(engine, &name, &source)
            .map_err(|error| ScriptAppError::Theme(error.to_string()))?;
        insert_theme_variant(&mut variants, variant)?;
    }
    theme_manager(variants.into_values(), primary)
}

fn module_id_from_path(root: &Path, path: &Path) -> Result<ModuleId, ScriptAppError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| ScriptAppError::ModulePath(path.to_path_buf()))?;
    let id = relative
        .with_extension("")
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    Ok(ModuleId::parse(id)?)
}

fn load_file_manifest(root: &Path, entry: &Path) -> Result<AppManifest, ScriptAppError> {
    let path = root.join("app.toml");
    let source = fs::read_to_string(&path).map_err(|source| ScriptAppError::Io {
        path: path.clone(),
        source,
    })?;
    let manifest: AppManifest =
        toml::from_str(&source).map_err(|error| ScriptAppError::Manifest(error.to_string()))?;
    let entry = module_id_from_path(root, entry)?;
    validate_manifest_entry(&manifest, &entry)?;
    Ok(manifest)
}

fn load_primary_file_theme(
    engine: &RuntimeEngine,
    entry: &Path,
) -> Result<(PathBuf, PathBuf, ThemeVariant), ScriptAppError> {
    let root = entry
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let path = root.join("theme.rhai");
    let source = fs::read_to_string(&path).map_err(|source| ScriptAppError::Io {
        path: path.clone(),
        source,
    })?;
    let theme = load_theme_source(engine.engine(), &path.to_string_lossy(), &source)
        .map_err(|error| ScriptAppError::Theme(error.to_string()))?;
    Ok((root, path, theme))
}

fn configure_file_modules(
    engine: &mut RuntimeEngine,
    root: &Path,
    entry: &Path,
    theme: &Path,
) -> Result<ModuleCompileCache, ScriptAppError> {
    let modules = discover_modules(root, entry, theme)?;
    let source = FileScriptSource::new(root, modules)?;
    let mut cache = ModuleCompileCache::new();
    cache.refresh(engine.engine(), &source, source.module_ids())?;
    engine.set_module_resolver(RestrictedModuleResolver::from_source_with_cache(
        &source, &cache,
    )?);
    Ok(cache)
}

fn register_file_assets(runtime: &UiRuntimeState, root: &Path) -> Result<(), ScriptAppError> {
    let asset_root = root.join("assets");
    if asset_root.exists() {
        runtime
            .assets
            .register("app", DirectoryAssetProvider::new(&asset_root)?)?;
    }
    Ok(())
}

fn validate_manifest_entry(manifest: &AppManifest, entry: &ModuleId) -> Result<(), ScriptAppError> {
    if &manifest.entry == entry {
        Ok(())
    } else {
        Err(ScriptAppError::ManifestEntry {
            manifest: manifest.entry.clone(),
            host: entry.clone(),
        })
    }
}

fn discover_modules(
    root: &Path,
    entry: &Path,
    theme: &Path,
) -> Result<Vec<ModuleId>, ScriptAppError> {
    let mut pending = vec![root.to_path_buf()];
    let mut modules = Vec::new();
    while let Some(directory) = pending.pop() {
        for item in fs::read_dir(&directory).map_err(|source| ScriptAppError::Io {
            path: directory.clone(),
            source,
        })? {
            let item = item.map_err(|source| ScriptAppError::Io {
                path: directory.clone(),
                source,
            })?;
            let path = item.path();
            if path.is_dir() {
                if !matches!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some("locales" | "themes")
                ) {
                    pending.push(path);
                }
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rhai")
                && path != entry
                && path != theme
            {
                modules.push(module_id_from_path(root, &path)?);
            }
        }
    }
    modules.sort();
    Ok(modules)
}

fn load_locale_directory(
    engine: &rhai::Engine,
    directory: &Path,
) -> Result<Option<LocaleManager>, ScriptAppError> {
    if !directory.exists() {
        return Ok(None);
    }
    let mut paths = fs::read_dir(directory)
        .map_err(|source| ScriptAppError::Io {
            path: directory.to_path_buf(),
            source,
        })?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|source| ScriptAppError::Io {
                    path: directory.to_path_buf(),
                    source,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.extension().and_then(|extension| extension.to_str()) == Some("rhai"));
    paths.sort();
    let mut bundles = Vec::<LocaleBundle>::new();
    for path in paths {
        let source = fs::read_to_string(&path).map_err(|source| ScriptAppError::Io {
            path: path.clone(),
            source,
        })?;
        bundles.push(
            load_locale_source(engine, &path.to_string_lossy(), &source)
                .map_err(|error| ScriptAppError::Locale(error.to_string()))?,
        );
    }
    if bundles.is_empty() {
        return Ok(None);
    }
    let fallback = bundles
        .iter()
        .find(|bundle| bundle.locale == "en")
        .unwrap_or(&bundles[0])
        .locale
        .clone();
    Ok(Some(
        LocaleManager::new(bundles, fallback.clone(), fallback)
            .map_err(|error| ScriptAppError::Locale(error.to_string()))?,
    ))
}

fn load_theme_directory(
    engine: &rhai::Engine,
    directory: &Path,
    primary: &ThemeVariant,
) -> Result<ThemeManager, ScriptAppError> {
    let mut variants = BTreeMap::from([(
        (primary.family.clone(), primary.name.clone()),
        primary.clone(),
    )]);
    if directory.exists() {
        let mut paths = fs::read_dir(directory)
            .map_err(|source| ScriptAppError::Io {
                path: directory.to_path_buf(),
                source,
            })?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|source| ScriptAppError::Io {
                        path: directory.to_path_buf(),
                        source,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.retain(|path| {
            path.extension().and_then(|extension| extension.to_str()) == Some("rhai")
        });
        paths.sort();
        for path in paths {
            let source = fs::read_to_string(&path).map_err(|source| ScriptAppError::Io {
                path: path.clone(),
                source,
            })?;
            let variant = load_theme_source(engine, &path.to_string_lossy(), &source)
                .map_err(|error| ScriptAppError::Theme(error.to_string()))?;
            insert_theme_variant(&mut variants, variant)?;
        }
    }
    theme_manager(variants.into_values(), primary)
}

fn insert_theme_variant(
    variants: &mut BTreeMap<(String, String), ThemeVariant>,
    variant: ThemeVariant,
) -> Result<(), ScriptAppError> {
    let key = (variant.family.clone(), variant.name.clone());
    if let Some(existing) = variants.get(&key) {
        if existing == &variant {
            return Ok(());
        }
        return Err(ScriptAppError::Theme(format!(
            "theme `{}/{}` is defined more than once with different tokens",
            key.0, key.1
        )));
    }
    variants.insert(key, variant);
    Ok(())
}

fn theme_manager(
    variants: impl IntoIterator<Item = ThemeVariant>,
    primary: &ThemeVariant,
) -> Result<ThemeManager, ScriptAppError> {
    ThemeManager::from_variants(
        variants,
        ThemeSelection::new(primary.family.clone(), primary.name.clone()),
    )
    .map_err(|error| ScriptAppError::Theme(error.to_string()))
}

#[derive(Clone)]
struct WindowProgram {
    compiled: CompiledUi,
    state_schema: ComponentStateSchema,
    component_exports: ComponentRegistry,
}

struct ScriptWindowFactory {
    program: RefCell<WindowProgram>,
    runtime: Rc<RefCell<UiRuntimeState>>,
    extensions: Rc<Vec<Box<dyn ScriptAppExtension>>>,
    theme: ThemeVariant,
    #[cfg(feature = "dev-reload")]
    development: bool,
}

impl ScriptWindowFactory {
    fn instantiate(
        &self,
        window_id: &str,
    ) -> Result<(RuntimeEngine, ScriptLifecycle, PrimitiveRegistry), String> {
        let mut engine = RuntimeEngine::new();
        for extension in self.extensions.iter() {
            extension.configure_engine(&mut engine)?;
        }
        let program = self.program.borrow().clone();
        engine
            .restore_component_exports(program.component_exports.clone())
            .map_err(|error| error.to_string())?;
        {
            let mut runtime = self.runtime.borrow_mut();
            for extension in self.extensions.iter() {
                extension.configure_window(window_id, &mut runtime)?;
            }
        }
        let mut lifecycle = ScriptLifecycle::new(
            program.compiled,
            Rc::clone(&self.runtime),
            ComponentInstancePath::root("App", window_id),
            Some(window_id.to_owned()),
            BTreeMap::new(),
            &program.state_schema,
        )
        .map_err(|error| error.to_string())?;
        lifecycle
            .start(&mut engine)
            .map_err(|error| error.to_string())?;
        let primitives = engine.primitive_registry();
        Ok((engine, lifecycle, primitives))
    }

    #[cfg(feature = "dev-reload")]
    fn update_program(
        &self,
        compiled: CompiledUi,
        state_schema: ComponentStateSchema,
        component_exports: ComponentRegistry,
    ) {
        *self.program.borrow_mut() = WindowProgram {
            compiled,
            state_schema,
            component_exports,
        };
    }

    fn program(&self) -> WindowProgram {
        self.program.borrow().clone()
    }
}

#[derive(Default)]
struct NativeWindowRegistry {
    handles: BTreeMap<String, AnyWindowHandle>,
    force_close: std::collections::BTreeSet<String>,
}

pub struct PreparedScriptApp {
    engine: RuntimeEngine,
    lifecycle: ScriptLifecycle,
    window_size: (f32, f32),
    theme: ThemeVariant,
    entry: PathBuf,
    ui_root: PathBuf,
    theme_path: PathBuf,
    development: bool,
    factory: Rc<ScriptWindowFactory>,
    key_bindings: Vec<KeyBindingSpec>,
    #[cfg(feature = "dev-reload")]
    module_cache: ModuleCompileCache,
}

impl PreparedScriptApp {
    /// Open the initial GPUI window and retain the script runtime in its entity.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptAppError::Window`] when GPUI cannot open the window.
    pub fn run(self) -> Result<(), ScriptAppError> {
        let key_bindings = self
            .key_bindings
            .iter()
            .map(KeyBindingSpec::to_gpui)
            .collect::<Result<Vec<_>, _>>()?;
        #[cfg(feature = "dev-reload")]
        let watcher = if self.development {
            Some(FileWatcher::new(&self.ui_root)?)
        } else {
            None
        };
        #[cfg(not(feature = "dev-reload"))]
        let _ = (
            &self.entry,
            &self.ui_root,
            &self.theme_path,
            self.development,
        );
        let error = Rc::new(RefCell::new(None));
        let reported_error = Rc::clone(&error);
        let native_windows = Rc::new(RefCell::new(NativeWindowRegistry::default()));
        Application::new().run(move |cx: &mut App| {
            self.launch_initial_window(
                #[cfg(feature = "dev-reload")]
                watcher,
                &native_windows,
                &reported_error,
                key_bindings,
                cx,
            );
        });
        match error.borrow_mut().take() {
            Some(message) => Err(ScriptAppError::Window(message)),
            None => Ok(()),
        }
    }

    fn launch_initial_window(
        self,
        #[cfg(feature = "dev-reload")] watcher: Option<FileWatcher>,
        native_windows: &Rc<RefCell<NativeWindowRegistry>>,
        reported_error: &Rc<RefCell<Option<String>>>,
        key_bindings: Vec<KeyBinding>,
        cx: &mut App,
    ) {
        init_text_input(cx);
        cx.bind_keys(key_bindings);
        #[cfg(feature = "dev-reload")]
        cx.bind_keys([
            KeyBinding::new("cmd-alt-i", ToggleInspector, Some(HOST_KEY_CONTEXT)),
            KeyBinding::new("f12", ToggleInspector, Some(HOST_KEY_CONTEXT)),
        ]);
        let bounds = Bounds::centered(
            None,
            size(px(self.window_size.0), px(self.window_size.1)),
            cx,
        );
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let primitives = self.engine.primitive_registry();
        let timings = self.engine.take_timings();
        let factory = Rc::clone(&self.factory);
        let view_native_windows = Rc::clone(native_windows);
        let result = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..WindowOptions::default()
            },
            |window, cx| {
                let entity = cx.new(|entity_cx| {
                    let async_task = spawn_host_poll(entity_cx);
                    let host_focus = entity_cx.focus_handle();
                    host_focus.focus(window);
                    #[cfg(feature = "dev-reload")]
                    let reload_task = watcher.as_ref().map(|_| spawn_host_reload_poll(entity_cx));
                    ScriptHostView {
                        window_id: "main".to_owned(),
                        engine: self.engine,
                        lifecycle: self.lifecycle,
                        primitives,
                        last_error: None,
                        theme: self.theme,
                        #[cfg(feature = "dev-reload")]
                        development: self.development,
                        #[cfg(feature = "dev-reload")]
                        inspector_open: false,
                        timings,
                        overlays: WindowOverlayCoordinator::default(),
                        factory: Rc::clone(&factory),
                        native_windows: Rc::clone(&view_native_windows),
                        host_focus,
                        disposed: false,
                        _async_task: async_task,
                        #[cfg(feature = "dev-reload")]
                        module_cache: self.module_cache,
                        #[cfg(feature = "dev-reload")]
                        watcher,
                        #[cfg(feature = "dev-reload")]
                        entry: self.entry,
                        #[cfg(feature = "dev-reload")]
                        ui_root: self.ui_root,
                        #[cfg(feature = "dev-reload")]
                        theme_path: self.theme_path,
                        #[cfg(feature = "dev-reload")]
                        _reload_task: reload_task,
                    }
                });
                install_close_interceptor(window, cx, &entity);
                entity
            },
        );
        match result {
            Ok(handle) => {
                native_windows
                    .borrow_mut()
                    .handles
                    .insert("main".to_owned(), handle.into());
                cx.activate(true);
            }
            Err(window_error) => {
                *reported_error.borrow_mut() = Some(window_error.to_string());
                cx.quit();
            }
        }
    }
}

fn open_secondary_window(
    spec: &ScriptWindowSpec,
    factory: &Rc<ScriptWindowFactory>,
    native_windows: &Rc<RefCell<NativeWindowRegistry>>,
    cx: &mut App,
) -> Result<(), String> {
    let factory = Rc::clone(factory);
    let native_windows = Rc::clone(native_windows);
    let window_id = spec.id.clone();
    let (engine, lifecycle, primitives) = match factory.instantiate(&window_id) {
        Ok(instance) => instance,
        Err(error) => {
            let root = ComponentInstancePath::root("App", &window_id);
            let _ = factory
                .runtime
                .borrow_mut()
                .release_window(&window_id, &root);
            return Err(error);
        }
    };
    let timings = engine.take_timings();
    let options = script_window_options(spec, cx);
    let view_factory = Rc::clone(&factory);
    let view_native_windows = Rc::clone(&native_windows);
    let view_window_id = window_id.clone();
    let result = cx.open_window(options, move |window, cx| {
        let entity = cx.new(|entity_cx| {
            let async_task = spawn_host_poll(entity_cx);
            let host_focus = entity_cx.focus_handle();
            host_focus.focus(window);
            ScriptHostView {
                window_id: view_window_id,
                engine,
                lifecycle,
                primitives,
                last_error: None,
                theme: view_factory.theme.clone(),
                #[cfg(feature = "dev-reload")]
                development: view_factory.development,
                #[cfg(feature = "dev-reload")]
                inspector_open: false,
                timings,
                overlays: WindowOverlayCoordinator::default(),
                factory: Rc::clone(&view_factory),
                native_windows: Rc::clone(&view_native_windows),
                host_focus,
                disposed: false,
                _async_task: async_task,
                #[cfg(feature = "dev-reload")]
                module_cache: ModuleCompileCache::new(),
                #[cfg(feature = "dev-reload")]
                watcher: None,
                #[cfg(feature = "dev-reload")]
                entry: PathBuf::new(),
                #[cfg(feature = "dev-reload")]
                ui_root: PathBuf::new(),
                #[cfg(feature = "dev-reload")]
                theme_path: PathBuf::new(),
                #[cfg(feature = "dev-reload")]
                _reload_task: None,
            }
        });
        install_close_interceptor(window, cx, &entity);
        entity
    });
    match result {
        Ok(handle) => {
            native_windows
                .borrow_mut()
                .handles
                .insert(window_id.clone(), handle.into());
            factory
                .runtime
                .borrow_mut()
                .windows
                .mark_open(&window_id)
                .map_err(|error| error.to_string())?;
            Ok(())
        }
        Err(error) => {
            let root = ComponentInstancePath::root("App", &window_id);
            let mut runtime = factory.runtime.borrow_mut();
            let _ = runtime.release_window(&window_id, &root);
            Err(error.to_string())
        }
    }
}

fn script_window_options(spec: &ScriptWindowSpec, cx: &mut App) -> WindowOptions {
    let bounds = Bounds::centered(
        None,
        size(
            px(validated_dimension_to_f32(spec.width)),
            px(validated_dimension_to_f32(spec.height)),
        ),
        cx,
    );
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some(SharedString::from(spec.title.clone())),
            ..TitlebarOptions::default()
        }),
        focus: spec.focus,
        ..WindowOptions::default()
    }
}

fn install_close_interceptor(window: &Window, cx: &App, entity: &gpui::Entity<ScriptHostView>) {
    let weak = entity.downgrade();
    window.on_window_should_close(cx, move |_, app| {
        weak.update(app, ScriptHostView::should_close)
            .unwrap_or(true)
    });
}

fn spawn_host_poll(cx: &mut Context<ScriptHostView>) -> Task<()> {
    cx.spawn(async move |entity: gpui::WeakEntity<ScriptHostView>, cx| {
        loop {
            Timer::after(Duration::from_millis(16)).await;
            if entity.update(cx, ScriptHostView::poll_async).is_err() {
                break;
            }
        }
    })
}

#[cfg(feature = "dev-reload")]
fn spawn_host_reload_poll(cx: &mut Context<ScriptHostView>) -> Task<()> {
    cx.spawn(async move |entity: gpui::WeakEntity<ScriptHostView>, cx| {
        loop {
            Timer::after(Duration::from_millis(100)).await;
            if entity.update(cx, ScriptHostView::poll_reload).is_err() {
                break;
            }
        }
    })
}

#[allow(clippy::cast_possible_truncation)]
fn validated_dimension_to_f32(value: f64) -> f32 {
    debug_assert!(value.is_finite() && (200.0..=4096.0).contains(&value));
    value as f32
}

fn event_propagation_from_dynamic(value: &rhai::Dynamic) -> crate::EventPropagation {
    if value.is::<rhai::ImmutableString>()
        && value.clone_cast::<rhai::ImmutableString>().as_str() == "propagate"
    {
        crate::EventPropagation::Propagate
    } else {
        crate::EventPropagation::Handled
    }
}

struct ScriptHostView {
    window_id: String,
    engine: RuntimeEngine,
    lifecycle: ScriptLifecycle,
    primitives: PrimitiveRegistry,
    last_error: Option<String>,
    theme: ThemeVariant,
    #[cfg(feature = "dev-reload")]
    development: bool,
    #[cfg(feature = "dev-reload")]
    inspector_open: bool,
    timings: Vec<crate::ExecutionTiming>,
    overlays: WindowOverlayCoordinator,
    factory: Rc<ScriptWindowFactory>,
    native_windows: Rc<RefCell<NativeWindowRegistry>>,
    host_focus: FocusHandle,
    disposed: bool,
    _async_task: Task<()>,
    #[cfg(feature = "dev-reload")]
    module_cache: ModuleCompileCache,
    #[cfg(feature = "dev-reload")]
    watcher: Option<FileWatcher>,
    #[cfg(feature = "dev-reload")]
    entry: PathBuf,
    #[cfg(feature = "dev-reload")]
    ui_root: PathBuf,
    #[cfg(feature = "dev-reload")]
    theme_path: PathBuf,
    #[cfg(feature = "dev-reload")]
    _reload_task: Option<Task<()>>,
}

fn handle_tab_navigation(event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) {
    if event.keystroke.key.as_str() == "tab" {
        if event.keystroke.modifiers.shift {
            window.focus_prev();
        } else {
            window.focus_next();
        }
        cx.stop_propagation();
    }
}

fn apply_root_text_direction(
    root: gpui::Stateful<gpui::Div>,
    direction: TextDirection,
) -> gpui::Stateful<gpui::Div> {
    match direction {
        TextDirection::LeftToRight => root.text_left(),
        TextDirection::RightToLeft => root.text_right(),
    }
}

fn build_host_root(host_focus: &FocusHandle, theme: &ThemeVariant) -> gpui::Stateful<gpui::Div> {
    div()
        .id("gpui-rhai-host")
        .size_full()
        .track_focus(host_focus)
        .bg(rgba(theme.tokens.colors["surface"].as_rgba_hex()))
        .text_color(rgba(theme.tokens.colors["text_primary"].as_rgba_hex()))
        .key_context(HOST_KEY_CONTEXT)
        .on_key_down(handle_tab_navigation)
}

impl Render for ScriptHostView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.prepare_render(window);
        let entity = cx.entity().downgrade();
        let dispatcher = NodeEventDispatcher::new(move |callback, payload, _, app| {
            entity
                .update(app, |view, cx| {
                    view.handle_node_event(&callback, payload, cx)
                })
                .unwrap_or(crate::EventPropagation::Handled)
        });
        let runtime = self.lifecycle.runtime();
        let appearance = match window.appearance() {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => SystemAppearance::Dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => SystemAppearance::Light,
        };
        let (assets, theme, animations, direction) = {
            let runtime = runtime.borrow();
            let root = ComponentInstancePath::root("App", self.window_id.clone());
            let theme = runtime
                .theme
                .as_ref()
                .and_then(|themes| {
                    themes
                        .resolve(Some(&self.window_id), Some(&root), appearance)
                        .ok()
                })
                .map_or_else(|| self.theme.clone(), |resolved| resolved.variant().clone());
            (
                runtime.assets.clone(),
                theme,
                runtime.animation_values.clone(),
                runtime
                    .locale
                    .as_ref()
                    .and_then(|locale| locale.direction(Some(&self.window_id), Some(&root)).ok())
                    .unwrap_or(TextDirection::LeftToRight),
            )
        };
        let animation_root = format!("window:{}/root", self.window_id);
        let render_resources = crate::renderer::WindowRenderResources {
            assets: &assets,
            dispatcher: &dispatcher,
            overlays: &self.overlays,
            animations: &animations,
            direction,
            root_path: &animation_root,
        };
        let content = self.lifecycle.root().map_or_else(
            || div().child("Script view has no root").into_any_element(),
            |root| {
                GpuiNodeRenderer::render_with_window_runtime(
                    root,
                    &theme,
                    &InteractionState::default(),
                    &self.primitives,
                    &render_resources,
                )
            },
        );
        let content = match &self.last_error {
            None => content,
            Some(error) => div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .p_2()
                        .bg(rgba(theme.tokens.colors["surface_raised"].as_rgba_hex()))
                        .text_color(rgba(theme.tokens.colors["danger"].as_rgba_hex()))
                        .border_1()
                        .border_color(rgba(theme.tokens.colors["danger"].as_rgba_hex()))
                        .child(error.clone()),
                )
                .child(content)
                .into_any_element(),
        };
        #[cfg(feature = "dev-reload")]
        let inspector = self.inspector_element(&runtime, &theme);
        let root = build_host_root(&self.host_focus, &theme)
            .child(content)
            .children({
                #[cfg(feature = "dev-reload")]
                {
                    inspector
                }
                #[cfg(not(feature = "dev-reload"))]
                {
                    Option::<gpui::AnyElement>::None
                }
            });
        let root = apply_root_text_direction(root, direction);
        let root = root.on_action(cx.listener(Self::dispatch_key_binding));
        #[cfg(feature = "dev-reload")]
        let root = root.on_action(cx.listener(Self::toggle_inspector));
        root.into_any_element()
    }
}

impl ScriptHostView {
    fn prepare_render(&mut self, window: &Window) {
        self.sync_viewport_class(window);
        debug_assert!(self.engine.is_current(self.lifecycle.generation()));
        self.reconcile_primitive_lifecycle();
        self.overlays.begin_frame(window.viewport_size());
    }

    fn reconcile_primitive_lifecycle(&mut self) {
        if let Some(root) = self.lifecycle.root()
            && let Err(error) = self.primitives.retain_tree(root)
        {
            self.last_error = Some(error.to_string());
        }
    }

    fn sync_viewport_class(&mut self, window: &Window) {
        let width = f64::from(window.viewport_size().width);
        let changed = self
            .lifecycle
            .runtime()
            .borrow_mut()
            .responsive
            .update_window(&self.window_id, width);
        match changed {
            Ok(true) => {
                if let Err(error) = self.lifecycle.render(&mut self.engine) {
                    self.last_error = Some(error.to_string());
                }
            }
            Ok(false) => {}
            Err(error) => self.last_error = Some(error.to_string()),
        }
    }

    fn dispatch_key_binding(
        &mut self,
        action: &DispatchScriptAction,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let result = ActionId::parse(&action.id).and_then(|id| {
            self.lifecycle
                .runtime()
                .borrow()
                .actions
                .dispatch(&id, UiValue::Null)
        });
        match result {
            Ok(invocation) => {
                if let Ok(mut runtime) = self.lifecycle.runtime().try_borrow_mut() {
                    runtime.traces.push(
                        crate::RuntimeTraceKind::Action,
                        self.lifecycle.root_path().to_string(),
                        format!("key binding {}", action.id),
                        None,
                        false,
                    );
                }
                self.handle_node_event(&invocation.callback, invocation.payload, cx);
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                cx.notify();
            }
        }
    }

    fn invoke_pending_effects(&mut self) -> Result<bool, String> {
        let mut processed = 0usize;
        loop {
            let (actions, events) = {
                let runtime = self.lifecycle.runtime();
                let mut runtime = runtime.borrow_mut();
                (
                    runtime.drain_pending_actions(),
                    runtime.drain_pending_events(),
                )
            };
            if actions.is_empty() && events.is_empty() {
                return Ok(processed > 0);
            }
            processed = processed.saturating_add(actions.len() + events.len());
            if processed > 64 {
                return Err(
                    "semantic event/action dispatch exceeded the 64-callback budget".to_owned(),
                );
            }
            for action in actions {
                let _ = self
                    .lifecycle
                    .invoke_callback_transactional(&self.engine, &action.callback, action.payload)
                    .map_err(|error| error.to_string())?;
            }
            for event in events {
                let _ = self
                    .lifecycle
                    .invoke_component_event_transactional(&self.engine, event)
                    .map_err(|error| error.to_string())?;
            }
        }
    }

    #[cfg(feature = "dev-reload")]
    fn inspector_element(
        &self,
        runtime: &Rc<RefCell<UiRuntimeState>>,
        theme: &ThemeVariant,
    ) -> Option<gpui::AnyElement> {
        (self.development && self.inspector_open).then(|| {
            let components = self.engine.component_exports().unwrap_or_default();
            let runtime = runtime.borrow();
            let snapshot = crate::InspectorSnapshot::capture(
                self.lifecycle.root(),
                &runtime,
                theme,
                &components,
                self.timings.clone(),
            );
            crate::devtools::inspector_element(&snapshot)
        })
    }

    fn should_close(&mut self, cx: &mut Context<Self>) -> bool {
        if self.disposed {
            return true;
        }
        let forced = self
            .native_windows
            .borrow_mut()
            .force_close
            .remove(&self.window_id);
        if forced {
            self.release_window();
            return true;
        }
        let handler = self
            .lifecycle
            .runtime()
            .borrow()
            .windows
            .close_handler(&self.window_id);
        let Some(handler) = handler else {
            self.release_window();
            return true;
        };
        let result = self
            .lifecycle
            .invoke_callback_transactional(&self.engine, &handler, UiValue::Null)
            .map_err(|error| error.to_string())
            .and_then(|_| self.invoke_pending_effects().map(|_| ()))
            .and_then(|()| {
                self.lifecycle
                    .render(&mut self.engine)
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(()) => {
                self.last_error = None;
                self.process_window_commands(cx);
                cx.notify();
                false
            }
            Err(error) => {
                self.last_error = Some(error);
                self.release_window();
                true
            }
        }
    }

    fn release_window(&mut self) {
        if self.disposed {
            return;
        }
        if let Err(error) = self.primitives.retain_mounted(&BTreeSet::new()) {
            self.last_error = Some(error.to_string());
        }
        let _ = self.lifecycle.dispose(&self.engine);
        let root = self.lifecycle.root_path().clone();
        let _ = self
            .lifecycle
            .runtime()
            .borrow_mut()
            .release_window(&self.window_id, &root);
        let mut native = self.native_windows.borrow_mut();
        native.handles.remove(&self.window_id);
        native.force_close.remove(&self.window_id);
        self.disposed = true;
    }

    fn process_window_commands(&mut self, cx: &mut Context<Self>) {
        let commands = self
            .lifecycle
            .runtime()
            .borrow_mut()
            .windows
            .drain_commands();
        for command in commands {
            let result = match command {
                WindowCommand::Open(spec) => {
                    open_secondary_window(&spec, &self.factory, &self.native_windows, cx)
                }
                WindowCommand::Focus(id) => {
                    let handle = self.native_windows.borrow().handles.get(&id).copied();
                    handle.map_or_else(
                        || Err(format!("native window `{id}` is unavailable")),
                        |handle| {
                            handle
                                .update(cx, |_, window, _| window.activate_window())
                                .map_err(|error| error.to_string())
                        },
                    )
                }
                WindowCommand::Close(id) => {
                    let handle = {
                        let mut native = self.native_windows.borrow_mut();
                        native.force_close.insert(id.clone());
                        native.handles.get(&id).copied()
                    };
                    handle.map_or_else(
                        || {
                            self.native_windows.borrow_mut().force_close.remove(&id);
                            Err(format!("native window `{id}` is unavailable"))
                        },
                        |handle| {
                            // A script commonly confirms closing from an event
                            // dispatched by the same window. Updating that window
                            // recursively fails in GPUI with `window not found`, so
                            // remove it after the current entity update unwinds.
                            let view = cx.weak_entity();
                            let native_windows = Rc::clone(&self.native_windows);
                            let close_id = id.clone();
                            cx.defer(move |cx| {
                                if let Err(error) =
                                    handle.update(cx, |_, window, _| window.remove_window())
                                {
                                    native_windows.borrow_mut().force_close.remove(&close_id);
                                    let message = error.to_string();
                                    let _ = view.update(cx, |view, cx| {
                                        view.last_error = Some(message);
                                        cx.notify();
                                    });
                                }
                            });
                            Ok(())
                        },
                    )
                }
            };
            if let Err(error) = result {
                self.last_error = Some(error);
            }
        }
    }

    #[cfg(feature = "dev-reload")]
    fn toggle_inspector(&mut self, _: &ToggleInspector, _: &mut Window, cx: &mut Context<Self>) {
        if self.development {
            self.inspector_open = !self.inspector_open;
            cx.notify();
        }
    }

    fn handle_node_event(
        &mut self,
        callback: &ScriptCallback,
        payload: UiValue,
        cx: &mut Context<Self>,
    ) -> crate::EventPropagation {
        if let Ok(mut runtime) = self.lifecycle.runtime().try_borrow_mut() {
            runtime.traces.push(
                crate::RuntimeTraceKind::Event,
                "/App[root]",
                format!("callback {}", callback.name()),
                Some(payload.clone()),
                false,
            );
        }
        let callback_result = self
            .lifecycle
            .invoke_callback_transactional(&self.engine, callback, payload)
            .map_err(|error| error.to_string());
        let propagation = callback_result.as_ref().map_or(
            crate::EventPropagation::Handled,
            event_propagation_from_dynamic,
        );
        let result = callback_result
            .map(|_| ())
            .and_then(|()| self.invoke_pending_effects().map(|_| ()))
            .and_then(|()| {
                self.lifecycle
                    .render(&mut self.engine)
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            });
        let succeeded = result.is_ok();
        match result {
            Ok(()) => self.last_error = None,
            Err(error) => self.last_error = Some(error),
        }
        let root = self.lifecycle.root_path().clone();
        self.lifecycle
            .runtime()
            .borrow_mut()
            .take_window_dirty(&root);
        self.process_window_commands(cx);
        self.collect_timings();
        cx.notify();
        if succeeded {
            propagation
        } else {
            crate::EventPropagation::Handled
        }
    }

    fn collect_timings(&mut self) {
        self.timings.extend(self.engine.take_timings());
        if self.timings.len() > 200 {
            self.timings.drain(0..self.timings.len() - 200);
        }
    }

    fn poll_async(&mut self, cx: &mut Context<Self>) {
        self.process_window_commands(cx);
        self.sync_program();
        let (initial_effects, mut error) = match self.invoke_pending_effects() {
            Ok(processed) => (processed, None),
            Err(error) => (false, Some(error)),
        };
        let generation = self.lifecycle.generation();
        let runtime = self.lifecycle.runtime();
        let root = self.lifecycle.root_path().clone();
        let (deliveries, animation_active, dirty) = {
            let mut runtime = runtime.borrow_mut();
            let _ = runtime.assets.retain_decode_generation(generation);
            let mut deliveries = runtime.tasks.drain(generation);
            deliveries.extend(runtime.subscriptions.drain(generation));
            if let Ok(asset_deliveries) = runtime.assets.drain_image_decodes(generation) {
                deliveries.extend(asset_deliveries);
            }
            runtime.queue_async(deliveries);
            let deliveries = runtime.take_window_async(&self.window_id, &root);
            let now = std::time::Instant::now();
            let frame = runtime.animations.tick(now);
            runtime.animation_values = runtime.animations.snapshot(now);
            (
                deliveries,
                frame.needs_frame || !frame.values.is_empty(),
                runtime.take_window_dirty(&root),
            )
        };
        if deliveries.is_empty() && !animation_active && !dirty && !initial_effects {
            if error.is_some() {
                self.last_error = error;
                cx.notify();
            }
            return;
        }

        for delivery in deliveries {
            if let Err(delivery_error) = self
                .lifecycle
                .invoke_async_delivery_transactional(&self.engine, delivery)
            {
                error = Some(delivery_error.to_string());
            }
        }
        if error.is_none()
            && let Err(action_error) = self.invoke_pending_effects()
        {
            error = Some(action_error);
        }
        if error.is_none()
            && let Err(render_error) = self.lifecycle.render(&mut self.engine)
        {
            error = Some(render_error.to_string());
        }
        self.last_error = error;
        self.collect_timings();
        cx.notify();
    }

    fn sync_program(&mut self) {
        let program = self.factory.program();
        if program.compiled.generation() == self.lifecycle.generation() {
            return;
        }
        if let Err(error) =
            self.lifecycle
                .reload(&mut self.engine, program.compiled, &program.state_schema)
        {
            self.last_error = Some(error.to_string());
        }
    }

    #[cfg(feature = "dev-reload")]
    fn poll_reload(&mut self, cx: &mut Context<Self>) {
        let Some(watcher) = &self.watcher else {
            return;
        };
        let batch = match watcher.poll() {
            Ok(batch) => batch,
            Err(error) => {
                self.last_error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        if batch.paths.is_empty() {
            return;
        }
        let theme_path = self.theme_path.canonicalize().ok();
        let themes_root = self.ui_root.join("themes").canonicalize().ok();
        let theme_changed = theme_path
            .as_ref()
            .is_some_and(|theme| batch.paths.contains(theme))
            || themes_root.as_ref().is_some_and(|themes_root| {
                batch.paths.iter().any(|path| path.starts_with(themes_root))
            });
        let locale_root = self.ui_root.join("locales").canonicalize().ok();
        let locale_changed = locale_root.as_ref().is_some_and(|locale_root| {
            batch.paths.iter().any(|path| path.starts_with(locale_root))
        });
        let assets_root = self.ui_root.join("assets").canonicalize().ok();
        let assets_changed = assets_root.as_ref().is_some_and(|assets_root| {
            batch.paths.iter().any(|path| path.starts_with(assets_root))
        });
        let manifest_path = self.ui_root.join("app.toml").canonicalize().ok();
        let manifest_changed = manifest_path
            .as_ref()
            .is_some_and(|manifest| batch.paths.contains(manifest));
        let script_changed = batch.paths.iter().any(|path| {
            path.extension().and_then(|extension| extension.to_str()) == Some("rhai")
                && Some(path) != theme_path.as_ref()
                && !locale_root
                    .as_ref()
                    .is_some_and(|locale_root| path.starts_with(locale_root))
                && !themes_root
                    .as_ref()
                    .is_some_and(|themes_root| path.starts_with(themes_root))
        });

        let result = if script_changed {
            self.reload_scripts(&batch.paths)
        } else {
            Ok(())
        }
        .and_then(|()| {
            if theme_changed {
                self.reload_theme()
            } else {
                Ok(())
            }
        })
        .and_then(|()| {
            if locale_changed {
                self.reload_locales()
            } else {
                Ok(())
            }
        })
        .and_then(|()| {
            if assets_changed {
                self.reload_assets()
            } else {
                Ok(())
            }
        })
        .and_then(|()| {
            if manifest_changed {
                self.reload_manifest()
            } else {
                Ok(())
            }
        });
        match result {
            Ok(()) => self.last_error = None,
            Err(error) => self.last_error = Some(error),
        }
        self.collect_timings();
        cx.notify();
    }

    #[cfg(feature = "dev-reload")]
    fn reload_scripts(&mut self, changed_paths: &BTreeSet<PathBuf>) -> Result<(), String> {
        let source = fs::read_to_string(&self.entry).map_err(|error| error.to_string())?;
        let modules = discover_modules(&self.ui_root, &self.entry, &self.theme_path)
            .map_err(|error| error.to_string())?;
        let file_source =
            FileScriptSource::new(&self.ui_root, modules).map_err(|error| error.to_string())?;
        let root = self
            .ui_root
            .canonicalize()
            .map_err(|error| error.to_string())?;
        let changed_modules = changed_paths.iter().filter_map(|path| {
            let relative = path.strip_prefix(&root).ok()?;
            (relative.extension().and_then(|value| value.to_str()) == Some("rhai"))
                .then(|| {
                    ModuleId::parse(
                        relative
                            .with_extension("")
                            .components()
                            .map(|component| component.as_os_str().to_string_lossy())
                            .collect::<Vec<_>>()
                            .join("/"),
                    )
                    .ok()
                })
                .flatten()
        });
        let refresh = self
            .module_cache
            .refresh(self.engine.engine(), &file_source, changed_modules)
            .map_err(|error| error.to_string())?;
        let resolver =
            RestrictedModuleResolver::from_source_with_cache(&file_source, &self.module_cache)
                .map_err(|error| error.to_string())?;
        let previous_exports = self
            .engine
            .component_exports()
            .map_err(|error| error.to_string())?;
        self.engine
            .clear_component_exports()
            .map_err(|error| error.to_string())?;
        self.engine.set_module_resolver(resolver);
        let candidate = self
            .engine
            .compile_self_contained_named(&self.entry.to_string_lossy(), &source);
        let result = candidate
            .map_err(|error| error.to_string())
            .and_then(|candidate| {
                let state_schema = self
                    .engine
                    .root_state_schema(&candidate)
                    .map_err(|error| error.to_string())?;
                let program_compiled = candidate.clone();
                let program_schema = state_schema.clone();
                let program_exports = self
                    .engine
                    .component_exports()
                    .map_err(|error| error.to_string())?;
                self.lifecycle
                    .reload(&mut self.engine, candidate, &state_schema)
                    .map(|_| {
                        self.factory.update_program(
                            program_compiled,
                            program_schema,
                            program_exports,
                        );
                    })
                    .map_err(|error| error.to_string())
            });
        if result.is_err() {
            self.engine
                .restore_component_exports(previous_exports)
                .map_err(|error| error.to_string())?;
        }
        if result.is_ok() {
            self.lifecycle.runtime().borrow_mut().traces.push(
                crate::RuntimeTraceKind::Reload,
                self.lifecycle.root_path().to_string(),
                format!(
                    "refreshed {} affected module(s), compiled {}",
                    refresh.affected.len(),
                    refresh.compiled.len()
                ),
                None,
                false,
            );
        }
        result
    }

    #[cfg(feature = "dev-reload")]
    fn reload_theme(&mut self) -> Result<(), String> {
        let source = fs::read_to_string(&self.theme_path).map_err(|error| error.to_string())?;
        let primary = load_theme_source(
            self.engine.engine(),
            &self.theme_path.to_string_lossy(),
            &source,
        )
        .map_err(|error| error.to_string())?;
        let runtime = self.lifecycle.runtime();
        let previous = runtime
            .borrow()
            .theme
            .as_ref()
            .map(|themes| themes.app_preference().clone());
        let mut themes =
            load_theme_directory(self.engine.engine(), &self.ui_root.join("themes"), &primary)
                .map_err(|error| error.to_string())?;
        if let Some(previous) = previous {
            themes
                .set_app(previous)
                .map_err(|error| error.to_string())?;
        }
        self.theme = themes
            .resolve(
                Some(&self.window_id),
                Some(&ComponentInstancePath::root("App", &self.window_id)),
                SystemAppearance::Dark,
            )
            .map_err(|error| error.to_string())?
            .variant()
            .clone();
        runtime.borrow_mut().theme = Some(themes);
        runtime.borrow_mut().mark_all_windows_dirty();
        Ok(())
    }

    #[cfg(feature = "dev-reload")]
    fn reload_locales(&mut self) -> Result<(), String> {
        let runtime = self.lifecycle.runtime();
        let previous = runtime
            .borrow()
            .locale
            .as_ref()
            .map(|locale| locale.app_locale().to_owned());
        let mut locales =
            load_locale_directory(self.engine.engine(), &self.ui_root.join("locales"))
                .map_err(|error| error.to_string())?;
        if let (Some(previous), Some(locales)) = (&previous, &mut locales) {
            let _ = locales.set_app(previous);
        }
        let mut runtime = runtime.borrow_mut();
        runtime.locale = locales;
        runtime.mark_all_windows_dirty();
        Ok(())
    }

    #[cfg(feature = "dev-reload")]
    fn reload_assets(&mut self) -> Result<(), String> {
        let runtime = self.lifecycle.runtime();
        let mut runtime = runtime.borrow_mut();
        runtime
            .assets
            .refresh_namespace("app")
            .map_err(|error| error.to_string())?;
        runtime.mark_all_windows_dirty();
        Ok(())
    }

    #[cfg(feature = "dev-reload")]
    fn reload_manifest(&mut self) -> Result<(), String> {
        let manifest =
            load_file_manifest(&self.ui_root, &self.entry).map_err(|error| error.to_string())?;
        let runtime = self.lifecycle.runtime();
        let mut runtime = runtime.borrow_mut();
        manifest
            .activate(&mut runtime.capabilities)
            .map_err(|error| error.to_string())?;
        runtime.mark_all_windows_dirty();
        Ok(())
    }
}

impl Drop for ScriptHostView {
    fn drop(&mut self) {
        self.release_window();
    }
}

#[derive(Debug, Error)]
pub enum ScriptAppError {
    #[error("failed to read script entry `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Lifecycle(#[from] crate::LifecycleError),
    #[error("failed to open GPUI window: {0}")]
    Window(String),
    #[error("failed to load UI theme: {0}")]
    Theme(String),
    #[error("failed to load UI locale: {0}")]
    Locale(String),
    #[error("runtime extension failed: {0}")]
    Extension(String),
    #[error("failed to decode application manifest: {0}")]
    Manifest(String),
    #[error("application manifest entry `{manifest}` does not match host entry `{host}`")]
    ManifestEntry { manifest: ModuleId, host: ModuleId },
    #[error(transparent)]
    Capability(#[from] CapabilityError),
    #[error(transparent)]
    Responsive(#[from] ResponsiveError),
    #[error(transparent)]
    Dependency(#[from] DependencyError),
    #[error(transparent)]
    ComponentExport(#[from] ComponentExportError),
    #[error(transparent)]
    Action(#[from] ActionError),
    #[error("Rhai module path `{0}` is outside the UI root")]
    ModulePath(PathBuf),
    #[error(transparent)]
    ModuleId(#[from] crate::ModuleIdError),
    #[error(transparent)]
    ScriptSource(#[from] crate::ScriptSourceError),
    #[error(transparent)]
    Asset(#[from] crate::AssetError),
    #[cfg(feature = "dev-reload")]
    #[error(transparent)]
    Watcher(#[from] crate::WatcherError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WindowCommand;

    fn write_manifest(directory: &Path) {
        fs::write(
            directory.join("app.toml"),
            "entry = \"main\"\nruntime_api = 1\n",
        )
        .unwrap();
    }

    #[test]
    fn reduced_motion_environment_values_are_normalized() {
        assert_eq!(parse_motion_preference("1"), MotionPreference::Reduced);
        assert_eq!(parse_motion_preference("TRUE"), MotionPreference::Reduced);
        assert_eq!(
            parse_motion_preference("reduced"),
            MotionPreference::Reduced
        );
        assert_eq!(parse_motion_preference("0"), MotionPreference::Normal);
    }

    #[test]
    fn pointer_callback_return_controls_gpui_propagation() {
        assert_eq!(
            event_propagation_from_dynamic(&rhai::Dynamic::from("propagate")),
            crate::EventPropagation::Propagate
        );
        assert_eq!(
            event_propagation_from_dynamic(&rhai::Dynamic::UNIT),
            crate::EventPropagation::Handled
        );
    }

    #[test]
    fn prepare_keeps_engine_and_lifecycle_alive() {
        let directory = tempfile::tempdir().unwrap();
        let entry = directory.path().join("main.rhai");
        fs::write(&entry, "fn view(ctx) { text(\"prepared\") }").unwrap();
        fs::write(
            directory.path().join("theme.rhai"),
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .unwrap();
        write_manifest(directory.path());
        let prepared = ScriptApp::new(&entry).prepare().unwrap();
        assert!(prepared.engine.is_current(prepared.lifecycle.generation()));
        assert!(prepared.lifecycle.root().is_some());
    }

    #[test]
    fn prepare_resolves_copied_component_modules() {
        let directory = tempfile::tempdir().unwrap();
        let components = directory.path().join("components");
        fs::create_dir_all(&components).unwrap();
        fs::write(
            components.join("greeting.rhai"),
            "fn Greeting() { text(\"hello\") }",
        )
        .unwrap();
        fs::write(
            directory.path().join("main.rhai"),
            "import \"components/greeting\" as greeting; fn view(ctx) { greeting::Greeting() }",
        )
        .unwrap();
        fs::write(
            directory.path().join("theme.rhai"),
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .unwrap();
        write_manifest(directory.path());

        let prepared = ScriptApp::new(directory.path().join("main.rhai"))
            .prepare()
            .unwrap();
        let root = prepared.lifecycle.root().unwrap();
        assert!(matches!(
            root.kind(),
            crate::UiNodeKind::Text { text } if text == "hello"
        ));
        assert_eq!(
            root.source().map(|source| source.module.as_str()),
            Some("components/greeting")
        );
    }

    #[test]
    fn secondary_window_engine_restores_compiled_component_exports() {
        let entry = ModuleId::parse("main").unwrap();
        let button = ModuleId::parse("components/button").unwrap();
        let scripts = EmbeddedScriptSource::new(BTreeMap::from([
            (
                entry.clone(),
                r#"
                    import "components/button" as button;
                    fn view(ctx) {
                        button::Button(#{ text: `Window ${ctx.window_id()}` })
                    }
                "#
                .to_owned(),
            ),
            (
                button,
                include_str!("../../../registry/components/button.rhai").to_owned(),
            ),
        ]));
        let prepared = EmbeddedScriptApp::new(
            entry,
            scripts,
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .prepare()
        .unwrap();

        let (_, lifecycle, _) = prepared.factory.instantiate("settings").unwrap();
        assert!(lifecycle.root().is_some());
    }

    #[test]
    fn startup_rejects_manifest_capabilities_missing_from_host() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(
            directory.path().join("main.rhai"),
            "fn view(ctx) { text(\"never starts\") }",
        )
        .unwrap();
        fs::write(
            directory.path().join("theme.rhai"),
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .unwrap();
        fs::write(
            directory.path().join("app.toml"),
            "entry = \"main\"\nruntime_api = 1\n[capabilities]\n\"app.missing\" = \"*\"\n",
        )
        .unwrap();

        assert!(matches!(
            ScriptApp::new(directory.path().join("main.rhai")).prepare(),
            Err(ScriptAppError::Capability(CapabilityError::Missing(_)))
        ));
    }

    #[test]
    fn embedded_asset_sources_match_file_provider_namespace() {
        let entry = ModuleId::parse("main").unwrap();
        let scripts = EmbeddedScriptSource::new(BTreeMap::from([(
            entry.clone(),
            r#"
                fn state_schema() {
                    #{ fields: #{ image: #{ schema: #{ type: "handle", kind: "image" },
                        "default": #{ type: "handle", value: #{ kind: "image", id: 0 } } } } }
                }
                fn init(ctx) { ctx.set_state("image", ctx.load_image(asset("app/check"))); }
                fn view(ctx) { image(ctx.get_state("image")) }
            "#
            .to_owned(),
        )]));
        let prepared = EmbeddedScriptApp::new(
            entry,
            scripts,
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .asset_sources([(
            "check".to_owned(),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: include_bytes!("../../../registry/assets/icons/check.svg").to_vec(),
            },
        )])
        .prepare()
        .unwrap();
        assert!(matches!(
            prepared.lifecycle.root().unwrap().kind(),
            crate::UiNodeKind::Image { handle } if handle.id() != 0
        ));
    }

    #[test]
    fn file_and_embedded_apps_produce_equivalent_locale_asset_tree() {
        let script = r#"
            fn state_schema() {
                #{ fields: #{ image: #{ schema: #{ type: "handle", kind: "image" },
                    "default": #{ type: "handle", value: #{ kind: "image", id: 0 } } } } }
            }
            fn init(ctx) { ctx.set_state("image", ctx.load_image(asset("app/check"))); }
            fn view(ctx) { row([text(ctx.t("common.loading")), image(ctx.get_state("image"))]) }
        "#;
        let locale = include_str!("../../../registry/locales/en.rhai");
        let theme = include_str!("../../../registry/themes/default_dark.rhai");
        let svg = include_bytes!("../../../registry/assets/icons/check.svg");
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("locales")).unwrap();
        fs::create_dir_all(directory.path().join("assets")).unwrap();
        fs::write(directory.path().join("main.rhai"), script).unwrap();
        fs::write(directory.path().join("theme.rhai"), theme).unwrap();
        fs::write(
            directory.path().join("app.toml"),
            "entry = \"main\"\nruntime_api = 1\n",
        )
        .unwrap();
        fs::write(directory.path().join("locales/en.rhai"), locale).unwrap();
        fs::write(directory.path().join("assets/check.svg"), svg).unwrap();
        let file = ScriptApp::new(directory.path().join("main.rhai"))
            .prepare()
            .unwrap();

        let entry = ModuleId::parse("main").unwrap();
        let embedded = EmbeddedScriptApp::new(
            entry.clone(),
            EmbeddedScriptSource::new(BTreeMap::from([(entry, script.to_owned())])),
            theme,
        )
        .locale_sources([("locales/en.rhai".to_owned(), locale.to_owned())])
        .asset_sources([(
            "check".to_owned(),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: svg.to_vec(),
            },
        )])
        .prepare()
        .unwrap();

        for root in [
            file.lifecycle.root().unwrap(),
            embedded.lifecycle.root().unwrap(),
        ] {
            let crate::UiNodeKind::Container { children } = root.kind() else {
                panic!("equivalence app must render a row");
            };
            assert!(
                matches!(&children[0].kind(), crate::UiNodeKind::Text { text } if text == "Loading")
            );
            assert!(
                matches!(&children[1].kind(), crate::UiNodeKind::Image { handle } if handle.id() == 1)
            );
        }
    }

    #[test]
    fn missing_entry_has_path_aware_error() {
        let path = Path::new("definitely-missing-ui/main.rhai");
        let Err(error) = ScriptApp::new(path).prepare() else {
            panic!("missing script unexpectedly prepared");
        };
        assert!(
            error
                .to_string()
                .contains("definitely-missing-ui/main.rhai")
        );
    }

    #[test]
    fn script_window_api_queues_open_and_installs_close_confirmation_handler() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn state_schema() { #{ fields: #{
                        close_pending: #{ schema: #{ type: "bool" },
                            "default": #{ type: "bool", value: false } }
                    } } }
                    fn close_requested(ctx, payload) {
                        ctx.set_state("close_pending", true);
                    }
                    fn init(ctx) {
                        ctx.set_close_handler(Fn("close_requested"));
                        ctx.open_window("settings", "Settings", 640, 480, true);
                    }
                    fn view(ctx) { text(ctx.window_id()) }
                "#,
            )
            .unwrap();
        let state_schema = engine.root_state_schema(&compiled).unwrap();
        let mut runtime = UiRuntimeState::new();
        runtime.windows.register_open("main").unwrap();
        let runtime = Rc::new(RefCell::new(runtime));
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            ComponentInstancePath::root("App", "main"),
            Some("main".to_owned()),
            BTreeMap::new(),
            &state_schema,
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();

        let close_handler = runtime.borrow().windows.close_handler("main").unwrap();
        let _ = lifecycle
            .invoke_callback_transactional(&engine, &close_handler, UiValue::Null)
            .unwrap();
        assert_eq!(
            runtime
                .borrow()
                .component_state
                .get(&ComponentInstancePath::root("App", "main"), "close_pending"),
            Some(&UiValue::Bool(true))
        );

        let mut runtime = runtime.borrow_mut();
        assert!(runtime.windows.close_handler("main").is_some());
        assert!(matches!(
            runtime.windows.drain_commands().as_slice(),
            [WindowCommand::Open(spec)] if spec.id == "settings" && spec.focus
        ));
    }

    #[test]
    fn script_registered_semantic_action_dispatches_generation_bound_callback() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn state_schema() {
                        #{ fields: #{ count: #{ schema: #{ type: "integer" },
                            "default": #{ type: "integer", value: 0 } } } }
                    }
                    fn increment(ctx, payload) {
                        ctx.set_state("count", ctx.get_state("count") + 1);
                    }
                    fn fire(ctx, payload) { ctx.dispatch_action("counter.increment", ()); }
                    fn init(ctx) { ctx.register_action("counter.increment", Fn("increment")); }
                    fn view(ctx) { text(`${ctx.get_state("count")}`).on_click(Fn("fire")) }
                "#,
            )
            .unwrap();
        let schema = engine.root_state_schema(&compiled).unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let root_path = ComponentInstancePath::root("App", "main");
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            root_path.clone(),
            Some("main".to_owned()),
            BTreeMap::new(),
            &schema,
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        let fire = lifecycle.root().unwrap().handlers()["click"].clone();
        let _ = lifecycle
            .invoke_callback(&engine, &fire, UiValue::Null)
            .unwrap();
        let actions = runtime.borrow_mut().drain_pending_actions();
        for action in actions {
            let _ = lifecycle
                .invoke_callback(&engine, &action.callback, action.payload)
                .unwrap();
        }
        lifecycle.render(&mut engine).unwrap();
        assert_eq!(
            runtime.borrow().component_state.get(&root_path, "count"),
            Some(&UiValue::Integer(1))
        );
    }
}
