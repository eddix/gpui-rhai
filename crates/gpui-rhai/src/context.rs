use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::time::Duration;

use rhai::{
    CustomType, Dynamic, Engine, EvalAltResult, FLOAT, FnPtr, INT, ImmutableString, Map, Position,
    TypeBuilder,
};
use thiserror::Error;

use crate::{
    ActionError, ActionId, ActionInvocation, ActionRegistry, AnimationKey, AnimationRuntime,
    AssetError, AssetId, AssetRegistry, AsyncRuntimeError, AsyncScope, CalendarClock,
    CapabilityError, CapabilityId, CapabilityRegistry, ComponentInstancePath, DateStyle,
    EventSchema, ImageDecodeHandle, LocaleError, LocaleManager, NumberFormatOptions, OpaqueHandle,
    ResponsiveError, ResponsiveRuntime, ScriptCallback, ScriptGeneration, ScriptWindowSpec,
    StateError, StateStore, StoreError, StoreId, StoreRegistry, SubscriptionHandle,
    SubscriptionRegistry, TaskHandle, TaskRegistry, TextDirection, ThemeError, ThemeManager,
    ThemePreference, ThemeSelection, UiEvent, UiValue, UiValueError, WindowCommandError,
    WindowCommandRegistry,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionPhase {
    Init,
    Render,
    Event,
    Dispose,
}

enum ThemeTarget {
    App,
    Window(String),
    Local,
}

impl ExecutionPhase {
    const fn allows_mutation(self) -> bool {
        matches!(self, Self::Init | Self::Event | Self::Dispose)
    }
}

#[derive(Debug, Default)]
pub struct UiRuntimeState {
    pub component_state: StateStore,
    pub stores: StoreRegistry,
    pub actions: ActionRegistry,
    pub capabilities: CapabilityRegistry,
    pub tasks: TaskRegistry,
    pub subscriptions: SubscriptionRegistry,
    pub locale: Option<LocaleManager>,
    pub calendar_clock: CalendarClock,
    pub theme: Option<ThemeManager>,
    pub assets: AssetRegistry,
    pub animations: AnimationRuntime,
    pub windows: WindowCommandRegistry,
    pub responsive: ResponsiveRuntime,
    pub animation_values: BTreeMap<AnimationKey, f64>,
    pub traces: crate::TraceBuffer,
    component_event_handlers: BTreeMap<(ComponentInstancePath, String), ScriptCallback>,
    dirty: BTreeSet<ComponentInstancePath>,
    pending_events: Vec<PendingEvent>,
    pending_actions: Vec<ActionInvocation>,
    pending_async: Vec<crate::AsyncDelivery>,
}

impl UiRuntimeState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Release every window-owned runtime resource while retaining app scope.
    ///
    /// # Errors
    ///
    /// Returns an asset-registry borrow error if image work is being drained.
    pub fn release_window(
        &mut self,
        window: &str,
        root: &ComponentInstancePath,
    ) -> Result<(), AssetError> {
        self.tasks
            .cancel_scope(&AsyncScope::Window(window.to_owned()));
        self.tasks.cancel_component_scope(root);
        self.subscriptions
            .cancel_scope(&AsyncScope::Window(window.to_owned()));
        self.subscriptions.cancel_component_scope(root);
        self.assets.cancel_window_scope(window, root)?;
        self.stores.remove_window(window);
        self.component_state.remove_scope(root);
        self.component_event_handlers
            .retain(|(path, _), _| !path.is_within(root));
        self.actions.remove_component_scope(root);
        self.dirty.retain(|path| !path.is_within(root));
        self.pending_events
            .retain(|event| !event.target.is_within(root));
        self.pending_actions.retain(|action| {
            !action
                .callback
                .component()
                .is_some_and(|path| path.is_within(root))
        });
        self.pending_async.retain(|delivery| match &delivery.scope {
            AsyncScope::App => true,
            AsyncScope::Window(id) => id != window,
            AsyncScope::Component(path) => !path.is_within(root),
        });
        if let Some(theme) = self.theme.as_mut() {
            theme.remove_window(window);
            theme.remove_scope(root);
        }
        if let Some(locale) = self.locale.as_mut() {
            locale.remove_window(window);
            locale.remove_scope(root);
        }
        self.animations
            .cancel_node_scope(&format!("window:{window}"));
        self.animation_values = self.animations.snapshot(std::time::Instant::now());
        self.windows.remove(window);
        self.responsive.remove_window(window);
        Ok(())
    }

    pub(crate) fn queue_async(
        &mut self,
        deliveries: impl IntoIterator<Item = crate::AsyncDelivery>,
    ) {
        self.pending_async.extend(deliveries);
    }

    pub(crate) fn take_window_async(
        &mut self,
        window: &str,
        root: &ComponentInstancePath,
    ) -> Vec<crate::AsyncDelivery> {
        let app_owner = self.windows.open_ids().into_iter().next();
        let pending = std::mem::take(&mut self.pending_async);
        let (accepted, retained) =
            pending
                .into_iter()
                .partition(|delivery| match &delivery.scope {
                    AsyncScope::App => app_owner.as_deref() == Some(window),
                    AsyncScope::Window(id) => id == window,
                    AsyncScope::Component(path) => path.is_within(root),
                });
        self.pending_async = retained;
        accepted
    }

    pub(crate) fn take_window_dirty(&mut self, root: &ComponentInstancePath) -> bool {
        let mut found = false;
        self.dirty.retain(|path| {
            if path.is_within(root) {
                found = true;
                false
            } else {
                true
            }
        });
        found
    }

    pub(crate) fn drain_pending_actions(&mut self) -> Vec<ActionInvocation> {
        std::mem::take(&mut self.pending_actions)
    }

    pub(crate) fn drain_pending_events(&mut self) -> Vec<PendingEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub(crate) fn component_event_handler(
        &self,
        component: &ComponentInstancePath,
        event: &str,
    ) -> Option<ScriptCallback> {
        self.component_event_handlers
            .get(&(component.clone(), event.to_owned()))
            .cloned()
    }

    pub(crate) fn replace_component_event_handlers(
        &mut self,
        root: &ComponentInstancePath,
        handlers: BTreeMap<(ComponentInstancePath, String), ScriptCallback>,
    ) {
        self.component_event_handlers
            .retain(|(path, _), _| !path.is_within(root));
        self.component_event_handlers.extend(handlers);
    }

    pub(crate) fn mark_all_windows_dirty(&mut self) {
        self.dirty.extend(
            self.windows
                .open_ids()
                .into_iter()
                .map(|window| ComponentInstancePath::root("App", window)),
        );
    }

    pub(crate) fn reconcile_component_lifetimes(
        &mut self,
        root: &ComponentInstancePath,
        active: &BTreeSet<ComponentInstancePath>,
        previous: &BTreeSet<ComponentInstancePath>,
    ) -> Result<(), AssetError> {
        for removed in previous
            .iter()
            .filter(|path| path.is_within(root) && *path != root && !active.contains(*path))
        {
            self.tasks.cancel_component_scope(removed);
            self.subscriptions.cancel_component_scope(removed);
            self.assets.cancel_component_scope(removed)?;
            self.actions.remove_component_scope(removed);
        }
        self.pending_async.retain(|delivery| {
            !matches!(
                &delivery.scope,
                AsyncScope::Component(path)
                    if path.is_within(root) && !active.contains(path)
            )
        });
        self.pending_actions.retain(|action| {
            !action
                .callback
                .component()
                .is_some_and(|path| path.is_within(root) && !active.contains(path))
        });
        self.pending_events.retain(|event| {
            !event.target.is_within(root) || active.contains(&event.target) || event.target == *root
        });
        self.stores.retain_reader_scope(root, active);
        self.dirty
            .retain(|path| !path.is_within(root) || path == root || active.contains(path));
        Ok(())
    }

    #[must_use]
    pub fn drain_batch(&mut self) -> UiMutationBatch {
        UiMutationBatch {
            dirty: std::mem::take(&mut self.dirty),
            events: std::mem::take(&mut self.pending_events),
            actions: std::mem::take(&mut self.pending_actions),
        }
    }

    #[must_use]
    pub fn dirty_components(&self) -> &BTreeSet<ComponentInstancePath> {
        &self.dirty
    }

    /// Capture rollback-capable UI state and async ownership checkpoints.
    ///
    /// # Errors
    ///
    /// Returns an asset-registry borrow error while decode state is in use.
    pub fn snapshot(&self) -> Result<UiStateSnapshot, AssetError> {
        Ok(UiStateSnapshot {
            component_state: self.component_state.clone(),
            stores: self.stores.clone(),
            actions: self.actions.clone(),
            component_event_handlers: self.component_event_handlers.clone(),
            dirty: self.dirty.clone(),
            pending_events: self.pending_events.clone(),
            pending_actions: self.pending_actions.clone(),
            pending_async: self.pending_async.clone(),
            locale: self.locale.clone(),
            theme: self.theme.clone(),
            animations: self.animations.clone(),
            animation_values: self.animation_values.clone(),
            windows: self.windows.clone(),
            responsive: self.responsive.clone(),
            task_ids: self.tasks.active_ids(),
            subscription_ids: self.subscriptions.active_ids(),
            decode_ids: self.assets.pending_decode_ids()?,
        })
    }

    /// Restore a snapshot and cancel async/image work created after it.
    ///
    /// # Errors
    ///
    /// Returns an asset-registry borrow error while decode state is in use.
    pub fn restore(&mut self, snapshot: UiStateSnapshot) -> Result<(), AssetError> {
        self.tasks.retain_ids(&snapshot.task_ids);
        self.subscriptions.retain_ids(&snapshot.subscription_ids);
        self.assets.retain_decode_ids(&snapshot.decode_ids)?;
        self.component_state = snapshot.component_state;
        self.stores = snapshot.stores;
        self.actions = snapshot.actions;
        self.component_event_handlers = snapshot.component_event_handlers;
        self.dirty = snapshot.dirty;
        self.pending_events = snapshot.pending_events;
        self.pending_actions = snapshot.pending_actions;
        self.pending_async = snapshot.pending_async;
        self.locale = snapshot.locale;
        self.theme = snapshot.theme;
        self.animations = snapshot.animations;
        self.animation_values = snapshot.animation_values;
        self.windows = snapshot.windows;
        self.responsive = snapshot.responsive;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct UiStateSnapshot {
    component_state: StateStore,
    stores: StoreRegistry,
    actions: ActionRegistry,
    component_event_handlers: BTreeMap<(ComponentInstancePath, String), ScriptCallback>,
    dirty: BTreeSet<ComponentInstancePath>,
    pending_events: Vec<PendingEvent>,
    pending_actions: Vec<ActionInvocation>,
    pending_async: Vec<crate::AsyncDelivery>,
    locale: Option<LocaleManager>,
    theme: Option<ThemeManager>,
    animations: AnimationRuntime,
    animation_values: BTreeMap<AnimationKey, f64>,
    windows: WindowCommandRegistry,
    responsive: ResponsiveRuntime,
    task_ids: BTreeSet<u64>,
    subscription_ids: BTreeSet<u64>,
    decode_ids: BTreeSet<u64>,
}

#[derive(Clone, Debug)]
pub struct PendingEvent {
    pub target: ComponentInstancePath,
    pub event: UiEvent,
}

#[derive(Clone, Debug, Default)]
pub struct UiMutationBatch {
    pub dirty: BTreeSet<ComponentInstancePath>,
    pub events: Vec<PendingEvent>,
    pub actions: Vec<ActionInvocation>,
}

#[derive(Clone, Debug)]
pub struct UiContext {
    runtime: Rc<RefCell<UiRuntimeState>>,
    component: ComponentInstancePath,
    window: Option<String>,
    view: Option<String>,
    phase: ExecutionPhase,
    events: BTreeMap<String, EventSchema>,
    generation: ScriptGeneration,
    native_context: Option<crate::engine::ScriptNativeContext>,
}

impl UiContext {
    #[must_use]
    pub fn new(
        runtime: Rc<RefCell<UiRuntimeState>>,
        component: ComponentInstancePath,
        window: Option<String>,
        phase: ExecutionPhase,
        events: BTreeMap<String, EventSchema>,
    ) -> Self {
        let context = Self {
            runtime,
            component,
            window,
            view: None,
            phase,
            events,
            generation: ScriptGeneration::default(),
            native_context: None,
        };
        if phase == ExecutionPhase::Render
            && let Ok(mut runtime) = context.runtime.try_borrow_mut()
        {
            runtime.stores.reset_reader(&context.component);
        }
        context
    }

    #[must_use]
    pub fn with_generation(mut self, generation: ScriptGeneration) -> Self {
        self.generation = generation;
        self
    }

    #[must_use]
    pub fn with_view_id(mut self, view: impl Into<String>) -> Self {
        self.view = Some(view.into());
        self
    }

    #[must_use]
    pub(crate) fn with_optional_view_id(mut self, view: Option<String>) -> Self {
        self.view = view;
        self
    }

    pub(crate) fn for_component(
        &self,
        component: ComponentInstancePath,
        events: BTreeMap<String, EventSchema>,
    ) -> Self {
        let context = Self {
            runtime: Rc::clone(&self.runtime),
            component,
            window: self.window.clone(),
            view: self.view.clone(),
            phase: self.phase,
            events,
            generation: self.generation,
            native_context: self.native_context.clone(),
        };
        if context.phase == ExecutionPhase::Render
            && let Ok(mut runtime) = context.runtime.try_borrow_mut()
        {
            runtime.stores.reset_reader(&context.component);
        }
        context
    }

    pub(crate) fn with_native_context(
        mut self,
        native_context: Option<crate::engine::ScriptNativeContext>,
    ) -> Self {
        self.native_context = native_context;
        self
    }

    fn scoped_callback(&self, function: FnPtr) -> ScriptCallback {
        let mut callback = ScriptCallback::from_fn_ptr(function, self.generation);
        callback.bind_component_if_unset(self.component.clone(), self.events.clone());
        if let Some(context) = &self.native_context {
            callback.bind_native_context_if_unset(Rc::clone(context));
        }
        callback
    }

    #[must_use]
    pub fn runtime(&self) -> &Rc<RefCell<UiRuntimeState>> {
        &self.runtime
    }

    pub(crate) fn component_path(&self) -> &ComponentInstancePath {
        &self.component
    }

    pub(crate) fn event_schemas(&self) -> &BTreeMap<String, EventSchema> {
        &self.events
    }

    /// Read declared local component state.
    ///
    /// # Errors
    ///
    /// Returns [`UiContextError`] for a poisoned runtime, unknown instance, or
    /// unknown state field.
    pub fn get_state(&self, field: &str) -> Result<UiValue, UiContextError> {
        self.runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?
            .component_state
            .get(&self.component, field)
            .cloned()
            .ok_or_else(|| UiContextError::UnknownState {
                component: self.component.clone(),
                field: field.to_owned(),
            })
    }

    /// Queue a schema-checked local state mutation.
    ///
    /// # Errors
    ///
    /// Returns [`UiContextError::MutationDuringRender`] in `view`, plus state
    /// and value-conversion errors.
    pub fn set_state(&self, field: &str, value: Dynamic) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let value = UiValue::from_dynamic(value)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let sensitive = runtime.component_state.is_sensitive(&self.component, field);
        runtime
            .component_state
            .set(&self.component, field, value.clone())?;
        runtime.dirty.insert(self.component.clone());
        runtime.traces.push(
            crate::RuntimeTraceKind::State,
            self.component.to_string(),
            format!("set {field}"),
            Some(value),
            sensitive,
        );
        Ok(())
    }

    /// Read and subscribe to an app-scoped store field.
    ///
    /// # Errors
    ///
    /// Returns store or lock errors.
    pub fn get_app_store(&self, store: &str, field: &str) -> Result<UiValue, UiContextError> {
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        Ok(runtime
            .stores
            .read_tracked(&self.component, &StoreId::app(store), field)?)
    }

    /// Queue a schema-checked app store mutation.
    ///
    /// # Errors
    ///
    /// Returns phase, conversion, store, or lock errors.
    pub fn set_app_store(
        &self,
        store: &str,
        field: &str,
        value: Dynamic,
    ) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let value = UiValue::from_dynamic(value)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let id = StoreId::app(store);
        let sensitive = runtime.stores.is_sensitive(&id, field);
        let invalidated = runtime.stores.write(&id, field, value.clone())?;
        runtime.dirty.extend(invalidated);
        runtime.traces.push(
            crate::RuntimeTraceKind::Store,
            format!("app:{store}"),
            format!("set {field}"),
            Some(value),
            sensitive,
        );
        Ok(())
    }

    /// Read and subscribe to a store scoped to this context's window.
    ///
    /// # Errors
    ///
    /// Returns [`UiContextError::MissingWindow`] outside a window, plus store
    /// and lock errors.
    pub fn get_window_store(&self, store: &str, field: &str) -> Result<UiValue, UiContextError> {
        let window = self.window.as_ref().ok_or(UiContextError::MissingWindow)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        Ok(runtime
            .stores
            .read_tracked(&self.component, &StoreId::window(window, store), field)?)
    }

    /// Queue a schema-checked mutation in the current window's store.
    ///
    /// # Errors
    ///
    /// Returns phase, missing-window, conversion, store, or borrow errors.
    pub fn set_window_store(
        &self,
        store: &str,
        field: &str,
        value: Dynamic,
    ) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let window = self.window.as_ref().ok_or(UiContextError::MissingWindow)?;
        let value = UiValue::from_dynamic(value)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let id = StoreId::window(window, store);
        let sensitive = runtime.stores.is_sensitive(&id, field);
        let invalidated = runtime.stores.write(&id, field, value.clone())?;
        runtime.dirty.extend(invalidated);
        runtime.traces.push(
            crate::RuntimeTraceKind::Store,
            format!("window:{window}:{store}"),
            format!("set {field}"),
            Some(value),
            sensitive,
        );
        Ok(())
    }

    /// Emit a declared semantic component event.
    ///
    /// # Errors
    ///
    /// Returns phase, schema, conversion, or lock errors.
    pub fn emit(&self, event: &str, payload: Dynamic) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let schema = self
            .events
            .get(event)
            .ok_or_else(|| UiContextError::UnknownEvent(event.to_owned()))?;
        schema
            .payload
            .validate(&payload)
            .map_err(UiContextError::InvalidEvent)?;
        let payload = UiValue::from_dynamic(payload)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        runtime.pending_events.push(PendingEvent {
            target: self.component.clone(),
            event: UiEvent {
                name: event.to_owned(),
                payload: payload.clone(),
            },
        });
        runtime.traces.push(
            crate::RuntimeTraceKind::Event,
            self.component.to_string(),
            format!("emit {event}"),
            Some(payload),
            false,
        );
        Ok(())
    }

    /// Dispatch a registered semantic action.
    ///
    /// # Errors
    ///
    /// Returns phase, action, conversion, or lock errors.
    pub fn dispatch_action(&self, action: &str, payload: Dynamic) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let id = ActionId::parse(action)?;
        let payload = UiValue::from_dynamic(payload)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let invocation = runtime.actions.dispatch(&id, payload.clone())?;
        runtime.pending_actions.push(invocation);
        runtime.traces.push(
            crate::RuntimeTraceKind::Action,
            self.component.to_string(),
            format!("dispatch {action}"),
            Some(payload),
            false,
        );
        Ok(())
    }

    /// Register or refresh an app-scoped semantic action callback.
    ///
    /// # Errors
    ///
    /// Returns phase, identifier, or borrow errors.
    pub fn register_action(&self, action: &str, callback: FnPtr) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let id = ActionId::parse(action)?;
        self.runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?
            .actions
            .register_or_replace(id, self.scoped_callback(callback));
        Ok(())
    }

    /// Enable or disable a registered semantic action.
    ///
    /// # Errors
    ///
    /// Returns phase, identifier, unknown-action, or borrow errors.
    pub fn set_action_enabled(&self, action: &str, enabled: bool) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let id = ActionId::parse(action)?;
        self.runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?
            .actions
            .set_enabled(&id, enabled)?;
        Ok(())
    }

    /// Invoke a manifest-declared Rust capability.
    ///
    /// # Errors
    ///
    /// Returns phase, identifier, schema, handler, conversion, or borrow errors.
    pub fn call_capability(
        &self,
        capability: &str,
        method: &str,
        input: Dynamic,
    ) -> Result<UiValue, UiContextError> {
        self.require_mutation()?;
        let id = CapabilityId::parse(capability)?;
        let input = UiValue::from_dynamic(input)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let output = runtime.capabilities.call(&id, method, input)?;
        runtime.traces.push(
            crate::RuntimeTraceKind::Capability,
            self.component.to_string(),
            format!("call {capability}.{method}"),
            None,
            true,
        );
        Ok(output)
    }

    /// Resolve one localized message for this window/component scope.
    ///
    /// # Errors
    ///
    /// Returns [`UiContextError::LocaleUnavailable`], locale, or borrow errors.
    pub fn text(&self, key: &str) -> Result<String, UiContextError> {
        let runtime = self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?;
        let locale = runtime
            .locale
            .as_ref()
            .ok_or(UiContextError::LocaleUnavailable)?;
        Ok(locale.text(self.window.as_deref(), Some(&self.component), key)?)
    }

    /// Resolve the logical text direction for this component scope.
    ///
    /// # Errors
    ///
    /// Returns [`UiContextError::LocaleUnavailable`], locale, or borrow errors.
    pub fn text_direction(&self) -> Result<String, UiContextError> {
        let runtime = self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?;
        let locale = runtime
            .locale
            .as_ref()
            .ok_or(UiContextError::LocaleUnavailable)?;
        match locale.direction(self.window.as_deref(), Some(&self.component))? {
            TextDirection::LeftToRight => Ok("ltr".to_owned()),
            TextDirection::RightToLeft => Ok("rtl".to_owned()),
        }
    }

    /// Return today's strict ISO date from the host-injected calendar Clock.
    ///
    /// This is a read-only presentation input and is available during render.
    ///
    /// # Errors
    ///
    /// Returns a borrow error when another callback owns the runtime state.
    pub fn today(&self) -> Result<String, UiContextError> {
        Ok(self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?
            .calendar_clock
            .today()
            .to_iso())
    }

    /// Format a strict ISO date through the selected locale.
    ///
    /// # Errors
    ///
    /// Returns locale, date, style, or runtime borrow errors.
    pub fn format_date(&self, iso_date: &str, style: &str) -> Result<String, UiContextError> {
        let runtime = self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?;
        let locale = runtime
            .locale
            .as_ref()
            .ok_or(UiContextError::LocaleUnavailable)?;
        Ok(locale.format_date(
            self.window.as_deref(),
            Some(&self.component),
            iso_date,
            DateStyle::parse(style)?,
        )?)
    }

    /// Return a detached read-only copy of selected calendar metadata.
    ///
    /// # Errors
    ///
    /// Returns locale, serialization, or runtime borrow errors.
    pub fn calendar_metadata(&self) -> Result<Map, UiContextError> {
        let runtime = self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?;
        let locale = runtime
            .locale
            .as_ref()
            .ok_or(UiContextError::LocaleUnavailable)?;
        let calendar = locale.calendar(self.window.as_deref(), Some(&self.component))?;
        let dynamic = rhai::serde::to_dynamic(calendar)
            .map_err(|error| LocaleError::Decode(error.to_string()))?;
        Ok(dynamic.cast::<Map>())
    }

    /// Return a detached read-only copy of selected number metadata.
    ///
    /// # Errors
    ///
    /// Returns locale, serialization, or runtime borrow errors.
    pub fn number_metadata(&self) -> Result<Map, UiContextError> {
        let runtime = self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?;
        let locale = runtime
            .locale
            .as_ref()
            .ok_or(UiContextError::LocaleUnavailable)?;
        let number = locale.number(self.window.as_deref(), Some(&self.component))?;
        let dynamic = rhai::serde::to_dynamic(number)
            .map_err(|error| LocaleError::Decode(error.to_string()))?;
        Ok(dynamic.cast::<Map>())
    }

    /// Format an integer through the selected locale.
    ///
    /// # Errors
    ///
    /// Returns locale, option, or runtime borrow errors.
    pub fn format_integer(
        &self,
        value: INT,
        options: NumberFormatOptions,
    ) -> Result<String, UiContextError> {
        let runtime = self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?;
        let locale = runtime
            .locale
            .as_ref()
            .ok_or(UiContextError::LocaleUnavailable)?;
        Ok(locale.format_integer(
            self.window.as_deref(),
            Some(&self.component),
            value,
            options,
        )?)
    }

    /// Format a finite decimal number through the selected locale.
    ///
    /// # Errors
    ///
    /// Returns locale, option, or runtime borrow errors.
    pub fn format_number(
        &self,
        value: FLOAT,
        options: NumberFormatOptions,
    ) -> Result<String, UiContextError> {
        let runtime = self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?;
        let locale = runtime
            .locale
            .as_ref()
            .ok_or(UiContextError::LocaleUnavailable)?;
        Ok(locale.format_number(
            self.window.as_deref(),
            Some(&self.component),
            value,
            options,
        )?)
    }

    /// Change the app locale at runtime without changing component state.
    ///
    /// # Errors
    ///
    /// Returns phase, locale, or borrow errors.
    pub fn set_locale(&self, locale: &str) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        runtime
            .locale
            .as_mut()
            .ok_or(UiContextError::LocaleUnavailable)?
            .set_app(locale)?;
        runtime.mark_all_windows_dirty();
        runtime.traces.push(
            crate::RuntimeTraceKind::Locale,
            self.component.to_string(),
            format!("select {locale}"),
            None,
            false,
        );
        Ok(())
    }

    /// Select an application theme without recompiling scripts or resetting state.
    ///
    /// # Errors
    ///
    /// Returns phase, theme availability, selection, or borrow errors.
    pub fn set_theme(&self, family: &str, variant: &str) -> Result<(), UiContextError> {
        self.set_theme_preference(
            ThemeTarget::App,
            ThemePreference::Fixed {
                selection: ThemeSelection::new(family, variant),
            },
            format!("select {family}/{variant}"),
        )
    }

    /// Select a theme for this context's window.
    ///
    /// # Errors
    ///
    /// Returns phase, missing-window, theme, or borrow errors.
    pub fn set_window_theme(&self, family: &str, variant: &str) -> Result<(), UiContextError> {
        let window = self.window.clone().ok_or(UiContextError::MissingWindow)?;
        self.set_theme_preference(
            ThemeTarget::Window(window),
            ThemePreference::Fixed {
                selection: ThemeSelection::new(family, variant),
            },
            format!("select window {family}/{variant}"),
        )
    }

    /// Return the stable script-visible ID of the current window.
    ///
    /// # Errors
    ///
    /// Returns [`UiContextError::MissingWindow`] outside a window lifecycle.
    pub fn window_id(&self) -> Result<String, UiContextError> {
        self.window.clone().ok_or(UiContextError::MissingWindow)
    }

    /// Return the stable identity of the mounted script view.
    ///
    /// # Errors
    ///
    /// Returns [`UiContextError::MissingView`] for manually constructed contexts
    /// that are not attached to a script view.
    pub fn view_id(&self) -> Result<String, UiContextError> {
        self.view.clone().ok_or(UiContextError::MissingView)
    }

    /// Return `compact`, `regular`, or `wide` for the current native window.
    ///
    /// # Errors
    ///
    /// Returns [`UiContextError::MissingWindow`] outside a window lifecycle.
    pub fn viewport_class(&self) -> Result<String, UiContextError> {
        let window = self.window.as_ref().ok_or(UiContextError::MissingWindow)?;
        Ok(self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?
            .responsive
            .class(window)
            .as_str()
            .to_owned())
    }

    /// Queue another native window running the same script entry.
    ///
    /// # Errors
    ///
    /// Returns phase, missing-window, validation, duplicate, or queue errors.
    pub fn open_window(&self, spec: ScriptWindowSpec) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let window = self.window.as_ref().ok_or(UiContextError::MissingWindow)?;
        let id = spec.id.clone();
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        runtime.windows.request_open_from(window, spec)?;
        runtime.traces.push(
            crate::RuntimeTraceKind::Window,
            self.component.to_string(),
            format!("open {id}"),
            None,
            false,
        );
        Ok(())
    }

    /// Queue activation of a registered native window.
    ///
    /// # Errors
    ///
    /// Returns phase, registry, or queue errors.
    pub fn focus_window(&self, id: &str) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let window = self.window.as_ref().ok_or(UiContextError::MissingWindow)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        runtime.windows.request_focus_from(window, id)?;
        runtime.traces.push(
            crate::RuntimeTraceKind::Window,
            self.component.to_string(),
            format!("focus {id}"),
            None,
            false,
        );
        Ok(())
    }

    /// Queue forced close after the script has performed any confirmation UI.
    ///
    /// # Errors
    ///
    /// Returns phase, registry, or queue errors.
    pub fn close_window(&self, id: &str) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let window = self.window.as_ref().ok_or(UiContextError::MissingWindow)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        runtime.windows.request_close_from(window, id)?;
        runtime.traces.push(
            crate::RuntimeTraceKind::Window,
            self.component.to_string(),
            format!("close {id}"),
            None,
            false,
        );
        Ok(())
    }

    /// Intercept native close requests with a generation-bound script callback.
    /// The callback may show confirmation UI and later call `close_window`.
    ///
    /// # Errors
    ///
    /// Returns phase, missing-window, registry, or borrow errors.
    pub fn set_close_handler(&self, callback: FnPtr) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let window = self.window.as_ref().ok_or(UiContextError::MissingWindow)?;
        self.runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?
            .windows
            .set_close_handler(window, Some(self.scoped_callback(callback)))?;
        Ok(())
    }

    /// Remove the current window's close-request interception.
    ///
    /// # Errors
    ///
    /// Returns phase, missing-window, registry, or borrow errors.
    pub fn clear_close_handler(&self) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let window = self.window.as_ref().ok_or(UiContextError::MissingWindow)?;
        self.runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?
            .windows
            .set_close_handler(window, None)?;
        Ok(())
    }

    /// Select a locale override for the current window.
    ///
    /// # Errors
    ///
    /// Returns phase, missing-window, locale, or borrow errors.
    pub fn set_window_locale(&self, locale: &str) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let window = self.window.clone().ok_or(UiContextError::MissingWindow)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        runtime
            .locale
            .as_mut()
            .ok_or(UiContextError::LocaleUnavailable)?
            .set_window(window.clone(), locale)?;
        runtime.dirty.insert(self.component.clone());
        runtime.traces.push(
            crate::RuntimeTraceKind::Locale,
            self.component.to_string(),
            format!("select window {window} locale {locale}"),
            None,
            false,
        );
        Ok(())
    }

    /// Select a locale override for the current component subtree.
    ///
    /// # Errors
    ///
    /// Returns phase, locale, or borrow errors.
    pub fn set_local_locale(&self, locale: &str) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        runtime
            .locale
            .as_mut()
            .ok_or(UiContextError::LocaleUnavailable)?
            .set_scope(self.component.clone(), locale)?;
        runtime.dirty.insert(self.component.clone());
        runtime.traces.push(
            crate::RuntimeTraceKind::Locale,
            self.component.to_string(),
            format!("select subtree locale {locale}"),
            None,
            false,
        );
        Ok(())
    }

    /// Select a theme for the current component subtree.
    ///
    /// # Errors
    ///
    /// Returns phase, theme, or borrow errors.
    pub fn set_local_theme(&self, family: &str, variant: &str) -> Result<(), UiContextError> {
        self.set_theme_preference(
            ThemeTarget::Local,
            ThemePreference::Fixed {
                selection: ThemeSelection::new(family, variant),
            },
            format!("select subtree {family}/{variant}"),
        )
    }

    /// Follow system appearance for one application theme family.
    ///
    /// # Errors
    ///
    /// Returns phase, theme, or borrow errors.
    pub fn set_theme_system(&self, family: &str) -> Result<(), UiContextError> {
        self.set_theme_preference(
            ThemeTarget::App,
            ThemePreference::System {
                family: family.to_owned(),
            },
            format!("follow system family {family}"),
        )
    }

    /// Follow system appearance for the current window.
    ///
    /// # Errors
    ///
    /// Returns phase, missing-window, theme, or borrow errors.
    pub fn set_window_theme_system(&self, family: &str) -> Result<(), UiContextError> {
        let window = self.window.clone().ok_or(UiContextError::MissingWindow)?;
        self.set_theme_preference(
            ThemeTarget::Window(window),
            ThemePreference::System {
                family: family.to_owned(),
            },
            format!("follow window system family {family}"),
        )
    }

    /// Follow system appearance for the current component subtree.
    ///
    /// # Errors
    ///
    /// Returns phase, theme, or borrow errors.
    pub fn set_local_theme_system(&self, family: &str) -> Result<(), UiContextError> {
        self.set_theme_preference(
            ThemeTarget::Local,
            ThemePreference::System {
                family: family.to_owned(),
            },
            format!("follow subtree system family {family}"),
        )
    }

    fn set_theme_preference(
        &self,
        target: ThemeTarget,
        preference: ThemePreference,
        message: String,
    ) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let theme = runtime
            .theme
            .as_mut()
            .ok_or(UiContextError::ThemeUnavailable)?;
        let app_target = matches!(&target, ThemeTarget::App);
        match target {
            ThemeTarget::App => theme.set_app(preference)?,
            ThemeTarget::Window(window) => theme.set_window(window, preference)?,
            ThemeTarget::Local => theme.set_scope(self.component.clone(), preference)?,
        }
        if app_target {
            runtime.mark_all_windows_dirty();
        } else {
            runtime.dirty.insert(self.component.clone());
        }
        runtime.traces.push(
            crate::RuntimeTraceKind::Theme,
            self.component.to_string(),
            message,
            None,
            false,
        );
        Ok(())
    }

    /// Change the central motion preference and settle nonessential animation
    /// immediately when reduced motion is requested.
    ///
    /// # Errors
    ///
    /// Returns phase or runtime borrow errors.
    pub fn set_reduced_motion(&self, reduced: bool) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        runtime.animations.set_preference(if reduced {
            crate::MotionPreference::Reduced
        } else {
            crate::MotionPreference::Normal
        });
        runtime.animation_values = runtime.animations.snapshot(std::time::Instant::now());
        runtime.dirty.insert(self.component.clone());
        runtime.traces.push(
            crate::RuntimeTraceKind::State,
            self.component.to_string(),
            format!("reduced motion {reduced}"),
            None,
            false,
        );
        Ok(())
    }

    /// Load and cache a logical image asset.
    ///
    /// # Errors
    ///
    /// Returns phase, provider, format, or borrow errors.
    pub fn load_image(&self, asset: &AssetId) -> Result<OpaqueHandle, UiContextError> {
        self.require_mutation()?;
        Ok(self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?
            .assets
            .load_image(asset)?
            .opaque()
            .clone())
    }

    /// Start generation-bound background raster validation/decode.
    ///
    /// # Errors
    ///
    /// Returns phase, generation, provider, spawn, or borrow errors.
    pub fn start_image_decode(
        &self,
        asset: &AssetId,
        success: FnPtr,
        error: FnPtr,
    ) -> Result<ImageDecodeHandle, UiContextError> {
        self.require_mutation()?;
        self.require_generation()?;
        let success = self.scoped_callback(success);
        let error = self.scoped_callback(error);
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let handle = runtime.assets.start_image_decode(
            asset,
            AsyncScope::Component(self.component.clone()),
            self.generation,
            success,
            error,
        )?;
        runtime.traces.push(
            crate::RuntimeTraceKind::Task,
            self.component.to_string(),
            format!("decode image {}", asset.as_str()),
            None,
            false,
        );
        Ok(handle)
    }

    /// Cancel a pending image decode owned by this runtime.
    ///
    /// # Errors
    ///
    /// Returns phase or borrow errors.
    pub fn cancel_image_decode(&self, handle: ImageDecodeHandle) -> Result<bool, UiContextError> {
        self.require_mutation()?;
        Ok(self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?
            .assets
            .cancel_image_decode(handle)?)
    }

    /// Cancel a one-shot task owned by this runtime.
    ///
    /// # Errors
    ///
    /// Returns phase or borrow errors.
    pub fn cancel_task(&self, handle: TaskHandle) -> Result<bool, UiContextError> {
        self.require_mutation()?;
        Ok(self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?
            .tasks
            .cancel(handle))
    }

    /// Cancel a continuous subscription owned by this runtime.
    ///
    /// # Errors
    ///
    /// Returns phase or borrow errors.
    pub fn cancel_subscription(&self, handle: SubscriptionHandle) -> Result<bool, UiContextError> {
        self.require_mutation()?;
        Ok(self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?
            .subscriptions
            .cancel(handle))
    }

    /// Start a one-shot asynchronous capability call.
    ///
    /// # Errors
    ///
    /// Returns phase, capability, schema, generation, spawn, or borrow errors.
    pub fn start_task(
        &self,
        capability: &str,
        method: &str,
        input: Dynamic,
        success: FnPtr,
        error: FnPtr,
    ) -> Result<TaskHandle, UiContextError> {
        self.require_mutation()?;
        self.require_generation()?;
        let id = CapabilityId::parse(capability)?;
        let input = UiValue::from_dynamic(input)?;
        let success = self.scoped_callback(success);
        let error = self.scoped_callback(error);
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let (work, output) = runtime.capabilities.start_task(&id, method, input)?;
        let handle = runtime.tasks.spawn(
            AsyncScope::Component(self.component.clone()),
            self.generation,
            success,
            error,
            output,
            work,
        )?;
        runtime.traces.push(
            crate::RuntimeTraceKind::Task,
            self.component.to_string(),
            format!("start {capability}.{method}"),
            None,
            true,
        );
        Ok(handle)
    }

    /// Start a continuous asynchronous capability subscription.
    ///
    /// # Errors
    ///
    /// Returns phase, capability, schema, generation, spawn, or borrow errors.
    pub fn start_subscription(
        &self,
        capability: &str,
        method: &str,
        input: Dynamic,
        success: FnPtr,
        error: FnPtr,
        throttle: Duration,
    ) -> Result<SubscriptionHandle, UiContextError> {
        self.require_mutation()?;
        self.require_generation()?;
        let id = CapabilityId::parse(capability)?;
        let input = UiValue::from_dynamic(input)?;
        let success = self.scoped_callback(success);
        let error = self.scoped_callback(error);
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let (work, output) = runtime
            .capabilities
            .start_subscription(&id, method, input)?;
        let (handle, emitter) = runtime.subscriptions.subscribe(
            AsyncScope::Component(self.component.clone()),
            self.generation,
            success,
            error,
            output,
            throttle,
        );
        let closer = emitter.clone();
        if let Err(spawn_error) = std::thread::Builder::new()
            .name("gpui-rhai-subscription".to_owned())
            .spawn(move || {
                work(emitter);
                closer.close();
            })
        {
            let _ = runtime.subscriptions.cancel(handle);
            return Err(AsyncRuntimeError::Spawn(spawn_error).into());
        }
        runtime.traces.push(
            crate::RuntimeTraceKind::Subscription,
            self.component.to_string(),
            format!("start {capability}.{method}"),
            None,
            true,
        );
        Ok(handle)
    }

    fn require_mutation(&self) -> Result<(), UiContextError> {
        if self.phase.allows_mutation() {
            Ok(())
        } else {
            Err(UiContextError::MutationDuringRender)
        }
    }

    fn require_generation(&self) -> Result<(), UiContextError> {
        if self.generation == ScriptGeneration::default() {
            Err(UiContextError::MissingGeneration)
        } else {
            Ok(())
        }
    }
}

impl CustomType for UiContext {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("UiContext")
            .with_fn("get_state", |context: &mut Self, field: ImmutableString| {
                context
                    .get_state(field.as_str())
                    .map(UiValue::into_dynamic)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            })
            .with_fn(
                "set_state",
                |context: &mut Self, field: ImmutableString, value: Dynamic| {
                    context
                        .set_state(field.as_str(), value)
                        .map_err(|error| Box::new(context_runtime_error(&error)))
                },
            )
            .with_fn(
                "emit",
                |context: &mut Self, event: ImmutableString, payload: Dynamic| {
                    context
                        .emit(event.as_str(), payload)
                        .map_err(|error| Box::new(context_runtime_error(&error)))
                },
            )
            .with_fn(
                "get_app_store",
                |context: &mut Self, store: ImmutableString, field: ImmutableString| {
                    context
                        .get_app_store(store.as_str(), field.as_str())
                        .map(UiValue::into_dynamic)
                        .map_err(|error| Box::new(context_runtime_error(&error)))
                },
            )
            .with_fn(
                "set_app_store",
                |context: &mut Self,
                 store: ImmutableString,
                 field: ImmutableString,
                 value: Dynamic| {
                    context
                        .set_app_store(store.as_str(), field.as_str(), value)
                        .map_err(|error| Box::new(context_runtime_error(&error)))
                },
            )
            .with_fn(
                "get_window_store",
                |context: &mut Self, store: ImmutableString, field: ImmutableString| {
                    context
                        .get_window_store(store.as_str(), field.as_str())
                        .map(UiValue::into_dynamic)
                        .map_err(|error| Box::new(context_runtime_error(&error)))
                },
            )
            .with_fn(
                "set_window_store",
                |context: &mut Self,
                 store: ImmutableString,
                 field: ImmutableString,
                 value: Dynamic| {
                    context
                        .set_window_store(store.as_str(), field.as_str(), value)
                        .map_err(|error| Box::new(context_runtime_error(&error)))
                },
            )
            .with_fn(
                "call_capability",
                |context: &mut Self,
                 capability: ImmutableString,
                 method: ImmutableString,
                 input: Dynamic| {
                    context
                        .call_capability(capability.as_str(), method.as_str(), input)
                        .map(UiValue::into_dynamic)
                        .map_err(|error| Box::new(context_runtime_error(&error)))
                },
            );
        register_async_context_methods(&mut builder);
        register_action_context_methods(&mut builder);
        register_locale_context_methods(&mut builder);
        register_theme_context_methods(&mut builder);
        register_motion_context_methods(&mut builder);
        register_asset_context_methods(&mut builder);
        register_window_context_methods(&mut builder);
    }
}

fn register_action_context_methods(builder: &mut TypeBuilder<UiContext>) {
    builder
        .with_fn(
            "dispatch_action",
            |context: &mut UiContext, action: ImmutableString, payload: Dynamic| {
                context
                    .dispatch_action(action.as_str(), payload)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "register_action",
            |context: &mut UiContext, action: ImmutableString, callback: FnPtr| {
                context
                    .register_action(action.as_str(), callback)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_action_enabled",
            |context: &mut UiContext, action: ImmutableString, enabled: bool| {
                context
                    .set_action_enabled(action.as_str(), enabled)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        );
}

fn register_async_context_methods(builder: &mut TypeBuilder<UiContext>) {
    builder
        .with_fn(
            "start_task",
            |context: &mut UiContext,
             capability: ImmutableString,
             method: ImmutableString,
             input: Dynamic,
             success: FnPtr,
             error: FnPtr| {
                context
                    .start_task(capability.as_str(), method.as_str(), input, success, error)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "start_subscription",
            |context: &mut UiContext,
             capability: ImmutableString,
             method: ImmutableString,
             input: Dynamic,
             success: FnPtr,
             error: FnPtr,
             throttle_ms: rhai::INT| {
                let throttle_ms = u64::try_from(throttle_ms).map_err(|_| {
                    Box::new(EvalAltResult::ErrorRuntime(
                        "subscription throttle must be non-negative".into(),
                        Position::NONE,
                    ))
                })?;
                context
                    .start_subscription(
                        capability.as_str(),
                        method.as_str(),
                        input,
                        success,
                        error,
                        Duration::from_millis(throttle_ms),
                    )
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "cancel_task",
            |context: &mut UiContext, handle: TaskHandle| {
                context
                    .cancel_task(handle)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "cancel_subscription",
            |context: &mut UiContext, handle: SubscriptionHandle| {
                context
                    .cancel_subscription(handle)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        );
}

fn register_locale_context_methods(builder: &mut TypeBuilder<UiContext>) {
    builder
        .with_fn("t", |context: &mut UiContext, key: ImmutableString| {
            context
                .text(key.as_str())
                .map_err(|error| Box::new(context_runtime_error(&error)))
        })
        .with_fn(
            "set_locale",
            |context: &mut UiContext, locale: ImmutableString| {
                context
                    .set_locale(locale.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn("text_direction", |context: &mut UiContext| {
            context
                .text_direction()
                .map_err(|error| Box::new(context_runtime_error(&error)))
        })
        .with_fn("today", |context: &mut UiContext| {
            context
                .today()
                .map_err(|error| Box::new(context_runtime_error(&error)))
        })
        .with_fn(
            "format_date",
            |context: &mut UiContext, date: ImmutableString, style: ImmutableString| {
                context
                    .format_date(date.as_str(), style.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn("calendar", |context: &mut UiContext| {
            context
                .calendar_metadata()
                .map_err(|error| Box::new(context_runtime_error(&error)))
        })
        .with_fn("number", |context: &mut UiContext| {
            context
                .number_metadata()
                .map_err(|error| Box::new(context_runtime_error(&error)))
        })
        .with_fn("format_number", |context: &mut UiContext, value: INT| {
            context
                .format_integer(value, NumberFormatOptions::default())
                .map_err(|error| Box::new(context_runtime_error(&error)))
        })
        .with_fn("format_number", |context: &mut UiContext, value: FLOAT| {
            context
                .format_number(value, NumberFormatOptions::default())
                .map_err(|error| Box::new(context_runtime_error(&error)))
        })
        .with_fn(
            "format_number",
            |context: &mut UiContext,
             value: INT,
             options: Map|
             -> Result<String, Box<EvalAltResult>> {
                let options = number_format_options(options)?;
                context
                    .format_integer(value, options)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "format_number",
            |context: &mut UiContext,
             value: FLOAT,
             options: Map|
             -> Result<String, Box<EvalAltResult>> {
                let options = number_format_options(options)?;
                context
                    .format_number(value, options)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        );
}

fn number_format_options(mut options: Map) -> Result<NumberFormatOptions, Box<EvalAltResult>> {
    let mut parsed = NumberFormatOptions::default();
    if let Some(value) = options.remove("min_fraction_digits") {
        let value = value.try_cast::<INT>().ok_or_else(|| {
            Box::new(EvalAltResult::ErrorRuntime(
                "min_fraction_digits must be an integer".into(),
                Position::NONE,
            ))
        })?;
        parsed.min_fraction_digits = u8::try_from(value).map_err(|_| {
            Box::new(EvalAltResult::ErrorRuntime(
                "min_fraction_digits must be between 0 and 12".into(),
                Position::NONE,
            ))
        })?;
    }
    if let Some(value) = options.remove("max_fraction_digits") {
        let value = value.try_cast::<INT>().ok_or_else(|| {
            Box::new(EvalAltResult::ErrorRuntime(
                "max_fraction_digits must be an integer".into(),
                Position::NONE,
            ))
        })?;
        parsed.max_fraction_digits = u8::try_from(value).map_err(|_| {
            Box::new(EvalAltResult::ErrorRuntime(
                "max_fraction_digits must be between 0 and 12".into(),
                Position::NONE,
            ))
        })?;
    }
    if let Some(value) = options.remove("grouping") {
        parsed.grouping = value.try_cast::<bool>().ok_or_else(|| {
            Box::new(EvalAltResult::ErrorRuntime(
                "grouping must be a bool".into(),
                Position::NONE,
            ))
        })?;
    }
    if let Some((unknown, _)) = options.into_iter().next() {
        return Err(Box::new(EvalAltResult::ErrorRuntime(
            format!("unknown number format option `{unknown}`").into(),
            Position::NONE,
        )));
    }
    parsed.validate().map_err(|error| {
        Box::new(EvalAltResult::ErrorRuntime(
            error.to_string().into(),
            Position::NONE,
        ))
    })?;
    Ok(parsed)
}

fn register_theme_context_methods(builder: &mut TypeBuilder<UiContext>) {
    builder
        .with_fn(
            "set_theme",
            |context: &mut UiContext, family: ImmutableString, variant: ImmutableString| {
                context
                    .set_theme(family.as_str(), variant.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_window_theme",
            |context: &mut UiContext, family: ImmutableString, variant: ImmutableString| {
                context
                    .set_window_theme(family.as_str(), variant.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_local_theme",
            |context: &mut UiContext, family: ImmutableString, variant: ImmutableString| {
                context
                    .set_local_theme(family.as_str(), variant.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_theme_system",
            |context: &mut UiContext, family: ImmutableString| {
                context
                    .set_theme_system(family.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_window_theme_system",
            |context: &mut UiContext, family: ImmutableString| {
                context
                    .set_window_theme_system(family.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_local_theme_system",
            |context: &mut UiContext, family: ImmutableString| {
                context
                    .set_local_theme_system(family.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        );
}

fn register_motion_context_methods(builder: &mut TypeBuilder<UiContext>) {
    builder.with_fn(
        "set_reduced_motion",
        |context: &mut UiContext, reduced: bool| {
            context
                .set_reduced_motion(reduced)
                .map_err(|error| Box::new(context_runtime_error(&error)))
        },
    );
}

fn register_asset_context_methods(builder: &mut TypeBuilder<UiContext>) {
    builder
        .with_fn("load_image", |context: &mut UiContext, asset: AssetId| {
            context
                .load_image(&asset)
                .map_err(|error| Box::new(context_runtime_error(&error)))
        })
        .with_fn(
            "start_image_decode",
            |context: &mut UiContext, asset: AssetId, success: FnPtr, error: FnPtr| {
                context
                    .start_image_decode(&asset, success, error)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "cancel_image_decode",
            |context: &mut UiContext, handle: ImageDecodeHandle| {
                context
                    .cancel_image_decode(handle)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        );
}

fn register_window_context_methods(builder: &mut TypeBuilder<UiContext>) {
    builder
        .with_fn("window_id", |context: &mut UiContext| {
            context
                .window_id()
                .map_err(|error| Box::new(context_runtime_error(&error)))
        })
        .with_fn("view_id", |context: &mut UiContext| {
            context
                .view_id()
                .map_err(|error| Box::new(context_runtime_error(&error)))
        })
        .with_fn("viewport_class", |context: &mut UiContext| {
            context
                .viewport_class()
                .map_err(|error| Box::new(context_runtime_error(&error)))
        })
        .with_fn(
            "open_window",
            |context: &mut UiContext,
             id: ImmutableString,
             title: ImmutableString,
             width: rhai::INT,
             height: rhai::INT,
             focus: bool| {
                context
                    .open_window(ScriptWindowSpec {
                        id: id.to_string(),
                        title: title.to_string(),
                        width: width.to_string().parse::<f64>().unwrap_or(f64::INFINITY),
                        height: height.to_string().parse::<f64>().unwrap_or(f64::INFINITY),
                        focus,
                    })
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "focus_window",
            |context: &mut UiContext, id: ImmutableString| {
                context
                    .focus_window(id.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "close_window",
            |context: &mut UiContext, id: ImmutableString| {
                context
                    .close_window(id.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_close_handler",
            |context: &mut UiContext, callback: FnPtr| {
                context
                    .set_close_handler(callback)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn("clear_close_handler", |context: &mut UiContext| {
            context
                .clear_close_handler()
                .map_err(|error| Box::new(context_runtime_error(&error)))
        })
        .with_fn(
            "set_window_locale",
            |context: &mut UiContext, locale: ImmutableString| {
                context
                    .set_window_locale(locale.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_local_locale",
            |context: &mut UiContext, locale: ImmutableString| {
                context
                    .set_local_locale(locale.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        );
}

pub(crate) fn register_ui_context_api(engine: &mut Engine) {
    engine.build_type::<UiContext>();
}

fn context_runtime_error(error: &UiContextError) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(error.to_string().into(), Position::NONE)
}

#[derive(Debug, Error)]
pub enum UiContextError {
    #[error("UI runtime state is already mutably borrowed by another callback")]
    Borrowed,
    #[error("state mutation and effects are forbidden during view rendering")]
    MutationDuringRender,
    #[error("async work requires a bound script generation")]
    MissingGeneration,
    #[error("no locale manager is configured for this application")]
    LocaleUnavailable,
    #[error("no theme manager is configured for this application")]
    ThemeUnavailable,
    #[error("component `{component}` has no state field `{field}`")]
    UnknownState {
        component: ComponentInstancePath,
        field: String,
    },
    #[error("event `{0}` is not declared by this component")]
    UnknownEvent(String),
    #[error("event payload is invalid: {0}")]
    InvalidEvent(crate::SchemaValidationError),
    #[error("this UI context is not associated with a window")]
    MissingWindow,
    #[error("this UI context is not associated with a mounted script view")]
    MissingView,
    #[error(transparent)]
    Window(#[from] WindowCommandError),
    #[error(transparent)]
    Responsive(#[from] ResponsiveError),
    #[error(transparent)]
    State(#[from] StateError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Action(#[from] ActionError),
    #[error(transparent)]
    Capability(#[from] CapabilityError),
    #[error(transparent)]
    Async(#[from] AsyncRuntimeError),
    #[error(transparent)]
    Locale(#[from] LocaleError),
    #[error(transparent)]
    Theme(#[from] ThemeError),
    #[error(transparent)]
    Asset(#[from] AssetError),
    #[error(transparent)]
    Value(#[from] UiValueError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComponentStateSchema, RuntimeEngine, StateField, ValueSchema};

    fn mounted_context(phase: ExecutionPhase) -> UiContext {
        let path = ComponentInstancePath::root("Counter", "counter");
        let schema = ComponentStateSchema::new(BTreeMap::from([(
            "count".to_owned(),
            StateField::new(ValueSchema::integer(), UiValue::Integer(0)),
        )]))
        .unwrap();
        let mut state = UiRuntimeState::new();
        let mut render = state.component_state.begin_render();
        render.mount(path.clone(), &schema).unwrap();
        state.component_state.commit_render(render);
        UiContext::new(
            Rc::new(RefCell::new(state)),
            path,
            Some("main".to_owned()),
            phase,
            BTreeMap::from([(
                "change".to_owned(),
                EventSchema {
                    payload: ValueSchema::integer(),
                },
            )]),
        )
    }

    #[test]
    fn render_context_rejects_mutation() {
        let context = mounted_context(ExecutionPhase::Render);
        assert!(matches!(
            context.set_state("count", Dynamic::from(1_i64)),
            Err(UiContextError::MutationDuringRender)
        ));
    }

    #[test]
    fn viewport_class_is_window_scoped_and_script_readable() {
        let context = mounted_context(ExecutionPhase::Render);
        context
            .runtime()
            .borrow_mut()
            .responsive
            .update_window("main", 480.0)
            .unwrap();
        assert_eq!(context.viewport_class().unwrap(), "compact");
    }

    #[test]
    fn event_mutations_are_batched() {
        let context = mounted_context(ExecutionPhase::Event);
        context.set_state("count", Dynamic::from(1_i64)).unwrap();
        context.set_state("count", Dynamic::from(2_i64)).unwrap();
        context.emit("change", Dynamic::from(2_i64)).unwrap();

        let mut runtime = context.runtime().borrow_mut();
        let batch = runtime.drain_batch();
        assert_eq!(batch.dirty.len(), 1);
        assert_eq!(batch.events.len(), 1);
        assert_eq!(
            runtime
                .component_state
                .get(&ComponentInstancePath::root("Counter", "counter"), "count",),
            Some(&UiValue::Integer(2))
        );
    }

    #[test]
    fn scripts_receive_restricted_context_without_gpui_types() {
        let context = mounted_context(ExecutionPhase::Event);
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn view() { text("context") }
                    fn increment(ctx) {
                        ctx.set_state("count", ctx.get_state("count") + 1);
                    }
                "#,
            )
            .unwrap();
        runtime.render(&compiled).unwrap();
        let callback = runtime.callback(&compiled, "increment").unwrap();
        let _ = runtime
            .invoke_callback(&compiled, &callback, (context.clone(),))
            .unwrap();
        assert_eq!(context.get_state("count").unwrap(), UiValue::Integer(1));
    }

    #[test]
    fn script_theme_switch_preserves_component_state_and_ast_generation() {
        let context = mounted_context(ExecutionPhase::Event);
        context.set_state("count", Dynamic::from(7_i64)).unwrap();
        let mut runtime = RuntimeEngine::new();
        let dark = crate::load_theme_source(
            runtime.engine(),
            "default_dark.rhai",
            include_str!("../../../registry/themes/default_dark.rhai"),
        )
        .unwrap();
        let mocha = crate::load_theme_source(
            runtime.engine(),
            "catppuccin_mocha.rhai",
            include_str!("../../../registry/themes/catppuccin_mocha.rhai"),
        )
        .unwrap();
        context.runtime().borrow_mut().theme = Some(
            ThemeManager::from_variants([dark, mocha], ThemeSelection::new("Default", "Dark"))
                .unwrap(),
        );
        let compiled = runtime
            .compile(
                r#"
                    fn view() { text("theme") }
                    fn switch_theme(ctx) { ctx.set_theme("Catppuccin", "Mocha"); }
                "#,
            )
            .unwrap();
        runtime.render(&compiled).unwrap();
        let generation = compiled.generation();
        let callback = runtime.callback(&compiled, "switch_theme").unwrap();
        let _ = runtime
            .invoke_callback(&compiled, &callback, (context.clone(),))
            .unwrap();

        assert_eq!(context.get_state("count").unwrap(), UiValue::Integer(7));
        assert_eq!(compiled.generation(), generation);
        let state = context.runtime().borrow();
        let theme = state
            .theme
            .as_ref()
            .unwrap()
            .resolve(Some("main"), None, crate::SystemAppearance::Dark)
            .unwrap();
        assert_eq!(theme.variant().family, "Catppuccin");
        assert_eq!(theme.variant().name, "Mocha");
        drop(state);
        context.set_theme_system("Default").unwrap();
        let state = context.runtime().borrow();
        assert!(matches!(
            state.theme.as_ref().unwrap().app_preference(),
            ThemePreference::System { family } if family == "Default"
        ));
    }

    #[test]
    fn script_reads_fixed_today_calendar_and_locale_formatters() {
        let mut engine = Engine::new();
        register_ui_context_api(&mut engine);
        let bundle = crate::load_locale_source(
            &engine,
            "en.rhai",
            include_str!("../../../registry/locales/en.rhai"),
        )
        .unwrap();
        let mut state = UiRuntimeState::new();
        state.locale = Some(LocaleManager::new([bundle], "en", "en").unwrap());
        state.calendar_clock =
            crate::CalendarClock::fixed(crate::GregorianDate::parse_iso("2026-08-29").unwrap());
        let context = UiContext::new(
            Rc::new(RefCell::new(state)),
            ComponentInstancePath::root("App", "root"),
            Some("main".to_owned()),
            ExecutionPhase::Render,
            BTreeMap::new(),
        );
        let mut scope = rhai::Scope::new();
        scope.push("ctx", context);
        let values = engine
            .eval_with_scope::<rhai::Array>(
                &mut scope,
                r#"[
                    ctx.today(),
                    ctx.format_date("2024-02-29", "long"),
                    ctx.format_number(12345, #{ min_fraction_digits: 0, max_fraction_digits: 0 }),
                    ctx.calendar().first_weekday,
                ]"#,
            )
            .unwrap();
        assert_eq!(values[0].clone_cast::<String>(), "2026-08-29");
        assert_eq!(
            values[1].clone_cast::<String>(),
            "Thursday, February 29, 2024"
        );
        assert_eq!(values[2].clone_cast::<String>(), "12,345");
        assert_eq!(values[3].clone_cast::<String>(), "sunday");
    }
}
