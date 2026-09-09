use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rhai::{
    AST, ASTNode, Array, Dynamic, Engine, EvalAltResult, Expr, FnPtr, FuncArgs, FuncRegistration,
    ImmutableString, Map, Module, ModuleResolver, Position, Scope, Stmt,
};
use thiserror::Error;

use crate::animation::register_animation_api;
use crate::asset::{AssetId, ImageDecodeHandle, asset_id_from_script};
use crate::backend::{AstInterpreter, ExecutionBackend};
use crate::canvas::register_canvas_api;
use crate::column_resize::{ColumnResizePrimitiveHandler, column_resize_primitive_descriptor};
use crate::component::{ComponentExportCollector, ComponentExportError, ComponentRegistry};
use crate::context::{UiContext, register_ui_context_api};
use crate::date::register_date_api;
use crate::node::{
    asset_image_node, box_node, canvas_node, column_node, directional_asset_image_node,
    directional_image_node, error_boundary_node, fragment_node, generic_directional_image_node,
    generic_image_node, image_node, layer_node, lazy_error_boundary_node, overlay_node,
    rich_text_node, row_node, span_value, stack_node, svg_node, text_node,
};
use crate::primitive::{PrimitiveDescriptor, PrimitiveError, PrimitiveHandler, PrimitiveRegistry};
use crate::range_input::{RangeInputPrimitiveHandler, range_input_primitive_descriptor};
use crate::style::register_style_api;
use crate::text_area::{
    TextAreaPrimitiveHandler, register_text_area_api, text_area_primitive_descriptor,
};
use crate::text_input::{TextInputPrimitiveHandler, text_input_primitive_descriptor};
use crate::value::OpaqueHandle;
use crate::{
    ComponentInstancePath, ComponentStateSchema, EventSchema, ExecutionPhase, ModuleId,
    RenderStateTransaction, UiNode, UiRuntimeState, UiValue,
};
use crate::{SubscriptionHandle, TaskHandle};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScriptGeneration(u64);

impl ScriptGeneration {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn initial() -> Self {
        Self(1)
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn next(self) -> Self {
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
    /// A formal-component subtree returned without executing its Rhai render.
    /// The payload is the number of retained formal component instances in the
    /// reused subtree, including its root.
    ComponentReuse(usize),
    VirtualCollection(String),
    Lifecycle(String),
    Callback(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionTiming {
    pub operation: ExecutionOperation,
    pub source: String,
    pub duration: Duration,
    pub operations: u64,
    pub operation_semantics: u32,
    pub slow: bool,
    pub succeeded: bool,
}

pub const OPERATION_SEMANTICS_VERSION: u32 = 2;

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
    #[error("callback `{name}` cannot be retained: {source}")]
    RetainedCallback {
        name: String,
        source: ScriptCallbackDefinitionError,
    },
    #[error(
        "callback `{name}` belongs to script generation {callback_generation}, but current generation is {current_generation}"
    )]
    StaleCallback {
        name: String,
        callback_generation: ScriptGeneration,
        current_generation: ScriptGeneration,
    },
    #[error("callback `{name}` belongs to an unmounted incarnation of component `{component}`")]
    StaleComponentCallback {
        name: String,
        component: ComponentInstancePath,
    },
    #[error("component runtime failed: {0}")]
    ComponentRuntime(String),
    #[error("component invocation `{0}` is not retained in the active generation")]
    MissingComponentInvocation(ComponentInstancePath),
    #[error("invalid script import: {0}")]
    Import(String),
    #[error(
        "compiled Rhai source contains an assignment target that Rhai 1.26 cannot evaluate safely at {0}"
    )]
    InvalidAssignmentTarget(Position),
}

#[derive(Debug, Error)]
pub enum ScriptCallbackDefinitionError {
    #[error("anonymous or capturing functions cannot escape one synchronous Rhai evaluation")]
    Anonymous,
    #[error("curried argument {index} cannot cross the retained callback boundary: {source}")]
    InvalidCurry {
        index: usize,
        source: crate::UiValueError,
    },
}

/// Private provenance carried inside a callback prop while a formal component
/// renders. Rhai clones curried values when a callback is forwarded, so this
/// survives arbitrary component depth without adding a public constructor or
/// type. Components treat callback props as opaque; the marker is stripped
/// before the callback crosses the retained boundary.
#[derive(Clone, Debug)]
struct ComponentCallbackBinding {
    component: ComponentInstancePath,
    incarnation: crate::ComponentIncarnation,
    events: BTreeMap<String, EventSchema>,
    context: Option<crate::invocation::ScriptInvocationContext>,
}

#[derive(Clone)]
pub struct CompiledUi {
    ast: AST,
    generation: ScriptGeneration,
}

#[derive(Clone, Debug)]
pub struct ScriptCallback {
    function: FnPtr,
    curry: Vec<UiValue>,
    generation: ScriptGeneration,
    component: Option<ComponentInstancePath>,
    incarnation: Option<crate::ComponentIncarnation>,
    events: BTreeMap<String, EventSchema>,
    #[allow(deprecated)]
    native_context: Option<crate::invocation::ScriptInvocationContext>,
}

impl PartialEq for ScriptCallback {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
            && self.curry == other.curry
            && self.generation == other.generation
            && self.component == other.component
            && self.incarnation == other.incarnation
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

    pub(crate) fn try_from_fn_ptr(
        mut function: FnPtr,
        generation: ScriptGeneration,
    ) -> Result<Self, ScriptCallbackDefinitionError> {
        if function.is_anonymous() {
            return Err(ScriptCallbackDefinitionError::Anonymous);
        }
        let mut binding = None;
        let mut curry = Vec::new();
        for (index, value) in function.iter_curry().cloned().enumerate() {
            if value.is::<ComponentCallbackBinding>() {
                binding = Some(value.cast::<ComponentCallbackBinding>());
                continue;
            }
            curry.push(
                UiValue::from_dynamic(value).map_err(|source| {
                    ScriptCallbackDefinitionError::InvalidCurry { index, source }
                })?,
            );
        }
        function.set_curry(curry.iter().cloned().map(UiValue::into_dynamic));
        let (component, incarnation, events, native_context) = binding.map_or_else(
            || (None, None, BTreeMap::new(), None),
            |binding| {
                (
                    Some(binding.component),
                    Some(binding.incarnation),
                    binding.events,
                    binding.context,
                )
            },
        );
        Ok(Self {
            function,
            curry,
            generation,
            component,
            incarnation,
            events,
            native_context,
        })
    }

    pub(crate) fn bind_generation(&mut self, generation: ScriptGeneration) {
        self.generation = generation;
    }

    #[cfg(test)]
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

    pub(crate) fn bind_component_scope_if_unset(
        &mut self,
        component: &ComponentInstancePath,
        incarnation: crate::ComponentIncarnation,
        events: BTreeMap<String, EventSchema>,
    ) {
        if self.component.is_none() {
            self.component = Some(component.clone());
            self.events = events;
        }
        if self.component.as_ref() == Some(component) && self.incarnation.is_none() {
            self.incarnation = Some(incarnation);
        }
    }

    pub(crate) fn bind_native_context_if_unset(
        &mut self,
        context: crate::invocation::ScriptInvocationContext,
    ) {
        if self.native_context.is_none() {
            self.native_context = Some(context);
        }
    }

    pub(crate) fn component(&self) -> Option<&ComponentInstancePath> {
        self.component.as_ref()
    }

    pub(crate) const fn incarnation(&self) -> Option<crate::ComponentIncarnation> {
        self.incarnation
    }

    pub(crate) fn events(&self) -> &BTreeMap<String, EventSchema> {
        &self.events
    }

    pub(crate) fn native_context(&self) -> Option<&crate::invocation::ScriptInvocationContext> {
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
    invocations: BTreeMap<ComponentInstancePath, ComponentInvocationRecipe>,
    effect_keys: Vec<Option<BTreeSet<String>>>,
    effects: BTreeMap<crate::EffectId, crate::EffectDescriptor>,
    timers: BTreeMap<crate::TimerId, crate::TimerDescriptor>,
    signals: BTreeMap<crate::SignalId, crate::signal::SignalDescriptor>,
    element_refs: BTreeSet<crate::ElementRefId>,
    virtual_collections: BTreeMap<crate::VirtualCollectionId, VirtualCollectionRecipe>,
    environment: ComponentRenderEnvironment,
    reuse: Option<ComponentReuseSnapshot>,
    reused: Vec<(String, usize)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ComponentRenderEnvironment {
    theme_generation: Option<u64>,
    component_style_generation: u64,
    locale_generation: Option<u64>,
    calendar_day: crate::GregorianDate,
}

#[derive(Clone)]
struct ComponentReusePlan {
    previous_root: Rc<UiNode>,
    subtrees: crate::node::ComponentSubtreeIndex,
    dirty: BTreeSet<ComponentInstancePath>,
}

impl ComponentReusePlan {
    fn new(previous_root: Rc<UiNode>, dirty: BTreeSet<ComponentInstancePath>) -> Self {
        let subtrees = crate::node::ComponentSubtreeIndex::new(&previous_root);
        Self {
            previous_root,
            subtrees,
            dirty,
        }
    }
}

#[derive(Clone)]
struct ComponentReuseSnapshot {
    previous_root: Rc<UiNode>,
    subtrees: crate::node::ComponentSubtreeIndex,
    dirty: BTreeSet<ComponentInstancePath>,
    invocations: BTreeMap<ComponentInstancePath, ComponentInvocationRecipe>,
    event_handlers: BTreeMap<(ComponentInstancePath, String), ScriptCallback>,
    effects: BTreeMap<crate::EffectId, crate::EffectDescriptor>,
    timers: BTreeMap<crate::TimerId, crate::TimerDescriptor>,
    signals: BTreeMap<crate::SignalId, crate::signal::SignalDescriptor>,
    element_refs: BTreeSet<crate::ElementRefId>,
    virtual_collections: BTreeMap<crate::VirtualCollectionId, VirtualCollectionRecipe>,
}

struct ReusedComponentScope {
    node: UiNode,
    invocations: BTreeMap<ComponentInstancePath, ComponentInvocationRecipe>,
    event_handlers: BTreeMap<(ComponentInstancePath, String), ScriptCallback>,
    resources: ReusedComponentResources,
}

struct ReusedComponentResources {
    effects: BTreeMap<crate::EffectId, crate::EffectDescriptor>,
    timers: BTreeMap<crate::TimerId, crate::TimerDescriptor>,
    signals: BTreeMap<crate::SignalId, crate::signal::SignalDescriptor>,
    element_refs: BTreeSet<crate::ElementRefId>,
    virtual_collections: BTreeMap<crate::VirtualCollectionId, VirtualCollectionRecipe>,
}

#[derive(Clone, Debug)]
struct VirtualCollectionRecipe {
    id: crate::VirtualCollectionId,
    data: crate::VirtualCollectionData,
    renderer: ScriptCallback,
    context: UiContext,
    event_context: UiContext,
    generation: ScriptGeneration,
}

type ActiveComponentRenderState = Rc<RefCell<Option<ActiveComponentRender>>>;

#[derive(Clone)]
struct PendingComponentCommit {
    root: ComponentInstancePath,
    previous: BTreeSet<ComponentInstancePath>,
    active: BTreeSet<ComponentInstancePath>,
    transaction: RenderStateTransaction,
    event_handlers: BTreeMap<(ComponentInstancePath, String), ScriptCallback>,
}

#[derive(Clone, Debug)]
pub struct ComponentInvocationRecipe {
    path: ComponentInstancePath,
    parent: ComponentInstancePath,
    component: ModuleId,
    export: String,
    declared_key: Option<String>,
    props: crate::ComponentProps,
    script_props: Map,
    component_context: UiContext,
    caller_context: UiContext,
    part_styles: BTreeMap<String, crate::Style>,
    event_callbacks: Vec<(String, FnPtr)>,
    component_events: BTreeMap<String, EventSchema>,
    declared_effects: BTreeSet<String>,
    render: FnPtr,
    snapshot: crate::node::ComponentOwnedSnapshot,
    context: crate::invocation::ScriptInvocationContext,
    generation: ScriptGeneration,
    environment: ComponentRenderEnvironment,
    reusable: bool,
}

#[derive(Clone)]
pub(crate) struct RegisteredComponentRender {
    generation: ScriptGeneration,
    render: FnPtr,
}

type ComponentRenderRegistry = Rc<RefCell<BTreeMap<ModuleId, RegisteredComponentRender>>>;

impl ComponentInvocationRecipe {
    #[must_use]
    pub fn path(&self) -> &ComponentInstancePath {
        &self.path
    }

    #[must_use]
    pub fn parent(&self) -> &ComponentInstancePath {
        &self.parent
    }

    #[must_use]
    pub fn component(&self) -> &ModuleId {
        &self.component
    }

    #[must_use]
    pub fn export(&self) -> &str {
        &self.export
    }

    #[must_use]
    pub fn declared_key(&self) -> Option<&str> {
        self.declared_key.as_deref()
    }

    #[must_use]
    pub fn props(&self) -> &crate::ComponentProps {
        &self.props
    }

    #[must_use]
    pub fn render_name(&self) -> &str {
        self.render.fn_name()
    }

    #[must_use]
    pub fn has_invocation_context(&self) -> bool {
        let _ = &self.context;
        true
    }

    #[must_use]
    pub fn generation(&self) -> ScriptGeneration {
        self.generation
    }
}

pub struct RuntimeEngine {
    engine: Engine,
    generation: ScriptGeneration,
    preparation_generation: Option<ScriptGeneration>,
    component_exports: ComponentExportCollector,
    evaluation_generation: Rc<Cell<ScriptGeneration>>,
    primitives: PrimitiveRegistry,
    primitive_modules: BTreeMap<String, Module>,
    timings: RefCell<Vec<ExecutionTiming>>,
    operation_tracker: Rc<OperationTracker>,
    slow_threshold: Duration,
    component_render: ActiveComponentRenderState,
    component_invocations: BTreeMap<ComponentInstancePath, ComponentInvocationRecipe>,
    component_effects: BTreeMap<crate::EffectId, crate::EffectDescriptor>,
    component_timers: BTreeMap<crate::TimerId, crate::TimerDescriptor>,
    component_signals: BTreeMap<crate::SignalId, crate::signal::SignalDescriptor>,
    component_element_refs: BTreeSet<crate::ElementRefId>,
    pending_component_commits: BTreeMap<ComponentInstancePath, PendingComponentCommit>,
    component_renderers: ComponentRenderRegistry,
    native_handlers: crate::NativeHandlerRegistry,
    syntax_registry: crate::SyntaxRegistry,
    document_runtime: crate::DocumentRuntimeConfig,
    virtual_collections: BTreeMap<crate::VirtualCollectionId, VirtualCollectionRecipe>,
}

const MAX_SCRIPT_OPERATIONS: u64 = 1_000_000;

#[derive(Debug, Default)]
struct OperationTracker {
    last_absolute: Cell<u64>,
    total: Cell<u64>,
}

impl OperationTracker {
    fn begin(&self, inherited_base: u64) {
        self.last_absolute.set(inherited_base);
        self.total.set(0);
    }

    fn observe(&self, absolute: u64) -> u64 {
        let last_absolute = self.last_absolute.get();
        let delta = if absolute >= last_absolute {
            absolute - last_absolute
        } else {
            // Returning from a nested evaluator resumes a smaller parent
            // counter. Rhai invokes progress once per operation, so the first
            // lower observation represents one newly consumed parent step.
            1
        };
        let total = self.total.get().saturating_add(delta);
        self.total.set(total);
        self.last_absolute.set(absolute);
        total
    }

    fn align(&self, inherited_base: u64) {
        // Stored NativeCallContext values clone the absolute counter captured
        // in an earlier evaluator. Align only that absolute baseline before a
        // retained call; the current execution session's aggregate must remain
        // intact across multiple dirty components or virtual items.
        self.last_absolute.set(inherited_base);
    }
}

#[derive(Clone, Copy)]
struct ExecutionTimingStart {
    instant: Instant,
    operations: u64,
}

#[derive(Clone)]
pub(crate) struct RuntimeEngineCheckpoint {
    generation: ScriptGeneration,
    evaluation_generation: ScriptGeneration,
    component_invocations: BTreeMap<ComponentInstancePath, ComponentInvocationRecipe>,
    component_effects: BTreeMap<crate::EffectId, crate::EffectDescriptor>,
    component_timers: BTreeMap<crate::TimerId, crate::TimerDescriptor>,
    component_signals: BTreeMap<crate::SignalId, crate::signal::SignalDescriptor>,
    component_element_refs: BTreeSet<crate::ElementRefId>,
    pending_component_commits: BTreeMap<ComponentInstancePath, PendingComponentCommit>,
    virtual_collections: BTreeMap<crate::VirtualCollectionId, VirtualCollectionRecipe>,
    component_snapshots: Vec<(crate::node::ComponentOwnedSnapshot, Option<Rc<UiNode>>)>,
}

impl Default for RuntimeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeEngine {
    #[cfg(feature = "dev-reload")]
    pub(crate) fn candidate_engine(&self) -> Self {
        let mut candidate = Self::new();
        candidate.slow_threshold = self.slow_threshold;
        candidate
    }

    /// Create a runtime with all built-in primitive descriptors.
    ///
    /// # Panics
    ///
    /// Panics only if a compile-time built-in descriptor violates the same
    /// validation rules enforced for downstream custom primitives.
    #[must_use]
    pub fn new() -> Self {
        let mut engine = Engine::new();
        engine.set_module_resolver(crate::source::RestrictedModuleResolver::new());
        let operation_tracker = Rc::new(OperationTracker::default());
        let progress = Rc::clone(&operation_tracker);
        engine.on_progress(move |operations| {
            let total = progress.observe(operations);
            (total > MAX_SCRIPT_OPERATIONS).then(|| {
                Dynamic::from(format!(
                    "script operation budget exceeded: {total} > {MAX_SCRIPT_OPERATIONS}"
                ))
            })
        });
        engine.build_type::<UiNode>();
        engine.build_type::<OpaqueHandle>();
        engine.build_type::<TaskHandle>();
        engine.build_type::<SubscriptionHandle>();
        engine.build_type::<AssetId>();
        engine.build_type::<ImageDecodeHandle>();
        engine.build_type::<crate::NativeSignal>();
        engine.build_type::<crate::EventResponse>();
        engine.build_type::<crate::ElementRef>();
        engine.build_type::<crate::NativeHandlerRef>();
        crate::native_collection::register_native_collection_api(&mut engine);
        crate::document::register_document_api(&mut engine);
        engine.build_type::<crate::Span>();
        register_ui_context_api(&mut engine);
        register_date_api(&mut engine);
        register_style_api(&mut engine);
        register_animation_api(&mut engine);
        register_text_area_api(&mut engine);
        register_canvas_api(&mut engine);
        let evaluation_generation = Rc::new(Cell::new(ScriptGeneration::default()));
        let component_exports = ComponentExportCollector::new();
        let (component_render, component_renderers) = register_component_runtime_apis(
            &mut engine,
            &component_exports,
            &evaluation_generation,
        );
        let primitives = PrimitiveRegistry::new();
        let native_handlers = crate::NativeHandlerRegistry::new();
        let syntax_registry = crate::SyntaxRegistry::new();
        let document_runtime = crate::DocumentRuntimeConfig::new();
        register_native_handler_api(&mut engine, &native_handlers);

        register_node_apis(&mut engine);

        configure_engine_limits(&mut engine);

        let mut runtime = Self {
            engine,
            generation: ScriptGeneration::default(),
            preparation_generation: None,
            component_exports,
            evaluation_generation,
            primitives,
            primitive_modules: BTreeMap::new(),
            timings: RefCell::new(Vec::new()),
            operation_tracker,
            slow_threshold: Duration::from_millis(16),
            component_render,
            component_invocations: BTreeMap::new(),
            component_effects: BTreeMap::new(),
            component_timers: BTreeMap::new(),
            component_signals: BTreeMap::new(),
            component_element_refs: BTreeSet::new(),
            pending_component_commits: BTreeMap::new(),
            component_renderers,
            native_handlers,
            syntax_registry,
            document_runtime,
            virtual_collections: BTreeMap::new(),
        };
        runtime.register_builtin_primitives();
        runtime
    }

    fn register_builtin_primitives(&mut self) {
        self.register_primitive(
            column_resize_primitive_descriptor(),
            ColumnResizePrimitiveHandler,
        )
        .expect("built-in column resize primitive descriptor is valid");
        self.register_primitive(
            text_input_primitive_descriptor(),
            TextInputPrimitiveHandler::default(),
        )
        .expect("built-in TextInput primitive descriptor is valid");
        self.register_primitive(
            text_area_primitive_descriptor(),
            TextAreaPrimitiveHandler::default(),
        )
        .expect("built-in Textarea primitive descriptor is valid");
        self.register_primitive(
            range_input_primitive_descriptor(),
            RangeInputPrimitiveHandler::default(),
        )
        .expect("built-in RangeInput primitive descriptor is valid");
        self.register_primitive(
            crate::code_viewer_primitive_descriptor(),
            crate::CodeViewerPrimitiveHandler::new(
                self.syntax_registry.clone(),
                self.document_runtime.clone(),
            ),
        )
        .expect("built-in CodeViewer primitive descriptor is valid");
        self.register_primitive(
            crate::diff_viewer_primitive_descriptor(),
            crate::DiffViewerPrimitiveHandler::new(
                self.syntax_registry.clone(),
                self.document_runtime.clone(),
            ),
        )
        .expect("built-in DiffViewer primitive descriptor is valid");
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
        let generation = self.candidate_generation();
        self.evaluation_generation.set(generation);
        self.begin_execution_session();
        let started = self.begin_timing();
        let result = self.engine.compile(source);
        self.record_timing(
            ExecutionOperation::Compile,
            source_name,
            started,
            result.is_ok(),
        );
        let mut ast = result.map_err(|error| RuntimeError::Compile(error.into()))?;
        validate_assignment_targets(&ast)?;
        ast.set_source(source_name);
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
        let generation = self.candidate_generation();
        self.evaluation_generation.set(generation);
        self.begin_execution_session();
        let started = self.begin_timing();
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
        validate_literal_imports(&ast)?;
        validate_assignment_targets(&ast)?;
        ast.set_source(source_name);
        Ok(CompiledUi { ast, generation })
    }

    pub fn set_module_resolver(&mut self, resolver: impl ModuleResolver + 'static) {
        self.engine.set_module_resolver(resolver);
    }

    /// Compile all modules and the entry AST for one candidate under one unique
    /// program identity. Independent calls receive different generations even
    /// when neither candidate has been rendered yet.
    ///
    /// # Errors
    ///
    /// Returns the error produced by `operation`.
    pub fn with_program_preparation<T, E>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        if self.preparation_generation.is_some() {
            return operation(self);
        }
        let generation = Self::allocate_generation();
        self.preparation_generation = Some(generation);
        let result = operation(self);
        self.preparation_generation = None;
        result
    }

    fn begin_component_render(
        &self,
        context: UiContext,
        generation: ScriptGeneration,
        root_effects: Option<BTreeSet<String>>,
        reuse_plan: Option<ComponentReusePlan>,
    ) -> Result<(), RuntimeError> {
        let root = context.component_path().clone();
        context.reset_non_reusable_render_reads();
        let (state_snapshot, environment, event_handlers) = {
            let runtime = context.runtime().try_borrow().map_err(|_| {
                RuntimeError::ComponentRuntime("UI state is already borrowed".to_owned())
            })?;
            (
                runtime.component_state.clone(),
                ComponentRenderEnvironment {
                    theme_generation: runtime.theme.as_ref().map(crate::ThemeManager::generation),
                    component_style_generation: runtime.component_style_generation(),
                    locale_generation: runtime
                        .locale
                        .as_ref()
                        .map(crate::LocaleManager::generation),
                    calendar_day: runtime.calendar_clock.today(),
                },
                reuse_plan
                    .as_ref()
                    .map(|_| runtime.component_event_handlers_in_scope(&root)),
            )
        };
        let mut transaction = context
            .runtime()
            .try_borrow()
            .map_err(|_| RuntimeError::ComponentRuntime("UI state is already borrowed".to_owned()))?
            .component_state
            .begin_render_scope(root.clone());
        let _ = transaction.retain_existing(&root);
        let reuse = reuse_plan.map(|plan| ComponentReuseSnapshot {
            subtrees: plan.subtrees,
            previous_root: plan.previous_root,
            dirty: plan.dirty,
            invocations: self
                .component_invocations
                .iter()
                .filter(|(path, _)| path.is_within(&root))
                .map(|(path, recipe)| (path.clone(), recipe.clone()))
                .collect(),
            event_handlers: event_handlers.unwrap_or_default(),
            effects: self
                .component_effects
                .iter()
                .filter(|(id, _)| id.component().is_within(&root))
                .map(|(id, descriptor)| (id.clone(), descriptor.clone()))
                .collect(),
            timers: self
                .component_timers
                .iter()
                .filter(|(id, _)| id.component().is_within(&root))
                .map(|(id, descriptor)| (id.clone(), descriptor.clone()))
                .collect(),
            signals: self
                .component_signals
                .iter()
                .filter(|(id, _)| id.component().is_within(&root))
                .map(|(id, descriptor)| (id.clone(), descriptor.clone()))
                .collect(),
            element_refs: self
                .component_element_refs
                .iter()
                .filter(|id| id.component().is_within(&root))
                .cloned()
                .collect(),
            virtual_collections: self
                .virtual_collections
                .iter()
                .filter(|(id, _)| id.component.is_within(&root))
                .map(|(id, recipe)| (id.clone(), recipe.clone()))
                .collect(),
        });
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
            invocations: BTreeMap::new(),
            effect_keys: vec![root_effects],
            effects: BTreeMap::new(),
            timers: BTreeMap::new(),
            signals: BTreeMap::new(),
            element_refs: BTreeSet::new(),
            virtual_collections: BTreeMap::new(),
            environment,
            reuse,
            reused: Vec::new(),
        });
        Ok(())
    }

    fn finish_component_render(&mut self, commit: bool) -> Result<(), RuntimeError> {
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
            self.timings
                .borrow_mut()
                .extend(
                    active
                        .reused
                        .iter()
                        .map(|(source, components)| ExecutionTiming {
                            operation: ExecutionOperation::ComponentReuse(*components),
                            source: source.clone(),
                            duration: Duration::ZERO,
                            operations: 0,
                            operation_semantics: OPERATION_SEMANTICS_VERSION,
                            slow: false,
                            succeeded: true,
                        }),
                );
            let root = active.root_context.component_path().clone();
            let previous = active.state_snapshot.paths();
            self.pending_component_commits.insert(
                root.clone(),
                PendingComponentCommit {
                    root: root.clone(),
                    previous,
                    active: active.seen,
                    transaction: active.transaction,
                    event_handlers: active.event_handlers,
                },
            );
            self.component_invocations
                .retain(|path, _| !path.is_within(&root));
            self.component_invocations.extend(active.invocations);
            self.component_effects
                .retain(|id, _| !id.component().is_within(&root));
            self.component_effects.extend(active.effects);
            self.component_timers
                .retain(|id, _| !id.component().is_within(&root));
            self.component_timers.extend(active.timers);
            self.component_signals
                .retain(|id, _| !id.component().is_within(&root));
            self.component_signals.extend(active.signals);
            self.component_element_refs
                .retain(|id| !id.component().is_within(&root));
            self.component_element_refs.extend(active.element_refs);
            self.virtual_collections
                .retain(|id, _| !id.component.is_within(&root));
            self.virtual_collections.extend(active.virtual_collections);
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
        self.begin_execution_session();
        let checkpoint = self.execution_checkpoint();
        let root_path = ComponentInstancePath::root("App", "root");
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let context = UiContext::new(
            Rc::clone(&runtime),
            root_path.clone(),
            None,
            ExecutionPhase::Render,
            BTreeMap::new(),
        )
        .with_generation(compiled.generation);
        self.begin_component_render(context, compiled.generation, None, None)?;
        let started = self.begin_timing();
        let result = AstInterpreter::call_fn::<UiNode, _>(
            &self.engine,
            &compiled.ast,
            &mut Scope::new(),
            "view",
            (),
        );
        self.record_timing(
            ExecutionOperation::Render,
            compiled.ast.source().unwrap_or("<script>"),
            started,
            result.is_ok(),
        );
        self.finish_component_render(result.is_ok())?;
        let mut root = result.map_err(RuntimeError::Evaluate)?;
        if !self.component_effects_in_scope(&root_path).is_empty()
            || !self.component_timers_in_scope(&root_path).is_empty()
            || !self.component_signals_in_scope(&root_path).is_empty()
            || !self.component_element_refs_in_scope(&root_path).is_empty()
            || !self.virtual_collection_ids_in_scope(&root_path).is_empty()
        {
            self.restore_execution_checkpoint(checkpoint);
            return Err(RuntimeError::ComponentRuntime(
                "effectful, timer/signal/ref-owning, or virtual components require ScriptLifecycle"
                    .to_owned(),
            ));
        }
        if let Err(error) = self.commit_component_renders(&mut runtime.borrow_mut()) {
            self.restore_execution_checkpoint(checkpoint);
            return Err(error);
        }
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
        let runtime = Rc::clone(context.runtime());
        let root = context.component_path().clone();
        let snapshot = runtime
            .try_borrow_mut()
            .map_err(|_| RuntimeError::ComponentRuntime("UI state is already borrowed".to_owned()))?
            .begin_transaction()
            .map_err(|error| RuntimeError::ComponentRuntime(error.to_string()))?;
        let checkpoint = self.execution_checkpoint();
        let result = self.render_with_context_staged(compiled, context);
        let result = result.and_then(|node| {
            if !self.component_effects_in_scope(&root).is_empty()
                || !self.component_timers_in_scope(&root).is_empty()
                || !self.component_signals_in_scope(&root).is_empty()
                || !self.component_element_refs_in_scope(&root).is_empty()
                || !self.virtual_collection_ids_in_scope(&root).is_empty()
            {
                return Err(RuntimeError::ComponentRuntime(
                    "effectful, timer/signal/ref-owning, or virtual components require ScriptLifecycle"
                        .to_owned(),
                ));
            }
            self.commit_component_renders(&mut runtime.borrow_mut())?;
            Ok(node)
        });
        match result {
            Ok(node) => {
                runtime
                    .try_borrow_mut()
                    .map_err(|_| {
                        RuntimeError::ComponentRuntime("UI state is already borrowed".to_owned())
                    })?
                    .commit_transaction()
                    .map_err(|error| RuntimeError::ComponentRuntime(error.to_string()))?;
                Ok(node)
            }
            Err(error) => {
                runtime
                    .try_borrow_mut()
                    .map_err(|_| {
                        RuntimeError::ComponentRuntime("UI state is already borrowed".to_owned())
                    })?
                    .restore(snapshot)
                    .map_err(|restore| RuntimeError::ComponentRuntime(restore.to_string()))?;
                self.restore_execution_checkpoint(checkpoint);
                Err(error)
            }
        }
    }

    pub(crate) fn render_with_context_staged(
        &mut self,
        compiled: &CompiledUi,
        context: UiContext,
    ) -> Result<UiNode, RuntimeError> {
        self.render_with_context_staged_impl(compiled, context, None)
    }

    pub(crate) fn render_with_context_staged_reusing(
        &mut self,
        compiled: &CompiledUi,
        context: UiContext,
        previous_root: Rc<UiNode>,
        dirty: &BTreeSet<ComponentInstancePath>,
    ) -> Result<UiNode, RuntimeError> {
        self.render_with_context_staged_impl(
            compiled,
            context,
            Some(ComponentReusePlan::new(previous_root, dirty.clone())),
        )
    }

    fn render_with_context_staged_impl(
        &mut self,
        compiled: &CompiledUi,
        context: UiContext,
        reuse: Option<ComponentReusePlan>,
    ) -> Result<UiNode, RuntimeError> {
        self.evaluation_generation.set(compiled.generation);
        self.begin_execution_session();
        self.begin_component_render(context.clone(), compiled.generation, None, reuse)?;
        let started = self.begin_timing();
        let result = AstInterpreter::call_fn::<UiNode, _>(
            &self.engine,
            &compiled.ast,
            &mut Scope::new(),
            "view",
            (context,),
        );
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

    pub(crate) fn rerender_component(
        &mut self,
        component: &ComponentInstancePath,
        previous_root: &UiNode,
        dirty: &BTreeSet<ComponentInstancePath>,
    ) -> Result<UiNode, RuntimeError> {
        let mut recipe = self
            .component_invocations
            .get(component)
            .cloned()
            .ok_or_else(|| RuntimeError::MissingComponentInvocation(component.clone()))?;
        if recipe.generation != self.generation {
            return Err(RuntimeError::StaleCallback {
                name: recipe.render.fn_name().to_owned(),
                callback_generation: recipe.generation,
                current_generation: self.generation,
            });
        }
        recipe
            .component_context
            .runtime()
            .try_borrow_mut()
            .map_err(|_| RuntimeError::ComponentRuntime("UI state is already borrowed".to_owned()))?
            .reset_component_readers(component);
        for value in recipe.script_props.values_mut() {
            hydrate_replayed_node_value(value);
        }
        let reuse = ComponentReusePlan::new(Rc::new(previous_root.clone()), dirty.clone());
        self.begin_component_render(
            recipe.component_context.clone(),
            recipe.generation,
            Some(recipe.declared_effects.clone()),
            Some(reuse),
        )?;
        register_component_invocation(&self.component_render, recipe.clone())
            .map_err(RuntimeError::Evaluate)?;

        self.align_execution_session_to(recipe.context.operation_base());
        let started = self.begin_timing();
        let result = (|| {
            let mut node = recipe
                .context
                .call::<UiNode>(
                    self.engine(),
                    &recipe.render,
                    (
                        recipe.component_context.clone(),
                        recipe.script_props.clone(),
                    ),
                )
                .map_err(RuntimeError::Evaluate)?;
            register_component_event_callbacks(
                &self.component_render,
                &recipe.path,
                &recipe.caller_context,
                recipe.event_callbacks.clone(),
                &recipe.context,
            )
            .map_err(RuntimeError::Evaluate)?;
            node = node.with_owned_part_styles(recipe.part_styles.clone());
            node =
                node.with_component_root_snapshot(recipe.path.clone(), recipe.snapshot.reference());
            node.bind_component_scope(
                &recipe.path,
                recipe.component_context.component_incarnation(),
                &recipe.component_events,
                Some(&recipe.context),
            );
            node.bind_generation(recipe.generation);
            Ok(node)
        })();
        {
            let mut active = self.component_render.try_borrow_mut().map_err(|_| {
                RuntimeError::ComponentRuntime(
                    "component render stack is already borrowed".to_owned(),
                )
            })?;
            if let Some(active) = active.as_mut() {
                let environment = active.environment;
                if let Some(retained) = active.invocations.get_mut(&recipe.path) {
                    retained.environment = environment;
                    retained.reusable = recipe.component_context.component_render_is_reusable();
                    if let Ok(node) = &result {
                        retained.snapshot.update_if_active(node);
                    }
                }
            }
        }
        self.record_timing(
            ExecutionOperation::Render,
            recipe.component.as_str(),
            started,
            result.is_ok(),
        );
        self.finish_component_render(result.is_ok())?;
        result
    }

    pub(crate) fn execution_checkpoint(&self) -> RuntimeEngineCheckpoint {
        RuntimeEngineCheckpoint {
            generation: self.generation,
            evaluation_generation: self.evaluation_generation.get(),
            component_invocations: self.component_invocations.clone(),
            component_effects: self.component_effects.clone(),
            component_timers: self.component_timers.clone(),
            component_signals: self.component_signals.clone(),
            component_element_refs: self.component_element_refs.clone(),
            pending_component_commits: self.pending_component_commits.clone(),
            virtual_collections: self.virtual_collections.clone(),
            component_snapshots: self
                .component_invocations
                .values()
                .map(|recipe| (recipe.snapshot.clone(), recipe.snapshot.current()))
                .collect(),
        }
    }

    pub(crate) fn restore_execution_checkpoint(&mut self, checkpoint: RuntimeEngineCheckpoint) {
        for (snapshot, value) in &checkpoint.component_snapshots {
            snapshot.restore(value.clone());
        }
        self.generation = checkpoint.generation;
        self.evaluation_generation
            .set(checkpoint.evaluation_generation);
        self.component_invocations = checkpoint.component_invocations;
        self.component_effects = checkpoint.component_effects;
        self.component_timers = checkpoint.component_timers;
        self.component_signals = checkpoint.component_signals;
        self.component_element_refs = checkpoint.component_element_refs;
        self.pending_component_commits = checkpoint.pending_component_commits;
        self.virtual_collections = checkpoint.virtual_collections;
    }

    pub(crate) fn commit_component_renders(
        &mut self,
        runtime: &mut UiRuntimeState,
    ) -> Result<(), RuntimeError> {
        let commits = std::mem::take(&mut self.pending_component_commits);
        for (_, commit) in commits {
            runtime
                .reconcile_component_lifetimes(&commit.root, &commit.active, &commit.previous)
                .map_err(|error| RuntimeError::ComponentRuntime(error.to_string()))?;
            runtime.replace_component_event_handlers(&commit.root, commit.event_handlers);
            runtime.component_state.commit_render(commit.transaction);
        }
        Ok(())
    }

    pub(crate) fn component_effects_in_scope(
        &self,
        root: &ComponentInstancePath,
    ) -> BTreeMap<crate::EffectId, crate::EffectDescriptor> {
        self.component_effects
            .iter()
            .filter(|(id, _)| id.component().is_within(root))
            .map(|(id, descriptor)| (id.clone(), descriptor.clone()))
            .collect()
    }

    pub(crate) fn component_signals_in_scope(
        &self,
        root: &ComponentInstancePath,
    ) -> BTreeMap<crate::SignalId, crate::signal::SignalDescriptor> {
        self.component_signals
            .iter()
            .filter(|(id, _)| id.component().is_within(root))
            .map(|(id, descriptor)| (id.clone(), descriptor.clone()))
            .collect()
    }

    pub(crate) fn component_timers_in_scope(
        &self,
        root: &ComponentInstancePath,
    ) -> BTreeMap<crate::TimerId, crate::TimerDescriptor> {
        self.component_timers
            .iter()
            .filter(|(id, _)| id.component().is_within(root))
            .map(|(id, descriptor)| (id.clone(), descriptor.clone()))
            .collect()
    }

    pub(crate) fn component_element_refs_in_scope(
        &self,
        root: &ComponentInstancePath,
    ) -> BTreeSet<crate::ElementRefId> {
        self.component_element_refs
            .iter()
            .filter(|id| id.component().is_within(root))
            .cloned()
            .collect()
    }

    pub(crate) fn virtual_collection_ids_in_scope(
        &self,
        root: &ComponentInstancePath,
    ) -> BTreeSet<crate::VirtualCollectionId> {
        self.virtual_collections
            .keys()
            .filter(|id| id.component.is_within(root))
            .cloned()
            .collect()
    }

    pub(crate) fn realize_virtual_collection(
        &mut self,
        id: &crate::VirtualCollectionId,
        indices: &BTreeSet<usize>,
    ) -> Result<BTreeMap<usize, UiNode>, RuntimeError> {
        let recipe = self.virtual_collections.get(id).cloned().ok_or_else(|| {
            RuntimeError::ComponentRuntime(format!(
                "virtual collection `{}` is not retained in the active generation",
                id.key
            ))
        })?;
        if recipe.generation != self.generation || recipe.id != *id {
            return Err(RuntimeError::StaleCallback {
                name: recipe.renderer.name().to_owned(),
                callback_generation: recipe.generation,
                current_generation: self.generation,
            });
        }
        let scope = id.component.child("VirtualCollection", id.key.clone());
        let context = recipe
            .context
            .for_component(scope, BTreeMap::new())
            .with_generation(recipe.generation);
        self.begin_component_render(context.clone(), recipe.generation, None, None)?;
        let result = (|| {
            let mut realized = BTreeMap::new();
            for index in indices
                .iter()
                .copied()
                .filter(|index| *index < recipe.data.len())
            {
                let item = recipe
                    .data
                    .item(index)
                    .map_err(|error| RuntimeError::ComponentRuntime(error.to_string()))?
                    .ok_or_else(|| {
                        RuntimeError::ComponentRuntime(format!(
                            "virtual collection item {index} disappeared"
                        ))
                    })?;
                let (key, payload) =
                    collection_payload(&item, index).map_err(RuntimeError::Evaluate)?;
                let invocation = recipe.renderer.native_context.as_ref().ok_or_else(|| {
                    RuntimeError::ComponentRuntime(
                        "virtual collection renderer lost its module context".to_owned(),
                    )
                })?;
                self.align_execution_session_to(invocation.operation_base());
                let started = self.begin_timing();
                let item = invocation.call::<UiNode>(
                    self.engine(),
                    &recipe.renderer.function,
                    (context.clone(), Dynamic::from_map(payload)),
                );
                self.record_timing(
                    ExecutionOperation::VirtualCollection(id.key.clone()),
                    id.key.as_str(),
                    started,
                    item.is_ok(),
                );
                let mut node = item.map_err(RuntimeError::Evaluate)?.with_key(key);
                node.bind_generation(recipe.generation);
                node.bind_component_scope(
                    recipe.event_context.component_path(),
                    recipe.event_context.component_incarnation(),
                    recipe.renderer.events(),
                    recipe.renderer.native_context(),
                );
                realized.insert(index, node);
            }
            Ok(realized)
        })();
        self.finish_component_render(result.is_ok())?;
        result
    }

    pub(crate) fn update_virtual_collection_snapshot(
        &mut self,
        id: &crate::VirtualCollectionId,
        items: &BTreeMap<usize, UiNode>,
    ) -> Result<(), RuntimeError> {
        let Some(owner) = self.component_invocations.get_mut(&id.component) else {
            return Ok(());
        };
        let Some(current) = owner.snapshot.current() else {
            return Ok(());
        };
        let mut rendered = current.as_ref().clone();
        if !rendered.replace_virtual_collection_items(id, items.clone()) {
            return Err(RuntimeError::ComponentRuntime(format!(
                "virtual collection `{}` is missing from its owning component snapshot",
                id.key
            )));
        }
        owner.snapshot.replace_if_active(Rc::new(rendered));
        Ok(())
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
        self.begin_execution_session();
        let started = self.begin_timing();
        let result = AstInterpreter::call_fn::<Dynamic, _>(
            &self.engine,
            &compiled.ast,
            &mut Scope::new(),
            function,
            (context,),
        );
        self.record_timing(
            ExecutionOperation::Lifecycle(function.to_owned()),
            compiled.ast.source().unwrap_or("<script>"),
            started,
            result.is_ok(),
        );
        let _ = result.map_err(RuntimeError::Evaluate)?;
        Ok(true)
    }

    pub(crate) fn call_optional_lifecycle_with_value(
        &self,
        compiled: &CompiledUi,
        function: &str,
        context: UiContext,
        value: UiValue,
    ) -> Result<bool, RuntimeError> {
        if !compiled.has_function(function, 2) {
            return Ok(false);
        }
        self.evaluation_generation.set(compiled.generation);
        self.begin_execution_session();
        let started = self.begin_timing();
        let result = AstInterpreter::call_fn::<Dynamic, _>(
            &self.engine,
            &compiled.ast,
            &mut Scope::new(),
            function,
            (context, value.into_dynamic()),
        );
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
        self.begin_execution_session();
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
        ScriptCallback::try_from_fn_ptr(function, compiled.generation).map_err(|source| {
            RuntimeError::RetainedCallback {
                name: function_name.to_owned(),
                source,
            }
        })
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

        self.invoke_callback_for_generation(compiled, callback, args)
    }

    pub(crate) fn invoke_callback_for_generation(
        &self,
        compiled: &CompiledUi,
        callback: &ScriptCallback,
        args: impl FuncArgs,
    ) -> Result<Dynamic, RuntimeError> {
        if callback.generation != compiled.generation {
            return Err(RuntimeError::StaleCallback {
                name: callback.name().to_owned(),
                callback_generation: callback.generation,
                current_generation: compiled.generation,
            });
        }
        self.evaluation_generation.set(compiled.generation);
        let operation_base = callback.native_context.as_ref().map_or(
            0,
            crate::invocation::ScriptInvocationContext::operation_base,
        );
        self.begin_execution_session_from(operation_base);
        let started = self.begin_timing();
        #[allow(deprecated)]
        let result = if let Some(context) = callback.native_context.as_ref() {
            context.call(self.engine(), &callback.function, args)
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

    /// Generate the official Rhai language-server definition source for every
    /// function/type/module registered on this engine.
    ///
    /// Standard packages are omitted because the language server supplies them.
    #[must_use]
    pub fn definition_source(&self) -> String {
        self.engine
            .definitions()
            .include_standard_packages(false)
            .single_file()
    }

    /// Validate statically visible direct and method call arities in an entry
    /// script and its installed modules.
    ///
    /// This complements real lifecycle execution because Rhai resolves
    /// functions dynamically and an initial render cannot cover every branch.
    /// The catalog comes from the same registered Engine metadata used by
    /// [`Self::definition_source`].
    ///
    /// # Errors
    ///
    /// Returns a metadata or parse error if the lint catalog cannot be built.
    pub fn lint_known_calls(
        &mut self,
        entry_name: &str,
        entry_source: &str,
        modules: &BTreeMap<ModuleId, String>,
    ) -> Result<Vec<crate::KnownCallDiagnostic>, crate::KnownCallLintError> {
        crate::script_lint::lint_known_calls(&mut self.engine, entry_name, entry_source, modules)
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

    pub fn component_invocations(
        &self,
    ) -> impl ExactSizeIterator<Item = &ComponentInvocationRecipe> {
        self.component_invocations.values()
    }

    pub(crate) fn component_renderer_snapshot(
        &self,
    ) -> Result<BTreeMap<ModuleId, RegisteredComponentRender>, ComponentExportError> {
        self.component_renderers
            .try_borrow()
            .map(|renderers| renderers.clone())
            .map_err(|_| ComponentExportError::Borrowed)
    }

    pub(crate) fn restore_component_renderers(
        &self,
        renderers: BTreeMap<ModuleId, RegisteredComponentRender>,
    ) -> Result<(), ComponentExportError> {
        *self
            .component_renderers
            .try_borrow_mut()
            .map_err(|_| ComponentExportError::Borrowed)? = renderers;
        Ok(())
    }

    /// Clear component exports before evaluating a hot-reload candidate.
    ///
    /// # Errors
    ///
    /// Returns [`ComponentExportError`] if the collector is unavailable.
    pub fn clear_component_exports(&self) -> Result<(), ComponentExportError> {
        self.component_renderers
            .try_borrow_mut()
            .map_err(|_| ComponentExportError::Borrowed)?
            .clear();
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

    /// Register a trusted Rust handler attachable by Rhai through `native_handler`.
    ///
    /// # Errors
    ///
    /// Returns descriptor/duplicate registry errors.
    pub fn register_native_handler(
        &self,
        descriptor: crate::NativeHandlerDescriptor,
        handler: impl FnMut(
            crate::NativeEvent,
            &mut UiRuntimeState,
            &mut gpui::Window,
            &mut gpui::App,
        ) -> Result<crate::EventResponse, String>
        + 'static,
    ) -> Result<(), crate::NativeHandlerError> {
        self.native_handlers.register(descriptor, handler)
    }

    #[must_use]
    pub fn native_handler_registry(&self) -> crate::NativeHandlerRegistry {
        self.native_handlers.clone()
    }

    #[must_use]
    pub fn syntax_registry(&self) -> crate::SyntaxRegistry {
        self.syntax_registry.clone()
    }

    #[must_use]
    pub fn document_runtime_config(&self) -> crate::DocumentRuntimeConfig {
        self.document_runtime.clone()
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
        started: ExecutionTimingStart,
        succeeded: bool,
    ) {
        let duration = started.instant.elapsed();
        let operations = self.operation_total().saturating_sub(started.operations);
        self.timings.borrow_mut().push(ExecutionTiming {
            operation,
            source: source.to_owned(),
            duration,
            operations,
            operation_semantics: OPERATION_SEMANTICS_VERSION,
            slow: duration >= self.slow_threshold,
            succeeded,
        });
    }

    pub(crate) fn begin_execution_session(&self) {
        self.begin_execution_session_from(0);
    }

    fn begin_execution_session_from(&self, inherited_base: u64) {
        self.operation_tracker.begin(inherited_base);
    }

    fn align_execution_session_to(&self, inherited_base: u64) {
        self.operation_tracker.align(inherited_base);
    }

    fn operation_total(&self) -> u64 {
        self.operation_tracker.total.get()
    }

    fn begin_timing(&self) -> ExecutionTimingStart {
        ExecutionTimingStart {
            instant: Instant::now(),
            operations: self.operation_total(),
        }
    }

    fn candidate_generation(&mut self) -> ScriptGeneration {
        self.preparation_generation
            .unwrap_or_else(Self::allocate_generation)
    }

    fn allocate_generation() -> ScriptGeneration {
        static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);
        ScriptGeneration(NEXT_GENERATION.fetch_add(1, Ordering::Relaxed))
    }
}

pub(crate) fn validate_assignment_targets(ast: &AST) -> Result<(), RuntimeError> {
    let mut invalid = None;
    ast.walk(&mut |path| {
        let Some(ASTNode::Stmt(Stmt::Assignment(assignment))) = path.last() else {
            return true;
        };
        if !matches!(
            assignment.1.lhs,
            Expr::ThisPtr(_) | Expr::Variable(..) | Expr::Index(..) | Expr::Dot(..)
        ) {
            invalid = Some(assignment.1.lhs.position());
            return false;
        }
        true
    });
    invalid.map_or(Ok(()), |position| {
        Err(RuntimeError::InvalidAssignmentTarget(position))
    })
}

fn validate_literal_imports(ast: &AST) -> Result<(), RuntimeError> {
    let mut dynamic = None;
    ast.walk(&mut |path| {
        let Some(ASTNode::Stmt(Stmt::Import(import, position))) = path.last() else {
            return true;
        };
        if matches!(import.0, Expr::StringConstant(..)) {
            true
        } else {
            dynamic = Some(*position);
            false
        }
    });
    dynamic.map_or(Ok(()), |position| {
        Err(RuntimeError::Import(format!(
            "Rhai imports must use a literal module string at {position}"
        )))
    })
}

fn register_node_apis(engine: &mut Engine) {
    FuncRegistration::new("text")
        .in_global_namespace()
        .register_into_engine(engine, text_node);
    FuncRegistration::new("text")
        .in_global_namespace()
        .register_into_engine(engine, rich_text_node);
    FuncRegistration::new("span")
        .in_global_namespace()
        .register_into_engine(engine, span_value);
    FuncRegistration::new("canvas")
        .in_global_namespace()
        .register_into_engine(engine, canvas_node);
    FuncRegistration::new("svg")
        .in_global_namespace()
        .register_into_engine(engine, svg_node);
    FuncRegistration::new("box")
        .in_global_namespace()
        .register_into_engine(engine, box_node);
    FuncRegistration::new("fragment")
        .in_global_namespace()
        .register_into_engine(engine, fragment_node);
    FuncRegistration::new("stack")
        .in_global_namespace()
        .register_into_engine(engine, stack_node);
    FuncRegistration::new("handled")
        .in_global_namespace()
        .register_into_engine(engine, || crate::EventResponse::new().stop());
    FuncRegistration::new("propagate")
        .in_global_namespace()
        .register_into_engine(engine, crate::EventResponse::new);
    FuncRegistration::new("event_response")
        .in_global_namespace()
        .register_into_engine(engine, crate::EventResponse::new);
    FuncRegistration::new("column")
        .in_global_namespace()
        .register_into_engine(engine, column_node);
    FuncRegistration::new("row")
        .in_global_namespace()
        .register_into_engine(engine, row_node);
    FuncRegistration::new("error_boundary")
        .in_global_namespace()
        .register_into_engine(engine, error_boundary_node);
    FuncRegistration::new("error_boundary_lazy")
        .in_global_namespace()
        .register_into_engine(engine, lazy_error_boundary_node);
    FuncRegistration::new("asset")
        .in_global_namespace()
        .register_into_engine(engine, asset_id_from_script);
    FuncRegistration::new("image")
        .in_global_namespace()
        .register_into_engine(engine, image_node);
    FuncRegistration::new("image")
        .in_global_namespace()
        .register_into_engine(engine, asset_image_node);
    FuncRegistration::new("directional_image")
        .in_global_namespace()
        .register_into_engine(engine, directional_image_node);
    FuncRegistration::new("directional_image")
        .in_global_namespace()
        .register_into_engine(engine, directional_asset_image_node);
    FuncRegistration::new("image_source")
        .in_global_namespace()
        .register_into_engine(engine, generic_image_node);
    FuncRegistration::new("directional_image_source")
        .in_global_namespace()
        .register_into_engine(engine, generic_directional_image_node);
    FuncRegistration::new("overlay")
        .in_global_namespace()
        .register_into_engine(engine, overlay_node);
    FuncRegistration::new("layer")
        .in_global_namespace()
        .register_into_engine(engine, layer_node);
}

fn register_native_handler_api(engine: &mut Engine, registry: &crate::NativeHandlerRegistry) {
    let registry = registry.clone();
    FuncRegistration::new("native_handler")
        .in_global_namespace()
        .register_into_engine(
            engine,
            move |id: ImmutableString| -> Result<crate::NativeHandlerRef, Box<EvalAltResult>> {
                let id = crate::NativeHandlerId::parse(id.to_string())
                    .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                registry
                    .resolve(&id)
                    .map_err(|error| Box::new(component_render_error(error.to_string())))
            },
        );
}

fn configure_engine_limits(engine: &mut Engine) {
    engine.set_max_call_levels(64);
    engine.set_max_expr_depths(64, 32);
    // Rhai's counter is evaluator-local and cloned into stored callback
    // contexts. The runtime's progress adapter enforces one cumulative budget
    // across nested evaluators and starts delayed callbacks with fresh quota.
    engine.set_max_operations(0);
    engine.set_max_array_size(10_000);
    engine.set_max_map_size(100_000);
    engine.set_max_string_size(1_048_576);
}

fn register_define_component_api(
    engine: &mut Engine,
    exports: &ComponentExportCollector,
    renderers: &ComponentRenderRegistry,
    generation: &Rc<Cell<ScriptGeneration>>,
) {
    let exports = exports.clone();
    let renderers = Rc::clone(renderers);
    let generation = Rc::clone(generation);
    FuncRegistration::new("define_component")
        .in_global_namespace()
        .register_into_engine(
            engine,
            move |mut raw: Map| -> Result<(), Box<EvalAltResult>> {
                let render = raw.remove("render").ok_or_else(|| {
                    Box::new(component_render_error(
                        "define_component requires a named `render` function",
                    ))
                })?;
                if !render.is::<FnPtr>() {
                    return Err(Box::new(component_render_error(
                        "define_component `render` must be a function pointer",
                    )));
                }
                let render = render.cast::<FnPtr>();
                if render.is_anonymous() || render.is_curried() {
                    return Err(Box::new(component_render_error(
                        "formal component render must be an uncurried named function",
                    )));
                }
                let decoded: crate::ComponentDefinition =
                    rhai::serde::from_dynamic(&Dynamic::from_map(raw))?;
                let definition = crate::ComponentDefinition::new(decoded.metadata, decoded.schema)
                    .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                let id = definition.metadata.id.clone();
                exports
                    .register_definition(definition)
                    .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                renderers
                    .try_borrow_mut()
                    .map_err(|_| {
                        Box::new(component_render_error(
                            "component render registry is already borrowed",
                        ))
                    })?
                    .insert(
                        id,
                        RegisteredComponentRender {
                            generation: generation.get(),
                            render,
                        },
                    );
                Ok(())
            },
        );
}

fn register_component_runtime_apis(
    engine: &mut Engine,
    exports: &ComponentExportCollector,
    generation: &Rc<Cell<ScriptGeneration>>,
) -> (ActiveComponentRenderState, ComponentRenderRegistry) {
    let active = Rc::new(RefCell::new(None));
    let renderers = Rc::new(RefCell::new(BTreeMap::new()));
    register_define_component_api(engine, exports, &renderers, generation);
    register_render_component_api(engine, exports, &renderers, &active);
    register_effect_api(engine, &active);
    register_timer_api(engine, &active);
    register_signal_api(engine, &active);
    register_element_ref_api(engine, &active);
    register_virtual_collection_api(engine, &active);
    (active, renderers)
}

fn register_render_component_api(
    engine: &mut Engine,
    exports: &ComponentExportCollector,
    renderers: &ComponentRenderRegistry,
    active: &ActiveComponentRenderState,
) {
    let exports = exports.clone();
    let renderers = Rc::clone(renderers);
    let active = Rc::clone(active);
    FuncRegistration::new("render_component")
        .in_global_namespace()
        .register_into_engine(
            engine,
            move |call: rhai::NativeCallContext<'_>,
                  id: ImmutableString,
                  props: Map|
                  -> Result<UiNode, Box<EvalAltResult>> {
                let id = ModuleId::parse(id.to_string())
                    .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                let generation = active
                    .try_borrow()
                    .map_err(|_| {
                        Box::new(component_render_error(
                            "component render stack is already borrowed",
                        ))
                    })?
                    .as_ref()
                    .ok_or_else(|| {
                        Box::new(component_render_error(
                            "render_component may run only inside view",
                        ))
                    })?
                    .generation;
                let registered = renderers
                    .try_borrow()
                    .map_err(|_| {
                        Box::new(component_render_error(
                            "component render registry is already borrowed",
                        ))
                    })?
                    .get(&id)
                    .cloned()
                    .ok_or_else(|| {
                        Box::new(component_render_error(format!(
                            "component `{id}` has no registered render function"
                        )))
                    })?;
                if registered.generation != generation {
                    return Err(Box::new(component_render_error(format!(
                        "component `{id}` render belongs to stale generation {}",
                        registered.generation
                    ))));
                }
                execute_component_render(
                    &call,
                    &exports,
                    &active,
                    id.as_str().into(),
                    props,
                    &registered.render,
                )
            },
        );
}

#[allow(clippy::too_many_lines)] // One cohesive Rhai component transaction; splitting hides ordering.
fn execute_component_render(
    call: &rhai::NativeCallContext<'_>,
    exports: &ComponentExportCollector,
    active: &ActiveComponentRenderState,
    id: ImmutableString,
    props: Map,
    render: &FnPtr,
) -> Result<UiNode, Box<EvalAltResult>> {
    let generation = active
        .try_borrow()
        .map_err(|_| {
            Box::new(component_render_error(
                "component render stack is already borrowed",
            ))
        })?
        .as_ref()
        .ok_or_else(|| {
            Box::new(component_render_error(
                "formal component rendering may run only inside view",
            ))
        })?
        .generation;
    let (component, mut invocation) = resolve_component_invocation(exports, id, props, generation)?;
    if render.is_anonymous() || render.is_curried() {
        return Err(Box::new(component_render_error(
            "formal component render must be an uncurried named function",
        )));
    }
    let part_styles = component_part_styles(&invocation.props);
    let recipe_props = invocation.retained_props.clone();
    let declared_key = invocation.key.clone();
    let render_recipe = render.clone();
    let (path, caller_context) = reserve_component_path(call, &component, &invocation, active)?;
    if let Some(node) = try_reuse_component_subtree(
        active,
        &path,
        &caller_context,
        &component,
        &invocation,
        render,
    )? {
        return Ok(node);
    }
    let native_context = crate::invocation::ScriptInvocationContext::capture(call);
    let caller_binding = caller_component_binding(call, &caller_context);
    bind_component_callback_props(
        &mut invocation.props,
        &component.schema.props,
        &caller_binding,
    );
    bind_component_node_props(
        &mut invocation.props,
        &component.schema.props,
        &caller_binding,
    );
    let event_callbacks = component_event_callbacks(&component, &invocation.props);
    let recipe_event_callbacks = event_callbacks.clone();
    let script_props = invocation.props.clone();
    let context = enter_component_render(
        &path,
        &component,
        &invocation,
        native_context.clone(),
        active,
    )?;
    let recipe_component_context = context.clone();
    let result = render.call_within_context::<UiNode>(call, (context, invocation.props));
    register_component_event_callbacks(
        active,
        &path,
        &caller_context,
        event_callbacks,
        &native_context,
    )?;
    let (environment, reusable) = component_render_metadata(active, &recipe_component_context)?;
    leave_component_render(active)?;
    let mut node = result?;
    node = node.with_owned_part_styles(part_styles.clone());
    let snapshot = crate::node::ComponentOwnedSnapshot::default();
    node = node.with_component_root_snapshot(path.clone(), snapshot.reference());
    node.bind_component_scope(
        &path,
        recipe_component_context.component_incarnation(),
        &component.schema.events,
        Some(&native_context),
    );
    register_component_invocation(
        active,
        ComponentInvocationRecipe {
            path: path.clone(),
            parent: caller_context.component_path().clone(),
            component: component.metadata.id.clone(),
            export: component.metadata.export.clone(),
            declared_key,
            props: recipe_props,
            script_props,
            component_context: recipe_component_context,
            caller_context: caller_context.clone(),
            part_styles,
            event_callbacks: recipe_event_callbacks,
            component_events: component.schema.events.clone(),
            declared_effects: component.schema.effects.clone(),
            render: render_recipe,
            snapshot,
            context: native_context.clone(),
            generation,
            environment,
            reusable,
        },
    )?;
    Ok(node)
}

fn caller_component_binding(
    call: &rhai::NativeCallContext<'_>,
    caller_context: &UiContext,
) -> ComponentCallbackBinding {
    ComponentCallbackBinding {
        component: caller_context.component_path().clone(),
        incarnation: caller_context.component_incarnation(),
        events: caller_context.event_schemas().clone(),
        context: Some(
            caller_context
                .native_context()
                .cloned()
                .unwrap_or_else(|| crate::invocation::ScriptInvocationContext::capture_entry(call)),
        ),
    }
}

fn component_render_metadata(
    active: &ActiveComponentRenderState,
    context: &UiContext,
) -> Result<(ComponentRenderEnvironment, bool), Box<EvalAltResult>> {
    let guard = active.try_borrow().map_err(|_| {
        Box::new(component_render_error(
            "component render stack is already borrowed",
        ))
    })?;
    let render = guard.as_ref().ok_or_else(|| {
        Box::new(component_render_error(
            "formal component rendering may run only inside view",
        ))
    })?;
    Ok((render.environment, context.component_render_is_reusable()))
}

fn try_reuse_component_subtree(
    shared: &ActiveComponentRenderState,
    path: &ComponentInstancePath,
    caller_context: &UiContext,
    component: &crate::ComponentDefinition,
    invocation: &crate::ComponentInvocation,
    render: &FnPtr,
) -> Result<Option<UiNode>, Box<EvalAltResult>> {
    let mut guard = shared.try_borrow_mut().map_err(|_| {
        Box::new(component_render_error(
            "component render stack is already borrowed",
        ))
    })?;
    let active = guard.as_mut().ok_or_else(|| {
        Box::new(component_render_error(
            "formal component rendering may run only inside view",
        ))
    })?;
    let Some(scope) =
        reusable_component_scope(active, path, caller_context, component, invocation, render)
    else {
        return Ok(None);
    };

    let reused_components = scope.invocations.len();
    for candidate in scope.invocations.keys() {
        let retained = active.transaction.retain_existing(candidate);
        debug_assert!(
            retained,
            "reuse candidates were checked against the state snapshot"
        );
    }
    active.seen.extend(scope.invocations.keys().cloned());
    active.invocations.extend(scope.invocations);
    active.event_handlers.extend(scope.event_handlers);
    active.effects.extend(scope.resources.effects);
    active.timers.extend(scope.resources.timers);
    active.signals.extend(scope.resources.signals);
    active.element_refs.extend(scope.resources.element_refs);
    active
        .virtual_collections
        .extend(scope.resources.virtual_collections);
    active
        .reused
        .push((component.metadata.id.to_string(), reused_components));
    Ok(Some(scope.node))
}

fn reusable_component_scope(
    active: &ActiveComponentRender,
    path: &ComponentInstancePath,
    caller_context: &UiContext,
    component: &crate::ComponentDefinition,
    invocation: &crate::ComponentInvocation,
    render: &FnPtr,
) -> Option<ReusedComponentScope> {
    let reuse = active.reuse.as_ref()?;
    if reuse.dirty.iter().any(|dirty| dirty.is_within(path)) {
        return None;
    }
    let recipe = reuse.invocations.get(path)?;
    if !recipe.reusable
        || recipe.generation != active.generation
        || recipe.environment != active.environment
        || recipe.parent != *caller_context.component_path()
        || recipe.component != component.metadata.id
        || recipe.export != component.metadata.export
        || recipe.declared_key != invocation.key
        || recipe.render.fn_name() != render.fn_name()
        || !recipe.props.reusable_eq(&invocation.retained_props)
    {
        return None;
    }
    let invocations = reuse
        .invocations
        .range(path.clone()..)
        .take_while(|(candidate, _)| candidate.is_within(path))
        .map(|(candidate, recipe)| (candidate.clone(), recipe.clone()))
        .collect::<BTreeMap<_, _>>();
    if invocations.is_empty()
        || invocations.values().any(|recipe| !recipe.reusable)
        || invocations
            .keys()
            .any(|candidate| !active.state_snapshot.contains_instance(candidate))
    {
        return None;
    }
    let node = if let Some(snapshot) = recipe.snapshot.current() {
        let mut node = snapshot.as_ref().clone();
        node.hydrate_component_subtrees();
        node
    } else {
        reuse.subtrees.get(&reuse.previous_root, path)?.clone()
    };
    Some(ReusedComponentScope {
        node,
        invocations,
        event_handlers: reuse
            .event_handlers
            .range((path.clone(), String::new())..)
            .take_while(|((candidate, _), _)| candidate.is_within(path))
            .map(|(id, callback)| (id.clone(), callback.clone()))
            .collect(),
        resources: reused_component_resources(reuse, path),
    })
}

fn reused_component_resources(
    reuse: &ComponentReuseSnapshot,
    path: &ComponentInstancePath,
) -> ReusedComponentResources {
    ReusedComponentResources {
        effects: reuse
            .effects
            .iter()
            .filter(|(id, _)| id.component().is_within(path))
            .map(|(id, descriptor)| (id.clone(), descriptor.clone()))
            .collect(),
        timers: reuse
            .timers
            .iter()
            .filter(|(id, _)| id.component().is_within(path))
            .map(|(id, descriptor)| (id.clone(), descriptor.clone()))
            .collect(),
        signals: reuse
            .signals
            .iter()
            .filter(|(id, _)| id.component().is_within(path))
            .map(|(id, descriptor)| (id.clone(), descriptor.clone()))
            .collect(),
        element_refs: reuse
            .element_refs
            .iter()
            .filter(|id| id.component().is_within(path))
            .cloned()
            .collect(),
        virtual_collections: reuse
            .virtual_collections
            .iter()
            .filter(|(id, _)| id.component.is_within(path))
            .map(|(id, recipe)| (id.clone(), recipe.clone()))
            .collect(),
    }
}

fn register_component_invocation(
    active: &ActiveComponentRenderState,
    recipe: ComponentInvocationRecipe,
) -> Result<(), Box<EvalAltResult>> {
    let mut guard = active.try_borrow_mut().map_err(|_| {
        Box::new(component_render_error(
            "component render stack is already borrowed",
        ))
    })?;
    let active = guard.as_mut().ok_or_else(|| {
        Box::new(component_render_error(
            "formal component rendering may run only inside view",
        ))
    })?;
    active.invocations.insert(recipe.path.clone(), recipe);
    Ok(())
}

fn resolve_component_invocation(
    exports: &ComponentExportCollector,
    id: ImmutableString,
    props: Map,
    generation: ScriptGeneration,
) -> Result<(crate::ComponentDefinition, crate::ComponentInvocation), Box<EvalAltResult>> {
    let id: String = id.into();
    let id =
        ModuleId::parse(id).map_err(|error| Box::new(component_render_error(error.to_string())))?;
    let registry = exports
        .snapshot()
        .map_err(|error| Box::new(component_render_error(error.to_string())))?;
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
        .invoke_with_generation(key, props, generation)
        .map_err(|error| Box::new(component_render_error(error.to_string())))?;
    Ok((component, invocation))
}

fn reserve_component_path(
    call: &rhai::NativeCallContext<'_>,
    component: &crate::ComponentDefinition,
    invocation: &crate::ComponentInvocation,
    shared: &ActiveComponentRenderState,
) -> Result<(ComponentInstancePath, UiContext), Box<EvalAltResult>> {
    let mut guard = shared.try_borrow_mut().map_err(|_| {
        Box::new(component_render_error(
            "component render stack is already borrowed",
        ))
    })?;
    let active = guard.as_mut().ok_or_else(|| {
        Box::new(component_render_error(
            "formal component rendering may run only inside view",
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
    Ok((path, caller_context))
}

fn enter_component_render(
    path: &ComponentInstancePath,
    component: &crate::ComponentDefinition,
    invocation: &crate::ComponentInvocation,
    native_context: crate::invocation::ScriptInvocationContext,
    shared: &ActiveComponentRenderState,
) -> Result<UiContext, Box<EvalAltResult>> {
    let mut guard = shared.try_borrow_mut().map_err(|_| {
        Box::new(component_render_error(
            "component render stack is already borrowed",
        ))
    })?;
    let active = guard.as_mut().ok_or_else(|| {
        Box::new(component_render_error(
            "formal component rendering may run only inside view",
        ))
    })?;
    active
        .transaction
        .mount(path.clone(), &component.schema.state)
        .map_err(|error| Box::new(component_render_error(error.to_string())))?;
    let global_part_styles = {
        let mut runtime = active
            .root_context
            .runtime()
            .try_borrow_mut()
            .map_err(|_| Box::new(component_render_error("UI state is already borrowed")))?;
        runtime
            .component_state
            .mount_instance(path.clone(), &component.schema.state)
            .map_err(|error| Box::new(component_render_error(error.to_string())))?;
        runtime
            .component_styles()
            .component(&component.metadata.id)
            .cloned()
            .unwrap_or_default()
    };
    let context = active
        .root_context
        .for_component(path.clone(), component.schema.events.clone())
        .with_component_styles(
            global_part_styles,
            component_root_style(&invocation.props),
            component_part_styles(&invocation.props),
        )
        .with_native_context(Some(native_context))
        .with_generation(active.generation);
    active.stack.push(path.clone());
    active.contexts.push(context.clone());
    active
        .effect_keys
        .push(Some(component.schema.effects.clone()));
    Ok(context)
}

fn leave_component_render(active: &ActiveComponentRenderState) -> Result<(), Box<EvalAltResult>> {
    let mut guard = active.try_borrow_mut().map_err(|_| {
        Box::new(component_render_error(
            "component render stack is already borrowed",
        ))
    })?;
    if let Some(active) = guard.as_mut() {
        active.stack.pop();
        active.contexts.pop();
        active.effect_keys.pop();
    }
    Ok(())
}

fn register_effect_api(engine: &mut Engine, active: &ActiveComponentRenderState) {
    let active = Rc::clone(active);
    FuncRegistration::new("effect")
        .in_global_namespace()
        .register_into_engine(
            engine,
            move |call: rhai::NativeCallContext<'_>,
                  key: ImmutableString,
                  dependencies: Dynamic,
                  start: FnPtr,
                  cleanup: FnPtr|
                  -> Result<(), Box<EvalAltResult>> {
                let key = key.to_string();
                let (component, context, generation, declared) = {
                    let guard = active.try_borrow().map_err(|_| {
                        Box::new(component_render_error(
                            "component render stack is already borrowed",
                        ))
                    })?;
                    let render = guard.as_ref().ok_or_else(|| {
                        Box::new(component_render_error(
                            "effect may run only during formal component render",
                        ))
                    })?;
                    let context = render.contexts.last().cloned().ok_or_else(|| {
                        Box::new(component_render_error(
                            "component render context stack is empty",
                        ))
                    })?;
                    let declared =
                        render
                            .effect_keys
                            .last()
                            .cloned()
                            .flatten()
                            .ok_or_else(|| {
                                Box::new(component_render_error(
                                    "effects may be declared only by formal components",
                                ))
                            })?;
                    (
                        context.component_path().clone(),
                        context,
                        render.generation,
                        declared,
                    )
                };
                if !declared.contains(&key) {
                    return Err(Box::new(component_render_error(
                        crate::EffectError::Undeclared { component, key }.to_string(),
                    )));
                }
                let id = crate::EffectId::new(component.clone(), key.clone())
                    .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                let dependencies = UiValue::from_dynamic(dependencies)
                    .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                let native_context = crate::invocation::ScriptInvocationContext::capture(&call);
                let mut start = ScriptCallback::try_from_fn_ptr(start, generation)
                    .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                start.bind_component_scope_if_unset(
                    &component,
                    context.component_incarnation(),
                    context.event_schemas().clone(),
                );
                start.bind_native_context_if_unset(native_context.clone());
                let mut cleanup = ScriptCallback::try_from_fn_ptr(cleanup, generation)
                    .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                cleanup.bind_component_scope_if_unset(
                    &component,
                    context.component_incarnation(),
                    context.event_schemas().clone(),
                );
                cleanup.bind_native_context_if_unset(native_context);
                let descriptor =
                    crate::EffectDescriptor::new(id.clone(), dependencies, start, cleanup);
                let mut guard = active.try_borrow_mut().map_err(|_| {
                    Box::new(component_render_error(
                        "component render stack is already borrowed",
                    ))
                })?;
                let render = guard.as_mut().ok_or_else(|| {
                    Box::new(component_render_error(
                        "effect may run only during formal component render",
                    ))
                })?;
                if render.effects.insert(id, descriptor).is_some() {
                    return Err(Box::new(component_render_error(
                        crate::EffectError::Duplicate { component, key }.to_string(),
                    )));
                }
                Ok(())
            },
        );
}

fn register_timer_api(engine: &mut Engine, active: &ActiveComponentRenderState) {
    let active = Rc::clone(active);
    FuncRegistration::new("timeout")
        .in_global_namespace()
        .register_into_engine(
            engine,
            move |call: rhai::NativeCallContext<'_>,
                  key: ImmutableString,
                  delay_ms: rhai::INT,
                  paused: bool,
                  callback: FnPtr,
                  payload: Dynamic|
                  -> Result<(), Box<EvalAltResult>> {
                let (component, context, generation) = {
                    let guard = active.try_borrow().map_err(|_| {
                        Box::new(component_render_error(
                            "component render stack is already borrowed",
                        ))
                    })?;
                    let render = guard.as_ref().ok_or_else(|| {
                        Box::new(component_render_error(
                            "timeout may run only during formal component render",
                        ))
                    })?;
                    let context = render.contexts.last().cloned().ok_or_else(|| {
                        Box::new(component_render_error(
                            "component render context stack is empty",
                        ))
                    })?;
                    (context.component_path().clone(), context, render.generation)
                };
                let delay_ms = u64::try_from(delay_ms).map_err(|_| {
                    Box::new(component_render_error(
                        "timeout delay must be a positive integer",
                    ))
                })?;
                let id = crate::TimerId::new(component.clone(), key.to_string())
                    .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                let payload = UiValue::from_dynamic(payload)
                    .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                let native_context = crate::invocation::ScriptInvocationContext::capture(&call);
                let mut callback = ScriptCallback::try_from_fn_ptr(callback, generation)
                    .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                callback.bind_component_scope_if_unset(
                    &component,
                    context.component_incarnation(),
                    context.event_schemas().clone(),
                );
                callback.bind_native_context_if_unset(native_context);
                let descriptor = crate::TimerDescriptor::new(
                    id.clone(),
                    Duration::from_millis(delay_ms),
                    paused,
                    callback,
                    payload,
                )
                .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                let mut guard = active.try_borrow_mut().map_err(|_| {
                    Box::new(component_render_error(
                        "component render stack is already borrowed",
                    ))
                })?;
                let render = guard.as_mut().ok_or_else(|| {
                    Box::new(component_render_error(
                        "timeout may run only during formal component render",
                    ))
                })?;
                if render.timers.insert(id.clone(), descriptor).is_some() {
                    return Err(Box::new(component_render_error(format!(
                        "timeout `{}` is declared more than once in `{}`",
                        id.key(),
                        id.component()
                    ))));
                }
                Ok(())
            },
        );
}

fn register_signal_api(engine: &mut Engine, active: &ActiveComponentRenderState) {
    let scalar_active = Rc::clone(active);
    FuncRegistration::new("signal")
        .in_global_namespace()
        .register_into_engine(
            engine,
            move |key: ImmutableString,
                  initial: Dynamic|
                  -> Result<crate::NativeSignal, Box<EvalAltResult>> {
                let key = key.to_string();
                let initial = crate::SignalValue::from_dynamic(initial)
                    .map_err(|error| Box::new(crate::signal::signal_runtime_error(&error)))?;
                declare_signal(&scalar_active, key, initial)
            },
        );

    let optional_active = Rc::clone(active);
    FuncRegistration::new("optional_float_signal")
        .in_global_namespace()
        .register_into_engine(
            engine,
            move |key: ImmutableString| -> Result<crate::NativeSignal, Box<EvalAltResult>> {
                declare_signal(
                    &optional_active,
                    key.to_string(),
                    crate::SignalValue::OptionalFloat(None),
                )
            },
        );
}

fn declare_signal(
    active: &ActiveComponentRenderState,
    key: String,
    initial: crate::SignalValue,
) -> Result<crate::NativeSignal, Box<EvalAltResult>> {
    let mut guard = active.try_borrow_mut().map_err(|_| {
        Box::new(crate::signal::signal_runtime_error(
            &"component render stack is already borrowed",
        ))
    })?;
    let render = guard.as_mut().ok_or_else(|| {
        Box::new(crate::signal::signal_runtime_error(
            &"signal may run only during formal component render",
        ))
    })?;
    if render.effect_keys.last().and_then(Option::as_ref).is_none() {
        return Err(Box::new(crate::signal::signal_runtime_error(
            &"signals may be declared only by formal components",
        )));
    }
    let context = render.contexts.last().ok_or_else(|| {
        Box::new(crate::signal::signal_runtime_error(
            &"component render context stack is empty",
        ))
    })?;
    let component = context.component_path().clone();
    let incarnation = context.component_incarnation();
    if render
        .signals
        .keys()
        .any(|id| id.component() == &component && id.key() == key)
    {
        return Err(Box::new(crate::signal::signal_runtime_error(
            &crate::SignalError::Duplicate { component, key },
        )));
    }
    let id = crate::SignalId::new_scoped(component, incarnation, key, initial.kind())
        .map_err(|error| Box::new(crate::signal::signal_runtime_error(&error)))?;
    let signal = crate::NativeSignal::new(id.clone());
    render
        .signals
        .insert(id, crate::signal::SignalDescriptor::new(initial));
    Ok(signal)
}

fn register_element_ref_api(engine: &mut Engine, active: &ActiveComponentRenderState) {
    let active = Rc::clone(active);
    FuncRegistration::new("element_ref")
        .in_global_namespace()
        .register_into_engine(
            engine,
            move |key: ImmutableString| -> Result<crate::ElementRef, Box<EvalAltResult>> {
                let key = key.to_string();
                let mut guard = active.try_borrow_mut().map_err(|_| {
                    Box::new(component_render_error(
                        "component render stack is already borrowed",
                    ))
                })?;
                let render = guard.as_mut().ok_or_else(|| {
                    Box::new(component_render_error(
                        "element_ref may run only during formal component render",
                    ))
                })?;
                if render.effect_keys.last().and_then(Option::as_ref).is_none() {
                    return Err(Box::new(component_render_error(
                        "element refs may be declared only by formal components",
                    )));
                }
                let component = render
                    .contexts
                    .last()
                    .ok_or_else(|| {
                        Box::new(component_render_error(
                            "component render context stack is empty",
                        ))
                    })?
                    .component_path()
                    .clone();
                let id = crate::ElementRefId::new(component.clone(), key.clone())
                    .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                if !render.element_refs.insert(id.clone()) {
                    return Err(Box::new(component_render_error(format!(
                        "element ref `{key}` is declared more than once by component `{component}`"
                    ))));
                }
                Ok(crate::ElementRef::new(id))
            },
        );
}

#[derive(Debug)]
struct DecodedVirtualCollection {
    key: String,
    label: String,
    data: crate::VirtualCollectionData,
    estimated_height: f64,
    height: Option<f64>,
    overdraw_pixels: f64,
    bottom_align: bool,
    follow_tail: bool,
    reveal_key: Option<String>,
    sticky_headers: Arc<BTreeSet<usize>>,
}

impl DecodedVirtualCollection {
    fn into_spec(
        self,
        id: crate::VirtualCollectionId,
        realized: BTreeMap<usize, UiNode>,
    ) -> crate::VirtualCollectionNodeSpec {
        crate::VirtualCollectionNodeSpec {
            id,
            label: self.label,
            data: self.data,
            realized,
            estimated_height: self.estimated_height,
            height: self.height,
            overdraw_pixels: self.overdraw_pixels,
            bottom_align: self.bottom_align,
            follow_tail: self.follow_tail,
            reveal_key: self.reveal_key,
            sticky_headers: self.sticky_headers,
        }
    }
}

fn decode_virtual_collection(
    mut config: Map,
) -> Result<DecodedVirtualCollection, Box<EvalAltResult>> {
    let key = collection_string(&mut config, "key")?;
    let label = collection_optional_string(&mut config, "label")?.unwrap_or_default();
    let estimated_height = collection_positive(&mut config, "estimated_height")?;
    let height = collection_optional_number(&mut config, "height")?;
    let fill_height = collection_optional_bool(&mut config, "fill_height")?.unwrap_or(false);
    let height = match (height, fill_height) {
        (Some(_), true) => {
            return Err(Box::new(component_render_error(
                "virtual collection must specify either `height` or `fill_height`, not both",
            )));
        }
        (None, false) => {
            return Err(Box::new(component_render_error(
                "virtual collection requires `height` or `fill_height: true`",
            )));
        }
        (None, true) => None,
        (Some(height), false) if height.is_finite() && height > 0.0 => Some(height),
        (Some(_), false) => {
            return Err(Box::new(component_render_error(
                "virtual collection `height` must be finite and positive",
            )));
        }
    };
    let overdraw_pixels = collection_optional_number(&mut config, "overdraw_pixels")?
        .unwrap_or(estimated_height * 2.0);
    if !overdraw_pixels.is_finite() || !(0.0..=10_000.0).contains(&overdraw_pixels) {
        return Err(Box::new(component_render_error(
            "virtual collection overdraw_pixels must be between 0 and 10000",
        )));
    }
    let alignment =
        collection_optional_string(&mut config, "alignment")?.unwrap_or_else(|| "top".to_owned());
    let bottom_align = match alignment.as_str() {
        "top" => false,
        "bottom" => true,
        _ => {
            return Err(Box::new(component_render_error(
                "virtual collection alignment must be `top` or `bottom`",
            )));
        }
    };
    let follow_tail = collection_optional_bool(&mut config, "follow_tail")?.unwrap_or(false);
    let data = collection_data(&mut config)?;
    let reveal_key = collection_optional_string(&mut config, "reveal_key")?;
    if reveal_key.as_deref() == Some("") {
        return Err(Box::new(component_render_error(
            "virtual collection `reveal_key` must not be empty",
        )));
    }
    if let Some(reveal_key) = reveal_key.as_deref()
        && !(0..data.len()).any(|index| data.key(index) == Some(reveal_key))
    {
        return Err(Box::new(component_render_error(format!(
            "virtual collection reveal key `{reveal_key}` is not present in its data"
        ))));
    }
    if follow_tail && reveal_key.is_some() {
        return Err(Box::new(component_render_error(
            "virtual collection must not combine `follow_tail` with `reveal_key`",
        )));
    }
    let sticky_headers = collection_optional_indices(&mut config, "sticky_headers")?
        .map_or_else(|| data.sticky_headers(), Arc::new);
    if let Some(index) = sticky_headers.iter().find(|index| **index >= data.len()) {
        return Err(Box::new(component_render_error(format!(
            "virtual collection sticky header index {index} is outside {} items",
            data.len()
        ))));
    }
    if bottom_align && !sticky_headers.is_empty() {
        return Err(Box::new(component_render_error(
            "sticky headers require top-aligned virtual collections",
        )));
    }
    if let Some((unknown, _)) = config.into_iter().next() {
        return Err(Box::new(component_render_error(format!(
            "unknown virtual collection field `{unknown}`"
        ))));
    }
    Ok(DecodedVirtualCollection {
        key,
        label,
        data,
        estimated_height,
        height,
        overdraw_pixels,
        bottom_align,
        follow_tail,
        reveal_key,
        sticky_headers,
    })
}

fn register_virtual_collection_api(engine: &mut Engine, active: &ActiveComponentRenderState) {
    let active = Rc::clone(active);
    FuncRegistration::new("virtual_collection")
        .in_global_namespace()
        .register_into_engine(
            engine,
            move |call: rhai::NativeCallContext<'_>,
                  config: Map,
                  renderer: FnPtr|
                  -> Result<UiNode, Box<EvalAltResult>> {
                validate_virtual_renderer(&renderer)?;
                let decoded = decode_virtual_collection(config)?;

                let VirtualCollectionContext {
                    component,
                    context,
                    generation,
                    events,
                } = virtual_collection_context(&active)?;
                let id = crate::VirtualCollectionId {
                    component: component.clone(),
                    key: decoded.key.clone(),
                };
                let collection_context = context
                    .for_component(
                        component.child("VirtualCollection", decoded.key.clone()),
                        BTreeMap::new(),
                    )
                    .with_generation(generation);
                let native_context = crate::invocation::ScriptInvocationContext::capture(&call);
                let mut callback = ScriptCallback::try_from_fn_ptr(renderer.clone(), generation)
                    .map_err(|error| Box::new(component_render_error(error.to_string())))?;
                callback.bind_component_scope_if_unset(
                    &component,
                    context.component_incarnation(),
                    events,
                );
                callback.bind_native_context_if_unset(native_context);

                let realized = realize_seeded_virtual_collection(
                    &call,
                    &renderer,
                    &collection_context,
                    &active,
                    &id,
                    &decoded,
                )?;
                let recipe = VirtualCollectionRecipe {
                    id: id.clone(),
                    data: decoded.data.clone(),
                    renderer: callback,
                    context: collection_context,
                    event_context: context,
                    generation,
                };
                let mut guard = active.try_borrow_mut().map_err(|_| {
                    Box::new(component_render_error(
                        "component render stack is already borrowed",
                    ))
                })?;
                let render = guard.as_mut().ok_or_else(|| {
                    Box::new(component_render_error(
                        "virtual_collection may run only during view render",
                    ))
                })?;
                if render
                    .virtual_collections
                    .insert(id.clone(), recipe)
                    .is_some()
                {
                    return Err(Box::new(component_render_error(format!(
                        "virtual collection `{}` is declared more than once in `{}`",
                        decoded.key, id.component
                    ))));
                }
                Ok(UiNode::virtual_collection(decoded.into_spec(id, realized)))
            },
        );
}

fn validate_virtual_renderer(renderer: &FnPtr) -> Result<(), Box<EvalAltResult>> {
    if renderer.is_anonymous() {
        Err(Box::new(component_render_error(
            "virtual collection item renderer must be a named function",
        )))
    } else {
        Ok(())
    }
}

fn realize_initial_collection(
    call: &rhai::NativeCallContext<'_>,
    renderer: &FnPtr,
    context: &UiContext,
    data: &crate::VirtualCollectionData,
    indices: &BTreeSet<usize>,
) -> Result<BTreeMap<usize, UiNode>, Box<EvalAltResult>> {
    let mut realized = BTreeMap::new();
    for index in indices.iter().copied().filter(|index| *index < data.len()) {
        let item = data
            .item(index)
            .map_err(|error| Box::new(component_render_error(error.to_string())))?
            .ok_or_else(|| {
                Box::new(component_render_error(format!(
                    "virtual collection item {index} disappeared"
                )))
            })?;
        let (item_key, payload) = collection_payload(&item, index)?;
        let node = renderer
            .call_within_context::<UiNode>(call, (context.clone(), payload))?
            .with_key(item_key);
        realized.insert(index, node);
    }
    Ok(realized)
}

fn realize_seeded_virtual_collection(
    call: &rhai::NativeCallContext<'_>,
    renderer: &FnPtr,
    context: &UiContext,
    active: &ActiveComponentRenderState,
    id: &crate::VirtualCollectionId,
    decoded: &DecodedVirtualCollection,
) -> Result<BTreeMap<usize, UiNode>, Box<EvalAltResult>> {
    let previous_metrics = context
        .runtime()
        .try_borrow()
        .map_err(|_| {
            Box::new(component_render_error(
                "UI state is already borrowed while seeding a virtual collection",
            ))
        })?
        .virtual_requests
        .metrics(id);
    let viewport = decoded.height.unwrap_or_else(|| {
        previous_metrics
            .as_ref()
            .map_or(decoded.estimated_height, |metrics| {
                if metrics.viewport_height > 0.0 {
                    metrics.viewport_height
                } else {
                    decoded.estimated_height
                }
            })
    });
    let count = nonnegative_usize(
        ((viewport + decoded.overdraw_pixels) / decoded.estimated_height).ceil() + 1.0,
    )
    .min(decoded.data.len());
    let indices =
        virtual_collection_seed_indices(active, id, decoded, count, previous_metrics.as_ref())?;
    enter_virtual_collection_scope(active, context.clone())?;
    let realized = realize_initial_collection(call, renderer, context, &decoded.data, &indices);
    leave_component_render(active)?;
    realized
}

fn virtual_collection_seed_indices(
    active: &ActiveComponentRenderState,
    id: &crate::VirtualCollectionId,
    decoded: &DecodedVirtualCollection,
    count: usize,
    metrics: Option<&crate::VirtualCollectionMetrics>,
) -> Result<BTreeSet<usize>, Box<EvalAltResult>> {
    let mut indices = BTreeSet::new();
    let guard = active.try_borrow().map_err(|_| {
        Box::new(component_render_error(
            "component render stack is already borrowed",
        ))
    })?;
    let reuse = guard.as_ref().and_then(|render| render.reuse.as_ref());
    let previous = reuse.and_then(|reuse| reuse.previous_root.virtual_collection_spec(id));
    let preserves_scroll = previous.is_some_and(|previous| {
        previous.estimated_height.to_bits() == decoded.estimated_height.to_bits()
            && previous.overdraw_pixels.to_bits() == decoded.overdraw_pixels.to_bits()
            && previous.bottom_align == decoded.bottom_align
            && same_collection_key_order(&previous.data, &decoded.data)
    });

    if preserves_scroll {
        if let Some(previous) = previous {
            indices.extend(previous.realized.keys().copied());
        }
        if let Some(metrics) = metrics {
            extend_virtual_window(
                &mut indices,
                metrics.scroll_item,
                count,
                decoded.data.len(),
                virtual_overdraw_items(decoded),
            );
        }
    } else if decoded.reveal_key.is_none() && !decoded.bottom_align && !decoded.follow_tail {
        extend_virtual_window(&mut indices, 0, count, decoded.data.len(), 0);
    }
    drop(guard);

    if let Some(reveal) = decoded.reveal_key.as_deref()
        && let Some(index) =
            (0..decoded.data.len()).find(|index| decoded.data.key(*index) == Some(reveal))
    {
        extend_virtual_window(
            &mut indices,
            index,
            count,
            decoded.data.len(),
            virtual_overdraw_items(decoded),
        );
        if let Some(header) = decoded.sticky_headers.range(..=index).next_back() {
            indices.insert(*header);
        }
    }
    if (decoded.bottom_align || decoded.follow_tail) && !decoded.data.is_empty() {
        let start = decoded.data.len().saturating_sub(count);
        indices.extend(start..decoded.data.len());
    }
    Ok(indices)
}

fn same_collection_key_order(
    current: &crate::VirtualCollectionData,
    next: &crate::VirtualCollectionData,
) -> bool {
    current.len() == next.len()
        && (0..current.len()).all(|index| current.key(index) == next.key(index))
}

fn virtual_overdraw_items(decoded: &DecodedVirtualCollection) -> usize {
    nonnegative_usize((decoded.overdraw_pixels / decoded.estimated_height).ceil())
}

fn extend_virtual_window(
    indices: &mut BTreeSet<usize>,
    anchor: usize,
    count: usize,
    len: usize,
    leading: usize,
) {
    let start = anchor.saturating_sub(leading).min(len);
    let end = anchor.saturating_add(count).min(len);
    indices.extend(start..end);
}

struct VirtualCollectionContext {
    component: ComponentInstancePath,
    context: UiContext,
    generation: ScriptGeneration,
    events: BTreeMap<String, EventSchema>,
}

fn enter_virtual_collection_scope(
    active: &ActiveComponentRenderState,
    context: UiContext,
) -> Result<(), Box<EvalAltResult>> {
    let mut guard = active.try_borrow_mut().map_err(|_| {
        Box::new(component_render_error(
            "component render stack is already borrowed",
        ))
    })?;
    let render = guard.as_mut().ok_or_else(|| {
        Box::new(component_render_error(
            "virtual_collection may run only during view render",
        ))
    })?;
    render.stack.push(context.component_path().clone());
    render.contexts.push(context);
    render.effect_keys.push(None);
    Ok(())
}

fn virtual_collection_context(
    active: &ActiveComponentRenderState,
) -> Result<VirtualCollectionContext, Box<EvalAltResult>> {
    let guard = active.try_borrow().map_err(|_| {
        Box::new(component_render_error(
            "component render stack is already borrowed",
        ))
    })?;
    let render = guard.as_ref().ok_or_else(|| {
        Box::new(component_render_error(
            "virtual_collection may run only during view render",
        ))
    })?;
    let context = render.contexts.last().cloned().ok_or_else(|| {
        Box::new(component_render_error(
            "component render context stack is empty",
        ))
    })?;
    Ok(VirtualCollectionContext {
        component: context.component_path().clone(),
        context: context.clone(),
        generation: render.generation,
        events: context.event_schemas().clone(),
    })
}

fn collection_string(config: &mut Map, name: &str) -> Result<String, Box<EvalAltResult>> {
    config
        .remove(name)
        .and_then(Dynamic::try_cast::<ImmutableString>)
        .map(|value| value.to_string())
        .ok_or_else(|| {
            Box::new(component_render_error(format!(
                "virtual collection `{name}` must be a string"
            )))
        })
}

fn collection_optional_string(
    config: &mut Map,
    name: &str,
) -> Result<Option<String>, Box<EvalAltResult>> {
    config
        .remove(name)
        .map(|value| {
            value
                .try_cast::<ImmutableString>()
                .map(|value| value.to_string())
                .ok_or_else(|| {
                    Box::new(component_render_error(format!(
                        "virtual collection `{name}` must be a string"
                    )))
                })
        })
        .transpose()
}

fn collection_optional_number(
    config: &mut Map,
    name: &str,
) -> Result<Option<f64>, Box<EvalAltResult>> {
    config
        .remove(name)
        .map(|value| {
            if value.is::<rhai::FLOAT>() {
                Ok(value.cast::<rhai::FLOAT>())
            } else if value.is::<rhai::INT>() {
                value
                    .cast::<rhai::INT>()
                    .to_string()
                    .parse::<f64>()
                    .map_err(|_| Box::new(component_render_error("invalid collection number")))
            } else {
                Err(Box::new(component_render_error(format!(
                    "virtual collection `{name}` must be a number"
                ))))
            }
        })
        .transpose()
}

fn collection_positive(config: &mut Map, name: &str) -> Result<f64, Box<EvalAltResult>> {
    let value = collection_optional_number(config, name)?.ok_or_else(|| {
        Box::new(component_render_error(format!(
            "virtual collection `{name}` is required"
        )))
    })?;
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(Box::new(component_render_error(format!(
            "virtual collection `{name}` must be finite and positive"
        ))))
    }
}

fn collection_optional_bool(
    config: &mut Map,
    name: &str,
) -> Result<Option<bool>, Box<EvalAltResult>> {
    config
        .remove(name)
        .map(|value| {
            value.try_cast::<bool>().ok_or_else(|| {
                Box::new(component_render_error(format!(
                    "virtual collection `{name}` must be bool"
                )))
            })
        })
        .transpose()
}

fn collection_optional_indices(
    config: &mut Map,
    name: &str,
) -> Result<Option<BTreeSet<usize>>, Box<EvalAltResult>> {
    let Some(value) = config.remove(name) else {
        return Ok(None);
    };
    let values = value.try_cast::<Array>().ok_or_else(|| {
        Box::new(component_render_error(format!(
            "virtual collection `{name}` must be an array of non-negative integers"
        )))
    })?;
    let mut indices = BTreeSet::new();
    for (position, value) in values.into_iter().enumerate() {
        let index = value.try_cast::<rhai::INT>().ok_or_else(|| {
            Box::new(component_render_error(format!(
                "virtual collection `{name}` item {position} must be an integer"
            )))
        })?;
        let index = usize::try_from(index).map_err(|_| {
            Box::new(component_render_error(format!(
                "virtual collection `{name}` item {position} must be non-negative"
            )))
        })?;
        if !indices.insert(index) {
            return Err(Box::new(component_render_error(format!(
                "virtual collection `{name}` contains duplicate index {index}"
            ))));
        }
    }
    Ok(Some(indices))
}

fn collection_data(config: &mut Map) -> Result<crate::VirtualCollectionData, Box<EvalAltResult>> {
    let data = config.remove("data").ok_or_else(|| {
        Box::new(component_render_error(
            "virtual collection `data` is required",
        ))
    })?;
    if data.is::<crate::NativeCollection>() {
        return Ok(crate::VirtualCollectionData::Native(
            data.cast::<crate::NativeCollection>(),
        ));
    }
    let data = data.try_cast::<Array>().ok_or_else(|| {
        Box::new(component_render_error(
            "virtual collection `data` must be an array or NativeCollection",
        ))
    })?;
    let data = data
        .into_iter()
        .map(|value| {
            UiValue::from_dynamic(value)
                .map_err(|error| Box::new(component_render_error(error.to_string())))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut keys = BTreeSet::new();
    for (index, item) in data.iter().enumerate() {
        let (key, _) = collection_payload(item, index)?;
        if !keys.insert(key.clone()) {
            return Err(Box::new(component_render_error(format!(
                "virtual collection data key `{key}` is duplicated"
            ))));
        }
    }
    Ok(crate::VirtualCollectionData::Values(data))
}

fn collection_payload(item: &UiValue, index: usize) -> Result<(String, Map), Box<EvalAltResult>> {
    let UiValue::Map(map) = item else {
        return Err(Box::new(component_render_error(
            "virtual collection data items must be maps with a string `key`",
        )));
    };
    let key = match map.get("key") {
        Some(UiValue::String(key)) if !key.is_empty() => key.clone(),
        _ => {
            return Err(Box::new(component_render_error(
                "virtual collection data item `key` must be a non-empty string",
            )));
        }
    };
    Ok((
        key.clone(),
        Map::from_iter([
            ("key".into(), Dynamic::from(key)),
            (
                "index".into(),
                Dynamic::from_int(rhai::INT::try_from(index).unwrap_or(rhai::INT::MAX)),
            ),
            ("item".into(), item.clone().into_dynamic()),
        ]),
    ))
}

fn nonnegative_usize(value: f64) -> usize {
    value.to_string().parse().unwrap_or(usize::MAX)
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

fn bind_component_callback_props(
    props: &mut Map,
    schema: &BTreeMap<String, crate::ObjectField>,
    binding: &ComponentCallbackBinding,
) {
    for (name, field) in schema {
        if schema_contains_callback(&field.schema)
            && let Some(value) = props.get_mut(name.as_str())
        {
            bind_component_callback_value(&field.schema, value, binding);
        }
    }
}

fn schema_contains_callback(schema: &crate::ValueSchema) -> bool {
    match schema {
        crate::ValueSchema::Callback => true,
        crate::ValueSchema::Array { items, .. }
        | crate::ValueSchema::Map { values: items }
        | crate::ValueSchema::Optional { value: items } => schema_contains_callback(items),
        crate::ValueSchema::Object { fields, .. } => fields
            .values()
            .any(|field| schema_contains_callback(&field.schema)),
        crate::ValueSchema::OneOf { variants } => variants.iter().any(schema_contains_callback),
        _ => false,
    }
}

fn bind_component_callback_value(
    schema: &crate::ValueSchema,
    value: &mut Dynamic,
    binding: &ComponentCallbackBinding,
) {
    match schema {
        crate::ValueSchema::Callback if value.is::<FnPtr>() => {
            let mut function = value.clone_cast::<FnPtr>();
            if !function
                .iter_curry()
                .any(Dynamic::is::<ComponentCallbackBinding>)
            {
                function.add_curry(Dynamic::from(binding.clone()));
                *value = Dynamic::from(function);
            }
        }
        crate::ValueSchema::Optional { value: inner } if !value.is_unit() => {
            bind_component_callback_value(inner, value, binding);
        }
        crate::ValueSchema::OneOf { variants } => {
            if let Some(variant) = variants
                .iter()
                .find(|variant| variant.validate(value).is_ok())
            {
                bind_component_callback_value(variant, value, binding);
            }
        }
        crate::ValueSchema::Array { items, .. }
            if schema_contains_callback(items) && value.is::<Array>() =>
        {
            let mut values = value.clone_cast::<Array>();
            for value in &mut values {
                bind_component_callback_value(items, value, binding);
            }
            *value = Dynamic::from_array(values);
        }
        crate::ValueSchema::Map {
            values: item_schema,
        } if schema_contains_callback(item_schema) && value.is::<Map>() => {
            let mut values = value.clone_cast::<Map>();
            for value in values.values_mut() {
                bind_component_callback_value(item_schema, value, binding);
            }
            *value = Dynamic::from_map(values);
        }
        crate::ValueSchema::Object { fields, .. } if value.is::<Map>() => {
            let mut values = value.clone_cast::<Map>();
            for (name, field) in fields {
                if schema_contains_callback(&field.schema)
                    && let Some(value) = values.get_mut(name.as_str())
                {
                    bind_component_callback_value(&field.schema, value, binding);
                }
            }
            *value = Dynamic::from_map(values);
        }
        _ => {}
    }
}

fn bind_component_node_props(
    props: &mut Map,
    schema: &BTreeMap<String, crate::ObjectField>,
    binding: &ComponentCallbackBinding,
) {
    for (name, field) in schema {
        if schema_contains_node(&field.schema)
            && let Some(value) = props.get_mut(name.as_str())
        {
            bind_component_node_value(&field.schema, value, binding);
        }
    }
}

fn schema_contains_node(schema: &crate::ValueSchema) -> bool {
    match schema {
        crate::ValueSchema::Node => true,
        crate::ValueSchema::Array { items, .. }
        | crate::ValueSchema::Map { values: items }
        | crate::ValueSchema::Optional { value: items } => schema_contains_node(items),
        crate::ValueSchema::Object { fields, .. } => fields
            .values()
            .any(|field| schema_contains_node(&field.schema)),
        crate::ValueSchema::OneOf { variants } => variants.iter().any(schema_contains_node),
        _ => false,
    }
}

fn bind_component_node_value(
    schema: &crate::ValueSchema,
    value: &mut Dynamic,
    binding: &ComponentCallbackBinding,
) {
    match schema {
        crate::ValueSchema::Node if value.is::<UiNode>() => {
            let mut node = value.clone_cast::<UiNode>();
            node.bind_component_scope(
                &binding.component,
                binding.incarnation,
                &binding.events,
                binding.context.as_ref(),
            );
            node.activate_component_snapshots();
            *value = Dynamic::from(node);
        }
        crate::ValueSchema::Optional { value: inner } if !value.is_unit() => {
            bind_component_node_value(inner, value, binding);
        }
        crate::ValueSchema::OneOf { variants } => {
            if let Some(variant) = variants
                .iter()
                .find(|variant| variant.validate(value).is_ok())
            {
                bind_component_node_value(variant, value, binding);
            }
        }
        crate::ValueSchema::Array { items, .. }
            if schema_contains_node(items) && value.is::<Array>() =>
        {
            let mut values = value.clone_cast::<Array>();
            for value in &mut values {
                bind_component_node_value(items, value, binding);
            }
            *value = Dynamic::from_array(values);
        }
        crate::ValueSchema::Map {
            values: item_schema,
        } if schema_contains_node(item_schema) && value.is::<Map>() => {
            let mut values = value.clone_cast::<Map>();
            for value in values.values_mut() {
                bind_component_node_value(item_schema, value, binding);
            }
            *value = Dynamic::from_map(values);
        }
        crate::ValueSchema::Object { fields, .. } if value.is::<Map>() => {
            let mut values = value.clone_cast::<Map>();
            for (name, field) in fields {
                if schema_contains_node(&field.schema)
                    && let Some(value) = values.get_mut(name.as_str())
                {
                    bind_component_node_value(&field.schema, value, binding);
                }
            }
            *value = Dynamic::from_map(values);
        }
        _ => {}
    }
}

fn hydrate_replayed_node_value(value: &mut Dynamic) {
    if value.is::<UiNode>() {
        let mut node = value.clone_cast::<UiNode>();
        node.hydrate_component_subtrees();
        *value = Dynamic::from(node);
    } else if value.is::<Array>() {
        let mut values = value.clone_cast::<Array>();
        for value in &mut values {
            hydrate_replayed_node_value(value);
        }
        *value = Dynamic::from_array(values);
    } else if value.is::<Map>() {
        let mut values = value.clone_cast::<Map>();
        for value in values.values_mut() {
            hydrate_replayed_node_value(value);
        }
        *value = Dynamic::from_map(values);
    }
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

fn component_root_style(props: &Map) -> Option<crate::Style> {
    props
        .get("style")
        .filter(|value| value.is::<crate::Style>())
        .map(Dynamic::clone_cast::<crate::Style>)
}

fn register_component_event_callbacks(
    active: &ActiveComponentRenderState,
    component: &ComponentInstancePath,
    caller: &UiContext,
    callbacks: Vec<(String, FnPtr)>,
    native_context: &crate::invocation::ScriptInvocationContext,
) -> Result<(), Box<EvalAltResult>> {
    let mut guard = active.try_borrow_mut().map_err(|_| {
        Box::new(component_render_error(
            "component render stack is already borrowed",
        ))
    })?;
    let active = guard.as_mut().ok_or_else(|| {
        Box::new(component_render_error(
            "formal component rendering may run only inside view",
        ))
    })?;
    for (event, function) in callbacks {
        let mut callback = ScriptCallback::try_from_fn_ptr(function, active.generation)
            .map_err(|error| Box::new(component_render_error(error.to_string())))?;
        callback.bind_component_scope_if_unset(
            caller.component_path(),
            caller.component_incarnation(),
            caller.event_schemas().clone(),
        );
        callback.bind_native_context_if_unset(native_context.clone());
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

    fn virtual_config(sticky: &[i64], alignment: &str) -> Map {
        let mut item = Map::new();
        item.insert("key".into(), Dynamic::from("header"));
        let mut config = Map::new();
        config.insert("key".into(), Dynamic::from("sections"));
        config.insert(
            "data".into(),
            Dynamic::from_array(vec![Dynamic::from_map(item)]),
        );
        config.insert("estimated_height".into(), Dynamic::from_float(30.0));
        config.insert("height".into(), Dynamic::from_float(120.0));
        config.insert("alignment".into(), Dynamic::from(alignment.to_owned()));
        config.insert(
            "sticky_headers".into(),
            Dynamic::from_array(sticky.iter().copied().map(Dynamic::from_int).collect()),
        );
        config
    }

    #[test]
    fn sticky_virtual_collection_indices_are_bounded_unique_and_top_aligned() {
        let decoded = decode_virtual_collection(virtual_config(&[0], "top")).unwrap();
        assert_eq!(decoded.sticky_headers.as_ref(), &BTreeSet::from([0]));

        for (sticky, alignment, expected) in [
            (&[1][..], "top", "outside 1 items"),
            (&[0, 0][..], "top", "duplicate index 0"),
            (&[0][..], "bottom", "require top-aligned"),
        ] {
            let error = decode_virtual_collection(virtual_config(sticky, alignment)).unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn virtual_collection_reveal_key_is_known_and_has_one_scroll_owner() {
        let mut config = virtual_config(&[], "top");
        config.insert("reveal_key".into(), Dynamic::from("header"));
        let decoded = decode_virtual_collection(config).unwrap();
        assert_eq!(decoded.reveal_key.as_deref(), Some("header"));

        let mut unknown = virtual_config(&[], "top");
        unknown.insert("reveal_key".into(), Dynamic::from("missing"));
        let error = decode_virtual_collection(unknown).unwrap_err();
        assert!(error.to_string().contains("is not present"), "{error}");

        let mut competing = virtual_config(&[], "top");
        competing.insert("reveal_key".into(), Dynamic::from("header"));
        competing.insert("follow_tail".into(), Dynamic::from(true));
        let error = decode_virtual_collection(competing).unwrap_err();
        assert!(error.to_string().contains("must not combine"), "{error}");
    }

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
        let crate::UiNodeKind::Box { children } = root.kind() else {
            panic!("column must render a box");
        };
        assert_eq!(children.len(), 2);
        assert_eq!(
            children[0].key().map(crate::NodeKey::as_str),
            Some("greeting")
        );
        assert!(children.iter().all(|child| child.source().is_some()));
    }

    #[test]
    fn runtime_emits_official_rhai_language_server_definitions() {
        let runtime = RuntimeEngine::new();
        let definitions = runtime.definition_source();
        assert!(definitions.starts_with("module static;"));
        assert!(definitions.contains("fn box"));
        assert!(definitions.contains("fn timeout"));
        assert!(definitions.contains("fn canvas_fill_path"));
        assert!(definitions.contains("UiNode"));
    }

    #[test]
    fn retained_callbacks_reject_capturing_functions_and_non_data_curry() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(r#"fn view() { text("x").on_click(|| ()) }"#)
            .unwrap();
        let error = runtime.render(&compiled).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("anonymous or capturing functions")
        );

        let mut callback = FnPtr::new("clicked").unwrap();
        callback.add_curry(Dynamic::from(UiNode::text("not durable data")));
        assert!(matches!(
            ScriptCallback::try_from_fn_ptr(callback, ScriptGeneration::initial()),
            Err(ScriptCallbackDefinitionError::InvalidCurry { index: 0, .. })
        ));
    }

    #[test]
    fn replay_hydrates_formal_subtrees_inside_nested_dynamic_node_shapes() {
        let root = ComponentInstancePath::root("View", "main");
        let outer = root.child("Outer", "outer");
        let inner = root.child("Inner", "inner");
        let inner_snapshot = crate::node::ComponentOwnedSnapshot::default();
        let mut current_inner = UiNode::text("current")
            .with_component_root_snapshot(inner.clone(), inner_snapshot.reference());
        current_inner.activate_component_snapshots();
        let outer_snapshot = crate::node::ComponentOwnedSnapshot::default();
        let mut current_outer = UiNode::box_node(vec![
            UiNode::text("outer"),
            UiNode::text("stale-inner")
                .with_component_root_snapshot(inner, inner_snapshot.reference()),
        ])
        .with_component_root_snapshot(outer.clone(), outer_snapshot.reference());
        current_outer.activate_component_snapshots();
        let stale = UiNode::box_node(vec![
            UiNode::text("stale-outer")
                .with_component_root_snapshot(outer, outer_snapshot.reference())
                .with_attribute("presentation", UiValue::Bool(true)),
        ]);
        let mut nested = Map::new();
        nested.insert(
            "object".into(),
            Dynamic::from_map(Map::from_iter([(
                "nodes".into(),
                Dynamic::from_array(vec![Dynamic::from(stale)]),
            )])),
        );
        let mut value = Dynamic::from_map(nested);

        hydrate_replayed_node_value(&mut value);

        let nested = value.cast::<Map>();
        let object = nested["object"].clone_cast::<Map>();
        let nodes = object["nodes"].clone_cast::<Array>();
        let node = nodes[0].clone_cast::<UiNode>();
        let crate::UiNodeKind::Box { children } = node.kind() else {
            panic!("nested node shape must preserve its raw wrapper");
        };
        assert_eq!(
            children[0].attributes().get("presentation"),
            Some(&UiValue::Bool(true))
        );
        let crate::UiNodeKind::Box { children } = children[0].kind() else {
            panic!("outer component must hydrate its owned snapshot");
        };
        assert!(
            matches!(children[1].kind(), crate::UiNodeKind::Text { text } if text == "current")
        );
    }

    #[test]
    fn final_component_api_registers_render_once_and_invokes_by_id() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    define_component(#{
                        metadata: #{
                            id: "components/message", "export": "Message", version: "0.1.0",
                            runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
                            dependencies: [], capabilities: #{}
                        },
                        schema: #{ props: #{}, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: [] },
                        render: Fn("render_Message")
                    });
                    fn Message(props) { render_component("components/message", props) }
                    fn render_Message(ctx, props) { text("final component api") }
                    fn view() { Message(#{ key: "message" }) }
                "#,
            )
            .unwrap();
        let root = runtime.render(&compiled).unwrap();
        assert!(matches!(
            root.kind(),
            crate::UiNodeKind::Text { text } if text == "final component api"
        ));
        let recipe = runtime.component_invocations().next().unwrap();
        assert_eq!(recipe.render_name(), "render_Message");
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
            root.handler("open_change")
                .unwrap()
                .as_script()
                .unwrap()
                .generation(),
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
    fn rhai_nodes_retain_ordered_handlers_for_all_event_phases() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn first(ctx, event) { event_response().prevent_default() }
                    fn second(ctx, event) { event_response().stop_immediate() }
                    fn capture(ctx, event) { propagate() }
                    fn view() {
                        text("drag")
                            .on_capture("pointer_down", Fn("capture"))
                            .on("pointer_down", Fn("first"))
                            .on("pointer_down", Fn("second"))
                    }
                "#,
            )
            .unwrap();
        let root = runtime.render(&compiled).unwrap();
        let bindings = root.event_handlers("pointer_down");
        assert_eq!(bindings.len(), 3);
        assert_eq!(bindings[0].phase(), crate::EventPhase::Capture);
        assert_eq!(bindings[1].phase(), crate::EventPhase::Target);
        assert_eq!(bindings[2].phase(), crate::EventPhase::Target);
        assert_eq!(bindings[2].handler().as_script().unwrap().name(), "second");
    }

    #[test]
    fn rhai_nodes_attach_only_registered_schema_checked_native_handlers() {
        let mut runtime = RuntimeEngine::new();
        let id = crate::NativeHandlerId::parse("timeline.drag").unwrap();
        runtime
            .register_native_handler(
                crate::NativeHandlerDescriptor::new(
                    id,
                    BTreeMap::from([("pointer_down".to_owned(), crate::ValueSchema::UiValue)]),
                )
                .unwrap(),
                |_, _, _, _| Ok(crate::EventResponse::new().capture_pointer()),
            )
            .unwrap();
        let compiled = runtime
            .compile(
                r#"
                    fn view() {
                        text("drag").on(
                            "pointer_down", native_handler("timeline.drag")
                        )
                    }
                "#,
            )
            .unwrap();
        let root = runtime.render(&compiled).unwrap();
        assert!(matches!(
            root.handler("pointer_down"),
            Some(crate::UiEventHandler::Native(reference))
                if reference.descriptor().id.as_str() == "timeline.drag"
        ));

        let input = runtime
            .compile(
                r#"
                    fn view() {
                        gpui_rhai::TextInputPrimitive(#{
                            key: "native-input", value: "", placeholder: "Edit",
                            disabled: false, read_only: false, typography: "body",
                            on_change: native_handler("timeline.drag")
                        })
                    }
                "#,
            )
            .unwrap();
        assert!(
            runtime.render(&input).is_ok(),
            "native handlers must cross formal component and primitive callback props"
        );

        let missing = runtime
            .compile(
                "fn view() { text(\"x\").on(\"pointer_down\", native_handler(\"app.missing\")) }",
            )
            .unwrap();
        assert!(runtime.render(&missing).is_err());
    }

    #[test]
    fn final_box_and_fragment_atoms_are_distinct_snapshots() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn view() {
                        box([text("a"), fragment([text("b"), text("c")])])
                    }
                "#,
            )
            .unwrap();
        let root = runtime.render(&compiled).unwrap();
        let crate::UiNodeKind::Box { children } = root.kind() else {
            panic!("root must be the final Box atom");
        };
        assert!(matches!(
            children[1].kind(),
            crate::UiNodeKind::Fragment { children } if children.len() == 2
        ));
    }

    #[test]
    fn text_accepts_typed_inline_span_runs() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn view() {
                        text([
                            span("Hello ").bold(),
                            span("world").color(rgb(0x22cc88)).italic()
                        ])
                    }
                "#,
            )
            .unwrap();
        let root = runtime.render(&compiled).unwrap();
        let crate::UiNodeKind::RichText { text, spans } = root.kind() else {
            panic!("typed spans must produce rich text");
        };
        assert_eq!(text.as_str(), "Hello world");
        assert_eq!(spans.len(), 2);
        assert!(spans[0].is_bold());
        assert!(spans[1].is_italic());
    }

    #[test]
    fn canvas_scene_is_keyed_validated_and_script_constructible() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn view() {
                        canvas(canvas_scene([
                            canvas_rect("panel", 0.0, 0.0, 100.0, 40.0, rgb(0x112233)),
                            canvas_circle("dot", 20.0, 20.0, 8.0, rgb(0xffcc00)),
                            canvas_line("axis", 0.0, 39.0, 100.0, 39.0, 1.0, rgb(0xffffff))
                        ]))
                    }
                "#,
            )
            .unwrap();
        let root = runtime.render(&compiled).unwrap();
        assert!(matches!(
            root.kind(),
            crate::UiNodeKind::Canvas { scene } if scene.commands().len() == 3
        ));
    }

    #[test]
    fn inline_svg_atom_is_bounded_and_self_contained() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn view() {
                        svg("<svg viewBox='0 0 10 10'><path fill='currentColor' d='M0 0L10 10'/></svg>")
                    }
                "#,
            )
            .unwrap();
        let root = runtime.render(&compiled).unwrap();
        assert!(matches!(
            root.kind(),
            crate::UiNodeKind::Svg { source } if source.as_str().contains("currentColor")
        ));

        let rejected = runtime
            .compile(r#"fn view() { svg("<svg><image href='https://example.com/a.png'/></svg>") }"#)
            .unwrap();
        assert!(runtime.render(&rejected).is_err());
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
        let compiled = runtime
            .compile(
                r#"
                    fn view() { text("timed") }
                    fn counted() { let total = 0; for value in 0..100 { total += value; } total }
                "#,
            )
            .unwrap();
        runtime.render(&compiled).unwrap();
        let callback = runtime.callback(&compiled, "counted").unwrap();
        let _ = runtime.invoke_callback(&compiled, &callback, ()).unwrap();
        let timings = runtime.take_timings();
        assert_eq!(timings.len(), 3);
        assert_eq!(timings[0].operation, ExecutionOperation::Compile);
        assert_eq!(timings[1].operation, ExecutionOperation::Render);
        assert_eq!(
            timings[2].operation,
            ExecutionOperation::Callback("counted".to_owned())
        );
        assert_eq!(timings[0].operations, 0);
        assert!(timings[1].operations > 0);
        assert!(timings[2].operations > timings[1].operations);
        assert!(timings.iter().all(|timing| timing.slow && timing.succeeded));
    }

    #[test]
    fn independent_candidates_never_share_callback_authority() {
        let mut runtime = RuntimeEngine::new();
        let first = runtime
            .compile("fn view() { text(\"first\") } fn action() { 1 }")
            .unwrap();
        let callback = runtime.callback(&first, "action").unwrap();
        let second = runtime
            .compile("fn view() { text(\"second\") } fn action() { 2 }")
            .unwrap();
        assert_ne!(first.generation(), second.generation());
        runtime.render(&second).unwrap();
        assert!(matches!(
            runtime.invoke_callback(&second, &callback, ()),
            Err(RuntimeError::StaleCallback { .. })
        ));
    }

    #[test]
    fn callbacks_cannot_cross_runtime_engines() {
        let mut first_engine = RuntimeEngine::new();
        let first = first_engine
            .compile("fn view() { text(\"first\") } fn action() { 1 }")
            .unwrap();
        first_engine.render(&first).unwrap();
        let callback = first_engine.callback(&first, "action").unwrap();

        let mut second_engine = RuntimeEngine::new();
        let second = second_engine
            .compile("fn view() { text(\"second\") } fn action() { 2 }")
            .unwrap();
        second_engine.render(&second).unwrap();
        assert!(matches!(
            second_engine.invoke_callback(&second, &callback, ()),
            Err(RuntimeError::StaleCallback { .. })
        ));
    }

    #[test]
    fn one_program_preparation_shares_identity_across_module_and_entry_compilation() {
        let mut runtime = RuntimeEngine::new();
        let (module, entry) = runtime
            .with_program_preparation(|runtime| {
                Ok::<_, RuntimeError>((
                    runtime.compile_named("module", "fn helper() { 1 }")?,
                    runtime.compile_named("entry", "fn view() { text(\"entry\") }")?,
                ))
            })
            .unwrap();
        assert_eq!(module.generation(), entry.generation());
        let next = runtime.compile("fn view() { text(\"next\") }").unwrap();
        assert_ne!(entry.generation(), next.generation());
    }

    #[test]
    fn unsafe_const_container_assignment_is_rejected_before_evaluation() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            RuntimeEngine::new()
                .compile("fn view() { const m = #{ x: 1 }; m.x = 2; text(\"done\") }")
        }));
        assert!(matches!(
            result,
            Ok(Err(RuntimeError::InvalidAssignmentTarget(_)))
        ));
    }

    #[test]
    fn default_runtime_rejects_filesystem_imports() {
        let directory = tempfile::tempdir().unwrap();
        let module = directory.path().join("external.rhai");
        std::fs::write(&module, "fn value() { \"external\" }").unwrap();
        let mut runtime = RuntimeEngine::new();
        let source = format!(
            "import {:?} as external; fn view() {{ text(external::value()) }}",
            module.with_extension("").to_string_lossy()
        );
        let compiled = runtime.compile(&source).unwrap();
        assert!(runtime.render(&compiled).is_err());
    }

    #[test]
    fn nested_component_work_consumes_one_shared_operation_budget() {
        let module = r#"
            define_component(#{
                metadata: #{ id: "components/heavy", "export": "Heavy", version: "0.1.1",
                    runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
                schema: #{ props: #{}, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"] },
                render: Fn("render_Heavy")
            });
            fn Heavy(props) { render_component("components/heavy", props) }
            fn render_Heavy(ctx, props) {
                let n = 0;
                for i in 0..200000 { n += 1; }
                text(`${n}`)
            }
        "#;
        let mut resolver = crate::RestrictedModuleResolver::new();
        resolver.insert("components/heavy", module).unwrap();
        let mut runtime = RuntimeEngine::new();
        runtime.set_module_resolver(resolver);
        let compiled = runtime
            .compile_self_contained_named(
                "budget",
                r#"import "components/heavy" as h;
                    fn view() { column([h::Heavy(#{}), h::Heavy(#{}), h::Heavy(#{}), h::Heavy(#{} )]) }"#,
            )
            .unwrap();
        let result = runtime.render(&compiled);
        assert!(result.is_err(), "nested work must exceed the shared budget");
        assert!(
            runtime
                .take_timings()
                .iter()
                .any(|timing| timing.operations > MAX_SCRIPT_OPERATIONS)
        );
    }

    #[test]
    fn retained_component_budget_ignores_parent_history_without_resetting_session_total() {
        let module = r#"
            define_component(#{
                metadata: #{ id: "components/probe", "export": "Probe", version: "0.1.1",
                    runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
                schema: #{ props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
                    state: #{ fields: #{ heavy: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } } } },
                    events: #{}, slots: #{}, parts: ["root"] },
                render: Fn("render_Probe")
            });
            fn Probe(props) { render_component("components/probe", props) }
            fn render_Probe(ctx, props) {
                let n = 0;
                if ctx.get_state("heavy") { for i in 0..100000 { n += 1; } }
                text(`${n}`)
            }
        "#;
        let mut measured = Vec::new();
        for parent_work in [0, 250_000] {
            let mut resolver = crate::RestrictedModuleResolver::new();
            resolver.insert("components/probe", module).unwrap();
            let mut engine = RuntimeEngine::new();
            engine.set_module_resolver(resolver);
            let source = format!(
                r#"import "components/probe" as probe;
                    fn view(ctx) {{
                        let n = 0;
                        for i in 0..{parent_work} {{ n += 1; }}
                        probe::Probe(#{{ key: "probe" }})
                    }}"#
            );
            let compiled = engine
                .compile_self_contained_named("incremental-budget", &source)
                .unwrap();
            let runtime = Rc::new(RefCell::new(crate::UiRuntimeState::new()));
            let mut lifecycle = crate::ScriptLifecycle::new(
                compiled,
                Rc::clone(&runtime),
                ComponentInstancePath::root("App", "budget"),
                None,
                BTreeMap::new(),
                &ComponentStateSchema::default(),
            )
            .unwrap();
            lifecycle.start(&mut engine).unwrap();
            let owner = engine
                .component_invocations()
                .next()
                .unwrap()
                .path()
                .clone();
            let _ = engine.take_timings();
            runtime
                .borrow_mut()
                .set_component_state_from_host(&owner, "heavy", UiValue::Bool(true))
                .unwrap();
            assert!(lifecycle.render_dirty(&mut engine).unwrap());
            let operations = engine
                .take_timings()
                .into_iter()
                .filter(|timing| timing.operation == ExecutionOperation::Render)
                .map(|timing| timing.operations)
                .sum::<u64>();
            assert!(operations < MAX_SCRIPT_OPERATIONS / 2, "{operations}");
            measured.push(operations);
        }
        assert_eq!(measured[0], measured[1]);
    }

    #[test]
    fn multiple_dirty_components_share_one_incremental_budget() {
        let module = r#"
            define_component(#{
                metadata: #{ id: "components/probe", "export": "Probe", version: "0.1.1",
                    runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
                schema: #{ props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
                    state: #{ fields: #{ heavy: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } } } },
                    events: #{}, slots: #{}, parts: ["root"] }, render: Fn("render_Probe") }
            );
            fn Probe(props) { render_component("components/probe", props) }
            fn render_Probe(ctx, props) {
                let n = 0;
                if ctx.get_state("heavy") { for i in 0..200000 { n += 1; } }
                text(`${n}`)
            }
        "#;
        let mut resolver = crate::RestrictedModuleResolver::new();
        resolver.insert("components/probe", module).unwrap();
        let mut engine = RuntimeEngine::new();
        engine.set_module_resolver(resolver);
        let compiled = engine
            .compile_self_contained_named(
                "incremental-aggregate-budget",
                r#"import "components/probe" as probe;
                    fn view(ctx) {
                        column([
                            probe::Probe(#{ key: "first" }),
                            probe::Probe(#{ key: "second" }),
                        ])
                    }"#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(crate::UiRuntimeState::new()));
        let mut lifecycle = crate::ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            ComponentInstancePath::root("App", "budget"),
            None,
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        let owners = engine
            .component_invocations()
            .map(|recipe| recipe.path().clone())
            .collect::<Vec<_>>();
        assert_eq!(owners.len(), 2);
        for owner in owners {
            runtime
                .borrow_mut()
                .set_component_state_from_host(&owner, "heavy", UiValue::Bool(true))
                .unwrap();
        }
        assert!(lifecycle.render_dirty(&mut engine).is_err());
    }

    #[test]
    fn retained_virtual_renderer_budget_ignores_parent_history() {
        let mut measured = Vec::new();
        for parent_work in [0, 250_000] {
            let mut engine = RuntimeEngine::new();
            let source = format!(
                r#"
                    fn render_item(ctx, payload) {{
                        let n = 0;
                        if payload.item.heavy {{ for i in 0..100000 {{ n += 1; }} }}
                        text(`${{n}}`).with_key(payload.key)
                    }}
                    fn view(ctx) {{
                        let n = 0;
                        for i in 0..{parent_work} {{ n += 1; }}
                        let data = [];
                        for index in 0..32 {{
                            data.push(#{{ key: `row-${{index}}`, heavy: index == 31 }});
                        }}
                        virtual_collection(#{{
                            key: "rows", label: "Rows", data: data,
                            estimated_height: 24, height: 24, overdraw_pixels: 0,
                            alignment: "top", follow_tail: false,
                        }}, Fn("render_item"))
                    }}
                "#
            );
            let compiled = engine.compile_named("virtual-budget", &source).unwrap();
            let runtime = Rc::new(RefCell::new(crate::UiRuntimeState::new()));
            let root_path = ComponentInstancePath::root("App", "budget");
            let mut lifecycle = crate::ScriptLifecycle::new(
                compiled,
                Rc::clone(&runtime),
                root_path.clone(),
                None,
                BTreeMap::new(),
                &ComponentStateSchema::default(),
            )
            .unwrap();
            lifecycle.start(&mut engine).unwrap();
            let id = engine
                .virtual_collection_ids_in_scope(&root_path)
                .into_iter()
                .next()
                .unwrap();
            let _ = engine.take_timings();
            runtime.borrow().virtual_requests.request(id, [31]);
            assert!(lifecycle.realize_virtual_requests(&mut engine).unwrap());
            let operations = engine
                .take_timings()
                .into_iter()
                .filter(|timing| {
                    matches!(timing.operation, ExecutionOperation::VirtualCollection(_))
                })
                .map(|timing| timing.operations)
                .sum::<u64>();
            assert!(operations < MAX_SCRIPT_OPERATIONS / 2, "{operations}");
            measured.push(operations);
        }
        assert_eq!(measured[0], measured[1]);
    }

    #[test]
    fn delayed_component_callback_starts_with_a_fresh_operation_budget() {
        let module = r#"
            define_component(#{
                metadata: #{ id: "components/counter", "export": "Counter", version: "0.1.1",
                    runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
                schema: #{ props: #{}, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"] },
                render: Fn("render_Counter")
            });
            fn Counter(props) { render_component("components/counter", props) }
            fn render_Counter(ctx, props) { text("counter").on_click(Fn("clicked")) }
            fn clicked(ctx, payload) {
                let n = 0;
                for i in 0..100000 { n += 1; }
                n
            }
        "#;
        let mut resolver = crate::RestrictedModuleResolver::new();
        resolver.insert("components/counter", module).unwrap();
        let mut engine = RuntimeEngine::new();
        engine.set_module_resolver(resolver);
        let compiled = engine
            .compile_self_contained_named(
                "callback-budget",
                r#"import "components/counter" as c;
                    fn view(ctx) {
                        let n = 0;
                        for i in 0..250000 { n += 1; }
                        c::Counter(#{} )
                    }"#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let mut lifecycle = crate::ScriptLifecycle::new(
            compiled,
            runtime,
            ComponentInstancePath::root("App", "budget"),
            None,
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap();
        let root = lifecycle.start(&mut engine).unwrap();
        let crate::UiEventHandler::Script(callback) = root.handlers()["click"][0].handler() else {
            panic!("component must expose a script click callback");
        };
        let callback = callback.clone();
        assert!(
            lifecycle
                .invoke_callback_transactional(&engine, &callback, UiValue::Null)
                .is_ok()
        );
    }

    #[test]
    fn old_component_handles_cannot_target_a_same_key_remount() {
        let module = r#"
            define_component(#{
                metadata: #{ id: "components/incarnation", "export": "Counter", version: "0.1.1",
                    runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
                schema: #{ props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
                    state: #{ fields: #{ count: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } } } },
                    events: #{}, slots: #{}, parts: ["root"] }, render: Fn("render_Counter")
            });
            fn Counter(props) { render_component("components/incarnation", props) }
            fn increment(ctx, payload) { ctx.set_state("count", ctx.get_state("count") + 1); }
            fn render_Counter(ctx, props) {
                let progress = signal("progress", 0.0);
                text(`${ctx.get_state("count")}`).bind_signal("opacity", progress).on_click(Fn("increment"))
            }
        "#;
        let mut resolver = crate::RestrictedModuleResolver::new();
        resolver.insert("components/incarnation", module).unwrap();
        let mut engine = RuntimeEngine::new();
        engine.set_module_resolver(resolver);
        let compiled = engine
            .compile_self_contained_named(
                "incarnation",
                r#"import "components/incarnation" as c;
                    fn view(ctx) {
                        if ctx.get_state("visible") { c::Counter(#{ key: "same" }) }
                        else { text("hidden") }
                    }"#,
            )
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let root_path = ComponentInstancePath::root("App", "incarnation");
        let schema = ComponentStateSchema::new(BTreeMap::from([(
            "visible".to_owned(),
            crate::StateField::new(crate::ValueSchema::Bool, UiValue::Bool(true)),
        )]))
        .unwrap();
        let mut lifecycle = crate::ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            root_path.clone(),
            None,
            BTreeMap::new(),
            &schema,
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        let component = engine
            .component_invocations()
            .next()
            .unwrap()
            .path()
            .clone();
        let crate::UiEventHandler::Script(old_callback) =
            lifecycle.root().unwrap().handlers()["click"][0].handler()
        else {
            panic!("component must expose its callback");
        };
        let old_callback = old_callback.clone();
        let old_signal = runtime
            .borrow()
            .signals
            .resolve(&component, "progress")
            .unwrap();
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root_path, "visible", UiValue::Bool(false))
            .unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root_path, "visible", UiValue::Bool(true))
            .unwrap();
        lifecycle.render_dirty(&mut engine).unwrap();
        assert!(matches!(
            lifecycle.invoke_callback_transactional(&engine, &old_callback, UiValue::Null),
            Err(crate::LifecycleError::Runtime(
                RuntimeError::StaleComponentCallback { .. }
            ))
        ));
        assert!(matches!(
            runtime
                .borrow_mut()
                .signals
                .write(&old_signal, crate::SignalValue::Float(0.5)),
            Err(crate::SignalError::Stale(_))
        ));
    }
}
