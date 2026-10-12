//! Setup shared by the integration tests that compile and mount Rhai apps: the
//! bundled official modules, engines that resolve imports against them, and the
//! standard root a test app renders and mounts at.

#![allow(
    dead_code,
    reason = "each test crate uses its own subset of these helpers"
)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui_rhai::{
    CompiledUi, ComponentInstancePath, ComponentStateSchema, Environment, ExecutionPhase,
    RestrictedModuleResolver, RuntimeEngine, RuntimeError, ScriptLifecycle, Symbol, UiContext,
    UiNode, UiRuntimeState,
};
use gpui_rhai_registry as registry;

/// Every bundled official module an app imports by ID, from the registry's own
/// release snapshot, so a new component is covered without editing a test.
pub fn official_modules() -> impl Iterator<Item = (&'static str, &'static str)> {
    registry::BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(registry::BUNDLED_LAYOUT_SOURCES_BY_ID)
        .chain(registry::BUNDLED_PATTERN_SOURCES_BY_ID)
        .chain(registry::BUNDLED_CHART_SOURCES_BY_ID)
        .copied()
}

/// The bundled source of the official module `id`, such as `components/table`.
pub fn official_source(id: &str) -> &'static str {
    official_modules()
        .find_map(|(module, source)| (module == id).then_some(source))
        .unwrap_or_else(|| panic!("no bundled official module {id}"))
}

/// An engine whose imports resolve to exactly `modules`, given as `(id, source)`.
pub fn engine_with<'source>(
    modules: impl IntoIterator<Item = (&'source str, &'source str)>,
) -> RuntimeEngine {
    let mut resolver = RestrictedModuleResolver::new();
    for (id, source) in modules {
        resolver
            .insert(id, source)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(resolver);
    engine
}

/// An engine whose imports resolve to every official module. Modules compile
/// only when an app imports them.
pub fn official_engine() -> RuntimeEngine {
    engine_with(official_modules())
}

/// Compiles `app` as the entry script `name`, embedding the modules it imports.
pub fn compile(engine: &mut RuntimeEngine, name: &str, app: &str) -> CompiledUi {
    engine
        .compile_self_contained_named(name, app)
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

/// An official engine and `app` compiled against it.
pub fn compile_official(name: &str, app: &str) -> (RuntimeEngine, CompiledUi) {
    let mut engine = official_engine();
    let compiled = compile(&mut engine, name, app);
    (engine, compiled)
}

/// The path every test app renders and mounts at.
pub fn root_path() -> ComponentInstancePath {
    ComponentInstancePath::root("App", "root")
}

/// Renders `view(ctx)` once at [`root_path`] in window `main` over `state`,
/// without mounting a lifecycle.
///
/// # Errors
///
/// Returns the render error, for tests that expect one.
pub fn try_render_in(
    engine: &mut RuntimeEngine,
    compiled: &CompiledUi,
    state: UiRuntimeState,
) -> Result<UiNode, RuntimeError> {
    let context = UiContext::new(
        Rc::new(RefCell::new(state)),
        root_path(),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    engine.render_with_context(compiled, context)
}

/// Renders `view(ctx)` once over fresh runtime state, as [`try_render_in`].
pub fn render(engine: &mut RuntimeEngine, compiled: &CompiledUi) -> UiNode {
    try_render_in(engine, compiled, UiRuntimeState::new()).unwrap()
}

/// A lifecycle for `compiled` at [`root_path`] in window `main`, holding `schema`
/// as its root state, not yet started.
pub fn lifecycle_with_schema(
    compiled: CompiledUi,
    runtime: &Rc<RefCell<UiRuntimeState>>,
    schema: &ComponentStateSchema,
) -> ScriptLifecycle {
    ScriptLifecycle::new(
        compiled,
        Rc::clone(runtime),
        root_path(),
        Some("main".to_owned()),
        BTreeMap::new(),
        schema,
    )
    .unwrap()
}

/// A lifecycle for `compiled` with the root state its `state_schema()` declares
/// (none without one), not yet started.
pub fn lifecycle_for(
    engine: &RuntimeEngine,
    compiled: CompiledUi,
    runtime: &Rc<RefCell<UiRuntimeState>>,
) -> ScriptLifecycle {
    let schema = engine.root_state_schema(&compiled).unwrap();
    lifecycle_with_schema(compiled, runtime, &schema)
}

/// [`lifecycle_for`], started: the app is mounted and rendered once.
pub fn mount(
    engine: &mut RuntimeEngine,
    compiled: CompiledUi,
    runtime: &Rc<RefCell<UiRuntimeState>>,
) -> ScriptLifecycle {
    let mut lifecycle = lifecycle_for(engine, compiled, runtime);
    lifecycle.start(engine).unwrap();
    lifecycle
}

/// Fresh, shared runtime state for one mounted app.
pub fn new_runtime() -> Rc<RefCell<UiRuntimeState>> {
    Rc::new(RefCell::new(UiRuntimeState::new()))
}

/// An environment holding `pairs` as `(name, value)`, such as `("density", "compact")`.
pub fn environment(pairs: &[(&str, &str)]) -> Environment {
    Environment::EMPTY
        .with_all(
            pairs
                .iter()
                .map(|&(name, value)| (Symbol::intern(name), Symbol::intern(value))),
        )
        .unwrap()
}
