use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
        self.entries.retain(|_, entry| {
            !matches!(&entry.scope, AsyncScope::Component(path) if path.is_within(component))
        });
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
    },
}

#[derive(Clone)]
pub struct SubscriptionEmitter {
    id: u64,
    sender: Sender<SubscriptionMessage>,
    active: Arc<AtomicBool>,
}

impl SubscriptionEmitter {
    /// Emit one value from any Rust worker thread.
    ///
    /// # Errors
    ///
    /// Returns [`AsyncRuntimeError::Closed`] after cancellation or registry drop.
    pub fn emit(&self, value: UiValue) -> Result<(), AsyncRuntimeError> {
        self.send(Ok(value))
    }

    /// Emit a structured error to the Rhai error callback.
    ///
    /// # Errors
    ///
    /// Returns [`AsyncRuntimeError::Closed`] after cancellation or registry drop.
    pub fn emit_error(&self, message: impl Into<String>) -> Result<(), AsyncRuntimeError> {
        self.send(Err(message.into()))
    }

    fn send(&self, result: Result<UiValue, String>) -> Result<(), AsyncRuntimeError> {
        if !self.active.load(Ordering::Acquire) {
            return Err(AsyncRuntimeError::Closed);
        }
        self.sender
            .send(SubscriptionMessage::Value {
                id: self.id,
                result,
            })
            .map_err(|_| AsyncRuntimeError::Closed)
    }

    pub fn close(&self) {
        if self.active.swap(false, Ordering::AcqRel) {
            let _ = self
                .sender
                .send(SubscriptionMessage::Closed { id: self.id });
        }
    }
}

struct SubscriptionEntry {
    scope: AsyncScope,
    generation: ScriptGeneration,
    callbacks: CallbackPair,
    output: ValueSchema,
    active: Arc<AtomicBool>,
    throttle: Duration,
    last_delivery: Option<Instant>,
    pending: Option<Result<UiValue, String>>,
}

pub struct SubscriptionRegistry {
    next_id: u64,
    entries: BTreeMap<u64, SubscriptionEntry>,
    sender: Sender<SubscriptionMessage>,
    receiver: Receiver<SubscriptionMessage>,
}

impl Default for SubscriptionRegistry {
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
        scope: AsyncScope,
        generation: ScriptGeneration,
        success: ScriptCallback,
        error: ScriptCallback,
        output: ValueSchema,
        throttle: Duration,
    ) -> (SubscriptionHandle, SubscriptionEmitter) {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let active = Arc::new(AtomicBool::new(true));
        self.entries.insert(
            id,
            SubscriptionEntry {
                scope,
                generation,
                callbacks: CallbackPair { success, error },
                output,
                active: Arc::clone(&active),
                throttle,
                last_delivery: None,
                pending: None,
            },
        );
        (
            SubscriptionHandle(id),
            SubscriptionEmitter {
                id,
                sender: self.sender.clone(),
                active,
            },
        )
    }

    #[must_use]
    pub fn cancel(&mut self, handle: SubscriptionHandle) -> bool {
        self.entries.remove(&handle.0).is_some_and(|entry| {
            entry.active.store(false, Ordering::Release);
            true
        })
    }

    pub fn cancel_scope(&mut self, scope: &AsyncScope) {
        self.entries.retain(|_, entry| {
            if &entry.scope == scope {
                entry.active.store(false, Ordering::Release);
                false
            } else {
                true
            }
        });
    }

    pub fn cancel_component_scope(&mut self, component: &ComponentInstancePath) {
        self.entries.retain(|_, entry| {
            let remove =
                matches!(&entry.scope, AsyncScope::Component(path) if path.is_within(component));
            if remove {
                entry.active.store(false, Ordering::Release);
            }
            !remove
        });
    }

    #[must_use]
    pub fn drain(&mut self, current: ScriptGeneration) -> Vec<AsyncDelivery> {
        let now = Instant::now();
        let mut closed = std::collections::BTreeSet::new();
        loop {
            match self.receiver.try_recv() {
                Ok(SubscriptionMessage::Value { id, result }) => {
                    let Some(entry) = self.entries.get_mut(&id) else {
                        continue;
                    };
                    if entry.generation == current {
                        entry.pending = Some(result);
                    } else {
                        entry.active.store(false, Ordering::Release);
                        closed.insert(id);
                    }
                }
                Ok(SubscriptionMessage::Closed { id }) => {
                    closed.insert(id);
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        let mut deliveries = Vec::new();
        for (id, entry) in &mut self.entries {
            if entry.generation != current {
                closed.insert(*id);
                continue;
            }
            let ready = closed.contains(id)
                || entry
                    .last_delivery
                    .is_none_or(|last| now.duration_since(last) >= entry.throttle);
            if ready && let Some(result) = entry.pending.take() {
                entry.last_delivery = Some(now);
                deliveries.push(subscription_delivery(entry, result));
            }
        }
        for id in closed {
            self.entries.remove(&id);
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
        self.entries.retain(|id, entry| {
            let keep = retained.contains(id);
            if !keep {
                entry.active.store(false, Ordering::Release);
            }
            keep
        });
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
    #[error("task or subscription is closed")]
    Closed,
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
    fn subscription_throttles_to_latest_value_and_cancels() {
        let (success, error, generation) = callbacks();
        let mut subscriptions = SubscriptionRegistry::new();
        let (handle, emitter) = subscriptions.subscribe(
            AsyncScope::Window("main".to_owned()),
            generation,
            success,
            error,
            ValueSchema::integer(),
            Duration::from_millis(50),
        );
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
        assert!(emitter.emit(UiValue::Integer(4)).is_err());
    }
}
