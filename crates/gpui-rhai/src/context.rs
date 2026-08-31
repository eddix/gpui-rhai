use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::time::Duration;

use rhai::{
    Array, CustomType, Dynamic, Engine, EvalAltResult, FLOAT, FnPtr, INT, ImmutableString, Map,
    Position, TypeBuilder,
};
use thiserror::Error;

use crate::{
    ActionError, ActionId, ActionInvocation, ActionRegistry, AnimationKey, AnimationRuntime,
    AssetError, AssetId, AssetRegistry, AsyncRuntimeError, AsyncScope, CalendarClock,
    CapabilityError, CapabilityId, CapabilityRegistry, ComponentInstancePath, DateStyle,
    EventSchema, ImageDecodeHandle, LocaleError, LocaleManager, NumberFormatOptions, OpaqueHandle,
    ResponsiveError, ResponsiveRuntime, ScriptCallback, ScriptGeneration, ScriptWindowSpec,
    StateError, StateStore, StoreError, StoreId, StoreRegistry, SubscriptionCloseReason,
    SubscriptionHandle, SubscriptionRegistration, SubscriptionRegistry, TaskHandle, TaskRegistry,
    TextDirection, ThemeError, ThemeManager, ThemePreference, ThemeSelection, UiEvent, UiValue,
    UiValueError, UiValuePath, UiValuePathError, UiValuePathSegment, WindowCommandError,
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
    pub timers: crate::TimerRegistry,
    pub locale: Option<LocaleManager>,
    pub calendar_clock: CalendarClock,
    pub clock: crate::RuntimeClock,
    pub theme: Option<ThemeManager>,
    pub assets: AssetRegistry,
    pub animations: AnimationRuntime,
    pub effects: crate::EffectRegistry,
    pub signals: crate::SignalRegistry,
    pub element_refs: crate::ElementRefRegistry,
    pub geometry: crate::GeometryRegistry,
    pub pointer_capture: crate::PointerCaptureRegistry,
    pub budgets: crate::RuntimeBudgets,
    pub virtual_requests: crate::VirtualRequestRegistry,
    pub windows: WindowCommandRegistry,
    pub responsive: ResponsiveRuntime,
    pub(crate) environment_dependencies:
        crate::environment_dependency::EnvironmentDependencyRegistry,
    pub animation_values: BTreeMap<AnimationKey, f64>,
    pub traces: crate::TraceBuffer,
    component_event_handlers: BTreeMap<(ComponentInstancePath, String), ScriptCallback>,
    dirty: BTreeSet<ComponentInstancePath>,
    pending_events: Vec<PendingEvent>,
    pending_actions: Vec<ActionInvocation>,
    pending_async: Vec<crate::AsyncDelivery>,
    pending_element_commands: Vec<crate::element_ref::ElementCommand>,
    repaint_windows: BTreeSet<String>,
}

impl UiRuntimeState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn trace_subscription_closures(&mut self) {
        for closure in self.subscriptions.take_closures() {
            let scope = match closure.scope {
                AsyncScope::App => "/App".to_owned(),
                AsyncScope::Window(window) => format!("window:{window}"),
                AsyncScope::Component(component) => component.to_string(),
                AsyncScope::Effect {
                    component,
                    key,
                    activation,
                } => format!("{component}/effect[{key}]#{activation}"),
            };
            self.traces.push(
                crate::RuntimeTraceKind::Subscription,
                scope,
                format!("close {}: {}", closure.label, closure.reason),
                None,
                false,
            );
        }
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
        self.timers.cancel_component_scope(root);
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
            AsyncScope::Component(_) | AsyncScope::Effect { .. } => {
                !delivery.scope.is_within_component(root)
            }
        });
        self.pending_element_commands
            .retain(|command| command.window() != window);
        self.environment_dependencies.remove_window(window);
        self.environment_dependencies.remove_scope(root);
        self.repaint_windows.remove(window);
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
        self.animation_values = self.animations.snapshot(self.clock.now());
        self.effects.remove_scope(root);
        self.signals.remove_scope(root);
        self.element_refs.remove_scope(root);
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

    pub(crate) fn cancel_async_scope(&mut self, scope: &AsyncScope) -> Result<(), AssetError> {
        self.tasks.cancel_scope(scope);
        self.subscriptions.cancel_scope(scope);
        self.assets.cancel_scope(scope)?;
        self.pending_async
            .retain(|delivery| &delivery.scope != scope);
        self.trace_subscription_closures();
        Ok(())
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
                    AsyncScope::Component(_) | AsyncScope::Effect { .. } => {
                        delivery.scope.is_within_component(root)
                    }
                });
        self.pending_async = retained;
        accepted
    }

    pub(crate) fn take_window_element_commands(
        &mut self,
        window: &str,
    ) -> Vec<crate::element_ref::ElementCommand> {
        let commands = std::mem::take(&mut self.pending_element_commands);
        let (selected, retained) = commands
            .into_iter()
            .partition(|command| command.window() == window);
        self.pending_element_commands = retained;
        selected
    }

    pub(crate) fn has_window_dirty(&self, root: &ComponentInstancePath) -> bool {
        self.dirty.iter().any(|path| path.is_within(root))
    }

    pub(crate) fn flush_geometry_dependencies(&mut self) {
        self.dirty.extend(self.geometry.take_dirty());
    }

    pub(crate) fn take_window_dirty_components(
        &mut self,
        root: &ComponentInstancePath,
    ) -> BTreeSet<ComponentInstancePath> {
        let all = std::mem::take(&mut self.dirty);
        let (selected, retained) = all.into_iter().partition(|path| path.is_within(root));
        self.dirty = retained;
        selected
    }

    pub(crate) fn drain_pending_actions(&mut self) -> Vec<ActionInvocation> {
        std::mem::take(&mut self.pending_actions)
    }

    pub(crate) fn drain_pending_events(&mut self) -> Vec<PendingEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub(crate) fn has_pending_dispatch(&self) -> bool {
        !self.pending_actions.is_empty() || !self.pending_events.is_empty()
    }

    pub(crate) fn has_virtual_requests(&self) -> bool {
        !self.virtual_requests.is_empty()
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

    pub(crate) fn mark_dirty(
        &mut self,
        components: impl IntoIterator<Item = ComponentInstancePath>,
    ) {
        self.dirty.extend(components);
    }

    pub(crate) fn mark_all_windows_repaint(&mut self) {
        self.repaint_windows.extend(self.windows.open_ids());
    }

    pub(crate) fn mark_window_repaint(&mut self, window: impl Into<String>) {
        self.repaint_windows.insert(window.into());
    }

    pub(crate) fn take_window_repaint(&mut self, window: &str) -> bool {
        self.repaint_windows.remove(window)
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
            self.timers.cancel_component_scope(removed);
            self.assets.cancel_component_scope(removed)?;
            self.actions.remove_component_scope(removed);
        }
        self.pending_async.retain(|delivery| {
            delivery
                .scope
                .component()
                .is_none_or(|path| !path.is_within(root) || active.contains(path))
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
        self.environment_dependencies.retain_scope(root, active);
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
            pending_element_commands: self.pending_element_commands.clone(),
            locale: self.locale.clone(),
            theme: self.theme.clone(),
            animations: self.animations.clone(),
            effects: self.effects.clone(),
            signals: self.signals.clone(),
            element_refs: self.element_refs.clone(),
            geometry: self.geometry.snapshot(),
            pointer_capture: self.pointer_capture.snapshot(),
            budgets: self.budgets.clone(),
            virtual_requests: self.virtual_requests.snapshot(),
            animation_values: self.animation_values.clone(),
            windows: self.windows.clone(),
            responsive: self.responsive.clone(),
            environment_dependencies: self.environment_dependencies.clone(),
            repaint_windows: self.repaint_windows.clone(),
            task_ids: self.tasks.active_ids(),
            subscription_ids: self.subscriptions.active_ids(),
            timers: self.timers.clone(),
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
        self.timers = snapshot.timers;
        self.assets.retain_decode_ids(&snapshot.decode_ids)?;
        self.component_state = snapshot.component_state;
        self.stores = snapshot.stores;
        self.actions = snapshot.actions;
        self.component_event_handlers = snapshot.component_event_handlers;
        self.dirty = snapshot.dirty;
        self.pending_events = snapshot.pending_events;
        self.pending_actions = snapshot.pending_actions;
        self.pending_async = snapshot.pending_async;
        self.pending_element_commands = snapshot.pending_element_commands;
        self.locale = snapshot.locale;
        self.theme = snapshot.theme;
        self.animations = snapshot.animations;
        self.effects = snapshot.effects;
        self.signals = snapshot.signals;
        self.element_refs = snapshot.element_refs;
        self.geometry.restore(snapshot.geometry);
        self.pointer_capture.restore(snapshot.pointer_capture);
        self.budgets = snapshot.budgets;
        self.virtual_requests.restore(snapshot.virtual_requests);
        self.animation_values = snapshot.animation_values;
        self.windows = snapshot.windows;
        self.responsive = snapshot.responsive;
        self.environment_dependencies = snapshot.environment_dependencies;
        self.repaint_windows = snapshot.repaint_windows;
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
    pending_element_commands: Vec<crate::element_ref::ElementCommand>,
    locale: Option<LocaleManager>,
    theme: Option<ThemeManager>,
    animations: AnimationRuntime,
    effects: crate::EffectRegistry,
    signals: crate::SignalRegistry,
    element_refs: crate::ElementRefRegistry,
    geometry: crate::geometry::GeometrySnapshot,
    pointer_capture: BTreeMap<u64, crate::NodeId>,
    budgets: crate::RuntimeBudgets,
    virtual_requests: crate::virtual_list::VirtualRequestSnapshot,
    animation_values: BTreeMap<AnimationKey, f64>,
    windows: WindowCommandRegistry,
    responsive: ResponsiveRuntime,
    environment_dependencies: crate::environment_dependency::EnvironmentDependencyRegistry,
    repaint_windows: BTreeSet<String>,
    task_ids: BTreeSet<u64>,
    subscription_ids: BTreeSet<u64>,
    timers: crate::TimerRegistry,
    decode_ids: BTreeSet<u64>,
}

impl UiStateSnapshot {
    pub(crate) const fn component_state(&self) -> &StateStore {
        &self.component_state
    }
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
    native_context: Option<crate::invocation::ScriptInvocationContext>,
    async_scope: Option<AsyncScope>,
    component_style: Option<crate::Style>,
    component_part_styles: BTreeMap<String, crate::Style>,
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
            async_scope: None,
            component_style: None,
            component_part_styles: BTreeMap::new(),
        };
        if phase == ExecutionPhase::Render
            && let Ok(mut runtime) = context.runtime.try_borrow_mut()
        {
            runtime.stores.reset_reader(&context.component);
            runtime
                .environment_dependencies
                .reset_reader(&context.component);
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
            async_scope: self.async_scope.clone(),
            component_style: self.component_style.clone(),
            component_part_styles: self.component_part_styles.clone(),
        };
        if context.phase == ExecutionPhase::Render
            && let Ok(mut runtime) = context.runtime.try_borrow_mut()
        {
            runtime.stores.reset_reader(&context.component);
            runtime
                .environment_dependencies
                .reset_reader(&context.component);
        }
        context
    }

    pub(crate) fn with_native_context(
        mut self,
        native_context: Option<crate::invocation::ScriptInvocationContext>,
    ) -> Self {
        self.native_context = native_context;
        self
    }

    pub(crate) fn with_async_scope(mut self, scope: AsyncScope) -> Self {
        self.async_scope = Some(scope);
        self
    }

    pub(crate) fn with_component_styles(
        mut self,
        style: Option<crate::Style>,
        part_styles: BTreeMap<String, crate::Style>,
    ) -> Self {
        self.component_style = style;
        self.component_part_styles = part_styles;
        self
    }

    fn resolve_component_style(&self, part: &str, mut base: crate::Style) -> crate::Style {
        if part == "root"
            && let Some(style) = &self.component_style
        {
            base = base.merged(style);
        }
        if let Some(style) = self.component_part_styles.get(part) {
            base = base.merged(style);
        }
        base
    }

    fn scoped_callback(&self, function: FnPtr) -> Result<ScriptCallback, UiContextError> {
        let mut callback = ScriptCallback::try_from_fn_ptr(function, self.generation)?;
        callback.bind_component_if_unset(self.component.clone(), self.events.clone());
        if let Some(context) = &self.native_context {
            callback.bind_native_context_if_unset(context.clone());
        }
        Ok(callback)
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

    /// Read one existing nested local-state path.
    ///
    /// Local state remains a component-scoped dependency: this accessor avoids
    /// copying an entire collection into Rhai but does not create a smaller
    /// rerender boundary than the owning component.
    ///
    /// # Errors
    ///
    /// Returns state, borrow, or nested-path errors.
    pub fn get_state_path(
        &self,
        field: &str,
        path: &UiValuePath,
    ) -> Result<UiValue, UiContextError> {
        Ok(self.get_state(field)?.get_path(path)?.clone())
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
        let changed = runtime
            .component_state
            .set(&self.component, field, value.clone())?;
        if changed {
            runtime.dirty.insert(self.component.clone());
        }
        runtime.traces.push(
            crate::RuntimeTraceKind::State,
            self.component.to_string(),
            format!("set {field}"),
            Some(value),
            sensitive,
        );
        Ok(())
    }

    /// Replace one existing nested local-state path.
    ///
    /// # Errors
    ///
    /// Returns phase, state, conversion, borrow, or nested-path errors. The
    /// complete resulting field is checked against its declared schema.
    pub fn set_state_path(
        &self,
        field: &str,
        path: &UiValuePath,
        value: Dynamic,
    ) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let value = UiValue::from_dynamic(value)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let mut root = runtime
            .component_state
            .get(&self.component, field)
            .cloned()
            .ok_or_else(|| UiContextError::UnknownState {
                component: self.component.clone(),
                field: field.to_owned(),
            })?;
        root.set_path(path, value.clone())?;
        let sensitive = runtime.component_state.is_sensitive(&self.component, field);
        if runtime.component_state.set(&self.component, field, root)? {
            runtime.dirty.insert(self.component.clone());
        }
        runtime.traces.push(
            crate::RuntimeTraceKind::State,
            self.component.to_string(),
            format!("set nested {field}"),
            Some(value),
            sensitive,
        );
        Ok(())
    }

    /// Read a native hot value without establishing a component dependency.
    ///
    /// # Errors
    ///
    /// Returns a stale-signal or runtime borrow error.
    pub fn get_signal(&self, signal: &crate::NativeSignal) -> Result<Dynamic, UiContextError> {
        Ok(self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?
            .signals
            .read(signal)?
            .into_dynamic())
    }

    /// Update a native hot value without dirtying a formal component.
    ///
    /// # Errors
    ///
    /// Returns during render, for a stale signal, a type mismatch, or an
    /// unsupported value.
    pub fn set_signal(
        &self,
        signal: &crate::NativeSignal,
        value: Dynamic,
    ) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let value = crate::SignalValue::from_dynamic(value)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let changed =
            runtime
                .signals
                .write_from(signal, value.clone(), crate::SignalWriter::Script)?;
        runtime.traces.push(
            crate::RuntimeTraceKind::Signal,
            signal.id().component().to_string(),
            format!(
                "set {} ({}){}",
                signal.id().key(),
                signal.id().kind().as_str(),
                if changed { "" } else { " unchanged" }
            ),
            None,
            false,
        );
        Ok(())
    }

    /// Resolve and read a component-local signal key without tracking a dependency.
    ///
    /// # Errors
    ///
    /// Returns when the key is not mounted or runtime state is borrowed.
    pub fn get_signal_by_key(&self, key: &str) -> Result<Dynamic, UiContextError> {
        let runtime = self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?;
        let signal = runtime.signals.resolve(&self.component, key)?;
        Ok(runtime.signals.read(&signal)?.into_dynamic())
    }

    /// Resolve and write a component-local signal key.
    ///
    /// # Errors
    ///
    /// Returns during render, when the key is not mounted, or for a type mismatch.
    pub fn set_signal_by_key(&self, key: &str, value: Dynamic) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let signal = self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?
            .signals
            .resolve(&self.component, key)?;
        self.set_signal(&signal, value)
    }

    /// Read last committed layout/visual geometry through a stable element ref.
    ///
    /// The exact node becomes a component dependency. The first render returns
    /// null until GPUI has committed prepaint, which then dirties the reader.
    ///
    /// # Errors
    ///
    /// Returns for stale refs, unavailable geometry, or runtime borrow conflicts.
    pub fn element_bounds(&self, reference: &crate::ElementRef) -> Result<UiValue, UiContextError> {
        let runtime = self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?;
        let node = runtime.element_refs.resolve(reference)?;
        let Some(geometry) = runtime.geometry.read_tracked(node, &self.component) else {
            return Ok(UiValue::Null);
        };
        Ok(UiValue::Map(BTreeMap::from([
            ("layout".to_owned(), geometry_bounds_value(geometry.layout)),
            ("visual".to_owned(), geometry_bounds_value(geometry.visual)),
            (
                "clip".to_owned(),
                geometry.clip.map_or(UiValue::Null, geometry_bounds_value),
            ),
        ])))
    }

    /// Queue focus for a mounted element ref in the current window.
    ///
    /// # Errors
    ///
    /// Returns during render, outside a window, or for a stale ref.
    pub fn focus_element(&self, reference: &crate::ElementRef) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let window = self.window.clone().ok_or(UiContextError::MissingWindow)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let node = runtime.element_refs.resolve(reference)?;
        runtime
            .pending_element_commands
            .push(crate::element_ref::ElementCommand::Focus { window, node });
        Ok(())
    }

    /// Resolve and queue focus for a component-local ref key.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::focus_element`] plus unknown keys.
    pub fn focus_element_by_key(&self, key: &str) -> Result<(), UiContextError> {
        let reference = self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?
            .element_refs
            .resolve_key(&self.component, key)?;
        self.focus_element(&reference)
    }

    /// Queue a positive visible scroll offset for a retained scroll container.
    ///
    /// # Errors
    ///
    /// Returns during render, outside a window, for a stale ref, or for invalid
    /// offsets.
    pub fn scroll_element_to(
        &self,
        reference: &crate::ElementRef,
        x: f64,
        y: f64,
    ) -> Result<(), UiContextError> {
        self.require_mutation()?;
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
            return Err(UiContextError::InvalidScrollOffset { x, y });
        }
        let window = self.window.clone().ok_or(UiContextError::MissingWindow)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let node = runtime.element_refs.resolve(reference)?;
        runtime
            .pending_element_commands
            .push(crate::element_ref::ElementCommand::ScrollTo { window, node, x, y });
        Ok(())
    }

    /// Resolve and queue scroll for a component-local ref key.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::scroll_element_to`] plus unknown keys.
    pub fn scroll_element_to_by_key(
        &self,
        key: &str,
        x: f64,
        y: f64,
    ) -> Result<(), UiContextError> {
        let reference = self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?
            .element_refs
            .resolve_key(&self.component, key)?;
        self.scroll_element_to(&reference, x, y)
    }

    /// Queue minimal ancestor scrolling that reveals a retained descendant.
    ///
    /// # Errors
    ///
    /// Returns during render, outside a window, or for a stale ref.
    pub fn scroll_element_into_view(
        &self,
        reference: &crate::ElementRef,
    ) -> Result<(), UiContextError> {
        self.require_mutation()?;
        let window = self.window.clone().ok_or(UiContextError::MissingWindow)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let node = runtime.element_refs.resolve(reference)?;
        runtime
            .pending_element_commands
            .push(crate::element_ref::ElementCommand::ScrollIntoView { window, node });
        Ok(())
    }

    /// Resolve and reveal a component-local ref key.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::scroll_element_into_view`] plus unknown keys.
    pub fn scroll_element_into_view_by_key(&self, key: &str) -> Result<(), UiContextError> {
        let reference = self
            .runtime
            .try_borrow()
            .map_err(|_| UiContextError::Borrowed)?
            .element_refs
            .resolve_key(&self.component, key)?;
        self.scroll_element_into_view(&reference)
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

    /// Read and subscribe to one exact app-store path.
    ///
    /// # Errors
    ///
    /// Returns store, path, or borrow errors.
    pub fn get_app_store_path(
        &self,
        store: &str,
        field: &str,
        path: &UiValuePath,
    ) -> Result<UiValue, UiContextError> {
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        Ok(runtime
            .stores
            .read_path_tracked(&self.component, &StoreId::app(store), field, path)?)
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

    /// Replace one exact app-store path and invalidate only affected readers.
    ///
    /// # Errors
    ///
    /// Returns phase, conversion, store, path, schema, or borrow errors.
    pub fn set_app_store_path(
        &self,
        store: &str,
        field: &str,
        path: &UiValuePath,
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
        let invalidated = runtime.stores.write_path(&id, field, path, value.clone())?;
        runtime.dirty.extend(invalidated);
        runtime.traces.push(
            crate::RuntimeTraceKind::Store,
            format!("app:{store}"),
            format!("set nested {field}"),
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

    /// Read and subscribe to one exact path in the current window's store.
    ///
    /// # Errors
    ///
    /// Returns missing-window, store, path, or borrow errors.
    pub fn get_window_store_path(
        &self,
        store: &str,
        field: &str,
        path: &UiValuePath,
    ) -> Result<UiValue, UiContextError> {
        let window = self.window.as_ref().ok_or(UiContextError::MissingWindow)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        Ok(runtime.stores.read_path_tracked(
            &self.component,
            &StoreId::window(window, store),
            field,
            path,
        )?)
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

    /// Replace one exact path in the current window's store.
    ///
    /// # Errors
    ///
    /// Returns phase, missing-window, conversion, store, path, schema, or
    /// borrow errors.
    pub fn set_window_store_path(
        &self,
        store: &str,
        field: &str,
        path: &UiValuePath,
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
        let invalidated = runtime.stores.write_path(&id, field, path, value.clone())?;
        runtime.dirty.extend(invalidated);
        runtime.traces.push(
            crate::RuntimeTraceKind::Store,
            format!("window:{window}:{store}"),
            format!("set nested {field}"),
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
        let callback = self.scoped_callback(callback)?;
        self.runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?
            .actions
            .register_or_replace(id, callback);
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

    fn read_locale<T>(
        &self,
        read: impl FnOnce(
            &LocaleManager,
            Option<&str>,
            &ComponentInstancePath,
        ) -> Result<T, LocaleError>,
    ) -> Result<T, UiContextError> {
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        runtime
            .environment_dependencies
            .track_locale(self.window.as_deref(), &self.component);
        let locale = runtime
            .locale
            .as_ref()
            .ok_or(UiContextError::LocaleUnavailable)?;
        Ok(read(locale, self.window.as_deref(), &self.component)?)
    }

    /// Resolve one localized message for this window/component scope.
    ///
    /// # Errors
    ///
    /// Returns [`UiContextError::LocaleUnavailable`], locale, or borrow errors.
    pub fn text(&self, key: &str) -> Result<String, UiContextError> {
        self.read_locale(|locale, window, component| locale.text(window, Some(component), key))
    }

    /// Resolve the logical text direction for this component scope.
    ///
    /// # Errors
    ///
    /// Returns [`UiContextError::LocaleUnavailable`], locale, or borrow errors.
    pub fn text_direction(&self) -> Result<String, UiContextError> {
        match self
            .read_locale(|locale, window, component| locale.direction(window, Some(component)))?
        {
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
        let style = DateStyle::parse(style)?;
        self.read_locale(|locale, window, component| {
            locale.format_date(window, Some(component), iso_date, style)
        })
    }

    /// Format a strict ISO date with the locale's month/year pattern.
    ///
    /// # Errors
    ///
    /// Returns locale, date, or runtime borrow errors.
    pub fn format_month_year(&self, iso_date: &str) -> Result<String, UiContextError> {
        self.read_locale(|locale, window, component| {
            locale.format_month_year(window, Some(component), iso_date)
        })
    }

    /// Return a detached read-only copy of selected calendar metadata.
    ///
    /// # Errors
    ///
    /// Returns locale, serialization, or runtime borrow errors.
    pub fn calendar_metadata(&self) -> Result<Map, UiContextError> {
        let calendar = self.read_locale(|locale, window, component| {
            locale.calendar(window, Some(component)).cloned()
        })?;
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
        let number = self.read_locale(|locale, window, component| {
            locale.number(window, Some(component)).cloned()
        })?;
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
        self.read_locale(|locale, window, component| {
            locale.format_integer(window, Some(component), value, options)
        })
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
        self.read_locale(|locale, window, component| {
            locale.format_number(window, Some(component), value, options)
        })
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
        let changed = {
            let locales = runtime
                .locale
                .as_mut()
                .ok_or(UiContextError::LocaleUnavailable)?;
            let previous = locales.generation();
            locales.set_app(locale)?;
            locales.generation() != previous
        };
        if changed {
            let invalidated = runtime.environment_dependencies.invalidate_locale_app();
            runtime.dirty.extend(invalidated);
            runtime.mark_all_windows_repaint();
        }
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
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        runtime
            .environment_dependencies
            .track_viewport(window, &self.component);
        Ok(runtime.responsive.class(window).as_str().to_owned())
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
        let callback = self.scoped_callback(callback)?;
        self.runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?
            .windows
            .set_close_handler(window, Some(callback))?;
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
        let changed = {
            let locales = runtime
                .locale
                .as_mut()
                .ok_or(UiContextError::LocaleUnavailable)?;
            let previous = locales.generation();
            locales.set_window(window.clone(), locale)?;
            locales.generation() != previous
        };
        if changed {
            let invalidated = runtime
                .environment_dependencies
                .invalidate_locale_window(&window);
            runtime.dirty.extend(invalidated);
            runtime.mark_window_repaint(window.clone());
        }
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
        let changed = {
            let locales = runtime
                .locale
                .as_mut()
                .ok_or(UiContextError::LocaleUnavailable)?;
            let previous = locales.generation();
            locales.set_scope(self.component.clone(), locale)?;
            locales.generation() != previous
        };
        if changed {
            let invalidated = runtime
                .environment_dependencies
                .invalidate_locale_scope(&self.component);
            runtime.dirty.extend(invalidated);
            if let Some(window) = &self.window {
                runtime.mark_window_repaint(window.clone());
            }
        }
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
        let now = runtime.clock.now();
        runtime.animation_values = runtime.animations.snapshot(now);
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
        let success = self.scoped_callback(success)?;
        let error = self.scoped_callback(error)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let handle = runtime.assets.start_image_decode(
            asset,
            self.async_scope
                .clone()
                .unwrap_or_else(|| AsyncScope::Component(self.component.clone())),
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

    /// Pause a declared timer by its component-local key.
    ///
    /// # Errors
    ///
    /// Returns phase, key, or borrow errors.
    pub fn pause_timeout(&self, key: &str) -> Result<bool, UiContextError> {
        self.require_mutation()?;
        let id = crate::TimerId::new(self.component.clone(), key)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let now = runtime.clock.now();
        Ok(runtime.timers.pause(&id, now))
    }

    /// Resume a declared timer by its component-local key.
    ///
    /// # Errors
    ///
    /// Returns phase, key, or borrow errors.
    pub fn resume_timeout(&self, key: &str) -> Result<bool, UiContextError> {
        self.require_mutation()?;
        let id = crate::TimerId::new(self.component.clone(), key)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let now = runtime.clock.now();
        Ok(runtime.timers.resume(&id, now))
    }

    /// Complete/cancel a declared timer until its signature changes or disappears.
    ///
    /// # Errors
    ///
    /// Returns phase, key, or borrow errors.
    pub fn cancel_timeout(&self, key: &str) -> Result<bool, UiContextError> {
        self.require_mutation()?;
        let id = crate::TimerId::new(self.component.clone(), key)?;
        Ok(self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?
            .timers
            .cancel(&id))
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
        let success = self.scoped_callback(success)?;
        let error = self.scoped_callback(error)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let (work, output) = runtime.capabilities.start_task(&id, method, input)?;
        let handle = runtime.tasks.spawn(
            self.async_scope
                .clone()
                .unwrap_or_else(|| AsyncScope::Component(self.component.clone())),
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
        let success = self.scoped_callback(success)?;
        let error = self.scoped_callback(error)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| UiContextError::Borrowed)?;
        let (work, output) = runtime
            .capabilities
            .start_subscription(&id, method, input)?;
        let registration = SubscriptionRegistration::new(
            format!("{capability}.{method}"),
            self.async_scope
                .clone()
                .unwrap_or_else(|| AsyncScope::Component(self.component.clone())),
            self.generation,
            success,
            error,
            output,
        )
        .with_throttle(throttle);
        let (handle, emitter) = runtime.subscriptions.subscribe(registration);
        let closer = emitter.clone();
        if let Err(spawn_error) = std::thread::Builder::new()
            .name("gpui-rhai-subscription".to_owned())
            .spawn(move || run_subscription_work(work, emitter, &closer))
        {
            let _ = runtime
                .subscriptions
                .cancel_with_reason(handle, SubscriptionCloseReason::StartupFailed);
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

fn run_subscription_work(
    work: crate::SubscriptionWork,
    emitter: crate::SubscriptionEmitter,
    closer: &crate::SubscriptionEmitter,
) {
    work.run(emitter);
    closer.close_with_reason(SubscriptionCloseReason::WorkReturned);
}

impl CustomType for UiContext {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("UiContext")
            .with_fn(
                "emit",
                |context: &mut Self, event: ImmutableString, payload: Dynamic| {
                    context
                        .emit(event.as_str(), payload)
                        .map_err(|error| Box::new(context_runtime_error(&error)))
                },
            )
            .with_fn(
                "component_style",
                |context: &mut Self, part: ImmutableString, base: crate::Style| {
                    context.resolve_component_style(part.as_str(), base)
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
        register_state_store_context_methods(&mut builder);
        register_signal_context_methods(&mut builder);
        register_element_ref_context_methods(&mut builder);
        register_async_context_methods(&mut builder);
        register_action_context_methods(&mut builder);
        register_locale_context_methods(&mut builder);
        register_theme_context_methods(&mut builder);
        register_motion_context_methods(&mut builder);
        register_asset_context_methods(&mut builder);
        register_window_context_methods(&mut builder);
    }
}

fn register_state_store_context_methods(builder: &mut TypeBuilder<UiContext>) {
    register_state_context_methods(builder);
    register_app_store_context_methods(builder);
    register_window_store_context_methods(builder);
}

fn register_state_context_methods(builder: &mut TypeBuilder<UiContext>) {
    builder
        .with_fn(
            "get_state",
            |context: &mut UiContext, field: ImmutableString| {
                context
                    .get_state(field.as_str())
                    .map(UiValue::into_dynamic)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "get_state_path",
            |context: &mut UiContext, field: ImmutableString, path: Array| {
                let path = rhai_value_path(path)?;
                context
                    .get_state_path(field.as_str(), &path)
                    .map(UiValue::into_dynamic)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_state",
            |context: &mut UiContext, field: ImmutableString, value: Dynamic| {
                context
                    .set_state(field.as_str(), value)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_state_path",
            |context: &mut UiContext, field: ImmutableString, path: Array, value: Dynamic| {
                let path = rhai_value_path(path)?;
                context
                    .set_state_path(field.as_str(), &path, value)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        );
}

fn register_app_store_context_methods(builder: &mut TypeBuilder<UiContext>) {
    builder
        .with_fn(
            "get_app_store",
            |context: &mut UiContext, store: ImmutableString, field: ImmutableString| {
                context
                    .get_app_store(store.as_str(), field.as_str())
                    .map(UiValue::into_dynamic)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "get_app_store_path",
            |context: &mut UiContext,
             store: ImmutableString,
             field: ImmutableString,
             path: Array| {
                let path = rhai_value_path(path)?;
                context
                    .get_app_store_path(store.as_str(), field.as_str(), &path)
                    .map(UiValue::into_dynamic)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_app_store",
            |context: &mut UiContext,
             store: ImmutableString,
             field: ImmutableString,
             value: Dynamic| {
                context
                    .set_app_store(store.as_str(), field.as_str(), value)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_app_store_path",
            |context: &mut UiContext,
             store: ImmutableString,
             field: ImmutableString,
             path: Array,
             value: Dynamic| {
                let path = rhai_value_path(path)?;
                context
                    .set_app_store_path(store.as_str(), field.as_str(), &path, value)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        );
}

fn register_window_store_context_methods(builder: &mut TypeBuilder<UiContext>) {
    builder
        .with_fn(
            "get_window_store",
            |context: &mut UiContext, store: ImmutableString, field: ImmutableString| {
                context
                    .get_window_store(store.as_str(), field.as_str())
                    .map(UiValue::into_dynamic)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "get_window_store_path",
            |context: &mut UiContext,
             store: ImmutableString,
             field: ImmutableString,
             path: Array| {
                let path = rhai_value_path(path)?;
                context
                    .get_window_store_path(store.as_str(), field.as_str(), &path)
                    .map(UiValue::into_dynamic)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_window_store",
            |context: &mut UiContext,
             store: ImmutableString,
             field: ImmutableString,
             value: Dynamic| {
                context
                    .set_window_store(store.as_str(), field.as_str(), value)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_window_store_path",
            |context: &mut UiContext,
             store: ImmutableString,
             field: ImmutableString,
             path: Array,
             value: Dynamic| {
                let path = rhai_value_path(path)?;
                context
                    .set_window_store_path(store.as_str(), field.as_str(), &path, value)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        );
}

fn rhai_value_path(values: Array) -> Result<UiValuePath, Box<EvalAltResult>> {
    if values.len() > 64 {
        return Err(Box::new(EvalAltResult::ErrorRuntime(
            "value path must have at most 64 segments".into(),
            Position::NONE,
        )));
    }
    let segments = values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let actual = value.type_name().to_owned();
            if let Some(key) = value.clone().try_cast::<ImmutableString>() {
                return Ok(UiValuePathSegment::Key(key.to_string()));
            }
            if let Some(index_value) = value.clone().try_cast::<INT>() {
                return usize::try_from(index_value)
                    .map(UiValuePathSegment::Index)
                    .map_err(|_| {
                        Box::new(EvalAltResult::ErrorRuntime(
                            format!("value path segment {index} must be a non-negative integer")
                                .into(),
                            Position::NONE,
                        ))
                    });
            }
            if let Some(mut selector) = value.try_cast::<Map>() {
                let by = selector
                    .remove("by")
                    .and_then(Dynamic::try_cast::<ImmutableString>);
                let key = selector
                    .remove("key")
                    .and_then(Dynamic::try_cast::<ImmutableString>);
                if selector.is_empty()
                    && let (Some(by), Some(key)) = (by, key)
                {
                    return Ok(UiValuePathSegment::Item {
                        key_field: by.to_string(),
                        key: key.to_string(),
                    });
                }
                return Err(Box::new(EvalAltResult::ErrorRuntime(
                    format!(
                        "value path segment {index} keyed selector must be exactly #{{ by: string, key: string }}"
                    )
                    .into(),
                    Position::NONE,
                )));
            }
            Err(Box::new(EvalAltResult::ErrorRuntime(
                format!(
                    "value path segment {index} must be a string, non-negative integer, or keyed selector map; got {actual}"
                )
                .into(),
                Position::NONE,
            )))
        })
        .collect::<Result<Vec<_>, _>>()?;
    UiValuePath::new(segments).map_err(|error| {
        Box::new(EvalAltResult::ErrorRuntime(
            error.to_string().into(),
            Position::NONE,
        ))
    })
}

fn register_signal_context_methods(builder: &mut TypeBuilder<UiContext>) {
    builder
        .with_fn(
            "get_signal",
            |context: &mut UiContext, signal: crate::NativeSignal| {
                context
                    .get_signal(&signal)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "get_signal",
            |context: &mut UiContext, key: ImmutableString| {
                context
                    .get_signal_by_key(key.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_signal",
            |context: &mut UiContext, signal: crate::NativeSignal, value: Dynamic| {
                context
                    .set_signal(&signal, value)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "set_signal",
            |context: &mut UiContext, key: ImmutableString, value: Dynamic| {
                context
                    .set_signal_by_key(key.as_str(), value)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        );
}

fn register_element_ref_context_methods(builder: &mut TypeBuilder<UiContext>) {
    builder
        .with_fn(
            "element_bounds",
            |context: &mut UiContext, reference: crate::ElementRef| {
                context
                    .element_bounds(&reference)
                    .map(UiValue::into_dynamic)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "focus",
            |context: &mut UiContext, reference: crate::ElementRef| {
                context
                    .focus_element(&reference)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn("focus", |context: &mut UiContext, key: ImmutableString| {
            context
                .focus_element_by_key(key.as_str())
                .map_err(|error| Box::new(context_runtime_error(&error)))
        });
}

fn geometry_bounds_value(bounds: crate::GeometryBounds) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("x".to_owned(), UiValue::Float(bounds.x)),
        ("y".to_owned(), UiValue::Float(bounds.y)),
        ("width".to_owned(), UiValue::Float(bounds.width)),
        ("height".to_owned(), UiValue::Float(bounds.height)),
    ]))
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
        )
        .with_fn(
            "scroll_to",
            |context: &mut UiContext, reference: crate::ElementRef, x: FLOAT, y: FLOAT| {
                context
                    .scroll_element_to(&reference, x, y)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "scroll_to",
            |context: &mut UiContext, key: ImmutableString, x: FLOAT, y: FLOAT| {
                context
                    .scroll_element_to_by_key(key.as_str(), x, y)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "scroll_into_view",
            |context: &mut UiContext, reference: crate::ElementRef| {
                context
                    .scroll_element_into_view(&reference)
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "scroll_into_view",
            |context: &mut UiContext, key: ImmutableString| {
                context
                    .scroll_element_into_view_by_key(key.as_str())
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
        )
        .with_fn(
            "pause_timeout",
            |context: &mut UiContext, key: ImmutableString| {
                context
                    .pause_timeout(key.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "resume_timeout",
            |context: &mut UiContext, key: ImmutableString| {
                context
                    .resume_timeout(key.as_str())
                    .map_err(|error| Box::new(context_runtime_error(&error)))
            },
        )
        .with_fn(
            "cancel_timeout",
            |context: &mut UiContext, key: ImmutableString| {
                context
                    .cancel_timeout(key.as_str())
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
        .with_fn(
            "format_month_year",
            |context: &mut UiContext, date: ImmutableString| {
                context
                    .format_month_year(date.as_str())
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
    #[error(transparent)]
    ValuePath(#[from] UiValuePathError),
    #[error(transparent)]
    Signal(#[from] crate::SignalError),
    #[error(transparent)]
    ElementRef(#[from] crate::ElementRefError),
    #[error(transparent)]
    Geometry(#[from] crate::GeometryError),
    #[error("scroll offset must be finite and non-negative, got ({x}, {y})")]
    InvalidScrollOffset { x: f64, y: f64 },
    #[error(transparent)]
    Callback(#[from] crate::ScriptCallbackDefinitionError),
    #[error(transparent)]
    Timer(#[from] crate::TimerError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComponentStateSchema, RuntimeEngine, StateField, SubscriptionWork, ValueSchema};

    fn mounted_context(phase: ExecutionPhase) -> UiContext {
        let path = ComponentInstancePath::root("Counter", "counter");
        let schema = ComponentStateSchema::new(BTreeMap::from([
            (
                "count".to_owned(),
                StateField::new(ValueSchema::integer(), UiValue::Integer(0)),
            ),
            (
                "profile".to_owned(),
                StateField::new(
                    ValueSchema::UiValue,
                    UiValue::Map(BTreeMap::from([(
                        "name".to_owned(),
                        UiValue::String("Ada".to_owned()),
                    )])),
                ),
            ),
        ]))
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
    fn locale_and_viewport_reads_register_exact_component_dependencies() {
        let engine = Engine::new();
        let en = crate::load_locale_source(
            &engine,
            "en.rhai",
            include_str!("../../../registry/locales/en.rhai"),
        )
        .unwrap();
        let zh = crate::load_locale_source(
            &engine,
            "zh_cn.rhai",
            include_str!("../../../registry/locales/zh_cn.rhai"),
        )
        .unwrap();
        let mut state = UiRuntimeState::new();
        state.locale = Some(LocaleManager::new([en, zh], "en", "en").unwrap());
        state.responsive.update_window("main", 480.0).unwrap();
        state.windows.register_open("main").unwrap();
        let runtime = Rc::new(RefCell::new(state));
        let reader = ComponentInstancePath::root("View", "main").child("Reader", "reader");
        let unrelated = ComponentInstancePath::root("View", "main").child("Static", "static");
        let render_context = UiContext::new(
            Rc::clone(&runtime),
            reader.clone(),
            Some("main".to_owned()),
            ExecutionPhase::Render,
            BTreeMap::new(),
        );
        assert!(!render_context.text("common.loading").unwrap().is_empty());
        assert_eq!(render_context.viewport_class().unwrap(), "compact");
        let event = UiContext::new(
            Rc::clone(&runtime),
            unrelated,
            Some("main".to_owned()),
            ExecutionPhase::Event,
            BTreeMap::new(),
        );
        event.set_locale("zh-CN").unwrap();

        assert_eq!(
            runtime.borrow_mut().drain_batch().dirty,
            BTreeSet::from([reader.clone()])
        );
        assert_eq!(
            runtime
                .borrow()
                .environment_dependencies
                .invalidate_viewport("main"),
            BTreeSet::from([reader])
        );
        assert!(runtime.borrow_mut().take_window_repaint("main"));
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
    fn scripts_use_bounded_nested_and_keyed_store_paths() {
        let context = mounted_context(ExecutionPhase::Event);
        let row = |id: &str, label: &str| {
            UiValue::Map(BTreeMap::from([
                ("id".to_owned(), UiValue::String(id.to_owned())),
                ("label".to_owned(), UiValue::String(label.to_owned())),
            ]))
        };
        context
            .runtime()
            .borrow_mut()
            .stores
            .declare(
                StoreId::app("model"),
                ComponentStateSchema::new(BTreeMap::from([(
                    "data".to_owned(),
                    StateField::new(
                        ValueSchema::UiValue,
                        UiValue::Map(BTreeMap::from([(
                            "rows".to_owned(),
                            UiValue::Array(vec![row("alpha", "Alpha"), row("beta", "Beta")]),
                        )])),
                    ),
                )]))
                .unwrap(),
            )
            .unwrap();
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn view() { text("paths") }
                    fn update(ctx) {
                        let name = ctx.get_state_path("profile", ["name"]);
                        ctx.set_state_path("profile", ["name"], name + " Lovelace");
                        let label = ctx.get_app_store_path(
                            "model",
                            "data",
                            ["rows", #{ by: "id", key: "beta" }, "label"]
                        );
                        ctx.set_app_store_path(
                            "model",
                            "data",
                            ["rows", #{ by: "id", key: "beta" }, "label"],
                            label + "!"
                        );
                    }
                "#,
            )
            .unwrap();
        runtime.render(&compiled).unwrap();
        let callback = runtime.callback(&compiled, "update").unwrap();
        let _: Dynamic = runtime
            .invoke_callback(&compiled, &callback, (context.clone(),))
            .unwrap();

        let profile = context.get_state("profile").unwrap();
        assert_eq!(
            profile
                .get_path(
                    &UiValuePath::new(vec![UiValuePathSegment::Key("name".to_owned())]).unwrap()
                )
                .unwrap(),
            &UiValue::String("Ada Lovelace".to_owned())
        );
        let row_path = UiValuePath::new(vec![
            UiValuePathSegment::Key("rows".to_owned()),
            UiValuePathSegment::Item {
                key_field: "id".to_owned(),
                key: "beta".to_owned(),
            },
            UiValuePathSegment::Key("label".to_owned()),
        ])
        .unwrap();
        assert_eq!(
            context
                .runtime()
                .borrow_mut()
                .stores
                .read_path_tracked(
                    &ComponentInstancePath::root("Test", "reader"),
                    &StoreId::app("model"),
                    "data",
                    &row_path,
                )
                .unwrap(),
            UiValue::String("Beta!".to_owned())
        );
        assert!(rhai_value_path(vec![Dynamic::from(-1_i64)]).is_err());
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

    #[test]
    fn returning_subscription_work_closes_external_emitters_with_reason() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn view() { text("subscription") }
                    fn success(ctx, value) { value }
                    fn failure(ctx, error) { error }
                "#,
            )
            .unwrap();
        let generation = compiled.generation();
        let success = engine.callback(&compiled, "success").unwrap();
        let failure = engine.callback(&compiled, "failure").unwrap();
        let mut state = UiRuntimeState::new();
        let registration = SubscriptionRegistration::new(
            "app.stream.watch",
            AsyncScope::App,
            generation,
            success,
            failure,
            ValueSchema::integer(),
        );
        let (_, emitter) = state.subscriptions.subscribe(registration);
        let external = emitter.clone();
        run_subscription_work(SubscriptionWork::new(|_| {}), emitter.clone(), &emitter);

        assert!(matches!(
            external.emit(UiValue::Integer(1)),
            Err(AsyncRuntimeError::Closed {
                reason: SubscriptionCloseReason::WorkReturned
            })
        ));
        let _ = state.subscriptions.drain(generation);
        assert_eq!(
            state.subscriptions.take_closures()[0].reason,
            SubscriptionCloseReason::WorkReturned
        );
        let registration = SubscriptionRegistration::new(
            "app.stream.trace",
            AsyncScope::App,
            generation,
            engine.callback(&compiled, "success").unwrap(),
            engine.callback(&compiled, "failure").unwrap(),
            ValueSchema::integer(),
        );
        let (_, emitter) = state.subscriptions.subscribe(registration);
        emitter.close_with_reason(SubscriptionCloseReason::WorkReturned);
        let _ = state.subscriptions.drain(generation);
        state.trace_subscription_closures();
        let traces = state.traces.snapshot();
        assert!(traces.iter().any(|trace| {
            trace.kind == crate::RuntimeTraceKind::Subscription
                && trace.message == "close app.stream.trace: the subscription work returned"
        }));
    }

    #[test]
    fn receiver_subscription_forwards_external_values_until_sender_drop() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn view() { text("subscription") }
                    fn success(ctx, value) { value }
                    fn failure(ctx, error) { error }
                "#,
            )
            .unwrap();
        let generation = compiled.generation();
        let registration = SubscriptionRegistration::new(
            "app.stream.receiver",
            AsyncScope::App,
            generation,
            engine.callback(&compiled, "success").unwrap(),
            engine.callback(&compiled, "failure").unwrap(),
            ValueSchema::integer(),
        );
        let mut subscriptions = SubscriptionRegistry::new();
        let (_, emitter) = subscriptions.subscribe(registration);
        let external = emitter.clone();
        let closer = emitter.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            run_subscription_work(SubscriptionWork::from_receiver(receiver), emitter, &closer);
        });
        sender.send(UiValue::Integer(42)).unwrap();
        drop(sender);
        worker.join().unwrap();

        let deliveries = subscriptions.drain(generation);
        assert_eq!(deliveries[0].payload, UiValue::Integer(42));
        assert!(matches!(
            external.emit(UiValue::Integer(43)),
            Err(AsyncRuntimeError::Closed {
                reason: SubscriptionCloseReason::WorkReturned
            })
        ));
    }

    #[test]
    fn focus_by_ref_key_queues_a_window_scoped_retained_command() {
        let context = mounted_context(ExecutionPhase::Event);
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(crate::UiNode::text("field")).unwrap();
        let reference = crate::ElementRef::new(
            crate::ElementRefId::new(context.component_path().clone(), "field").unwrap(),
        );
        context.runtime().borrow_mut().element_refs.reconcile(
            context.component_path(),
            BTreeMap::from([(reference.id().clone(), tree.root_id().unwrap())]),
        );
        context.focus_element_by_key("field").unwrap();
        context
            .scroll_element_to_by_key("field", 12.0, 24.0)
            .unwrap();
        context.scroll_element_into_view_by_key("field").unwrap();
        let commands = context
            .runtime()
            .borrow_mut()
            .take_window_element_commands("main");
        assert!(matches!(
            commands.as_slice(),
            [crate::element_ref::ElementCommand::Focus { node, .. },
             crate::element_ref::ElementCommand::ScrollTo { node: scroll_node, x, y, .. },
             crate::element_ref::ElementCommand::ScrollIntoView { node: reveal_node, .. }]
                if *node == tree.root_id().unwrap()
                    && scroll_node == node
                    && reveal_node == node
                    && (*x - 12.0).abs() < f64::EPSILON
                    && (*y - 24.0).abs() < f64::EPSILON
        ));
    }
}
