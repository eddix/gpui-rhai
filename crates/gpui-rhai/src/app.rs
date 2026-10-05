use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::KeyBinding;
use gpui::actions;
use gpui::{
    AnyElement, AnyWindowHandle, App, AppContext, Bounds, Context, DispatchPhase, Element,
    ElementId, Entity, FocusHandle, Global, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, MouseButton, MouseDownEvent, ParentElement, Pixels,
    Render, Role, ScrollAnchor, ScrollHandle, SharedString, StatefulInteractiveElement, Styled,
    Subscription, Task, TitlebarOptions, Window, WindowAppearance, WindowBounds, WindowOptions,
    deferred, div, px, rgba, size,
};
use thiserror::Error;

#[cfg(feature = "dev-reload")]
use crate::FileWatcher;
use crate::overlay_element::WindowOverlayCoordinator;
use crate::{
    ActionError, ActionId, AppManifest, AssetData, AssetId, AssetRegistry, CapabilityError,
    CompiledUi, ComponentExportError, ComponentInstancePath, ComponentRegistry,
    ComponentStateSchema, ComponentStyleError, ComponentStyleSheet, DependencyError, Diagnostic,
    DirectoryAssetProvider, DispatchScriptAction, EmbeddedScriptSource, FileScriptSource,
    GpuiNodeRenderer, InMemoryAssetProvider, InteractionState, KeyBindingSpec, LocaleBundle,
    LocaleManager, ModuleCompileCache, ModuleId, MotionPreference, MotionRuntime,
    NodeEventDispatcher, PrimitiveRegistry, ResponsiveError, ResponsiveRuntime,
    RestrictedModuleResolver, RuntimeEngine, RuntimeError, ScriptCallback, ScriptLifecycle,
    ScriptSource, ScriptWindowSpec, SystemAppearance, TextDirection, ThemeManager, ThemeSelection,
    ThemeSnapshot, ThemeTokenOverrides, ThemeVariant, UiRuntimeState, UiValue, ViewportBreakpoints,
    WindowCommand, WindowCommandPolicy, init_text_area, init_text_input, load_component_styles,
    load_locale_source,
};

#[cfg(feature = "dev-reload")]
actions!(gpui_rhai_devtools, [ToggleInspector]);
actions!(gpui_rhai_host, [CopySelectedText]);

const HOST_KEY_CONTEXT: &str = "GPUIRhaiHost";

#[derive(Default)]
struct ScriptRuntimeInstallation {
    bindings: BTreeMap<(String, Option<String>), ActionId>,
    loaded_fonts: BTreeSet<u64>,
    window_command_owners: BTreeMap<gpui::WindowId, (std::rc::Weak<()>, String)>,
}

impl Global for ScriptRuntimeInstallation {}

thread_local! {
    static FONT_NAMES: RefCell<Option<(usize, Rc<BTreeSet<String>>)>> =
        const { RefCell::new(None) };
}

/// The font families the text system can resolve, cached per set of fonts
/// loaded through the runtime.
fn available_font_names(cx: &App) -> Rc<BTreeSet<String>> {
    let loaded = cx
        .try_global::<ScriptRuntimeInstallation>()
        .map_or(0, |installation| installation.loaded_fonts.len());
    FONT_NAMES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((key, names)) = cache.as_ref()
            && *key == loaded
        {
            return Rc::clone(names);
        }
        let names = Rc::new(cx.text_system().all_font_names().into_iter().collect());
        *cache = Some((loaded, Rc::clone(&names)));
        names
    })
}

/// Install GPUI Rhai's application-wide input actions once.
pub fn install(cx: &mut App) {
    if cx.has_global::<ScriptRuntimeInstallation>() {
        return;
    }
    init_text_input(cx);
    init_text_area(cx);
    crate::init_document_view(cx);
    cx.bind_keys([KeyBinding::new(
        "cmd-c",
        CopySelectedText,
        Some(HOST_KEY_CONTEXT),
    )]);
    cx.set_global(ScriptRuntimeInstallation::default());
}

#[derive(Clone, Debug)]
pub struct ScriptViewConfig {
    view_id: String,
    paint_background: bool,
    show_error_banner: bool,
}

impl ScriptViewConfig {
    #[must_use]
    pub fn new(view_id: impl Into<String>) -> Self {
        Self {
            view_id: view_id.into(),
            paint_background: false,
            show_error_banner: true,
        }
    }

    #[must_use]
    pub const fn paint_background(mut self, paint: bool) -> Self {
        self.paint_background = paint;
        self
    }

    /// Control the built-in selectable runtime-error banner for this view.
    ///
    /// Hosts that disable it must surface [`ScriptViewHandle::last_error`]
    /// themselves so failed candidates are not silent.
    #[must_use]
    pub const fn show_error_banner(mut self, show: bool) -> Self {
        self.show_error_banner = show;
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
    interactions: crate::interaction::WindowInteractionCoordinator,
    fallback_focus: FocusHandle,
    views: BTreeMap<String, HostViewRegistration>,
    pending_focus_recovery: Vec<FocusHandle>,
    overlay_viewport: Option<crate::OverlayBounds>,
    native_window: Option<AnyWindowHandle>,
    window_command_owner: Option<NativeWindowCommandOwner>,
    frame_active: bool,
    container_bounds: Option<Bounds<Pixels>>,
    #[allow(dead_code)]
    escape_interceptor: Option<Subscription>,
}

struct NativeWindowCommandOwner {
    view_id: String,
    _lease: Rc<()>,
}

struct HostViewRegistration {
    focus: FocusHandle,
    lease: Rc<()>,
}

#[derive(Clone)]
struct NativeWindowAuthority {
    window: gpui::WindowId,
    lease: std::rc::Weak<()>,
}

impl NativeWindowAuthority {
    fn is_current(&self, cx: &App) -> bool {
        self.lease.strong_count() > 0
            && cx
                .windows()
                .iter()
                .any(|window| window.window_id() == self.window)
            && cx
                .global::<ScriptRuntimeInstallation>()
                .window_command_owners
                .get(&self.window)
                .is_some_and(|(lease, _)| lease.ptr_eq(&self.lease))
    }
}

impl ScriptViewHost {
    /// Create one interaction/overlay domain, normally one per GPUI window.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::InvalidId`] for an unsafe window identifier.
    pub fn new(window_id: impl Into<String>, cx: &mut App) -> Result<Self, ScriptViewError> {
        install(cx);
        let window_id = window_id.into();
        validate_view_id(&window_id)?;
        let inner = Rc::new(RefCell::new(ScriptViewHostState {
            window_id,
            overlays: WindowOverlayCoordinator::default(),
            interactions: crate::interaction::WindowInteractionCoordinator::default(),
            fallback_focus: cx.focus_handle(),
            views: BTreeMap::new(),
            pending_focus_recovery: Vec::new(),
            overlay_viewport: None,
            native_window: None,
            window_command_owner: None,
            frame_active: false,
            container_bounds: None,
            escape_interceptor: None,
        }));
        let weak = Rc::downgrade(&inner);
        let interceptor = cx.intercept_keystrokes(move |event, window, cx| {
            if event.keystroke.key.as_str() != "escape" {
                return;
            }
            let Some(state) = weak.upgrade() else {
                return;
            };
            let interactions = state.borrow().interactions.clone();
            if interactions.cancel_window(window, cx) {
                cx.stop_propagation();
            }
        });
        inner.borrow_mut().escape_interceptor = Some(interceptor);
        Ok(Self { inner })
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
            .child(SharedLayerPortalElement {
                coordinator: overlays,
            })
            .into_any_element();
        ScriptViewHostFrame {
            host: self.clone(),
            child: Some(child),
        }
        .into_any_element()
    }

    fn reserve_view(&self, view_id: &str) -> Result<std::rc::Weak<()>, ScriptViewError> {
        validate_view_id(view_id)?;
        let mut state = self.inner.borrow_mut();
        if state.views.contains_key(view_id) {
            return Err(ScriptViewError::DuplicateView(view_id.to_owned()));
        }
        let fallback = state.fallback_focus.clone();
        let lease = Rc::new(());
        let weak = Rc::downgrade(&lease);
        state.views.insert(
            view_id.to_owned(),
            HostViewRegistration {
                focus: fallback,
                lease,
            },
        );
        Ok(weak)
    }

    fn attach_view_focus(&self, view_id: &str, focus: FocusHandle) {
        if let Some(entry) = self.inner.borrow_mut().views.get_mut(view_id) {
            entry.focus = focus;
        }
    }

    fn bind_window(
        &self,
        view_id: &str,
        handle: AnyWindowHandle,
        policy: WindowCommandPolicy,
        cx: &mut App,
    ) -> Result<Option<NativeWindowAuthority>, ScriptViewError> {
        let mut state = self.inner.borrow_mut();
        if state
            .native_window
            .is_some_and(|previous| previous.window_id() != handle.window_id())
        {
            return Err(ScriptViewError::WrongNativeWindow(state.window_id.clone()));
        }
        if policy == WindowCommandPolicy::ApplicationOwned {
            if let Some(owner) = &state.window_command_owner {
                return Err(ScriptViewError::WindowCommandOwner {
                    window: state.window_id.clone(),
                    owner: owner.view_id.clone(),
                });
            }
            let owners = &mut cx
                .global_mut::<ScriptRuntimeInstallation>()
                .window_command_owners;
            owners.retain(|_, (lease, _)| lease.strong_count() > 0);
            if let Some((_, owner)) = owners.get(&handle.window_id()) {
                return Err(ScriptViewError::WindowCommandOwner {
                    window: state.window_id.clone(),
                    owner: owner.clone(),
                });
            }
            let lease = Rc::clone(&state.views[view_id].lease);
            owners.insert(
                handle.window_id(),
                (Rc::downgrade(&lease), view_id.to_owned()),
            );
            state.window_command_owner = Some(NativeWindowCommandOwner {
                view_id: view_id.to_owned(),
                _lease: lease,
            });
        }
        state.native_window = Some(handle);
        Ok(
            (policy == WindowCommandPolicy::ApplicationOwned).then(|| NativeWindowAuthority {
                window: handle.window_id(),
                lease: Rc::downgrade(&state.views[view_id].lease),
            }),
        )
    }

    fn unregister_view(&self, view_id: &str, lease: &std::rc::Weak<()>) {
        let mut state = self.inner.borrow_mut();
        if state
            .views
            .get(view_id)
            .is_none_or(|current| !Rc::downgrade(&current.lease).ptr_eq(lease))
        {
            return;
        }
        if state
            .window_command_owner
            .as_ref()
            .is_some_and(|owner| owner.view_id == view_id)
        {
            state.window_command_owner = None;
        }
        if let Some(registration) = state.views.remove(view_id) {
            state.pending_focus_recovery.push(registration.focus);
        }
        state.overlays.remove_view(view_id);
        state.interactions.discard_view(view_id);
    }

    fn quiesce_view(&self, view_id: &str, focus: &FocusHandle, window: &mut Window, cx: &mut App) {
        let (fallback, overlays, interactions) = {
            let state = self.inner.borrow();
            (
                state.fallback_focus.clone(),
                state.overlays.clone(),
                state.interactions.clone(),
            )
        };
        if focus.contains_focused(window, cx) {
            fallback.focus(window, cx);
        }
        overlays.remove_view(view_id);
        interactions.cancel_view(view_id, window, cx);
    }

    fn overlays(&self) -> WindowOverlayCoordinator {
        self.inner.borrow().overlays.clone()
    }

    fn interactions(&self) -> crate::interaction::WindowInteractionCoordinator {
        self.inner.borrow().interactions.clone()
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
        let (overlays, viewport, fallback, pending) = {
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
                state.fallback_focus.clone(),
                std::mem::take(&mut state.pending_focus_recovery),
            )
        };
        overlays.begin_host_frame(viewport);
        self.host.interactions().begin_frame();
        if pending
            .iter()
            .any(|focus| focus.contains_focused(window, cx))
        {
            fallback.focus(window, cx);
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
        let (overlays, interactions, container_bounds) = {
            let state = self.host.inner.borrow();
            (
                state.overlays.clone(),
                state.interactions.clone(),
                state.container_bounds,
            )
        };
        interactions.finish_frame(window, cx);
        interactions.install(window);
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

struct SharedLayerPortalElement {
    coordinator: WindowOverlayCoordinator,
}

impl Element for SharedLayerPortalElement {
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
        let layers = self.coordinator.take_layer_elements();
        let mut layer = deferred(
            div()
                .absolute()
                .left(pixel_from_f64(viewport.x))
                .top(pixel_from_f64(viewport.y))
                .w(pixel_from_f64(viewport.width))
                .h(pixel_from_f64(viewport.height))
                .children(layers),
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

impl IntoElement for SharedLayerPortalElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[derive(Clone)]
pub struct ScriptViewHandle(Rc<ScriptViewHandleInner>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScriptViewState {
    Active,
    Suspended,
    Disposed,
}

#[derive(Clone)]
pub struct ThemeHandle(Entity<ThemeHandleState>);

struct ThemeHandleState {
    snapshot: ThemeSnapshot,
}

impl ThemeHandleState {
    fn replace(&mut self, variant: ThemeVariant) -> bool {
        if self.snapshot.variant == variant {
            return false;
        }
        self.snapshot = ThemeSnapshot::new(self.snapshot.revision.saturating_add(1), variant);
        true
    }
}

impl ThemeHandle {
    fn new(variant: ThemeVariant, cx: &mut App) -> Self {
        Self(cx.new(|_| ThemeHandleState {
            snapshot: ThemeSnapshot::new(1, variant),
        }))
    }

    /// Read the latest effective theme without invoking Rhai.
    #[must_use]
    pub fn snapshot(&self, cx: &App) -> ThemeSnapshot {
        self.0.read(cx).snapshot.clone()
    }

    /// Observe effective-theme changes from application-level host code.
    ///
    /// The returned subscription must be retained for as long as observation is
    /// required.
    pub fn observe(
        &self,
        cx: &mut App,
        mut on_change: impl FnMut(ThemeSnapshot, &mut App) + 'static,
    ) -> Subscription {
        cx.observe(&self.0, move |entity, cx| {
            let snapshot = entity.read(cx).snapshot.clone();
            on_change(snapshot, cx);
        })
    }

    /// Observe effective-theme changes from another GPUI entity.
    ///
    /// This is the ergonomic path for Host-owned chrome: retain the returned
    /// subscription in the observing entity and redraw only that entity.
    pub fn observe_in<T: 'static>(
        &self,
        cx: &mut Context<T>,
        mut on_change: impl FnMut(&mut T, ThemeSnapshot, &mut Context<T>) + 'static,
    ) -> Subscription {
        cx.observe(&self.0, move |owner, entity, cx| {
            let snapshot = entity.read(cx).snapshot.clone();
            on_change(owner, snapshot, cx);
        })
    }

    fn matches(&self, variant: &ThemeVariant, cx: &App) -> bool {
        &self.0.read(cx).snapshot.variant == variant
    }

    fn publish(&self, variant: ThemeVariant, cx: &mut App) {
        self.0.update(cx, |state, cx| {
            if state.replace(variant) {
                cx.notify();
            }
        });
    }
}

/// Drainable execution diagnostics for one mounted script view.
///
/// Taking a snapshot does not invoke Rhai or alter the mounted UI. Timings are
/// drained so benchmark samples never double-count earlier work.
#[derive(Clone, Debug)]
pub struct ScriptViewPerformanceSnapshot {
    pub timings: Vec<crate::ExecutionTiming>,
    pub virtual_collections: Vec<crate::VirtualCollectionMetrics>,
    pub retained_nodes: usize,
    pub dirty_components: usize,
    pub dirty_component_paths: Vec<String>,
    pub pending_virtual_requests: bool,
}

struct ScriptViewHandleInner {
    entity: Entity<ScriptHostView>,
    theme: ThemeHandle,
    host: ScriptViewHost,
    view_id: String,
    state: Rc<Cell<ScriptViewState>>,
    measured_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    mount_lease: std::rc::Weak<()>,
}

impl Drop for ScriptViewHandleInner {
    fn drop(&mut self) {
        if self.state.get() != ScriptViewState::Disposed {
            self.host.unregister_view(&self.view_id, &self.mount_lease);
        }
    }
}

impl ScriptViewHandle {
    fn require_not_disposed(&self) -> Result<ScriptViewState, ScriptViewError> {
        let state = self.0.state.get();
        if state == ScriptViewState::Disposed {
            Err(ScriptViewError::DisposedView(self.0.view_id.clone()))
        } else {
            Ok(state)
        }
    }

    fn require_active(&self) -> Result<(), ScriptViewError> {
        match self.require_not_disposed()? {
            ScriptViewState::Active => Ok(()),
            ScriptViewState::Suspended => {
                Err(ScriptViewError::SuspendedView(self.0.view_id.clone()))
            }
            ScriptViewState::Disposed => unreachable!(),
        }
    }

    #[must_use]
    pub fn view_id(&self) -> &str {
        &self.0.view_id
    }

    #[must_use]
    pub fn state(&self) -> ScriptViewState {
        self.0.state.get()
    }

    /// Return a read-only, observable handle to this view's effective theme.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after explicit disposal.
    pub fn theme(&self) -> Result<ThemeHandle, ScriptViewError> {
        self.require_not_disposed()?;
        Ok(self.0.theme.clone())
    }

    /// Read this view's latest effective theme without invoking Rhai.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after explicit disposal.
    pub fn theme_snapshot(&self, cx: &App) -> Result<ThemeSnapshot, ScriptViewError> {
        Ok(self.theme()?.snapshot(cx))
    }

    /// Select this view's app-level theme from trusted Host code.
    ///
    /// # Errors
    ///
    /// Returns after disposal or when the requested theme is unavailable.
    pub fn select_theme(
        &self,
        family: &str,
        variant: &str,
        cx: &mut App,
    ) -> Result<bool, ScriptViewError> {
        self.require_not_disposed()?;
        self.0.entity.update(cx, |view, cx| {
            let changed = view
                .lifecycle
                .runtime()
                .borrow_mut()
                .select_theme_from_host(family, variant)
                .map_err(|error| ScriptViewError::Theme(error.to_string()))?;
            if changed {
                cx.notify();
            }
            Ok(changed)
        })
    }

    /// Select this view's app-level locale from trusted Host code.
    ///
    /// # Errors
    ///
    /// Returns after disposal or when the requested locale is unavailable.
    pub fn select_locale(&self, locale: &str, cx: &mut App) -> Result<bool, ScriptViewError> {
        self.require_not_disposed()?;
        self.0.entity.update(cx, |view, cx| {
            let changed = view
                .lifecycle
                .runtime()
                .borrow_mut()
                .select_locale_from_host(locale)
                .map_err(|error| ScriptViewError::Locale(error.to_string()))?;
            if changed {
                cx.notify();
            }
            Ok(changed)
        })
    }

    /// Replace this view's Host-owned motion preference without resetting state.
    ///
    /// # Errors
    ///
    /// Returns after disposal.
    pub fn set_motion_preference(
        &self,
        preference: MotionPreference,
        cx: &mut App,
    ) -> Result<bool, ScriptViewError> {
        self.require_not_disposed()?;
        self.0.entity.update(cx, |view, cx| {
            let changed = view
                .lifecycle
                .runtime()
                .borrow_mut()
                .set_motion_preference_from_host(preference);
            if changed {
                cx.notify();
            }
            Ok(changed)
        })
    }

    /// Return the latest rendered declarative root.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after disposal.
    /// Return the last successfully committed declarative tree.
    ///
    /// A failed render deliberately preserves this last-good tree. Pair this
    /// method with [`Self::last_error`] when the caller must distinguish a
    /// current successful tree from a rollback after failure.
    pub fn root(&self, cx: &App) -> Result<Option<crate::UiNode>, ScriptViewError> {
        self.require_not_disposed()?;
        Ok(self.0.entity.read(cx).lifecycle.root().cloned())
    }

    /// The revision of the view's committed tree. It advances whenever a render
    /// commits, so Host work that depends only on the committed composition
    /// (an audit, a snapshot) can skip frames that merely repaint.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after disposal.
    pub fn committed_revision(&self, cx: &App) -> Result<u64, ScriptViewError> {
        self.require_not_disposed()?;
        Ok(self.0.entity.read(cx).lifecycle.revision())
    }

    /// Return the latest mounted-view render, callback, delivery, or reload error.
    ///
    /// Errors remain available while the last-good tree continues to render.
    /// A later successful script transaction clears the value; native-only
    /// repaint and animation frames do not.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after disposal.
    pub fn last_error(&self, cx: &App) -> Result<Option<String>, ScriptViewError> {
        self.require_not_disposed()?;
        Ok(self
            .0
            .entity
            .read(cx)
            .last_failure
            .as_ref()
            .map(|failure| failure.message.clone()))
    }

    /// Return structured details for the latest script runtime error.
    ///
    /// Non-script host errors only have a string representation and return
    /// `None` here.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after disposal.
    pub fn last_diagnostic(&self, cx: &App) -> Result<Option<Diagnostic>, ScriptViewError> {
        self.require_not_disposed()?;
        Ok(self
            .0
            .entity
            .read(cx)
            .last_failure
            .as_ref()
            .and_then(|failure| failure.diagnostic.as_deref().cloned()))
    }

    /// Drain execution timings and snapshot retained/virtual metrics.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after disposal.
    pub fn take_performance_snapshot(
        &self,
        cx: &mut App,
    ) -> Result<ScriptViewPerformanceSnapshot, ScriptViewError> {
        self.require_not_disposed()?;
        Ok(self.0.entity.update(cx, |view, _| {
            let (
                virtual_collections,
                dirty_components,
                dirty_component_paths,
                pending_virtual_requests,
            ) = {
                let runtime_handle = view.lifecycle.runtime();
                let runtime = runtime_handle.borrow();
                (
                    runtime.virtual_requests.inspect(),
                    runtime.dirty_components().len(),
                    runtime
                        .dirty_components()
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                    runtime.has_virtual_requests(),
                )
            };
            ScriptViewPerformanceSnapshot {
                timings: std::mem::take(&mut view.timings),
                virtual_collections,
                retained_nodes: view.lifecycle.retained().len(),
                dirty_components,
                dirty_component_paths,
                pending_virtual_requests,
            }
        }))
    }

    /// Read one mounted native hot value without invoking Rhai.
    ///
    /// # Errors
    ///
    /// Returns after disposal or when the signal is stale.
    pub fn read_signal(
        &self,
        signal: &crate::NativeSignal,
        cx: &App,
    ) -> Result<crate::SignalValue, ScriptViewError> {
        self.require_not_disposed()?;
        Ok(self
            .0
            .entity
            .read(cx)
            .lifecycle
            .runtime()
            .borrow()
            .signals
            .read(signal)?)
    }

    /// Update one mounted native hot value on the GPUI foreground thread.
    ///
    /// This repaints signal-bound properties without running a Rhai component.
    ///
    /// # Errors
    ///
    /// Returns after disposal or for stale/type-incompatible values.
    pub fn write_signal(
        &self,
        signal: &crate::NativeSignal,
        value: crate::SignalValue,
        cx: &mut App,
    ) -> Result<bool, ScriptViewError> {
        self.require_not_disposed()?;
        self.0.entity.update(cx, |view, cx| {
            let changed = view
                .lifecycle
                .runtime()
                .borrow_mut()
                .signals
                .write(signal, value)?;
            if changed {
                cx.notify();
            }
            Ok::<_, ScriptViewError>(changed)
        })
    }

    /// Replace one registered Rust-owned collection on the GPUI foreground thread.
    ///
    /// Only components that read this collection are marked dirty. The
    /// replacement is immutable and can be prepared away from the Rhai runtime.
    ///
    /// # Errors
    ///
    /// Returns after disposal or when the collection name is unknown.
    pub fn replace_native_collection(
        &self,
        name: &str,
        collection: crate::NativeCollection,
        cx: &mut App,
    ) -> Result<bool, ScriptViewError> {
        self.require_not_disposed()?;
        self.0.entity.update(cx, |view, cx| {
            let changed = view
                .lifecycle
                .runtime()
                .borrow_mut()
                .replace_native_collection_from_host(name, collection)?;
            if changed {
                cx.notify();
            }
            Ok::<_, ScriptViewError>(changed)
        })
    }

    /// Register a new Rust-owned collection on an already mounted view.
    ///
    /// The name must not already exist. Registration marks the view root dirty;
    /// an active view rerenders on the next foreground cycle, while a suspended
    /// view consumes the collection when it resumes.
    ///
    /// # Errors
    ///
    /// Returns after disposal or for an unsafe/duplicate collection name.
    pub fn register_native_collection(
        &self,
        name: &str,
        collection: crate::NativeCollection,
        cx: &mut App,
    ) -> Result<(), ScriptViewError> {
        self.require_not_disposed()?;
        self.0.entity.update(cx, |view, cx| {
            let root = view.lifecycle.root_path().clone();
            view.lifecycle
                .runtime()
                .borrow_mut()
                .register_native_collection_from_host(&root, name, collection)?;
            if view.state.get() == ScriptViewState::Active {
                cx.notify();
            }
            Ok::<_, ScriptViewError>(())
        })
    }

    /// Replace one registered Host-owned text document revision.
    ///
    /// Only components that read this document are invalidated. The immutable
    /// text snapshot may be prepared on a background thread before publication.
    ///
    /// # Errors
    ///
    /// Returns after disposal or when the document name is unknown.
    pub fn replace_native_text_document(
        &self,
        name: &str,
        document: crate::NativeTextDocument,
        cx: &mut App,
    ) -> Result<bool, ScriptViewError> {
        self.require_not_disposed()?;
        self.0.entity.update(cx, |view, cx| {
            let changed = view
                .lifecycle
                .runtime()
                .borrow_mut()
                .replace_native_text_document_from_host(name, document)?;
            if changed {
                cx.notify();
            }
            Ok::<_, ScriptViewError>(changed)
        })
    }

    /// Return the measured embedding element.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after explicit disposal.
    pub fn element(&self) -> Result<AnyElement, ScriptViewError> {
        self.require_active()?;
        Ok(MeasuredScriptViewElement {
            entity: self.0.entity.clone(),
            measured_bounds: Rc::clone(&self.0.measured_bounds),
        }
        .into_any_element())
    }

    /// Return the measured view as a zero-basis, shrinkable host flex item.
    ///
    /// Use this as the direct child of a Rust flex row or column. It prevents
    /// horizontally scrollable script content from contributing an automatic
    /// min-content width that expands the surrounding host layout.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after explicit disposal.
    pub fn flex_item(&self) -> Result<AnyElement, ScriptViewError> {
        Ok(div()
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .debug_selector(|| format!("gpui-rhai-flex-item:{}", self.view_id()))
            .child(self.element()?)
            .into_any_element())
    }

    pub(crate) fn host_slot_item(&self) -> Result<AnyElement, ScriptViewError> {
        let item = self.flex_item()?;
        if self.0.host.frame_active() {
            Ok(item)
        } else {
            Ok(self.0.host.container(item))
        }
    }

    /// Quiesce this retained view without discarding script or native UI state.
    ///
    /// # Errors
    ///
    /// Returns after disposal or when a suspend hook/effect cleanup fails.
    pub fn suspend(&self, window: &mut Window, cx: &mut App) -> Result<bool, ScriptViewError> {
        match self.require_not_disposed()? {
            ScriptViewState::Suspended => return Ok(false),
            ScriptViewState::Active => {}
            ScriptViewState::Disposed => unreachable!(),
        }
        self.0
            .entity
            .update(cx, |view, cx| view.suspend_view(window, cx))
    }

    /// Atomically resume a suspended view before returning it to Host layout.
    ///
    /// # Errors
    ///
    /// Returns after disposal or when pending delivery, resume hook, reload, or
    /// the first restored render fails. A compensable failure leaves the view
    /// suspended; failed compensation disposes and quarantines the view.
    pub fn resume(&self, cx: &mut App) -> Result<bool, ScriptViewError> {
        match self.require_not_disposed()? {
            ScriptViewState::Active => return Ok(false),
            ScriptViewState::Suspended => {}
            ScriptViewState::Disposed => unreachable!(),
        }
        self.0.entity.update(cx, ScriptHostView::resume_view)
    }

    /// Dispose this view immediately. The operation is idempotent.
    ///
    /// # Errors
    ///
    /// Returns an entity update error only if GPUI has already released the
    /// underlying view unexpectedly.
    pub fn dispose(&self, cx: &mut App) -> Result<(), ScriptViewError> {
        if self.0.state.get() == ScriptViewState::Disposed {
            return Ok(());
        }
        self.0.entity.update(cx, |view, cx| {
            view.release_view();
            cx.notify();
        });
        self.0
            .host
            .unregister_view(&self.0.view_id, &self.0.mount_lease);
        cx.refresh_windows();
        Ok(())
    }

    /// Focus this view's stable root handle.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after disposal.
    pub fn focus(&self, window: &mut Window, cx: &mut App) -> Result<(), ScriptViewError> {
        self.require_active()?;
        let focus = self.0.entity.read(cx).host_focus.clone();
        focus.focus(window, cx);
        Ok(())
    }

    /// Audit the committed composition with the rules of this view's profile
    /// (`ui/profile.rhai` or [`EmbeddedScriptView::profile_source`]). With no
    /// profile the result is empty.
    ///
    /// # Errors
    ///
    /// Returns after disposal.
    pub fn composition_audit(&self, cx: &App) -> Result<Vec<crate::AuditFinding>, ScriptViewError> {
        let rules = self
            .0
            .entity
            .read(cx)
            .lifecycle
            .runtime()
            .borrow()
            .audit_rules
            .clone();
        self.composition_audit_with(&rules, cx)
    }

    /// Audit the committed composition with an explicit rule set.
    ///
    /// # Errors
    ///
    /// Returns after disposal.
    pub fn composition_audit_with(
        &self,
        rules: &crate::AuditRules,
        cx: &App,
    ) -> Result<Vec<crate::AuditFinding>, ScriptViewError> {
        self.require_active()?;
        let theme = self.theme_snapshot(cx)?.variant;
        let view = self.0.entity.read(cx);
        let geometry = view
            .lifecycle
            .runtime()
            .borrow()
            .geometry_for(Some(&view.view_id));
        let direction = {
            let runtime = view.lifecycle.runtime();
            let runtime = runtime.borrow();
            let root = view.lifecycle.root_path();
            runtime
                .locale
                .as_ref()
                .and_then(|locale| locale.direction(Some(&view.window_id), Some(root)).ok())
                .unwrap_or(TextDirection::LeftToRight)
        };
        // Enumerating system fonts costs about 100 ms, so it happens only when
        // the font rule is on, and once per set of loaded fonts.
        let fonts = rules
            .contains(crate::AuditRule::UnresolvedFont)
            .then(|| available_font_names(cx));
        Ok(crate::composition_audit::audit(
            &crate::composition_audit::AuditInputs {
                tree: view.lifecycle.retained(),
                geometry: &geometry,
                theme: &theme,
                rules,
                available_fonts: fonts.as_deref(),
                direction,
            },
        ))
    }

    /// Snapshot the retained semantic tree and last committed geometry.
    ///
    /// # Errors
    ///
    /// Returns after disposal or for an invalid retained semantic graph.
    pub fn accessibility_snapshot(
        &self,
        cx: &App,
    ) -> Result<crate::AccessibilityTree, ScriptViewError> {
        self.require_active()?;
        let view = self.0.entity.read(cx);
        let geometry = view
            .lifecycle
            .runtime()
            .borrow()
            .geometry_for(Some(&view.view_id));
        let mut tree = crate::AccessibilityTree::from_committed(
            view.lifecycle.semantics(),
            view.lifecycle.retained(),
            &geometry,
            true,
        );
        tree.apply_primitive_projections(
            view.primitives
                .accessibility_projections(view.lifecycle.retained(), cx),
        );
        if let Some(failure) = &view.last_failure {
            let chart_error = failure.diagnostic.as_ref().is_some_and(|diagnostic| {
                diagnostic
                    .source
                    .as_deref()
                    .is_some_and(|source| source.starts_with("charts/"))
                    || diagnostic
                        .component
                        .as_deref()
                        .is_some_and(|component| component.contains("/Chart["))
            });
            tree.mark_runtime_error(&failure.message, chart_error);
        }
        Ok(tree)
    }

    /// Snapshot the stable language-neutral automation tree.
    ///
    /// # Errors
    ///
    /// Returns after disposal or for an invalid retained semantic graph.
    pub fn automation_snapshot(
        &self,
        cx: &App,
    ) -> Result<crate::AutomationSnapshot, ScriptViewError> {
        Ok(crate::AutomationSnapshot::from_accessibility(
            &self.accessibility_snapshot(cx)?,
        ))
    }

    /// Execute one automation command on the GPUI foreground thread.
    ///
    /// Dispatch and action commands use the mounted production handlers;
    /// deterministic time advance requires an injected controllable clock.
    ///
    /// # Errors
    ///
    /// Returns after disposal, for locator/command failures, or runtime errors.
    pub fn automate(
        &self,
        command: crate::AutomationCommand,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<crate::AutomationResult, ScriptViewError> {
        self.require_active()?;
        match command {
            crate::AutomationCommand::Snapshot => Ok(crate::AutomationResult::Snapshot {
                snapshot: self.automation_snapshot(cx)?,
            }),
            crate::AutomationCommand::Query { locator } => {
                let view = self.0.entity.read(cx);
                let geometry = view
                    .lifecycle
                    .runtime()
                    .borrow()
                    .geometry_for(Some(&view.view_id));
                let mut tree = crate::AccessibilityTree::from_committed(
                    view.lifecycle.semantics(),
                    view.lifecycle.retained(),
                    &geometry,
                    true,
                );
                tree.apply_primitive_projections(
                    view.primitives
                        .accessibility_projections(view.lifecycle.retained(), cx),
                );
                let id = crate::automation::resolve_locator(&tree, &locator)?;
                let node = tree
                    .node(id)
                    .ok_or_else(|| crate::AutomationError::StaleTarget(id.get()))?;
                Ok(crate::AutomationResult::Node {
                    node: Box::new(crate::AutomationNode::from(node)),
                })
            }
            command => self
                .0
                .entity
                .update(cx, |view, cx| view.execute_automation(command, window, cx)),
        }
    }

    /// Return the last successfully committed retained diff report.
    ///
    /// # Errors
    ///
    /// Returns after disposal.
    pub fn reconcile_report(&self, cx: &App) -> Result<crate::ReconcileReport, ScriptViewError> {
        self.require_not_disposed()?;
        Ok(self
            .0
            .entity
            .read(cx)
            .lifecycle
            .retained()
            .last_report()
            .clone())
    }

    /// Focus a mounted retained element without invoking Rhai.
    ///
    /// # Errors
    ///
    /// Returns after disposal, for a stale ref, or when the ref has no active
    /// GPUI focus handle.
    pub fn focus_element(
        &self,
        reference: &crate::ElementRef,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<(), ScriptViewError> {
        self.require_active()?;
        let handle = self.0.entity.update(cx, |view, _| {
            let node = view
                .lifecycle
                .runtime()
                .borrow()
                .element_refs
                .resolve(reference)?;
            view.focus_handles.get(&node).cloned().ok_or_else(|| {
                ScriptViewError::ElementRef(crate::ElementRefError::Stale(reference.id().clone()))
            })
        })?;
        handle.focus(window, cx);
        Ok(())
    }

    #[cfg(feature = "dev-reload")]
    /// Open or close this view's isolated development inspector.
    ///
    /// # Errors
    ///
    /// Returns [`ScriptViewError::DisposedView`] after disposal.
    pub fn set_inspector_open(&self, open: bool, cx: &mut App) -> Result<(), ScriptViewError> {
        self.require_active()?;
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

#[derive(Clone, Copy, Default)]
struct ScriptExecutionPolicy {
    operation_limit: Option<u64>,
    expression_depth_limits: Option<(usize, usize)>,
}

impl ScriptExecutionPolicy {
    fn apply(self, engine: &mut RuntimeEngine) {
        if let Some(limit) = self.operation_limit {
            engine.set_operation_limit(limit);
        }
        if let Some((global, functions)) = self.expression_depth_limits {
            engine.set_expression_depth_limits(global, functions);
        }
    }
}

pub struct FileScriptView {
    entry: PathBuf,
    development: bool,
    motion_preference: MotionPreference,
    motion_quality: crate::MotionQuality,
    extensions: Vec<Box<dyn ScriptViewExtension>>,
    key_bindings: Vec<KeyBindingSpec>,
    viewport_breakpoints: ViewportBreakpoints,
    calendar_clock: crate::CalendarClock,
    runtime_clock: crate::RuntimeClock,
    fonts: Vec<crate::FontSource>,
    token_base: Option<String>,
    theme_token_overrides: ThemeTokenOverrides,
    execution_policy: ScriptExecutionPolicy,
}

impl FileScriptView {
    #[must_use]
    pub fn new(entry: impl Into<PathBuf>) -> Self {
        Self {
            entry: entry.into(),
            development: cfg!(feature = "dev-reload") && cfg!(debug_assertions),
            motion_preference: motion_preference_from_env(),
            motion_quality: crate::MotionQuality::High,
            extensions: Vec::new(),
            key_bindings: Vec::new(),
            viewport_breakpoints: ViewportBreakpoints::default(),
            calendar_clock: crate::CalendarClock::default(),
            runtime_clock: crate::RuntimeClock::default(),
            fonts: Vec::new(),
            token_base: None,
            theme_token_overrides: ThemeTokenOverrides::default(),
            execution_policy: ScriptExecutionPolicy::default(),
        }
    }

    /// Use this token base source instead of discovering `ui/tokens.rhai`.
    #[must_use]
    pub fn token_base(mut self, source: impl Into<String>) -> Self {
        self.token_base = Some(source.into());
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
    pub const fn motion_quality(mut self, quality: crate::MotionQuality) -> Self {
        self.motion_quality = quality;
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

    #[must_use]
    pub fn runtime_clock(mut self, clock: crate::RuntimeClock) -> Self {
        self.runtime_clock = clock;
        self
    }

    #[must_use]
    pub fn font_source(mut self, font: crate::FontSource) -> Self {
        self.fonts.push(font);
        self
    }

    #[must_use]
    pub fn font_sources(mut self, fonts: impl IntoIterator<Item = crate::FontSource>) -> Self {
        self.fonts.extend(fonts);
        self
    }

    /// Apply host-owned user preferences to every theme loaded by this view.
    #[must_use]
    pub fn theme_token_overrides(mut self, overrides: ThemeTokenOverrides) -> Self {
        self.theme_token_overrides = overrides;
        self
    }

    /// Raise or lower the per-execution script operation budget for this
    /// view's scripts. Omitting this builder keeps the runtime's built-in
    /// 1,000,000-operation budget; hosts that run known one-shot cacheable
    /// script phases may raise it. The limit is applied to every engine
    /// instantiated for this view, including hot-reload candidates and
    /// additional windows.
    /// Zero normalizes to one. This quota does not promise a frame time or
    /// preempt native Rust work. Trusted extensions run after builder policy.
    #[must_use]
    pub fn operation_limit(mut self, limit: u64) -> Self {
        self.execution_policy.operation_limit = Some(limit);
        self
    }

    /// Set trusted parser depth limits; default global=64/function=32.
    /// Zero normalizes to one. Builder policy is applied before trusted
    /// extensions, which may override it; the same ordering applies to children.
    #[must_use]
    pub fn expression_depth_limits(mut self, global: usize, functions: usize) -> Self {
        self.execution_policy.expression_depth_limits = Some((global, functions));
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
        self.execution_policy.apply(&mut engine);
        for extension in extensions.iter() {
            extension
                .configure_engine(&mut engine)
                .map_err(ScriptViewError::Extension)?;
        }
        let theme_layers = file_theme_layers(
            engine.engine(),
            &self.entry,
            self.token_base.as_deref(),
            &self.theme_token_overrides,
        )?;
        let (ui_root, theme_path, theme) =
            load_primary_file_theme(&engine, &self.entry, &theme_layers)?;
        let style_path = ui_root.join("styles.rhai");
        let mut fonts = self.fonts;
        fonts.extend(load_file_fonts(&ui_root.join("fonts"))?);
        crate::validate_font_sources(&fonts)?;
        let manifest = load_file_manifest(&ui_root, &self.entry)?;
        let (module_cache, compiled) = engine.with_program_preparation(|engine| {
            let module_cache =
                configure_file_modules(engine, &ui_root, &self.entry, &theme_path, &style_path)?;
            let compiled =
                engine.compile_self_contained_named(&self.entry.to_string_lossy(), &source)?;
            Ok::<_, ScriptViewError>((module_cache, compiled))
        })?;
        #[cfg(not(feature = "dev-reload"))]
        let _ = &module_cache;
        let component_exports = engine.component_exports()?;
        let component_renderers = engine.component_renderer_snapshot()?;
        manifest.validate_components(&component_exports)?;
        let component_styles =
            load_file_component_styles(engine.engine(), &style_path, &component_exports)?;
        let state_schema = engine.root_state_schema(&compiled)?;
        let mut runtime_state = UiRuntimeState::new();
        runtime_state.motions = MotionRuntime::new(self.motion_preference);
        runtime_state.motions.set_quality(self.motion_quality);
        runtime_state.responsive = ResponsiveRuntime::new(self.viewport_breakpoints);
        runtime_state.calendar_clock = self.calendar_clock;
        runtime_state.clock = self.runtime_clock;
        runtime_state.key_bindings.clone_from(&self.key_bindings);
        runtime_state.locale = load_locale_directory(engine.engine(), &ui_root.join("locales"))?;
        let themes = load_theme_directory(
            engine.engine(),
            &ui_root.join("themes"),
            &theme,
            &theme_layers,
        )?;
        validate_component_tokens(&themes, &component_exports)?;
        runtime_state.theme = Some(themes);
        runtime_state.audit_rules = load_file_profile(engine.engine(), &ui_root)?.audit;
        runtime_state.replace_component_styles_from_host(component_styles);
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
            component_renderers,
        };
        let factory = Rc::new(ScriptWindowFactory {
            program: RefCell::new(program),
            runtime,
            extensions,
            theme: theme.clone(),
            #[cfg(feature = "dev-reload")]
            theme_layers: RefCell::new(theme_layers),
            show_error_banner: Cell::new(true),
            execution_policy: self.execution_policy,
            #[cfg(feature = "dev-reload")]
            development: self.development,
        });
        Ok(PreparedScriptView {
            engine,
            theme,
            entry: self.entry,
            ui_root,
            theme_path,
            style_path,
            development: self.development,
            factory,
            key_bindings: self.key_bindings,
            fonts,
            #[cfg(feature = "dev-reload")]
            module_cache,
        })
    }
}

pub struct EmbeddedScriptView {
    entry: ModuleId,
    scripts: EmbeddedScriptSource,
    theme_source: String,
    component_style_source: Option<String>,
    locales: Vec<(String, String)>,
    themes: Vec<(String, String)>,
    development: bool,
    motion_preference: MotionPreference,
    motion_quality: crate::MotionQuality,
    extensions: Vec<Box<dyn ScriptViewExtension>>,
    manifest: AppManifest,
    key_bindings: Vec<KeyBindingSpec>,
    assets: BTreeMap<String, AssetData>,
    viewport_breakpoints: ViewportBreakpoints,
    calendar_clock: crate::CalendarClock,
    runtime_clock: crate::RuntimeClock,
    fonts: Vec<crate::FontSource>,
    token_base: Option<String>,
    profile_source: Option<String>,
    theme_token_overrides: ThemeTokenOverrides,
    execution_policy: ScriptExecutionPolicy,
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
            component_style_source: None,
            locales: Vec::new(),
            themes: Vec::new(),
            development: false,
            motion_preference: motion_preference_from_env(),
            motion_quality: crate::MotionQuality::High,
            extensions: Vec::new(),
            manifest,
            key_bindings: Vec::new(),
            assets: BTreeMap::new(),
            viewport_breakpoints: ViewportBreakpoints::default(),
            calendar_clock: crate::CalendarClock::default(),
            runtime_clock: crate::RuntimeClock::default(),
            fonts: Vec::new(),
            token_base: None,
            profile_source: None,
            theme_token_overrides: ThemeTokenOverrides::default(),
            execution_policy: ScriptExecutionPolicy::default(),
        }
    }

    /// Install an application profile (`profile() -> map`) whose audit rules
    /// [`ScriptViewHandle::composition_audit`] applies.
    #[must_use]
    pub fn profile_source(mut self, source: impl Into<String>) -> Self {
        self.profile_source = Some(source.into());
        self
    }

    /// Install a token base (`tokens() -> map`) beneath every embedded theme.
    #[must_use]
    pub fn token_base(mut self, source: impl Into<String>) -> Self {
        self.token_base = Some(source.into());
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

    /// Install one application-wide typed component stylesheet.
    #[must_use]
    pub fn component_styles(mut self, source: impl Into<String>) -> Self {
        self.component_style_source = Some(source.into());
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
    pub const fn motion_quality(mut self, quality: crate::MotionQuality) -> Self {
        self.motion_quality = quality;
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

    #[must_use]
    pub fn runtime_clock(mut self, clock: crate::RuntimeClock) -> Self {
        self.runtime_clock = clock;
        self
    }

    #[must_use]
    pub fn font_source(mut self, font: crate::FontSource) -> Self {
        self.fonts.push(font);
        self
    }

    #[must_use]
    pub fn font_sources(mut self, fonts: impl IntoIterator<Item = crate::FontSource>) -> Self {
        self.fonts.extend(fonts);
        self
    }

    /// Apply host-owned user preferences to every embedded theme.
    #[must_use]
    pub fn theme_token_overrides(mut self, overrides: ThemeTokenOverrides) -> Self {
        self.theme_token_overrides = overrides;
        self
    }

    /// Raise or lower the per-execution script operation budget for this
    /// view's scripts. Omitting this builder keeps the runtime's built-in
    /// 1,000,000-operation budget; hosts that run known one-shot cacheable
    /// script phases may raise it. The limit is applied to every engine
    /// instantiated for this view, including additional windows.
    /// Zero normalizes to one. Trusted extensions run after builder policy;
    /// operation quotas are cooperative and are not wall-clock deadlines.
    #[must_use]
    pub fn operation_limit(mut self, limit: u64) -> Self {
        self.execution_policy.operation_limit = Some(limit);
        self
    }

    /// Set trusted parser depth limits; default global=64/function=32.
    /// Zero normalizes to one; trusted extensions run after this policy.
    #[must_use]
    pub fn expression_depth_limits(mut self, global: usize, functions: usize) -> Self {
        self.execution_policy.expression_depth_limits = Some((global, functions));
        self
    }

    /// Compile and initialize a fully embedded application.
    ///
    /// # Errors
    ///
    /// Returns source, theme, compile, or lifecycle errors.
    #[allow(clippy::too_many_lines)] // One ordered preparation pipeline, mirroring FileScriptView.
    pub fn prepare(self) -> Result<PreparedScriptView, ScriptViewError> {
        crate::validate_font_sources(&self.fonts)?;
        let entry = self.scripts.load(&self.entry)?;
        validate_manifest_entry(&self.manifest, &self.entry)?;
        let extensions = Rc::new(self.extensions);
        let mut engine = RuntimeEngine::new();
        self.execution_policy.apply(&mut engine);
        for extension in extensions.iter() {
            extension
                .configure_engine(&mut engine)
                .map_err(ScriptViewError::Extension)?;
        }
        let theme_layers = embedded_theme_layers(
            engine.engine(),
            self.token_base.as_deref(),
            &self.theme_token_overrides,
        )?;
        let theme = load_theme_with_overrides(
            engine.engine(),
            "<embedded-theme>",
            &self.theme_source,
            &theme_layers,
        )?;
        engine.set_module_resolver(RestrictedModuleResolver::from_source(&self.scripts)?);
        let compiled = engine.with_program_preparation(|engine| {
            preload_component_modules(engine, self.scripts.module_ids())?;
            engine
                .compile_self_contained_named(self.entry.as_str(), &entry.source)
                .map_err(ScriptViewError::from)
        })?;
        let component_exports = engine.component_exports()?;
        let component_renderers = engine.component_renderer_snapshot()?;
        self.manifest.validate_components(&component_exports)?;
        let component_styles = self.component_style_source.map_or_else(
            || Ok(ComponentStyleSheet::default()),
            |source| {
                load_component_styles(
                    engine.engine(),
                    "<embedded-component-styles>",
                    &source,
                    &component_exports,
                )
            },
        )?;
        let state_schema = engine.root_state_schema(&compiled)?;
        let mut runtime_state = UiRuntimeState::new();
        runtime_state.motions = MotionRuntime::new(self.motion_preference);
        runtime_state.motions.set_quality(self.motion_quality);
        runtime_state.responsive = ResponsiveRuntime::new(self.viewport_breakpoints);
        runtime_state.calendar_clock = self.calendar_clock;
        runtime_state.clock = self.runtime_clock;
        runtime_state.key_bindings.clone_from(&self.key_bindings);
        runtime_state.locale = load_embedded_locales(engine.engine(), self.locales)?;
        runtime_state.theme = Some(validated_themes(
            load_embedded_themes(engine.engine(), self.themes, &theme, &theme_layers)?,
            &component_exports,
        )?);
        runtime_state.replace_component_styles_from_host(component_styles);
        if let Some(source) = &self.profile_source {
            runtime_state.audit_rules =
                crate::load_profile_source(engine.engine(), "<embedded-profile>", source)
                    .map_err(ScriptViewError::Extension)?
                    .audit;
        }
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
            component_renderers,
        };
        let factory = Rc::new(ScriptWindowFactory {
            program: RefCell::new(program),
            runtime,
            extensions,
            theme: theme.clone(),
            #[cfg(feature = "dev-reload")]
            theme_layers: RefCell::new(theme_layers),
            show_error_banner: Cell::new(true),
            execution_policy: self.execution_policy,
            #[cfg(feature = "dev-reload")]
            development: self.development,
        });
        Ok(PreparedScriptView {
            engine,
            theme,
            entry: PathBuf::new(),
            ui_root: PathBuf::new(),
            theme_path: PathBuf::new(),
            style_path: PathBuf::new(),
            development: self.development,
            factory,
            key_bindings: self.key_bindings,
            fonts: self.fonts,
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
    overrides: &ThemeLayers,
) -> Result<ThemeManager, ScriptViewError> {
    let mut variants = BTreeMap::from([(
        (primary.family.clone(), primary.name.clone()),
        primary.clone(),
    )]);
    for (name, source) in sources {
        let variant = load_theme_with_overrides(engine, &name, &source, overrides)?;
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
    overrides: &ThemeLayers,
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
    let theme =
        load_theme_with_overrides(engine.engine(), &path.to_string_lossy(), &source, overrides)?;
    Ok((root, path, theme))
}

fn load_file_component_styles(
    engine: &rhai::Engine,
    path: &Path,
    components: &ComponentRegistry,
) -> Result<ComponentStyleSheet, ScriptViewError> {
    match fs::read_to_string(path) {
        Ok(source) => Ok(load_component_styles(
            engine,
            &path.to_string_lossy(),
            &source,
            components,
        )?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(ComponentStyleSheet::default())
        }
        Err(source) => Err(ScriptViewError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn configure_file_modules(
    engine: &mut RuntimeEngine,
    root: &Path,
    entry: &Path,
    theme: &Path,
    styles: &Path,
) -> Result<ModuleCompileCache, ScriptViewError> {
    let modules = discover_modules(root, entry, theme, styles)?;
    let source = FileScriptSource::new(root, modules)?;
    let mut cache = ModuleCompileCache::new();
    cache.refresh(engine.engine(), &source, source.module_ids())?;
    engine.set_module_resolver(RestrictedModuleResolver::from_source_with_cache(
        &source, &cache,
    )?);
    preload_component_modules(engine, source.module_ids())?;
    Ok(cache)
}

fn preload_component_modules(
    engine: &mut RuntimeEngine,
    modules: impl IntoIterator<Item = ModuleId>,
) -> Result<(), ScriptViewError> {
    let imports = modules
        .into_iter()
        .filter(|id| {
            ["components/", "layouts/", "patterns/"]
                .iter()
                .any(|category| id.as_str().starts_with(category))
        })
        .enumerate()
        .map(|(index, id)| format!("import \"{id}\" as component_{index};"))
        .collect::<Vec<_>>()
        .join("\n");
    if !imports.is_empty() {
        engine.compile_self_contained_named("<component-registry>", &imports)?;
    }
    Ok(())
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

fn load_file_fonts(directory: &Path) -> Result<Vec<crate::FontSource>, ScriptViewError> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut paths = collect_font_paths(directory)?;
    paths.retain(|path| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "ttf" | "otf" | "ttc"
                )
            })
    });
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let bytes = fs::read(&path).map_err(|error| crate::FontError::Io {
                path: path.display().to_string(),
                message: error.to_string(),
            })?;
            let label = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("font")
                .to_owned();
            crate::FontSource::new(label, bytes).map_err(ScriptViewError::from)
        })
        .collect()
}

fn collect_font_paths(directory: &Path) -> Result<Vec<PathBuf>, ScriptViewError> {
    let mut pending = vec![directory.to_path_buf()];
    let mut files = Vec::new();
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current).map_err(|error| crate::FontError::Io {
            path: current.display().to_string(),
            message: error.to_string(),
        })? {
            let path = entry
                .map_err(|error| crate::FontError::Io {
                    path: current.display().to_string(),
                    message: error.to_string(),
                })?
                .path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn install_declared_fonts(
    fonts: Vec<crate::FontSource>,
    cx: &mut App,
) -> Result<(), ScriptViewError> {
    if fonts.is_empty() {
        return Ok(());
    }
    let fresh = {
        let loaded = &cx.global::<ScriptRuntimeInstallation>().loaded_fonts;
        fonts
            .into_iter()
            .filter(|font| !loaded.contains(&font.fingerprint()))
            .collect::<Vec<_>>()
    };
    if fresh.is_empty() {
        return Ok(());
    }
    let fingerprints = fresh
        .iter()
        .map(crate::FontSource::fingerprint)
        .collect::<Vec<_>>();
    let bytes = fresh
        .into_iter()
        .map(|font| Cow::Owned(font.into_bytes()))
        .collect();
    cx.text_system()
        .add_fonts(bytes)
        .map_err(|error| crate::FontError::Load(error.to_string()))?;
    cx.global_mut::<ScriptRuntimeInstallation>()
        .loaded_fonts
        .extend(fingerprints);
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
    styles: &Path,
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
                && path != styles
                && path != root.join(TOKEN_BASE_FILE)
                && path != root.join(PROFILE_FILE)
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
    overrides: &ThemeLayers,
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
            let variant =
                load_theme_with_overrides(engine, &path.to_string_lossy(), &source, overrides)?;
            insert_theme_variant(&mut variants, variant)?;
        }
    }
    theme_manager(variants.into_values(), primary)
}

/// The token layers beneath and above every palette theme of one view.
#[derive(Clone, Debug, Default)]
pub(crate) struct ThemeLayers {
    base: Option<crate::TokenLayer>,
    overrides: ThemeTokenOverrides,
}

/// The token base of a file application: the builder source, else
/// `ui/tokens.rhai` when present.
fn file_theme_layers(
    engine: &rhai::Engine,
    entry: &Path,
    builder_source: Option<&str>,
    overrides: &ThemeTokenOverrides,
) -> Result<ThemeLayers, ScriptViewError> {
    let path = entry
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(TOKEN_BASE_FILE);
    let base = match builder_source {
        Some(source) => Some((String::from("<token-base>"), source.to_owned())),
        None if path.exists() => Some((
            path.to_string_lossy().into_owned(),
            fs::read_to_string(&path).map_err(|source| ScriptViewError::Io {
                path: path.clone(),
                source,
            })?,
        )),
        None => None,
    };
    Ok(ThemeLayers {
        base: base
            .map(|(name, source)| {
                crate::load_token_base(engine, &name, &source)
                    .map_err(|error| ScriptViewError::Theme(error.to_string()))
            })
            .transpose()?,
        overrides: overrides.clone(),
    })
}

fn load_file_profile(
    engine: &rhai::Engine,
    ui_root: &Path,
) -> Result<crate::Profile, ScriptViewError> {
    let path = ui_root.join(PROFILE_FILE);
    if !path.exists() {
        return Ok(crate::Profile::default());
    }
    let source = fs::read_to_string(&path).map_err(|source| ScriptViewError::Io {
        path: path.clone(),
        source,
    })?;
    crate::load_profile_source(engine, &path.to_string_lossy(), &source)
        .map_err(ScriptViewError::Extension)
}

fn embedded_theme_layers(
    engine: &rhai::Engine,
    token_base: Option<&str>,
    overrides: &ThemeTokenOverrides,
) -> Result<ThemeLayers, ScriptViewError> {
    Ok(ThemeLayers {
        base: token_base
            .map(|source| {
                crate::load_token_base(engine, "<embedded-tokens>", source)
                    .map_err(|error| ScriptViewError::Theme(error.to_string()))
            })
            .transpose()?,
        overrides: overrides.clone(),
    })
}

/// File name of the token base inside the UI root.
pub(crate) const TOKEN_BASE_FILE: &str = "tokens.rhai";
/// File name of the profile configuration inside the UI root.
pub(crate) const PROFILE_FILE: &str = "profile.rhai";

fn load_theme_with_overrides(
    engine: &rhai::Engine,
    source_name: &str,
    source: &str,
    layers: &ThemeLayers,
) -> Result<ThemeVariant, ScriptViewError> {
    crate::load_theme_with_layers(
        engine,
        layers.base.as_ref(),
        source_name,
        source,
        &layers.overrides,
    )
    .map_err(|error| ScriptViewError::Theme(error.to_string()))
}

fn validated_themes(
    themes: ThemeManager,
    exports: &crate::ComponentRegistry,
) -> Result<ThemeManager, ScriptViewError> {
    validate_component_tokens(&themes, exports)?;
    Ok(themes)
}

/// Check every loaded theme against the tokens declared by components.
fn validate_component_tokens(
    themes: &ThemeManager,
    exports: &crate::ComponentRegistry,
) -> Result<(), ScriptViewError> {
    for variant in themes.variants() {
        for (_, definition) in exports.iter() {
            let metadata = &definition.metadata;
            let environment = metadata
                .environment
                .iter()
                .map(|name| format!("environment.{name}"))
                .collect::<Vec<_>>();
            variant
                .require(
                    metadata.id.as_str(),
                    metadata
                        .tokens
                        .iter()
                        .chain(&environment)
                        .map(String::as_str),
                )
                .map_err(|error| ScriptViewError::Theme(error.to_string()))?;
        }
    }
    Ok(())
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
    component_renderers: BTreeMap<ModuleId, crate::engine::RegisteredComponentRender>,
}

struct ScriptWindowFactory {
    program: RefCell<WindowProgram>,
    runtime: Rc<RefCell<UiRuntimeState>>,
    extensions: Rc<Vec<Box<dyn ScriptViewExtension>>>,
    theme: ThemeVariant,
    #[cfg(feature = "dev-reload")]
    theme_layers: RefCell<ThemeLayers>,
    show_error_banner: Cell<bool>,
    execution_policy: ScriptExecutionPolicy,
    #[cfg(feature = "dev-reload")]
    development: bool,
}

impl ScriptWindowFactory {
    fn instantiate(
        &self,
        view_id: &str,
        window_id: &str,
        mount_lease: Option<&std::rc::Weak<()>>,
    ) -> Result<(RuntimeEngine, ScriptLifecycle, PrimitiveRegistry), ScriptViewError> {
        let mut engine = RuntimeEngine::new();
        self.execution_policy.apply(&mut engine);
        for extension in self.extensions.iter() {
            extension
                .configure_engine(&mut engine)
                .map_err(ScriptViewError::Extension)?;
        }
        let program = self.program.borrow().clone();
        engine.restore_component_exports(program.component_exports.clone())?;
        engine.restore_component_renderers(program.component_renderers.clone())?;
        let lifecycle = self.mount_lifecycle(
            &mut engine,
            program,
            view_id,
            window_id,
            WindowCommandPolicy::ApplicationOwned,
            false,
            mount_lease,
        )?;
        let primitives = engine.primitive_registry();
        Ok((engine, lifecycle, primitives))
    }

    #[allow(clippy::too_many_arguments)]
    fn mount_lifecycle(
        &self,
        engine: &mut RuntimeEngine,
        program: WindowProgram,
        view_id: &str,
        window_id: &str,
        policy: WindowCommandPolicy,
        register_window: bool,
        mount_lease: Option<&std::rc::Weak<()>>,
    ) -> Result<ScriptLifecycle, ScriptViewError> {
        {
            let mut runtime = self.runtime.borrow_mut();
            if register_window {
                runtime
                    .windows
                    .register_open_for_view(window_id, policy, view_id)
                    .map_err(|error| ScriptViewError::Extension(error.to_string()))?;
            }
            if let Some(lease) = mount_lease {
                runtime
                    .windows
                    .qualify_mount(window_id, lease)
                    .map_err(|error| ScriptViewError::Extension(error.to_string()))?;
            }
            for extension in self.extensions.iter() {
                extension
                    .configure_window(window_id, &mut runtime)
                    .map_err(ScriptViewError::Extension)?;
            }
        }
        let mut lifecycle = ScriptLifecycle::new(
            program.compiled,
            Rc::clone(&self.runtime),
            ComponentInstancePath::root("View", view_id),
            Some(window_id.to_owned()),
            BTreeMap::new(),
            &program.state_schema,
        )?
        .with_view_id(view_id);
        if let Err(error) = lifecycle.start(engine) {
            let message = error.to_string();
            if let crate::LifecycleError::Runtime(runtime_error) = &error
                && let Ok(context) = lifecycle.diagnostic_context(engine, None)
            {
                return Err(ScriptViewError::ScriptDiagnostic {
                    message,
                    diagnostic: Box::new(Diagnostic::from_runtime(runtime_error, &context)),
                });
            }
            return Err(ScriptViewError::Lifecycle(error));
        }
        Ok(lifecycle)
    }

    #[cfg(feature = "dev-reload")]
    fn update_program(
        &self,
        compiled: CompiledUi,
        state_schema: ComponentStateSchema,
        component_exports: ComponentRegistry,
        component_renderers: BTreeMap<ModuleId, crate::engine::RegisteredComponentRender>,
    ) {
        *self.program.borrow_mut() = WindowProgram {
            compiled,
            state_schema,
            component_exports,
            component_renderers,
        };
    }

    fn program(&self) -> WindowProgram {
        self.program.borrow().clone()
    }
}

#[derive(Default)]
struct NativeWindowRegistry {
    handles: BTreeMap<String, NativeWindowRegistration>,
    force_close: std::collections::BTreeSet<String>,
}

#[derive(Clone)]
struct NativeWindowRegistration {
    handle: AnyWindowHandle,
    authority: NativeWindowAuthority,
}

fn window_command_is_current(
    command: &crate::QueuedWindowCommand,
    windows: &crate::WindowCommandRegistry,
    cx: &App,
) -> bool {
    windows.is_current_target(command)
        && windows.is_current_origin(command)
        && (command.is_host_origin()
            || command.source_binding().is_some_and(|binding| {
                NativeWindowAuthority {
                    window: binding.window,
                    lease: binding.lease,
                }
                .is_current(cx)
            }))
}

pub struct PreparedScriptView {
    engine: RuntimeEngine,
    theme: ThemeVariant,
    entry: PathBuf,
    ui_root: PathBuf,
    theme_path: PathBuf,
    style_path: PathBuf,
    development: bool,
    factory: Rc<ScriptWindowFactory>,
    key_bindings: Vec<KeyBindingSpec>,
    fonts: Vec<crate::FontSource>,
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
            &Rc::new(RefCell::new(NativeWindowRegistry::default())),
            WindowCommandPolicy::Disabled,
            window,
            cx,
        )
    }

    /// Mount the single script command owner of an existing Rust-owned window.
    ///
    /// Unlike [`Self::mount`], this explicitly enables open/focus/close commands,
    /// registers the native window, and installs the script close interceptor.
    /// Rust still owns the window's root and layout. Other views on the same
    /// Host use ordinary `mount` and do not inherit this authority. This replaces
    /// the window's previous should-close callback; opt in only when delegating
    /// that policy to this view. Disposing the view does not close the window.
    ///
    /// # Errors
    ///
    /// Returns mount errors, [`ScriptViewError::WindowCommandOwner`] for an
    /// existing command owner, or [`ScriptViewError::WrongNativeWindow`] when
    /// the Host belongs to another native window.
    pub fn mount_window(
        self,
        config: ScriptViewConfig,
        host: ScriptViewHost,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<ScriptViewHandle, ScriptViewError> {
        self.mount_with_registry(
            config,
            host,
            &Rc::new(RefCell::new(NativeWindowRegistry::default())),
            WindowCommandPolicy::ApplicationOwned,
            window,
            cx,
        )
    }

    #[allow(clippy::too_many_lines)]
    fn mount_with_registry(
        mut self,
        config: ScriptViewConfig,
        host: ScriptViewHost,
        native_windows: &Rc<RefCell<NativeWindowRegistry>>,
        policy: WindowCommandPolicy,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<ScriptViewHandle, ScriptViewError> {
        install(cx);
        install_declared_fonts(std::mem::take(&mut self.fonts), cx)?;
        #[cfg(feature = "dev-reload")]
        let watcher = development_watcher(self.development, &self.ui_root)?;
        let mount_lease = host.reserve_view(&config.view_id)?;
        let window_authority = host
            .bind_window(&config.view_id, window.window_handle(), policy, cx)
            .inspect_err(|_| {
                host.unregister_view(&config.view_id, &mount_lease);
            })?;
        let window_id = host.window_id();
        self.factory
            .runtime
            .borrow_mut()
            .update_window_appearance(&window_id, system_appearance(window.appearance()));
        let lifecycle = mount_prepared_lifecycle(
            &self.factory,
            &mut self.engine,
            &host,
            &config.view_id,
            &window_id,
            policy,
            &mount_lease,
        )?;
        if window_authority.is_some() {
            self.factory
                .runtime
                .borrow_mut()
                .windows
                .bind_native(&window_id, window.window_handle().window_id())
                .map_err(|error| ScriptViewError::Extension(error.to_string()))?;
        }
        #[cfg(not(feature = "dev-reload"))]
        let _ = (
            &self.entry,
            &self.ui_root,
            &self.theme_path,
            &self.style_path,
            self.development,
        );
        let primitives = self.engine.primitive_registry();
        let timings = self.engine.take_timings();
        let factory = Rc::clone(&self.factory);
        let overlays = host.overlays();
        let view_id = config.view_id.clone();
        let view_host = host.clone();
        let (theme_handle, view_theme_handle) =
            theme_handles_for_lifecycle(&lifecycle, &self.theme, window, cx);
        let view_state = Rc::new(Cell::new(ScriptViewState::Active));
        let entity_view_state = Rc::clone(&view_state);
        let entity_activity_wake = crate::async_runtime::AsyncWake::default();
        let entity = cx.new(|entity_cx| {
            let runtime_tasks = spawn_host_runtime_tasks(
                entity_cx,
                &lifecycle,
                Rc::clone(&entity_view_state),
                entity_activity_wake.clone(),
            );
            let host_focus = entity_cx.focus_handle();
            #[cfg(feature = "dev-reload")]
            let reload_task = watcher
                .as_ref()
                .map(|_| spawn_host_reload_poll(entity_cx, Rc::clone(&entity_view_state)));
            ScriptHostView {
                mount_lease: mount_lease.clone(),
                view_id: view_id.clone(),
                window_id,
                engine: self.engine,
                lifecycle,
                primitives,
                last_failure: None,
                theme: self.theme,
                theme_handle: view_theme_handle,
                #[cfg(feature = "dev-reload")]
                development: self.development,
                #[cfg(feature = "dev-reload")]
                inspector_open: false,
                timings,
                overlays,
                host: view_host,
                paint_background: config.paint_background,
                show_error_banner: config.show_error_banner,
                content_bounds: None,
                factory,
                native_windows: Rc::clone(native_windows),
                host_focus,
                focus_handles: BTreeMap::new(),
                scroll_handles: BTreeMap::new(),
                scroll_anchors: BTreeMap::new(),
                text_selection: crate::renderer::TextSelectionRegistry::default(),
                last_motion_sample: None,
                state: entity_view_state,
                direct_signal_access: Rc::new(Cell::new(false)),
                pending_interaction_cancel: false,
                activity_wake: entity_activity_wake,
                _runtime_tasks: runtime_tasks,
                window_activation: None,
                window_appearance: None,
                window_closed: None,
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
                style_path: self.style_path,
                #[cfg(feature = "dev-reload")]
                _reload_task: reload_task,
                #[cfg(feature = "dev-reload")]
                pending_reload_paths: BTreeSet::new(),
            }
        });
        install_window_lifecycle_hooks(&entity, window, cx);
        if let Some(authority) = window_authority {
            native_windows.borrow_mut().handles.insert(
                host.window_id(),
                NativeWindowRegistration {
                    handle: window.window_handle(),
                    authority,
                },
            );
            install_close_interceptor(window, cx, &entity);
        }
        attach_script_view_focus(&host, &config.view_id, &entity, cx);
        Ok(mounted_script_view_handle(
            entity,
            theme_handle,
            host,
            config,
            view_state,
            mount_lease,
        ))
    }
}

#[cfg(feature = "dev-reload")]
fn development_watcher(
    development: bool,
    ui_root: &Path,
) -> Result<Option<FileWatcher>, ScriptViewError> {
    if development && !ui_root.as_os_str().is_empty() {
        Ok(Some(FileWatcher::new(ui_root)?))
    } else {
        Ok(None)
    }
}

type WindowOptionsConfigurator =
    Box<dyn FnOnce(WindowOptions, &mut App) -> WindowOptions + 'static>;

pub struct ScriptApplication {
    prepared: PreparedScriptView,
    window_size: (f32, f32),
    window_options: Option<WindowOptionsConfigurator>,
    show_error_banner: bool,
}

impl ScriptApplication {
    #[must_use]
    pub const fn new(prepared: PreparedScriptView) -> Self {
        Self {
            prepared,
            window_size: (720.0, 480.0),
            window_options: None,
            show_error_banner: true,
        }
    }

    #[must_use]
    pub const fn window_size(mut self, width: f32, height: f32) -> Self {
        self.window_size = (width, height);
        self
    }

    /// Control built-in runtime-error banners in standalone script windows.
    ///
    /// Applications that disable them must provide another visible surface for
    /// [`ScriptViewHandle::last_error`].
    #[must_use]
    pub const fn show_error_banner(mut self, show: bool) -> Self {
        self.show_error_banner = show;
        self
    }

    /// Configure the trusted standalone window without exposing native window
    /// authority to Rhai. The callback receives the centered default options.
    #[must_use]
    pub fn window_options(
        mut self,
        configure: impl FnOnce(WindowOptions, &mut App) -> WindowOptions + 'static,
    ) -> Self {
        self.window_options = Some(Box::new(configure));
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
        let window_options = self.window_options;
        let prepared = self.prepared;
        let show_error_banner = self.show_error_banner;
        prepared.factory.show_error_banner.set(show_error_banner);
        gpui_platform::application().run(move |cx: &mut App| {
            install(cx);
            let host = match ScriptViewHost::new("main", cx) {
                Ok(host) => host,
                Err(error) => {
                    report_fatal(&reported_error, error.to_string());
                    cx.quit();
                    return;
                }
            };
            if let Err(error) = host.bind_keys(prepared.key_bindings.clone(), cx) {
                report_fatal(&reported_error, error.to_string());
                cx.quit();
                return;
            }
            #[cfg(feature = "dev-reload")]
            cx.bind_keys([
                KeyBinding::new("cmd-alt-i", ToggleInspector, Some(HOST_KEY_CONTEXT)),
                KeyBinding::new("f12", ToggleInspector, Some(HOST_KEY_CONTEXT)),
            ]);
            let window_options = standalone_window_options(window_size, window_options, cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            let view_native_windows = Rc::clone(&native_windows);
            let mount_error = Rc::clone(&reported_error);
            let result = cx.open_window(window_options, move |window, cx| {
                let root = cx.new(|_| ScriptApplicationRoot {
                    host: host.clone(),
                    view: None,
                    error: None,
                });
                let weak_root = root.downgrade();
                window.defer(cx, move |window, cx| {
                    let view = prepared.mount_with_registry(
                        ScriptViewConfig::new("main")
                            .paint_background(true)
                            .show_error_banner(show_error_banner),
                        host,
                        &view_native_windows,
                        WindowCommandPolicy::ApplicationOwned,
                        window,
                        cx,
                    );
                    match view {
                        Ok(view) => {
                            let _ = view.focus(window, cx);
                            let _ = weak_root.update(cx, |root, cx| {
                                root.view = Some(view);
                                cx.notify();
                            });
                        }
                        Err(error) => {
                            let message = error.to_string();
                            report_fatal(&mount_error, message.clone());
                            let _ = weak_root.update(cx, |root, cx| {
                                root.error = Some(message);
                                cx.notify();
                            });
                            cx.defer(|cx| cx.quit());
                        }
                    }
                });
                root
            });
            match result {
                Ok(_handle) => {
                    cx.activate(true);
                }
                Err(error) => {
                    report_fatal(&reported_error, error.to_string());
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

/// Record a fatal startup error. Platforms such as macOS terminate the process
/// on quit, so `run` may never return it; the message is also written to stderr.
fn report_fatal(slot: &RefCell<Option<String>>, message: String) {
    eprintln!("gpui-rhai: {message}");
    *slot.borrow_mut() = Some(message);
}

fn standalone_window_options(
    window_size: (f32, f32),
    configure: Option<WindowOptionsConfigurator>,
    cx: &mut App,
) -> WindowOptions {
    let bounds = Bounds::centered(None, size(px(window_size.0), px(window_size.1)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..WindowOptions::default()
    };
    match configure {
        Some(configure) => configure(options, cx),
        None => options,
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
) -> Result<(), ScriptFailure> {
    let factory = Rc::clone(factory);
    let native_windows = Rc::clone(native_windows);
    let window_id = spec.id.clone();
    let host = ScriptViewHost::new(window_id.clone(), cx).map_err(ScriptFailure::from)?;
    let mount_lease = host.reserve_view(&window_id).map_err(ScriptFailure::from)?;
    factory
        .runtime
        .borrow_mut()
        .windows
        .qualify_mount(&window_id, &mount_lease)
        .map_err(|error| ScriptFailure::plain(error.to_string()))?;
    let startup_error = Rc::new(RefCell::new(None));
    let view_startup_error = Rc::clone(&startup_error);
    let options = script_window_options(spec, cx);
    let view_factory = Rc::clone(&factory);
    let view_native_windows = Rc::clone(&native_windows);
    let view_window_id = window_id.clone();
    let view_host = host.clone();
    let view_mount_lease = mount_lease.clone();
    let result = cx.open_window(options, move |window, cx| {
        let window_authority = view_host
            .bind_window(
                &view_window_id,
                window.window_handle(),
                WindowCommandPolicy::ApplicationOwned,
                cx,
            )
            .expect("fresh secondary Host has no native window or command owner");
        view_factory
            .runtime
            .borrow_mut()
            .windows
            .bind_native(&view_window_id, window.window_handle().window_id())
            .expect("secondary reservation keeps its qualified mount identity");
        view_factory
            .runtime
            .borrow_mut()
            .update_window_appearance(&view_window_id, system_appearance(window.appearance()));
        // Init/effects run only after the native appearance and the same
        // reserved mount are known. Never replay side-effecting init to repair
        // a guessed pre-native theme.
        let (engine, lifecycle, primitives) = match view_factory.instantiate(
            &view_window_id,
            &view_window_id,
            Some(&view_mount_lease),
        ) {
            Ok(instance) => instance,
            Err(error) => {
                view_host.unregister_view(&view_window_id, &view_mount_lease);
                let root = ComponentInstancePath::root("View", &view_window_id);
                let _ = view_factory
                    .runtime
                    .borrow_mut()
                    .release_window(&view_window_id, &root);
                let failure = ScriptFailure::from(error);
                let message = failure.message.clone();
                *view_startup_error.borrow_mut() = Some(failure);
                return cx.new(|_| ScriptApplicationRoot {
                    host: view_host,
                    view: None,
                    error: Some(message),
                });
            }
        };
        let timings = engine.take_timings();
        let (theme_handle, view_theme_handle) =
            theme_handles_for_lifecycle(&lifecycle, &view_factory.theme, window, cx);
        let view_state = Rc::new(Cell::new(ScriptViewState::Active));
        let entity_view_state = Rc::clone(&view_state);
        let entity_activity_wake = crate::async_runtime::AsyncWake::default();
        let entity = cx.new(|entity_cx| {
            let runtime_tasks = spawn_host_runtime_tasks(
                entity_cx,
                &lifecycle,
                Rc::clone(&entity_view_state),
                entity_activity_wake.clone(),
            );
            let host_focus = entity_cx.focus_handle();
            ScriptHostView {
                mount_lease: view_mount_lease.clone(),
                view_id: view_window_id.clone(),
                window_id: view_window_id.clone(),
                engine,
                lifecycle,
                primitives,
                last_failure: None,
                theme: view_factory.theme.clone(),
                theme_handle: view_theme_handle,
                #[cfg(feature = "dev-reload")]
                development: view_factory.development,
                #[cfg(feature = "dev-reload")]
                inspector_open: false,
                timings,
                overlays: view_host.overlays(),
                host: view_host.clone(),
                paint_background: true,
                show_error_banner: view_factory.show_error_banner.get(),
                content_bounds: None,
                factory: Rc::clone(&view_factory),
                native_windows: Rc::clone(&view_native_windows),
                host_focus,
                focus_handles: BTreeMap::new(),
                scroll_handles: BTreeMap::new(),
                scroll_anchors: BTreeMap::new(),
                text_selection: crate::renderer::TextSelectionRegistry::default(),
                last_motion_sample: None,
                state: entity_view_state,
                direct_signal_access: Rc::new(Cell::new(false)),
                pending_interaction_cancel: false,
                activity_wake: entity_activity_wake,
                _runtime_tasks: runtime_tasks,
                window_activation: None,
                window_appearance: None,
                window_closed: None,
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
                style_path: PathBuf::new(),
                #[cfg(feature = "dev-reload")]
                _reload_task: None,
                #[cfg(feature = "dev-reload")]
                pending_reload_paths: BTreeSet::new(),
            }
        });
        install_window_lifecycle_hooks(&entity, window, cx);
        if let Some(authority) = window_authority {
            view_native_windows.borrow_mut().handles.insert(
                view_window_id.clone(),
                NativeWindowRegistration {
                    handle: window.window_handle(),
                    authority,
                },
            );
        }
        view_host.attach_view_focus(&view_window_id, entity.read(cx).host_focus.clone());
        let view = ScriptViewHandle(Rc::new(ScriptViewHandleInner {
            entity: entity.clone(),
            theme: theme_handle,
            host: view_host.clone(),
            view_id: view_window_id,
            state: view_state,
            measured_bounds: Rc::new(Cell::new(None)),
            mount_lease: view_mount_lease,
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
            if let Some(error) = startup_error.borrow_mut().take() {
                let _ = handle.update(cx, |_, window, _| window.remove_window());
                return Err(error);
            }
            factory
                .runtime
                .borrow_mut()
                .windows
                .mark_open(&window_id)
                .map_err(|error| ScriptFailure::plain(error.to_string()))?;
            Ok(())
        }
        Err(error) => {
            host.unregister_view(&window_id, &mount_lease);
            let root = ComponentInstancePath::root("View", &window_id);
            let mut runtime = factory.runtime.borrow_mut();
            let _ = runtime.release_window(&window_id, &root);
            Err(ScriptFailure::plain(error.to_string()))
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

struct HostRuntimeTasks {
    _frame_poll: Task<()>,
    _task_delivery: Task<()>,
    _subscription_delivery: Task<()>,
    _virtual_delivery: Task<()>,
}

fn spawn_host_runtime_tasks(
    cx: &mut Context<ScriptHostView>,
    lifecycle: &ScriptLifecycle,
    state: Rc<Cell<ScriptViewState>>,
    activity_wake: crate::async_runtime::AsyncWake,
) -> HostRuntimeTasks {
    let (task_wake, subscription_wake, virtual_wake) = {
        let runtime = lifecycle.runtime();
        let runtime = runtime.borrow();
        (
            runtime.tasks.wake(),
            runtime.subscriptions.wake(),
            runtime.virtual_requests.wake(),
        )
    };
    HostRuntimeTasks {
        _frame_poll: spawn_host_frame_poll(cx, state, activity_wake),
        _task_delivery: spawn_host_delivery_pump(cx, task_wake),
        _subscription_delivery: spawn_host_delivery_pump(cx, subscription_wake),
        _virtual_delivery: spawn_host_delivery_pump(cx, virtual_wake),
    }
}

fn spawn_host_frame_poll(
    cx: &mut Context<ScriptHostView>,
    state: Rc<Cell<ScriptViewState>>,
    activity_wake: crate::async_runtime::AsyncWake,
) -> Task<()> {
    let mut listener = activity_wake.listen();
    cx.spawn(async move |entity: gpui::WeakEntity<ScriptHostView>, cx| {
        loop {
            match state.get() {
                ScriptViewState::Disposed => break,
                ScriptViewState::Suspended => {
                    listener.await;
                    listener = activity_wake.listen();
                    continue;
                }
                ScriptViewState::Active => {}
            }
            cx.background_executor()
                .timer(Duration::from_millis(16))
                .await;
            if entity.update(cx, ScriptHostView::poll_async).is_err() {
                break;
            }
        }
    })
}

fn spawn_host_delivery_pump(
    cx: &mut Context<ScriptHostView>,
    wake: crate::async_runtime::AsyncWake,
) -> Task<()> {
    // Register before the entity becomes externally visible. The first drain
    // catches work that completed during lifecycle initialization; thereafter
    // the listener closes the send/drain race without fixed-rate polling.
    let mut listener = wake.listen();
    cx.spawn(async move |entity: gpui::WeakEntity<ScriptHostView>, cx| {
        loop {
            if entity.update(cx, ScriptHostView::poll_async).is_err() {
                break;
            }
            listener.await;
            listener = wake.listen();
        }
    })
}

#[cfg(feature = "dev-reload")]
fn spawn_host_reload_poll(
    cx: &mut Context<ScriptHostView>,
    state: Rc<Cell<ScriptViewState>>,
) -> Task<()> {
    cx.spawn(async move |entity: gpui::WeakEntity<ScriptHostView>, cx| {
        loop {
            if state.get() == ScriptViewState::Disposed {
                break;
            }
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
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

fn event_response_from_dynamic(value: &rhai::Dynamic) -> crate::EventResponse {
    if value.is::<crate::EventResponse>() {
        value.clone_cast::<crate::EventResponse>()
    } else if value.is::<rhai::ImmutableString>()
        && value.clone_cast::<rhai::ImmutableString>().as_str() == "propagate"
    {
        crate::EventResponse::new()
    } else {
        crate::EventResponse::new().stop()
    }
}

fn automation_pointer_id(payload: &UiValue) -> Option<u64> {
    let UiValue::Map(payload) = payload else {
        return None;
    };
    match payload.get("pointer_id") {
        Some(UiValue::Integer(id)) => u64::try_from(*id).ok(),
        _ => None,
    }
}

#[derive(Clone, Debug)]
struct ScriptFailure {
    message: String,
    diagnostic: Option<Box<Diagnostic>>,
}

impl ScriptFailure {
    fn plain(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            diagnostic: None,
        }
    }

    fn with_diagnostic(message: impl Into<String>, diagnostic: Diagnostic) -> Self {
        Self {
            message: message.into(),
            diagnostic: Some(Box::new(diagnostic)),
        }
    }

    fn append_context(mut self, context: impl AsRef<str>) -> Self {
        self.message = format!("{}; {}", self.message, context.as_ref());
        self
    }

    fn into_view_error(self) -> ScriptViewError {
        match self.diagnostic {
            Some(diagnostic) => ScriptViewError::ScriptDiagnostic {
                message: self.message,
                diagnostic,
            },
            None => ScriptViewError::Resume(self.message),
        }
    }
}

impl From<String> for ScriptFailure {
    fn from(message: String) -> Self {
        Self::plain(message)
    }
}

#[allow(clippy::struct_excessive_bools)]
struct ScriptHostView {
    mount_lease: std::rc::Weak<()>,
    view_id: String,
    window_id: String,
    engine: RuntimeEngine,
    lifecycle: ScriptLifecycle,
    primitives: PrimitiveRegistry,
    last_failure: Option<ScriptFailure>,
    theme: ThemeVariant,
    theme_handle: ThemeHandle,
    #[cfg(feature = "dev-reload")]
    development: bool,
    #[cfg(feature = "dev-reload")]
    inspector_open: bool,
    timings: Vec<crate::ExecutionTiming>,
    overlays: WindowOverlayCoordinator,
    host: ScriptViewHost,
    paint_background: bool,
    show_error_banner: bool,
    content_bounds: Option<Bounds<Pixels>>,
    factory: Rc<ScriptWindowFactory>,
    native_windows: Rc<RefCell<NativeWindowRegistry>>,
    host_focus: FocusHandle,
    focus_handles: BTreeMap<crate::NodeId, FocusHandle>,
    scroll_handles: BTreeMap<crate::NodeId, ScrollHandle>,
    scroll_anchors: BTreeMap<crate::NodeId, ScrollAnchor>,
    text_selection: crate::renderer::TextSelectionRegistry,
    last_motion_sample: Option<Instant>,
    state: Rc<Cell<ScriptViewState>>,
    direct_signal_access: Rc<Cell<bool>>,
    pending_interaction_cancel: bool,
    activity_wake: crate::async_runtime::AsyncWake,
    _runtime_tasks: HostRuntimeTasks,
    #[allow(dead_code)]
    window_activation: Option<Subscription>,
    window_appearance: Option<Subscription>,
    #[allow(dead_code)]
    window_closed: Option<Subscription>,
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
    style_path: PathBuf,
    #[cfg(feature = "dev-reload")]
    _reload_task: Option<Task<()>>,
    #[cfg(feature = "dev-reload")]
    pending_reload_paths: BTreeSet<PathBuf>,
}

fn install_window_lifecycle_hooks(
    entity: &Entity<ScriptHostView>,
    window: &mut Window,
    cx: &mut App,
) {
    let subscription = entity.update(cx, |_, entity_cx| {
        entity_cx.observe_window_activation(window, |view, window, cx| {
            if !window.is_window_active() && view.cancel_view_interaction(window, cx) {
                cx.notify();
            }
        })
    });
    entity.update(cx, |view, _| view.window_activation = Some(subscription));
    let appearance = entity.update(cx, |_, entity_cx| {
        entity_cx.observe_window_appearance(window, |view, window, cx| {
            view.on_window_appearance_changed(window.appearance(), cx);
        })
    });
    entity.update(cx, |view, _| view.window_appearance = Some(appearance));
    let window_id = window.window_handle().window_id();
    let weak = entity.downgrade();
    let closed = cx.on_window_closed(move |cx, closed_id| {
        if closed_id == window_id {
            let weak = weak.clone();
            cx.defer(move |cx| {
                let _ = weak.update(cx, |view, cx| {
                    view.release_view();
                    cx.notify();
                });
            });
        }
    });
    entity.update(cx, |view, _| view.window_closed = Some(closed));
}

struct DirectSignalAccessGuard {
    flag: Rc<Cell<bool>>,
    previous: bool,
}

#[derive(Clone)]
struct PrimitiveRuntimeReader {
    owner: gpui::WeakEntity<ScriptHostView>,
    runtime: Rc<RefCell<UiRuntimeState>>,
    view_id: String,
    lifecycle_access: Rc<Cell<bool>>,
}

impl PrimitiveRuntimeReader {
    fn read<T>(
        &self,
        app: &App,
        query: impl FnOnce(&UiRuntimeState, &str) -> Option<T>,
    ) -> Option<T> {
        if self.lifecycle_access.get() {
            query(&self.runtime.borrow(), &self.view_id)
        } else {
            self.owner
                .read_with(app, |view, _| {
                    query(&view.lifecycle.runtime().borrow(), &view.view_id)
                })
                .ok()
                .flatten()
        }
    }
}

impl Drop for DirectSignalAccessGuard {
    fn drop(&mut self) {
        self.flag.set(self.previous);
    }
}

fn nearest_scroll_ancestor(
    tree: &crate::RetainedUiTree,
    node: crate::NodeId,
) -> Option<crate::NodeId> {
    let mut current = tree.node(node)?.parent();
    while let Some(node) = current {
        let retained = tree.node(node)?;
        if retained.scrollable() {
            return Some(node);
        }
        current = retained.parent();
    }
    None
}

struct ScriptRenderSnapshot {
    now: Instant,
    clock: crate::RuntimeClock,
    motion_preference: crate::MotionPreference,
    motion_quality: crate::MotionQuality,
    assets: AssetRegistry,
    theme: ThemeVariant,
    motions: BTreeMap<crate::MotionKey, f64>,
    motion_ghosts: Vec<crate::motion::MotionGhost>,
    signals: crate::SignalRegistry,
    geometry: crate::GeometryRegistry,
    pointer_capture: crate::PointerCaptureRegistry,
    virtual_requests: crate::VirtualRequestRegistry,
    direction: TextDirection,
    locale: String,
    number: Option<crate::NumberMetadata>,
}

struct ScriptViewTransaction {
    runtime: crate::UiStateSnapshot,
    engine: crate::engine::RuntimeEngineCheckpoint,
    lifecycle: crate::lifecycle::ScriptLifecycleCheckpoint,
}

fn handle_tab_navigation(event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App) {
    if event.keystroke.key.as_str() == "tab" {
        if event.keystroke.modifiers.shift {
            window.focus_prev(cx);
        } else {
            window.focus_next(cx);
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
    // Themes need not use the semantic vocabulary; without it the host keeps
    // GPUI's default text color and paints no background.
    let mut root = div()
        .id("gpui-rhai-host")
        .size_full()
        .track_focus(host_focus)
        .key_context(HOST_KEY_CONTEXT)
        .on_key_down(handle_tab_navigation);
    if let Some(color) = theme.tokens.color("text_primary") {
        root = root.text_color(rgba(color.as_rgba_hex()));
    }
    match theme.tokens.color("surface") {
        Some(color) if paint_background => root.bg(rgba(color.as_rgba_hex())),
        _ => root,
    }
}

#[cfg(target_os = "macos")]
const fn error_banner_font_family() -> &'static str {
    "Menlo"
}

#[cfg(target_os = "windows")]
const fn error_banner_font_family() -> &'static str {
    "Consolas"
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const fn error_banner_font_family() -> &'static str {
    "DejaVu Sans Mono"
}

fn build_error_banner(
    view_id: &str,
    window_id: &str,
    error: &str,
    theme: &ThemeVariant,
    selection: crate::renderer::TextSelectionRegistry,
    host_focus: FocusHandle,
    content: AnyElement,
) -> AnyElement {
    let selector = format!("gpui-rhai-error-banner:{view_id}");
    let accessibility_id = selector.clone();
    let owner = format!("window:{window_id}/view:{view_id}/error-banner");
    // The banner is developer-facing and must render with any vocabulary.
    let color = |token: &str, fallback: u32| {
        theme
            .tokens
            .color(token)
            .unwrap_or(crate::Rgba8::from_rgba_hex(fallback))
    };
    let error_text = crate::renderer::selectable_text_element(
        owner,
        error,
        color("selection", 0x3a5f_cd66),
        selection,
        Some(host_focus),
    );
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .id(ElementId::Name(accessibility_id.into()))
                .role(Role::Alert)
                .aria_label("Script view error")
                .aria_description(error.to_owned())
                .debug_selector(move || selector.clone())
                .p_2()
                .font_family(error_banner_font_family())
                .text_size(px(12.0))
                .line_height(px(18.0))
                .bg(rgba(color("surface_raised", 0x1f1f_1fff).as_rgba_hex()))
                .text_color(rgba(color("danger", 0xff55_55ff).as_rgba_hex()))
                .border_1()
                .border_color(rgba(color("danger", 0xff55_55ff).as_rgba_hex()))
                .child(error_text),
        )
        .child(content)
        .into_any_element()
}

#[allow(clippy::too_many_lines)]
fn script_node_dispatcher(
    cx: &Context<ScriptHostView>,
    runtime: Rc<RefCell<UiRuntimeState>>,
    direct_signal_access: Rc<Cell<bool>>,
    view_id: &str,
) -> NodeEventDispatcher {
    let script_entity = cx.entity().downgrade();
    let native_entity = script_entity.clone();
    let signal_entity = script_entity.clone();
    let signal_read_entity = script_entity.clone();
    let signal_read_runtime = Rc::clone(&runtime);
    let signal_read_direct = Rc::clone(&direct_signal_access);
    let geometry_reader = PrimitiveRuntimeReader {
        owner: script_entity.clone(),
        runtime: Rc::clone(&runtime),
        view_id: view_id.to_owned(),
        lifecycle_access: Rc::clone(&direct_signal_access),
    };
    let canvas_geometry_reader = geometry_reader.clone();
    let canvas_bounds_reader = geometry_reader.clone();
    NodeEventDispatcher::new(move |callback, payload, target, window, app| {
        script_entity
            .update(app, |view, cx| {
                view.handle_node_event(&callback, payload, target, window, cx)
            })
            .unwrap_or_else(|_| crate::EventResponse::new().stop())
    })
    .with_native(move |handler, event, payload, target, window, app| {
        native_entity
            .update(app, |view, cx| {
                view.handle_native_event(&handler, event, payload, target, window, cx)
            })
            .unwrap_or_else(|_| crate::EventResponse::new().stop())
    })
    .with_signal_write(move |updates, app| {
        let stale = updates
            .first()
            .map(|(signal, _)| crate::SignalError::Stale(signal.id().clone()));
        let direct_runtime = Rc::clone(&runtime);
        if direct_signal_access.get() {
            return direct_runtime
                .try_borrow_mut()
                .map_err(|_| stale.expect("non-empty signal patches have a first member"))?
                .signals
                .write_batch_from(&updates, crate::SignalWriter::Primitive);
        }
        signal_entity
            .update(app, |view, cx| {
                let changed = view
                    .lifecycle
                    .runtime()
                    .borrow_mut()
                    .signals
                    .write_batch_from(&updates, crate::SignalWriter::Primitive)?;
                if changed && view.state.get() == ScriptViewState::Active {
                    cx.notify();
                }
                Ok(changed)
            })
            .unwrap_or_else(|_| Err(stale.expect("non-empty signal patches have a first member")))
    })
    .with_signal_read(move |signal, app| {
        if signal_read_direct.get() {
            return signal_read_runtime.borrow().signals.read(signal);
        }
        signal_read_entity
            .read_with(app, |view, _| {
                view.lifecycle.runtime().borrow().signals.read(signal)
            })
            .unwrap_or_else(|_| Err(crate::SignalError::Stale(signal.id().clone())))
    })
    .with_element_bounds(move |reference, app| {
        geometry_reader.read(app, |runtime, view_id| {
            let node = runtime.element_refs.resolve(reference).ok()?;
            runtime
                .geometry_for(Some(view_id))
                .presented(node)
                .ok()
                .map(|geometry| geometry.layout)
        })
    })
    .with_canvas_local_point(move |reference, point, app| {
        canvas_geometry_reader.read(app, |runtime, view_id| {
            let node = runtime.element_refs.resolve(reference).ok()?;
            let geometry = runtime.geometry_for(Some(view_id));
            let bounds = geometry.canvas_drawable(node)?;
            let local = (point.0 - bounds.visual.x, point.1 - bounds.visual.y);
            Some(
                crate::canvas::canvas_motion_affine(
                    bounds.layout.width,
                    bounds.layout.height,
                    geometry.canvas_transform(node),
                )
                .inverse()?
                .map_point(local),
            )
        })
    })
    .with_canvas_bounds(move |reference, app| {
        canvas_bounds_reader.read(app, |runtime, view_id| {
            let node = runtime.element_refs.resolve(reference).ok()?;
            runtime
                .geometry_for(Some(view_id))
                .canvas_drawable(node)
                .map(|bounds| bounds.layout)
        })
    })
}

const fn system_appearance(appearance: WindowAppearance) -> SystemAppearance {
    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => SystemAppearance::Dark,
        WindowAppearance::Light | WindowAppearance::VibrantLight => SystemAppearance::Light,
    }
}

fn resolve_root_theme(
    lifecycle: &ScriptLifecycle,
    fallback: &ThemeVariant,
    appearance: SystemAppearance,
) -> ThemeVariant {
    let runtime = lifecycle.runtime();
    resolve_root_theme_from_runtime(
        &runtime.borrow(),
        lifecycle.root_path(),
        lifecycle.window_id().unwrap_or_default(),
        appearance,
        fallback,
    )
}

fn theme_handles_for_lifecycle(
    lifecycle: &ScriptLifecycle,
    fallback: &ThemeVariant,
    window: &Window,
    cx: &mut App,
) -> (ThemeHandle, ThemeHandle) {
    let handle = ThemeHandle::new(
        resolve_root_theme(lifecycle, fallback, system_appearance(window.appearance())),
        cx,
    );
    (handle.clone(), handle)
}

fn attach_script_view_focus(
    host: &ScriptViewHost,
    view_id: &str,
    entity: &Entity<ScriptHostView>,
    cx: &App,
) {
    host.attach_view_focus(view_id, entity.read(cx).host_focus.clone());
}

fn mounted_script_view_handle(
    entity: Entity<ScriptHostView>,
    theme: ThemeHandle,
    host: ScriptViewHost,
    config: ScriptViewConfig,
    state: Rc<Cell<ScriptViewState>>,
    mount_lease: std::rc::Weak<()>,
) -> ScriptViewHandle {
    ScriptViewHandle(Rc::new(ScriptViewHandleInner {
        entity,
        theme,
        host,
        view_id: config.view_id,
        state,
        measured_bounds: Rc::new(Cell::new(None)),
        mount_lease,
    }))
}

fn mount_prepared_lifecycle(
    factory: &ScriptWindowFactory,
    engine: &mut RuntimeEngine,
    host: &ScriptViewHost,
    view_id: &str,
    window_id: &str,
    policy: WindowCommandPolicy,
    lease: &std::rc::Weak<()>,
) -> Result<ScriptLifecycle, ScriptViewError> {
    factory
        .mount_lifecycle(
            engine,
            factory.program(),
            view_id,
            window_id,
            policy,
            true,
            Some(lease),
        )
        .inspect_err(|_| {
            host.unregister_view(view_id, lease);
            factory.runtime.borrow_mut().windows.remove(window_id);
        })
}

fn resolve_root_theme_from_runtime(
    runtime: &UiRuntimeState,
    root: &ComponentInstancePath,
    window_id: &str,
    appearance: SystemAppearance,
    fallback: &ThemeVariant,
) -> ThemeVariant {
    runtime
        .theme
        .as_ref()
        .and_then(|themes| themes.resolve(Some(window_id), Some(root), appearance).ok())
        .map_or_else(|| fallback.clone(), |resolved| resolved.variant().clone())
}

impl Render for ScriptHostView {
    #[allow(clippy::too_many_lines)]
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.state.get() != ScriptViewState::Active {
            return div().into_any_element();
        }
        self.consume_pending_interaction_cancel(window, cx);
        self.prepare_host_render(window, cx);
        let motion_root = format!("window:{}/view:{}/root", self.window_id, self.view_id);
        let (motion_active, committed_motion_events) = self.sample_motion_frame(&motion_root);
        let dispatcher = script_node_dispatcher(
            cx,
            self.lifecycle.runtime(),
            Rc::clone(&self.direct_signal_access),
            &self.view_id,
        );
        let appearance = system_appearance(window.appearance());
        let snapshot = self.render_snapshot(appearance);
        self.publish_theme_after_render(&snapshot.theme, cx);
        let a11y_active = window.is_a11y_active();
        let interactions = self.host.interactions();
        let mut semantics = self.lifecycle.semantics().clone();
        if a11y_active {
            semantics.apply_primitive_projections(
                self.primitives
                    .accessibility_projections(self.lifecycle.retained(), cx),
            );
        }
        let focus_path = self.focus_path(window);
        let render_resources = crate::renderer::WindowRenderResources {
            now: snapshot.now,
            clock: &snapshot.clock,
            motion_preference: snapshot.motion_preference,
            motion_quality: snapshot.motion_quality,
            assets: &snapshot.assets,
            dispatcher: &dispatcher,
            overlays: &self.overlays,
            interactions: &interactions,
            motions: &snapshot.motions,
            signals: &snapshot.signals,
            geometry: &snapshot.geometry,
            pointer_capture: &snapshot.pointer_capture,
            focus_handles: &self.focus_handles,
            scroll_handles: &self.scroll_handles,
            scroll_anchors: &self.scroll_anchors,
            virtual_requests: &snapshot.virtual_requests,
            text_selection: &self.text_selection,
            host_focus: Some(&self.host_focus),
            direction: snapshot.direction,
            locale: &snapshot.locale,
            number: snapshot.number.as_ref(),
            ambient_text_color: None,
            environment: crate::Environment::EMPTY,
            inherited_disabled: false,
            focus_path: &focus_path,
            owner_focused: false,
            root_path: &motion_root,
            view_id: &self.view_id,
            semantics: &semantics,
            a11y_active,
        };
        let content = self.lifecycle.retained().root().map_or_else(
            || div().child("Script view has no root").into_any_element(),
            |_| {
                GpuiNodeRenderer::render_retained_with_window_runtime(
                    self.lifecycle.retained(),
                    &snapshot.theme,
                    &InteractionState::default(),
                    &self.primitives,
                    &render_resources,
                )
            },
        );
        let content = match (&self.last_failure, self.show_error_banner) {
            (Some(failure), true) => build_error_banner(
                &self.view_id,
                &self.window_id,
                &failure.message,
                &snapshot.theme,
                self.text_selection.clone(),
                self.host_focus.clone(),
                content,
            ),
            _ => content,
        };
        let motion_ghosts = snapshot
            .motion_ghosts
            .iter()
            .map(|ghost| {
                crate::renderer::render_motion_ghost(
                    ghost,
                    &snapshot.theme,
                    &self.primitives,
                    &render_resources,
                )
            })
            .collect::<Vec<_>>();
        #[cfg(feature = "dev-reload")]
        let runtime = self.lifecycle.runtime();
        #[cfg(feature = "dev-reload")]
        let inspector = self.inspector_element(&runtime, &snapshot.theme);
        let text_selection = self.text_selection.clone();
        let root = build_host_root(&self.host_focus, &snapshot.theme, self.paint_background)
            .on_mouse_down(MouseButton::Left, move |event, window, _| {
                if text_selection.clear_outside(event.position) {
                    window.refresh();
                }
            })
            .child(content)
            .children(motion_ghosts)
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
        let root = apply_root_text_direction(root, snapshot.direction);
        let root = root
            .on_action(cx.listener(Self::dispatch_key_binding))
            .on_action(cx.listener(Self::copy_selected_text));
        #[cfg(feature = "dev-reload")]
        let root = root.on_action(cx.listener(Self::toggle_inspector));
        let has_committed_motion_events = !committed_motion_events.is_empty();
        cx.on_next_frame(window, move |view, _, cx| {
            view.lifecycle
                .runtime()
                .borrow()
                .geometry_for(Some(&view.view_id))
                .finish_frame();
            view.deliver_committed_motion_events(committed_motion_events, cx);
        });
        if motion_active || has_committed_motion_events {
            window.request_animation_frame();
        }
        crate::renderer::pointer_capture_router_element(
            root.into_any_element(),
            &self.view_id,
            self.lifecycle.retained(),
            &dispatcher,
            &snapshot.pointer_capture,
            &snapshot.geometry,
            &self.scroll_handles,
            interactions.clone(),
        )
    }
}

fn native_lifecycle_error(
    operation: &str,
    error: &crate::PrimitiveError,
    rollback: Option<&crate::PrimitiveError>,
) -> String {
    rollback.map_or_else(
        || format!("native primitive {operation} failed: {error}"),
        |rollback| {
            format!("native primitive {operation} failed: {error}; compensation failed: {rollback}")
        },
    )
}

impl ScriptHostView {
    fn on_window_appearance_changed(
        &mut self,
        appearance: WindowAppearance,
        cx: &mut Context<Self>,
    ) {
        if self.state.get() == ScriptViewState::Disposed {
            return;
        }
        let changed = self
            .lifecycle
            .runtime()
            .borrow_mut()
            .update_window_appearance(&self.window_id, system_appearance(appearance));
        if changed && self.state.get() == ScriptViewState::Active {
            // Ingest before scheduling normal transactional work. Do not run
            // Rhai or effects inside GPUI's appearance observer delivery.
            // Defer with a weak entity; poll_async rechecks current lifecycle.
            let owner = cx.weak_entity();
            cx.defer(move |cx| {
                let _ = owner.update(cx, Self::poll_async);
            });
            cx.notify();
        }
        // Suspended views retain the new environment/dirty owners for resume;
        // no script, effect or animation is activated by this notification.
    }

    fn sample_motion_frame(&mut self, domain: &str) -> (bool, Vec<crate::MotionTimelineEvent>) {
        let runtime = self.lifecycle.runtime();
        let mut runtime = runtime.borrow_mut();
        let now = runtime.clock.now();
        let clock_advanced = self.last_motion_sample != Some(now);
        self.last_motion_sample = Some(now);
        let (frame, events) = sample_motion_domain(&mut runtime, domain, now);
        (clock_advanced && frame.needs_frame, events)
    }

    fn deliver_committed_motion_events(
        &mut self,
        events: Vec<crate::MotionTimelineEvent>,
        cx: &mut Context<Self>,
    ) {
        match self.state.get() {
            ScriptViewState::Suspended => {
                self.lifecycle
                    .runtime()
                    .borrow_mut()
                    .motions
                    .prepend_timeline_events(events);
                return;
            }
            ScriptViewState::Disposed => return,
            ScriptViewState::Active => {}
        }
        let mut changed = false;
        let mut first_error = None;
        for event in events {
            if event.callback.is_none() {
                continue;
            }
            let result = self.run_script_transaction(|view| {
                let callback_changed = view.invoke_motion_timeline_callback(event)?;
                let mut work_changed = view.invoke_pending_effects()?;
                work_changed |= view
                    .lifecycle
                    .render_dirty(&mut view.engine)
                    .map_err(|error| view.lifecycle_failure(&error, None))?;
                Ok(callback_changed || work_changed)
            });
            match result {
                Ok(event_changed) => changed |= event_changed,
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        if let Some(error) = first_error {
            self.set_failure(error);
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    fn lifecycle_failure(
        &self,
        error: &crate::LifecycleError,
        component: Option<&ComponentInstancePath>,
    ) -> ScriptFailure {
        self.lifecycle_failure_with_engine(&self.engine, error, component)
    }

    fn lifecycle_failure_with_engine(
        &self,
        engine: &RuntimeEngine,
        error: &crate::LifecycleError,
        component: Option<&ComponentInstancePath>,
    ) -> ScriptFailure {
        let message = error.to_string();
        if let crate::LifecycleError::Runtime(runtime_error) = error
            && let Ok(context) = self.lifecycle.diagnostic_context(engine, component)
        {
            return ScriptFailure::with_diagnostic(
                message,
                Diagnostic::from_runtime(runtime_error, &context),
            );
        }
        ScriptFailure::plain(message)
    }

    #[cfg(feature = "dev-reload")]
    fn runtime_failure_with_engine(
        &self,
        engine: &RuntimeEngine,
        error: &RuntimeError,
        component: Option<&ComponentInstancePath>,
    ) -> ScriptFailure {
        let message = error.to_string();
        match self.lifecycle.diagnostic_context(engine, component) {
            Ok(context) => {
                ScriptFailure::with_diagnostic(message, Diagnostic::from_runtime(error, &context))
            }
            Err(_) => ScriptFailure::plain(message),
        }
    }

    fn set_failure(&mut self, failure: ScriptFailure) {
        self.last_failure = Some(failure);
    }

    fn set_plain_failure(&mut self, message: impl Into<String>) {
        self.set_failure(ScriptFailure::plain(message));
    }

    fn clear_failure(&mut self) {
        self.last_failure = None;
    }

    fn failure_message(&self) -> Option<String> {
        self.last_failure
            .as_ref()
            .map(|failure| failure.message.clone())
    }

    fn publish_theme_after_render(&self, theme: &ThemeVariant, cx: &mut Context<Self>) {
        if self.theme_handle.matches(theme, cx) {
            return;
        }
        let handle = self.theme_handle.clone();
        let theme = theme.clone();
        cx.defer(move |cx| handle.publish(theme, cx));
    }

    fn execute_automation(
        &mut self,
        command: crate::AutomationCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<crate::AutomationResult, ScriptViewError> {
        match command {
            crate::AutomationCommand::Dispatch {
                locator,
                event,
                payload,
            } => self.automation_dispatch(&locator, &event, payload, window, cx),
            crate::AutomationCommand::Action { id, payload } => {
                let action = ActionId::parse(&id)?;
                let invocation = self
                    .lifecycle
                    .runtime()
                    .borrow()
                    .actions
                    .dispatch(&action, payload.unwrap_or(UiValue::Null))?;
                let _ = self.handle_node_event(
                    &invocation.callback,
                    invocation.payload,
                    None,
                    window,
                    cx,
                );
                if let Some(error) = self.failure_message() {
                    return Err(crate::AutomationError::Command(error).into());
                }
                Ok(crate::AutomationResult::Action { id })
            }
            crate::AutomationCommand::AdvanceTime { millis } => {
                let duration = Duration::from_millis(millis);
                let clock = self.lifecycle.runtime().borrow().clock.clone();
                if !clock.advance(duration) {
                    return Err(crate::AutomationError::ClockNotControllable.into());
                }
                self.clear_failure();
                self.poll_async(cx);
                if let Some(error) = self.failure_message() {
                    return Err(crate::AutomationError::Command(error).into());
                }
                Ok(crate::AutomationResult::Advanced { millis })
            }
            crate::AutomationCommand::Snapshot | crate::AutomationCommand::Query { .. } => {
                unreachable!("read-only automation commands are handled by ScriptViewHandle")
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn automation_dispatch(
        &mut self,
        locator: &crate::AutomationLocator,
        event: &str,
        payload: Option<UiValue>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<crate::AutomationResult, ScriptViewError> {
        self.clear_failure();
        let runtime = self.lifecycle.runtime();
        let geometry = runtime.borrow().geometry_for(Some(&self.view_id));
        let mut accessibility = crate::AccessibilityTree::from_committed(
            self.lifecycle.semantics(),
            self.lifecycle.retained(),
            &geometry,
            true,
        );
        accessibility.apply_primitive_projections(
            self.primitives
                .accessibility_projections(self.lifecycle.retained(), cx),
        );
        let target = crate::automation::resolve_locator(&accessibility, locator)?;
        let target_node = self
            .lifecycle
            .retained()
            .node(target)
            .ok_or_else(|| crate::AutomationError::StaleTarget(target.get()))?;
        let payload = payload
            .or_else(|| target_node.handler_payload(event).cloned())
            .unwrap_or(UiValue::Null);
        let native_target = target_node
            .primitive()
            .zip(target_node.key())
            .map(|(primitive, key)| (primitive.clone(), key.to_owned()));
        let steps = crate::automation::dispatch_plan(self.lifecycle.retained(), target, event)?;
        let mut response = crate::EventResponse::new();
        let mut invoked = 0usize;
        let mut visited = Vec::new();
        let mut stop_after: Option<(crate::NodeId, crate::EventPhase)> = None;
        for step in steps {
            if stop_after.is_some_and(|phase| phase != (step.node, step.phase)) {
                break;
            }
            if visited.last() != Some(&step.node) {
                visited.push(step.node);
            }
            let event_target = geometry.get(step.node).map(|geometry| geometry.visual);
            let current = match &step.handler {
                crate::UiEventHandler::Script(callback) => {
                    self.handle_node_event(callback, payload.clone(), event_target, window, cx)
                }
                crate::UiEventHandler::Host(callback) => {
                    callback.invoke(payload.clone(), window, cx)
                }
                crate::UiEventHandler::Native(handler) => self.handle_native_event(
                    handler,
                    event.to_owned(),
                    payload.clone(),
                    event_target,
                    window,
                    cx,
                ),
            };
            invoked = invoked.saturating_add(1);
            if let Some(error) = self.failure_message() {
                return Err(crate::AutomationError::Command(error).into());
            }
            response.merge(current);
            match current.propagation() {
                crate::PropagationControl::StopImmediate => break,
                crate::PropagationControl::Stop => stop_after = Some((step.node, step.phase)),
                crate::PropagationControl::Continue => {}
            }
            if let Some(pointer_id) = automation_pointer_id(&payload) {
                match current.pointer_capture() {
                    crate::PointerCaptureDirective::Capture => {
                        runtime
                            .borrow()
                            .pointer_capture_for(Some(&self.view_id))
                            .capture(pointer_id, target);
                    }
                    crate::PointerCaptureDirective::Release => {
                        runtime
                            .borrow()
                            .pointer_capture_for(Some(&self.view_id))
                            .release(pointer_id);
                    }
                    crate::PointerCaptureDirective::None => {}
                }
            }
        }
        if invoked == 0
            && let Some(key) = event.strip_prefix("key:")
            && let Some((primitive, keyed)) = native_target
        {
            let instance = crate::PrimitiveInstanceId::new(primitive, keyed, target);
            if let Some(proposal) = self
                .primitives
                .perform_key(&instance, key)
                .map_err(|error| crate::AutomationError::Command(error.to_string()))?
            {
                visited.push(target);
                invoked = 1;
                let current = match proposal.handler {
                    Some(crate::UiEventHandler::Script(callback)) => {
                        self.handle_node_event(&callback, proposal.payload, None, window, cx)
                    }
                    Some(crate::UiEventHandler::Host(callback)) => {
                        callback.invoke(proposal.payload, window, cx)
                    }
                    Some(crate::UiEventHandler::Native(handler)) => self.handle_native_event(
                        &handler,
                        proposal.event,
                        proposal.payload,
                        None,
                        window,
                        cx,
                    ),
                    None => crate::EventResponse::new().stop(),
                };
                if let Some(error) = self.failure_message() {
                    return Err(crate::AutomationError::Command(error).into());
                }
                response.merge(current.stop());
            }
        }
        Ok(crate::AutomationResult::Dispatch {
            report: crate::AutomationDispatchReport {
                target: target.get(),
                visited: visited.into_iter().map(crate::NodeId::get).collect(),
                invoked,
                default_prevented: response.default_prevented(),
                stopped: response.stops_propagation(),
            },
        })
    }

    fn render_snapshot(&self, appearance: SystemAppearance) -> ScriptRenderSnapshot {
        let runtime = self.lifecycle.runtime();
        let mut runtime = runtime.borrow_mut();
        runtime.update_window_appearance(&self.window_id, appearance);
        let root = self.lifecycle.root_path();
        let theme = resolve_root_theme_from_runtime(
            &runtime,
            root,
            &self.window_id,
            appearance,
            &self.theme,
        );
        let direction = runtime
            .locale
            .as_ref()
            .and_then(|locale| locale.direction(Some(&self.window_id), Some(root)).ok())
            .unwrap_or(TextDirection::LeftToRight);
        let locale = runtime
            .locale
            .as_ref()
            .and_then(|locale| locale.locale(Some(&self.window_id), Some(root)).ok())
            .unwrap_or("en")
            .to_owned();
        let number = runtime
            .locale
            .as_ref()
            .and_then(|locale| locale.number(Some(&self.window_id), Some(root)).ok())
            .cloned();
        let motion_domain = format!("window:{}/view:{}/root", self.window_id, self.view_id);
        ScriptRenderSnapshot {
            now: runtime.clock.now(),
            clock: runtime.clock.clone(),
            motion_preference: runtime.motions.preference(),
            motion_quality: runtime.motions.quality(),
            assets: runtime.assets.clone(),
            theme,
            motions: runtime.motion_values.clone(),
            motion_ghosts: runtime
                .motion_ghosts
                .iter()
                .filter(|ghost| ghost.domain == motion_domain)
                .cloned()
                .collect(),
            signals: runtime.signals.clone(),
            geometry: runtime.geometry_for(Some(&self.view_id)),
            pointer_capture: runtime.pointer_capture_for(Some(&self.view_id)),
            virtual_requests: runtime.virtual_requests.clone(),
            direction,
            locale,
            number,
        }
    }

    fn prepare_host_render(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.prepare_render(window);
        self.host
            .interactions()
            .set_retained_tree(&self.view_id, self.lifecycle.retained());
        self.lifecycle
            .runtime()
            .borrow()
            .geometry_for(Some(&self.view_id))
            .begin_frame();
        self.text_selection.retain(self.lifecycle.retained());
        self.sync_focus_handles(cx);
        self.process_element_commands(window, cx);
    }

    fn set_content_bounds(&mut self, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        if self.content_bounds != Some(bounds) {
            self.content_bounds = Some(bounds);
            cx.notify();
        }
    }

    fn prepare_render(&mut self, window: &Window) {
        if !self.host.frame_active() {
            self.set_plain_failure(
                "embedded ScriptView must be rendered inside ScriptViewHost::container",
            );
        }
        self.sync_viewport_class(window);
        debug_assert!(self.engine.is_current(self.lifecycle.generation()));
        self.reconcile_primitive_lifecycle();
    }

    /// The focused retained node followed by its ancestors, for `group_focus`
    /// and `focus_within` styles.
    fn focus_path(&self, window: &Window) -> Vec<crate::NodeId> {
        let Some(focused) = self
            .focus_handles
            .iter()
            .find(|(_, handle)| handle.is_focused(window))
            .map(|(node, _)| *node)
        else {
            return Vec::new();
        };
        let mut path = vec![focused];
        let mut cursor = self
            .lifecycle
            .retained()
            .node(focused)
            .and_then(crate::RetainedNode::parent);
        while let Some(node) = cursor {
            path.push(node);
            cursor = self
                .lifecycle
                .retained()
                .node(node)
                .and_then(crate::RetainedNode::parent);
        }
        path
    }

    fn sync_focus_handles(&mut self, cx: &mut Context<Self>) {
        let active = self
            .lifecycle
            .retained()
            .nodes()
            .filter(|node| node.element_ref().is_some() || node.focus_styled())
            .map(crate::RetainedNode::id)
            .collect::<BTreeSet<_>>();
        self.focus_handles.retain(|node, _| active.contains(node));
        for node in active {
            self.focus_handles
                .entry(node)
                .or_insert_with(|| cx.focus_handle());
        }
        let scrollable = self
            .lifecycle
            .retained()
            .nodes()
            .filter(|node| node.scrollable())
            .map(crate::RetainedNode::id)
            .collect::<BTreeSet<_>>();
        self.scroll_handles
            .retain(|node, _| scrollable.contains(node));
        for node in scrollable {
            self.scroll_handles.entry(node).or_default();
        }
        let anchors = self
            .lifecycle
            .retained()
            .nodes()
            .filter(|node| node.element_ref().is_some())
            .filter_map(|node| {
                nearest_scroll_ancestor(self.lifecycle.retained(), node.id())
                    .map(|ancestor| (node.id(), ancestor))
            })
            .collect::<BTreeMap<_, _>>();
        self.scroll_anchors
            .retain(|node, _| anchors.contains_key(node));
        for (node, ancestor) in anchors {
            if let Some(handle) = self.scroll_handles.get(&ancestor) {
                self.scroll_anchors
                    .entry(node)
                    .or_insert_with(|| ScrollAnchor::for_handle(handle.clone()));
            }
        }
    }

    fn process_element_commands(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let commands = self
            .lifecycle
            .runtime()
            .borrow_mut()
            .take_window_element_commands(&self.window_id);
        for command in commands {
            match command {
                crate::element_ref::ElementCommand::Focus { node, .. } => {
                    if let Some(handle) = self.focus_handles.get(&node) {
                        handle.focus(window, cx);
                    } else {
                        self.set_plain_failure(format!(
                            "retained node {node} is not focusable or has been unmounted"
                        ));
                    }
                }
                crate::element_ref::ElementCommand::ScrollTo { node, x, y, .. } => {
                    if let Some(handle) = self.scroll_handles.get(&node) {
                        handle.set_offset(gpui::point(pixel_from_f64(-x), pixel_from_f64(-y)));
                    } else {
                        self.set_plain_failure(format!(
                            "retained node {node} is not a scroll container or has been unmounted"
                        ));
                    }
                }
                crate::element_ref::ElementCommand::ScrollIntoView { node, .. } => {
                    if let Some(anchor) = self.scroll_anchors.get(&node) {
                        anchor.scroll_to(window, cx);
                    } else {
                        self.set_plain_failure(format!(
                            "retained node {node} has no scrollable ancestor or was unmounted"
                        ));
                    }
                }
            }
        }
    }

    fn reconcile_primitive_lifecycle(&mut self) {
        if !self.lifecycle.retained().is_empty()
            && let Err(error) = self.primitives.retain_tree(self.lifecycle.retained())
        {
            self.set_plain_failure(error.to_string());
        }
    }

    fn sync_viewport_class(&mut self, window: &Window) {
        let width = self.content_bounds.map_or_else(
            || f64::from(window.viewport_size().width),
            |bounds| f64::from(bounds.size.width),
        );
        let needs_update = self
            .lifecycle
            .runtime()
            .borrow()
            .responsive
            .would_update_window(&self.window_id, width);
        match needs_update {
            Ok(false) => return,
            Err(error) => {
                self.set_plain_failure(error.to_string());
                return;
            }
            Ok(true) => {}
        }
        let result = self.run_script_transaction(|view| {
            let runtime = view.lifecycle.runtime();
            let changed = {
                let mut runtime = runtime.borrow_mut();
                let changed = runtime
                    .responsive
                    .update_window(&view.window_id, width)
                    .map_err(|error| error.to_string())?;
                if changed {
                    let invalidated = runtime
                        .environment_dependencies
                        .invalidate_viewport(&view.window_id);
                    runtime.mark_dirty(invalidated);
                }
                changed
            };
            if changed {
                let result = view.lifecycle.render_dirty(&mut view.engine);
                result.map_err(|error| view.lifecycle_failure(&error, None))?;
            }
            Ok(())
        });
        if let Err(error) = result {
            self.set_failure(error);
        }
    }

    fn dispatch_key_binding(
        &mut self,
        action: &DispatchScriptAction,
        window: &mut Window,
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
                self.handle_node_event(&invocation.callback, invocation.payload, None, window, cx);
            }
            Err(error) => {
                self.set_plain_failure(error.to_string());
                cx.notify();
            }
        }
    }

    fn copy_selected_text(&mut self, _: &CopySelectedText, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.text_selection.selected_text() {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            cx.stop_propagation();
        }
    }

    fn invoke_pending_effects(&mut self) -> Result<bool, ScriptFailure> {
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
                return Err(ScriptFailure::plain(
                    "semantic event/action dispatch exceeded the 64-callback budget",
                ));
            }
            for action in actions {
                let component = action.callback.component().cloned();
                let result = self.lifecycle.invoke_callback_transactional(
                    &self.engine,
                    &action.callback,
                    action.payload,
                );
                let _ =
                    result.map_err(|error| self.lifecycle_failure(&error, component.as_ref()))?;
            }
            for event in events {
                let component = event.target.clone();
                let result = self
                    .lifecycle
                    .invoke_component_event_transactional(&self.engine, event);
                let _ = result.map_err(|error| self.lifecycle_failure(&error, Some(&component)))?;
            }
        }
    }

    fn invoke_motion_timeline_callback(
        &mut self,
        event: crate::MotionTimelineEvent,
    ) -> Result<bool, ScriptFailure> {
        let Some(callback) = event.callback else {
            return Ok(false);
        };
        if callback.generation() != self.lifecycle.generation()
            || callback
                .component()
                .zip(callback.incarnation())
                .is_some_and(|(component, incarnation)| {
                    self.lifecycle
                        .runtime()
                        .borrow()
                        .component_incarnation(component)
                        != Some(incarnation)
                })
        {
            return Ok(false);
        }
        let component = callback.component().cloned();
        let payload = UiValue::Map(BTreeMap::from([
            (
                "name".to_owned(),
                UiValue::String(event.handle.name().to_owned()),
            ),
            (
                "kind".to_owned(),
                UiValue::String(
                    match event.kind {
                        crate::MotionTimelineEventKind::Complete => "complete",
                        crate::MotionTimelineEventKind::Cancel => "cancel",
                    }
                    .to_owned(),
                ),
            ),
        ]));
        let _ = self
            .lifecycle
            .invoke_callback(&self.engine, &callback, payload)
            .map_err(|error| self.lifecycle_failure(&error, component.as_ref()))?;
        Ok(true)
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
            let snapshot = crate::InspectorSnapshot::capture_for_view(
                self.lifecycle.root(),
                &runtime,
                theme,
                &components,
                self.timings.clone(),
                Some(&self.view_id),
            );
            crate::devtools::inspector_element(&snapshot)
        })
    }

    fn should_close(&mut self, cx: &mut Context<Self>) -> bool {
        if self.state.get() == ScriptViewState::Disposed {
            return true;
        }
        if self.state.get() == ScriptViewState::Suspended {
            self.release_view();
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
        let component = handler.component().cloned();
        let result = self.run_script_transaction(|view| {
            let result = view
                .lifecycle
                .invoke_callback(&view.engine, &handler, UiValue::Null);
            let _ = result.map_err(|error| view.lifecycle_failure(&error, component.as_ref()))?;
            view.invoke_pending_effects()?;
            let result = view.lifecycle.render_dirty(&mut view.engine);
            result.map_err(|error| view.lifecycle_failure(&error, None))?;
            Ok(())
        });
        match result {
            Ok(()) => {
                self.clear_failure();
                self.process_window_commands(cx);
                cx.notify();
                false
            }
            Err(error) => {
                self.set_failure(error);
                self.release_view();
                true
            }
        }
    }

    fn suspend_view(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<bool, ScriptViewError> {
        let _signal_access = self.direct_signal_access_guard();
        if let Err(error) = self.primitives.suspend_mounted(cx) {
            let rollback = self.restore_native_active(cx).err();
            let message = native_lifecycle_error("suspend", &error, rollback.as_ref());
            if rollback.is_some() {
                self.quiesce_host_view(window, cx);
                let message = self.fault_native_lifecycle(message, cx);
                return Err(ScriptViewError::Suspend(message));
            }
            self.set_plain_failure(message.clone());
            return Err(ScriptViewError::Suspend(message));
        }
        let changed = match self.lifecycle.suspend(&mut self.engine) {
            Ok(changed) => changed,
            Err(error) => {
                let rollback = self.restore_native_active(cx).err();
                let rollback_failed = rollback.is_some();
                let message = rollback.as_ref().map_or_else(
                    || error.to_string(),
                    |rollback| format!("{error}; native rollback failed: {rollback}"),
                );
                if rollback_failed {
                    self.quiesce_host_view(window, cx);
                    let message = self.fault_native_lifecycle(message, cx);
                    return Err(ScriptViewError::Suspend(message));
                }
                self.set_plain_failure(message);
                return Err(error.into());
            }
        };
        if !changed {
            if let Err(error) = self.restore_native_active(cx) {
                let message = self.fault_native_lifecycle(error.to_string(), cx);
                return Err(ScriptViewError::Suspend(message));
            }
            return Ok(false);
        }
        self.quiesce_host_view(window, cx);
        self.state.set(ScriptViewState::Suspended);
        self.clear_failure();
        Ok(true)
    }

    fn quiesce_host_view(&self, window: &mut Window, cx: &mut App) {
        let _signal_access = self.direct_signal_access_guard();
        self.host
            .quiesce_view(&self.view_id, &self.host_focus, window, cx);
    }

    fn cancel_view_interaction(&self, window: &mut Window, cx: &mut App) -> bool {
        let _signal_access = self.direct_signal_access_guard();
        self.host
            .interactions()
            .cancel_view(&self.view_id, window, cx)
    }

    fn mark_interaction_contract_changed(&mut self, changed: bool) {
        self.pending_interaction_cancel |= changed;
    }

    fn consume_pending_interaction_cancel(&mut self, window: &mut Window, cx: &mut App) {
        if std::mem::take(&mut self.pending_interaction_cancel) {
            self.cancel_view_interaction(window, cx);
        }
    }

    fn direct_signal_access_guard(&self) -> DirectSignalAccessGuard {
        let previous = self.direct_signal_access.replace(true);
        DirectSignalAccessGuard {
            flag: Rc::clone(&self.direct_signal_access),
            previous,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn resume_view(&mut self, cx: &mut Context<Self>) -> Result<bool, ScriptViewError> {
        if self.state.get() == ScriptViewState::Active {
            return Ok(false);
        }
        let _signal_access = self.direct_signal_access_guard();
        #[cfg(feature = "dev-reload")]
        let pending_reload_error = self.apply_pending_reload_on_resume();
        #[cfg(not(feature = "dev-reload"))]
        let pending_reload_error: Option<ScriptFailure> = None;
        self.collect_suspended_deliveries()?;
        let program = self.factory.program();
        let migrating = program.compiled.generation() != self.lifecycle.generation();
        let previous_exports = migrating
            .then(|| self.engine.component_exports())
            .transpose()
            .map_err(|error| ScriptViewError::Resume(error.to_string()))?;
        let previous_renderers = migrating
            .then(|| self.engine.component_renderer_snapshot())
            .transpose()
            .map_err(|error| ScriptViewError::Resume(error.to_string()))?;
        if migrating {
            self.engine
                .restore_component_exports(program.component_exports.clone())
                .map_err(|error| ScriptViewError::Resume(error.to_string()))?;
            if let Err(error) = self
                .engine
                .restore_component_renderers(program.component_renderers.clone())
            {
                if let Some(previous_exports) = previous_exports.clone() {
                    let _ = self.engine.restore_component_exports(previous_exports);
                }
                return Err(ScriptViewError::Resume(error.to_string()));
            }
        }
        self.prepare_native_resume(cx)?;
        let result = self.run_script_transaction(|view| {
            if migrating {
                view.lifecycle
                    .runtime()
                    .borrow_mut()
                    .discard_component_async_before_generation(
                        view.lifecycle.root_path(),
                        program.compiled.generation(),
                    );
                let result = view.lifecycle.resume_reload(
                    &mut view.engine,
                    program.compiled,
                    &program.state_schema,
                );
                result
                    .map(|_| true)
                    .map_err(|error| view.lifecycle_failure(&error, None))?;
                return Ok(true);
            }
            let pending = view
                .lifecycle
                .runtime()
                .borrow_mut()
                .take_window_async(&view.window_id, view.lifecycle.root_path());
            for delivery in pending {
                let component = delivery
                    .callback
                    .component()
                    .cloned()
                    .or_else(|| delivery.scope.component().cloned());
                let result = view.lifecycle.invoke_async_delivery(&view.engine, delivery);
                let _ =
                    result.map_err(|error| view.lifecycle_failure(&error, component.as_ref()))?;
            }
            view.invoke_pending_effects()?;
            let result = view.lifecycle.resume(&mut view.engine);
            result.map_err(|error| view.lifecycle_failure(&error, None))
        });
        match result {
            Ok(changed) => {
                if let Err(error) = self.primitives.commit_resume_mounted(cx) {
                    let rollback = self.primitives.suspend_mounted(cx).err();
                    let message =
                        native_lifecycle_error("commit resume", &error, rollback.as_ref());
                    let message = self.fault_native_lifecycle(message, cx);
                    return Err(ScriptViewError::Resume(message));
                }
                self.state.set(ScriptViewState::Active);
                self.activity_wake.notify();
                if let Some(error) = pending_reload_error {
                    self.set_failure(error);
                } else {
                    self.clear_failure();
                }
                self.collect_timings();
                cx.notify();
                Ok(changed)
            }
            Err(error) => {
                let native_rollback = self.primitives.suspend_mounted(cx).err();
                if let Some(previous_exports) = previous_exports {
                    let _ = self.engine.restore_component_exports(previous_exports);
                }
                if let Some(previous_renderers) = previous_renderers {
                    let _ = self.engine.restore_component_renderers(previous_renderers);
                }
                let public_error = error.clone().into_view_error();
                if let Some(native_rollback) = native_rollback {
                    let message = format!(
                        "{public_error}; native suspend compensation failed: {native_rollback}"
                    );
                    let message = self.fault_native_lifecycle(message, cx);
                    Err(ScriptViewError::Resume(message))
                } else {
                    self.set_failure(error);
                    Err(public_error)
                }
            }
        }
    }

    fn prepare_native_resume(&mut self, cx: &mut Context<Self>) -> Result<(), ScriptViewError> {
        if let Err(error) = self.primitives.resume_mounted(cx) {
            let rollback = self.primitives.suspend_mounted(cx).err();
            let message = native_lifecycle_error("resume", &error, rollback.as_ref());
            if rollback.is_some() {
                let message = self.fault_native_lifecycle(message, cx);
                return Err(ScriptViewError::Resume(message));
            }
            self.set_plain_failure(message.clone());
            return Err(ScriptViewError::Resume(message));
        }
        Ok(())
    }

    fn restore_native_active(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<(), crate::PrimitiveError> {
        self.primitives.resume_mounted(cx)?;
        self.primitives.commit_resume_mounted(cx)
    }

    fn fault_native_lifecycle(&mut self, message: String, cx: &mut Context<Self>) -> String {
        self.set_plain_failure(message.clone());
        self.release_view();
        cx.notify();
        message
    }

    fn collect_suspended_deliveries(&mut self) -> Result<(), ScriptViewError> {
        let generation = self.lifecycle.generation();
        let runtime = self.lifecycle.runtime();
        let mut runtime = runtime.borrow_mut();
        let capacity = runtime.suspended_delivery_capacity_remaining();
        let mut deliveries = runtime.tasks.drain_up_to(generation, capacity);
        let remaining = capacity.saturating_sub(deliveries.len());
        deliveries.extend(runtime.subscriptions.drain_up_to(generation, remaining));
        runtime.trace_subscription_closures();
        runtime.queue_suspended_async(deliveries)?;
        Ok(())
    }

    fn release_view(&mut self) {
        if self.state.get() == ScriptViewState::Disposed {
            return;
        }
        self.window_appearance.take();
        if let Err(error) = self.primitives.retain_mounted(&BTreeSet::new()) {
            self.set_plain_failure(error.to_string());
        }
        let _ = self.lifecycle.dispose(&mut self.engine);
        let root = self.lifecycle.root_path().clone();
        let _ = self
            .lifecycle
            .runtime()
            .borrow_mut()
            .release_window(&self.window_id, &root);
        self.lifecycle
            .runtime()
            .borrow_mut()
            .remove_presentation(&self.view_id);
        let mut native = self.native_windows.borrow_mut();
        if native
            .handles
            .get(&self.window_id)
            .is_some_and(|registration| registration.authority.lease.ptr_eq(&self.mount_lease))
        {
            native.handles.remove(&self.window_id);
            native.force_close.remove(&self.window_id);
        }
        drop(native);
        self.host.unregister_view(&self.view_id, &self.mount_lease);
        self.state.set(ScriptViewState::Disposed);
        self.activity_wake.notify();
    }

    fn process_window_commands(&mut self, cx: &mut Context<Self>) {
        let commands = self
            .lifecycle
            .runtime()
            .borrow_mut()
            .windows
            .drain_commands();
        let runtime = self.lifecycle.runtime();
        for queued in commands {
            if !window_command_is_current(&queued, &runtime.borrow().windows, cx) {
                runtime.borrow_mut().windows.cancel_open(&queued);
                continue;
            }
            let closing = matches!(queued.command(), WindowCommand::Close(_));
            let result = match queued.command().clone() {
                WindowCommand::Open(spec) => {
                    open_secondary_window(&spec, &self.factory, &self.native_windows, cx)
                }
                WindowCommand::Focus(id) | WindowCommand::Close(id) => {
                    let registration = self
                        .native_windows
                        .borrow()
                        .handles
                        .get(&id)
                        .cloned()
                        .filter(|registration| {
                            registration.authority.is_current(cx)
                                && queued.target_binding().is_some_and(|target| {
                                    target.window == registration.handle.window_id()
                                        && target.lease.ptr_eq(&registration.authority.lease)
                                })
                        });
                    registration.map_or_else(
                        || {
                            Err(ScriptFailure::plain(format!(
                                "native window `{id}` is unavailable"
                            )))
                        },
                        |registration| {
                            let handle = registration.handle;
                            let target_authority = registration.authority;
                            // A script commonly focuses/closes from an event
                            // dispatched by the same window. Updating that window
                            // recursively fails in GPUI with `window not found`, so
                            // apply it after the current entity update unwinds.
                            let view = cx.weak_entity();
                            let native_windows = Rc::clone(&self.native_windows);
                            let runtime = Rc::clone(&runtime);
                            let close_id = id.clone();
                            cx.defer(move |cx| {
                                // Authority may be revoked between queueing and
                                // execution. Do not let a disposed owner operate
                                // on its replacement or leave a stale close bypass.
                                if !window_command_is_current(
                                    &queued,
                                    &runtime.borrow().windows,
                                    cx,
                                ) || !target_authority.is_current(cx)
                                    || native_windows.borrow().handles.get(&close_id).is_none_or(
                                        |current| current.handle.window_id() != handle.window_id(),
                                    )
                                {
                                    return;
                                }
                                if closing {
                                    native_windows
                                        .borrow_mut()
                                        .force_close
                                        .insert(close_id.clone());
                                }
                                if let Err(error) = handle.update(cx, |_, window, _| {
                                    if closing {
                                        window.remove_window();
                                    } else {
                                        window.activate_window();
                                    }
                                }) {
                                    native_windows.borrow_mut().force_close.remove(&close_id);
                                    let message = error.to_string();
                                    let _ = view.update(cx, |view, cx| {
                                        view.set_plain_failure(message);
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
                self.set_failure(error);
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
        event_target: Option<crate::GeometryBounds>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> crate::EventResponse {
        if self.state.get() != ScriptViewState::Active {
            return crate::EventResponse::new().stop();
        }
        if let Ok(mut runtime) = self.lifecycle.runtime().try_borrow_mut() {
            runtime.traces.push(
                crate::RuntimeTraceKind::Event,
                "/App[root]",
                format!("callback {}", callback.name()),
                None,
                true,
            );
        }
        let callback_result = self.run_script_transaction(|view| {
            let result = view.lifecycle.invoke_callback_with_event_target(
                &view.engine,
                callback,
                payload,
                event_target,
            );
            let value =
                result.map_err(|error| view.lifecycle_failure(&error, callback.component()))?;
            view.invoke_pending_effects()?;
            let result = view.lifecycle.render_dirty(&mut view.engine);
            let rendered = result.map_err(|error| view.lifecycle_failure(&error, None))?;
            Ok((value, rendered))
        });
        let response = callback_result.as_ref().map_or_else(
            |_| crate::EventResponse::new().stop(),
            |(value, _)| event_response_from_dynamic(value),
        );
        let rendered = callback_result
            .as_ref()
            .is_ok_and(|(_, rendered)| *rendered);
        let succeeded = callback_result.is_ok();
        match callback_result {
            Ok(_) => self.clear_failure(),
            Err(error) => self.set_failure(error),
        }
        self.process_window_commands(cx);
        self.process_element_commands(window, cx);
        self.mark_interaction_contract_changed(rendered);
        self.consume_pending_interaction_cancel(window, cx);
        self.collect_timings();
        cx.notify();
        if succeeded {
            response
        } else {
            crate::EventResponse::new().stop()
        }
    }

    fn handle_native_event(
        &mut self,
        handler: &crate::NativeHandlerRef,
        event: String,
        payload: UiValue,
        event_target: Option<crate::GeometryBounds>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> crate::EventResponse {
        if self.state.get() != ScriptViewState::Active {
            return crate::EventResponse::new().stop();
        }
        let registry = self.engine.native_handler_registry();
        let result = self.run_script_transaction(|view| {
            let response = {
                let runtime = view.lifecycle.runtime();
                let mut runtime = runtime
                    .try_borrow_mut()
                    .map_err(|_| "UI runtime state is already borrowed".to_owned())?;
                registry
                    .invoke(
                        handler,
                        crate::NativeEvent {
                            name: event,
                            payload,
                            target: event_target,
                        },
                        &mut runtime,
                        window,
                        cx,
                    )
                    .map_err(|error| error.to_string())?
            };
            view.invoke_pending_effects()?;
            let result = view.lifecycle.render_dirty(&mut view.engine);
            let rendered = result.map_err(|error| view.lifecycle_failure(&error, None))?;
            Ok((response, rendered))
        });
        match result {
            Ok((response, rendered)) => {
                self.clear_failure();
                self.process_window_commands(cx);
                self.process_element_commands(window, cx);
                self.mark_interaction_contract_changed(rendered);
                self.consume_pending_interaction_cancel(window, cx);
                self.collect_timings();
                cx.notify();
                response
            }
            Err(error) => {
                self.set_failure(error);
                cx.notify();
                crate::EventResponse::new().stop()
            }
        }
    }

    fn begin_script_transaction(&self) -> Result<ScriptViewTransaction, String> {
        let runtime = self
            .lifecycle
            .runtime()
            .try_borrow_mut()
            .map_err(|_| "UI runtime state is already borrowed".to_owned())?
            .begin_transaction()
            .map_err(|error| error.to_string())?;
        Ok(ScriptViewTransaction {
            runtime,
            engine: self.engine.execution_checkpoint(),
            lifecycle: self.lifecycle.execution_checkpoint(),
        })
    }

    fn rollback_script_transaction(
        &mut self,
        transaction: ScriptViewTransaction,
    ) -> Result<(), String> {
        let runtime = self.lifecycle.runtime();
        let runtime_result = match runtime.try_borrow_mut() {
            Ok(mut runtime) => runtime
                .restore(transaction.runtime)
                .map_err(|error| error.to_string()),
            Err(_) => Err("UI runtime state is already borrowed".to_owned()),
        };
        self.engine.restore_execution_checkpoint(transaction.engine);
        self.lifecycle
            .restore_execution_checkpoint(transaction.lifecycle);
        runtime_result
    }

    fn run_script_transaction<T>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, ScriptFailure>,
    ) -> Result<T, ScriptFailure> {
        let transaction = self
            .begin_script_transaction()
            .map_err(ScriptFailure::from)?;
        match operation(self) {
            Ok(value) => {
                let runtime = self.lifecycle.runtime();
                let commit = match runtime.try_borrow_mut() {
                    Ok(mut runtime) => runtime
                        .commit_transaction()
                        .map_err(|error| error.to_string()),
                    Err(_) => Err("UI runtime state is already borrowed".to_owned()),
                };
                match commit {
                    Ok(()) => Ok(value),
                    Err(error) => {
                        let rollback = self.rollback_script_transaction(transaction);
                        Err(ScriptFailure::plain(rollback.map_or_else(
                            |rollback| {
                                format!(
                                    "transaction commit failed: {error}; rollback also failed: {rollback}"
                                )
                            },
                            |()| format!("transaction commit failed: {error}"),
                        )))
                    }
                }
            }
            Err(error) => {
                let rollback = self.rollback_script_transaction(transaction);
                match rollback {
                    Ok(()) => Err(error),
                    Err(rollback) => Err(error
                        .append_context(format!("transaction rollback also failed: {rollback}"))),
                }
            }
        }
    }

    fn collect_timings(&mut self) {
        self.timings.extend(self.engine.take_timings());
        if self.timings.len() > 200 {
            self.timings.drain(0..self.timings.len() - 200);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn poll_async(&mut self, cx: &mut Context<Self>) {
        if self.state.get() == ScriptViewState::Suspended {
            if let Err(error) = self.collect_suspended_deliveries() {
                self.set_plain_failure(error.to_string());
                cx.notify();
            }
            return;
        }
        if self.state.get() == ScriptViewState::Disposed {
            return;
        }
        self.process_window_commands(cx);
        self.sync_program();
        let generation = self.lifecycle.generation();
        let runtime = self.lifecycle.runtime();
        let root = self.lifecycle.root_path().clone();
        let (deliveries, dirty, pending_dispatch, virtual_requests, repaint) = {
            let mut runtime = runtime.borrow_mut();
            runtime.flush_geometry_dependencies();
            let _ = runtime.assets.retain_decode_generation(generation);
            let now = runtime.clock.now();
            let mut deliveries = runtime.tasks.drain(generation);
            deliveries.extend(runtime.subscriptions.drain(generation));
            deliveries.extend(runtime.timers.drain(now, generation));
            runtime.trace_subscription_closures();
            if let Ok(asset_deliveries) = runtime.assets.drain_image_decodes(generation) {
                deliveries.extend(asset_deliveries);
            }
            runtime.queue_async(deliveries);
            let deliveries = runtime.take_window_async(&self.window_id, &root);
            (
                deliveries,
                runtime.has_window_dirty(&root),
                runtime.has_pending_dispatch(),
                runtime.has_virtual_requests(),
                runtime.take_window_repaint(&self.window_id),
            )
        };
        let has_script_work =
            !deliveries.is_empty() || dirty || pending_dispatch || virtual_requests;
        if !has_script_work && !repaint {
            return;
        }

        let (delivery_changed, delivery_contract_changed, delivery_error) =
            self.deliver_async_batch(deliveries);
        self.mark_interaction_contract_changed(delivery_contract_changed);
        let result = if dirty || pending_dispatch || virtual_requests {
            self.run_script_transaction(|view| {
                let mut changed = view.invoke_pending_effects()?;
                let result = view.lifecycle.realize_virtual_requests(&mut view.engine);
                changed |= result.map_err(|error| view.lifecycle_failure(&error, None))?;
                let result = view.lifecycle.render_dirty(&mut view.engine);
                let contract_changed =
                    result.map_err(|error| view.lifecycle_failure(&error, None))?;
                changed |= contract_changed;
                Ok((
                    changed || delivery_changed,
                    contract_changed || delivery_contract_changed,
                ))
            })
        } else {
            Ok((delivery_changed, delivery_contract_changed))
        };
        let result = match (result, delivery_error) {
            (Ok(_), Some(error)) => Err(error),
            (result, _) => result,
        };
        let notify = match result {
            Ok((changed, contract_changed)) => {
                self.mark_interaction_contract_changed(contract_changed);
                if has_script_work {
                    self.clear_failure();
                }
                self.process_window_commands(cx);
                changed || repaint
            }
            Err(error) => {
                self.set_failure(error);
                true
            }
        };
        self.collect_timings();
        if notify {
            cx.notify();
        }
    }

    fn deliver_async_batch(
        &mut self,
        deliveries: Vec<crate::AsyncDelivery>,
    ) -> (bool, bool, Option<ScriptFailure>) {
        let mut changed = false;
        let mut contract_changed = false;
        let mut first_error = None;
        for delivery in deliveries {
            let scope = format!("{:?}", delivery.scope);
            let component = delivery
                .callback
                .component()
                .cloned()
                .or_else(|| delivery.scope.component().cloned());
            let result = self.run_script_transaction(|view| {
                let mut delivery_changed = view.invoke_pending_effects()?;
                let result = view.lifecycle.invoke_async_delivery(&view.engine, delivery);
                let _ =
                    result.map_err(|error| view.lifecycle_failure(&error, component.as_ref()))?;
                delivery_changed |= view.invoke_pending_effects()?;
                let result = view.lifecycle.render_dirty(&mut view.engine);
                let rendered = result.map_err(|error| view.lifecycle_failure(&error, None))?;
                delivery_changed |= rendered;
                Ok((delivery_changed, rendered))
            });
            match result {
                Ok((delivery_changed, rendered)) => {
                    changed |= delivery_changed;
                    contract_changed |= rendered;
                }
                Err(error) => {
                    self.lifecycle.runtime().borrow_mut().traces.push(
                        crate::RuntimeTraceKind::Task,
                        scope,
                        format!("delivery failed: {}", error.message),
                        None,
                        false,
                    );
                    first_error.get_or_insert(error);
                }
            }
        }
        (changed, contract_changed, first_error)
    }

    fn sync_program(&mut self) {
        let program = self.factory.program();
        if program.compiled.generation() == self.lifecycle.generation() {
            return;
        }
        let previous_exports = match self.engine.component_exports() {
            Ok(exports) => exports,
            Err(error) => {
                self.set_plain_failure(error.to_string());
                return;
            }
        };
        let previous_renderers = match self.engine.component_renderer_snapshot() {
            Ok(renderers) => renderers,
            Err(error) => {
                self.set_plain_failure(error.to_string());
                return;
            }
        };
        if let Err(error) = self
            .engine
            .restore_component_exports(program.component_exports.clone())
        {
            self.set_plain_failure(error.to_string());
            return;
        }
        if let Err(error) = self
            .engine
            .restore_component_renderers(program.component_renderers.clone())
        {
            self.set_plain_failure(error.to_string());
            let _ = self.engine.restore_component_exports(previous_exports);
            return;
        }
        if let Err(error) =
            self.lifecycle
                .reload(&mut self.engine, program.compiled, &program.state_schema)
        {
            let mut failure = self.lifecycle_failure(&error, None);
            let rollback_exports = self.engine.restore_component_exports(previous_exports);
            let rollback_renderers = self.engine.restore_component_renderers(previous_renderers);
            if let Some(rollback) = rollback_exports.err().or_else(|| rollback_renderers.err()) {
                failure = failure.append_context(format!("rollback failed: {rollback}"));
            }
            self.set_failure(failure);
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
                self.set_plain_failure(error.to_string());
                cx.notify();
                return;
            }
        };
        if batch.paths.is_empty() {
            return;
        }
        if self.state.get() == ScriptViewState::Suspended {
            self.pending_reload_paths.extend(batch.paths);
            return;
        }
        let result = self.reload_changed_paths(&batch.paths);
        match result {
            Ok(()) => self.clear_failure(),
            Err(error) => self.set_failure(error),
        }
        self.collect_timings();
        cx.notify();
    }

    #[cfg(feature = "dev-reload")]
    fn apply_pending_reload_on_resume(&mut self) -> Option<ScriptFailure> {
        let paths = std::mem::take(&mut self.pending_reload_paths);
        if paths.is_empty() {
            return None;
        }
        // A broken edit does not strand the retained view. Resume keeps using
        // the last successfully compiled program and exposes the candidate
        // error through the normal development error banner.
        self.reload_changed_paths(&paths).err()
    }

    #[cfg(feature = "dev-reload")]
    fn reload_changed_paths(&mut self, paths: &BTreeSet<PathBuf>) -> Result<(), ScriptFailure> {
        let theme_path = self.theme_path.canonicalize().ok();
        let style_path = self.style_path.canonicalize().ok();
        let themes_root = self.ui_root.join("themes").canonicalize().ok();
        let tokens_path = self.ui_root.join(TOKEN_BASE_FILE).canonicalize().ok();
        let theme_changed = tokens_path
            .as_ref()
            .is_some_and(|tokens| paths.contains(tokens))
            || theme_path
                .as_ref()
                .is_some_and(|theme| paths.contains(theme))
            || themes_root
                .as_ref()
                .is_some_and(|themes_root| paths.iter().any(|path| path.starts_with(themes_root)));
        let style_changed = style_path
            .as_ref()
            .is_some_and(|styles| paths.contains(styles))
            || paths.iter().any(|path| path.ends_with(&self.style_path));
        let locale_root = self.ui_root.join("locales").canonicalize().ok();
        let locale_changed = locale_root
            .as_ref()
            .is_some_and(|locale_root| paths.iter().any(|path| path.starts_with(locale_root)));
        let assets_root = self.ui_root.join("assets").canonicalize().ok();
        let assets_changed = assets_root
            .as_ref()
            .is_some_and(|assets_root| paths.iter().any(|path| path.starts_with(assets_root)));
        let manifest_path = self.ui_root.join("app.toml").canonicalize().ok();
        let manifest_changed = manifest_path
            .as_ref()
            .is_some_and(|manifest| paths.contains(manifest));
        let script_changed = paths.iter().any(|path| {
            path.extension().and_then(|extension| extension.to_str()) == Some("rhai")
                && Some(path) != theme_path.as_ref()
                && Some(path) != tokens_path.as_ref()
                && Some(path) != style_path.as_ref()
                && !path.ends_with(&self.style_path)
                && !locale_root
                    .as_ref()
                    .is_some_and(|locale_root| path.starts_with(locale_root))
                && !themes_root
                    .as_ref()
                    .is_some_and(|themes_root| path.starts_with(themes_root))
        });

        if script_changed {
            self.reload_scripts(paths)
        } else {
            Ok(())
        }
        .and_then(|()| {
            if theme_changed {
                self.reload_theme().map_err(ScriptFailure::from)
            } else {
                Ok(())
            }
        })
        .and_then(|()| {
            if style_changed {
                self.reload_component_styles()
            } else {
                Ok(())
            }
        })
        .and_then(|()| {
            if locale_changed {
                self.reload_locales().map_err(ScriptFailure::from)
            } else {
                Ok(())
            }
        })
        .and_then(|()| {
            if assets_changed {
                self.reload_assets().map_err(ScriptFailure::from)
            } else {
                Ok(())
            }
        })
        .and_then(|()| {
            if manifest_changed {
                self.reload_manifest().map_err(ScriptFailure::from)
            } else {
                Ok(())
            }
        })
    }

    #[cfg(feature = "dev-reload")]
    fn reload_scripts(&mut self, changed_paths: &BTreeSet<PathBuf>) -> Result<(), ScriptFailure> {
        let source = fs::read_to_string(&self.entry)
            .map_err(|error| ScriptFailure::plain(error.to_string()))?;
        let modules = discover_modules(
            &self.ui_root,
            &self.entry,
            &self.theme_path,
            &self.style_path,
        )
        .map_err(|error| ScriptFailure::plain(error.to_string()))?;
        let file_source = FileScriptSource::new(&self.ui_root, modules)
            .map_err(|error| ScriptFailure::plain(error.to_string()))?;
        let root = self
            .ui_root
            .canonicalize()
            .map_err(|error| ScriptFailure::plain(error.to_string()))?;
        let changed_modules = changed_module_ids(&root, changed_paths);
        let mut candidate_engine = self.engine.candidate_engine();
        for extension in self.factory.extensions.iter() {
            extension
                .configure_engine(&mut candidate_engine)
                .map_err(ScriptFailure::plain)?;
        }
        let mut candidate_cache = self.module_cache.clone();
        let refresh = candidate_cache
            .refresh(candidate_engine.engine(), &file_source, changed_modules)
            .map_err(|error| ScriptFailure::plain(error.to_string()))?;
        let resolver =
            RestrictedModuleResolver::from_source_with_cache(&file_source, &candidate_cache)
                .map_err(|error| ScriptFailure::plain(error.to_string()))?;
        let candidate = candidate_engine.with_program_preparation(|engine| {
            engine.set_module_resolver(resolver);
            preload_component_modules(engine, file_source.module_ids())
                .map_err(|error| ScriptFailure::plain(error.to_string()))?;
            let candidate = engine
                .compile_self_contained_named(&self.entry.to_string_lossy(), &source)
                .map_err(|error| self.runtime_failure_with_engine(engine, &error, None))?;
            let state_schema = engine
                .root_state_schema(&candidate)
                .map_err(|error| self.runtime_failure_with_engine(engine, &error, None))?;
            let program_exports = engine
                .component_exports()
                .map_err(|error| ScriptFailure::plain(error.to_string()))?;
            let program_renderers = engine
                .component_renderer_snapshot()
                .map_err(|error| ScriptFailure::plain(error.to_string()))?;
            Ok::<_, ScriptFailure>((candidate, state_schema, program_exports, program_renderers))
        });
        let result = candidate.and_then(
            |(candidate, state_schema, program_exports, program_renderers)| {
                let program_compiled = candidate.clone();
                let program_schema = state_schema.clone();
                load_file_component_styles(
                    candidate_engine.engine(),
                    &self.style_path,
                    &program_exports,
                )
                .map_err(|error| ScriptFailure::plain(error.to_string()))?;
                if self.state.get() != ScriptViewState::Suspended {
                    if let Err(error) =
                        self.lifecycle
                            .reload(&mut candidate_engine, candidate, &state_schema)
                    {
                        return Err(self.lifecycle_failure_with_engine(
                            &candidate_engine,
                            &error,
                            None,
                        ));
                    }
                    self.engine = candidate_engine;
                }
                self.module_cache = candidate_cache;
                self.factory.update_program(
                    program_compiled,
                    program_schema,
                    program_exports,
                    program_renderers,
                );
                Ok(())
            },
        );
        if result.is_ok() {
            self.trace_script_reload(refresh.affected.len(), refresh.compiled.len());
        }
        result
    }

    #[cfg(feature = "dev-reload")]
    fn trace_script_reload(&self, affected: usize, compiled: usize) {
        self.lifecycle.runtime().borrow_mut().traces.push(
            crate::RuntimeTraceKind::Reload,
            self.lifecycle.root_path().to_string(),
            format!("refreshed {affected} affected module(s), compiled {compiled}"),
            None,
            false,
        );
    }

    #[cfg(feature = "dev-reload")]
    fn reload_theme(&mut self) -> Result<(), String> {
        let tokens_path = self.ui_root.join(TOKEN_BASE_FILE);
        if tokens_path.exists() {
            let source = fs::read_to_string(&tokens_path).map_err(|error| error.to_string())?;
            let base = crate::load_token_base(
                self.engine.engine(),
                &tokens_path.to_string_lossy(),
                &source,
            )
            .map_err(|error| error.to_string())?;
            self.factory.theme_layers.borrow_mut().base = Some(base);
        }
        let layers = self.factory.theme_layers.borrow().clone();
        let source = fs::read_to_string(&self.theme_path).map_err(|error| error.to_string())?;
        let primary = crate::load_theme_with_layers(
            self.engine.engine(),
            layers.base.as_ref(),
            &self.theme_path.to_string_lossy(),
            &source,
            &layers.overrides,
        )
        .map_err(|error| error.to_string())?;
        let runtime = self.lifecycle.runtime();
        let previous = runtime
            .borrow()
            .theme
            .as_ref()
            .map(|themes| themes.app_preference().clone());
        let mut themes = load_theme_directory(
            self.engine.engine(),
            &self.ui_root.join("themes"),
            &primary,
            &layers,
        )
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
    fn reload_component_styles(&mut self) -> Result<(), ScriptFailure> {
        let components = self
            .engine
            .component_exports()
            .map_err(|error| ScriptFailure::plain(error.to_string()))?;
        let styles = self
            .load_current_component_styles(&components)
            .map_err(ScriptFailure::from)?;
        self.run_script_transaction(|view| {
            let changed = view
                .lifecycle
                .runtime()
                .borrow_mut()
                .replace_component_styles_from_host(styles);
            if changed && view.state.get() == ScriptViewState::Active {
                let result = view.lifecycle.render_dirty(&mut view.engine);
                result.map_err(|error| view.lifecycle_failure(&error, None))?;
            }
            Ok(())
        })
    }

    #[cfg(feature = "dev-reload")]
    fn load_current_component_styles(
        &self,
        components: &ComponentRegistry,
    ) -> Result<ComponentStyleSheet, String> {
        load_file_component_styles(self.engine.engine(), &self.style_path, components)
            .map_err(|error| error.to_string())
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

fn sample_motion_domain(
    runtime: &mut UiRuntimeState,
    domain: &str,
    now: Instant,
) -> (crate::MotionFrame, Vec<crate::MotionTimelineEvent>) {
    let frame = runtime.motions.tick_scope(now, domain);
    runtime.motion_values = runtime.motions.snapshot(now);
    let ghosts = std::mem::take(&mut runtime.motion_ghosts);
    let mut retained = Vec::with_capacity(ghosts.len());
    for ghost in ghosts {
        if ghost.domain != domain || runtime.motions.is_node_scope_active(&ghost.path) {
            retained.push(ghost);
        } else {
            runtime.motions.cancel_node_scope(&ghost.path);
        }
    }
    runtime.motion_ghosts = retained;
    let events = runtime.motions.drain_timeline_events_for_domain(domain);
    (frame, events)
}

#[cfg(feature = "dev-reload")]
fn changed_module_ids(root: &Path, changed_paths: &BTreeSet<PathBuf>) -> Vec<ModuleId> {
    changed_paths
        .iter()
        .filter_map(|path| {
            let relative = path.strip_prefix(root).ok()?;
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
        })
        .collect()
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
    #[error("{message}")]
    ScriptDiagnostic {
        message: String,
        diagnostic: Box<Diagnostic>,
    },
    #[error("failed to open GPUI window: {0}")]
    Window(String),
    #[error("script view ID `{0}` must be 1-64 ASCII alphanumeric, `_`, or `-` characters")]
    InvalidId(String),
    #[error("script view `{0}` is already mounted in this host")]
    DuplicateView(String),
    #[error("Host `{0}` is already bound to a different native window")]
    WrongNativeWindow(String),
    #[error("window `{window}` already has script command owner `{owner}`")]
    WindowCommandOwner { window: String, owner: String },
    #[error("script view `{0}` has been disposed")]
    DisposedView(String),
    #[error("script view `{0}` is suspended; resume it before rendering or interaction")]
    SuspendedView(String),
    #[error("script view suspend failed: {0}")]
    Suspend(String),
    #[error("script view resume failed: {0}")]
    Resume(String),
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
    #[error(transparent)]
    ComponentStyle(#[from] ComponentStyleError),
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
    Async(#[from] crate::AsyncRuntimeError),
    #[error(transparent)]
    Responsive(#[from] ResponsiveError),
    #[error(transparent)]
    Dependency(#[from] DependencyError),
    #[error(transparent)]
    ComponentExport(#[from] ComponentExportError),
    #[error(transparent)]
    Action(#[from] ActionError),
    #[error(transparent)]
    Signal(#[from] crate::SignalError),
    #[error(transparent)]
    NativeCollection(#[from] crate::NativeCollectionError),
    #[error(transparent)]
    Document(#[from] crate::DocumentError),
    #[error(transparent)]
    ElementRef(#[from] crate::ElementRefError),
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
    #[error(transparent)]
    Font(#[from] crate::FontError),
    #[error(transparent)]
    Accessibility(#[from] crate::AccessibilityError),
    #[error(transparent)]
    Automation(#[from] crate::AutomationError),
    #[cfg(feature = "dev-reload")]
    #[error(transparent)]
    Watcher(#[from] crate::WatcherError),
}

impl ScriptViewError {
    #[must_use]
    pub fn diagnostic(&self) -> Option<&Diagnostic> {
        match self {
            Self::ScriptDiagnostic { diagnostic, .. } => Some(diagnostic),
            _ => None,
        }
    }
}

impl From<ScriptViewError> for ScriptFailure {
    fn from(error: ScriptViewError) -> Self {
        match error {
            ScriptViewError::ScriptDiagnostic {
                message,
                diagnostic,
            } => Self::with_diagnostic(message, *diagnostic),
            error => Self::plain(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WindowCommand;

    #[test]
    fn motion_frame_freezes_only_events_present_at_its_sample_boundary() {
        let now = Instant::now();
        let domain = "window:w/view:v/root";
        let source = |name: &str| {
            crate::MotionTimeline::new(
                name,
                crate::MotionTimelineStep::Track(crate::MotionTrack {
                    target: ".".to_owned(),
                    source: crate::MotionSource::Transition(crate::MotionTransition::new(
                        crate::MotionProperty::Opacity,
                        0.0,
                        1.0,
                        1,
                    )),
                }),
            )
        };
        let mut runtime = UiRuntimeState::new();
        runtime
            .motions
            .start_timeline(
                ComponentInstancePath::root("UiNode", format!("{domain}/first")),
                source("first"),
                now,
            )
            .unwrap();
        let (_, committed) = sample_motion_domain(
            &mut runtime,
            domain,
            now + std::time::Duration::from_millis(1),
        );
        assert_eq!(committed.len(), 1);

        let later = runtime
            .motions
            .start_timeline(
                ComponentInstancePath::root("UiNode", format!("{domain}/later")),
                source("later"),
                now,
            )
            .unwrap();
        runtime.motions.cancel_timeline(&later).unwrap();
        assert_eq!(committed.len(), 1, "the committed batch is immutable");
        assert_eq!(
            runtime
                .motions
                .drain_timeline_events_for_domain(domain)
                .len(),
            1,
            "events created after sampling remain for the next rendered frame"
        );
    }

    fn write_manifest(directory: &Path) {
        fs::write(
            directory.join("app.toml"),
            "entry = \"main\"\nruntime_api = 3\n",
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
                None,
            )
            .unwrap();
        (prepared.engine, lifecycle)
    }

    #[test]
    fn error_banner_policy_defaults_on_and_can_be_disabled() {
        assert!(ScriptViewConfig::new("default").show_error_banner);
        assert!(
            !ScriptViewConfig::new("custom")
                .show_error_banner(false)
                .show_error_banner
        );
    }

    #[test]
    fn structured_script_error_keeps_human_display_separate_from_diagnostic() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile_named("ui/failure.rhai", "fn view() { throw \"boom\"; }")
            .unwrap();
        let runtime_error = engine.render(&compiled).unwrap_err();
        let diagnostic = Diagnostic::from_runtime(
            &runtime_error,
            &crate::DiagnosticContext {
                source: Some("ui/failure.rhai".to_owned()),
                component: Some(ComponentInstancePath::root("View", "failure")),
                execution_timing: engine.last_failed_timing(),
                ..crate::DiagnosticContext::default()
            },
        );
        let error = ScriptViewError::ScriptDiagnostic {
            message: "human message".to_owned(),
            diagnostic: Box::new(diagnostic.clone()),
        };

        assert_eq!(error.to_string(), "human message");
        assert_eq!(error.diagnostic(), Some(&diagnostic));
        let failure = ScriptFailure::from(error);
        assert_eq!(failure.message, "human message");
        assert_eq!(failure.diagnostic, Some(Box::new(diagnostic)));
    }

    #[test]
    fn initial_mount_failure_returns_a_structured_root_diagnostic() {
        let entry = ModuleId::parse("main").unwrap();
        let mut prepared = EmbeddedScriptView::new(
            entry.clone(),
            EmbeddedScriptSource::new(BTreeMap::from([(
                entry,
                "fn view(ctx) { throw \"mount failure\"; }".to_owned(),
            )])),
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .prepare()
        .unwrap();
        let program = prepared.factory.program();
        let Err(error) = prepared.factory.mount_lifecycle(
            &mut prepared.engine,
            program,
            "broken-view",
            "main",
            WindowCommandPolicy::Disabled,
            true,
            None,
        ) else {
            panic!("mount unexpectedly succeeded");
        };

        assert!(error.to_string().contains("mount failure"));
        assert!(!error.to_string().contains("component_state"));
        let diagnostic = error.diagnostic().expect("script diagnostic");
        assert_eq!(diagnostic.component.as_deref(), Some("/View[broken-view]"));
        assert_eq!(diagnostic.key.as_deref(), Some("broken-view"));
        assert_eq!(
            diagnostic
                .execution
                .as_ref()
                .map(|execution| &execution.operation),
            Some(&crate::ExecutionOperation::Render)
        );
    }

    #[test]
    fn theme_handle_state_only_advances_for_an_effective_theme_change() {
        let engine = RuntimeEngine::new();
        let dark = crate::load_theme_with_layers(
            engine.engine(),
            Some(
                &crate::load_token_base(
                    engine.engine(),
                    "tokens.rhai",
                    include_str!("../../../registry/tokens.rhai"),
                )
                .unwrap(),
            ),
            "default_dark.rhai",
            include_str!("../../../registry/themes/default_dark.rhai"),
            &crate::ThemeTokenOverrides::default(),
        )
        .unwrap();
        let mut light = dark.clone();
        light.name = "Light".to_owned();
        light.mode = crate::ThemeMode::Light;
        let mut state = ThemeHandleState {
            snapshot: ThemeSnapshot::new(7, dark.clone()),
        };

        assert!(!state.replace(dark));
        assert_eq!(state.snapshot.revision, 7);
        assert!(state.replace(light.clone()));
        assert_eq!(state.snapshot.revision, 8);
        assert_eq!(state.snapshot.variant, light);
        assert!(state.snapshot.variant.tokens.radii.contains_key("md"));
        assert!(
            state
                .snapshot
                .variant
                .tokens
                .typography
                .roles
                .contains_key("body")
        );
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
    fn retained_descendant_resolves_nearest_scroll_ancestor() {
        let root = crate::UiNode::box_node(vec![
            crate::UiNode::box_node(vec![crate::UiNode::text("target").with_key("target")])
                .with_key("middle"),
        ])
        .with_key("scroll")
        .with_style(&crate::Style::new().overflow_y_scroll());
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(root).unwrap();
        let target = tree
            .nodes()
            .find(|node| node.key() == Some("target"))
            .unwrap()
            .id();
        assert_eq!(nearest_scroll_ancestor(&tree, target), tree.root_id());
    }

    #[test]
    fn pointer_callback_return_controls_gpui_propagation() {
        assert_eq!(
            event_response_from_dynamic(&rhai::Dynamic::from("propagate")).propagation(),
            crate::PropagationControl::Continue
        );
        assert_eq!(
            event_response_from_dynamic(&rhai::Dynamic::UNIT).propagation(),
            crate::PropagationControl::Stop
        );
        let response = crate::EventResponse::new()
            .prevent_default()
            .capture_pointer();
        assert_eq!(
            event_response_from_dynamic(&rhai::Dynamic::from(response)),
            response
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
    fn embedded_host_theme_overrides_cover_primary_and_additional_variants() {
        let entry = ModuleId::parse("main").unwrap();
        let scripts = EmbeddedScriptSource::new(BTreeMap::from([(
            entry.clone(),
            "fn view(ctx) { text(\"theme override\") }".to_owned(),
        )]));
        let overrides = ThemeTokenOverrides {
            colors: BTreeMap::from([(
                "accent".to_owned(),
                crate::Rgba8::from_rgb_hex(0x00aa_55cc),
            )]),
            radii: BTreeMap::from([
                ("sm".to_owned(), crate::Length::Pixels(4.0)),
                ("md".to_owned(), crate::Length::Pixels(7.0)),
                ("lg".to_owned(), crate::Length::Pixels(10.0)),
            ]),
            ..ThemeTokenOverrides::default()
        };
        let prepared = EmbeddedScriptView::new(
            entry,
            scripts,
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .theme_sources([(
            "nord.rhai".to_owned(),
            include_str!("../../../registry/themes/nord.rhai").to_owned(),
        )])
        .theme_token_overrides(overrides)
        .prepare()
        .unwrap();
        assert_eq!(
            prepared.theme.tokens.radii["md"],
            crate::Variable::Fixed(crate::Length::Pixels(7.0))
        );
        assert_eq!(
            prepared.theme.tokens.color("accent"),
            Some(crate::Rgba8::from_rgb_hex(0x00aa_55cc))
        );

        let mut runtime = prepared.factory.runtime.borrow_mut();
        let themes = runtime.theme.as_mut().unwrap();
        themes
            .set_app(crate::ThemePreference::Fixed {
                selection: ThemeSelection::new("Nord", "Dark"),
            })
            .unwrap();
        assert_eq!(
            themes
                .resolve(None, None, SystemAppearance::Dark)
                .unwrap()
                .variant()
                .tokens
                .radii["md"],
            crate::Variable::Fixed(crate::Length::Pixels(7.0))
        );
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
    fn file_view_applies_the_validated_component_stylesheet() {
        let directory = tempfile::tempdir().unwrap();
        let components = directory.path().join("components");
        fs::create_dir_all(&components).unwrap();
        fs::write(
            components.join("button.rhai"),
            include_str!("../../../registry/components/button.rhai"),
        )
        .unwrap();
        fs::write(
            directory.path().join("main.rhai"),
            r#"import "components/button" as button;
            fn view(ctx) { button::Button(#{ text: "Styled" }) }
            "#,
        )
        .unwrap();
        fs::write(
            directory.path().join("styles.rhai"),
            r#"fn component_styles() {
                #{ "components/button": #{ root: style().height(px(41)) } }
            }
            "#,
        )
        .unwrap();
        fs::write(
            directory.path().join("theme.rhai"),
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .unwrap();
        fs::write(
            directory.path().join(TOKEN_BASE_FILE),
            include_str!("../../../registry/tokens.rhai"),
        )
        .unwrap();
        write_manifest(directory.path());

        let prepared = FileScriptView::new(directory.path().join("main.rhai"))
            .prepare()
            .unwrap();
        let (_, lifecycle) = start_prepared(prepared, "widget", "main");
        assert_eq!(
            lifecycle.root().unwrap().style().base.height,
            Some(crate::LayoutLength::Definite(crate::Length::Pixels(41.0)))
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
        .token_base(include_str!("../../../registry/tokens.rhai"))
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
            .instantiate("settings-view", "settings", None)
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
            "entry = \"main\"\nruntime_api = 3\n[capabilities]\n\"app.missing\" = \"*\"\n",
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
    fn embedded_font_sources_are_validated_and_retained_for_mount() {
        let entry = ModuleId::parse("main").unwrap();
        let scripts = EmbeddedScriptSource::new(BTreeMap::from([(
            entry.clone(),
            "fn view(ctx) { text(\"font probe\") }".to_owned(),
        )]));
        let font =
            crate::FontSource::new("Art Display", [b"OTTO".as_slice(), &[0, 1, 2, 3]].concat())
                .unwrap();
        let prepared = EmbeddedScriptView::new(
            entry,
            scripts,
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .font_source(font)
        .prepare()
        .unwrap();
        assert_eq!(prepared.fonts.len(), 1);
        assert_eq!(prepared.fonts[0].label(), "Art Display");
    }

    #[test]
    fn embedded_runtime_clock_is_installed_before_initial_render() {
        let entry = ModuleId::parse("main").unwrap();
        let scripts = EmbeddedScriptSource::new(BTreeMap::from([(
            entry.clone(),
            "fn view(ctx) { text(\"clock probe\") }".to_owned(),
        )]));
        let start = std::time::Instant::now();
        let manual = crate::ManualRuntimeClock::new(start);
        let prepared = EmbeddedScriptView::new(
            entry,
            scripts,
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .runtime_clock(manual.clock())
        .prepare()
        .unwrap();

        manual.advance(std::time::Duration::from_millis(48));
        assert_eq!(
            prepared.factory.runtime.borrow().clock.now(),
            start + std::time::Duration::from_millis(48)
        );
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
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/check.svg"]
}
*/
define_component(#{
    metadata: #{ id: "components/declarative_icon", "export": "DeclarativeIcon",
        version: "0.1.0", runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{}, assets: ["icons/check.svg"] },
    schema: #{ props: #{}, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"] },
    render: Fn("render_DeclarativeIcon")
});
fn DeclarativeIcon(props) { render_component("components/declarative_icon", props) }
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
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [], "capabilities": {}, "assets": ["icons/missing.svg"]
}
*/
define_component(#{ metadata: #{ id: "components/missing_asset", "export": "MissingAsset",
    version: "0.1.0", runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
    dependencies: [], capabilities: #{}, assets: ["icons/missing.svg"] },
    schema: #{ props: #{}, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"] },
    render: Fn("render_MissingAsset") });
fn MissingAsset(props) { render_component("components/missing_asset", props) }
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
            "entry = \"main\"\nruntime_api = 3\n",
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
            let crate::UiNodeKind::Box { children } = root.kind() else {
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
            runtime.windows.drain_commands().iter().map(crate::QueuedWindowCommand::command).collect::<Vec<_>>().as_slice(),
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
        let fire = lifecycle
            .root()
            .unwrap()
            .handler("click")
            .unwrap()
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
