use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use rhai::Dynamic;
use thiserror::Error;

use crate::{
    AnimationError, AsyncDelivery, AsyncScope, CompiledUi, ComponentInstancePath,
    ComponentStateSchema, EventSchema, ExecutionPhase, RuntimeEngine, RuntimeError, ScriptCallback,
    ScriptGeneration, StateError, UiContext, UiNode, UiRuntimeState, UiValue,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleState {
    Created,
    Initialized,
    Running,
    Disposed,
}

pub struct ScriptLifecycle {
    compiled: CompiledUi,
    runtime: Rc<RefCell<UiRuntimeState>>,
    root_path: ComponentInstancePath,
    window: Option<String>,
    view: Option<String>,
    events: BTreeMap<String, EventSchema>,
    state: LifecycleState,
    root: Option<UiNode>,
    retained: crate::RetainedUiTree,
}

#[derive(Default)]
struct RetainedDeclarations {
    effects: BTreeMap<crate::EffectId, crate::EffectDescriptor>,
    timers: BTreeMap<crate::TimerId, crate::TimerDescriptor>,
    signals: BTreeMap<crate::SignalId, crate::signal::SignalDescriptor>,
    element_refs: BTreeMap<crate::ElementRefId, crate::NodeId>,
}

impl ScriptLifecycle {
    /// Create an application lifecycle and mount its root component state.
    ///
    /// # Errors
    ///
    /// Returns [`LifecycleError`] when runtime state is already borrowed or the
    /// root state schema cannot mount.
    pub fn new(
        compiled: CompiledUi,
        runtime: Rc<RefCell<UiRuntimeState>>,
        root_path: ComponentInstancePath,
        window: Option<String>,
        events: BTreeMap<String, EventSchema>,
        state_schema: &ComponentStateSchema,
    ) -> Result<Self, LifecycleError> {
        {
            let mut runtime_state = runtime
                .try_borrow_mut()
                .map_err(|_| LifecycleError::Borrowed)?;
            let mut transaction = runtime_state
                .component_state
                .begin_render_scope(root_path.clone());
            transaction.mount(root_path.clone(), state_schema)?;
            runtime_state.component_state.commit_render(transaction);
        }
        Ok(Self {
            compiled,
            runtime,
            root_path,
            window,
            view: None,
            events,
            state: LifecycleState::Created,
            root: None,
            retained: crate::RetainedUiTree::new(),
        })
    }

    #[must_use]
    pub const fn state(&self) -> LifecycleState {
        self.state
    }

    #[must_use]
    pub fn root(&self) -> Option<&UiNode> {
        self.root.as_ref()
    }

    #[must_use]
    pub const fn retained(&self) -> &crate::RetainedUiTree {
        &self.retained
    }

    #[must_use]
    pub fn generation(&self) -> ScriptGeneration {
        self.compiled.generation()
    }

    #[must_use]
    pub fn runtime(&self) -> Rc<RefCell<UiRuntimeState>> {
        Rc::clone(&self.runtime)
    }

    #[must_use]
    pub fn with_view_id(mut self, view: impl Into<String>) -> Self {
        self.view = Some(view.into());
        self
    }

    /// Run optional `init(ctx)` exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`LifecycleError::InvalidTransition`] outside `Created`, or a
    /// script evaluation error.
    pub fn initialize(&mut self, engine: &RuntimeEngine) -> Result<(), LifecycleError> {
        self.require_state(LifecycleState::Created)?;
        let context = self.context(ExecutionPhase::Init);
        engine.call_optional_lifecycle(&self.compiled, "init", context)?;
        self.state = LifecycleState::Initialized;
        Ok(())
    }

    /// Evaluate required `view(ctx)` and activate its generation on success.
    ///
    /// # Errors
    ///
    /// Returns [`LifecycleError::InvalidTransition`] before initialization or
    /// after disposal, or a script evaluation error.
    pub fn render(&mut self, engine: &mut RuntimeEngine) -> Result<&UiNode, LifecycleError> {
        if !matches!(
            self.state,
            LifecycleState::Initialized | LifecycleState::Running
        ) {
            return Err(LifecycleError::InvalidTransition {
                from: self.state,
                operation: "render",
            });
        }
        let runtime_snapshot = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .snapshot()?;
        let engine_checkpoint = engine.execution_checkpoint();
        let result = (|| {
            let context = self.context(ExecutionPhase::Render);
            let root = engine.render_with_context_staged(&self.compiled, context)?;
            let mut retained = self.retained.clone();
            retained.reconcile(root.clone())?;
            self.validate_resource_budgets(engine, &retained)?;
            self.reconcile_animations(&root)?;
            self.reconcile_effects(
                engine,
                &self.compiled,
                runtime_snapshot.component_state().clone(),
                &retained,
            )?;
            self.retain_geometry_nodes(&retained)?;
            self.validate_signal_bindings(&root)?;
            Ok((root, retained))
        })();
        match result {
            Ok((root, retained)) => {
                self.trace_reconcile("full", retained.last_report());
                self.retained = retained;
                self.state = LifecycleState::Running;
                Ok(self.root.insert(root))
            }
            Err(error) => {
                self.runtime
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .restore(runtime_snapshot)?;
                engine.restore_execution_checkpoint(engine_checkpoint);
                Err(error)
            }
        }
    }

    /// Rerender only the topmost dirty formal component subtrees.
    ///
    /// Falls back to a complete root render when the root itself is dirty or a
    /// component has no active invocation recipe.
    ///
    /// # Errors
    ///
    /// Returns component invocation, script evaluation, reconciliation, or
    /// runtime-state rollback errors.
    pub fn render_dirty(&mut self, engine: &mut RuntimeEngine) -> Result<bool, LifecycleError> {
        if !self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .has_window_dirty(&self.root_path)
        {
            return Ok(false);
        }
        let runtime_snapshot = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .snapshot()?;
        let engine_checkpoint = engine.execution_checkpoint();
        let dirty = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?
            .take_window_dirty_components(&self.root_path);
        debug_assert!(!dirty.is_empty());
        let dirty = topmost_paths(&dirty);
        if dirty.iter().any(|path| path == &self.root_path)
            || dirty.iter().any(|path| {
                engine
                    .component_invocations()
                    .all(|recipe| recipe.path() != path)
            })
        {
            return match self.render(engine) {
                Ok(_) => Ok(true),
                Err(error) => {
                    self.runtime
                        .try_borrow_mut()
                        .map_err(|_| LifecycleError::Borrowed)?
                        .restore(runtime_snapshot)?;
                    engine.restore_execution_checkpoint(engine_checkpoint);
                    Err(error)
                }
            };
        }

        let mut root = self.root.clone().ok_or(LifecycleError::MissingRoot)?;
        let mut retained = self.retained.clone();
        let result = (|| {
            for path in dirty {
                let subtree = engine.rerender_component(&path)?;
                if !root.replace_component_subtree(&path, subtree) {
                    return Err(LifecycleError::MissingComponentSubtree(path));
                }
            }
            retained.reconcile(root.clone())?;
            self.validate_resource_budgets(engine, &retained)?;
            self.reconcile_animations(&root)?;
            self.reconcile_effects(
                engine,
                &self.compiled,
                runtime_snapshot.component_state().clone(),
                &retained,
            )?;
            self.retain_geometry_nodes(&retained)?;
            self.validate_signal_bindings(&root)?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.trace_reconcile("incremental", retained.last_report());
                self.root = Some(root);
                self.retained = retained;
                self.state = LifecycleState::Running;
                Ok(true)
            }
            Err(error) => {
                self.runtime
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .restore(runtime_snapshot)?;
                engine.restore_execution_checkpoint(engine_checkpoint);
                Err(error)
            }
        }
    }

    /// Realize requested data-backed virtual items outside GPUI layout/paint.
    ///
    /// # Errors
    ///
    /// Returns renderer, retained reconciliation, effect, or rollback errors.
    pub fn realize_virtual_requests(
        &mut self,
        engine: &mut RuntimeEngine,
    ) -> Result<bool, LifecycleError> {
        let runtime_snapshot = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .snapshot()?;
        let engine_checkpoint = engine.execution_checkpoint();
        let requests = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .virtual_requests
            .drain();
        let mut selected = Vec::new();
        for (id, indices) in requests {
            if id.component.is_within(&self.root_path) {
                selected.push((id, indices));
            } else {
                self.runtime
                    .try_borrow()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .virtual_requests
                    .request(id, indices);
            }
        }
        if selected.is_empty() {
            return Ok(false);
        }
        let mut root = self.root.clone().ok_or(LifecycleError::MissingRoot)?;
        let mut retained = self.retained.clone();
        let result = (|| {
            let mut changed = false;
            for (id, indices) in selected {
                let existing = root
                    .virtual_collection_items(&id)
                    .cloned()
                    .ok_or_else(|| LifecycleError::MissingVirtualCollection(id.clone()))?;
                let missing = indices
                    .iter()
                    .filter(|index| !existing.contains_key(index))
                    .copied()
                    .collect::<BTreeSet<_>>();
                if missing.is_empty() {
                    continue;
                }
                let items = engine.realize_virtual_collection(&id, &indices)?;
                if !root.replace_virtual_collection_items(&id, items) {
                    return Err(LifecycleError::MissingVirtualCollection(id));
                }
                changed = true;
            }
            if !changed {
                return Ok(false);
            }
            retained.reconcile(root.clone())?;
            self.validate_resource_budgets(engine, &retained)?;
            self.reconcile_animations(&root)?;
            self.reconcile_effects(
                engine,
                &self.compiled,
                runtime_snapshot.component_state().clone(),
                &retained,
            )?;
            self.retain_geometry_nodes(&retained)?;
            self.validate_signal_bindings(&root)?;
            Ok(true)
        })();
        match result {
            Ok(changed) => {
                if changed {
                    self.trace_reconcile("virtual", retained.last_report());
                    self.root = Some(root);
                    self.retained = retained;
                }
                Ok(changed)
            }
            Err(error) => {
                self.runtime
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .restore(runtime_snapshot)?;
                engine.restore_execution_checkpoint(engine_checkpoint);
                Err(error)
            }
        }
    }

    /// Run optional `dispose(ctx)` once and make the lifecycle terminal.
    ///
    /// # Errors
    ///
    /// Returns [`LifecycleError::InvalidTransition`] before initialization or
    /// after disposal, or a script evaluation error.
    pub fn dispose(&mut self, engine: &mut RuntimeEngine) -> Result<(), LifecycleError> {
        if !matches!(
            self.state,
            LifecycleState::Initialized | LifecycleState::Running
        ) {
            return Err(LifecycleError::InvalidTransition {
                from: self.state,
                operation: "dispose",
            });
        }
        let snapshot = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .snapshot()?;
        let result = (|| {
            self.reconcile_effect_candidate(
                engine,
                &self.compiled,
                RetainedDeclarations::default(),
                None,
            )?;
            let context = self.context(ExecutionPhase::Dispose);
            engine.call_optional_lifecycle(&self.compiled, "dispose", context)?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.state = LifecycleState::Disposed;
                Ok(())
            }
            Err(error) => {
                self.runtime
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .restore(snapshot)?;
                Err(error)
            }
        }
    }

    /// Invoke a generation-bound event callback with `(ctx, payload)`.
    ///
    /// # Errors
    ///
    /// Returns stale callback or Rhai evaluation errors.
    pub fn invoke_callback(
        &self,
        engine: &RuntimeEngine,
        callback: &ScriptCallback,
        payload: UiValue,
    ) -> Result<Dynamic, LifecycleError> {
        let root_context = self.context(ExecutionPhase::Event);
        let context = callback
            .component()
            .map_or(root_context.clone(), |component| {
                root_context.for_component(component.clone(), callback.events().clone())
            })
            .with_native_context(callback.native_context().cloned());
        Ok(engine.invoke_callback(&self.compiled, callback, (context, payload.into_dynamic()))?)
    }

    /// Invoke a foreground callback and roll back runtime UI state if it fails.
    /// External capability side effects are outside this transaction.
    ///
    /// # Errors
    ///
    /// Returns callback or runtime borrow errors.
    pub fn invoke_callback_transactional(
        &self,
        engine: &RuntimeEngine,
        callback: &ScriptCallback,
        payload: UiValue,
    ) -> Result<Dynamic, LifecycleError> {
        let snapshot = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .snapshot()?;
        match self.invoke_callback(engine, callback, payload) {
            Ok(value) => Ok(value),
            Err(error) => {
                self.runtime
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .restore(snapshot)?;
                Err(error)
            }
        }
    }

    /// Invoke an async delivery in its owning component scope.
    ///
    /// # Errors
    ///
    /// Returns stale callback or Rhai evaluation errors.
    pub fn invoke_async_delivery(
        &self,
        engine: &RuntimeEngine,
        delivery: AsyncDelivery,
    ) -> Result<Dynamic, LifecycleError> {
        let component =
            delivery
                .callback
                .component()
                .cloned()
                .unwrap_or_else(|| match delivery.scope {
                    AsyncScope::Component(component) => component,
                    AsyncScope::App | AsyncScope::Window(_) => self.root_path.clone(),
                });
        let events = delivery.callback.events().clone();
        let context = UiContext::new(
            Rc::clone(&self.runtime),
            component,
            self.window.clone(),
            ExecutionPhase::Event,
            events,
        )
        .with_optional_view_id(self.view.clone())
        .with_generation(self.compiled.generation())
        .with_native_context(delivery.callback.native_context().cloned());
        Ok(engine.invoke_callback(
            &self.compiled,
            &delivery.callback,
            (context, delivery.payload.into_dynamic()),
        )?)
    }

    /// Deliver async work with the same UI-state rollback as foreground events.
    ///
    /// # Errors
    ///
    /// Returns callback or runtime borrow errors.
    pub fn invoke_async_delivery_transactional(
        &self,
        engine: &RuntimeEngine,
        delivery: AsyncDelivery,
    ) -> Result<Dynamic, LifecycleError> {
        let snapshot = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .snapshot()?;
        match self.invoke_async_delivery(engine, delivery) {
            Ok(value) => Ok(value),
            Err(error) => {
                self.runtime
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .restore(snapshot)?;
                Err(error)
            }
        }
    }

    /// Deliver one declared component event to its rendered caller callback.
    /// Missing optional listeners are treated as intentionally unobserved.
    ///
    /// # Errors
    ///
    /// Returns callback or runtime borrow errors.
    pub fn invoke_component_event_transactional(
        &self,
        engine: &RuntimeEngine,
        pending: crate::PendingEvent,
    ) -> Result<Option<Dynamic>, LifecycleError> {
        let callback = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .component_event_handler(&pending.target, &pending.event.name);
        callback
            .map(|callback| {
                self.invoke_callback_transactional(engine, &callback, pending.event.payload)
            })
            .transpose()
    }

    /// Transactionally run candidate `init(ctx)` and `view(ctx)` during hot reload.
    ///
    /// Component/store state and the last-good AST/root are preserved when
    /// either candidate lifecycle stage fails.
    ///
    /// # Errors
    ///
    /// Returns runtime or borrow errors from the candidate generation.
    pub fn reload(
        &mut self,
        engine: &mut RuntimeEngine,
        candidate: CompiledUi,
        state_schema: &ComponentStateSchema,
    ) -> Result<&UiNode, LifecycleError> {
        if self.state == LifecycleState::Disposed {
            return Err(LifecycleError::InvalidTransition {
                from: self.state,
                operation: "reload",
            });
        }
        let snapshot = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .snapshot()?;
        let engine_checkpoint = engine.execution_checkpoint();
        {
            let mut runtime = self
                .runtime
                .try_borrow_mut()
                .map_err(|_| LifecycleError::Borrowed)?;
            runtime
                .component_state
                .mount_instance(self.root_path.clone(), state_schema)?;
        }
        let result: Result<(UiNode, crate::RetainedUiTree), LifecycleError> = (|| {
            engine.call_optional_lifecycle(
                &candidate,
                "init",
                self.context_for(ExecutionPhase::Init, candidate.generation()),
            )?;
            let root = engine.render_with_context_staged(
                &candidate,
                self.context_for(ExecutionPhase::Render, candidate.generation()),
            )?;
            let mut retained = self.retained.clone();
            retained.reconcile(root.clone())?;
            self.validate_resource_budgets(engine, &retained)?;
            self.reconcile_animations(&root)?;
            self.reconcile_effects(
                engine,
                &candidate,
                snapshot.component_state().clone(),
                &retained,
            )?;
            self.retain_geometry_nodes(&retained)?;
            self.validate_signal_bindings(&root)?;
            Ok((root, retained))
        })();
        match result {
            Ok((root, retained)) => {
                self.trace_reconcile("reload", retained.last_report());
                self.compiled = candidate;
                self.retained = retained;
                self.state = LifecycleState::Running;
                Ok(self.root.insert(root))
            }
            Err(error) => {
                self.runtime
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .restore(snapshot)?;
                engine.restore_execution_checkpoint(engine_checkpoint);
                Err(error)
            }
        }
    }

    /// Run initialization followed by the first render.
    ///
    /// # Errors
    ///
    /// Returns lifecycle or script errors from either stage.
    pub fn start(&mut self, engine: &mut RuntimeEngine) -> Result<&UiNode, LifecycleError> {
        self.initialize(engine)?;
        self.render(engine)
    }

    fn context(&self, phase: ExecutionPhase) -> UiContext {
        self.context_for(phase, self.compiled.generation())
    }

    fn reconcile_animations(&self, root: &UiNode) -> Result<(), LifecycleError> {
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?;
        let now = runtime.clock.now();
        let values = crate::animation::reconcile_node_animations_scoped(
            root,
            &mut runtime.animations,
            now,
            &self.animation_root_path(),
        )?;
        runtime.animation_values = values;
        Ok(())
    }

    fn validate_resource_budgets(
        &self,
        engine: &RuntimeEngine,
        retained: &crate::RetainedUiTree,
    ) -> Result<(), LifecycleError> {
        let budgets = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .budgets
            .clone();
        budgets.validate()?;
        crate::RuntimeBudgets::check("retained_nodes", retained.len(), budgets.retained_nodes)?;
        let handlers = retained.nodes().fold(0usize, |total, node| {
            total.saturating_add(node.handler_count())
        });
        crate::RuntimeBudgets::check("event_handlers", handlers, budgets.event_handlers)?;
        let canvas_commands = retained.nodes().fold(0usize, |total, node| {
            total.saturating_add(node.canvas_command_count())
        });
        crate::RuntimeBudgets::check("canvas_commands", canvas_commands, budgets.canvas_commands)?;
        let virtual_data = retained.nodes().fold(0usize, |total, node| {
            total.saturating_add(node.virtual_data_item_count())
        });
        let virtual_realized = retained.nodes().fold(0usize, |total, node| {
            total.saturating_add(node.virtual_realized_item_count())
        });
        crate::RuntimeBudgets::check(
            "virtual_data_items",
            virtual_data,
            budgets.virtual_data_items,
        )?;
        crate::RuntimeBudgets::check(
            "virtual_realized_items",
            virtual_realized,
            budgets.virtual_realized_items,
        )?;
        crate::RuntimeBudgets::check(
            "formal_components",
            engine.component_invocations().len(),
            budgets.formal_components,
        )?;
        crate::RuntimeBudgets::check(
            "effects",
            engine.component_effects_in_scope(&self.root_path).len(),
            budgets.effects,
        )?;
        crate::RuntimeBudgets::check(
            "signals",
            engine.component_signals_in_scope(&self.root_path).len(),
            budgets.signals,
        )?;
        crate::RuntimeBudgets::check(
            "element_refs",
            engine
                .component_element_refs_in_scope(&self.root_path)
                .len(),
            budgets.element_refs,
        )?;
        crate::RuntimeBudgets::check(
            "timers",
            engine.component_timers_in_scope(&self.root_path).len(),
            budgets.timers,
        )?;
        Ok(())
    }

    fn reconcile_effects(
        &self,
        engine: &mut RuntimeEngine,
        candidate: &CompiledUi,
        previous_state: crate::StateStore,
        retained: &crate::RetainedUiTree,
    ) -> Result<(), LifecycleError> {
        let declarations = RetainedDeclarations {
            effects: engine.component_effects_in_scope(&self.root_path),
            timers: engine.component_timers_in_scope(&self.root_path),
            signals: engine.component_signals_in_scope(&self.root_path),
            element_refs: self.element_ref_bindings(engine, retained)?,
        };
        self.reconcile_effect_candidate(engine, candidate, declarations, Some(previous_state))
    }

    fn reconcile_effect_candidate(
        &self,
        engine: &mut RuntimeEngine,
        candidate: &CompiledUi,
        declarations: RetainedDeclarations,
        previous_state: Option<crate::StateStore>,
    ) -> Result<(), LifecycleError> {
        let plan = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .effects
            .plan(&self.root_path, declarations.effects);
        let transition_count = plan.cleanup().len().saturating_add(plan.start().len());
        if transition_count > 64 {
            return Err(LifecycleError::EffectBudget(transition_count));
        }
        if !plan.cleanup().is_empty()
            && let Some(previous_state) = previous_state
        {
            self.runtime
                .try_borrow_mut()
                .map_err(|_| LifecycleError::Borrowed)?
                .component_state = previous_state;
        }
        for descriptor in plan.cleanup() {
            self.invoke_effect_callback(
                engine,
                candidate,
                descriptor.cleanup(),
                descriptor.dependencies().clone(),
            )?;
        }
        let virtual_collections = engine.virtual_collection_ids_in_scope(&self.root_path);
        {
            let mut runtime = self
                .runtime
                .try_borrow_mut()
                .map_err(|_| LifecycleError::Borrowed)?;
            let now = runtime.clock.now();
            engine.commit_component_renders(&mut runtime)?;
            runtime
                .timers
                .reconcile(&self.root_path, declarations.timers, now);
            runtime
                .signals
                .reconcile(&self.root_path, declarations.signals);
            runtime
                .element_refs
                .reconcile(&self.root_path, declarations.element_refs);
            runtime.virtual_requests.retain(&virtual_collections);
        }
        for descriptor in plan.start() {
            self.invoke_effect_callback(
                engine,
                candidate,
                descriptor.start(),
                descriptor.dependencies().clone(),
            )?;
        }
        self.runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?
            .effects
            .commit(plan);
        Ok(())
    }

    fn invoke_effect_callback(
        &self,
        engine: &RuntimeEngine,
        candidate: &CompiledUi,
        callback: &ScriptCallback,
        dependencies: UiValue,
    ) -> Result<(), LifecycleError> {
        let compiled = if callback.generation() == candidate.generation() {
            candidate
        } else if callback.generation() == self.compiled.generation() {
            &self.compiled
        } else {
            return Err(LifecycleError::StaleEffect {
                name: callback.name().to_owned(),
                generation: callback.generation(),
            });
        };
        let root_context = self.context_for(ExecutionPhase::Event, compiled.generation());
        let context = callback
            .component()
            .map_or(root_context.clone(), |component| {
                root_context.for_component(component.clone(), callback.events().clone())
            })
            .with_native_context(callback.native_context().cloned());
        let _ = engine.invoke_callback_for_generation(
            compiled,
            callback,
            (context, dependencies.into_dynamic()),
        )?;
        Ok(())
    }

    fn validate_signal_bindings(&self, root: &UiNode) -> Result<(), LifecycleError> {
        let runtime = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?;
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            for (_, signal) in node.signal_bindings() {
                let _ = runtime.signals.read(signal)?;
            }
            for (_, children) in node.retained_child_groups() {
                pending.extend(children);
            }
        }
        Ok(())
    }

    fn element_ref_bindings(
        &self,
        engine: &RuntimeEngine,
        retained: &crate::RetainedUiTree,
    ) -> Result<BTreeMap<crate::ElementRefId, crate::NodeId>, LifecycleError> {
        let declared = engine.component_element_refs_in_scope(&self.root_path);
        let mut bindings = BTreeMap::new();
        for node in retained.nodes() {
            let Some(reference) = node.element_ref() else {
                continue;
            };
            if node.key().is_none() {
                return Err(crate::ElementRefError::MissingNodeKey(reference.id().clone()).into());
            }
            if !declared.contains(reference.id()) {
                return Err(crate::ElementRefError::Undeclared(reference.id().clone()).into());
            }
            if bindings.insert(reference.id().clone(), node.id()).is_some() {
                return Err(
                    crate::ElementRefError::DuplicateBinding(reference.id().clone()).into(),
                );
            }
        }
        Ok(bindings)
    }

    fn retain_geometry_nodes(
        &self,
        retained: &crate::RetainedUiTree,
    ) -> Result<(), LifecycleError> {
        let nodes = retained.nodes().map(crate::RetainedNode::id).collect();
        let runtime = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?;
        runtime.geometry.retain_nodes(&nodes);
        runtime.pointer_capture.retain_nodes(&nodes);
        Ok(())
    }

    #[must_use]
    pub fn window_id(&self) -> Option<&str> {
        self.window.as_deref()
    }

    #[must_use]
    pub fn view_id(&self) -> Option<&str> {
        self.view.as_deref()
    }

    #[must_use]
    pub fn root_path(&self) -> &ComponentInstancePath {
        &self.root_path
    }

    #[must_use]
    pub fn compiled(&self) -> CompiledUi {
        self.compiled.clone()
    }

    fn animation_root_path(&self) -> String {
        match (self.window.as_deref(), self.view.as_deref()) {
            (Some(window), Some(view)) => format!("window:{window}/view:{view}/root"),
            (Some(window), None) => format!("window:{window}/root"),
            (None, Some(view)) => format!("view:{view}/root"),
            (None, None) => "root".to_owned(),
        }
    }

    fn trace_reconcile(&self, operation: &str, report: &crate::ReconcileReport) {
        const ID_SAMPLE_LIMIT: usize = 16;

        fn ids(values: &[crate::NodeId]) -> crate::UiValue {
            crate::UiValue::Array(
                values
                    .iter()
                    .take(ID_SAMPLE_LIMIT)
                    .map(|id| crate::UiValue::Integer(i64::try_from(id.get()).unwrap_or(i64::MAX)))
                    .collect(),
            )
        }

        let metrics = report.metrics();
        let truncated = [
            &report.mounted,
            &report.preserved,
            &report.moved,
            &report.unmounted,
        ]
        .into_iter()
        .any(|nodes| nodes.len() > ID_SAMPLE_LIMIT);
        let payload = crate::UiValue::Map(BTreeMap::from([
            ("mounted".to_owned(), ids(&report.mounted)),
            ("preserved".to_owned(), ids(&report.preserved)),
            ("moved".to_owned(), ids(&report.moved)),
            ("unmounted".to_owned(), ids(&report.unmounted)),
            ("truncated".to_owned(), crate::UiValue::Bool(truncated)),
        ]));
        if let Ok(mut runtime) = self.runtime.try_borrow_mut() {
            runtime.traces.push(
                crate::RuntimeTraceKind::Reconcile,
                self.root_path.to_string(),
                format!(
                    "{operation}: mounted={} preserved={} moved={} unmounted={}",
                    metrics.mounted, metrics.preserved, metrics.moved, metrics.unmounted
                ),
                Some(payload),
                false,
            );
        }
    }

    fn context_for(&self, phase: ExecutionPhase, generation: ScriptGeneration) -> UiContext {
        UiContext::new(
            Rc::clone(&self.runtime),
            self.root_path.clone(),
            self.window.clone(),
            phase,
            self.events.clone(),
        )
        .with_optional_view_id(self.view.clone())
        .with_generation(generation)
    }

    fn require_state(&self, required: LifecycleState) -> Result<(), LifecycleError> {
        if self.state == required {
            Ok(())
        } else {
            Err(LifecycleError::InvalidTransition {
                from: self.state,
                operation: "initialize",
            })
        }
    }
}

#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("cannot {operation} while lifecycle is {from:?}")]
    InvalidTransition {
        from: LifecycleState,
        operation: &'static str,
    },
    #[error("UI runtime state is already borrowed")]
    Borrowed,
    #[error(transparent)]
    State(#[from] StateError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Animation(#[from] AnimationError),
    #[error(transparent)]
    Reconcile(#[from] crate::ReconcileError),
    #[error("script lifecycle has no accepted root")]
    MissingRoot,
    #[error("component subtree `{0}` is missing from the accepted UiNode snapshot")]
    MissingComponentSubtree(ComponentInstancePath),
    #[error("virtual collection `{0:?}` is missing from the accepted UiNode snapshot")]
    MissingVirtualCollection(crate::VirtualCollectionId),
    #[error("effect transition exceeded the 64-callback budget with {0} callbacks")]
    EffectBudget(usize),
    #[error("effect callback `{name}` belongs to unavailable generation {generation}")]
    StaleEffect {
        name: String,
        generation: ScriptGeneration,
    },
    #[error(transparent)]
    Asset(#[from] crate::AssetError),
    #[error(transparent)]
    Signal(#[from] crate::SignalError),
    #[error(transparent)]
    ElementRef(#[from] crate::ElementRefError),
    #[error(transparent)]
    Budget(#[from] crate::RuntimeBudgetError),
}

fn topmost_paths(paths: &BTreeSet<ComponentInstancePath>) -> Vec<ComponentInstancePath> {
    paths
        .iter()
        .filter(|path| {
            !paths
                .iter()
                .any(|candidate| *path != candidate && path.is_within(candidate))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AssetData, AsyncCapabilityHandler, CapabilityDescriptor, CapabilityId, CapabilityMethod,
        InMemoryAssetProvider, OpaqueHandle, StateField, SubscriptionCapabilityHandler,
        SubscriptionWork, TaskWork, UiValue, ValueSchema,
    };
    use semver::{Version, VersionReq};
    use std::time::{Duration, Instant};

    fn state_schema() -> ComponentStateSchema {
        ComponentStateSchema::new(BTreeMap::from([(
            "phase".to_owned(),
            StateField::new(ValueSchema::string(), UiValue::String("created".to_owned())),
        )]))
        .unwrap()
    }

    #[test]
    fn lifecycle_order_and_optional_functions_are_enforced() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn init(ctx) { ctx.set_state("phase", "initialized"); }
                    fn view(ctx) { text(ctx.get_state("phase")) }
                    fn dispose(ctx) { ctx.set_state("phase", "disposed"); }
                "#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let path = ComponentInstancePath::root("App", "root");
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            path.clone(),
            Some("main".to_owned()),
            BTreeMap::new(),
            &state_schema(),
        )
        .unwrap();

        assert!(matches!(
            lifecycle.render(&mut engine),
            Err(LifecycleError::InvalidTransition { .. })
        ));
        let root = lifecycle.start(&mut engine).unwrap();
        assert!(matches!(
            root.kind(),
            crate::UiNodeKind::Text { text } if text == "initialized"
        ));
        assert!(root.source().is_some());
        let reconcile = runtime
            .borrow()
            .traces
            .snapshot()
            .into_iter()
            .find(|trace| trace.kind == crate::RuntimeTraceKind::Reconcile)
            .expect("successful initial render must trace retained mutations");
        assert!(reconcile.message.contains("full: mounted=1"));
        assert!(matches!(reconcile.payload, Some(UiValue::Map(_))));
        lifecycle.dispose(&mut engine).unwrap();
        assert_eq!(lifecycle.state(), LifecycleState::Disposed);
        assert_eq!(
            runtime.borrow().component_state.get(&path, "phase"),
            Some(&UiValue::String("disposed".to_owned()))
        );
        assert!(matches!(
            lifecycle.dispose(&mut engine),
            Err(LifecycleError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn lifecycle_animation_uses_the_host_runtime_clock() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn view(ctx) {
                        text("clocked")
                            .with_key("probe")
                            .animate(transition("width", 0.0, 100.0, 100, "linear"))
                    }
                "#,
            )
            .unwrap();
        let start = Instant::now();
        let manual = crate::ManualRuntimeClock::new(start);
        let mut runtime_state = UiRuntimeState::new();
        runtime_state.clock = manual.clock();
        let runtime = Rc::new(RefCell::new(runtime_state));
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            ComponentInstancePath::root("App", "root"),
            Some("main".to_owned()),
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap();

        lifecycle.start(&mut engine).unwrap();
        assert_eq!(
            runtime
                .borrow()
                .animation_values
                .values()
                .copied()
                .collect::<Vec<_>>(),
            vec![0.0]
        );

        manual.advance(Duration::from_millis(50));
        let mut runtime = runtime.borrow_mut();
        let now = runtime.clock.now();
        let frame = runtime.animations.tick(now);
        assert_eq!(
            frame.values.values().copied().collect::<Vec<_>>(),
            vec![50.0]
        );
    }

    #[test]
    fn failed_event_callback_rolls_back_ui_state() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn view(ctx) { text(ctx.get_state("phase")) }
                    fn fail(ctx, payload) {
                        ctx.set_state("phase", "partial");
                        ctx.register_action("test.temporary", Fn("fail"));
                        ctx.open_window("temporary", "Temporary", 400, 300, false);
                        throw "event failed";
                    }
                "#,
            )
            .unwrap();
        let callback = engine.callback(&compiled, "fail").unwrap();
        let mut runtime_state = UiRuntimeState::new();
        runtime_state.windows.register_open("main").unwrap();
        let runtime = Rc::new(RefCell::new(runtime_state));
        let path = ComponentInstancePath::root("App", "root");
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            path.clone(),
            Some("main".to_owned()),
            BTreeMap::new(),
            &state_schema(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        assert!(
            lifecycle
                .invoke_callback_transactional(&engine, &callback, UiValue::Null)
                .is_err()
        );
        assert_eq!(
            runtime.borrow().component_state.get(&path, "phase"),
            Some(&UiValue::String("created".to_owned()))
        );
        let runtime = runtime.borrow();
        assert!(!runtime.windows.contains("temporary"));
        assert!(
            runtime
                .actions
                .dispatch(
                    &crate::ActionId::parse("test.temporary").unwrap(),
                    UiValue::Null,
                )
                .is_err()
        );
    }

    #[test]
    fn missing_optional_lifecycle_functions_are_valid() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile("fn view(ctx) { text(\"only view\") }")
            .unwrap();
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::new(RefCell::new(UiRuntimeState::new())),
            ComponentInstancePath::root("App", "root"),
            None,
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        lifecycle.dispose(&mut engine).unwrap();
    }

    #[test]
    fn retained_candidate_over_host_budget_is_rejected_before_commit() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile("fn view(ctx) { box([text(\"child\")]) }")
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        runtime.borrow_mut().budgets.retained_nodes = 1;
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            ComponentInstancePath::root("App", "root"),
            Some("main".to_owned()),
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap();
        assert!(matches!(
            lifecycle.start(&mut engine),
            Err(LifecycleError::Budget(
                crate::RuntimeBudgetError::Exceeded {
                    resource: "retained_nodes",
                    actual: 2,
                    limit: 1,
                }
            ))
        ));
        assert!(lifecycle.retained().is_empty());
    }

    #[test]
    fn hot_reload_rolls_back_candidate_init_state_on_render_failure() {
        let mut engine = RuntimeEngine::new();
        let active = engine
            .compile(
                r#"
                    fn init(ctx) { ctx.set_state("phase", "active"); }
                    fn view(ctx) { text(ctx.get_state("phase")) }
                "#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let path = ComponentInstancePath::root("App", "root");
        let mut lifecycle = ScriptLifecycle::new(
            active,
            Rc::clone(&runtime),
            path.clone(),
            None,
            BTreeMap::new(),
            &state_schema(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        let active_generation = lifecycle.generation();

        let rejected = engine
            .compile(
                r#"
                    fn init(ctx) { ctx.set_state("phase", "candidate"); }
                    fn view(ctx) { throw "reject"; }
                "#,
            )
            .unwrap();
        assert!(
            lifecycle
                .reload(&mut engine, rejected, &state_schema())
                .is_err()
        );
        assert_eq!(lifecycle.generation(), active_generation);
        assert_eq!(
            runtime.borrow().component_state.get(&path, "phase"),
            Some(&UiValue::String("active".to_owned()))
        );
    }

    #[test]
    fn hot_reload_rolls_back_engine_generation_after_retained_validation_failure() {
        let mut engine = RuntimeEngine::new();
        let active = engine
            .compile(
                r#"
                    fn init(ctx) { ctx.set_state("phase", "active"); }
                    fn view(ctx) { text(ctx.get_state("phase")) }
                "#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let path = ComponentInstancePath::root("App", "root");
        let mut lifecycle = ScriptLifecycle::new(
            active,
            Rc::clone(&runtime),
            path.clone(),
            None,
            BTreeMap::new(),
            &state_schema(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        let active_generation = lifecycle.generation();
        let reconcile_trace_count = runtime
            .borrow()
            .traces
            .snapshot()
            .iter()
            .filter(|trace| trace.kind == crate::RuntimeTraceKind::Reconcile)
            .count();

        let rejected = engine
            .compile(
                r#"
                    fn init(ctx) { ctx.set_state("phase", "candidate"); }
                    fn view(ctx) {
                        row([
                            text("first").with_key("duplicate"),
                            text("second").with_key("duplicate")
                        ])
                    }
                "#,
            )
            .unwrap();
        assert!(
            lifecycle
                .reload(&mut engine, rejected, &state_schema())
                .is_err()
        );
        assert_eq!(lifecycle.generation(), active_generation);
        assert!(engine.is_current(active_generation));
        assert_eq!(
            runtime.borrow().component_state.get(&path, "phase"),
            Some(&UiValue::String("active".to_owned()))
        );
        assert!(matches!(
            lifecycle.root().unwrap().kind(),
            crate::UiNodeKind::Text { text } if text == "active"
        ));
        assert_eq!(
            runtime
                .borrow()
                .traces
                .snapshot()
                .iter()
                .filter(|trace| trace.kind == crate::RuntimeTraceKind::Reconcile)
                .count(),
            reconcile_trace_count
        );
    }

    #[test]
    fn async_capability_completes_through_foreground_delivery() {
        struct Echo;
        impl AsyncCapabilityHandler for Echo {
            fn start(&mut self, _: &str, input: UiValue) -> Result<TaskWork, String> {
                Ok(Box::new(move || Ok(input)))
            }
        }

        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn init(ctx) {
                        ctx.start_task(
                            "app.echo", "echo", "loaded",
                            Fn("loaded"), Fn("failed")
                        );
                    }
                    fn loaded(ctx, value) { ctx.set_state("phase", value); }
                    fn failed(ctx, error) { ctx.set_state("phase", "failed"); }
                    fn view(ctx) { text(ctx.get_state("phase")) }
                "#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let capability = CapabilityId::parse("app.echo").unwrap();
        runtime
            .borrow_mut()
            .capabilities
            .register_async(
                CapabilityDescriptor {
                    id: capability.clone(),
                    version: Version::new(1, 0, 0),
                    methods: BTreeMap::from([(
                        "echo".to_owned(),
                        CapabilityMethod {
                            input: ValueSchema::string(),
                            output: ValueSchema::string(),
                        },
                    )]),
                },
                Echo,
            )
            .unwrap();
        runtime
            .borrow_mut()
            .capabilities
            .activate(&BTreeMap::from([(capability, VersionReq::STAR)]))
            .unwrap();
        let path = ComponentInstancePath::root("App", "root");
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            path.clone(),
            None,
            BTreeMap::new(),
            &state_schema(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();

        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let deliveries = runtime.borrow_mut().tasks.drain(lifecycle.generation());
            if !deliveries.is_empty() {
                for delivery in deliveries {
                    let _ = lifecycle.invoke_async_delivery(&engine, delivery).unwrap();
                }
                lifecycle.render(&mut engine).unwrap();
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(
            runtime.borrow().component_state.get(&path, "phase"),
            Some(&UiValue::String("loaded".to_owned()))
        );
    }

    #[test]
    fn async_image_decode_completes_through_lifecycle_delivery() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1, 1)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn init(ctx) {
                        ctx.start_image_decode(
                            asset("app/pixel"), Fn("loaded"), Fn("failed")
                        );
                    }
                    fn loaded(ctx, handle) { ctx.set_state("image", handle); }
                    fn failed(ctx, error) { () }
                    fn view(ctx) { image(ctx.get_state("image")) }
                "#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        runtime
            .borrow()
            .assets
            .register(
                "app",
                InMemoryAssetProvider::new(BTreeMap::from([(
                    "pixel".to_owned(),
                    AssetData {
                        mime_type: "image/png".to_owned(),
                        bytes: png.into_inner(),
                    },
                )])),
            )
            .unwrap();
        let path = ComponentInstancePath::root("App", "root");
        let schema = ComponentStateSchema::new(BTreeMap::from([(
            "image".to_owned(),
            StateField::new(
                ValueSchema::Handle {
                    kind: "image".to_owned(),
                },
                UiValue::Handle(OpaqueHandle::new("image", 0)),
            ),
        )]))
        .unwrap();
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            path.clone(),
            None,
            BTreeMap::new(),
            &schema,
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();

        let assets = runtime.borrow().assets.clone();
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let deliveries = assets.drain_image_decodes(lifecycle.generation()).unwrap();
            if !deliveries.is_empty() {
                for delivery in deliveries {
                    let _ = lifecycle.invoke_async_delivery(&engine, delivery).unwrap();
                }
                lifecycle.render(&mut engine).unwrap();
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(matches!(
            runtime.borrow().component_state.get(&path, "image"),
            Some(UiValue::Handle(handle)) if handle.id() != 0
        ));
    }

    #[test]
    fn subscription_capability_streams_until_close() {
        struct Stream;
        impl SubscriptionCapabilityHandler for Stream {
            fn subscribe(&mut self, _: &str, _: UiValue) -> Result<SubscriptionWork, String> {
                Ok(SubscriptionWork::new(|emitter| {
                    emitter.emit(UiValue::String("first".to_owned())).unwrap();
                    emitter.emit(UiValue::String("second".to_owned())).unwrap();
                }))
            }
        }

        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn init(ctx) {
                        ctx.start_subscription(
                            "app.stream", "watch", (),
                            Fn("received"), Fn("failed"), 0
                        );
                    }
                    fn received(ctx, value) { ctx.set_state("phase", value); }
                    fn failed(ctx, error) { ctx.set_state("phase", "failed"); }
                    fn view(ctx) { text(ctx.get_state("phase")) }
                "#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let capability = CapabilityId::parse("app.stream").unwrap();
        runtime
            .borrow_mut()
            .capabilities
            .register_subscription(
                CapabilityDescriptor {
                    id: capability.clone(),
                    version: Version::new(1, 0, 0),
                    methods: BTreeMap::from([(
                        "watch".to_owned(),
                        CapabilityMethod {
                            input: ValueSchema::Null,
                            output: ValueSchema::string(),
                        },
                    )]),
                },
                Stream,
            )
            .unwrap();
        runtime
            .borrow_mut()
            .capabilities
            .activate(&BTreeMap::from([(capability, VersionReq::STAR)]))
            .unwrap();
        let path = ComponentInstancePath::root("App", "root");
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            path.clone(),
            None,
            BTreeMap::new(),
            &state_schema(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();

        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let deliveries = runtime
                .borrow_mut()
                .subscriptions
                .drain(lifecycle.generation());
            for delivery in deliveries {
                let _ = lifecycle.invoke_async_delivery(&engine, delivery).unwrap();
            }
            if runtime.borrow().subscriptions.active_count() == 0 {
                lifecycle.render(&mut engine).unwrap();
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(
            runtime.borrow().component_state.get(&path, "phase"),
            Some(&UiValue::String("second".to_owned()))
        );
    }
}
