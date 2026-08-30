use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

#[cfg(feature = "dev-reload")]
use gpui::KeyBinding;
#[cfg(feature = "dev-reload")]
use gpui::actions;
use gpui::{
    AnyElement, AnyWindowHandle, App, AppContext, Application, Bounds, Context, DispatchPhase,
    Element, ElementId, Entity, FocusHandle, Global, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, MouseDownEvent, ParentElement, Pixels, Render,
    SharedString, Styled, Task, Timer, TitlebarOptions, Window, WindowAppearance, WindowBounds,
    WindowOptions, deferred, div, px, rgba, size,
};
use thiserror::Error;

#[cfg(feature = "dev-reload")]
use crate::FileWatcher;
use crate::overlay_element::WindowOverlayCoordinator;
use crate::{
    ActionError, ActionId, AnimationRuntime, AppManifest, AssetData, AssetId, CapabilityError,
    CompiledUi, ComponentExportError, ComponentInstancePath, ComponentRegistry,
    ComponentStateSchema, DependencyError, DirectoryAssetProvider, DispatchScriptAction,
    EmbeddedScriptSource, FileScriptSource, GpuiNodeRenderer, InMemoryAssetProvider,
    InteractionState, KeyBindingSpec, LocaleBundle, LocaleManager, ModuleCompileCache, ModuleId,
    MotionPreference, NodeEventDispatcher, PrimitiveRegistry, ResponsiveError, ResponsiveRuntime,
    RestrictedModuleResolver, RuntimeEngine, RuntimeError, ScriptCallback, ScriptLifecycle,
    ScriptSource, ScriptWindowSpec, SystemAppearance, TextDirection, ThemeManager, ThemeSelection,
    ThemeVariant, UiRuntimeState, UiValue, ViewportBreakpoints, WindowCommand, WindowCommandPolicy,
    init_text_area, init_text_input, load_locale_source, load_theme_source,
};

#[cfg(feature = "dev-reload")]
actions!(gpui_rhai_devtools, [ToggleInspector]);

const HOST_KEY_CONTEXT: &str = "GPUIRhaiHost";

#[derive(Default)]
struct ScriptRuntimeInstallation {
    bindings: BTreeMap<(String, Option<String>), ActionId>,
}

impl Global for ScriptRuntimeInstallation {}

/// Install GPUI Rhai's application-wide input actions once.
pub fn install(cx: &mut App) {
    if cx.has_global::<ScriptRuntimeInstallation>() {
        return;
    }
    init_text_input(cx);
    init_text_area(cx);
    cx.set_global(ScriptRuntimeInstallation::default());
}

#[derive(Clone, Debug)]
pub struct ScriptViewConfig {
    view_id: String,
    paint_background: bool,
}

impl ScriptViewConfig {
    #[must_use]
    pub fn new(view_id: impl Into<String>) -> Self {
        Self {
            view_id: view_id.into(),
            paint_background: false,
        }
    }

    #[must_use]
    pub const fn paint_background(mut self, paint: bool) -> Self {
        self.paint_background = paint;
        self
    }

    #[must_use]
    pub fn view_id(&self) -> &str {
        &self.view_id
    }
}

#[derive(Clone)]
pub struct ScriptViewHost {
    inner: Rc<RefCell<ScriptViewHostState>>,
}

struct ScriptViewHostState {
    window_id: String,
    overlays: WindowOverlayCoordinator,
    fallback_focus: FocusHandle,
    views: BTreeMap<String, FocusHandle>,
    pending_focus_recovery: Vec<FocusHandle>,
    overlay_viewport: Option<crate::OverlayBounds>,
    toast_max_visible: usize,
    window_policy: WindowCommandPolicy,
    frame_active: bool,
    container_bounds: Option<Bounds<Pixels>>,
}

impl ScriptViewHost {
    /// Create one interaction/overlay domain, normally one per GPUI window.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::InvalidId`] for an unsafe window identifier.
    pub fn new(window_id: impl Into<String>, cx: &mut App) -> Result<Self, ScriptViewError> {
        Self::new_with_policy(window_id, WindowCommandPolicy::Disabled, cx)
    }

    fn new_with_policy(
        window_id: impl Into<String>,
        window_policy: WindowCommandPolicy,
        cx: &mut App,
    ) -> Result<Self, ScriptViewError> {
        install(cx);
        let window_id = window_id.into();
        validate_view_id(&window_id)?;
        Ok(Self {
            inner: Rc::new(RefCell::new(ScriptViewHostState {
                window_id,
                overlays: WindowOverlayCoordinator::default(),
                fallback_focus: cx.focus_handle(),
                views: BTreeMap::new(),
                pending_focus_recovery: Vec::new(),
                overlay_viewport: None,
                toast_max_visible: 3,
                window_policy,
                frame_active: false,
                container_bounds: None,
            })),
        })
    }

    #[must_use]
    pub fn window_id(&self) -> String {
        self.inner.borrow().window_id.clone()
    }

    /// Restrict this Host's overlays to an explicit absolute rectangle.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::Overlay`] for invalid geometry.
    pub fn set_overlay_viewport(
        &self,
        viewport: crate::OverlayBounds,
    ) -> Result<(), ScriptViewError> {
        crate::OverlayManager::new(viewport)?;
        self.inner.borrow_mut().overlay_viewport = Some(viewport);
        Ok(())
    }

    pub fn use_window_overlay_viewport(&self) {
        self.inner.borrow_mut().overlay_viewport = None;
    }

    pub fn set_toast_max_visible(&self, max_visible: usize) {
        let max_visible = max_visible.max(1);
        let mut state = self.inner.borrow_mut();
        state.toast_max_visible = max_visible;
        state.overlays.set_toast_max_visible(max_visible);
    }

    #[must_use]
    pub fn overlay_placement(
        &self,
        view_id: &str,
        local_id: &str,
    ) -> Option<crate::PlacementResult> {
        self.inner
            .borrow()
            .overlays
            .placement(view_id, &crate::OverlayId::new(local_id))
    }

    #[must_use]
    pub fn visible_toast_count(&self, region: crate::ToastRegion) -> usize {
        self.inner.borrow().overlays.toast_visible_count(region)
    }

    /// Bind host-approved script actions once at App scope.
    ///
    /// # Errors
    ///
    /// Returns a conflict when another binding already owns the same keys and
    /// context, or an invalid-key error from GPUI conversion.
    pub fn bind_keys(
        &self,
        bindings: impl IntoIterator<Item = KeyBindingSpec>,
        cx: &mut App,
    ) -> Result<(), ScriptViewError> {
        install(cx);
        let bindings = bindings.into_iter().collect::<Vec<_>>();
        let converted = bindings
            .iter()
            .map(KeyBindingSpec::to_gpui)
            .collect::<Result<Vec<_>, _>>()?;
        let mut new = Vec::new();
        {
            let installed = cx.global_mut::<ScriptRuntimeInstallation>();
            for (index, binding) in bindings.iter().enumerate() {
                let key = (binding.keystrokes.clone(), binding.context.clone());
                if let Some(existing) = installed.bindings.get(&key)
                    && existing != &binding.action
                {
                    return Err(ScriptViewError::KeyBindingConflict {
                        keystrokes: binding.keystrokes.clone(),
                        context: binding.context.clone(),
                        existing: existing.clone(),
                        requested: binding.action.clone(),
                    });
                }
                if let std::collections::btree_map::Entry::Vacant(entry) =
                    installed.bindings.entry(key)
                {
                    entry.insert(binding.action.clone());
                    new.push(index);
                }
            }
        }
        cx.bind_keys(new.into_iter().map(|index| converted[index].clone()));
        Ok(())
    }

    #[must_use]
    pub fn container(&self, child: impl IntoElement) -> AnyElement {
        let (fallback, overlays) = {
            let state = self.inner.borrow();
            (state.fallback_focus.clone(), state.overlays.clone())
        };
        let escape_overlays = overlays.clone();
        let child = div()
            .size_full()
            .track_focus(&fallback)
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key.as_str() == "escape"
                    && escape_overlays.dismiss_escape(window, cx)
                {
                    cx.stop_propagation();
                }
            })
            .child(child)
            .child(SharedToastPortalElement {
                coordinator: overlays,
            })
            .into_any_element();
        ScriptViewHostFrame {
            host: self.clone(),
            child: Some(child),
        }
        .into_any_element()
    }

    fn reserve_view(&self, view_id: &str) -> Result<(), ScriptViewError> {
        validate_view_id(view_id)?;
        let mut state = self.inner.borrow_mut();
        if state.views.contains_key(view_id) {
            return Err(ScriptViewError::DuplicateView(view_id.to_owned()));
        }
        let fallback = state.fallback_focus.clone();
        state.views.insert(view_id.to_owned(), fallback);
        Ok(())
    }

    fn attach_view_focus(&self, view_id: &str, focus: FocusHandle) {
        if let Some(entry) = self.inner.borrow_mut().views.get_mut(view_id) {
            *entry = focus;
        }
    }

    fn unregister_view(&self, view_id: &str) {
        let mut state = self.inner.borrow_mut();
        if let Some(focus) = state.views.remove(view_id) {
            state.pending_focus_recovery.push(focus);
        }
        state.overlays.remove_view(view_id);
    }

    fn overlays(&self) -> WindowOverlayCoordinator {
        self.inner.borrow().overlays.clone()
    }

    fn window_policy(&self) -> WindowCommandPolicy {
        self.inner.borrow().window_policy
    }

    fn frame_active(&self) -> bool {
        self.inner.borrow().frame_active
    }
}

fn validate_view_id(id: &str) -> Result<(), ScriptViewError> {
    let valid = (1..=64).contains(&id.len())
        && id
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'));
    if valid {
        Ok(())
    } else {
        Err(ScriptViewError::InvalidId(id.to_owned()))
    }
}

struct ScriptViewHostFrame {
    host: ScriptViewHost,
    child: Option<AnyElement>,
}

impl Element for ScriptViewHostFrame {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let (overlays, viewport, toast_max_visible, fallback, pending) = {
            let mut state = self.host.inner.borrow_mut();
            state.frame_active = true;
            let viewport = state
                .overlay_viewport
                .unwrap_or_else(|| crate::OverlayBounds {
                    x: 0.0,
                    y: 0.0,
                    width: f64::from(window.viewport_size().width),
                    height: f64::from(window.viewport_size().height),
                });
            (
                state.overlays.clone(),
                viewport,
                state.toast_max_visible,
                state.fallback_focus.clone(),
                std::mem::take(&mut state.pending_focus_recovery),
            )
        };
        overlays.begin_host_frame(viewport);
        overlays.set_toast_max_visible(toast_max_visible);
        if pending
            .iter()
            .any(|focus| focus.contains_focused(window, cx))
        {
            fallback.focus(window);
        }
        let mut child = self.child.take().expect("host frame lays out once");
        let layout = child.request_layout(window, cx);
        (layout, child)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.host.inner.borrow_mut().container_bounds = Some(bounds);
        child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
        self.host.inner.borrow_mut().frame_active = false;
        let overlays = self.host.overlays();
        let container_bounds = self.host.inner.borrow().container_bounds;
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase == DispatchPhase::Capture
                && container_bounds.is_some_and(|bounds| bounds.contains(&event.position))
            {
                let _ = overlays.dismiss_outside(event.position, window, cx);
            }
        });
    }
}

impl IntoElement for ScriptViewHostFrame {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct SharedToastPortalElement {
    coordinator: WindowOverlayCoordinator,
}

impl Element for SharedToastPortalElement {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let viewport = self.coordinator.viewport_or_window(window.viewport_size());
        let columns =
            self.coordinator
                .take_toast_elements()
                .into_iter()
                .map(|(region, elements)| {
                    let column = div()
                        .absolute()
                        .w(px(320.0))
                        .flex()
                        .flex_col()
                        .gap_2()
                        .children(elements);
                    match region {
                        crate::ToastRegion::TopLeft => column.top(px(12.0)).left(px(12.0)),
                        crate::ToastRegion::TopRight => column.top(px(12.0)).right(px(12.0)),
                        crate::ToastRegion::BottomLeft => column.bottom(px(12.0)).left(px(12.0)),
                        crate::ToastRegion::BottomRight => column.bottom(px(12.0)).right(px(12.0)),
                    }
                });
        let mut layer = deferred(
            div()
                .absolute()
                .left(pixel_from_f64(viewport.x))
                .top(pixel_from_f64(viewport.y))
                .w(pixel_from_f64(viewport.width))
                .h(pixel_from_f64(viewport.height))
                .children(columns),
        )
        .with_priority(9_000)
        .into_any_element();
        let layout = layer.request_layout(window, cx);
        (layout, layer)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        layer: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        layer.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        layer: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        layer.paint(window, cx);
    }
}

impl IntoElement for SharedToastPortalElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[derive(Clone)]
pub struct ScriptViewHandle(Rc<ScriptViewHandleInner>);

struct ScriptViewHandleInner {
    entity: Entity<ScriptHostView>,
    host: ScriptViewHost,
    view_id: String,
    disposed: Cell<bool>,
    measured_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl Drop for ScriptViewHandleInner {
    fn drop(&mut self) {
        if !self.disposed.replace(true) {
            self.host.unregister_view(&self.view_id);
        }
    }
}

impl ScriptViewHandle {
    #[must_use]
    pub fn view_id(&self) -> &str {
        &self.0.view_id
    }

    /// Return the latest rendered declarative root.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after disposal.
    pub fn root(&self, cx: &App) -> Result<Option<crate::UiNode>, ScriptViewError> {
        if self.0.disposed.get() {
            return Err(ScriptViewError::DisposedView(self.0.view_id.clone()));
        }
        Ok(self.0.entity.read(cx).lifecycle.root().cloned())
    }

    /// Return the measured embedding element.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after explicit disposal.
    pub fn element(&self) -> Result<AnyElement, ScriptViewError> {
        if self.0.disposed.get() {
            return Err(ScriptViewError::DisposedView(self.0.view_id.clone()));
        }
        Ok(MeasuredScriptViewElement {
            entity: self.0.entity.clone(),
            measured_bounds: Rc::clone(&self.0.measured_bounds),
        }
        .into_any_element())
    }

    /// Dispose this view immediately. The operation is idempotent.
    ///
    /// # Errors
    ///
    /// Returns an entity update error only if GPUI has already released the
    /// underlying view unexpectedly.
    pub fn dispose(&self, cx: &mut App) -> Result<(), ScriptViewError> {
        if self.0.disposed.replace(true) {
            return Ok(());
        }
        self.0.entity.update(cx, |view, cx| {
            view.release_view();
            cx.notify();
        });
        self.0.host.unregister_view(&self.0.view_id);
        cx.refresh_windows();
        Ok(())
    }

    /// Focus this view's stable root handle.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after disposal.
    pub fn focus(&self, window: &mut Window, cx: &App) -> Result<(), ScriptViewError> {
        if self.0.disposed.get() {
            return Err(ScriptViewError::DisposedView(self.0.view_id.clone()));
        }
        self.0.entity.read(cx).host_focus.focus(window);
        Ok(())
    }

    #[cfg(feature = "dev-reload")]
    /// Open or close this view's isolated development inspector.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after disposal.
    pub fn set_inspector_open(&self, open: bool, cx: &mut App) -> Result<(), ScriptViewError> {
        if self.0.disposed.get() {
            return Err(ScriptViewError::DisposedView(self.0.view_id.clone()));
        }
        self.0.entity.update(cx, |view, cx| {
            view.inspector_open = open;
            cx.notify();
        });
        Ok(())
    }
}

struct MeasuredScriptViewElement {
    entity: Entity<ScriptHostView>,
    measured_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl Element for MeasuredScriptViewElement {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut child = self.entity.clone().into_any_element();
        let layout = child.request_layout(window, cx);
        (layout, child)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.prepaint(window, cx);
        if self.measured_bounds.get() != Some(bounds) {
            self.measured_bounds.set(Some(bounds));
            let view = self.entity.downgrade();
            window.defer(cx, move |_, cx| {
                let _ = view.update(cx, |view, cx| view.set_content_bounds(bounds, cx));
            });
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
    }
}

impl IntoElement for MeasuredScriptViewElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

pub trait ScriptViewExtension {
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

pub struct FileScriptView {
    entry: PathBuf,
    development: bool,
    motion_preference: MotionPreference,
    extensions: Vec<Box<dyn ScriptViewExtension>>,
    key_bindings: Vec<KeyBindingSpec>,
    viewport_breakpoints: ViewportBreakpoints,
    calendar_clock: crate::CalendarClock,
}

impl FileScriptView {
    #[must_use]
    pub fn new(entry: impl Into<PathBuf>) -> Self {
        Self {
            entry: entry.into(),
            development: cfg!(feature = "dev-reload") && cfg!(debug_assertions),
            motion_preference: motion_preference_from_env(),
            extensions: Vec::new(),
            key_bindings: Vec::new(),
            viewport_breakpoints: ViewportBreakpoints::default(),
            calendar_clock: crate::CalendarClock::default(),
        }
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
    pub fn extension(mut self, extension: impl ScriptViewExtension + 'static) -> Self {
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

    #[must_use]
    pub fn calendar_clock(mut self, clock: crate::CalendarClock) -> Self {
        self.calendar_clock = clock;
        self
    }

    /// Read, compile, initialize, and render the app before opening GPUI.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError`] for source I/O, compilation, lifecycle, or
    /// initial rendering failures.
    pub fn prepare(self) -> Result<PreparedScriptView, ScriptViewError> {
        let source = fs::read_to_string(&self.entry).map_err(|source| ScriptViewError::Io {
            path: self.entry.clone(),
            source,
        })?;
        let extensions = Rc::new(self.extensions);
        let mut engine = RuntimeEngine::new();
        for extension in extensions.iter() {
            extension
                .configure_engine(&mut engine)
                .map_err(ScriptViewError::Extension)?;
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
        runtime_state.calendar_clock = self.calendar_clock;
        runtime_state.locale = load_locale_directory(engine.engine(), &ui_root.join("locales"))?;
        runtime_state.theme = Some(load_theme_directory(
            engine.engine(),
            &ui_root.join("themes"),
            &theme,
        )?);
        register_file_assets(&runtime_state, &ui_root)?;
        preload_component_assets(&runtime_state, &component_exports)?;
        for extension in extensions.iter() {
            extension
                .configure_runtime(&mut runtime_state)
                .map_err(ScriptViewError::Extension)?;
        }
        manifest.activate(&mut runtime_state.capabilities)?;
        let runtime = Rc::new(RefCell::new(runtime_state));
        let program = WindowProgram {
            compiled: compiled.clone(),
            state_schema: state_schema.clone(),
            component_exports,
        };
        let factory = Rc::new(ScriptWindowFactory {
            program: RefCell::new(program),
            runtime,
            extensions,
            theme: theme.clone(),
            #[cfg(feature = "dev-reload")]
            development: self.development,
        });
        Ok(PreparedScriptView {
            engine,
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
}

pub struct EmbeddedScriptView {
    entry: ModuleId,
    scripts: EmbeddedScriptSource,
    theme_source: String,
    locales: Vec<(String, String)>,
    themes: Vec<(String, String)>,
    development: bool,
    motion_preference: MotionPreference,
    extensions: Vec<Box<dyn ScriptViewExtension>>,
    manifest: AppManifest,
    key_bindings: Vec<KeyBindingSpec>,
    assets: BTreeMap<String, AssetData>,
    viewport_breakpoints: ViewportBreakpoints,
    calendar_clock: crate::CalendarClock,
}

impl EmbeddedScriptView {
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
            locales: Vec::new(),
            themes: Vec::new(),
            development: false,
            motion_preference: motion_preference_from_env(),
            extensions: Vec::new(),
            manifest,
            key_bindings: Vec::new(),
            assets: BTreeMap::new(),
            viewport_breakpoints: ViewportBreakpoints::default(),
            calendar_clock: crate::CalendarClock::default(),
        }
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
    pub fn extension(mut self, extension: impl ScriptViewExtension + 'static) -> Self {
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

    #[must_use]
    pub fn calendar_clock(mut self, clock: crate::CalendarClock) -> Self {
        self.calendar_clock = clock;
        self
    }

    /// Compile and initialize a fully embedded application.
    ///
    /// # Errors
    ///
    /// Returns source, theme, compile, or lifecycle errors.
    pub fn prepare(self) -> Result<PreparedScriptView, ScriptViewError> {
        let entry = self.scripts.load(&self.entry)?;
        validate_manifest_entry(&self.manifest, &self.entry)?;
        let extensions = Rc::new(self.extensions);
        let mut engine = RuntimeEngine::new();
        for extension in extensions.iter() {
            extension
                .configure_engine(&mut engine)
                .map_err(ScriptViewError::Extension)?;
        }
        let theme = load_theme_source(engine.engine(), "<embedded-theme>", &self.theme_source)
            .map_err(|error| ScriptViewError::Theme(error.to_string()))?;
        engine.set_module_resolver(RestrictedModuleResolver::from_source(&self.scripts)?);
        let compiled = engine.compile_self_contained_named(self.entry.as_str(), &entry.source)?;
        let component_exports = engine.component_exports()?;
        self.manifest.validate_components(&component_exports)?;
        let state_schema = engine.root_state_schema(&compiled)?;
        let mut runtime_state = UiRuntimeState::new();
        runtime_state.animations = AnimationRuntime::new(self.motion_preference);
        runtime_state.responsive = ResponsiveRuntime::new(self.viewport_breakpoints);
        runtime_state.calendar_clock = self.calendar_clock;
        runtime_state.locale = load_embedded_locales(engine.engine(), self.locales)?;
        runtime_state.theme = Some(load_embedded_themes(engine.engine(), self.themes, &theme)?);
        if !self.assets.is_empty() {
            runtime_state
                .assets
                .register("app", InMemoryAssetProvider::new(self.assets))?;
        }
        preload_component_assets(&runtime_state, &component_exports)?;
        for extension in extensions.iter() {
            extension
                .configure_runtime(&mut runtime_state)
                .map_err(ScriptViewError::Extension)?;
        }
        self.manifest.activate(&mut runtime_state.capabilities)?;
        let runtime = Rc::new(RefCell::new(runtime_state));
        let program = WindowProgram {
            compiled: compiled.clone(),
            state_schema: state_schema.clone(),
            component_exports,
        };
        let factory = Rc::new(ScriptWindowFactory {
            program: RefCell::new(program),
            runtime,
            extensions,
            theme: theme.clone(),
            #[cfg(feature = "dev-reload")]
            development: self.development,
        });
        Ok(PreparedScriptView {
            engine,
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
}

fn load_embedded_locales(
    engine: &rhai::Engine,
    sources: Vec<(String, String)>,
) -> Result<Option<LocaleManager>, ScriptViewError> {
    let mut bundles = sources
        .into_iter()
        .map(|(name, source)| {
            load_locale_source(engine, &name, &source)
                .map_err(|error| ScriptViewError::Locale(error.to_string()))
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
            .map_err(|error| ScriptViewError::Locale(error.to_string()))?,
    ))
}

fn load_embedded_themes(
    engine: &rhai::Engine,
    sources: Vec<(String, String)>,
    primary: &ThemeVariant,
) -> Result<ThemeManager, ScriptViewError> {
    let mut variants = BTreeMap::from([(
        (primary.family.clone(), primary.name.clone()),
        primary.clone(),
    )]);
    for (name, source) in sources {
        let variant = load_theme_source(engine, &name, &source)
            .map_err(|error| ScriptViewError::Theme(error.to_string()))?;
        insert_theme_variant(&mut variants, variant)?;
    }
    theme_manager(variants.into_values(), primary)
}

fn module_id_from_path(root: &Path, path: &Path) -> Result<ModuleId, ScriptViewError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| ScriptViewError::ModulePath(path.to_path_buf()))?;
    let id = relative
        .with_extension("")
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    Ok(ModuleId::parse(id)?)
}

fn load_file_manifest(root: &Path, entry: &Path) -> Result<AppManifest, ScriptViewError> {
    let path = root.join("app.toml");
    let source = fs::read_to_string(&path).map_err(|source| ScriptViewError::Io {
        path: path.clone(),
        source,
    })?;
    let manifest: AppManifest =
        toml::from_str(&source).map_err(|error| ScriptViewError::Manifest(error.to_string()))?;
    let entry = module_id_from_path(root, entry)?;
    validate_manifest_entry(&manifest, &entry)?;
    Ok(manifest)
}

fn load_primary_file_theme(
    engine: &RuntimeEngine,
    entry: &Path,
) -> Result<(PathBuf, PathBuf, ThemeVariant), ScriptViewError> {
    let root = entry
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let path = root.join("theme.rhai");
    let source = fs::read_to_string(&path).map_err(|source| ScriptViewError::Io {
        path: path.clone(),
        source,
    })?;
    let theme = load_theme_source(engine.engine(), &path.to_string_lossy(), &source)
        .map_err(|error| ScriptViewError::Theme(error.to_string()))?;
    Ok((root, path, theme))
}

fn configure_file_modules(
    engine: &mut RuntimeEngine,
    root: &Path,
    entry: &Path,
    theme: &Path,
) -> Result<ModuleCompileCache, ScriptViewError> {
    let modules = discover_modules(root, entry, theme)?;
    let source = FileScriptSource::new(root, modules)?;
    let mut cache = ModuleCompileCache::new();
    cache.refresh(engine.engine(), &source, source.module_ids())?;
    engine.set_module_resolver(RestrictedModuleResolver::from_source_with_cache(
        &source, &cache,
    )?);
    Ok(cache)
}

fn register_file_assets(runtime: &UiRuntimeState, root: &Path) -> Result<(), ScriptViewError> {
    let asset_root = root.join("assets");
    if asset_root.exists() {
        runtime
            .assets
            .register("app", DirectoryAssetProvider::new(&asset_root)?)?;
    }
    Ok(())
}

fn preload_component_assets(
    runtime: &UiRuntimeState,
    components: &ComponentRegistry,
) -> Result<(), ScriptViewError> {
    let ids = components
        .iter()
        .flat_map(|(_, component)| component.metadata.assets.iter())
        .map(|asset| {
            let logical = Path::new(asset).with_extension("");
            let logical = logical
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            AssetId::parse(format!("app/{logical}")).map_err(ScriptViewError::Asset)
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    runtime.assets.preload_images(ids)?;
    Ok(())
}

fn validate_manifest_entry(
    manifest: &AppManifest,
    entry: &ModuleId,
) -> Result<(), ScriptViewError> {
    if &manifest.entry == entry {
        Ok(())
    } else {
        Err(ScriptViewError::ManifestEntry {
            manifest: manifest.entry.clone(),
            host: entry.clone(),
        })
    }
}

fn discover_modules(
    root: &Path,
    entry: &Path,
    theme: &Path,
) -> Result<Vec<ModuleId>, ScriptViewError> {
    let mut pending = vec![root.to_path_buf()];
    let mut modules = Vec::new();
    while let Some(directory) = pending.pop() {
        for item in fs::read_dir(&directory).map_err(|source| ScriptViewError::Io {
            path: directory.clone(),
            source,
        })? {
            let item = item.map_err(|source| ScriptViewError::Io {
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
) -> Result<Option<LocaleManager>, ScriptViewError> {
    if !directory.exists() {
        return Ok(None);
    }
    let mut paths = fs::read_dir(directory)
        .map_err(|source| ScriptViewError::Io {
            path: directory.to_path_buf(),
            source,
        })?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|source| ScriptViewError::Io {
                    path: directory.to_path_buf(),
                    source,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.extension().and_then(|extension| extension.to_str()) == Some("rhai"));
    paths.sort();
    let mut bundles = Vec::<LocaleBundle>::new();
    for path in paths {
        let source = fs::read_to_string(&path).map_err(|source| ScriptViewError::Io {
            path: path.clone(),
            source,
        })?;
        bundles.push(
            load_locale_source(engine, &path.to_string_lossy(), &source)
                .map_err(|error| ScriptViewError::Locale(error.to_string()))?,
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
            .map_err(|error| ScriptViewError::Locale(error.to_string()))?,
    ))
}

fn load_theme_directory(
    engine: &rhai::Engine,
    directory: &Path,
    primary: &ThemeVariant,
) -> Result<ThemeManager, ScriptViewError> {
    let mut variants = BTreeMap::from([(
        (primary.family.clone(), primary.name.clone()),
        primary.clone(),
    )]);
    if directory.exists() {
        let mut paths = fs::read_dir(directory)
            .map_err(|source| ScriptViewError::Io {
                path: directory.to_path_buf(),
                source,
            })?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|source| ScriptViewError::Io {
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
            let source = fs::read_to_string(&path).map_err(|source| ScriptViewError::Io {
                path: path.clone(),
                source,
            })?;
            let variant = load_theme_source(engine, &path.to_string_lossy(), &source)
                .map_err(|error| ScriptViewError::Theme(error.to_string()))?;
            insert_theme_variant(&mut variants, variant)?;
        }
    }
    theme_manager(variants.into_values(), primary)
}

fn insert_theme_variant(
    variants: &mut BTreeMap<(String, String), ThemeVariant>,
    variant: ThemeVariant,
) -> Result<(), ScriptViewError> {
    let key = (variant.family.clone(), variant.name.clone());
    if let Some(existing) = variants.get(&key) {
        if existing == &variant {
            return Ok(());
        }
        return Err(ScriptViewError::Theme(format!(
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
) -> Result<ThemeManager, ScriptViewError> {
    ThemeManager::from_variants(
        variants,
        ThemeSelection::new(primary.family.clone(), primary.name.clone()),
    )
    .map_err(|error| ScriptViewError::Theme(error.to_string()))
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
    extensions: Rc<Vec<Box<dyn ScriptViewExtension>>>,
    theme: ThemeVariant,
    #[cfg(feature = "dev-reload")]
    development: bool,
}

impl ScriptWindowFactory {
    fn instantiate(
        &self,
        view_id: &str,
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
        let lifecycle = self.mount_lifecycle(
            &mut engine,
            program,
            view_id,
            window_id,
            WindowCommandPolicy::ApplicationOwned,
            false,
        )?;
        let primitives = engine.primitive_registry();
        Ok((engine, lifecycle, primitives))
    }

    fn mount_lifecycle(
        &self,
        engine: &mut RuntimeEngine,
        program: WindowProgram,
        view_id: &str,
        window_id: &str,
        policy: WindowCommandPolicy,
        register_window: bool,
    ) -> Result<ScriptLifecycle, String> {
        {
            let mut runtime = self.runtime.borrow_mut();
            if register_window {
                runtime
                    .windows
                    .register_open_for_view(window_id, policy, view_id)
                    .map_err(|error| error.to_string())?;
            }
            for extension in self.extensions.iter() {
                extension.configure_window(window_id, &mut runtime)?;
            }
        }
        let mut lifecycle = ScriptLifecycle::new(
            program.compiled,
            Rc::clone(&self.runtime),
            ComponentInstancePath::root("View", view_id),
            Some(window_id.to_owned()),
            BTreeMap::new(),
            &program.state_schema,
        )
        .map_err(|error| error.to_string())?
        .with_view_id(view_id);
        lifecycle.start(engine).map_err(|error| error.to_string())?;
        Ok(lifecycle)
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

pub struct PreparedScriptView {
    engine: RuntimeEngine,
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

impl PreparedScriptView {
    #[must_use]
    pub fn key_bindings(&self) -> &[KeyBindingSpec] {
        &self.key_bindings
    }

    /// Mount one isolated script view into an existing GPUI window.
    ///
    /// # Errors
    ///
    /// Returns identity, lifecycle, extension, or watcher errors.
    pub fn mount(
        self,
        config: ScriptViewConfig,
        host: ScriptViewHost,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<ScriptViewHandle, ScriptViewError> {
        self.mount_with_registry(
            config,
            host,
            Rc::new(RefCell::new(NativeWindowRegistry::default())),
            window,
            cx,
        )
    }

    fn mount_with_registry(
        mut self,
        config: ScriptViewConfig,
        host: ScriptViewHost,
        native_windows: Rc<RefCell<NativeWindowRegistry>>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Result<ScriptViewHandle, ScriptViewError> {
        install(cx);
        #[cfg(feature = "dev-reload")]
        let watcher = if self.development && !self.ui_root.as_os_str().is_empty() {
            Some(FileWatcher::new(&self.ui_root)?)
        } else {
            None
        };
        host.reserve_view(&config.view_id)?;
        let window_id = host.window_id();
        let program = self.factory.program();
        let lifecycle = self
            .factory
            .mount_lifecycle(
                &mut self.engine,
                program,
                &config.view_id,
                &window_id,
                host.window_policy(),
                true,
            )
            .map_err(ScriptViewError::Extension);
        let lifecycle = match lifecycle {
            Ok(lifecycle) => lifecycle,
            Err(error) => {
                host.unregister_view(&config.view_id);
                return Err(error);
            }
        };
        #[cfg(not(feature = "dev-reload"))]
        let _ = (
            &self.entry,
            &self.ui_root,
            &self.theme_path,
            self.development,
        );
        let primitives = self.engine.primitive_registry();
        let timings = self.engine.take_timings();
        let factory = Rc::clone(&self.factory);
        let overlays = host.overlays();
        let view_id = config.view_id.clone();
        let view_host = host.clone();
        let entity = cx.new(|entity_cx| {
            let async_task = spawn_host_poll(entity_cx);
            let host_focus = entity_cx.focus_handle();
            #[cfg(feature = "dev-reload")]
            let reload_task = watcher.as_ref().map(|_| spawn_host_reload_poll(entity_cx));
            ScriptHostView {
                view_id: view_id.clone(),
                window_id,
                engine: self.engine,
                lifecycle,
                primitives,
                last_error: None,
                theme: self.theme,
                #[cfg(feature = "dev-reload")]
                development: self.development,
                #[cfg(feature = "dev-reload")]
                inspector_open: false,
                timings,
                overlays,
                host: view_host,
                paint_background: config.paint_background,
                content_bounds: None,
                factory,
                native_windows,
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
        let focus = entity.read(cx).host_focus.clone();
        host.attach_view_focus(&config.view_id, focus);
        Ok(ScriptViewHandle(Rc::new(ScriptViewHandleInner {
            entity,
            host,
            view_id: config.view_id,
            disposed: Cell::new(false),
            measured_bounds: Rc::new(Cell::new(None)),
        })))
    }
}

pub struct ScriptApplication {
    prepared: PreparedScriptView,
    window_size: (f32, f32),
}

impl ScriptApplication {
    #[must_use]
    pub const fn new(prepared: PreparedScriptView) -> Self {
        Self {
            prepared,
            window_size: (720.0, 480.0),
        }
    }

    #[must_use]
    pub const fn window_size(mut self, width: f32, height: f32) -> Self {
        self.window_size = (width, height);
        self
    }

    /// Run this prepared view as a standalone GPUI application.
    ///
    /// # Errors
    ///
    /// Returns key-binding, mount, or native-window errors.
    pub fn run(self) -> Result<(), ScriptViewError> {
        let error = Rc::new(RefCell::new(None));
        let reported_error = Rc::clone(&error);
        let native_windows = Rc::new(RefCell::new(NativeWindowRegistry::default()));
        let window_size = self.window_size;
        let prepared = self.prepared;
        Application::new().run(move |cx: &mut App| {
            install(cx);
            let host = match ScriptViewHost::new_with_policy(
                "main",
                WindowCommandPolicy::ApplicationOwned,
                cx,
            ) {
                Ok(host) => host,
                Err(error) => {
                    *reported_error.borrow_mut() = Some(error.to_string());
                    cx.quit();
                    return;
                }
            };
            if let Err(error) = host.bind_keys(prepared.key_bindings.clone(), cx) {
                *reported_error.borrow_mut() = Some(error.to_string());
                cx.quit();
                return;
            }
            #[cfg(feature = "dev-reload")]
            cx.bind_keys([
                KeyBinding::new("cmd-alt-i", ToggleInspector, Some(HOST_KEY_CONTEXT)),
                KeyBinding::new("f12", ToggleInspector, Some(HOST_KEY_CONTEXT)),
            ]);
            let bounds = Bounds::centered(None, size(px(window_size.0), px(window_size.1)), cx);
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            let view_native_windows = Rc::clone(&native_windows);
            let mount_error = Rc::clone(&reported_error);
            let result = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..WindowOptions::default()
                },
                move |window, cx| {
                    let root = cx.new(|_| ScriptApplicationRoot {
                        host: host.clone(),
                        view: None,
                        error: None,
                    });
                    let weak_root = root.downgrade();
                    window.defer(cx, move |window, cx| {
                        let view = prepared.mount_with_registry(
                            ScriptViewConfig::new("main").paint_background(true),
                            host,
                            Rc::clone(&view_native_windows),
                            window,
                            cx,
                        );
                        match view {
                            Ok(view) => {
                                let _ = view.focus(window, cx);
                                install_close_interceptor(window, cx, &view.0.entity);
                                let _ = weak_root.update(cx, |root, cx| {
                                    root.view = Some(view);
                                    cx.notify();
                                });
                            }
                            Err(error) => {
                                let message = error.to_string();
                                *mount_error.borrow_mut() = Some(message.clone());
                                let _ = weak_root.update(cx, |root, cx| {
                                    root.error = Some(message);
                                    cx.notify();
                                });
                                cx.defer(|cx| cx.quit());
                            }
                        }
                    });
                    root
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
                Err(error) => {
                    *reported_error.borrow_mut() = Some(error.to_string());
                    cx.quit();
                }
            }
        });
        match error.borrow_mut().take() {
            Some(message) => Err(ScriptViewError::Window(message)),
            None => Ok(()),
        }
    }
}

struct ScriptApplicationRoot {
    host: ScriptViewHost,
    view: Option<ScriptViewHandle>,
    error: Option<String>,
}

impl Render for ScriptApplicationRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.as_ref().map_or_else(
            || {
                self.error.as_ref().map_or_else(
                    || div().child("Loading…").into_any_element(),
                    |error| div().child(error.clone()).into_any_element(),
                )
            },
            |view| {
                view.element()
                    .unwrap_or_else(|error| div().child(error.to_string()).into_any_element())
            },
        ))
    }
}

#[allow(clippy::too_many_lines)]
fn open_secondary_window(
    spec: &ScriptWindowSpec,
    factory: &Rc<ScriptWindowFactory>,
    native_windows: &Rc<RefCell<NativeWindowRegistry>>,
    cx: &mut App,
) -> Result<(), String> {
    let factory = Rc::clone(factory);
    let native_windows = Rc::clone(native_windows);
    let window_id = spec.id.clone();
    let host = ScriptViewHost::new_with_policy(
        window_id.clone(),
        WindowCommandPolicy::ApplicationOwned,
        cx,
    )
    .map_err(|error| error.to_string())?;
    host.reserve_view(&window_id)
        .map_err(|error| error.to_string())?;
    let (engine, lifecycle, primitives) = match factory.instantiate(&window_id, &window_id) {
        Ok(instance) => instance,
        Err(error) => {
            host.unregister_view(&window_id);
            let root = ComponentInstancePath::root("View", &window_id);
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
    let view_host = host.clone();
    let result = cx.open_window(options, move |window, cx| {
        let entity = cx.new(|entity_cx| {
            let async_task = spawn_host_poll(entity_cx);
            let host_focus = entity_cx.focus_handle();
            ScriptHostView {
                view_id: view_window_id.clone(),
                window_id: view_window_id.clone(),
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
                overlays: view_host.overlays(),
                host: view_host.clone(),
                paint_background: true,
                content_bounds: None,
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
        view_host.attach_view_focus(&view_window_id, entity.read(cx).host_focus.clone());
        let view = ScriptViewHandle(Rc::new(ScriptViewHandleInner {
            entity: entity.clone(),
            host: view_host.clone(),
            view_id: view_window_id,
            disposed: Cell::new(false),
            measured_bounds: Rc::new(Cell::new(None)),
        }));
        let _ = view.focus(window, cx);
        install_close_interceptor(window, cx, &entity);
        cx.new(|_| ScriptApplicationRoot {
            host: view_host,
            view: Some(view),
            error: None,
        })
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
            host.unregister_view(&window_id);
            let root = ComponentInstancePath::root("View", &window_id);
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

fn pixel_from_f64(value: f64) -> Pixels {
    px(value.to_string().parse::<f32>().unwrap_or(f32::MAX))
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

#[allow(clippy::struct_excessive_bools)]
struct ScriptHostView {
    view_id: String,
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
    host: ScriptViewHost,
    paint_background: bool,
    content_bounds: Option<Bounds<Pixels>>,
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

fn build_host_root(
    host_focus: &FocusHandle,
    theme: &ThemeVariant,
    paint_background: bool,
) -> gpui::Stateful<gpui::Div> {
    let root = div()
        .id("gpui-rhai-host")
        .size_full()
        .track_focus(host_focus)
        .text_color(rgba(theme.tokens.colors["text_primary"].as_rgba_hex()))
        .key_context(HOST_KEY_CONTEXT)
        .on_key_down(handle_tab_navigation);
    if paint_background {
        root.bg(rgba(theme.tokens.colors["surface"].as_rgba_hex()))
    } else {
        root
    }
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
            let root = self.lifecycle.root_path().clone();
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
        let animation_root = format!("window:{}/view:{}/root", self.window_id, self.view_id);
        let render_resources = crate::renderer::WindowRenderResources {
            assets: &assets,
            dispatcher: &dispatcher,
            overlays: &self.overlays,
            animations: &animations,
            direction,
            root_path: &animation_root,
            view_id: &self.view_id,
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
        let root = build_host_root(&self.host_focus, &theme, self.paint_background)
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
    fn set_content_bounds(&mut self, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        if self.content_bounds != Some(bounds) {
            self.content_bounds = Some(bounds);
            cx.notify();
        }
    }

    fn prepare_render(&mut self, window: &Window) {
        if !self.host.frame_active() {
            self.last_error = Some(
                "embedded ScriptView must be rendered inside ScriptViewHost::container".to_owned(),
            );
        }
        self.sync_viewport_class(window);
        debug_assert!(self.engine.is_current(self.lifecycle.generation()));
        self.reconcile_primitive_lifecycle();
    }

    fn reconcile_primitive_lifecycle(&mut self) {
        if let Some(root) = self.lifecycle.root()
            && let Err(error) = self.primitives.retain_tree(root)
        {
            self.last_error = Some(error.to_string());
        }
    }

    fn sync_viewport_class(&mut self, window: &Window) {
        let width = self.content_bounds.map_or_else(
            || f64::from(window.viewport_size().width),
            |bounds| f64::from(bounds.size.width),
        );
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
            self.release_view();
            return true;
        }
        let handler = self
            .lifecycle
            .runtime()
            .borrow()
            .windows
            .close_handler(&self.window_id);
        let Some(handler) = handler else {
            self.release_view();
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
                self.release_view();
                true
            }
        }
    }

    fn release_view(&mut self) {
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
        drop(native);
        self.host.unregister_view(&self.view_id);
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
            runtime.trace_subscription_closures();
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
        if let Err(error) = self
            .engine
            .restore_component_exports(program.component_exports.clone())
        {
            self.last_error = Some(error.to_string());
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
        self.release_view();
    }
}

#[derive(Debug, Error)]
pub enum ScriptViewError {
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
    #[error("script view ID `{0}` must be 1-64 ASCII alphanumeric, `_`, or `-` characters")]
    InvalidId(String),
    #[error("script view `{0}` is already mounted in this host")]
    DuplicateView(String),
    #[error("script view `{0}` has been disposed")]
    DisposedView(String),
    #[error(
        "key binding `{keystrokes}` ({context:?}) conflicts: `{existing:?}` is already bound, requested `{requested:?}`"
    )]
    KeyBindingConflict {
        keystrokes: String,
        context: Option<String>,
        existing: ActionId,
        requested: ActionId,
    },
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
    #[error(transparent)]
    Overlay(#[from] crate::OverlayError),
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

    fn start_prepared(
        mut prepared: PreparedScriptView,
        view_id: &str,
        window_id: &str,
    ) -> (RuntimeEngine, ScriptLifecycle) {
        let program = prepared.factory.program();
        let lifecycle = prepared
            .factory
            .mount_lifecycle(
                &mut prepared.engine,
                program,
                view_id,
                window_id,
                WindowCommandPolicy::Disabled,
                true,
            )
            .unwrap();
        (prepared.engine, lifecycle)
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
    fn prepared_view_starts_engine_and_lifecycle_on_mount() {
        let directory = tempfile::tempdir().unwrap();
        let entry = directory.path().join("main.rhai");
        fs::write(&entry, "fn view(ctx) { text(\"prepared\") }").unwrap();
        fs::write(
            directory.path().join("theme.rhai"),
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .unwrap();
        write_manifest(directory.path());
        let prepared = FileScriptView::new(&entry).prepare().unwrap();
        let (engine, lifecycle) = start_prepared(prepared, "widget", "main");
        assert!(engine.is_current(lifecycle.generation()));
        assert!(lifecycle.root().is_some());
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

        let prepared = FileScriptView::new(directory.path().join("main.rhai"))
            .prepare()
            .unwrap();
        let (_, lifecycle) = start_prepared(prepared, "widget", "main");
        let root = lifecycle.root().unwrap();
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
        let prepared = EmbeddedScriptView::new(
            entry,
            scripts,
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .prepare()
        .unwrap();

        prepared
            .factory
            .runtime
            .borrow_mut()
            .windows
            .request_open(ScriptWindowSpec {
                id: "settings".to_owned(),
                title: "Settings".to_owned(),
                width: 400.0,
                height: 300.0,
                focus: true,
            })
            .unwrap();
        let (_, lifecycle, _) = prepared
            .factory
            .instantiate("settings-view", "settings")
            .unwrap();
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
            FileScriptView::new(directory.path().join("main.rhai")).prepare(),
            Err(ScriptViewError::Capability(CapabilityError::Missing(_)))
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
        let prepared = EmbeddedScriptView::new(
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
        let (_, lifecycle) = start_prepared(prepared, "asset-view", "main");
        assert!(matches!(
            lifecycle.root().unwrap().kind(),
            crate::UiNodeKind::Image {
                source: crate::ImageSourceSpec::Handle(handle),
            } if handle.id() != 0
        ));
    }

    #[test]
    fn declared_component_assets_preload_and_render_without_lifecycle_io() {
        let entry = ModuleId::parse("main").unwrap();
        let component = ModuleId::parse("components/declarative_icon").unwrap();
        let scripts = EmbeddedScriptSource::new(BTreeMap::from([
            (
                entry.clone(),
                r#"
                    import "components/declarative_icon" as declarative_icon;
                    fn view(ctx) { declarative_icon::DeclarativeIcon(#{} ) }
                "#
                .to_owned(),
            ),
            (
                component,
                r#"/* gpui-rhai
{
  "id": "components/declarative_icon",
  "export": "DeclarativeIcon",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/check.svg"]
}
*/
export_component(#{
    metadata: #{ id: "components/declarative_icon", "export": "DeclarativeIcon",
        version: "0.1.0", runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{}, assets: ["icons/check.svg"] },
    schema: #{ props: #{}, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"] }
});
fn DeclarativeIcon(props) {
    component_render("components/declarative_icon", props, Fn("render_DeclarativeIcon"))
}
fn render_DeclarativeIcon(ctx, props) { image(asset("app/icons/check")) }
"#
                .to_owned(),
            ),
        ]));
        let prepared = EmbeddedScriptView::new(
            entry,
            scripts,
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .asset_sources([(
            "icons/check".to_owned(),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: include_bytes!("../../../registry/assets/icons/check.svg").to_vec(),
            },
        )])
        .prepare()
        .unwrap();
        assert!(
            prepared
                .factory
                .runtime
                .borrow()
                .assets
                .cached_image(&AssetId::parse("app/icons/check").unwrap())
                .is_ok()
        );
        let (_, lifecycle) = start_prepared(prepared, "asset-view", "main");
        assert!(matches!(
            lifecycle.root().unwrap().kind(),
            crate::UiNodeKind::Image {
                source: crate::ImageSourceSpec::Asset(asset),
            } if asset.as_str() == "app/icons/check"
        ));
    }

    #[test]
    fn missing_declared_component_asset_fails_preparation() {
        let entry = ModuleId::parse("main").unwrap();
        let component = ModuleId::parse("components/missing_asset").unwrap();
        let scripts = EmbeddedScriptSource::new(BTreeMap::from([
            (
                entry.clone(),
                "import \"components/missing_asset\" as missing; fn view(ctx) { missing::MissingAsset(#{} ) }"
                    .to_owned(),
            ),
            (
                component,
                r#"/* gpui-rhai
{
  "id": "components/missing_asset", "export": "MissingAsset", "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [], "capabilities": {}, "assets": ["icons/missing.svg"]
}
*/
export_component(#{ metadata: #{ id: "components/missing_asset", "export": "MissingAsset",
    version: "0.1.0", runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
    dependencies: [], capabilities: #{}, assets: ["icons/missing.svg"] },
    schema: #{ props: #{}, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"] } });
fn MissingAsset(props) { component_render("components/missing_asset", props, Fn("render_MissingAsset")) }
fn render_MissingAsset(ctx, props) { image(asset("app/icons/missing")) }
"#
                .to_owned(),
            ),
        ]));
        let result = EmbeddedScriptView::new(
            entry,
            scripts,
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .prepare();
        assert!(matches!(result, Err(ScriptViewError::Asset(_))));
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
        let file = FileScriptView::new(directory.path().join("main.rhai"))
            .prepare()
            .unwrap();

        let entry = ModuleId::parse("main").unwrap();
        let embedded = EmbeddedScriptView::new(
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

        let (_, file) = start_prepared(file, "file-view", "main");
        let (_, embedded) = start_prepared(embedded, "embedded-view", "main");

        for root in [file.root().unwrap(), embedded.root().unwrap()] {
            let crate::UiNodeKind::Container { children } = root.kind() else {
                panic!("equivalence app must render a row");
            };
            assert!(
                matches!(&children[0].kind(), crate::UiNodeKind::Text { text } if text == "Loading")
            );
            assert!(matches!(
                &children[1].kind(),
                crate::UiNodeKind::Image {
                    source: crate::ImageSourceSpec::Handle(handle),
                } if handle.id() == 1
            ));
        }
    }

    #[test]
    fn missing_entry_has_path_aware_error() {
        let path = Path::new("definitely-missing-ui/main.rhai");
        let Err(error) = FileScriptView::new(path).prepare() else {
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
        let fire = lifecycle.root().unwrap().handlers()["click"]
            .as_script()
            .unwrap()
            .clone();
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
