use std::cell::RefCell;
use std::collections::BTreeMap;
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
        let context = self.context(ExecutionPhase::Render);
        let root = engine.render_with_context(&self.compiled, context)?;
        self.reconcile_animations(&root)?;
        self.state = LifecycleState::Running;
        Ok(self.root.insert(root))
    }

    /// Run optional `dispose(ctx)` once and make the lifecycle terminal.
    ///
    /// # Errors
    ///
    /// Returns [`LifecycleError::InvalidTransition`] before initialization or
    /// after disposal, or a script evaluation error.
    pub fn dispose(&mut self, engine: &RuntimeEngine) -> Result<(), LifecycleError> {
        if !matches!(
            self.state,
            LifecycleState::Initialized | LifecycleState::Running
        ) {
            return Err(LifecycleError::InvalidTransition {
                from: self.state,
                operation: "dispose",
            });
        }
        let context = self.context(ExecutionPhase::Dispose);
        engine.call_optional_lifecycle(&self.compiled, "dispose", context)?;
        self.state = LifecycleState::Disposed;
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
        {
            let mut runtime = self
                .runtime
                .try_borrow_mut()
                .map_err(|_| LifecycleError::Borrowed)?;
            runtime
                .component_state
                .mount_instance(self.root_path.clone(), state_schema)?;
        }
        let result: Result<UiNode, LifecycleError> = (|| {
            engine.call_optional_lifecycle(
                &candidate,
                "init",
                self.context_for(ExecutionPhase::Init, candidate.generation()),
            )?;
            let root = engine.render_with_context(
                &candidate,
                self.context_for(ExecutionPhase::Render, candidate.generation()),
            )?;
            self.reconcile_animations(&root)?;
            Ok(root)
        })();
        match result {
            Ok(root) => {
                self.compiled = candidate;
                self.state = LifecycleState::Running;
                Ok(self.root.insert(root))
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
        let values = crate::animation::reconcile_node_animations_scoped(
            root,
            &mut runtime.animations,
            std::time::Instant::now(),
            &self.animation_root_path(),
        )?;
        runtime.animation_values = values;
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
    Asset(#[from] crate::AssetError),
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
        lifecycle.dispose(&engine).unwrap();
        assert_eq!(lifecycle.state(), LifecycleState::Disposed);
        assert_eq!(
            runtime.borrow().component_state.get(&path, "phase"),
            Some(&UiValue::String("disposed".to_owned()))
        );
        assert!(matches!(
            lifecycle.dispose(&engine),
            Err(LifecycleError::InvalidTransition { .. })
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
        lifecycle.dispose(&engine).unwrap();
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
