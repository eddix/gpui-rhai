use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;
use std::time::{Duration, Instant};

use rhai::{
    AST, Dynamic, Engine, EvalAltResult, FnPtr, FuncArgs, FuncRegistration, ImmutableString, Map,
    Module, ModuleResolver, Position, Scope,
};
use thiserror::Error;

use crate::animation::register_animation_api;
use crate::asset::{AssetId, ImageDecodeHandle, asset_id_from_script};
use crate::component::{ComponentExportCollector, ComponentExportError, ComponentRegistry};
use crate::context::{UiContext, register_ui_context_api};
use crate::node::{
    column_node, directional_image_node, dropdown_node, error_boundary_node, image_node,
    lazy_error_boundary_node, overlay_node, row_node, text_node, toast_host_node,
    virtual_list_node,
};
use crate::primitive::{PrimitiveDescriptor, PrimitiveError, PrimitiveHandler, PrimitiveRegistry};
use crate::style::register_style_api;
use crate::text_input::{TextInputPrimitiveHandler, text_input_primitive_descriptor};
use crate::value::OpaqueHandle;
use crate::virtual_list::register_virtual_list_api;
use crate::{
    ComponentInstancePath, ComponentStateSchema, EventSchema, ExecutionPhase, ModuleId,
    RenderStateTransaction, UiNode, UiRuntimeState,
};
use crate::{SubscriptionHandle, TaskHandle};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScriptGeneration(u64);

impl ScriptGeneration {
    #[must_use]
    pub fn initial() -> Self {
        Self(1)
    }

    #[must_use]
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl fmt::Display for ScriptGeneration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionOperation {
    Compile,
    Render,
    Lifecycle(String),
    Callback(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionTiming {
    pub operation: ExecutionOperation,
    pub source: String,
    pub duration: Duration,
    pub slow: bool,
    pub succeeded: bool,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("failed to compile Rhai UI source: {0}")]
    Compile(#[source] Box<EvalAltResult>),
    #[error("failed to evaluate Rhai UI source: {0}")]
    Evaluate(#[source] Box<EvalAltResult>),
    #[error("callback `{name}` is invalid: {source}")]
    CallbackDefinition {
        name: String,
        #[source]
        source: Box<EvalAltResult>,
    },
    #[error(
        "callback `{name}` belongs to script generation {callback_generation}, but current generation is {current_generation}"
    )]
    StaleCallback {
        name: String,
        callback_generation: ScriptGeneration,
        current_generation: ScriptGeneration,
    },
    #[error("component runtime failed: {0}")]
    ComponentRuntime(String),
    #[error("invalid script import: {0}")]
    Import(String),
}

#[derive(Clone)]
pub struct CompiledUi {
    ast: AST,
    generation: ScriptGeneration,
}

#[allow(deprecated)]
pub(crate) type ScriptNativeContext = Rc<rhai::NativeCallContextStore>;

#[derive(Clone, Debug)]
pub struct ScriptCallback {
    function: FnPtr,
    generation: ScriptGeneration,
    component: Option<ComponentInstancePath>,
    events: BTreeMap<String, EventSchema>,
    #[allow(deprecated)]
    native_context: Option<ScriptNativeContext>,
}

impl PartialEq for ScriptCallback {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
            && self.generation == other.generation
            && self.component == other.component
    }
}

impl Eq for ScriptCallback {}

impl ScriptCallback {
    #[must_use]
    pub fn name(&self) -> &str {
        self.function.fn_name()
    }

    #[must_use]
    pub fn generation(&self) -> ScriptGeneration {
        self.generation
    }

    pub(crate) fn from_fn_ptr(function: FnPtr, generation: ScriptGeneration) -> Self {
        Self {
            function,
            generation,
            component: None,
            events: BTreeMap::new(),
            native_context: None,
        }
    }

    pub(crate) fn bind_generation(&mut self, generation: ScriptGeneration) {
        self.generation = generation;
    }

    pub(crate) fn bind_component_if_unset(
        &mut self,
        component: ComponentInstancePath,
        events: BTreeMap<String, EventSchema>,
    ) {
        if self.component.is_none() {
            self.component = Some(component);
            self.events = events;
        }
    }

    pub(crate) fn bind_native_context_if_unset(&mut self, context: ScriptNativeContext) {
        if self.native_context.is_none() {
            self.native_context = Some(context);
        }
    }

    pub(crate) fn component(&self) -> Option<&ComponentInstancePath> {
        self.component.as_ref()
    }

    pub(crate) fn events(&self) -> &BTreeMap<String, EventSchema> {
        &self.events
    }

    pub(crate) fn native_context(&self) -> Option<&ScriptNativeContext> {
        self.native_context.as_ref()
    }
}

impl CompiledUi {
    #[must_use]
    pub fn generation(&self) -> ScriptGeneration {
        self.generation
    }

    #[must_use]
    pub fn has_function(&self, name: &str, parameter_count: usize) -> bool {
        self.ast
            .iter_functions()
            .any(|function| function.name == name && function.params.len() == parameter_count)
    }
}

struct ActiveComponentRender {
    root_context: UiContext,
    stack: Vec<ComponentInstancePath>,
    contexts: Vec<UiContext>,
    seen: BTreeSet<ComponentInstancePath>,
    positions: BTreeMap<String, usize>,
    transaction: RenderStateTransaction,
    generation: ScriptGeneration,
    state_snapshot: crate::StateStore,
    event_handlers: BTreeMap<(ComponentInstancePath, String), ScriptCallback>,
}

pub struct RuntimeEngine {
    engine: Engine,
    generation: ScriptGeneration,
    component_exports: ComponentExportCollector,
    evaluation_generation: Rc<Cell<ScriptGeneration>>,
    primitives: PrimitiveRegistry,
    primitive_modules: BTreeMap<String, Module>,
    timings: RefCell<Vec<ExecutionTiming>>,
    slow_threshold: Duration,
    component_render: Rc<RefCell<Option<ActiveComponentRender>>>,
}

impl Default for RuntimeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeEngine {
    /// Create a runtime with all built-in primitive descriptors.
    ///
    /// # Panics
    ///
    /// Panics only if a compile-time built-in descriptor violates the same
    /// validation rules enforced for downstream custom primitives.
    #[must_use]
    pub fn new() -> Self {
        let mut engine = Engine::new();
        engine.build_type::<UiNode>();
        engine.build_type::<OpaqueHandle>();
        engine.build_type::<TaskHandle>();
        engine.build_type::<SubscriptionHandle>();
        engine.build_type::<AssetId>();
        engine.build_type::<ImageDecodeHandle>();
        register_ui_context_api(&mut engine);
        register_style_api(&mut engine);
        register_animation_api(&mut engine);
        register_virtual_list_api(&mut engine);
        let component_exports = ComponentExportCollector::new();
        component_exports.register_into(&mut engine);
        register_component_props_api(&mut engine, &component_exports);
        let component_render = Rc::new(RefCell::new(None));
        register_component_render_api(&mut engine, &component_exports, &component_render);
        let evaluation_generation = Rc::new(Cell::new(ScriptGeneration::default()));
        let primitives = PrimitiveRegistry::new();

        FuncRegistration::new("text")
            .in_global_namespace()
            .register_into_engine(&mut engine, text_node);
        FuncRegistration::new("handled")
            .in_global_namespace()
            .register_into_engine(&mut engine, || ImmutableString::from("handled"));
        FuncRegistration::new("propagate")
            .in_global_namespace()
            .register_into_engine(&mut engine, || ImmutableString::from("propagate"));
        FuncRegistration::new("column")
            .in_global_namespace()
            .register_into_engine(&mut engine, column_node);
        FuncRegistration::new("row")
            .in_global_namespace()
            .register_into_engine(&mut engine, row_node);
        FuncRegistration::new("error_boundary")
            .in_global_namespace()
            .register_into_engine(&mut engine, error_boundary_node);
        FuncRegistration::new("error_boundary_lazy")
            .in_global_namespace()
            .register_into_engine(&mut engine, lazy_error_boundary_node);
        FuncRegistration::new("asset")
            .in_global_namespace()
            .register_into_engine(&mut engine, asset_id_from_script);
        FuncRegistration::new("image")
            .in_global_namespace()
            .register_into_engine(&mut engine, image_node);
        FuncRegistration::new("directional_image")
            .in_global_namespace()
            .register_into_engine(&mut engine, directional_image_node);
        FuncRegistration::new("overlay")
            .in_global_namespace()
            .register_into_engine(&mut engine, overlay_node);
        FuncRegistration::new("dropdown")
            .in_global_namespace()
            .register_into_engine(&mut engine, dropdown_node);
        FuncRegistration::new("toast_host")
            .in_global_namespace()
            .register_into_engine(&mut engine, toast_host_node);
        FuncRegistration::new("virtual_list")
            .in_global_namespace()
            .register_into_engine(&mut engine, virtual_list_node);

        engine.set_max_call_levels(64);
        engine.set_max_expr_depths(64, 32);
        engine.set_max_operations(1_000_000);
        engine.set_max_array_size(10_000);
        engine.set_max_map_size(100_000);
        engine.set_max_string_size(1_048_576);

        let mut runtime = Self {
            engine,
            generation: ScriptGeneration::default(),
            component_exports,
            evaluation_generation,
            primitives,
            primitive_modules: BTreeMap::new(),
            timings: RefCell::new(Vec::new()),
            slow_threshold: Duration::from_millis(16),
            component_render,
        };
        runtime
            .register_primitive(
                text_input_primitive_descriptor(),
                TextInputPrimitiveHandler::default(),
            )
            .expect("built-in TextInput primitive descriptor is valid");
        runtime
    }

    /// Compile the default application entry source.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Compile`] when Rhai rejects the source.
    pub fn compile(&mut self, source: &str) -> Result<CompiledUi, RuntimeError> {
        self.compile_named("ui/main.rhai", source)
    }

    /// Compile source with an explicit diagnostic name and advance the script generation.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Compile`] when Rhai rejects the source.
    pub fn compile_named(
        &mut self,
        source_name: &str,
        source: &str,
    ) -> Result<CompiledUi, RuntimeError> {
        let started = Instant::now();
        let result = self.engine.compile(source);
        self.record_timing(
            ExecutionOperation::Compile,
            source_name,
            started,
            result.is_ok(),
        );
        let mut ast = result.map_err(|error| RuntimeError::Compile(error.into()))?;
        ast.set_source(source_name);
        let generation = self.candidate_generation();
        Ok(CompiledUi { ast, generation })
    }

    /// Compile source while eagerly resolving and embedding literal imports.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Compile`] for source, import, or module errors.
    pub fn compile_self_contained_named(
        &mut self,
        source_name: &str,
        source: &str,
    ) -> Result<CompiledUi, RuntimeError> {
        crate::extract_imports(source).map_err(|error| RuntimeError::Import(error.to_string()))?;
        let started = Instant::now();
        let result = self
            .engine
            .compile_into_self_contained(&Scope::new(), source);
        self.record_timing(
            ExecutionOperation::Compile,
            source_name,
            started,
            result.is_ok(),
        );
        let mut ast = result.map_err(RuntimeError::Compile)?;
        ast.set_source(source_name);
        Ok(CompiledUi {
            ast,
            generation: self.candidate_generation(),
        })
    }

    pub fn set_module_resolver(&mut self, resolver: impl ModuleResolver + 'static) {
        self.engine.set_module_resolver(resolver);
    }

    fn begin_component_render(
        &self,
        context: UiContext,
        generation: ScriptGeneration,
    ) -> Result<(), RuntimeError> {
        let root = context.component_path().clone();
        let mut transaction = context
            .runtime()
            .try_borrow()
            .map_err(|_| RuntimeError::ComponentRuntime("UI state is already borrowed".to_owned()))?
            .component_state
            .begin_render_scope(root.clone());
        let state_snapshot = context
            .runtime()
            .try_borrow()
            .map_err(|_| RuntimeError::ComponentRuntime("UI state is already borrowed".to_owned()))?
            .component_state
            .clone();
        let _ = transaction.retain_existing(&root);
        let mut active = self.component_render.try_borrow_mut().map_err(|_| {
            RuntimeError::ComponentRuntime("component render stack is already borrowed".to_owned())
        })?;
        *active = Some(ActiveComponentRender {
            root_context: context.clone(),
            stack: vec![root],
            contexts: vec![context],
            seen: BTreeSet::new(),
            positions: BTreeMap::new(),
            transaction,
            generation,
            state_snapshot,
            event_handlers: BTreeMap::new(),
        });
        Ok(())
    }

    fn finish_component_render(&self, commit: bool) -> Result<(), RuntimeError> {
        let active = self
            .component_render
            .try_borrow_mut()
            .map_err(|_| {
                RuntimeError::ComponentRuntime(
                    "component render stack is already borrowed".to_owned(),
                )
            })?
            .take();
        let Some(active) = active else {
            return Ok(());
        };
        if commit {
            let root = active.root_context.component_path().clone();
            let previous = active.state_snapshot.paths();
            let mut runtime = active
                .root_context
                .runtime()
                .try_borrow_mut()
                .map_err(|_| {
                    RuntimeError::ComponentRuntime("UI state is already borrowed".to_owned())
                })?;
            runtime
                .reconcile_component_lifetimes(&root, &active.seen, &previous)
                .map_err(|error| RuntimeError::ComponentRuntime(error.to_string()))?;
            runtime.replace_component_event_handlers(&root, active.event_handlers);
            runtime.component_state.commit_render(active.transaction);
        } else {
            active
                .root_context
                .runtime()
                .try_borrow_mut()
                .map_err(|_| {
                    RuntimeError::ComponentRuntime("UI state is already borrowed".to_owned())
                })?
                .component_state = active.state_snapshot;
        }
        Ok(())
    }

    /// Evaluate the compiled application's `view` function.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Evaluate`] when `view` is missing, fails, or
    /// returns a value other than [`UiNode`].
    pub fn render(&mut self, compiled: &CompiledUi) -> Result<UiNode, RuntimeError> {
        self.evaluation_generation.set(compiled.generation);
        let context = UiContext::new(
            Rc::new(RefCell::new(UiRuntimeState::new())),
            ComponentInstancePath::root("App", "root"),
            None,
            ExecutionPhase::Render,
            BTreeMap::new(),
        )
        .with_generation(compiled.generation);
        self.begin_component_render(context, compiled.generation)?;
        let started = Instant::now();
        let result = self
            .engine
            .call_fn::<UiNode>(&mut Scope::new(), &compiled.ast, "view", ());
        self.record_timing(
            ExecutionOperation::Render,
            compiled.ast.source().unwrap_or("<script>"),
            started,
            result.is_ok(),
        );
        self.finish_component_render(result.is_ok())?;
        let mut root = result.map_err(RuntimeError::Evaluate)?;
        root.bind_generation(compiled.generation);
        self.generation = compiled.generation;
        Ok(root)
    }

    /// Evaluate the compiled application's `view(ctx)` function.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Evaluate`] when `view` fails or returns a value
    /// other than [`UiNode`].
    pub fn render_with_context(
        &mut self,
        compiled: &CompiledUi,
        context: UiContext,
    ) -> Result<UiNode, RuntimeError> {
        self.evaluation_generation.set(compiled.generation);
        self.begin_component_render(context.clone(), compiled.generation)?;
        let started = Instant::now();
        let result =
            self.engine
                .call_fn::<UiNode>(&mut Scope::new(), &compiled.ast, "view", (context,));
        self.record_timing(
            ExecutionOperation::Render,
            compiled.ast.source().unwrap_or("<script>"),
            started,
            result.is_ok(),
        );
        self.finish_component_render(result.is_ok())?;
        let mut root = result.map_err(RuntimeError::Evaluate)?;
        root.bind_generation(compiled.generation);
        self.generation = compiled.generation;
        Ok(root)
    }

    /// Invoke an optional one-argument lifecycle function.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Evaluate`] when the function exists but fails.
    pub fn call_optional_lifecycle(
        &self,
        compiled: &CompiledUi,
        function: &str,
        context: UiContext,
    ) -> Result<bool, RuntimeError> {
        if !compiled.has_function(function, 1) {
            return Ok(false);
        }
        self.evaluation_generation.set(compiled.generation);
        let started = Instant::now();
        let result =
            self.engine
                .call_fn::<Dynamic>(&mut Scope::new(), &compiled.ast, function, (context,));
        self.record_timing(
            ExecutionOperation::Lifecycle(function.to_owned()),
            compiled.ast.source().unwrap_or("<script>"),
            started,
            result.is_ok(),
        );
        let _ = result.map_err(RuntimeError::Evaluate)?;
        Ok(true)
    }

    /// Decode optional root `state_schema()` from an application entry.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Evaluate`] when the function fails or cannot be
    /// decoded as [`ComponentStateSchema`].
    pub fn root_state_schema(
        &self,
        compiled: &CompiledUi,
    ) -> Result<ComponentStateSchema, RuntimeError> {
        if !compiled.has_function("state_schema", 0) {
            return Ok(ComponentStateSchema::default());
        }
        let raw: Dynamic = self
            .engine
            .call_fn(&mut Scope::new(), &compiled.ast, "state_schema", ())
            .map_err(RuntimeError::Evaluate)?;
        rhai::serde::from_dynamic(&raw).map_err(RuntimeError::Evaluate)
    }

    /// Create a generation-bound callback for a script function.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::CallbackDefinition`] when the function name is
    /// not a valid Rhai function pointer name.
    pub fn callback(
        &self,
        compiled: &CompiledUi,
        function_name: &str,
    ) -> Result<ScriptCallback, RuntimeError> {
        let function =
            FnPtr::new(function_name).map_err(|source| RuntimeError::CallbackDefinition {
                name: function_name.to_owned(),
                source,
            })?;
        Ok(ScriptCallback::from_fn_ptr(function, compiled.generation))
    }

    /// Invoke a callback only when it belongs to the current compiled generation.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::StaleCallback`] for obsolete callbacks and
    /// [`RuntimeError::Evaluate`] when the Rhai function fails.
    pub fn invoke_callback(
        &self,
        compiled: &CompiledUi,
        callback: &ScriptCallback,
        args: impl FuncArgs,
    ) -> Result<Dynamic, RuntimeError> {
        if !self.is_current(callback.generation) || callback.generation != compiled.generation {
            return Err(RuntimeError::StaleCallback {
                name: callback.name().to_owned(),
                callback_generation: callback.generation,
                current_generation: self.generation,
            });
        }

        self.evaluation_generation.set(compiled.generation);
        let started = Instant::now();
        #[allow(deprecated)]
        let result = if let Some(stored) = callback.native_context.as_ref() {
            let context = stored.create_context(self.engine());
            callback.function.call_within_context(&context, args)
        } else {
            callback.function.call(self.engine(), &compiled.ast, args)
        };
        self.record_timing(
            ExecutionOperation::Callback(callback.name().to_owned()),
            compiled.ast.source().unwrap_or("<script>"),
            started,
            result.is_ok(),
        );
        result.map_err(RuntimeError::Evaluate)
    }

    #[must_use]
    pub fn is_current(&self, generation: ScriptGeneration) -> bool {
        generation == self.generation
    }

    #[must_use]
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Return a snapshot of components exported through this engine.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentExportError`] if the export registry was poisoned by
    /// a prior panic.
    pub fn component_exports(&self) -> Result<ComponentRegistry, ComponentExportError> {
        self.component_exports.snapshot()
    }

    /// Clear component exports before evaluating a hot-reload candidate.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentExportError`] if the collector is unavailable.
    pub fn clear_component_exports(&self) -> Result<(), ComponentExportError> {
        self.component_exports.clear()
    }

    /// Restore a prior export snapshot after a rejected candidate.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentExportError`] if the collector is unavailable.
    pub fn restore_component_exports(
        &self,
        registry: ComponentRegistry,
    ) -> Result<(), ComponentExportError> {
        self.component_exports.replace(registry)
    }

    /// Register a namespaced Rust custom primitive and its Rhai constructor.
    ///
    /// # Errors
    ///
    /// Returns [`PrimitiveError`] when the descriptor conflicts or is invalid.
    pub fn register_primitive(
        &mut self,
        descriptor: PrimitiveDescriptor,
        handler: impl PrimitiveHandler + 'static,
    ) -> Result<(), PrimitiveError> {
        self.primitives.register(descriptor.clone(), handler)?;

        let namespace = descriptor.id.namespace().to_owned();
        let export = descriptor.export.clone();
        let id = descriptor.id;
        let primitives = self.primitives.clone();
        let generation = Rc::clone(&self.evaluation_generation);
        let module = self.primitive_modules.entry(namespace.clone()).or_default();
        FuncRegistration::new(export).set_into_module(
            module,
            move |mut props: Map| -> Result<UiNode, Box<EvalAltResult>> {
                let key = props
                    .remove("key")
                    .map(|value| {
                        if value.is::<ImmutableString>() {
                            Ok(value.cast::<ImmutableString>().to_string())
                        } else {
                            Err(Box::new(EvalAltResult::ErrorRuntime(
                                "primitive key must be a string".into(),
                                Position::NONE,
                            )))
                        }
                    })
                    .transpose()?;
                primitives
                    .create_node(&id, key, &props, generation.get())
                    .map_err(|error| {
                        Box::new(EvalAltResult::ErrorRuntime(
                            error.to_string().into(),
                            Position::NONE,
                        ))
                    })
            },
        );
        self.engine
            .register_static_module(namespace, module.clone().into());
        Ok(())
    }

    #[must_use]
    pub fn primitive_registry(&self) -> PrimitiveRegistry {
        self.primitives.clone()
    }

    pub fn set_slow_threshold(&mut self, threshold: Duration) {
        self.slow_threshold = threshold;
    }

    #[must_use]
    pub fn take_timings(&self) -> Vec<ExecutionTiming> {
        std::mem::take(&mut *self.timings.borrow_mut())
    }

    fn record_timing(
        &self,
        operation: ExecutionOperation,
        source: &str,
        started: Instant,
        succeeded: bool,
    ) {
        let duration = started.elapsed();
        self.timings.borrow_mut().push(ExecutionTiming {
            operation,
            source: source.to_owned(),
            duration,
            slow: duration >= self.slow_threshold,
            succeeded,
        });
    }

    fn candidate_generation(&self) -> ScriptGeneration {
        if self.generation == ScriptGeneration::default() {
            ScriptGeneration::initial()
        } else {
            self.generation.next()
        }
    }
}

fn register_component_props_api(engine: &mut Engine, exports: &ComponentExportCollector) {
    let exports = exports.clone();
    FuncRegistration::new("component_props")
        .in_global_namespace()
        .register_into_engine(
            engine,
            move |id: ImmutableString, props: Map| -> Result<Map, Box<EvalAltResult>> {
                let id = ModuleId::parse(id.to_string())
                    .map_err(|error| Box::new(component_props_error(error.to_string())))?;
                let registry = exports
                    .snapshot()
                    .map_err(|error| Box::new(component_props_error(error.to_string())))?;
                let component = registry.get(&id).ok_or_else(|| {
                    Box::new(component_props_error(format!(
                        "component `{id}` is not exported"
                    )))
                })?;
                let key = props.get("key").and_then(|value| {
                    value
                        .is::<ImmutableString>()
                        .then(|| value.clone_cast::<ImmutableString>().to_string())
                });
                component
                    .invoke(key, props)
                    .map(|invocation| invocation.props)
                    .map_err(|error| Box::new(component_props_error(error.to_string())))
            },
        );
}

fn component_props_error(message: String) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(message.into(), Position::NONE)
}

fn register_component_render_api(
    engine: &mut Engine,
    exports: &ComponentExportCollector,
    active: &Rc<RefCell<Option<ActiveComponentRender>>>,
) {
    let exports = exports.clone();
    let active = Rc::clone(active);
    FuncRegistration::new("component_render")
        .in_global_namespace()
        .register_into_engine(
            engine,
            move |call: rhai::NativeCallContext<'_>,
                  id: ImmutableString,
                  props: Map,
                  render: FnPtr|
                  -> Result<UiNode, Box<EvalAltResult>> {
                let (component, invocation) = resolve_component_invocation(&exports, id, props)?;
                let event_callbacks = component_event_callbacks(&component, &invocation.props);
                let caller_callbacks = component_callback_names(&invocation.props);
                let part_styles = component_part_styles(&invocation.props);
                let (path, context, caller_context) =
                    enter_component_render(&call, &component, &invocation, &active)?;
                let result =
                    render.call_within_context::<UiNode>(&call, (context, invocation.props));
                #[allow(deprecated)]
                let native_context = Rc::new(call.store_data());
                register_component_event_callbacks(
                    &active,
                    &path,
                    &caller_context,
                    event_callbacks,
                    &native_context,
                )?;
                drop(render);
                leave_component_render(&active)?;
                let mut node = result?;
                node = node.with_part_styles(part_styles);
                node.bind_callback_scope_by_name(
                    &caller_callbacks,
                    caller_context.component_path(),
                    caller_context.event_schemas(),
                    Some(&native_context),
                );
                node.bind_component_scope(&path, &component.schema.events, Some(&native_context));
                Ok(node)
            },
        );
}

fn resolve_component_invocation(
    exports: &ComponentExportCollector,
    id: ImmutableString,
    props: Map,
) -> Result<(crate::ComponentDefinition, crate::ComponentInvocation), Box<EvalAltResult>> {
    let id: String = id.into();
    let id =
        ModuleId::parse(id).map_err(|error| Box::new(component_props_error(error.to_string())))?;
    let registry = exports
        .snapshot()
        .map_err(|error| Box::new(component_props_error(error.to_string())))?;
    let component = registry.get(&id).cloned().ok_or_else(|| {
        Box::new(component_render_error(format!(
            "component `{id}` is not exported"
        )))
    })?;
    let key = props.get("key").and_then(|value| {
        value
            .is::<ImmutableString>()
            .then(|| value.clone_cast::<ImmutableString>().to_string())
    });
    let invocation = component
        .invoke(key, props)
        .map_err(|error| Box::new(component_render_error(error.to_string())))?;
    Ok((component, invocation))
}

fn enter_component_render(
    call: &rhai::NativeCallContext<'_>,
    component: &crate::ComponentDefinition,
    invocation: &crate::ComponentInvocation,
    shared: &Rc<RefCell<Option<ActiveComponentRender>>>,
) -> Result<(ComponentInstancePath, UiContext, UiContext), Box<EvalAltResult>> {
    let mut guard = shared.try_borrow_mut().map_err(|_| {
        Box::new(component_render_error(
            "component render stack is already borrowed",
        ))
    })?;
    let active = guard.as_mut().ok_or_else(|| {
        Box::new(component_render_error(
            "component_render may run only inside view",
        ))
    })?;
    let caller_context = active.contexts.last().cloned().ok_or_else(|| {
        Box::new(component_render_error(
            "component render context stack has no caller",
        ))
    })?;
    let parent = active.stack.last().cloned().ok_or_else(|| {
        Box::new(component_render_error(
            "component render stack has no application root",
        ))
    })?;
    let key = invocation.key.clone().unwrap_or_else(|| {
        let callsite = format!(
            "{}@{}",
            call.call_source().unwrap_or("<script>"),
            call.call_position()
        );
        let occurrence = active
            .positions
            .entry(format!("{parent}/{}/{callsite}", component.metadata.export))
            .or_default();
        let key = format!("{callsite}#{occurrence}");
        *occurrence = occurrence.saturating_add(1);
        key
    });
    let path = parent.child(&component.metadata.export, key);
    if !active.seen.insert(path.clone()) {
        return Err(Box::new(component_render_error(format!(
            "duplicate component instance path `{path}`; add a stable key"
        ))));
    }
    active
        .transaction
        .mount(path.clone(), &component.schema.state)
        .map_err(|error| Box::new(component_render_error(error.to_string())))?;
    active
        .root_context
        .runtime()
        .try_borrow_mut()
        .map_err(|_| Box::new(component_render_error("UI state is already borrowed")))?
        .component_state
        .mount_instance(path.clone(), &component.schema.state)
        .map_err(|error| Box::new(component_render_error(error.to_string())))?;
    let context = active
        .root_context
        .for_component(path.clone(), component.schema.events.clone())
        .with_generation(active.generation);
    active.stack.push(path.clone());
    active.contexts.push(context.clone());
    Ok((path, context, caller_context))
}

fn leave_component_render(
    active: &Rc<RefCell<Option<ActiveComponentRender>>>,
) -> Result<(), Box<EvalAltResult>> {
    let mut guard = active.try_borrow_mut().map_err(|_| {
        Box::new(component_render_error(
            "component render stack is already borrowed",
        ))
    })?;
    if let Some(active) = guard.as_mut() {
        active.stack.pop();
        active.contexts.pop();
    }
    Ok(())
}

fn component_event_callbacks(
    component: &crate::ComponentDefinition,
    props: &Map,
) -> Vec<(String, FnPtr)> {
    component
        .schema
        .events
        .keys()
        .filter_map(|event| {
            props
                .get(format!("on_{event}").as_str())
                .filter(|value| value.is::<FnPtr>())
                .map(|value| (event.clone(), value.clone_cast::<FnPtr>()))
        })
        .collect()
}

fn component_callback_names(props: &Map) -> BTreeSet<String> {
    props
        .values()
        .filter(|value| value.is::<FnPtr>())
        .map(|value| value.clone_cast::<FnPtr>().fn_name().to_owned())
        .collect()
}

fn component_part_styles(props: &Map) -> BTreeMap<String, crate::Style> {
    props
        .get("part_styles")
        .filter(|value| value.is::<Map>())
        .map(Dynamic::clone_cast::<Map>)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(name, value)| {
            value
                .is::<crate::Style>()
                .then(|| (name.to_string(), value.cast::<crate::Style>()))
        })
        .collect()
}

fn register_component_event_callbacks(
    active: &Rc<RefCell<Option<ActiveComponentRender>>>,
    component: &ComponentInstancePath,
    caller: &UiContext,
    callbacks: Vec<(String, FnPtr)>,
    native_context: &ScriptNativeContext,
) -> Result<(), Box<EvalAltResult>> {
    let mut guard = active.try_borrow_mut().map_err(|_| {
        Box::new(component_render_error(
            "component render stack is already borrowed",
        ))
    })?;
    let active = guard.as_mut().ok_or_else(|| {
        Box::new(component_render_error(
            "component_render may run only inside view",
        ))
    })?;
    for (event, function) in callbacks {
        let mut callback = ScriptCallback::from_fn_ptr(function, active.generation);
        callback.bind_component_if_unset(
            caller.component_path().clone(),
            caller.event_schemas().clone(),
        );
        callback.bind_native_context_if_unset(Rc::clone(native_context));
        active
            .event_handlers
            .insert((component.clone(), event), callback);
    }
    Ok(())
}

fn component_render_error(message: impl Into<String>) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(Dynamic::from(message.into()), Position::NONE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_builds_a_declarative_tree() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn view() {
                        column([
                            text("Hello").with_key("greeting"),
                            text("from Rhai")
                        ])
                    }
                "#,
            )
            .expect("script should compile");

        let root = runtime.render(&compiled).expect("view should render");
        assert_eq!(
            root.source().map(|source| source.module.as_str()),
            Some("ui/main.rhai")
        );
        let crate::UiNodeKind::Container { children } = root.kind() else {
            panic!("column must render a container");
        };
        assert_eq!(children.len(), 2);
        assert_eq!(
            children[0].key().map(crate::NodeKey::as_str),
            Some("greeting")
        );
        assert!(children.iter().all(|child| child.source().is_some()));
    }

    #[test]
    fn self_contained_compile_rejects_dynamic_imports_before_evaluation() {
        let mut runtime = RuntimeEngine::new();
        assert!(matches!(
            runtime.compile_self_contained_named(
                "ui/dynamic_import.rhai",
                "let path = \"components/button\"; import path as button;",
            ),
            Err(RuntimeError::Import(_))
        ));
    }

    #[test]
    fn script_builds_an_error_boundary() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn view() {
                        error_boundary(text("content"), text("fallback"))
                    }
                "#,
            )
            .unwrap();
        let root = runtime.render(&compiled).unwrap();
        assert!(matches!(
            root.kind(),
            crate::UiNodeKind::ErrorBoundary { .. }
        ));
    }

    #[test]
    fn lazy_error_boundary_catches_rhai_evaluation_failure() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn broken() { throw "boom"; }
                    fn fallback() { text("fallback") }
                    fn view() { error_boundary_lazy(Fn("broken"), Fn("fallback")) }
                "#,
            )
            .unwrap();
        let root = runtime.render(&compiled).unwrap();
        assert!(matches!(root.kind(), crate::UiNodeKind::Text { text } if text == "fallback"));
        assert!(matches!(
            root.attributes().get("boundary_error"),
            Some(crate::UiValue::String(message)) if message.contains("boom")
        ));
    }

    #[test]
    fn script_builds_a_typed_controlled_overlay() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn changed(ctx, open) { () }
                    fn view() {
                        overlay(
                            text("trigger"),
                            text("content"),
                            #{
                                id: "account",
                                kind: "popover",
                                placement: "bottom",
                                open: true,
                                gap: 6
                            }
                        ).on_open_change(Fn("changed"))
                    }
                "#,
            )
            .unwrap();
        let root = runtime.render(&compiled).unwrap();
        assert!(matches!(
            root.kind(),
            crate::UiNodeKind::Overlay { spec, .. }
                if spec.id.as_str() == "account"
                    && spec.open
                    && (spec.gap - 6.0).abs() < f64::EPSILON
                    && spec.placement == crate::OverlayPlacement::Bottom
        ));
        assert_eq!(
            root.handlers()["open_change"].generation(),
            compiled.generation()
        );
    }

    #[test]
    fn click_handlers_can_carry_typed_semantic_payloads() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn changed(ctx, value) { () }
                    fn view() {
                        text("toggle").on_click_value(
                            Fn("changed"), #{ checked: true, source: "pointer" }
                        )
                    }
                "#,
            )
            .unwrap();
        let root = runtime.render(&compiled).unwrap();
        assert_eq!(
            root.handler_payload("click"),
            Some(&crate::UiValue::Map(BTreeMap::from([
                ("checked".to_owned(), crate::UiValue::Bool(true)),
                (
                    "source".to_owned(),
                    crate::UiValue::String("pointer".to_owned()),
                ),
            ])))
        );
        assert!(root.handlers().contains_key("click"));
    }

    #[test]
    fn recompilation_invalidates_previous_generation() {
        let mut runtime = RuntimeEngine::new();
        let first = runtime.compile("fn view() { text(\"first\") }").unwrap();
        runtime.render(&first).unwrap();
        let second = runtime.compile("fn view() { text(\"second\") }").unwrap();

        assert!(runtime.is_current(first.generation()));
        assert!(!runtime.is_current(second.generation()));
        runtime.render(&second).unwrap();
        assert!(!runtime.is_current(first.generation()));
        assert!(runtime.is_current(second.generation()));
    }

    #[test]
    fn callbacks_are_rejected_after_reload() {
        let mut runtime = RuntimeEngine::new();
        let first = runtime
            .compile(
                r#"
                    fn view() { text("first") }
                    fn add_suffix(value) { value + "!" }
                "#,
            )
            .unwrap();
        runtime.render(&first).unwrap();
        let callback = runtime.callback(&first, "add_suffix").unwrap();

        let value = runtime
            .invoke_callback(&first, &callback, ("hello",))
            .unwrap();
        assert_eq!(value.into_immutable_string().unwrap(), "hello!");

        let second = runtime.compile("fn view() { text(\"second\") }").unwrap();
        runtime.render(&second).unwrap();
        assert!(matches!(
            runtime.invoke_callback(&second, &callback, ("hello",)),
            Err(RuntimeError::StaleCallback { .. })
        ));
    }

    #[test]
    fn failed_candidate_keeps_last_good_generation_active() {
        let mut runtime = RuntimeEngine::new();
        let active = runtime
            .compile(
                r#"
                    fn view() { text("active") }
                    fn action(value) { value }
                "#,
            )
            .unwrap();
        runtime.render(&active).unwrap();
        let callback = runtime.callback(&active, "action").unwrap();

        let candidate = runtime
            .compile("fn view() { throw \"render failed\"; }")
            .unwrap();
        assert!(runtime.render(&candidate).is_err());
        assert!(runtime.is_current(active.generation()));
        let _ = runtime
            .invoke_callback(&active, &callback, ("still active",))
            .unwrap();
    }

    #[test]
    fn execution_timings_cover_compile_and_render() {
        let mut runtime = RuntimeEngine::new();
        runtime.set_slow_threshold(Duration::ZERO);
        let compiled = runtime.compile("fn view() { text(\"timed\") }").unwrap();
        runtime.render(&compiled).unwrap();
        let timings = runtime.take_timings();
        assert_eq!(timings.len(), 2);
        assert_eq!(timings[0].operation, ExecutionOperation::Compile);
        assert_eq!(timings[1].operation, ExecutionOperation::Render);
        assert!(timings.iter().all(|timing| timing.slow && timing.succeeded));
    }
}
