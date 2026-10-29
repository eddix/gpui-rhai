use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::state::topmost_paths;
use rhai::Dynamic;
use thiserror::Error;

use crate::{
    AsyncDelivery, AsyncScope, CompiledUi, ComponentInstancePath, ComponentStateSchema,
    EventSchema, ExecutionPhase, MotionError, RuntimeEngine, RuntimeError, ScriptCallback,
    ScriptGeneration, StateError, UiContext, UiNode, UiRuntimeState, UiValue,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleState {
    Created,
    Initialized,
    Running,
    Suspended,
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
    root: Option<Rc<UiNode>>,
    retained: crate::RetainedUiTree,
    semantics: crate::CommittedSemanticFrame,
    suspended_at: Option<std::time::Instant>,
    /// Advances whenever a new retained tree is committed (or restored).
    revision: u64,
    commit_statistics: CommitStatistics,
}

#[derive(Clone)]
pub(crate) struct ScriptLifecycleCheckpoint {
    compiled: CompiledUi,
    state: LifecycleState,
    root: Option<Rc<UiNode>>,
    retained: crate::RetainedUiTree,
    semantics: crate::CommittedSemanticFrame,
    suspended_at: Option<std::time::Instant>,
}

/// Commits and the time spent in each of their phases, summed since the last
/// [`ScriptLifecycle::take_commit_statistics`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CommitStatistics {
    pub commits: u64,
    /// Phases in the order they first ran.
    pub phases: Vec<(&'static str, std::time::Duration)>,
}

impl CommitStatistics {
    fn add(&mut self, phase: &'static str, elapsed: std::time::Duration) {
        if let Some((_, total)) = self.phases.iter_mut().find(|(name, _)| *name == phase) {
            *total += elapsed;
        } else {
            self.phases.push((phase, elapsed));
        }
    }

    pub(crate) fn merge(&mut self, other: Self) {
        self.commits += other.commits;
        for (phase, elapsed) in other.phases {
            self.add(phase, elapsed);
        }
    }
}

/// Laps one commit's phases; folded into the lifecycle's statistics when the
/// commit ends, whether it succeeded or not.
struct PhaseClock {
    last: std::time::Instant,
    laps: CommitStatistics,
}

impl PhaseClock {
    fn start() -> Self {
        Self {
            last: std::time::Instant::now(),
            laps: CommitStatistics::default(),
        }
    }

    fn lap(&mut self, phase: &'static str) {
        let now = std::time::Instant::now();
        self.laps.add(phase, now - self.last);
        self.last = now;
    }
}

#[derive(Default)]
struct RetainedDeclarations {
    effects: BTreeMap<crate::EffectId, crate::EffectDescriptor>,
    timers: BTreeMap<crate::TimerId, crate::TimerDescriptor>,
    signals: BTreeMap<crate::SignalId, crate::signal::SignalDescriptor>,
    element_refs: BTreeMap<crate::ElementRefId, crate::NodeId>,
}

impl ScriptLifecycle {
    pub(crate) fn execution_checkpoint(&self) -> ScriptLifecycleCheckpoint {
        ScriptLifecycleCheckpoint {
            compiled: self.compiled.clone(),
            state: self.state,
            root: self.root.clone(),
            retained: self.retained.clone(),
            semantics: self.semantics.clone(),
            suspended_at: self.suspended_at,
        }
    }

    pub(crate) fn restore_execution_checkpoint(&mut self, checkpoint: ScriptLifecycleCheckpoint) {
        self.compiled = checkpoint.compiled;
        self.state = checkpoint.state;
        self.root = checkpoint.root;
        self.retained = checkpoint.retained;
        self.semantics = checkpoint.semantics;
        self.revision = self.revision.wrapping_add(1);
        self.suspended_at = checkpoint.suspended_at;
    }

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
            runtime_state.ensure_component_incarnation(&root_path);
            runtime_state
                .effect_cascades
                .register_generation(&root_path, compiled.generation());
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
            semantics: crate::CommittedSemanticFrame::default(),
            suspended_at: None,
            revision: 0,
            commit_statistics: CommitStatistics::default(),
        })
    }

    /// Commits since the last call and the time each phase took.
    pub(crate) fn take_commit_statistics(&mut self) -> CommitStatistics {
        std::mem::take(&mut self.commit_statistics)
    }

    /// The revision of the committed retained tree; it advances on every
    /// commit, so observers can skip work when nothing new was rendered.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub const fn state(&self) -> LifecycleState {
        self.state
    }

    #[must_use]
    pub fn root(&self) -> Option<&UiNode> {
        self.root.as_deref()
    }

    #[must_use]
    pub const fn retained(&self) -> &crate::RetainedUiTree {
        &self.retained
    }

    #[must_use]
    pub const fn semantics(&self) -> &crate::CommittedSemanticFrame {
        &self.semantics
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
        let view = view.into();
        if let Ok(mut runtime) = self.runtime.try_borrow_mut() {
            runtime.ensure_presentation(&view);
        }
        self.view = Some(view);
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
        let snapshot = self.begin_runtime_transaction()?;
        let context = self.context(ExecutionPhase::Init);
        match engine.call_optional_lifecycle(&self.compiled, "init", context) {
            Ok(_) => {
                self.state = LifecycleState::Initialized;
                self.commit_runtime_transaction()
            }
            Err(error) => {
                self.runtime
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .restore(snapshot)?;
                Err(error.into())
            }
        }
    }

    /// Validate the initial `view(ctx)` without executing `init(ctx)`.
    ///
    /// This entry point is for static tooling that has no Host capability
    /// implementations. It validates the rendered tree but deliberately does
    /// not claim that Host-dependent initialization succeeded.
    ///
    /// # Errors
    ///
    /// Returns the same render, reconciliation, and resource errors as
    /// [`Self::render`].
    pub fn validate_initial_view_without_init(
        &mut self,
        engine: &mut RuntimeEngine,
    ) -> Result<&UiNode, LifecycleError> {
        self.require_state(LifecycleState::Created)?;
        self.state = LifecycleState::Initialized;
        self.render(engine)
    }

    /// Evaluate required `view(ctx)` and activate its generation on success.
    ///
    /// # Errors
    ///
    /// Returns [`LifecycleError::InvalidTransition`] before initialization or
    /// after disposal, or a script evaluation error.
    pub fn render(&mut self, engine: &mut RuntimeEngine) -> Result<&UiNode, LifecycleError> {
        self.render_impl(engine, None)
    }

    fn render_impl(
        &mut self,
        engine: &mut RuntimeEngine,
        reuse_dirty: Option<&BTreeSet<ComponentInstancePath>>,
    ) -> Result<&UiNode, LifecycleError> {
        if !matches!(
            self.state,
            LifecycleState::Initialized | LifecycleState::Running
        ) {
            return Err(LifecycleError::InvalidTransition {
                from: self.state,
                operation: "render",
            });
        }
        let mut clock = PhaseClock::start();
        let runtime_snapshot = self.begin_runtime_transaction()?;
        let engine_checkpoint = engine.execution_checkpoint();
        let previous_root = reuse_dirty.and_then(|_| self.root.clone());
        let result = (|| {
            let context = self.context(ExecutionPhase::Render);
            let root = match (previous_root.as_ref(), reuse_dirty) {
                (Some(previous_root), Some(dirty)) => engine.render_with_context_staged_reusing(
                    &self.compiled,
                    context,
                    Rc::clone(previous_root),
                    dirty,
                )?,
                _ => engine.render_with_context_staged(&self.compiled, context)?,
            };
            clock.lap("render");
            let mut retained = self.retained.clone();
            clock.lap("clone");
            let semantics =
                self.commit_phases(engine, &runtime_snapshot, &mut retained, &root, &mut clock)?;
            Ok((root, retained, semantics))
        })();
        self.commit_statistics.merge(clock.laps);
        match result {
            Ok((root, retained, semantics)) => {
                self.trace_reconcile("full", retained.last_report());
                self.retained = retained;
                self.semantics = semantics;
                self.revision = self.revision.wrapping_add(1);
                self.state = LifecycleState::Running;
                self.root = Some(Rc::new(root));
                self.commit_runtime_transaction()?;
                self.root.as_deref().ok_or(LifecycleError::MissingRoot)
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
        let runtime_snapshot = self.begin_runtime_transaction()?;
        let engine_checkpoint = engine.execution_checkpoint();
        let dirty = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?
            .take_window_dirty_components(&self.root_path);
        debug_assert!(!dirty.is_empty());
        let topmost = topmost_paths(&dirty);
        if topmost.iter().any(|path| path == &self.root_path)
            || topmost.iter().any(|path| {
                engine
                    .component_invocations()
                    .all(|recipe| recipe.path() != path)
            })
        {
            return self.render_dirty_full(engine, &dirty, runtime_snapshot, engine_checkpoint);
        }

        let mut root = self
            .root
            .as_deref()
            .cloned()
            .ok_or(LifecycleError::MissingRoot)?;
        let active = engine
            .component_invocations()
            .map(|recipe| recipe.path().clone())
            .collect::<BTreeSet<_>>();
        let Some(topmost) = replaceable_component_paths(&root, &self.root_path, topmost, &active)
        else {
            return self.render_dirty_full(engine, &dirty, runtime_snapshot, engine_checkpoint);
        };
        engine.begin_execution_session();
        let mut clock = PhaseClock::start();
        let mut retained = self.retained.clone();
        clock.lap("clone");
        let result = (|| {
            for path in topmost {
                let subtree = engine.rerender_component(&path, &root, &dirty)?;
                if !root.replace_component_subtree(&path, subtree) {
                    return Err(LifecycleError::MissingComponentSubtree(path));
                }
            }
            clock.lap("render");
            self.commit_phases(engine, &runtime_snapshot, &mut retained, &root, &mut clock)
        })();
        self.commit_statistics.merge(clock.laps);
        match result {
            Ok(semantics) => {
                self.trace_reconcile("incremental", retained.last_report());
                self.root = Some(Rc::new(root));
                self.retained = retained;
                self.semantics = semantics;
                self.revision = self.revision.wrapping_add(1);
                self.state = LifecycleState::Running;
                self.commit_runtime_transaction()?;
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

    fn render_dirty_full(
        &mut self,
        engine: &mut RuntimeEngine,
        dirty: &BTreeSet<ComponentInstancePath>,
        runtime_snapshot: crate::UiStateSnapshot,
        engine_checkpoint: crate::engine::RuntimeEngineCheckpoint,
    ) -> Result<bool, LifecycleError> {
        match self.render_impl(engine, Some(dirty)) {
            Ok(_) => {
                self.commit_runtime_transaction()?;
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
    #[allow(clippy::too_many_lines)]
    pub fn realize_virtual_requests(
        &mut self,
        engine: &mut RuntimeEngine,
    ) -> Result<bool, LifecycleError> {
        if !self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .virtual_requests
            .has_scope(&self.root_path)
        {
            return Ok(false);
        }
        let runtime_snapshot = self.begin_runtime_transaction()?;
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
        engine.begin_execution_session();
        let mut root = self
            .root
            .as_deref()
            .cloned()
            .ok_or(LifecycleError::MissingRoot)?;
        let mut clock = PhaseClock::start();
        let mut retained = self.retained.clone();
        clock.lap("clone");
        let result = (|| {
            let mut changed = false;
            for (id, indices) in selected {
                // Parent targets are processed first. A child that existed at
                // batch start may legitimately be removed by a parent prune.
                if root.virtual_collection_spec(&id).is_none()
                    && self
                        .root
                        .as_deref()
                        .is_some_and(|root| root.virtual_collection_spec(&id).is_some())
                {
                    continue;
                }
                let mut items = root
                    .virtual_collection_items(&id)
                    .cloned()
                    .ok_or_else(|| LifecycleError::MissingVirtualCollection(id.clone()))?;
                let previous_indices = items.keys().copied().collect::<BTreeSet<_>>();
                if previous_indices == indices {
                    continue;
                }
                let inherited_motion_group = root.virtual_collection_spec(&id).and_then(|spec| {
                    spec.inherited_motion_group
                        .clone()
                        .map(|group| (group, spec.inherited_motion_scope.clone()))
                });
                clock.lap("lookup");
                items = engine.realize_virtual_collection(&id, &indices, &items)?;
                clock.lap("realize");
                if let Some((group, scope)) = inherited_motion_group {
                    for node in items.values_mut() {
                        crate::node::apply_motion_group(node, &group, scope.as_deref());
                    }
                }
                if !root.replace_virtual_collection_items(&id, items.clone()) {
                    return Err(LifecycleError::MissingVirtualCollection(id));
                }
                engine.update_virtual_collection_snapshot(&id, &items);
                changed = true;
            }
            clock.lap("replace");
            if !changed {
                return Ok((false, self.semantics.clone()));
            }
            let semantics =
                self.commit_phases(engine, &runtime_snapshot, &mut retained, &root, &mut clock)?;
            Ok((true, semantics))
        })();
        self.commit_statistics.merge(clock.laps);
        match result {
            Ok((changed, semantics)) => {
                if changed {
                    self.trace_reconcile("virtual", retained.last_report());
                    self.root = Some(Rc::new(root));
                    self.retained = retained;
                    self.semantics = semantics;
                    self.revision = self.revision.wrapping_add(1);
                }
                self.commit_runtime_transaction()?;
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

    /// The phases every commit runs over its candidate root, in order:
    /// reconcile `retained` (the working copy that becomes the committed tree
    /// on success) against `root`, then budgets, motion, exit ghosts,
    /// effects, geometry, signal bindings and the semantic frame.
    fn commit_phases(
        &self,
        engine: &mut RuntimeEngine,
        runtime_snapshot: &crate::UiStateSnapshot,
        retained: &mut crate::RetainedUiTree,
        root: &UiNode,
        clock: &mut PhaseClock,
    ) -> Result<crate::CommittedSemanticFrame, LifecycleError> {
        clock.laps.commits += 1;
        let motion_reconcile_report = retained.reconcile(root.clone())?;
        clock.lap("reconcile");
        self.validate_resource_budgets(engine, retained, root)?;
        clock.lap("budgets");
        self.reconcile_motion(root, retained, self.compiled.generation())?;
        clock.lap("motion");
        self.reconcile_exit_ghosts(&motion_reconcile_report)?;
        clock.lap("ghosts");
        self.reconcile_effects(
            engine,
            &self.compiled,
            runtime_snapshot.component_state().clone(),
            retained,
            root,
        )?;
        clock.lap("effects");
        self.retain_geometry_nodes(retained)?;
        clock.lap("geometry");
        self.validate_signal_bindings(root)?;
        clock.lap("signals");
        let semantics = crate::CommittedSemanticFrame::from_retained(retained)?;
        clock.lap("semantics");
        Ok(semantics)
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
            LifecycleState::Initialized | LifecycleState::Running | LifecycleState::Suspended
        ) {
            return Err(LifecycleError::InvalidTransition {
                from: self.state,
                operation: "dispose",
            });
        }
        let snapshot = self.begin_runtime_transaction()?;
        self.restart_effect_cascade()?;
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
                self.runtime
                    .try_borrow()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .virtual_requests
                    .release_scope(&self.root_path);
                self.commit_runtime_transaction()?;
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

    /// Quiesce one retained view without discarding its tree, state, native
    /// entities, measurements, or current script generation.
    ///
    /// # Errors
    ///
    /// Returns lifecycle, callback, cleanup, or rollback errors. Repeated
    /// suspension is an idempotent no-op.
    pub fn suspend(&mut self, engine: &mut RuntimeEngine) -> Result<bool, LifecycleError> {
        if self.state == LifecycleState::Suspended {
            return Ok(false);
        }
        if self.state != LifecycleState::Running {
            return Err(LifecycleError::InvalidTransition {
                from: self.state,
                operation: "suspend",
            });
        }
        let snapshot = self.begin_runtime_transaction()?;
        self.restart_effect_cascade()?;
        let checkpoint = engine.execution_checkpoint();
        let result = (|| {
            engine.call_optional_lifecycle(
                &self.compiled,
                "suspend",
                self.context(ExecutionPhase::Suspend),
            )?;
            self.stop_active_effects(engine)?;
            let mut runtime = self
                .runtime
                .try_borrow_mut()
                .map_err(|_| LifecycleError::Borrowed)?;
            let now = runtime.clock.now();
            runtime.timers.pause_component_scope(&self.root_path, now);
            runtime
                .motions
                .suspend_node_scope(&self.motion_root_path(), now);
            runtime.motion_values = runtime.motions.snapshot(now);
            runtime
                .pointer_capture_for(self.presentation_scope())
                .clear();
            Ok(now)
        })();
        match result {
            Ok(now) => {
                self.suspended_at = Some(now);
                self.state = LifecycleState::Suspended;
                self.commit_runtime_transaction()?;
                Ok(true)
            }
            Err(error) => {
                self.runtime
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .restore(snapshot)?;
                engine.restore_execution_checkpoint(checkpoint);
                Err(error)
            }
        }
    }

    /// Resume a retained view transactionally against current Host data.
    ///
    /// The optional `resume(ctx, elapsed_ms)` hook runs before one full render;
    /// effect declarations then restart with fresh activation identities.
    ///
    /// # Errors
    ///
    /// Returns lifecycle, callback, render, reconciliation, or rollback errors.
    pub fn resume(&mut self, engine: &mut RuntimeEngine) -> Result<bool, LifecycleError> {
        if self.state == LifecycleState::Running {
            return Ok(false);
        }
        if self.state != LifecycleState::Suspended {
            return Err(LifecycleError::InvalidTransition {
                from: self.state,
                operation: "resume",
            });
        }
        let snapshot = self.begin_runtime_transaction()?;
        self.restart_effect_cascade()?;
        let checkpoint = engine.execution_checkpoint();
        let suspended_at = self
            .suspended_at
            .ok_or(LifecycleError::MissingSuspendTime)?;
        let now = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .clock
            .now();
        let elapsed = now.saturating_duration_since(suspended_at);
        let result = (|| {
            {
                self.resume_runtime_mechanisms(now, elapsed)?;
            }
            engine.call_optional_lifecycle_with_value(
                &self.compiled,
                "resume",
                self.context(ExecutionPhase::Resume),
                UiValue::Integer(i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)),
            )?;
            self.state = LifecycleState::Initialized;
            self.render_impl(engine, None)?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.suspended_at = None;
                self.state = LifecycleState::Running;
                self.commit_runtime_transaction()?;
                Ok(true)
            }
            Err(error) => {
                self.state = LifecycleState::Suspended;
                self.runtime
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .restore(snapshot)?;
                engine.restore_execution_checkpoint(checkpoint);
                Err(error)
            }
        }
    }

    fn stop_active_effects(&self, engine: &RuntimeEngine) -> Result<(), LifecycleError> {
        let plan = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .effects
            .plan(&self.root_path, BTreeMap::new());
        if plan.transition_count() > crate::effect::EFFECT_TRANSITION_CALLBACK_BUDGET {
            return Err(LifecycleError::EffectBudget(plan.transition_count()));
        }
        for (descriptor, scope) in plan.cleanup_descriptors() {
            self.invoke_effect_callback(
                engine,
                &self.compiled,
                descriptor.cleanup(),
                descriptor.dependencies().clone(),
                scope,
            )?;
        }
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?;
        for (_, scope) in plan.cleanup_descriptors() {
            runtime.cancel_async_scope(&scope)?;
        }
        runtime.effects.commit(plan);
        Ok(())
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
        self.invoke_callback_with_event_target(engine, callback, payload, None)
    }

    pub(crate) fn invoke_callback_with_event_target(
        &self,
        engine: &RuntimeEngine,
        callback: &ScriptCallback,
        payload: UiValue,
        event_target: Option<crate::GeometryBounds>,
    ) -> Result<Dynamic, LifecycleError> {
        self.validate_callback_owner(callback)?;
        // Effect-generated semantic events retain their origin when delivered
        // later. They are part of the same cascade, not new user operations.
        {
            let mut runtime = self
                .runtime
                .try_borrow_mut()
                .map_err(|_| LifecycleError::Borrowed)?;
            runtime.prune_pending_dispatch();
            if runtime.invocation_origin() == &crate::InvocationOrigin::Effect {
                let root = runtime
                    .dispatch_owner
                    .as_ref()
                    .and_then(crate::SemanticDispatchOwner::cascade_root)
                    .cloned()
                    .unwrap_or_else(|| self.root_path.clone());
                if runtime.effect_cascades.blocked(&root) {
                    return Ok(Dynamic::UNIT);
                }
                if runtime.dispatch_owner.is_none() {
                    runtime.effect_cascades.retain_unowned_dispatch(&root);
                }
                // Pure semantic recursion may never restart an effect. Every
                // effect-origin callback shares the graph's transition budget.
                runtime
                    .effect_cascades
                    .charge(&root, 1)
                    .map_err(LifecycleError::EffectBudget)?;
            } else {
                runtime.effect_cascades.restart(&self.root_path);
            }
        }
        let root_context = self.context(ExecutionPhase::Event);
        let context = callback
            .component()
            .map_or(root_context.clone(), |component| {
                root_context.for_component(component.clone(), callback.events().clone())
            })
            .with_native_context(callback.native_context().cloned())
            .with_event_target(event_target);
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
        let snapshot = self.begin_runtime_transaction()?;
        match self.invoke_callback(engine, callback, payload) {
            Ok(value) => {
                self.commit_runtime_transaction()?;
                Ok(value)
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
        if !self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .effects
            .contains_scope(&delivery.scope)
        {
            return Ok(Dynamic::UNIT);
        }
        self.validate_callback_owner(&delivery.callback)?;
        self.restart_effect_cascade()?;
        let component = delivery
            .callback
            .component()
            .cloned()
            .or_else(|| delivery.scope.component().cloned())
            .unwrap_or_else(|| self.root_path.clone());
        let events = delivery.callback.events().clone();
        let scope = delivery.scope.clone();
        let origin = delivery.origin.clone();
        let context = UiContext::new(
            Rc::clone(&self.runtime),
            component,
            self.window.clone(),
            ExecutionPhase::Event,
            events,
        )
        .with_optional_view_id(self.view.clone())
        .with_generation(self.compiled.generation())
        .with_native_context(delivery.callback.native_context().cloned())
        .with_async_scope(scope);
        self.with_dispatch_owner(None, || {
            self.with_origin(origin, || {
                Ok(engine.invoke_callback(
                    &self.compiled,
                    &delivery.callback,
                    (context, delivery.payload.into_dynamic()),
                )?)
            })
        })
    }

    /// Run `invoke` with `origin` as the running invocation's origin, then
    /// restore the previous one.
    pub(crate) fn with_origin<T>(
        &self,
        origin: crate::InvocationOrigin,
        invoke: impl FnOnce() -> Result<T, LifecycleError>,
    ) -> Result<T, LifecycleError> {
        let previous = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?
            .replace_origin(origin);
        let result = invoke();
        self.runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?
            .replace_origin(previous);
        result
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
        let snapshot = self.begin_runtime_transaction()?;
        match self.invoke_async_delivery(engine, delivery) {
            Ok(value) => {
                self.commit_runtime_transaction()?;
                Ok(value)
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
        if !self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .dispatch_owner_active(pending.owner.as_ref())
        {
            return Ok(None);
        }
        let callback = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .component_event_handler(&pending.target, &pending.event.name);
        callback
            .map(|callback| {
                self.with_dispatch_owner(pending.owner, || {
                    self.with_origin(pending.origin, || {
                        self.invoke_callback_transactional(engine, &callback, pending.event.payload)
                    })
                })
            })
            .transpose()
    }

    /// Deliver a queued semantic action with its original origin and opaque
    /// provenance. Stale generation, mount, activation or rejected derivations
    /// are ignored, including clones retained by an external Host.
    ///
    /// # Errors
    /// Returns callback, effect-budget or transaction errors.
    pub fn invoke_action_transactional(
        &self,
        engine: &RuntimeEngine,
        pending: crate::ActionInvocation,
    ) -> Result<Dynamic, LifecycleError> {
        if pending.callback.generation() != self.generation()
            || !self
                .runtime
                .try_borrow()
                .map_err(|_| LifecycleError::Borrowed)?
                .dispatch_owner_active(pending.owner.as_ref())
        {
            return Ok(Dynamic::UNIT);
        }
        self.with_dispatch_owner(pending.owner, || {
            self.with_origin(pending.origin, || {
                self.invoke_callback_transactional(engine, &pending.callback, pending.payload)
            })
        })
    }

    fn with_dispatch_owner<T>(
        &self,
        owner: Option<crate::SemanticDispatchOwner>,
        invoke: impl FnOnce() -> Result<T, LifecycleError>,
    ) -> Result<T, LifecycleError> {
        let previous = {
            let mut runtime = self
                .runtime
                .try_borrow_mut()
                .map_err(|_| LifecycleError::Borrowed)?;
            std::mem::replace(&mut runtime.dispatch_owner, owner)
        };
        let result = invoke();
        self.runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?
            .dispatch_owner = previous;
        result
    }

    pub(crate) fn reject_semantic_dispatch_budget(
        &self,
        origin: &crate::InvocationOrigin,
        owner: Option<&crate::SemanticDispatchOwner>,
    ) -> Result<(), LifecycleError> {
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?;
        if origin == &crate::InvocationOrigin::Effect && runtime.dispatch_owner_active(owner) {
            let root = owner
                .and_then(crate::SemanticDispatchOwner::cascade_root)
                .unwrap_or(&self.root_path);
            runtime.effect_cascades.reject(root);
        }
        Ok(())
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
        if self.state != LifecycleState::Running {
            return Err(LifecycleError::InvalidTransition {
                from: self.state,
                operation: "reload",
            });
        }
        let snapshot = self.begin_runtime_transaction()?;
        self.restart_effect_cascade()?;
        let engine_checkpoint = engine.execution_checkpoint();
        let result: Result<
            (UiNode, crate::RetainedUiTree, crate::CommittedSemanticFrame),
            LifecycleError,
        > = (|| {
            self.runtime
                .try_borrow_mut()
                .map_err(|_| LifecycleError::Borrowed)?
                .component_state
                .mount_instance(self.root_path.clone(), state_schema)?;
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
            let motion_reconcile_report = retained.reconcile(root.clone())?;
            self.validate_resource_budgets(engine, &retained, &root)?;
            self.reconcile_motion(&root, &retained, candidate.generation())?;
            self.reconcile_exit_ghosts(&motion_reconcile_report)?;
            self.reconcile_effects(
                engine,
                &candidate,
                snapshot.component_state().clone(),
                &retained,
                &root,
            )?;
            self.retain_geometry_nodes(&retained)?;
            self.validate_signal_bindings(&root)?;
            let semantics = crate::CommittedSemanticFrame::from_retained(&retained)?;
            Ok((root, retained, semantics))
        })();
        match result {
            Ok((root, retained, semantics)) => {
                self.trace_reconcile("reload", retained.last_report());
                self.compiled = candidate;
                self.retained = retained;
                self.semantics = semantics;
                self.revision = self.revision.wrapping_add(1);
                self.state = LifecycleState::Running;
                self.root = Some(Rc::new(root));
                self.commit_runtime_transaction()?;
                self.root.as_deref().ok_or(LifecycleError::MissingRoot)
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

    /// Resume a suspended retained view into a newer compiled generation.
    ///
    /// Candidate initialization, the optional resume hook, reconciliation,
    /// and effect activation form one rollback-safe transaction. Async
    /// deliveries from the replaced generation must be discarded by the host
    /// before entering this transaction.
    ///
    /// # Errors
    ///
    /// Returns lifecycle, callback, render, reconciliation, or rollback errors.
    pub fn resume_reload(
        &mut self,
        engine: &mut RuntimeEngine,
        candidate: CompiledUi,
        state_schema: &ComponentStateSchema,
    ) -> Result<&UiNode, LifecycleError> {
        if self.state != LifecycleState::Suspended {
            return Err(LifecycleError::InvalidTransition {
                from: self.state,
                operation: "resume_reload",
            });
        }
        let snapshot = self.begin_runtime_transaction()?;
        self.restart_effect_cascade()?;
        let engine_checkpoint = engine.execution_checkpoint();
        let suspended_at = self
            .suspended_at
            .ok_or(LifecycleError::MissingSuspendTime)?;
        let now = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .clock
            .now();
        let elapsed = now.saturating_duration_since(suspended_at);
        let result: Result<
            (UiNode, crate::RetainedUiTree, crate::CommittedSemanticFrame),
            LifecycleError,
        > = (|| {
            self.runtime
                .try_borrow_mut()
                .map_err(|_| LifecycleError::Borrowed)?
                .component_state
                .mount_instance(self.root_path.clone(), state_schema)?;
            engine.call_optional_lifecycle(
                &candidate,
                "init",
                self.context_for(ExecutionPhase::Init, candidate.generation()),
            )?;
            engine.call_optional_lifecycle_with_value(
                &candidate,
                "resume",
                self.context_for(ExecutionPhase::Resume, candidate.generation()),
                UiValue::Integer(i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)),
            )?;
            {
                let mut runtime = self
                    .runtime
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Borrowed)?;
                runtime.motion_values = runtime.motions.snapshot(now);
            }
            let root = engine.render_with_context_staged(
                &candidate,
                self.context_for(ExecutionPhase::Render, candidate.generation()),
            )?;
            let mut retained = self.retained.clone();
            let motion_reconcile_report = retained.reconcile(root.clone())?;
            self.validate_resource_budgets(engine, &retained, &root)?;
            self.reconcile_motion(&root, &retained, candidate.generation())?;
            self.reconcile_exit_ghosts(&motion_reconcile_report)?;
            self.reconcile_effects(
                engine,
                &candidate,
                snapshot.component_state().clone(),
                &retained,
                &root,
            )?;
            self.resume_runtime_mechanisms(now, elapsed)?;
            self.retain_geometry_nodes(&retained)?;
            self.validate_signal_bindings(&root)?;
            let semantics = crate::CommittedSemanticFrame::from_retained(&retained)?;
            Ok((root, retained, semantics))
        })();
        match result {
            Ok((root, retained, semantics)) => {
                self.trace_reconcile("resume_reload", retained.last_report());
                self.compiled = candidate;
                self.retained = retained;
                self.semantics = semantics;
                self.revision = self.revision.wrapping_add(1);
                self.root = Some(Rc::new(root));
                self.suspended_at = None;
                self.state = LifecycleState::Running;
                self.commit_runtime_transaction()?;
                self.root.as_deref().ok_or(LifecycleError::MissingRoot)
            }
            Err(error) => {
                self.runtime
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .restore(snapshot)?;
                engine.restore_execution_checkpoint(engine_checkpoint);
                self.state = LifecycleState::Suspended;
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
        let runtime_snapshot = self.begin_runtime_transaction()?;
        self.restart_effect_cascade()?;
        let engine_checkpoint = engine.execution_checkpoint();
        let lifecycle_checkpoint = self.execution_checkpoint();
        let result = (|| {
            self.initialize(engine)?;
            self.render(engine)?;
            Ok::<_, LifecycleError>(())
        })();
        match result {
            Ok(()) => {
                self.commit_runtime_transaction()?;
                self.root.as_deref().ok_or(LifecycleError::MissingRoot)
            }
            Err(error) => {
                self.runtime
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Borrowed)?
                    .restore(runtime_snapshot)?;
                engine.restore_execution_checkpoint(engine_checkpoint);
                self.restore_execution_checkpoint(lifecycle_checkpoint);
                Err(error)
            }
        }
    }

    fn context(&self, phase: ExecutionPhase) -> UiContext {
        self.context_for(phase, self.compiled.generation())
    }

    fn reconcile_motion(
        &self,
        root: &UiNode,
        retained: &crate::RetainedUiTree,
        generation: ScriptGeneration,
    ) -> Result<(), LifecycleError> {
        let domain = self.motion_root_path();
        let inert = retained
            .root_id()
            .and_then(|id| retained.node(id))
            .is_some_and(|node| node.summary().motion == 0)
            && self
                .runtime
                .try_borrow()
                .map_err(|_| LifecycleError::Borrowed)?
                .motions
                .live_scope_idle(&domain);
        if inert {
            return self.reconcile_inert_motion(root, retained, generation, &domain);
        }
        let identities = crate::motion::retained_motion_identities(root, retained, &domain)?;
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?;
        let now = runtime.clock.now();
        let geometry_slots = crate::motion::node_motion_resource_usage(root).geometry_slots;
        crate::RuntimeBudgets::check(
            "active_motions",
            geometry_slots,
            runtime.budgets.active_motions,
        )?;
        let active_limit = runtime.budgets.active_motions;
        runtime.motions.set_active_limit(active_limit);
        let incarnations = runtime.component_incarnations_snapshot();
        let root_incarnation = runtime
            .component_incarnation(&self.root_path)
            .unwrap_or_else(crate::ComponentIncarnation::unscoped);
        let shared_layout_ids = crate::motion::shared_layout_ids(root)?;
        runtime
            .geometry_for(self.presentation_scope())
            .retain_shared_layout_ids(&shared_layout_ids);
        runtime
            .motions
            .set_active_reservation(&domain, geometry_slots);
        let values = crate::motion::reconcile_node_motion_scoped_owned(
            root,
            &mut runtime.motions,
            now,
            &domain,
            crate::motion::MotionReconcileContext {
                domain: &domain,
                root_component: &self.root_path,
                root_incarnation,
                generation,
                incarnations: &incarnations,
                identities: Some(&identities),
            },
        )?;
        let usage = runtime.motions.resource_usage();
        let active = usage.active.saturating_add(usage.geometry_slots);
        crate::RuntimeBudgets::check("active_motions", active, runtime.budgets.active_motions)?;
        runtime.reconcile_page_visibility(retained, &self.root_path, &domain);
        runtime.motion_values = if values.is_empty() {
            values
        } else {
            runtime.motions.snapshot(now)
        };
        Ok(())
    }

    /// [`Self::reconcile_motion`] for a tree that declares no motion
    /// ([`crate::motion::node_takes_motion`]) in a domain that holds none:
    /// the declarations, identities and scene it would walk the tree for are
    /// empty, and the scopes it would retain hold nothing to cancel, so what
    /// remains are its budget checks, reservations and resamples.
    fn reconcile_inert_motion(
        &self,
        root: &UiNode,
        retained: &crate::RetainedUiTree,
        generation: ScriptGeneration,
        domain: &str,
    ) -> Result<(), LifecycleError> {
        #[cfg(debug_assertions)]
        let reference = self.reference_motion_outcome(root, retained, generation, domain);
        #[cfg(not(debug_assertions))]
        let _ = (root, generation);
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?;
        let now = runtime.clock.now();
        let active_limit = runtime.budgets.active_motions;
        runtime.motions.set_active_limit(active_limit);
        runtime
            .geometry_for(self.presentation_scope())
            .retain_shared_layout_ids(&BTreeSet::new());
        runtime.motions.set_active_reservation(domain, 0);
        let usage = runtime.motions.resource_usage();
        let active = usage.active.saturating_add(usage.geometry_slots);
        let result = if active > active_limit {
            Err(crate::motion::MotionError::ActiveBudget {
                actual: active,
                limit: active_limit,
            }
            .into())
        } else {
            runtime.reconcile_page_visibility(retained, &self.root_path, domain);
            runtime.motion_values = runtime.motions.snapshot(now);
            Ok(())
        };
        #[cfg(debug_assertions)]
        debug_assert_eq!(
            result
                .as_ref()
                .map(|()| runtime.motions.snapshot(now))
                .map_err(ToString::to_string),
            reference,
            "an inert commit's motion phase differs from the full one"
        );
        result
    }

    /// The motion values (or the error) the full motion phase reaches from
    /// the current motion state, which it leaves as it is.
    #[cfg(debug_assertions)]
    fn reference_motion_outcome(
        &self,
        root: &UiNode,
        retained: &crate::RetainedUiTree,
        generation: ScriptGeneration,
        domain: &str,
    ) -> Result<BTreeMap<crate::MotionKey, f64>, String> {
        let runtime = self.runtime.borrow();
        let mut motions = runtime.motions.transaction_snapshot();
        let now = runtime.clock.now();
        let outcome = (|| {
            let identities = crate::motion::retained_motion_identities(root, retained, domain)?;
            let geometry_slots = crate::motion::node_motion_resource_usage(root).geometry_slots;
            crate::RuntimeBudgets::check(
                "active_motions",
                geometry_slots,
                runtime.budgets.active_motions,
            )?;
            motions.set_active_limit(runtime.budgets.active_motions);
            let shared_layout_ids = crate::motion::shared_layout_ids(root)?;
            assert!(shared_layout_ids.is_empty(), "an inert tree shares layout");
            motions.set_active_reservation(domain, geometry_slots);
            let incarnations = runtime.component_incarnations_snapshot();
            crate::motion::reconcile_node_motion_scoped_owned(
                root,
                &mut motions,
                now,
                domain,
                crate::motion::MotionReconcileContext {
                    domain,
                    root_component: &self.root_path,
                    root_incarnation: runtime
                        .component_incarnation(&self.root_path)
                        .unwrap_or_else(crate::ComponentIncarnation::unscoped),
                    generation,
                    incarnations: &incarnations,
                    identities: Some(&identities),
                },
            )?;
            let usage = motions.resource_usage();
            let active = usage.active.saturating_add(usage.geometry_slots);
            crate::RuntimeBudgets::check("active_motions", active, runtime.budgets.active_motions)?;
            Ok::<_, LifecycleError>(motions.snapshot(now))
        })();
        outcome.map_err(|error| error.to_string())
    }

    fn resume_runtime_mechanisms(
        &self,
        now: std::time::Instant,
        elapsed: std::time::Duration,
    ) -> Result<(), LifecycleError> {
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?;
        runtime.timers.resume_component_scope(&self.root_path, now);
        runtime
            .motions
            .resume_node_scope(&self.motion_root_path(), now);
        runtime
            .geometry_for(self.presentation_scope())
            .delay_motion(elapsed);
        runtime.motion_values = runtime.motions.snapshot(now);
        Ok(())
    }

    fn reconcile_exit_ghosts(&self, report: &crate::ReconcileReport) -> Result<(), LifecycleError> {
        let Some(previous) = self.root.as_deref() else {
            return Ok(());
        };
        let Some(root_id) = self.retained.root_id() else {
            return Ok(());
        };
        let removed = report.unmounted.iter().copied().collect::<BTreeSet<_>>();
        let mut exits = Vec::<(crate::NodeId, &UiNode)>::new();
        collect_removed_exit_nodes(previous, root_id, &self.retained, &removed, &mut exits);
        if exits.is_empty() {
            return Ok(());
        }
        let mut runtime = self
            .runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?;
        let now = runtime.clock.now();
        let domain = self.motion_root_path();
        let geometry_registry = runtime.geometry_for(self.presentation_scope());
        for (retained, node) in exits {
            let Some(geometry) = geometry_registry.get(retained) else {
                continue;
            };
            let ghost = node.motion_ghost().ok_or_else(|| {
                LifecycleError::Motion(MotionError::UnsupportedExitGhost(format!(
                    "{:?}",
                    node.kind_tag()
                )))
            })?;
            let path = format!("{domain}/ghost:{}", retained.get());
            if runtime.motion_ghosts.iter().any(|ghost| ghost.path == path) {
                continue;
            }
            for source in node.exit_motions() {
                runtime.motions.start_exit(
                    ComponentInstancePath::root("UiNode", path.clone()),
                    source.clone(),
                    now,
                )?;
            }
            if runtime.motions.is_node_scope_active(&path) {
                runtime.motion_ghosts.push(crate::motion::MotionGhost {
                    id: retained.get().to_string(),
                    node: ghost,
                    bounds: geometry.visual,
                    path,
                    domain: domain.clone(),
                });
            }
        }
        crate::RuntimeBudgets::check(
            "motion_ghosts",
            runtime.motion_ghosts.len(),
            runtime.budgets.motion_ghosts,
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn validate_resource_budgets(
        &self,
        engine: &RuntimeEngine,
        retained: &crate::RetainedUiTree,
        root: &UiNode,
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
        let layers = retained
            .nodes()
            .filter(|node| node.kind() == crate::UiNodeKindTag::Layer)
            .count();
        crate::RuntimeBudgets::check("layers", layers, budgets.layers)?;
        let canvas_scenes = retained
            .nodes()
            .filter(|node| node.kind() == crate::UiNodeKindTag::Canvas)
            .count();
        crate::RuntimeBudgets::check("canvas_scenes", canvas_scenes, budgets.canvas_scenes)?;
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
        let motion = crate::motion::node_motion_resource_usage(root);
        crate::RuntimeBudgets::check(
            "motion_declarations",
            motion.declarations,
            budgets.motion_declarations,
        )?;
        crate::RuntimeBudgets::check(
            "motion_keyframes",
            motion.keyframes,
            budgets.motion_keyframes,
        )?;
        crate::RuntimeBudgets::check(
            "motion_timelines",
            motion.timelines,
            budgets.motion_timelines,
        )?;
        crate::RuntimeBudgets::check(
            "motion_timeline_steps",
            motion.timeline_steps,
            budgets.motion_timeline_steps,
        )?;
        crate::RuntimeBudgets::check(
            "motion_particles",
            motion.particles,
            budgets.motion_particles,
        )?;
        crate::RuntimeBudgets::check(
            "shared_motion_snapshots",
            motion.shared_snapshots,
            budgets.shared_motion_snapshots,
        )?;
        crate::RuntimeBudgets::check(
            "active_motions",
            motion.geometry_slots,
            budgets.active_motions,
        )?;
        Ok(())
    }

    fn reconcile_effects(
        &self,
        engine: &mut RuntimeEngine,
        candidate: &CompiledUi,
        previous_state: crate::StateStore,
        retained: &crate::RetainedUiTree,
        root: &UiNode,
    ) -> Result<(), LifecycleError> {
        let declarations = RetainedDeclarations {
            effects: engine.component_effects_in_scope(&self.root_path),
            timers: engine.component_timers_in_scope(&self.root_path),
            signals: engine.component_signals_in_scope(&self.root_path),
            element_refs: self.element_ref_bindings(engine, retained)?,
        };
        // The items virtual collections keep keep their reads too.
        let mut contributions = root.virtual_read_contributions();
        contributions.extend(engine.kept_virtual_read_contributions(&self.root_path));
        self.runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?
            .retain_virtual_read_contributions(&self.root_path, &contributions);
        self.reconcile_effect_candidate(engine, candidate, declarations, Some(previous_state))?;
        self.runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .virtual_requests
            .native_views
            .retain_scope(&self.root_path, retained, candidate.generation());
        Ok(())
    }

    fn reconcile_effect_candidate(
        &self,
        engine: &mut RuntimeEngine,
        candidate: &CompiledUi,
        declarations: RetainedDeclarations,
        previous_state: Option<crate::StateStore>,
    ) -> Result<(), LifecycleError> {
        {
            let mut runtime = self
                .runtime
                .try_borrow_mut()
                .map_err(|_| LifecycleError::Borrowed)?;
            runtime
                .effect_cascades
                .register_generation(&self.root_path, candidate.generation());
            runtime.prune_pending_dispatch();
        }
        let plan = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .effects
            .plan(&self.root_path, declarations.effects);
        let transition_count = plan.transition_count();
        self.runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?
            .effect_cascades
            .charge(&self.root_path, transition_count)
            .map_err(LifecycleError::EffectBudget)?;
        if plan.has_cleanup()
            && let Some(previous_state) = previous_state
        {
            self.runtime
                .try_borrow_mut()
                .map_err(|_| LifecycleError::Borrowed)?
                .component_state = previous_state;
        }
        for (descriptor, scope) in plan.cleanup_descriptors() {
            self.invoke_effect_callback(
                engine,
                candidate,
                descriptor.cleanup(),
                descriptor.dependencies().clone(),
                scope,
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
            let geometry_bindings = runtime.element_refs.geometry_bindings(&self.root_path);
            let geometry = runtime.geometry_for(self.presentation_scope());
            geometry.sync_ref_readers(&self.root_path, geometry_bindings);
            runtime
                .virtual_requests
                .retain_within(&self.root_path, &virtual_collections);
        }
        for (descriptor, scope) in plan.start_descriptors() {
            self.invoke_effect_callback(
                engine,
                candidate,
                descriptor.start(),
                descriptor.dependencies().clone(),
                scope,
            )?;
        }
        {
            let mut runtime = self
                .runtime
                .try_borrow_mut()
                .map_err(|_| LifecycleError::Borrowed)?;
            for (_, scope) in plan.cleanup_descriptors() {
                runtime.cancel_async_scope(&scope)?;
            }
            runtime.effects.commit(plan);
        }
        Ok(())
    }

    fn invoke_effect_callback(
        &self,
        engine: &RuntimeEngine,
        candidate: &CompiledUi,
        callback: &ScriptCallback,
        dependencies: UiValue,
        scope: AsyncScope,
    ) -> Result<(), LifecycleError> {
        self.validate_callback_owner(callback)?;
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
            .with_native_context(callback.native_context().cloned())
            .with_async_scope(scope);
        self.with_dispatch_owner(None, || {
            self.with_origin(crate::InvocationOrigin::Effect, || {
                let _ = engine.invoke_callback_for_generation(
                    compiled,
                    callback,
                    (context, dependencies.into_dynamic()),
                )?;
                Ok(())
            })
        })
    }

    fn validate_callback_owner(&self, callback: &ScriptCallback) -> Result<(), LifecycleError> {
        let (Some(component), Some(incarnation)) = (callback.component(), callback.incarnation())
        else {
            return Ok(());
        };
        let active = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?
            .component_incarnation(component);
        if active == Some(incarnation) {
            Ok(())
        } else {
            Err(RuntimeError::StaleComponentCallback {
                name: callback.name().to_owned(),
                component: component.clone(),
            }
            .into())
        }
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
            if let Some(style) = node.signal_style() {
                let _ = runtime.signals.read(&style.signal)?;
            }
            for binding in node.progress_motions() {
                if let Some(trigger) = &binding.signal_trigger {
                    let _ = runtime.signals.read(&trigger.signal)?;
                }
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
        let runtime = self
            .runtime
            .try_borrow()
            .map_err(|_| LifecycleError::Borrowed)?;
        runtime
            .geometry_for(self.presentation_scope())
            .retain_nodes(|node| retained.node(node).is_some());
        runtime
            .pointer_capture_for(self.presentation_scope())
            .retain_nodes(|node| retained.node(node).is_some() && retained.is_page_visible(node));
        Ok(())
    }

    fn presentation_scope(&self) -> Option<&str> {
        self.view.as_deref().or(self.window.as_deref())
    }

    fn begin_runtime_transaction(&self) -> Result<crate::UiStateSnapshot, LifecycleError> {
        self.runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?
            .begin_transaction()
            .map_err(LifecycleError::from)
    }

    fn restart_effect_cascade(&self) -> Result<(), LifecycleError> {
        self.runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?
            .effect_cascades
            .restart(&self.root_path);
        Ok(())
    }

    fn commit_runtime_transaction(&self) -> Result<(), LifecycleError> {
        self.runtime
            .try_borrow_mut()
            .map_err(|_| LifecycleError::Borrowed)?
            .commit_transaction()?;
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

    pub(crate) fn diagnostic_context(
        &self,
        engine: &RuntimeEngine,
        component: Option<&ComponentInstancePath>,
    ) -> Result<crate::DiagnosticContext, crate::DiagnosticContextError> {
        let root_context = self.context(ExecutionPhase::Event);
        let component = component
            .cloned()
            .or_else(|| engine.last_failed_component())
            .unwrap_or_else(|| self.root_path.clone());
        let key = component.leaf_key().map(str::to_owned);
        let context = root_context.for_component(component, BTreeMap::new());
        let source = engine.last_failed_timing().map(|timing| timing.source);
        crate::DiagnosticContext::capture(engine, &context, source, key)
    }

    #[must_use]
    pub fn compiled(&self) -> CompiledUi {
        self.compiled.clone()
    }

    fn motion_root_path(&self) -> String {
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
    Motion(#[from] MotionError),
    #[error(transparent)]
    Reconcile(#[from] crate::ReconcileError),
    #[error("script lifecycle has no accepted root")]
    MissingRoot,
    #[error("suspended lifecycle is missing its monotonic suspension timestamp")]
    MissingSuspendTime,
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
    Transaction(#[from] crate::UiTransactionError),
    #[error(transparent)]
    Signal(#[from] crate::SignalError),
    #[error(transparent)]
    ElementRef(#[from] crate::ElementRefError),
    #[error(transparent)]
    Budget(#[from] crate::RuntimeBudgetError),
    #[error(transparent)]
    Accessibility(#[from] crate::AccessibilityError),
}

fn collect_removed_exit_nodes<'a>(
    node: &'a UiNode,
    id: crate::NodeId,
    retained: &crate::RetainedUiTree,
    removed: &BTreeSet<crate::NodeId>,
    output: &mut Vec<(crate::NodeId, &'a UiNode)>,
) {
    if removed.contains(&id) && !node.exit_motions().is_empty() {
        output.push((id, node));
        return;
    }
    let Some(retained_node) = retained.node(id) else {
        return;
    };
    for (group, children) in node.retained_child_groups() {
        let ids = retained_node
            .children()
            .filter(|child| child.group() == group)
            .map(crate::RetainedChildLink::node)
            .collect::<Vec<_>>();
        for (child, id) in children.into_iter().zip(ids) {
            collect_removed_exit_nodes(child, id, retained, removed, output);
        }
    }
}

fn replaceable_component_paths(
    root: &UiNode,
    root_path: &ComponentInstancePath,
    paths: Vec<ComponentInstancePath>,
    active: &BTreeSet<ComponentInstancePath>,
) -> Option<Vec<ComponentInstancePath>> {
    let index = crate::node::ComponentSubtreeIndex::new(root);
    let mut replaceable = BTreeSet::new();
    for path in paths {
        let mut candidate = path;
        loop {
            if active.contains(&candidate) && index.get(root, &candidate).is_some() {
                replaceable.insert(candidate);
                break;
            }
            candidate = candidate.parent()?;
            if &candidate == root_path {
                return None;
            }
        }
    }
    Some(topmost_paths(&replaceable))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AssetData, AsyncCapabilityHandler, CapabilityDescriptor, CapabilityId, CapabilityMethod,
        ExecutionOperation, InMemoryAssetProvider, OpaqueHandle, StateField,
        SubscriptionCapabilityHandler, SubscriptionWork, TaskWork, UiNodeKind, UiValue,
        ValueSchema,
    };
    use semver::{Version, VersionReq};
    use std::time::{Duration, Instant};

    #[test]
    fn effect_origin_followup_does_not_restart_a_rejected_cascade() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
            fn followup(ctx, payload) { ctx.set_state("phase", "changed"); }
            fn view(ctx) { text(ctx.get_state("phase")) }
        "#,
            )
            .unwrap();
        let followup = engine.callback(&compiled, "followup").unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let root = ComponentInstancePath::root("App", "main");
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            root.clone(),
            Some("main".to_owned()),
            BTreeMap::new(),
            &state_schema(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        {
            let mut runtime = runtime.borrow_mut();
            runtime.effect_cascades.restart(&root);
            runtime.effect_cascades.charge(&root, 64).unwrap();
            assert!(runtime.effect_cascades.charge(&root, 1).is_err());
        }
        let _ = lifecycle
            .with_origin(crate::InvocationOrigin::Effect, || {
                lifecycle.invoke_callback_transactional(&engine, &followup, UiValue::Null)
            })
            .unwrap();
        assert!(!lifecycle.render_dirty(&mut engine).unwrap());
        let _ = lifecycle
            .invoke_callback_transactional(&engine, &followup, UiValue::Null)
            .unwrap();
        assert!(lifecycle.render_dirty(&mut engine).unwrap());
    }

    #[test]
    fn pure_event_loop_without_effect_transitions_shares_budget_and_prunes_restored_queue() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
            fn echo(ctx, payload) { ctx.emit("tick", ()); }
            fn view(ctx) { text("stable") }
        "#,
            )
            .unwrap();
        let mut echo = engine.callback(&compiled, "echo").unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let root = ComponentInstancePath::root("App", "pure_event");
        let events = BTreeMap::from([(
            "tick".to_owned(),
            crate::EventSchema::new(ValueSchema::Null),
        )]);
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            root.clone(),
            None,
            events.clone(),
            &ComponentStateSchema::default(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        echo.bind_component_scope_if_unset(
            &root,
            runtime.borrow().component_incarnation(&root).unwrap(),
            events,
        );
        runtime.borrow_mut().replace_component_event_handlers(
            &root,
            BTreeMap::from([((root.clone(), "tick".to_owned()), echo)]),
        );
        let context = lifecycle.context(ExecutionPhase::Event);
        lifecycle
            .with_origin(crate::InvocationOrigin::Effect, || {
                context.emit("tick", Dynamic::UNIT).unwrap();
                Ok(())
            })
            .unwrap();
        let checkpoint = runtime.borrow_mut().begin_transaction().unwrap();
        let mut rejected = false;
        for _ in 0..65 {
            let pending = runtime.borrow_mut().drain_batch().events.pop().unwrap();
            match lifecycle.invoke_component_event_transactional(&engine, pending) {
                Ok(Some(_)) => (),
                Err(LifecycleError::EffectBudget(65)) => {
                    rejected = true;
                    break;
                }
                result => panic!("unexpected pure event result {result:?}"),
            }
        }
        assert!(rejected);
        runtime
            .borrow_mut()
            .rollback_transaction(checkpoint)
            .unwrap();
        assert!(runtime.borrow_mut().drain_batch().events.is_empty());
        assert!(!lifecycle.render_dirty(&mut engine).unwrap());
    }

    fn state_schema() -> ComponentStateSchema {
        ComponentStateSchema::new(BTreeMap::from([(
            "phase".to_owned(),
            StateField::new(ValueSchema::string(), UiValue::String("created".to_owned())),
        )]))
        .unwrap()
    }

    const STREAM_COMPONENT_APP: &str = r#"
        define_component(#{
            metadata: #{ id: "test/stream", "export": "StreamProbe", version: "0.1.0",
                runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
                dependencies: [], capabilities: #{ "app.stream": "*" } },
            schema: #{ props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
                state: #{ fields: #{ phase: #{ schema: #{ type: "string" },
                    "default": #{ type: "string", value: "idle" } } } },
                events: #{}, slots: #{}, parts: [], effects: ["watch"] },
            render: Fn("render_StreamProbe"),
        });
        fn start_stream(ctx, deps) {
            ctx.start_subscription(
                "app.stream", "watch", (),
                Fn("received"), Fn("failed"), #{ delivery: "all" }
            );
        }
        fn stop_stream(ctx, deps) { () }
        fn received(ctx, value) { ctx.set_state("phase", value); }
        fn failed(ctx, error) { ctx.set_state("phase", "failed"); }
        fn render_StreamProbe(ctx, props) {
            effect("watch", (), Fn("start_stream"), Fn("stop_stream"));
            text(ctx.get_state("phase"))
        }
        fn view(ctx) { render_component("test/stream", #{ key: "stream" }) }
    "#;

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
    fn lifecycle_motion_uses_the_host_runtime_clock() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn view(ctx) {
                        text("clocked")
                            .with_key("probe")
                            .motion(motion_transition("width", 0.0, 100.0, #{ duration_ms: 100, easing: "linear" }))
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
                .motion_values
                .values()
                .copied()
                .collect::<Vec<_>>(),
            vec![0.0]
        );

        manual.advance(Duration::from_millis(50));
        let mut runtime = runtime.borrow_mut();
        let now = runtime.clock.now();
        let frame = runtime.motions.tick(now);
        assert_eq!(
            frame.values.values().copied().collect::<Vec<_>>(),
            vec![50.0]
        );
    }

    #[test]
    fn reload_migrates_timeline_authority_to_the_candidate_generation() {
        let source = r#"
            fn view(ctx) {
                text("x").with_key("x").timeline(motion_timeline("intro",
                    motion_track(".", motion_transition("opacity", 0.0, 1.0,
                        #{ duration_ms: 1000, easing: "linear" })), #{}))
            }
        "#;
        let mut engine = RuntimeEngine::new();
        let compiled = engine.compile(source).unwrap();
        let now = Instant::now();
        let manual = crate::ManualRuntimeClock::new(now);
        let mut state = UiRuntimeState::new();
        state.clock = manual.clock();
        let runtime = Rc::new(RefCell::new(state));
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            ComponentInstancePath::root("View", "reload"),
            Some("window".to_owned()),
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap()
        .with_view_id("reload");
        lifecycle.start(&mut engine).unwrap();
        let old = lifecycle
            .context(ExecutionPhase::Event)
            .motion_handle("intro")
            .unwrap();
        manual.advance(Duration::from_millis(250));
        let generation = lifecycle.generation();
        let rejected = engine
            .compile(r#"fn view(ctx) { throw "reload rejected"; }"#)
            .unwrap();
        assert!(
            lifecycle
                .reload(&mut engine, rejected, &ComponentStateSchema::default(),)
                .is_err()
        );
        assert_eq!(lifecycle.generation(), generation);
        assert_eq!(
            runtime.borrow().motions.timeline_state(&old),
            Some(crate::MotionPlaybackState::Playing)
        );
        let candidate = engine.compile(source).unwrap();
        lifecycle
            .reload(&mut engine, candidate, &ComponentStateSchema::default())
            .unwrap();
        let current = lifecycle
            .context(ExecutionPhase::Event)
            .motion_handle("intro")
            .unwrap();
        assert_ne!(old, current);
        assert_eq!(runtime.borrow().motions.timeline_state(&old), None);
        assert_eq!(
            runtime.borrow().motions.timeline_state(&current),
            Some(crate::MotionPlaybackState::Playing)
        );
        assert!(
            (runtime
                .borrow()
                .motions
                .snapshot(manual.clock().now())
                .values()
                .next()
                .copied()
                .unwrap()
                - 0.25)
                .abs()
                < 0.01
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn resume_reload_unfreezes_motion_at_the_preserved_position() {
        let source = r#"
            fn view(ctx) {
                text("x").with_key("x").motion(motion_transition(
                    "width", 0.0, 100.0, #{ duration_ms: 1000, easing: "linear" }))
            }
        "#;
        let source_with_new_motion = r#"
            fn view(ctx) {
                text("x").with_key("x")
                    .motion(motion_transition("width", 0.0, 100.0,
                        #{ duration_ms: 1000, easing: "linear" }))
                    .motion(motion_transition("opacity", 0.0, 1.0,
                        #{ duration_ms: 1000, easing: "linear" }))
            }
        "#;
        let mut engine = RuntimeEngine::new();
        let compiled = engine.compile(source).unwrap();
        let now = Instant::now();
        let manual = crate::ManualRuntimeClock::new(now);
        let mut state = UiRuntimeState::new();
        state.clock = manual.clock();
        let runtime = Rc::new(RefCell::new(state));
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            ComponentInstancePath::root("View", "resume-reload"),
            Some("window".to_owned()),
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap()
        .with_view_id("resume-reload");
        lifecycle.start(&mut engine).unwrap();
        let node = lifecycle.retained().root_id().unwrap();
        let geometry = runtime.borrow().geometry_for(Some("resume-reload"));
        let old_bounds = crate::GeometryBounds::new(0.0, 0.0, 100.0, 20.0).unwrap();
        let target_bounds = crate::GeometryBounds::new(100.0, 0.0, 100.0, 20.0).unwrap();
        geometry.update(
            node,
            crate::ElementGeometry {
                layout: old_bounds,
                visual: old_bounds,
                clip: None,
            },
        );
        let layout_request = |now| crate::geometry::LayoutMotionRequest {
            shared: None,
            duration: Duration::from_secs(1),
            easing: crate::MotionEasing::Linear,
            preference: crate::MotionPreference::Normal,
            now,
        };
        let _ = geometry.sample_layout_motion(node, target_bounds, layout_request(now));
        let trigger = crate::MotionProgressBinding::new(
            crate::MotionProgressDriver::Hover,
            crate::MotionSource::Transition(crate::MotionTransition::new(
                crate::MotionProperty::Opacity,
                0.0,
                1.0,
                1_000,
            )),
        )
        .unwrap();
        geometry.set_motion_trigger(node, crate::MotionProgressDriver::Hover, true);
        let _ =
            geometry.sample_motion_trigger(node, &trigger, crate::MotionPreference::Normal, now);
        manual.advance(Duration::from_millis(200));
        let before_layout = geometry.sample_layout_motion(
            node,
            target_bounds,
            layout_request(manual.clock().now()),
        );
        let before_trigger = geometry.sample_motion_trigger(
            node,
            &trigger,
            crate::MotionPreference::Normal,
            manual.clock().now(),
        );
        lifecycle.suspend(&mut engine).unwrap();
        manual.advance(Duration::from_millis(500));
        let rejected = engine
            .compile(r#"fn view(ctx) { throw "resume reload rejected"; }"#)
            .unwrap();
        assert!(
            lifecycle
                .resume_reload(&mut engine, rejected, &ComponentStateSchema::default(),)
                .is_err()
        );
        assert_eq!(lifecycle.state(), LifecycleState::Suspended);
        {
            let mut runtime = runtime.borrow_mut();
            let frame = runtime.motions.tick(manual.clock().now());
            assert!(!frame.needs_frame);
            assert!((frame.values.values().next().copied().unwrap() - 20.0).abs() < 0.01);
        }
        let candidate = engine.compile(source_with_new_motion).unwrap();
        lifecycle
            .resume_reload(&mut engine, candidate, &ComponentStateSchema::default())
            .unwrap();
        let resumed_layout = geometry.sample_layout_motion(
            node,
            target_bounds,
            layout_request(manual.clock().now()),
        );
        let resumed_trigger = geometry.sample_motion_trigger(
            node,
            &trigger,
            crate::MotionPreference::Normal,
            manual.clock().now(),
        );
        assert!((resumed_layout.offset_x - before_layout.offset_x).abs() < 0.01);
        assert!((resumed_trigger.value - before_trigger.value).abs() < 0.01);
        manual.advance(Duration::from_millis(200));
        let mut runtime = runtime.borrow_mut();
        let frame = runtime.motions.tick(manual.clock().now());
        assert!(frame.needs_frame);
        assert!(frame.values.iter().any(|(key, value)| {
            key.property == crate::MotionProperty::Width && (*value - 40.0).abs() < 0.01
        }));
        assert!(frame.values.iter().any(|(key, value)| {
            key.property == crate::MotionProperty::Opacity && (*value - 0.2).abs() < 0.01
        }));
        drop(runtime);
        let advanced_layout = geometry.sample_layout_motion(
            node,
            target_bounds,
            layout_request(manual.clock().now()),
        );
        let advanced_trigger = geometry.sample_motion_trigger(
            node,
            &trigger,
            crate::MotionPreference::Normal,
            manual.clock().now(),
        );
        assert!(advanced_layout.offset_x > resumed_layout.offset_x);
        assert!(advanced_trigger.value > resumed_trigger.value);
    }

    #[test]
    fn retained_root_remount_allocates_new_motion_identity() {
        let source = r#"
            fn view(ctx) {
                text("card").with_key(if ctx.get_state("second") { "two" } else { "one" })
                    .timeline(motion_timeline("intro", motion_track(".",
                        motion_transition("opacity", 0.0, 1.0,
                            #{ duration_ms: 1000, easing: "linear" })), #{}))
            }
        "#;
        let schema = ComponentStateSchema::new(BTreeMap::from([(
            "second".to_owned(),
            StateField::new(ValueSchema::Bool, UiValue::Bool(false)),
        )]))
        .unwrap();
        let mut engine = RuntimeEngine::new();
        let compiled = engine.compile(source).unwrap();
        let now = Instant::now();
        let manual = crate::ManualRuntimeClock::new(now);
        let mut state = UiRuntimeState::new();
        state.clock = manual.clock();
        let runtime = Rc::new(RefCell::new(state));
        let root = ComponentInstancePath::root("View", "remount");
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            root.clone(),
            Some("window".to_owned()),
            BTreeMap::new(),
            &schema,
        )
        .unwrap()
        .with_view_id("remount");
        lifecycle.start(&mut engine).unwrap();
        let old_node = lifecycle.retained().root_id().unwrap();
        let old_handle = lifecycle
            .context(ExecutionPhase::Event)
            .motion_handle("intro")
            .unwrap();
        manual.advance(Duration::from_millis(500));
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "second", UiValue::Bool(true))
            .unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();
        let new_node = lifecycle.retained().root_id().unwrap();
        let new_handle = lifecycle
            .context(ExecutionPhase::Event)
            .motion_handle("intro")
            .unwrap();
        assert_ne!(old_node, new_node);
        assert_ne!(old_handle, new_handle);
        assert_eq!(runtime.borrow().motions.timeline_state(&old_handle), None);
        assert!(
            runtime
                .borrow()
                .motions
                .snapshot(manual.clock().now())
                .values()
                .next()
                .copied()
                .unwrap()
                .abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn geometry_motion_reserves_capacity_from_timeline_control() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn view(ctx) {
                        text("card").with_key("card")
                            .layout_motion(100, "linear")
                            .timeline(motion_timeline("intro", motion_track(".",
                                motion_transition("opacity", 0.0, 1.0,
                                    #{ duration_ms: 100 })), #{ autoplay: false }))
                    }
                "#,
            )
            .unwrap();
        let mut state = UiRuntimeState::new();
        state.budgets.active_motions = 1;
        let runtime = Rc::new(RefCell::new(state));
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            ComponentInstancePath::root("View", "budget"),
            None,
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        let context = lifecycle.context(ExecutionPhase::Event);
        let handle = context.motion_handle("intro").unwrap();
        assert!(matches!(
            context.play_motion(&handle),
            Err(crate::UiContextError::Motion(
                crate::MotionError::ActiveBudget {
                    actual: 2,
                    limit: 1,
                }
            ))
        ));
    }

    #[test]
    fn motion_budget_admits_active_timeline_to_idle_plus_direct_replacement() {
        let source = r#"
            fn view(ctx) {
                let switching = ctx.get_state("go");
                let node = text("card").with_key("card").timeline(motion_timeline(
                    "intro", motion_track(".", motion_transition("opacity", 0.0, 1.0,
                        #{ duration_ms: 1000 })), #{ autoplay: !switching }));
                if switching {
                    node.motion(motion_transition("width", 0.0, 100.0,
                        #{ duration_ms: 1000 }))
                } else {
                    node
                }
            }
        "#;
        let schema = ComponentStateSchema::new(BTreeMap::from([(
            "go".to_owned(),
            StateField::new(ValueSchema::Bool, UiValue::Bool(false)),
        )]))
        .unwrap();
        let mut engine = RuntimeEngine::new();
        let compiled = engine.compile(source).unwrap();
        let mut state = UiRuntimeState::new();
        state.budgets.active_motions = 1;
        let runtime = Rc::new(RefCell::new(state));
        let root = ComponentInstancePath::root("View", "budget-replacement");
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            root.clone(),
            None,
            BTreeMap::new(),
            &schema,
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "go", UiValue::Bool(true))
            .unwrap();
        assert!(lifecycle.render_dirty(&mut engine).unwrap());
        assert_eq!(runtime.borrow().motions.resource_usage().active, 1);
        let handle = lifecycle
            .context(ExecutionPhase::Event)
            .motion_handle("intro")
            .unwrap();
        assert_eq!(
            runtime.borrow().motions.timeline_state(&handle),
            Some(crate::MotionPlaybackState::Idle)
        );
    }

    #[test]
    fn virtual_item_motion_uses_the_stable_data_key_path() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn render_row(ctx, props) {
                        text("A").with_key(props.key)
                            .motion(motion_transition("opacity", 0.0, 1.0,
                                #{ duration_ms: 1000 }))
                    }
                    fn view(ctx) {
                        virtual_collection(#{
                            key: "list",
                            data: [#{ key: "alpha" }],
                            height: 100,
                            estimated_height: 20
                        }, Fn("render_row"))
                    }
                "#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            ComponentInstancePath::root("View", "virtual"),
            None,
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();

        let values = runtime.borrow().motions.snapshot(Instant::now());
        let item = lifecycle
            .retained()
            .nodes()
            .find(|node| node.key() == Some("alpha"))
            .unwrap()
            .id();
        assert!(values.contains_key(&crate::MotionKey::for_node(
            &crate::motion::retained_node_path("root", item),
            crate::MotionProperty::Opacity,
        )));
        assert_eq!(values.len(), 1);
    }

    #[test]
    fn exit_ghost_uses_the_views_committed_geometry_domain() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn view(ctx) {
                        let children = [text(ctx.get_state("n").to_string()).with_key("counter")];
                        if ctx.get_state("visible") {
                            children.push(text("gone").with_key("panel")
                                .exit_motion(motion_transition("opacity", 1.0, 0.0,
                                    #{ duration_ms: 1000, easing: "linear" })))
                        }
                        column(children)
                    }
                "#,
            )
            .unwrap();
        let manual = crate::ManualRuntimeClock::new(Instant::now());
        let mut state = UiRuntimeState::new();
        state.clock = manual.clock();
        let runtime = Rc::new(RefCell::new(state));
        let root = ComponentInstancePath::root("View", "v");
        let schema = ComponentStateSchema::new(BTreeMap::from([
            (
                "visible".to_owned(),
                StateField::new(ValueSchema::Bool, UiValue::Bool(true)),
            ),
            (
                "n".to_owned(),
                StateField::new(ValueSchema::integer(), UiValue::Integer(0)),
            ),
        ]))
        .unwrap();
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            root.clone(),
            Some("w".to_owned()),
            BTreeMap::new(),
            &schema,
        )
        .unwrap()
        .with_view_id("v");
        lifecycle.start(&mut engine).unwrap();
        let panel = lifecycle
            .retained()
            .nodes()
            .find(|node| node.key() == Some("panel"))
            .unwrap()
            .id();
        let bounds = crate::GeometryBounds::new(0.0, 0.0, 100.0, 20.0).unwrap();
        runtime.borrow_mut().presentation_geometry("v").update(
            panel,
            crate::ElementGeometry {
                layout: bounds,
                visual: bounds,
                clip: None,
            },
        );
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "visible", UiValue::Bool(false))
            .unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();

        let ghost_path = {
            let runtime = runtime.borrow();
            assert_eq!(runtime.motion_ghosts.len(), 1);
            assert_eq!(runtime.motion_ghosts[0].domain, "window:w/view:v/root");
            assert_eq!(runtime.motion_ghosts[0].bounds, bounds);
            assert!(
                runtime.motion_ghosts[0]
                    .path
                    .starts_with("window:w/view:v/root/ghost:")
            );
            assert!(runtime.motions.resource_usage().active > 0);
            runtime.motion_ghosts[0].path.clone()
        };
        manual.advance(Duration::from_millis(100));
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "n", UiValue::Integer(1))
            .unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();
        assert_eq!(runtime.borrow().motions.resource_usage().active, 1);
        assert!(
            (runtime.borrow().motions.snapshot(manual.clock().now())
                [&crate::MotionKey::for_node(&ghost_path, crate::MotionProperty::Opacity,)]
                - 0.9)
                .abs()
                < 0.01
        );
        let now = manual.clock().now();
        let frame = runtime
            .borrow_mut()
            .motions
            .tick_scope(now + Duration::from_millis(900), "window:w/view:v/root");
        assert!(frame.completed.contains(&crate::MotionKey::for_node(
            &ghost_path,
            crate::MotionProperty::Opacity,
        )));
    }

    #[test]
    fn suspend_resume_preserves_state_and_freezes_motion_time() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn suspend(ctx) { ctx.set_state("phase", "suspended"); }
                    fn resume(ctx, elapsed_ms) {
                        ctx.set_state("phase", "resumed");
                        ctx.set_state("elapsed", elapsed_ms);
                    }
                    fn view(ctx) {
                        text(ctx.get_state("phase")).with_key("probe")
                            .motion(motion_transition("width", 0.0, 100.0, #{ duration_ms: 100, easing: "linear" }))
                    }
                "#,
            )
            .unwrap();
        let start = Instant::now();
        let manual = crate::ManualRuntimeClock::new(start);
        let mut runtime_state = UiRuntimeState::new();
        runtime_state.clock = manual.clock();
        let runtime = Rc::new(RefCell::new(runtime_state));
        let path = ComponentInstancePath::root("App", "root");
        let schema = ComponentStateSchema::new(BTreeMap::from([
            (
                "phase".to_owned(),
                StateField::new(ValueSchema::string(), UiValue::String("active".to_owned())),
            ),
            (
                "elapsed".to_owned(),
                StateField::new(ValueSchema::integer(), UiValue::Integer(0)),
            ),
        ]))
        .unwrap();
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            path.clone(),
            Some("main".to_owned()),
            BTreeMap::new(),
            &schema,
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        manual.advance(Duration::from_millis(50));
        {
            let mut runtime = runtime.borrow_mut();
            let now = runtime.clock.now();
            runtime.motion_values = runtime.motions.tick(now).values;
        }
        let before = *runtime.borrow().motion_values.values().next().unwrap();
        assert!(lifecycle.suspend(&mut engine).unwrap());
        assert_eq!(lifecycle.state(), LifecycleState::Suspended);
        assert_eq!(
            runtime.borrow().component_state.get(&path, "phase"),
            Some(&UiValue::String("suspended".to_owned()))
        );
        assert!(!lifecycle.suspend(&mut engine).unwrap());

        manual.advance(Duration::from_secs(1));
        {
            let mut runtime = runtime.borrow_mut();
            let now = runtime.clock.now();
            let frame = runtime.motions.tick(now);
            assert!(
                !frame.needs_frame,
                "a suspended scope must not keep another window's frame loop alive"
            );
            runtime.motion_values = frame.values;
        }
        assert!(
            (before - *runtime.borrow().motion_values.values().next().unwrap()).abs() < 0.001,
            "a shared runtime tick must not advance a suspended scope"
        );
        assert!(lifecycle.resume(&mut engine).unwrap());
        assert_eq!(lifecycle.state(), LifecycleState::Running);
        assert_eq!(
            runtime.borrow().component_state.get(&path, "elapsed"),
            Some(&UiValue::Integer(1_000))
        );
        let after = *runtime.borrow().motion_values.values().next().unwrap();
        assert!(
            (before - after).abs() < 0.001,
            "animation advanced while suspended"
        );
        assert!(!lifecycle.resume(&mut engine).unwrap());
    }

    #[test]
    fn suspended_reload_migrates_atomically_and_keeps_last_good_on_failure() {
        let mut engine = RuntimeEngine::new();
        let active = engine
            .compile(
                r#"
                    fn suspend(ctx) { ctx.set_state("phase", "parked"); }
                    fn view(ctx) { text(ctx.get_state("phase")) }
                "#,
            )
            .unwrap();
        let active_generation = active.generation();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let path = ComponentInstancePath::root("App", "root");
        let mut lifecycle = ScriptLifecycle::new(
            active,
            Rc::clone(&runtime),
            path.clone(),
            Some("main".to_owned()),
            BTreeMap::new(),
            &state_schema(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        lifecycle.suspend(&mut engine).unwrap();

        let rejected = engine
            .compile(
                r#"
                    fn init(ctx) { ctx.set_state("phase", "candidate-init"); }
                    fn resume(ctx, elapsed_ms) { throw "candidate resume failed"; }
                    fn view(ctx) { text(ctx.get_state("phase")) }
                "#,
            )
            .unwrap();
        assert!(
            lifecycle
                .resume_reload(&mut engine, rejected, &state_schema())
                .is_err()
        );
        assert_eq!(lifecycle.state(), LifecycleState::Suspended);
        assert_eq!(lifecycle.generation(), active_generation);
        assert_eq!(
            runtime.borrow().component_state.get(&path, "phase"),
            Some(&UiValue::String("parked".to_owned()))
        );

        let accepted = engine
            .compile(
                r#"
                    fn init(ctx) { ctx.set_state("phase", "candidate-init"); }
                    fn resume(ctx, elapsed_ms) { ctx.set_state("phase", "candidate-resumed"); }
                    fn view(ctx) { text(ctx.get_state("phase")) }
                "#,
            )
            .unwrap();
        let accepted_generation = accepted.generation();
        lifecycle
            .resume_reload(&mut engine, accepted, &state_schema())
            .unwrap();
        assert_eq!(lifecycle.state(), LifecycleState::Running);
        assert_eq!(lifecycle.generation(), accepted_generation);
        assert_eq!(
            runtime.borrow().component_state.get(&path, "phase"),
            Some(&UiValue::String("candidate-resumed".to_owned()))
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn virtual_realization_commits_the_complete_target_window_and_prunes_old_items() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn render_item(ctx, payload) {
                        text(payload.item.label).with_key(payload.key)
                    }
                    fn view(ctx) {
                        let data = [];
                        for index in 0..32 {
                            data.push(#{ key: `row-${index}`, label: `Row ${index}` });
                        }
                        virtual_collection(#{
                            key: "rows", label: "Rows", data: data,
                            estimated_height: 24, height: 96,
                            overdraw_pixels: 24, alignment: "top", follow_tail: false,
                            batch_items: 1, retain_items: 0
                        }, Fn("render_item"))
                    }
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
            &ComponentStateSchema::default(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        let id = engine
            .virtual_collection_ids_in_scope(&path)
            .into_iter()
            .next()
            .unwrap();
        let initial_indices = lifecycle
            .root()
            .unwrap()
            .virtual_collection_items(&id)
            .unwrap()
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let retained = *initial_indices.last().unwrap();
        let first_new = retained + 1;
        let second_new = first_new + 1;
        let _ = engine.take_timings();

        runtime
            .borrow()
            .virtual_requests
            .request(id.clone(), [retained, first_new]);
        assert!(lifecycle.realize_virtual_requests(&mut engine).unwrap());
        assert_eq!(
            lifecycle
                .root()
                .unwrap()
                .virtual_collection_items(&id)
                .unwrap()
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![retained, first_new]
        );
        let timings = engine.take_timings();
        assert_eq!(
            timings
                .iter()
                .filter(|timing| matches!(
                    timing.operation,
                    ExecutionOperation::VirtualCollection(_)
                ))
                .count(),
            1
        );
        assert!(timings.iter().all(|timing| timing.operations > 0));

        runtime
            .borrow()
            .virtual_requests
            .request(id.clone(), [retained, first_new]);
        assert!(!lifecycle.realize_virtual_requests(&mut engine).unwrap());
        assert_eq!(
            engine.take_timings(),
            [] as [crate::engine::ExecutionTiming; 0]
        );

        runtime
            .borrow()
            .virtual_requests
            .request(id.clone(), [first_new, second_new]);
        assert!(lifecycle.realize_virtual_requests(&mut engine).unwrap());
        assert_eq!(
            lifecycle
                .root()
                .unwrap()
                .virtual_collection_items(&id)
                .unwrap()
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![first_new, second_new]
        );
        assert_eq!(
            engine
                .take_timings()
                .iter()
                .filter(|timing| matches!(
                    timing.operation,
                    ExecutionOperation::VirtualCollection(_)
                ))
                .count(),
            1
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn virtual_target_commit_retains_and_releases_the_complete_component_manifest() {
        fn counter_callback(node: &UiNode) -> Option<ScriptCallback> {
            if let Some(handler) = node.handler("click") {
                return handler.as_script().cloned();
            }
            match node.kind() {
                crate::UiNodeKind::Box { children } | crate::UiNodeKind::Fragment { children } => {
                    children.iter().find_map(counter_callback)
                }
                _ => None,
            }
        }
        for (wrapper, row) in [
            (
                "child",
                r#"render_component("test/virtual_counter",#{key:payload.key})"#,
            ),
            (
                "child",
                r#"render_component("test/wrapper",#{key:payload.key,depth:0})"#,
            ),
            (
                "child",
                r#"render_component("test/wrapper",#{key:payload.key,depth:2})"#,
            ),
            (
                "column([child])",
                r#"render_component("test/wrapper",#{key:payload.key,depth:0})"#,
            ),
        ] {
            let mut engine = RuntimeEngine::new();
            let source = r#"
            define_component(#{
                metadata: #{id:"test/virtual_counter", "export":"Counter", version:"0.1.8",
                    runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{}},
                schema: #{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
                    state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:7}}}},
                    events:#{},slots:#{},parts:[],effects:["keep"]},render:Fn("render_counter")
            });
            fn read_count(ctx,payload) {ctx.get_state("count")}
            fn start(ctx,deps) {()}
            fn cleanup(ctx,deps) {()}
            fn tick(ctx,value) {()}
            fn render_counter(ctx,props) {
                effect("keep",(),Fn("start"),Fn("cleanup"));
                timeout("tick",1000,false,Fn("tick"),());
                let opacity=signal("opacity",1.0);
                text(`${ctx.get_state("count")}`).with_key(props.key).bind_signal("opacity",opacity)
                    .with_ref(element_ref("counter")).on_click(Fn("read_count"))
            }
            define_component(#{
                metadata:#{id:"test/wrapper","export":"Wrapper",version:"0.1.8",
                    runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{}},
                schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},
                    depth:#{schema:#{type:"integer"},required:true,sensitive:false}},
                    state:#{fields:#{}},events:#{},slots:#{},parts:[]},render:Fn("render_wrapper")
            });
            fn render_wrapper(ctx,props) {
                let child=if props.depth==0 {
                    render_component("test/virtual_counter",#{key:"inner"})
                } else {
                    render_component("test/wrapper",#{key:"inner",depth:props.depth-1})
                };
                WRAPPER_BODY
            }
            fn row(ctx,payload) {
                if payload.index==19 {throw "candidate row failed";}
                ROW_BODY
            }
            fn view(ctx) {
                let data=[];for index in 0..20 {data.push(#{key:`row-${index}`});}
                virtual_collection(#{key:"rows",label:"Rows",data:data,height:24,
                    estimated_height:24,overdraw_pixels:0,batch_items:1,retain_items:0},Fn("row"))
            }
        "#.replace("WRAPPER_BODY",wrapper).replace("ROW_BODY",row);
            let compiled = engine.compile(&source).unwrap();
            let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
            let path = ComponentInstancePath::root("App", "manifest");
            let mut lifecycle = ScriptLifecycle::new(
                compiled,
                Rc::clone(&runtime),
                path.clone(),
                None,
                BTreeMap::new(),
                &ComponentStateSchema::default(),
            )
            .unwrap();
            lifecycle.start(&mut engine).unwrap();
            let id = engine
                .virtual_collection_ids_in_scope(&path)
                .into_iter()
                .next()
                .unwrap();
            let callback = counter_callback(
                &lifecycle
                    .root()
                    .unwrap()
                    .virtual_collection_items(&id)
                    .unwrap()[&0],
            )
            .unwrap();
            let child = callback.component().unwrap().clone();
            let signal = runtime.borrow().signals.resolve(&child, "opacity").unwrap();
            let initial_effects = runtime
                .borrow()
                .effects
                .iter_active()
                .filter(|(id, _, _)| id.component() == &child)
                .map(|(id, _, activation)| (id.clone(), activation))
                .collect::<Vec<_>>();
            runtime
                .borrow()
                .virtual_requests
                .request(id.clone(), [0, 10]);
            lifecycle.realize_virtual_requests(&mut engine).unwrap();
            assert_eq!(
                lifecycle
                    .invoke_callback(&engine, &callback, UiValue::Null)
                    .unwrap()
                    .as_int()
                    .unwrap(),
                7
            );
            assert_eq!(
                runtime
                    .borrow()
                    .effects
                    .iter_active()
                    .filter(|(id, _, _)| id.component() == &child)
                    .map(|(id, _, activation)| (id.clone(), activation))
                    .collect::<Vec<_>>(),
                initial_effects
            );
            assert!(runtime.borrow().signals.read(&signal).is_ok());
            assert_eq!(engine.component_timers_in_scope(&child).len(), 1);
            assert_eq!(engine.component_element_refs_in_scope(&child).len(), 1);
            let event = UiContext::new(
                Rc::clone(&runtime),
                child.clone(),
                None,
                ExecutionPhase::Event,
                BTreeMap::new(),
            );
            event
                .set_state("count", UiValue::Integer(8).into_dynamic())
                .unwrap();
            assert!(lifecycle.render_dirty(&mut engine).unwrap());
            assert_eq!(
                lifecycle
                    .invoke_callback(&engine, &callback, UiValue::Null)
                    .unwrap()
                    .as_int()
                    .unwrap(),
                8,
                "transparent descendants remain executable after scrolling"
            );
            runtime
                .borrow()
                .virtual_requests
                .request(id.clone(), [0, 10, 19]);
            assert!(lifecycle.realize_virtual_requests(&mut engine).is_err());
            assert_eq!(
                lifecycle
                    .root()
                    .unwrap()
                    .virtual_collection_items(&id)
                    .unwrap()
                    .keys()
                    .copied()
                    .collect::<Vec<_>>(),
                vec![0, 10]
            );
            assert!(runtime.borrow().signals.read(&signal).is_ok());
            runtime.borrow().virtual_requests.request_target(id, [10]);
            lifecycle.realize_virtual_requests(&mut engine).unwrap();
            assert!(
                runtime
                    .borrow()
                    .component_state
                    .get(&child, "count")
                    .is_none()
            );
            assert!(
                lifecycle
                    .invoke_callback(&engine, &callback, UiValue::Null)
                    .is_err()
            );
            assert!(runtime.borrow().signals.read(&signal).is_err());
            assert!(engine.component_timers_in_scope(&child).is_empty());
            assert!(engine.component_element_refs_in_scope(&child).is_empty());
            assert!(
                !runtime
                    .borrow()
                    .effects
                    .iter()
                    .any(|(id, _)| id.component() == &child)
            );
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn nested_virtual_targets_preserve_callbacks_snapshots_and_prune_atomically() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine.compile(r#"
            define_component(#{
                metadata:#{id:"test/panel","export":"Panel",version:"0.1.8",
                    runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{}},
                schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
                    state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:7}}}},
                    events:#{},slots:#{},parts:[]},render:Fn("render_panel")
            });
            fn data() {let rows=[];for i in 0..20 {rows.push(#{key:`row-${i}`});}rows}
            fn read_count(ctx,payload) {ctx.get_state("count")}
            fn inner_row(ctx,payload) {
                if payload.index==19 {throw "candidate rejected";}
                text(payload.key).on_click(Fn("read_count"))
            }
            fn outer_row(ctx,payload) {
                virtual_collection(#{key:`inner-${payload.key}`,label:"Inner",data:data(),height:24,
                    estimated_height:24,overdraw_pixels:0,batch_items:1,retain_items:0},Fn("inner_row"))
            }
            fn render_panel(ctx,props) {
                virtual_collection(#{key:"outer",label:"Outer",data:data(),height:24,
                    estimated_height:24,overdraw_pixels:0,batch_items:1,retain_items:0},Fn("outer_row"))
            }
            fn view(ctx) {column([text(`${ctx.get_state("other")}`),render_component("test/panel",#{key:"panel"})])}
        "#).unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let path = ComponentInstancePath::root("App", "nested");
        let schema = ComponentStateSchema::new(BTreeMap::from([(
            "other".into(),
            crate::StateField::new(crate::ValueSchema::integer(), UiValue::Integer(0)),
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
        let panel = path.child("Panel", "panel");
        let ids = engine.virtual_collection_ids_in_scope(&path);
        let outer = ids.iter().find(|id| id.key == "outer").unwrap().clone();
        let inner = ids
            .iter()
            .find(|id| id.key == "inner-row-0")
            .unwrap()
            .clone();
        runtime
            .borrow()
            .virtual_requests
            .request_target(inner.clone(), [10]);
        lifecycle.realize_virtual_requests(&mut engine).unwrap();
        let callback = lifecycle
            .root()
            .unwrap()
            .virtual_collection_items(&inner)
            .unwrap()[&10]
            .handler("click")
            .unwrap()
            .as_script()
            .unwrap()
            .clone();
        assert_eq!(callback.component(), Some(&panel));
        assert_eq!(
            lifecycle
                .invoke_callback(&engine, &callback, UiValue::Null)
                .unwrap()
                .as_int()
                .unwrap(),
            7
        );

        // An unrelated root update reuses Panel. Its committed snapshot must
        // contain the latest inner target, not the initial seed window.
        let context = UiContext::new(
            Rc::clone(&runtime),
            path,
            None,
            ExecutionPhase::Event,
            BTreeMap::new(),
        );
        context
            .set_state("other", UiValue::Integer(1).into_dynamic())
            .unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();
        assert_eq!(
            lifecycle
                .root()
                .unwrap()
                .virtual_collection_items(&inner)
                .unwrap()
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![10]
        );

        // Keep the outer row while adding a sibling, then exercise rollback.
        runtime
            .borrow()
            .virtual_requests
            .request_target(outer.clone(), [0, 10]);
        lifecycle.realize_virtual_requests(&mut engine).unwrap();
        assert!(
            engine
                .virtual_collection_ids_in_scope(&panel)
                .contains(&inner)
        );
        runtime
            .borrow()
            .virtual_requests
            .request_target(inner.clone(), [10, 19]);
        assert!(lifecycle.realize_virtual_requests(&mut engine).is_err());
        assert_eq!(
            lifecycle
                .root()
                .unwrap()
                .virtual_collection_items(&inner)
                .unwrap()
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![10]
        );

        // A simultaneous parent prune makes a child request obsolete; it must
        // not roll back the parent or recreate the removed collection.
        runtime
            .borrow()
            .virtual_requests
            .request_target(outer, [10]);
        runtime
            .borrow()
            .virtual_requests
            .request_target(inner.clone(), [11]);
        lifecycle.realize_virtual_requests(&mut engine).unwrap();
        assert!(
            lifecycle
                .root()
                .unwrap()
                .virtual_collection_spec(&inner)
                .is_none()
        );
        assert!(
            !engine
                .virtual_collection_ids_in_scope(&panel)
                .contains(&inner)
        );
    }

    /// 64 rows of 24px in a 96px viewport with one row of overdraw, `EXTRA` appended to
    /// the collection's config. Rows from 40 on are a component that owns an effect, and
    /// row 50 fails to render. The state `version` renders the collection again.
    const RENDER_AHEAD: &str = r#"
        define_component(#{
            metadata:#{id:"test/ticking","export":"Ticking",version:"0.1.0",
                runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{}},
            schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
                state:#{fields:#{}},events:#{},slots:#{},parts:[],effects:["tick"]},
            render:Fn("render_ticking")
        });
        fn start(ctx,deps) {()}
        fn stop(ctx,deps) {()}
        fn render_ticking(ctx,props) {
            effect("tick",(),Fn("start"),Fn("stop"));
            text("ticking").with_key(props.key)
        }
        fn row(ctx,payload) {
            if payload.index == 50 { throw "row 50 failed"; }
            if payload.index >= 40 { render_component("test/ticking",#{key:payload.key}) }
            else { text(payload.item.label).with_key(payload.key) }
        }
        fn view(ctx) {
            let version = ctx.get_state("version");
            let data=[];
            for index in 0..64 {data.push(#{key:`row-${index}`,label:`Row ${index}:${version}`});}
            virtual_collection(#{key:"rows",label:"Rows",data:data,height:96,
                estimated_height:24,overdraw_pixels:24 EXTRA},Fn("row"))
        }
    "#;

    struct RenderAhead {
        engine: RuntimeEngine,
        lifecycle: ScriptLifecycle,
        runtime: Rc<RefCell<UiRuntimeState>>,
        id: crate::VirtualCollectionId,
        path: ComponentInstancePath,
    }

    impl RenderAhead {
        fn start(extra: &str, policy: crate::VirtualCollectionPolicy) -> Self {
            let mut engine = RuntimeEngine::new();
            engine.set_virtual_collection_policy(policy);
            let compiled = engine
                .compile(&RENDER_AHEAD.replace("EXTRA", extra))
                .unwrap();
            let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
            let path = ComponentInstancePath::root("App", "ahead");
            let schema = ComponentStateSchema::new(BTreeMap::from([(
                "version".to_owned(),
                StateField::new(ValueSchema::integer(), UiValue::Integer(0)),
            )]))
            .unwrap();
            let mut lifecycle = ScriptLifecycle::new(
                compiled,
                Rc::clone(&runtime),
                path.clone(),
                Some("main".to_owned()),
                BTreeMap::new(),
                &schema,
            )
            .unwrap();
            lifecycle.start(&mut engine).unwrap();
            let id = engine
                .virtual_collection_ids_in_scope(&path)
                .into_iter()
                .next()
                .unwrap();
            let _ = engine.take_timings();
            Self {
                engine,
                lifecycle,
                runtime,
                id,
                path,
            }
        }

        fn realized(&self) -> Vec<usize> {
            self.lifecycle
                .root()
                .unwrap()
                .virtual_collection_items(&self.id)
                .unwrap()
                .keys()
                .copied()
                .collect()
        }

        /// Ask for `range`, as a frame that needs it does, replacing any pending target;
        /// returns the rows the renderer rendered for it.
        fn request(&mut self, range: std::ops::Range<usize>) -> usize {
            self.runtime
                .borrow()
                .virtual_requests
                .request_target(self.id.clone(), range);
            self.lifecycle
                .realize_virtual_requests(&mut self.engine)
                .unwrap();
            self.engine
                .take_timings()
                .iter()
                .filter(|timing| {
                    matches!(timing.operation, ExecutionOperation::VirtualCollection(_))
                })
                .count()
        }

        fn render_again(&mut self) {
            self.runtime
                .borrow_mut()
                .set_component_state_from_host(&self.path, "version", UiValue::Integer(1))
                .unwrap();
            assert!(self.lifecycle.render_dirty(&mut self.engine).unwrap());
            let _ = self.engine.take_timings();
        }
    }

    #[test]
    fn a_commit_keeps_the_requests_of_other_scopes_sharing_the_runtime() {
        let mut ahead = RenderAhead::start(
            ",batch_items:1,retain_items:0",
            crate::VirtualCollectionPolicy::default(),
        );
        // A window opened from this view shares its runtime and commits on its own.
        let theirs = crate::VirtualCollectionId {
            component: ComponentInstancePath::root("View", "window"),
            key: "rows".to_owned(),
        };
        ahead
            .runtime
            .borrow()
            .virtual_requests
            .request(theirs.clone(), [4]);
        assert_eq!(ahead.request(2..8), 2);
        let pending = ahead.runtime.borrow().virtual_requests.drain();
        assert_eq!(pending.get(&theirs), Some(&BTreeSet::from([4])));
    }

    #[test]
    fn a_scroll_that_needs_new_items_renders_a_batch_beyond_them() {
        let mut ahead = RenderAhead::start(
            ",batch_items:8,retain_items:0",
            crate::VirtualCollectionPolicy::default(),
        );
        assert_eq!(ahead.realized(), (0..6).collect::<Vec<_>>());
        // Rows 6 and 7 are needed; seven more follow them.
        assert_eq!(ahead.request(2..8), 9);
        assert_eq!(ahead.realized(), (2..15).collect::<Vec<_>>());
        // Scrolling back up needs rows 0 and 1, and renders seven above them where there
        // are rows: none.
        assert_eq!(ahead.request(0..6), 2);
        assert_eq!(ahead.realized(), (0..6).collect::<Vec<_>>());
    }

    #[test]
    fn items_leaving_the_range_come_back_without_rendering() {
        let mut ahead = RenderAhead::start(
            ",batch_items:1,retain_items:32",
            crate::VirtualCollectionPolicy::default(),
        );
        assert_eq!(ahead.request(20..26), 6);
        assert_eq!(ahead.request(0..6), 0, "rows 0 to 5 were kept");
        assert_eq!(ahead.realized(), (0..6).collect::<Vec<_>>());
        let items = ahead
            .lifecycle
            .root()
            .unwrap()
            .virtual_collection_items(&ahead.id)
            .unwrap()
            .clone();
        assert!(matches!(items[&3].kind(), UiNodeKind::Text { text } if text == "Row 3:0"));
        assert_eq!(ahead.request(20..26), 0, "and so were rows 20 to 25");
    }

    #[test]
    fn kept_items_are_dropped_when_the_collection_renders_again() {
        let mut ahead = RenderAhead::start(
            ",batch_items:1,retain_items:32",
            crate::VirtualCollectionPolicy::default(),
        );
        assert_eq!(ahead.request(20..26), 6);
        assert_eq!(ahead.request(30..36), 6);
        ahead.render_again();
        // Rows 20 to 25 were kept from the first render; the new one renders them anew,
        // with the new data.
        assert_eq!(ahead.request(20..26), 6);
        let items = ahead
            .lifecycle
            .root()
            .unwrap()
            .virtual_collection_items(&ahead.id)
            .unwrap()
            .clone();
        assert!(matches!(items[&22].kind(), UiNodeKind::Text { text } if text == "Row 22:1"));
    }

    #[test]
    fn an_item_whose_component_owns_an_effect_is_released_not_kept() {
        let mut ahead = RenderAhead::start(
            ",batch_items:1,retain_items:32",
            crate::VirtualCollectionPolicy::default(),
        );
        assert_eq!(ahead.request(40..46), 6);
        assert_eq!(ahead.request(20..26), 6);
        // Rows 40 to 45 stopped their effects when they left; they render again.
        assert_eq!(ahead.request(40..46), 6);
        // Rows 20 to 25 own no work and were kept.
        assert_eq!(ahead.request(20..26), 0);
    }

    #[test]
    fn kept_items_survive_a_failed_realization() {
        let mut ahead = RenderAhead::start(
            ",batch_items:1,retain_items:32",
            crate::VirtualCollectionPolicy::default(),
        );
        assert_eq!(ahead.request(20..26), 6);
        // Row 50 fails: the items roll back to 20 to 25, which this attempt had kept.
        ahead
            .runtime
            .borrow()
            .virtual_requests
            .request(ahead.id.clone(), 48..52);
        assert!(
            ahead
                .lifecycle
                .realize_virtual_requests(&mut ahead.engine)
                .is_err()
        );
        let _ = ahead.engine.take_timings();
        assert_eq!(ahead.realized(), (20..26).collect::<Vec<_>>());
        // What the collection kept before and during the failed attempt still comes back
        // without rendering.
        assert_eq!(ahead.request(0..6), 0);
        assert_eq!(ahead.request(20..26), 0);
        assert_eq!(ahead.realized(), (20..26).collect::<Vec<_>>());
    }

    #[test]
    fn a_failing_item_rendered_ahead_falls_back_to_the_requested_ones() {
        let mut ahead = RenderAhead::start(
            ",batch_items:8,retain_items:0",
            crate::VirtualCollectionPolicy::default(),
        );
        // The batch after rows 44 and 45 reaches row 50, which fails: rows 44 to 49 render
        // and 50 fails (seven renders), then only the requested rows render again (two).
        // No failure is reported.
        assert_eq!(ahead.request(44..46), 9);
        assert_eq!(ahead.realized(), vec![44, 45]);
        // Within the engine's session, before the lifecycle starts another, the recovered
        // failure is not the last one either.
        let previous = ahead
            .lifecycle
            .root()
            .unwrap()
            .virtual_collection_items(&ahead.id)
            .unwrap()
            .clone();
        ahead.engine.begin_execution_session();
        let realized = ahead
            .engine
            .realize_virtual_collection(&ahead.id, &BTreeSet::from([46, 47]), &previous)
            .unwrap();
        assert_eq!(realized.keys().copied().collect::<Vec<_>>(), vec![46, 47]);
        assert_eq!(ahead.engine.last_failed_timing(), None);
        assert_eq!(ahead.engine.last_failed_component(), None);
        // A failing requested row still fails the realization.
        ahead
            .runtime
            .borrow()
            .virtual_requests
            .request_target(ahead.id.clone(), 50..51);
        assert!(
            ahead
                .lifecycle
                .realize_virtual_requests(&mut ahead.engine)
                .is_err()
        );
    }

    #[test]
    fn the_host_policy_applies_unless_a_collection_sets_its_own() {
        let policy = crate::VirtualCollectionPolicy {
            overscan_items: Some(3),
            batch_items: Some(4),
            retain_items: Some(0),
        };
        let mut ahead = RenderAhead::start("", policy);
        // Three items of overscan replace the collection's own 24 pixels.
        let spec = ahead
            .lifecycle
            .root()
            .unwrap()
            .virtual_collection_spec(&ahead.id)
            .unwrap()
            .clone();
        assert!((spec.overdraw_pixels - 72.0).abs() < f64::EPSILON);
        let first = *ahead.realized().last().unwrap() + 1;
        // The Host's batch of four: the needed row and three more.
        assert_eq!(ahead.request(first..first + 1), 4);

        let mut own = RenderAhead::start(",overscan_items:0,batch_items:1", policy);
        let spec = own
            .lifecycle
            .root()
            .unwrap()
            .virtual_collection_spec(&own.id)
            .unwrap()
            .clone();
        assert!(spec.overdraw_pixels.abs() < f64::EPSILON);
        let first = *own.realized().last().unwrap() + 1;
        assert_eq!(own.request(first..first + 1), 1);
    }

    #[test]
    fn render_ahead_settings_are_validated() {
        for (extra, field) in [
            (",batch_items:0", "batch_items"),
            (",retain_items:-1", "retain_items"),
            (",overscan_items:1001", "overscan_items"),
            (",batch_items:2.5", "batch_items"),
        ] {
            let mut engine = RuntimeEngine::new();
            let compiled = engine
                .compile(&RENDER_AHEAD.replace("EXTRA", extra))
                .unwrap();
            let schema = ComponentStateSchema::new(BTreeMap::from([(
                "version".to_owned(),
                StateField::new(ValueSchema::integer(), UiValue::Integer(0)),
            )]))
            .unwrap();
            let mut lifecycle = ScriptLifecycle::new(
                compiled,
                Rc::new(RefCell::new(UiRuntimeState::new())),
                ComponentInstancePath::root("App", "invalid"),
                Some("main".to_owned()),
                BTreeMap::new(),
                &schema,
            )
            .unwrap();
            let error = lifecycle.start(&mut engine).unwrap_err().to_string();
            assert!(error.contains(field), "{extra}: {error}");
        }
    }

    #[test]
    fn controlled_rerender_synchronously_rebuilds_the_retained_virtual_window() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn render_item(ctx, payload) {
                        text(`${payload.item.label}:${payload.item.active}`).with_key(payload.key)
                    }
                    fn view(ctx) {
                        let active = ctx.get_state("active");
                        let data = [];
                        for index in 0..32 {
                            data.push(#{
                                key: `row-${index}`,
                                label: `Row ${index}`,
                                active: index == active,
                            });
                        }
                        virtual_collection(#{
                            key: "rows", label: "Rows", data: data,
                            estimated_height: 24, height: 96,
                            overdraw_pixels: 24, alignment: "top", follow_tail: false,
                            batch_items: 1, retain_items: 0,
                            reveal_key: `row-${active}`,
                        }, Fn("render_item"))
                    }
                "#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let path = ComponentInstancePath::root("App", "root");
        let schema = ComponentStateSchema::new(BTreeMap::from([(
            "active".to_owned(),
            StateField::new(ValueSchema::integer(), UiValue::Integer(0)),
        )]))
        .unwrap();
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            path.clone(),
            Some("main".to_owned()),
            BTreeMap::new(),
            &schema,
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        let id = engine
            .virtual_collection_ids_in_scope(&path)
            .into_iter()
            .next()
            .unwrap();

        runtime
            .borrow()
            .virtual_requests
            .request(id.clone(), 14..20);
        assert!(lifecycle.realize_virtual_requests(&mut engine).unwrap());
        assert_eq!(
            lifecycle
                .root()
                .unwrap()
                .virtual_collection_items(&id)
                .unwrap()
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            (14..20).collect::<Vec<_>>()
        );

        runtime
            .borrow_mut()
            .set_component_state_from_host(&path, "active", UiValue::Integer(16))
            .unwrap();
        assert!(lifecycle.render_dirty(&mut engine).unwrap());
        let realized = lifecycle
            .root()
            .unwrap()
            .virtual_collection_items(&id)
            .unwrap();
        for index in 14..20 {
            assert!(
                realized.contains_key(&index),
                "retained row {index} is missing"
            );
        }
        assert!(matches!(
            realized[&16].kind(),
            crate::UiNodeKind::Text { text } if text == "Row 16:true"
        ));
        assert!(matches!(
            realized[&15].kind(),
            crate::UiNodeKind::Text { text } if text == "Row 15:false"
        ));
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
    fn invalid_native_semantics_reject_the_candidate_before_commit() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"fn view(ctx) {
                    text("Mystery").accessibility_role("mystery_widget")
                }"#,
            )
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
        assert!(matches!(
            lifecycle.start(&mut engine),
            Err(LifecycleError::Accessibility(
                crate::AccessibilityError::UnsupportedRole(role)
            )) if role == "mystery_widget"
        ));
        assert!(lifecycle.retained().is_empty());
        assert!(lifecycle.semantics().nodes().next().is_none());
    }

    #[test]
    fn layer_budget_rejects_candidate_before_commit() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn view(ctx) {
                        box([
                            layer(text("first"), #{ id: "first", placement: "top_right" }),
                            layer(text("second"), #{ id: "second", placement: "bottom_right" })
                        ])
                    }
                "#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        runtime.borrow_mut().budgets.layers = 1;
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
                    resource: "layers",
                    actual: 2,
                    limit: 1,
                }
            ))
        ));
        assert!(lifecycle.retained().is_empty());
    }

    #[test]
    fn canvas_scene_budget_is_independent_from_command_budget() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn view(ctx) {
                        box([
                            canvas(canvas_scene([canvas_rect("a", 0.0, 0.0, 1.0, 1.0, rgb(0))])),
                            canvas(canvas_scene([canvas_rect("b", 0.0, 0.0, 1.0, 1.0, rgb(0))]))
                        ])
                    }
                "#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        {
            let mut runtime = runtime.borrow_mut();
            runtime.budgets.canvas_scenes = 1;
            runtime.budgets.canvas_commands = 10;
        }
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
                    resource: "canvas_scenes",
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
    fn hot_reload_keeps_a_disabled_action_disabled() {
        let source = r#"
            fn init(ctx) { ctx.register_action("document.save", Fn("save")); }
            fn save(ctx, payload) {}
            fn disable(ctx, payload) { ctx.set_action_enabled("document.save", false); }
            fn view(ctx) { text("actions") }
        "#;
        let mut engine = RuntimeEngine::new();
        let active = engine.compile(source).unwrap();
        let disable = engine.callback(&active, "disable").unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let mut lifecycle = ScriptLifecycle::new(
            active,
            Rc::clone(&runtime),
            ComponentInstancePath::root("App", "root"),
            None,
            BTreeMap::new(),
            &state_schema(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        let id = crate::ActionId::parse("document.save").unwrap();
        assert_eq!(runtime.borrow().actions.is_enabled(&id), Some(true));
        let _ = lifecycle
            .invoke_callback(&engine, &disable, UiValue::Null)
            .unwrap();
        assert_eq!(runtime.borrow().actions.is_enabled(&id), Some(false));

        let candidate = engine.compile(source).unwrap();
        let generation = candidate.generation();
        lifecycle
            .reload(&mut engine, candidate, &state_schema())
            .unwrap();
        let mut runtime = runtime.borrow_mut();
        assert_eq!(runtime.actions.is_enabled(&id), Some(false));
        runtime.actions.set_enabled(&id, true).unwrap();
        let invocation = runtime.actions.dispatch(&id, UiValue::Null).unwrap();
        assert_eq!(invocation.callback.generation(), generation);
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
                Ok(TaskWork::new(move || Ok(input)))
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
        let compiled = engine.compile(&STREAM_COMPONENT_APP.replace(
            "fn view(ctx) { render_component(\"test/stream\", #{ key: \"stream\" }) }",
            "fn view(ctx) { if ctx.get_state(\"phase\")==\"hidden\" {text(\"hidden\")} else {render_component(\"test/stream\", #{key:\"stream\"})} }",
        )).unwrap();
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
        let mut already_drained = None;
        loop {
            let deliveries = runtime
                .borrow_mut()
                .subscriptions
                .drain(lifecycle.generation());
            for delivery in deliveries {
                already_drained = Some(delivery.clone());
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
            runtime
                .borrow()
                .component_state
                .get(&path.child("StreamProbe", "stream"), "phase"),
            Some(&UiValue::String("second".to_owned()))
        );
        let late = already_drained.unwrap();
        runtime
            .borrow_mut()
            .set_component_state_from_host(&path, "phase", UiValue::String("hidden".to_owned()))
            .unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();
        let callback = late.callback.clone();
        let _ = lifecycle.invoke_async_delivery(&engine, late).unwrap();
        assert!(
            lifecycle
                .invoke_callback(&engine, &callback, UiValue::Null)
                .is_err()
        );
    }

    #[test]
    fn subscription_start_outside_a_declarative_effect_is_rejected() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn init(ctx) {
                        ctx.start_subscription(
                            "app.stream", "watch", (),
                            Fn("received"), Fn("failed"), #{ delivery: "all" }
                        );
                    }
                    fn received(ctx, value) { () }
                    fn failed(ctx, error) { () }
                    fn view(ctx) { text("never mounted") }
                "#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            runtime,
            ComponentInstancePath::root("App", "root"),
            None,
            BTreeMap::new(),
            &state_schema(),
        )
        .unwrap();
        let error = lifecycle.start(&mut engine).unwrap_err().to_string();
        assert!(
            error.contains("subscriptions must be started by a declarative component effect"),
            "unexpected error: {error}"
        );
    }

    fn assert_table_panel_marker(
        lifecycle: &ScriptLifecycle,
        runtime: &UiRuntimeState,
        track: bool,
        internal: bool,
        count: i32,
    ) {
        let root = lifecycle.root().unwrap();
        let panel = if track {
            root
        } else {
            let UiNodeKind::Box { children } = root.kind() else {
                panic!("expected track")
            };
            &children[0]
        };
        assert_eq!(
            panel.style().base.width,
            Some(crate::LayoutLength::Definite(
                crate::Length::pixels(123.0).unwrap()
            ))
        );
        if track {
            let Some(crate::table_layout::TableLayout::Columns(columns)) = panel.table_layout()
            else {
                panic!("track marker lost: internal={internal}, count={count}")
            };
            let plan = crate::table_layout::resolve(columns.as_slice(), 400.0, &runtime.signals);
            assert_eq!(
                plan.widths,
                vec![120.0 + if internal { f64::from(count) } else { 0.0 }, 140.0]
            );
        } else {
            assert_eq!(
                panel.table_column(),
                Some(if internal {
                    usize::try_from(count % 2).unwrap()
                } else {
                    0
                })
            );
        }
        let UiNodeKind::Box { children } = panel.kind() else {
            panic!("expected panel")
        };
        assert!(
            matches!(children[0].kind(), UiNodeKind::Text { text } if text.as_str() == count.to_string())
        );
    }

    fn assert_table_modifier_replay(track: bool) {
        for internal in [true, false] {
            let modifier = if track {
                ".with_table_track(columns(120), [])"
            } else {
                ".with_table_column(0)"
            };
            let internal_modifier = if track {
                ".with_table_track(columns(120 + count), [])"
            } else {
                ".with_table_column(count % 2)"
            };
            let panel = format!(
                "render_component(\"tests/table_panel\", #{{key:\"panel\"}}){}.with_style(style().width(px(123)))",
                if internal { "" } else { modifier }
            );
            let source = r#"
                define_component(#{
                    metadata:#{id:"tests/table_panel","export":"Panel",version:"0.1.8",
                        runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{}},
                    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
                        state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}},
                        events:#{},slots:#{},parts:[]},render:Fn("render_panel")
                });
                fn columns(width) { [#{key:"a",width:#{kind:"fixed",value:width}},
                    #{key:"b",width:#{kind:"fixed",value:140}}] }
                fn render_panel(ctx, props) {
                    let count = ctx.get_state("count");
                    if count < 0 { throw "rejected panel candidate"; }
                    column([text(`${count}`)])INTERNAL
                }
                fn view(ctx) { VIEW }
            "#
            .replace("INTERNAL", if internal { internal_modifier } else { "" })
            .replace(
                "VIEW",
                &if track { panel } else { format!("column([{panel}]).with_table_track(columns(120), [])") },
            );
            let mut engine = RuntimeEngine::new();
            let compiled = engine
                .compile_named("table-modifier.rhai", &source)
                .unwrap();
            let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
            let root = ComponentInstancePath::root("App", format!("table-{track}-{internal}"));
            let component = root.child("Panel", "panel");
            let mut lifecycle = ScriptLifecycle::new(
                compiled,
                Rc::clone(&runtime),
                root,
                Some("main".into()),
                BTreeMap::new(),
                &ComponentStateSchema::default(),
            )
            .unwrap();
            lifecycle.start(&mut engine).unwrap();
            assert_table_panel_marker(&lifecycle, &runtime.borrow(), track, internal, 0);
            for count in [1, 2] {
                runtime
                    .borrow_mut()
                    .set_component_state_from_host(
                        &component,
                        "count",
                        UiValue::Integer(i64::from(count)),
                    )
                    .unwrap();
                assert_eq!(
                    runtime.borrow().dirty_components(),
                    &BTreeSet::from([component.clone()])
                );
                assert!(lifecycle.render_dirty(&mut engine).unwrap());
                assert_table_panel_marker(&lifecycle, &runtime.borrow(), track, internal, count);
            }
            runtime
                .borrow_mut()
                .set_component_state_from_host(&component, "count", UiValue::Integer(-1))
                .unwrap();
            assert!(lifecycle.render_dirty(&mut engine).is_err());
            assert_table_panel_marker(&lifecycle, &runtime.borrow(), track, internal, 2);
            runtime
                .borrow_mut()
                .set_component_state_from_host(&component, "count", UiValue::Integer(3))
                .unwrap();
            assert!(lifecycle.render_dirty(&mut engine).unwrap());
            assert_table_panel_marker(&lifecycle, &runtime.borrow(), track, internal, 3);
        }
    }

    #[test]
    fn table_modifiers_replay_track_across_incremental_updates_and_failed_candidates() {
        assert_table_modifier_replay(true);
    }

    #[test]
    fn table_modifiers_replay_column_across_incremental_updates_and_failed_candidates() {
        assert_table_modifier_replay(false);
    }
}
