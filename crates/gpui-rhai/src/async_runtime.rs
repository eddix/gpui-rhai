use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::time::{Duration, Instant};

use rhai::{CustomType, TypeBuilder};
use thiserror::Error;

use crate::{
    ComponentInstancePath, SchemaValidationError, ScriptCallback, ScriptGeneration, UiValue,
    ValueSchema,
};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AsyncScope {
    App,
    Window(String),
    Component(ComponentInstancePath),
    Effect {
        component: ComponentInstancePath,
        key: String,
        activation: u64,
    },
}

impl AsyncScope {
    #[must_use]
    pub const fn component(&self) -> Option<&ComponentInstancePath> {
        match self {
            Self::Component(component) | Self::Effect { component, .. } => Some(component),
            Self::App | Self::Window(_) => None,
        }
    }

    #[must_use]
    pub fn is_within_component(&self, root: &ComponentInstancePath) -> bool {
        self.component()
            .is_some_and(|component| component.is_within(root))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TaskHandle(u64);

impl CustomType for TaskHandle {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("TaskHandle")
            .with_fn("to_string", |handle: &mut Self| {
                format!("task#{}", handle.0)
            });
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SubscriptionHandle(u64);

impl CustomType for SubscriptionHandle {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("SubscriptionHandle")
            .with_fn("to_string", |handle: &mut Self| {
                format!("subscription#{}", handle.0)
            });
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum SubscriptionCloseReason {
    #[error("the subscription work returned")]
    WorkReturned,
    #[error("the producer closed its emitter")]
    ProducerClosed,
    #[error("the subscription was cancelled explicitly")]
    Cancelled,
    #[error("the owning scope was disposed")]
    ScopeDisposed,
    #[error("the script generation became stale")]
    GenerationStale,
    #[error("the creating transaction was rolled back")]
    TransactionRolledBack,
    #[error("subscription startup failed")]
    StartupFailed,
    #[error("the subscription registry was dropped")]
    RegistryDropped,
}

impl SubscriptionCloseReason {
    const fn code(self) -> u8 {
        match self {
            Self::WorkReturned => 1,
            Self::ProducerClosed => 2,
            Self::Cancelled => 3,
            Self::ScopeDisposed => 4,
            Self::GenerationStale => 5,
            Self::TransactionRolledBack => 6,
            Self::StartupFailed => 7,
            Self::RegistryDropped => 8,
        }
    }

    const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::WorkReturned),
            2 => Some(Self::ProducerClosed),
            3 => Some(Self::Cancelled),
            4 => Some(Self::ScopeDisposed),
            5 => Some(Self::GenerationStale),
            6 => Some(Self::TransactionRolledBack),
            7 => Some(Self::StartupFailed),
            8 => Some(Self::RegistryDropped),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct SubscriptionLifetime {
    close_reason: AtomicU8,
}

impl SubscriptionLifetime {
    fn new() -> Self {
        Self {
            close_reason: AtomicU8::new(0),
        }
    }

    fn close(&self, reason: SubscriptionCloseReason) -> bool {
        self.close_reason
            .compare_exchange(0, reason.code(), Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn close_reason(&self) -> Option<SubscriptionCloseReason> {
        SubscriptionCloseReason::from_code(self.close_reason.load(Ordering::Acquire))
    }
}

#[derive(Clone, Debug)]
pub struct AsyncDelivery {
    pub callback: ScriptCallback,
    pub payload: UiValue,
    pub scope: AsyncScope,
}

#[derive(Clone)]
struct CallbackPair {
    success: ScriptCallback,
    error: ScriptCallback,
}

struct TaskEntry {
    scope: AsyncScope,
    generation: ScriptGeneration,
    callbacks: CallbackPair,
    output: ValueSchema,
}

struct TaskMessage {
    id: u64,
    result: Result<UiValue, String>,
}

pub struct TaskRegistry {
    next_id: u64,
    entries: BTreeMap<u64, TaskEntry>,
    sender: Sender<TaskMessage>,
    receiver: Receiver<TaskMessage>,
}

impl Default for TaskRegistry {
    fn default() -> Self {
        let (sender, receiver) = channel();
        Self {
            next_id: 1,
            entries: BTreeMap::new(),
            sender,
            receiver,
        }
    }
}

impl fmt::Debug for TaskRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskRegistry")
            .field("active", &self.entries.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl TaskRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn Rust work while retaining Rhai callbacks on the foreground side.
    ///
    /// # Errors
    ///
    /// Returns [`AsyncRuntimeError::Spawn`] when the worker thread cannot start.
    pub fn spawn(
        &mut self,
        scope: AsyncScope,
        generation: ScriptGeneration,
        success: ScriptCallback,
        error: ScriptCallback,
        output: ValueSchema,
        work: impl FnOnce() -> Result<UiValue, String> + Send + 'static,
    ) -> Result<TaskHandle, AsyncRuntimeError> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.entries.insert(
            id,
            TaskEntry {
                scope,
                generation,
                callbacks: CallbackPair { success, error },
                output,
            },
        );
        let sender = self.sender.clone();
        if let Err(spawn_error) = std::thread::Builder::new()
            .name(format!("gpui-rhai-task-{id}"))
            .spawn(move || {
                let _ = sender.send(TaskMessage { id, result: work() });
            })
        {
            self.entries.remove(&id);
            return Err(AsyncRuntimeError::Spawn(spawn_error));
        }
        Ok(TaskHandle(id))
    }

    #[must_use]
    pub fn cancel(&mut self, handle: TaskHandle) -> bool {
        self.entries.remove(&handle.0).is_some()
    }

    pub fn cancel_scope(&mut self, scope: &AsyncScope) {
        self.entries.retain(|_, entry| &entry.scope != scope);
    }

    pub fn cancel_component_scope(&mut self, component: &ComponentInstancePath) {
        self.entries
            .retain(|_, entry| !entry.scope.is_within_component(component));
    }

    #[must_use]
    pub fn drain(&mut self, current: ScriptGeneration) -> Vec<AsyncDelivery> {
        let mut deliveries = Vec::new();
        while let Ok(message) = self.receiver.try_recv() {
            let Some(entry) = self.entries.remove(&message.id) else {
                continue;
            };
            if entry.generation != current {
                continue;
            }
            deliveries.push(task_delivery(entry, message.result));
        }
        deliveries
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn active_ids(&self) -> std::collections::BTreeSet<u64> {
        self.entries.keys().copied().collect()
    }

    pub(crate) fn retain_ids(&mut self, retained: &std::collections::BTreeSet<u64>) {
        self.entries.retain(|id, _| retained.contains(id));
    }
}

fn task_delivery(entry: TaskEntry, result: Result<UiValue, String>) -> AsyncDelivery {
    match result {
        Ok(value) => match entry.output.validate(&value.clone().into_dynamic()) {
            Ok(()) => AsyncDelivery {
                callback: entry.callbacks.success,
                payload: value,
                scope: entry.scope,
            },
            Err(error) => AsyncDelivery {
                callback: entry.callbacks.error,
                payload: error_payload(error.to_string()),
                scope: entry.scope,
            },
        },
        Err(error) => AsyncDelivery {
            callback: entry.callbacks.error,
            payload: error_payload(error),
            scope: entry.scope,
        },
    }
}

enum SubscriptionMessage {
    Value {
        id: u64,
        result: Result<UiValue, String>,
    },
    Closed {
        id: u64,
        reason: SubscriptionCloseReason,
    },
}

#[derive(Clone)]
pub struct SubscriptionEmitter {
    id: u64,
    sender: Sender<SubscriptionMessage>,
    lifetime: Arc<SubscriptionLifetime>,
}

impl SubscriptionEmitter {
    /// Emit one value from any Rust worker thread.
    ///
    /// # Errors
    ///
    /// Returns [`AsyncRuntimeError::Closed`] after the producer work returns,
    /// explicit cancellation, scope teardown, generation replacement, or
    /// registry drop.
    pub fn emit(&self, value: UiValue) -> Result<(), AsyncRuntimeError> {
        self.send(Ok(value))
    }

    /// Emit a structured error to the Rhai error callback.
    ///
    /// # Errors
    ///
    /// Returns [`AsyncRuntimeError::Closed`] after the producer work returns,
    /// explicit cancellation, scope teardown, generation replacement, or
    /// registry drop.
    pub fn emit_error(&self, message: impl Into<String>) -> Result<(), AsyncRuntimeError> {
        self.send(Err(message.into()))
    }

    fn send(&self, result: Result<UiValue, String>) -> Result<(), AsyncRuntimeError> {
        if let Some(reason) = self.lifetime.close_reason() {
            return Err(AsyncRuntimeError::Closed { reason });
        }
        self.sender
            .send(SubscriptionMessage::Value {
                id: self.id,
                result,
            })
            .map_err(|_| AsyncRuntimeError::Closed {
                reason: self
                    .lifetime
                    .close_reason()
                    .unwrap_or(SubscriptionCloseReason::RegistryDropped),
            })
    }

    /// Close the stream explicitly from the producer side.
    pub fn close(&self) {
        self.close_with_reason(SubscriptionCloseReason::ProducerClosed);
    }

    /// Return the first reason that closed this emitter, if any.
    #[must_use]
    pub fn close_reason(&self) -> Option<SubscriptionCloseReason> {
        self.lifetime.close_reason()
    }

    pub(crate) fn close_with_reason(&self, reason: SubscriptionCloseReason) {
        if self.lifetime.close(reason) {
            let _ = self.sender.send(SubscriptionMessage::Closed {
                id: self.id,
                reason,
            });
        }
    }
}

struct SubscriptionEntry {
    label: String,
    scope: AsyncScope,
    generation: ScriptGeneration,
    callbacks: CallbackPair,
    output: ValueSchema,
    lifetime: Arc<SubscriptionLifetime>,
    throttle: Duration,
    last_delivery: Option<Instant>,
    pending: Option<Result<UiValue, String>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SubscriptionClosure {
    pub label: String,
    pub scope: AsyncScope,
    pub reason: SubscriptionCloseReason,
}

pub struct SubscriptionRegistration {
    label: String,
    scope: AsyncScope,
    generation: ScriptGeneration,
    success: ScriptCallback,
    error: ScriptCallback,
    output: ValueSchema,
    throttle: Duration,
}

impl SubscriptionRegistration {
    #[must_use]
    pub fn new(
        label: impl Into<String>,
        scope: AsyncScope,
        generation: ScriptGeneration,
        success: ScriptCallback,
        error: ScriptCallback,
        output: ValueSchema,
    ) -> Self {
        Self {
            label: label.into(),
            scope,
            generation,
            success,
            error,
            output,
            throttle: Duration::ZERO,
        }
    }

    #[must_use]
    pub const fn with_throttle(mut self, throttle: Duration) -> Self {
        self.throttle = throttle;
        self
    }
}

pub struct SubscriptionRegistry {
    next_id: u64,
    entries: BTreeMap<u64, SubscriptionEntry>,
    sender: Sender<SubscriptionMessage>,
    receiver: Receiver<SubscriptionMessage>,
    closures: Vec<SubscriptionClosure>,
}

impl Default for SubscriptionRegistry {
    fn default() -> Self {
        let (sender, receiver) = channel();
        Self {
            next_id: 1,
            entries: BTreeMap::new(),
            sender,
            receiver,
            closures: Vec::new(),
        }
    }
}

impl fmt::Debug for SubscriptionRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubscriptionRegistry")
            .field("active", &self.entries.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl SubscriptionRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn subscribe(
        &mut self,
        registration: SubscriptionRegistration,
    ) -> (SubscriptionHandle, SubscriptionEmitter) {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let lifetime = Arc::new(SubscriptionLifetime::new());
        self.entries.insert(
            id,
            SubscriptionEntry {
                label: registration.label,
                scope: registration.scope,
                generation: registration.generation,
                callbacks: CallbackPair {
                    success: registration.success,
                    error: registration.error,
                },
                output: registration.output,
                lifetime: Arc::clone(&lifetime),
                throttle: registration.throttle,
                last_delivery: None,
                pending: None,
            },
        );
        (
            SubscriptionHandle(id),
            SubscriptionEmitter {
                id,
                sender: self.sender.clone(),
                lifetime,
            },
        )
    }

    #[must_use]
    pub fn cancel(&mut self, handle: SubscriptionHandle) -> bool {
        self.cancel_with_reason(handle, SubscriptionCloseReason::Cancelled)
    }

    pub(crate) fn cancel_with_reason(
        &mut self,
        handle: SubscriptionHandle,
        reason: SubscriptionCloseReason,
    ) -> bool {
        let Some(entry) = self.entries.remove(&handle.0) else {
            return false;
        };
        self.closures.push(close_subscription_entry(&entry, reason));
        true
    }

    pub fn cancel_scope(&mut self, scope: &AsyncScope) {
        let mut closures = Vec::new();
        self.entries.retain(|_, entry| {
            if &entry.scope == scope {
                closures.push(close_subscription_entry(
                    entry,
                    SubscriptionCloseReason::ScopeDisposed,
                ));
                false
            } else {
                true
            }
        });
        self.closures.extend(closures);
    }

    pub fn cancel_component_scope(&mut self, component: &ComponentInstancePath) {
        let mut closures = Vec::new();
        self.entries.retain(|_, entry| {
            let remove = entry.scope.is_within_component(component);
            if remove {
                closures.push(close_subscription_entry(
                    entry,
                    SubscriptionCloseReason::ScopeDisposed,
                ));
            }
            !remove
        });
        self.closures.extend(closures);
    }

    #[must_use]
    pub fn drain(&mut self, current: ScriptGeneration) -> Vec<AsyncDelivery> {
        let now = Instant::now();
        let mut closed = BTreeMap::new();
        loop {
            match self.receiver.try_recv() {
                Ok(SubscriptionMessage::Value { id, result }) => {
                    let Some(entry) = self.entries.get_mut(&id) else {
                        continue;
                    };
                    if entry.generation == current {
                        entry.pending = Some(result);
                    } else {
                        entry
                            .lifetime
                            .close(SubscriptionCloseReason::GenerationStale);
                        closed.entry(id).or_insert_with(|| {
                            entry
                                .lifetime
                                .close_reason()
                                .unwrap_or(SubscriptionCloseReason::GenerationStale)
                        });
                    }
                }
                Ok(SubscriptionMessage::Closed { id, reason }) => {
                    if self.entries.contains_key(&id) {
                        closed.entry(id).or_insert(reason);
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        let mut deliveries = Vec::new();
        for (id, entry) in &mut self.entries {
            if entry.generation != current {
                entry
                    .lifetime
                    .close(SubscriptionCloseReason::GenerationStale);
                closed.entry(*id).or_insert_with(|| {
                    entry
                        .lifetime
                        .close_reason()
                        .unwrap_or(SubscriptionCloseReason::GenerationStale)
                });
                continue;
            }
            let ready = closed.contains_key(id)
                || entry
                    .last_delivery
                    .is_none_or(|last| now.duration_since(last) >= entry.throttle);
            if ready && let Some(result) = entry.pending.take() {
                entry.last_delivery = Some(now);
                deliveries.push(subscription_delivery(entry, result));
            }
        }
        for (id, reason) in closed {
            if let Some(entry) = self.entries.remove(&id) {
                self.closures.push(close_subscription_entry(&entry, reason));
            }
        }
        deliveries
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn active_ids(&self) -> std::collections::BTreeSet<u64> {
        self.entries.keys().copied().collect()
    }

    pub(crate) fn retain_ids(&mut self, retained: &std::collections::BTreeSet<u64>) {
        let mut closures = Vec::new();
        self.entries.retain(|id, entry| {
            let keep = retained.contains(id);
            if !keep {
                closures.push(close_subscription_entry(
                    entry,
                    SubscriptionCloseReason::TransactionRolledBack,
                ));
            }
            keep
        });
        self.closures.extend(closures);
    }

    pub(crate) fn take_closures(&mut self) -> Vec<SubscriptionClosure> {
        std::mem::take(&mut self.closures)
    }
}

impl Drop for SubscriptionRegistry {
    fn drop(&mut self) {
        for entry in self.entries.values() {
            entry
                .lifetime
                .close(SubscriptionCloseReason::RegistryDropped);
        }
    }
}

fn close_subscription_entry(
    entry: &SubscriptionEntry,
    reason: SubscriptionCloseReason,
) -> SubscriptionClosure {
    entry.lifetime.close(reason);
    SubscriptionClosure {
        label: entry.label.clone(),
        scope: entry.scope.clone(),
        reason: entry.lifetime.close_reason().unwrap_or(reason),
    }
}

fn subscription_delivery(
    entry: &SubscriptionEntry,
    result: Result<UiValue, String>,
) -> AsyncDelivery {
    match result {
        Ok(value) => match entry.output.validate(&value.clone().into_dynamic()) {
            Ok(()) => AsyncDelivery {
                callback: entry.callbacks.success.clone(),
                payload: value,
                scope: entry.scope.clone(),
            },
            Err(error) => AsyncDelivery {
                callback: entry.callbacks.error.clone(),
                payload: error_payload(error.to_string()),
                scope: entry.scope.clone(),
            },
        },
        Err(error) => AsyncDelivery {
            callback: entry.callbacks.error.clone(),
            payload: error_payload(error),
            scope: entry.scope.clone(),
        },
    }
}

fn error_payload(message: String) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("kind".to_owned(), UiValue::String("async_error".to_owned())),
        ("message".to_owned(), UiValue::String(message)),
    ]))
}

#[derive(Debug, Error)]
pub enum AsyncRuntimeError {
    #[error("failed to spawn async worker: {0}")]
    Spawn(std::io::Error),
    #[error("subscription is closed: {reason}")]
    Closed { reason: SubscriptionCloseReason },
    #[error("async output is invalid: {0}")]
    InvalidOutput(#[from] SchemaValidationError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeEngine;

    fn callbacks() -> (ScriptCallback, ScriptCallback, ScriptGeneration) {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn view() { text("async") }
                    fn success(ctx, value) { value }
                    fn failure(ctx, error) { error }
                "#,
            )
            .unwrap();
        runtime.render(&compiled).unwrap();
        (
            runtime.callback(&compiled, "success").unwrap(),
            runtime.callback(&compiled, "failure").unwrap(),
            compiled.generation(),
        )
    }

    #[test]
    fn task_completion_delivers_on_foreground_drain() {
        let (success, error, generation) = callbacks();
        let mut tasks = TaskRegistry::new();
        tasks
            .spawn(
                AsyncScope::App,
                generation,
                success.clone(),
                error,
                ValueSchema::string(),
                || Ok(UiValue::String("done".to_owned())),
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let deliveries = tasks.drain(generation);
            if let Some(delivery) = deliveries.into_iter().next() {
                assert_eq!(delivery.callback, success);
                assert_eq!(delivery.payload, UiValue::String("done".to_owned()));
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }

    #[test]
    fn canceled_and_stale_tasks_never_deliver() {
        let (success, error, generation) = callbacks();
        let mut tasks = TaskRegistry::new();
        let handle = tasks
            .spawn(
                AsyncScope::App,
                generation,
                success.clone(),
                error.clone(),
                ValueSchema::Null,
                || Ok(UiValue::Null),
            )
            .unwrap();
        assert!(tasks.cancel(handle));
        std::thread::sleep(Duration::from_millis(10));
        assert!(tasks.drain(generation).is_empty());

        tasks
            .spawn(
                AsyncScope::App,
                generation,
                success,
                error,
                ValueSchema::Null,
                || Ok(UiValue::Null),
            )
            .unwrap();
        std::thread::sleep(Duration::from_millis(10));
        assert!(tasks.drain(generation.next()).is_empty());
    }

    #[test]
    fn window_and_component_scope_cancellation_preserves_app_tasks() {
        let (success, error, generation) = callbacks();
        let root = ComponentInstancePath::root("App", "settings");
        let mut tasks = TaskRegistry::new();
        for scope in [
            AsyncScope::App,
            AsyncScope::Window("settings".to_owned()),
            AsyncScope::Component(root.child("Panel", "root")),
            AsyncScope::Effect {
                component: root.child("Panel", "effect"),
                key: "watch".to_owned(),
                activation: 1,
            },
        ] {
            tasks
                .spawn(
                    scope,
                    generation,
                    success.clone(),
                    error.clone(),
                    ValueSchema::Null,
                    || Ok(UiValue::Null),
                )
                .unwrap();
        }
        tasks.cancel_scope(&AsyncScope::Window("settings".to_owned()));
        tasks.cancel_component_scope(&root);
        assert_eq!(tasks.active_count(), 1);
    }

    #[test]
    fn exact_effect_activation_cancellation_preserves_replacement_work() {
        let (success, error, generation) = callbacks();
        let component = ComponentInstancePath::root("App", "main").child("Probe", "primary");
        let old = AsyncScope::Effect {
            component: component.clone(),
            key: "watch".to_owned(),
            activation: 1,
        };
        let replacement = AsyncScope::Effect {
            component,
            key: "watch".to_owned(),
            activation: 2,
        };
        let mut tasks = TaskRegistry::new();
        for scope in [old.clone(), replacement.clone()] {
            tasks
                .spawn(
                    scope,
                    generation,
                    success.clone(),
                    error.clone(),
                    ValueSchema::Null,
                    || Ok(UiValue::Null),
                )
                .unwrap();
        }
        tasks.cancel_scope(&old);
        assert_eq!(tasks.active_count(), 1);

        let mut subscriptions = SubscriptionRegistry::new();
        for scope in [old.clone(), replacement] {
            let _ = subscriptions.subscribe(SubscriptionRegistration::new(
                "app.stream.watch",
                scope,
                generation,
                success.clone(),
                error.clone(),
                ValueSchema::Null,
            ));
        }
        subscriptions.cancel_scope(&old);
        assert_eq!(subscriptions.active_count(), 1);
        assert_eq!(subscriptions.take_closures().len(), 1);
    }

    #[test]
    fn subscription_throttles_to_latest_value_and_cancels() {
        let (success, error, generation) = callbacks();
        let mut subscriptions = SubscriptionRegistry::new();
        let registration = SubscriptionRegistration::new(
            "app.stream.watch",
            AsyncScope::Window("main".to_owned()),
            generation,
            success,
            error,
            ValueSchema::integer(),
        )
        .with_throttle(Duration::from_millis(50));
        let (handle, emitter) = subscriptions.subscribe(registration);
        emitter.emit(UiValue::Integer(1)).unwrap();
        let first = subscriptions.drain(generation);
        assert_eq!(first[0].payload, UiValue::Integer(1));
        emitter.emit(UiValue::Integer(2)).unwrap();
        emitter.emit(UiValue::Integer(3)).unwrap();
        assert!(subscriptions.drain(generation).is_empty());
        std::thread::sleep(Duration::from_millis(60));
        let latest = subscriptions.drain(generation);
        assert_eq!(latest[0].payload, UiValue::Integer(3));
        assert!(subscriptions.cancel(handle));
        assert!(matches!(
            emitter.emit(UiValue::Integer(4)),
            Err(AsyncRuntimeError::Closed {
                reason: SubscriptionCloseReason::Cancelled
            })
        ));
        assert_eq!(
            subscriptions.take_closures(),
            vec![SubscriptionClosure {
                label: "app.stream.watch".to_owned(),
                scope: AsyncScope::Window("main".to_owned()),
                reason: SubscriptionCloseReason::Cancelled,
            }]
        );
    }

    #[test]
    fn producer_close_and_registry_drop_report_first_close_reason() {
        let (success, error, generation) = callbacks();
        let mut subscriptions = SubscriptionRegistry::new();
        let registration = SubscriptionRegistration::new(
            "app.stream.watch",
            AsyncScope::App,
            generation,
            success.clone(),
            error.clone(),
            ValueSchema::integer(),
        );
        let (_, emitter) = subscriptions.subscribe(registration);
        emitter.close();
        let _ = subscriptions.drain(generation);
        assert!(matches!(
            emitter.emit(UiValue::Integer(1)),
            Err(AsyncRuntimeError::Closed {
                reason: SubscriptionCloseReason::ProducerClosed
            })
        ));
        assert_eq!(
            subscriptions.take_closures()[0].reason,
            SubscriptionCloseReason::ProducerClosed
        );

        let orphan = {
            let mut registry = SubscriptionRegistry::new();
            let registration = SubscriptionRegistration::new(
                "app.stream.orphan",
                AsyncScope::App,
                generation,
                success,
                error,
                ValueSchema::integer(),
            );
            let (_, emitter) = registry.subscribe(registration);
            emitter
        };
        assert!(matches!(
            orphan.emit(UiValue::Integer(2)),
            Err(AsyncRuntimeError::Closed {
                reason: SubscriptionCloseReason::RegistryDropped
            })
        ));
    }
}
