//! Mounting an embedded script view in a test window: the `Host` window root, the bundled
//! assets and the `Mount` builder over the official registry.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
use gpui_rhai::{
    AssetData, EmbeddedScriptSource, EmbeddedScriptView, ModuleId, MotionPreference,
    PreparedScriptView, ScriptViewConfig, ScriptViewError, ScriptViewHandle, ScriptViewHost,
    ScriptViewState,
};
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_CHART_SOURCES_BY_ID, BUNDLED_COMPONENT_SOURCES_BY_ID,
    BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_MOTION_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID,
    DEFAULT_THEME, TOKEN_BASE_SOURCE,
};

/// One mounted script view as the window root.
pub(crate) struct Host {
    pub(crate) host: ScriptViewHost,
    pub(crate) view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

/// A window root that draws an empty container once the view is suspended or disposed, for
/// tests that keep drawing the window after that.
pub(crate) struct ActiveHost {
    pub(crate) host: ScriptViewHost,
    pub(crate) view: ScriptViewHandle,
}

impl Render for ActiveHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        if self.view.state() == ScriptViewState::Active {
            self.host.container(self.view.element().unwrap())
        } else {
            self.host.container(gpui::div())
        }
    }
}

/// Every bundled asset keyed as component metadata names it, without the `.svg` suffix.
pub(crate) fn bundled_assets() -> Vec<(String, AssetData)> {
    BUNDLED_ASSET_SOURCES
        .iter()
        .map(|(path, source)| {
            (
                path.strip_suffix(".svg").unwrap_or(path).to_owned(),
                AssetData {
                    mime_type: "image/svg+xml".to_owned(),
                    bytes: source.as_bytes().to_vec(),
                },
            )
        })
        .collect()
}

/// The source of one official module (component, layout, pattern, chart or motion) by its ID.
#[track_caller]
pub(crate) fn registry_source(id: &str) -> &'static str {
    [
        BUNDLED_COMPONENT_SOURCES_BY_ID,
        BUNDLED_LAYOUT_SOURCES_BY_ID,
        BUNDLED_PATTERN_SOURCES_BY_ID,
        BUNDLED_CHART_SOURCES_BY_ID,
        BUNDLED_MOTION_SOURCES_BY_ID,
    ]
    .iter()
    .flat_map(|table| table.iter())
    .find(|(module, _)| *module == id)
    .map(|(_, source)| *source)
    .unwrap_or_else(|| panic!("no official module {id:?}"))
}

type ConfigureView = Box<dyn FnOnce(EmbeddedScriptView) -> EmbeddedScriptView>;
type ConfigureMount = Box<dyn FnOnce(ScriptViewConfig) -> ScriptViewConfig>;

/// An embedded view to mount in a test window.
///
/// [`Mount::new`] starts where `EmbeddedScriptView` starts, plus the default dark theme and the
/// token base: the entry module `main`, no other modules, no locale, no assets and the motion
/// preference from the environment. Each difference a test needs is one call, so a test that
/// mounts without assets or with a custom extension says so where it mounts.
pub(crate) struct Mount {
    modules: BTreeMap<ModuleId, String>,
    theme: String,
    token_base: bool,
    locales: Vec<(String, String)>,
    assets: Vec<(String, AssetData)>,
    motion: Option<MotionPreference>,
    host_id: String,
    view_id: String,
    view: Vec<ConfigureView>,
    config: Vec<ConfigureMount>,
}

impl Mount {
    /// `main` as the entry module `main` (see the type docs for the rest).
    pub(crate) fn new(main: impl Into<String>) -> Self {
        Self {
            modules: BTreeMap::from([(ModuleId::parse("main").unwrap(), main.into())]),
            theme: DEFAULT_THEME.to_owned(),
            token_base: true,
            locales: Vec::new(),
            assets: Vec::new(),
            motion: None,
            host_id: "view".to_owned(),
            view_id: "view".to_owned(),
            view: Vec::new(),
            config: Vec::new(),
        }
    }

    /// `main` over every official component, with the bundled assets: how most component
    /// tests mount.
    pub(crate) fn components(main: impl Into<String>) -> Self {
        Self::new(main)
            .modules(BUNDLED_COMPONENT_SOURCES_BY_ID)
            .bundled_assets()
    }

    /// Adds every module of a registry table, e.g. `BUNDLED_LAYOUT_SOURCES_BY_ID`.
    pub(crate) fn modules(mut self, table: &[(&str, &str)]) -> Self {
        for (id, source) in table {
            self.modules
                .insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
        }
        self
    }

    /// Adds one module with its own source.
    pub(crate) fn module(mut self, id: &str, source: impl Into<String>) -> Self {
        self.modules
            .insert(ModuleId::parse(id).unwrap(), source.into());
        self
    }

    /// Adds one official module by its ID, e.g. `"components/input"`.
    #[track_caller]
    pub(crate) fn registry(self, id: &str) -> Self {
        let source = registry_source(id);
        self.module(id, source)
    }

    /// Replaces the default dark theme.
    pub(crate) fn theme(mut self, source: impl Into<String>) -> Self {
        self.theme = source.into();
        self
    }

    /// Mounts without the token base beneath the theme.
    pub(crate) fn no_token_base(mut self) -> Self {
        self.token_base = false;
        self
    }

    /// Adds a locale file, e.g. `("en.rhai", EN_LOCALE)`.
    pub(crate) fn locale(mut self, name: &str, source: &str) -> Self {
        self.locales.push((name.to_owned(), source.to_owned()));
        self
    }

    /// Installs [`bundled_assets`].
    pub(crate) fn bundled_assets(mut self) -> Self {
        self.assets = bundled_assets();
        self
    }

    /// Sets the motion preference instead of reading it from the environment.
    pub(crate) fn motion(mut self, preference: MotionPreference) -> Self {
        self.motion = Some(preference);
        self
    }

    /// The host and view IDs (both `"view"` unless set).
    pub(crate) fn id(self, id: &str) -> Self {
        self.ids(id, id)
    }

    /// Different host and view IDs.
    pub(crate) fn ids(mut self, host: &str, view: &str) -> Self {
        host.clone_into(&mut self.host_id);
        view.clone_into(&mut self.view_id);
        self
    }

    /// Any other `EmbeddedScriptView` option: extensions, clocks, theme sources, styles.
    pub(crate) fn with(
        mut self,
        configure: impl FnOnce(EmbeddedScriptView) -> EmbeddedScriptView + 'static,
    ) -> Self {
        self.view.push(Box::new(configure));
        self
    }

    /// `ScriptViewConfig` options beyond the view ID, e.g. `window_drag_areas(true)`.
    pub(crate) fn config(
        mut self,
        configure: impl FnOnce(ScriptViewConfig) -> ScriptViewConfig + 'static,
    ) -> Self {
        self.config.push(Box::new(configure));
        self
    }

    fn embedded(&mut self) -> EmbeddedScriptView {
        let mut view = EmbeddedScriptView::new(
            ModuleId::parse("main").unwrap(),
            EmbeddedScriptSource::new(std::mem::take(&mut self.modules)),
            std::mem::take(&mut self.theme),
        );
        if self.token_base {
            view = view.token_base(TOKEN_BASE_SOURCE);
        }
        if !self.locales.is_empty() {
            view = view.locale_sources(std::mem::take(&mut self.locales));
        }
        if !self.assets.is_empty() {
            view = view.asset_sources(std::mem::take(&mut self.assets));
        }
        if let Some(preference) = self.motion {
            view = view.motion_preference(preference);
        }
        for configure in self.view.drain(..) {
            view = configure(view);
        }
        view
    }

    /// Compiles the view without mounting it.
    pub(crate) fn prepare(mut self) -> Result<PreparedScriptView, ScriptViewError> {
        self.embedded().prepare()
    }

    /// Installs the runtime, opens a window with the view as its root and runs until parked.
    pub(crate) fn mount_window(
        self,
        cx: &mut TestAppContext,
    ) -> (WindowHandle<Host>, ScriptViewHandle) {
        self.mount_root(cx, |host, view| Host { host, view })
    }

    /// [`Mount::mount_window`] with another window root, built from the host and the view
    /// (e.g. a root that still draws once the view is suspended).
    pub(crate) fn mount_root<R: Render>(
        mut self,
        cx: &mut TestAppContext,
        root: impl FnOnce(ScriptViewHost, ScriptViewHandle) -> R + 'static,
    ) -> (WindowHandle<R>, ScriptViewHandle) {
        cx.update(gpui_rhai::install);
        let prepared = self.embedded().prepare().unwrap();
        let mut config = ScriptViewConfig::new(&self.view_id);
        for configure in self.config.drain(..) {
            config = configure(config);
        }
        let host_id = self.host_id.clone();
        let mounted = Rc::new(RefCell::new(None));
        let capture = Rc::clone(&mounted);
        let window = cx.add_window(move |window, cx| {
            let host = ScriptViewHost::new(host_id, cx).unwrap();
            let view = prepared.mount(config, host.clone(), window, cx).unwrap();
            *capture.borrow_mut() = Some(view.clone());
            root(host, view)
        });
        cx.run_until_parked();
        let view = mounted.borrow().clone().unwrap();
        (window, view)
    }

    /// [`Mount::mount_window`], then a visual context on the window.
    pub(crate) fn mount(self, cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
        let (window, view) = self.mount_window(cx);
        (VisualTestContext::from_window(*window, cx), view)
    }
}
