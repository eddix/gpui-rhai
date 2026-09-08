use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{Receiver, Sender, SyncSender, TrySendError, channel, sync_channel};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use event_listener::{Event, EventListener};
use rhai::{CustomType, TypeBuilder};
use thiserror::Error;

use crate::{
    ComponentInstancePath, SchemaValidationError, ScriptCallback, ScriptGeneration, UiValue,
    ValueSchema,
};

type BackgroundJob = Box<dyn FnOnce() + Send + 'static>;

struct BackgroundExecutor {
    sender: SyncSender<BackgroundJob>,
}

impl BackgroundExecutor {
    fn start() -> Option<Self> {
        let (sender, receiver) = sync_channel::<BackgroundJob>(1_024);
        let receiver = Arc::new(Mutex::new(receiver));
        let workers = std::thread::available_parallelism()
            .map_or(2, usize::from)
            .clamp(1, 4);
        for index in 0..workers {
            let receiver = Arc::clone(&receiver);
            if std::thread::Builder::new()
                .name(format!("gpui-rhai-worker-{index}"))
                .spawn(move || {
                    loop {
                        let job = receiver
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .recv();
                        let Ok(job) = job else {
                            break;
                        };
                        job();
                    }
                })
                .is_err()
            {
                return None;
            }
        }
        Some(Self { sender })
    }

    fn submit(&self, job: BackgroundJob) -> Result<(), AsyncRuntimeError> {
        self.sender.try_send(job).map_err(|error| match error {
            TrySendError::Full(_) => AsyncRuntimeError::WorkerQueueFull,
            TrySendError::Disconnected(_) => AsyncRuntimeError::WorkerPoolUnavailable,
        })
    }
}

fn background_executor() -> Result<&'static BackgroundExecutor, AsyncRuntimeError> {
    static EXECUTOR: OnceLock<Option<BackgroundExecutor>> = OnceLock::new();
    EXECUTOR
        .get_or_init(BackgroundExecutor::start)
        .as_ref()
        .ok_or(AsyncRuntimeError::WorkerPoolUnavailable)
}

/// Thread-safe edge notification for foreground delivery pumps.
///
/// The message queues remain the source of truth. A wake may be coalesced or
/// arrive before a view subscribes; each foreground pump therefore drains once
/// before awaiting its first notification.
#[derive(Clone, Debug, Default)]
pub(crate) struct AsyncWake {
    event: Arc<Event>,
}

impl AsyncWake {
    pub(crate) fn listen(&self) -> EventListener {
        self.event.listen()
    }

    pub(crate) fn notify(&self) {
        self.event.notify(usize::MAX);
    }
}

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

#[derive(Clone, Debug, Default)]
pub struct TaskCancellation {
    cancelled: Arc<AtomicBool>,
}

impl TaskCancellation {
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
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

#[derive(Clone, Debug)]
struct CallbackPair {
    success: ScriptCallback,
    error: ScriptCallback,
}

#[derive(Clone, Debug)]
struct TaskEntry {
    scope: AsyncScope,
    generation: ScriptGeneration,
    callbacks: CallbackPair,
    output: ValueSchema,
    cancellation: TaskCancellation,
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
    wake: AsyncWake,
    deferred_cancellations: BTreeMap<u64, TaskEntry>,
    transaction_depth: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct TaskRegistrySnapshot {
    entries: BTreeMap<u64, TaskEntry>,
    deferred_cancellations: BTreeMap<u64, TaskEntry>,
    transaction_depth: usize,
}

impl Default for TaskRegistry {
    fn default() -> Self {
        let (sender, receiver) = channel();
        Self {
            next_id: 1,
            entries: BTreeMap::new(),
            sender,
            receiver,
            wake: AsyncWake::default(),
            deferred_cancellations: BTreeMap::new(),
            transaction_depth: 0,
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

    pub(crate) fn begin_transaction(&mut self) {
        self.transaction_depth = self.transaction_depth.saturating_add(1);
    }

    pub(crate) fn commit_transaction(&mut self) {
        self.transaction_depth = self.transaction_depth.saturating_sub(1);
        if self.transaction_depth == 0 {
            for entry in std::mem::take(&mut self.deferred_cancellations).into_values() {
                entry.cancellation.cancel();
            }
        }
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
        self.spawn_cancellable(scope, generation, success, error, output, move |_| work())
    }

    pub(crate) fn spawn_cancellable(
        &mut self,
        scope: AsyncScope,
        generation: ScriptGeneration,
        success: ScriptCallback,
        error: ScriptCallback,
        output: ValueSchema,
        work: impl FnOnce(TaskCancellation) -> Result<UiValue, String> + Send + 'static,
    ) -> Result<TaskHandle, AsyncRuntimeError> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let cancellation = TaskCancellation::default();
        self.entries.insert(
            id,
            TaskEntry {
                scope,
                generation,
                callbacks: CallbackPair { success, error },
                output,
                cancellation: cancellation.clone(),
            },
        );
        let sender = self.sender.clone();
        let wake = self.wake.clone();
        let job = Box::new(move || {
            let result =
                catch_unwind(AssertUnwindSafe(|| work(cancellation))).unwrap_or_else(|payload| {
                    let message = payload.downcast_ref::<&str>().map_or_else(
                        || {
                            payload.downcast_ref::<String>().map_or_else(
                                || "background task panicked".to_owned(),
                                |message| format!("background task panicked: {message}"),
                            )
                        },
                        |message| format!("background task panicked: {message}"),
                    );
                    Err(message)
                });
            if sender.send(TaskMessage { id, result }).is_ok() {
                wake.notify();
            }
        });
        if let Err(error) = background_executor().and_then(|executor| executor.submit(job)) {
            self.entries.remove(&id);
            return Err(error);
        }
        Ok(TaskHandle(id))
    }

    #[must_use]
    pub fn cancel(&mut self, handle: TaskHandle) -> bool {
        let Some(entry) = self.entries.remove(&handle.0) else {
            return false;
        };
        self.defer_or_cancel(handle.0, entry);
        true
    }

    pub fn cancel_scope(&mut self, scope: &AsyncScope) {
        let ids = self
            .entries
            .iter()
            .filter_map(|(id, entry)| (&entry.scope == scope).then_some(*id))
            .collect::<Vec<_>>();
        self.cancel_ids(ids);
    }

    pub fn cancel_component_scope(&mut self, component: &ComponentInstancePath) {
        let ids = self
            .entries
            .iter()
            .filter_map(|(id, entry)| entry.scope.is_within_component(component).then_some(*id))
            .collect::<Vec<_>>();
        self.cancel_ids(ids);
    }

    #[must_use]
    pub fn drain(&mut self, current: ScriptGeneration) -> Vec<AsyncDelivery> {
        self.drain_up_to(current, usize::MAX)
    }

    #[must_use]
    pub(crate) fn drain_up_to(
        &mut self,
        current: ScriptGeneration,
        limit: usize,
    ) -> Vec<AsyncDelivery> {
        let mut deliveries = Vec::new();
        while deliveries.len() < limit {
            let Ok(message) = self.receiver.try_recv() else {
                break;
            };
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

    pub(crate) fn snapshot(&self) -> TaskRegistrySnapshot {
        TaskRegistrySnapshot {
            entries: self.entries.clone(),
            deferred_cancellations: self.deferred_cancellations.clone(),
            transaction_depth: self.transaction_depth,
        }
    }

    pub(crate) fn restore(&mut self, snapshot: TaskRegistrySnapshot) {
        for (id, entry) in self
            .entries
            .iter()
            .chain(self.deferred_cancellations.iter())
        {
            if !snapshot.entries.contains_key(id)
                && !snapshot.deferred_cancellations.contains_key(id)
            {
                entry.cancellation.cancel();
            }
        }
        self.entries = snapshot.entries;
        self.deferred_cancellations = snapshot.deferred_cancellations;
        self.transaction_depth = snapshot.transaction_depth;
    }

    fn cancel_ids(&mut self, ids: impl IntoIterator<Item = u64>) {
        for id in ids {
            if let Some(entry) = self.entries.remove(&id) {
                self.defer_or_cancel(id, entry);
            }
        }
    }

    fn defer_or_cancel(&mut self, id: u64, entry: TaskEntry) {
        if self.transaction_depth > 0 {
            self.deferred_cancellations.insert(id, entry);
        } else {
            entry.cancellation.cancel();
        }
    }

    pub(crate) fn wake(&self) -> AsyncWake {
        self.wake.clone()
    }
}

fn task_delivery(entry: TaskEntry, result: Result<UiValue, String>) -> AsyncDelivery {
    match result {
        Ok(value) => match entry.output.validate_ui_value(&value) {
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

const DEFAULT_SUBSCRIPTION_CAPACITY: usize = 64;
const MAX_SUBSCRIPTION_CAPACITY: usize = 4_096;
const MAX_SUBSCRIPTION_THROTTLE: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubscriptionDeliveryPolicy {
    All,
    Latest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubscriptionOptions {
    delivery: SubscriptionDeliveryPolicy,
    capacity: usize,
    throttle: Duration,
}

impl Default for SubscriptionOptions {
    fn default() -> Self {
        Self {
            delivery: SubscriptionDeliveryPolicy::All,
            capacity: DEFAULT_SUBSCRIPTION_CAPACITY,
            throttle: Duration::ZERO,
        }
    }
}

impl SubscriptionOptions {
    /// Build an explicit bounded subscription delivery contract.
    ///
    /// # Errors
    ///
    /// Returns for a capacity outside 1..=4096 or a throttle above 60 seconds.
    pub fn new(
        delivery: SubscriptionDeliveryPolicy,
        capacity: usize,
        throttle: Duration,
    ) -> Result<Self, AsyncRuntimeError> {
        if !(1..=MAX_SUBSCRIPTION_CAPACITY).contains(&capacity) {
            return Err(AsyncRuntimeError::InvalidCapacity(capacity));
        }
        if throttle > MAX_SUBSCRIPTION_THROTTLE {
            return Err(AsyncRuntimeError::InvalidThrottle(throttle));
        }
        Ok(Self {
            delivery,
            capacity,
            throttle,
        })
    }

    #[must_use]
    pub const fn delivery(self) -> SubscriptionDeliveryPolicy {
        self.delivery
    }

    #[must_use]
    pub const fn capacity(self) -> usize {
        self.capacity
    }

    #[must_use]
    pub const fn throttle(self) -> Duration {
        self.throttle
    }
}

#[derive(Debug)]
struct SubscriptionBuffer {
    values: VecDeque<Result<UiValue, String>>,
    delivery: SubscriptionDeliveryPolicy,
    capacity: usize,
}

#[derive(Debug)]
struct SubscriptionQueue {
    buffer: Mutex<SubscriptionBuffer>,
    space: Condvar,
    #[cfg(test)]
    blocked_producers: AtomicUsize,
}

impl SubscriptionBuffer {
    fn push(&mut self, result: Result<UiValue, String>) -> Result<(), AsyncRuntimeError> {
        match self.delivery {
            SubscriptionDeliveryPolicy::All if self.values.len() >= self.capacity => {
                Err(AsyncRuntimeError::Backpressure {
                    capacity: self.capacity,
                })
            }
            SubscriptionDeliveryPolicy::All => {
                self.values.push_back(result);
                Ok(())
            }
            SubscriptionDeliveryPolicy::Latest => {
                if let Some(latest) = self.values.back_mut() {
                    *latest = result;
                } else {
                    self.values.push_back(result);
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone)]
pub struct SubscriptionEmitter {
    pending: Arc<SubscriptionQueue>,
    lifetime: Arc<SubscriptionLifetime>,
    wake: AsyncWake,
}

impl SubscriptionEmitter {
    /// Emit one value from any Rust worker thread.
    ///
    /// # Errors
    ///
    /// Returns [`AsyncRuntimeError::Closed`] after teardown, or
    /// [`AsyncRuntimeError::Backpressure`] when a lossless stream's bounded
    /// pending queue is full. Latest-only streams explicitly replace their
    /// pending value instead.
    pub fn emit(&self, value: UiValue) -> Result<(), AsyncRuntimeError> {
        self.send(Ok(value))
    }

    /// Emit one lossless value, waiting for foreground capacity or cancellation.
    /// Latest-only streams never block because they explicitly replace a value.
    ///
    /// # Errors
    ///
    /// Returns when the stream closes or its shared queue is poisoned.
    pub fn emit_blocking(&self, value: UiValue) -> Result<(), AsyncRuntimeError> {
        self.send_blocking(Ok(value))
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
        let mut pending = self
            .pending
            .buffer
            .lock()
            .map_err(|_| AsyncRuntimeError::Poisoned)?;
        if let Some(reason) = self.lifetime.close_reason() {
            return Err(AsyncRuntimeError::Closed { reason });
        }
        pending.push(result)?;
        drop(pending);
        self.wake.notify();
        Ok(())
    }

    fn send_blocking(&self, result: Result<UiValue, String>) -> Result<(), AsyncRuntimeError> {
        let mut result = Some(result);
        let mut pending = self
            .pending
            .buffer
            .lock()
            .map_err(|_| AsyncRuntimeError::Poisoned)?;
        loop {
            if let Some(reason) = self.lifetime.close_reason() {
                return Err(AsyncRuntimeError::Closed { reason });
            }
            if pending.delivery == SubscriptionDeliveryPolicy::All
                && pending.values.len() >= pending.capacity
            {
                #[cfg(test)]
                self.pending
                    .blocked_producers
                    .fetch_add(1, Ordering::AcqRel);
                let waited = self.pending.space.wait(pending);
                #[cfg(test)]
                self.pending
                    .blocked_producers
                    .fetch_sub(1, Ordering::AcqRel);
                pending = waited.map_err(|_| AsyncRuntimeError::Poisoned)?;
                continue;
            }
            pending.push(result.take().expect("subscription value is sent once"))?;
            drop(pending);
            self.wake.notify();
            return Ok(());
        }
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
        if close_subscription_queue(&self.pending, &self.lifetime, reason, false) == reason {
            self.wake.notify();
        }
    }
}

#[derive(Clone, Debug)]
struct SubscriptionEntry {
    label: String,
    scope: AsyncScope,
    generation: ScriptGeneration,
    callbacks: CallbackPair,
    output: ValueSchema,
    lifetime: Arc<SubscriptionLifetime>,
    pending: Arc<SubscriptionQueue>,
    delivery: SubscriptionDeliveryPolicy,
    throttle: Duration,
    last_delivery: Option<Instant>,
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
    delivery: SubscriptionDeliveryPolicy,
    capacity: usize,
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
            delivery: SubscriptionDeliveryPolicy::All,
            capacity: DEFAULT_SUBSCRIPTION_CAPACITY,
            throttle: Duration::ZERO,
        }
    }

    /// Set the minimum interval between foreground delivery batches.
    ///
    /// Lossless streams retain every value. Latest-only streams coalesce values
    /// received during the interval.
    #[must_use]
    pub const fn with_throttle(mut self, throttle: Duration) -> Self {
        self.throttle = throttle;
        self
    }

    /// Select lossless ordered or explicit latest-only delivery.
    #[must_use]
    pub const fn with_delivery_policy(mut self, delivery: SubscriptionDeliveryPolicy) -> Self {
        self.delivery = delivery;
        self
    }

    #[must_use]
    pub const fn with_options(mut self, options: SubscriptionOptions) -> Self {
        self.delivery = options.delivery;
        self.capacity = options.capacity;
        self.throttle = options.throttle;
        self
    }

    /// Bound pending values for lossless delivery.
    ///
    /// # Errors
    ///
    /// Returns [`AsyncRuntimeError::InvalidCapacity`] outside 1..=4096.
    pub fn with_capacity(mut self, capacity: usize) -> Result<Self, AsyncRuntimeError> {
        if !(1..=MAX_SUBSCRIPTION_CAPACITY).contains(&capacity) {
            return Err(AsyncRuntimeError::InvalidCapacity(capacity));
        }
        self.capacity = capacity;
        Ok(self)
    }
}

pub struct SubscriptionRegistry {
    next_id: u64,
    entries: BTreeMap<u64, SubscriptionEntry>,
    closures: Vec<SubscriptionClosure>,
    deferred_closures: BTreeMap<u64, (SubscriptionEntry, SubscriptionCloseReason)>,
    transaction_depth: usize,
    wake: AsyncWake,
}

impl Default for SubscriptionRegistry {
    fn default() -> Self {
        Self {
            next_id: 1,
            entries: BTreeMap::new(),
            closures: Vec::new(),
            deferred_closures: BTreeMap::new(),
            transaction_depth: 0,
            wake: AsyncWake::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SubscriptionRegistrySnapshot {
    entries: BTreeMap<u64, SubscriptionEntry>,
    deferred_closures: BTreeMap<u64, (SubscriptionEntry, SubscriptionCloseReason)>,
    transaction_depth: usize,
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

    pub(crate) fn begin_transaction(&mut self) {
        self.transaction_depth = self.transaction_depth.saturating_add(1);
    }

    pub(crate) fn commit_transaction(&mut self) {
        self.transaction_depth = self.transaction_depth.saturating_sub(1);
        if self.transaction_depth == 0 {
            self.finalize_deferred_closures();
        }
    }

    pub(crate) fn snapshot(&self) -> SubscriptionRegistrySnapshot {
        SubscriptionRegistrySnapshot {
            entries: self.entries.clone(),
            deferred_closures: self.deferred_closures.clone(),
            transaction_depth: self.transaction_depth,
        }
    }

    pub(crate) fn restore(&mut self, snapshot: SubscriptionRegistrySnapshot) {
        for (id, entry) in &self.entries {
            if !snapshot.entries.contains_key(id) && !snapshot.deferred_closures.contains_key(id) {
                self.closures.push(close_subscription_entry(
                    entry,
                    SubscriptionCloseReason::TransactionRolledBack,
                ));
            }
        }
        for (id, (entry, _)) in &self.deferred_closures {
            if !snapshot.entries.contains_key(id) && !snapshot.deferred_closures.contains_key(id) {
                self.closures.push(close_subscription_entry(
                    entry,
                    SubscriptionCloseReason::TransactionRolledBack,
                ));
            }
        }
        self.entries = snapshot.entries;
        self.deferred_closures = snapshot.deferred_closures;
        self.transaction_depth = snapshot.transaction_depth;
    }

    fn defer_or_close(
        &mut self,
        id: u64,
        entry: SubscriptionEntry,
        reason: SubscriptionCloseReason,
    ) {
        if self.transaction_depth > 0 {
            self.deferred_closures.insert(id, (entry, reason));
        } else {
            self.closures.push(close_subscription_entry(&entry, reason));
        }
    }

    fn finalize_deferred_closures(&mut self) {
        let deferred = std::mem::take(&mut self.deferred_closures);
        self.closures.extend(
            deferred
                .into_values()
                .map(|(entry, reason)| close_subscription_entry(&entry, reason)),
        );
    }

    #[must_use]
    pub fn subscribe(
        &mut self,
        registration: SubscriptionRegistration,
    ) -> (SubscriptionHandle, SubscriptionEmitter) {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let lifetime = Arc::new(SubscriptionLifetime::new());
        let pending = Arc::new(SubscriptionQueue {
            buffer: Mutex::new(SubscriptionBuffer {
                values: VecDeque::new(),
                delivery: registration.delivery,
                capacity: registration.capacity,
            }),
            space: Condvar::new(),
            #[cfg(test)]
            blocked_producers: AtomicUsize::new(0),
        });
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
                pending: Arc::clone(&pending),
                delivery: registration.delivery,
                throttle: registration.throttle,
                last_delivery: None,
            },
        );
        (
            SubscriptionHandle(id),
            SubscriptionEmitter {
                pending,
                lifetime,
                wake: self.wake.clone(),
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
        self.defer_or_close(handle.0, entry, reason);
        true
    }

    pub fn cancel_scope(&mut self, scope: &AsyncScope) {
        let mut closures = Vec::new();
        self.entries.retain(|id, entry| {
            if &entry.scope == scope {
                closures.push((*id, entry.clone(), SubscriptionCloseReason::ScopeDisposed));
                false
            } else {
                true
            }
        });
        for (id, entry, reason) in closures {
            self.defer_or_close(id, entry, reason);
        }
    }

    pub fn cancel_component_scope(&mut self, component: &ComponentInstancePath) {
        let mut closures = Vec::new();
        self.entries.retain(|id, entry| {
            let remove = entry.scope.is_within_component(component);
            if remove {
                closures.push((*id, entry.clone(), SubscriptionCloseReason::ScopeDisposed));
            }
            !remove
        });
        for (id, entry, reason) in closures {
            self.defer_or_close(id, entry, reason);
        }
    }

    #[must_use]
    pub fn drain(&mut self, current: ScriptGeneration) -> Vec<AsyncDelivery> {
        self.drain_up_to(current, usize::MAX)
    }

    #[must_use]
    pub(crate) fn drain_up_to(
        &mut self,
        current: ScriptGeneration,
        limit: usize,
    ) -> Vec<AsyncDelivery> {
        let now = Instant::now();
        let mut closed = BTreeMap::new();
        let mut deliveries = Vec::new();
        for (id, entry) in &mut self.entries {
            if entry.generation != current {
                let reason = close_subscription_queue(
                    &entry.pending,
                    &entry.lifetime,
                    SubscriptionCloseReason::GenerationStale,
                    true,
                );
                closed.entry(*id).or_insert(reason);
                continue;
            }
            if let Some(reason) = entry.lifetime.close_reason() {
                closed.insert(*id, reason);
            }
            let ready = closed.contains_key(id)
                || entry
                    .last_delivery
                    .is_none_or(|last| now.duration_since(last) >= entry.throttle);
            if ready && deliveries.len() < limit {
                let mut pending = entry
                    .pending
                    .buffer
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let requested = if closed.contains_key(id)
                    || entry.delivery == SubscriptionDeliveryPolicy::Latest
                    || entry.throttle.is_zero()
                {
                    pending.values.len()
                } else {
                    usize::from(!pending.values.is_empty())
                };
                let take = requested.min(limit.saturating_sub(deliveries.len()));
                for result in pending.values.drain(..take) {
                    deliveries.push(subscription_delivery(entry, result));
                }
                if take > 0 {
                    entry.pending.space.notify_all();
                }
                if take > 0 {
                    entry.last_delivery = Some(now);
                }
            }
        }
        for (id, reason) in closed {
            let empty = self.entries.get(&id).is_none_or(|entry| {
                entry
                    .pending
                    .buffer
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .values
                    .is_empty()
            });
            if empty && let Some(entry) = self.entries.remove(&id) {
                self.closures.push(close_subscription_entry(&entry, reason));
            }
        }
        deliveries
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn take_closures(&mut self) -> Vec<SubscriptionClosure> {
        if self.transaction_depth == 0 {
            self.finalize_deferred_closures();
        }
        std::mem::take(&mut self.closures)
    }

    pub(crate) fn wake(&self) -> AsyncWake {
        self.wake.clone()
    }
}

impl Drop for SubscriptionRegistry {
    fn drop(&mut self) {
        for entry in self
            .entries
            .values()
            .chain(self.deferred_closures.values().map(|(entry, _)| entry))
        {
            close_subscription_queue(
                &entry.pending,
                &entry.lifetime,
                SubscriptionCloseReason::RegistryDropped,
                true,
            );
        }
    }
}

fn close_subscription_queue(
    pending: &SubscriptionQueue,
    lifetime: &SubscriptionLifetime,
    reason: SubscriptionCloseReason,
    discard: bool,
) -> SubscriptionCloseReason {
    // The close predicate and Condvar wait are synchronized by the same mutex.
    // This prevents a producer from observing an open stream, missing the close
    // notification, and then sleeping forever on a full queue.
    let mut buffer = pending
        .buffer
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    lifetime.close(reason);
    if discard {
        buffer.values.clear();
    }
    let reason = lifetime.close_reason().unwrap_or(reason);
    drop(buffer);
    pending.space.notify_all();
    reason
}

fn close_subscription_entry(
    entry: &SubscriptionEntry,
    reason: SubscriptionCloseReason,
) -> SubscriptionClosure {
    let reason = close_subscription_queue(&entry.pending, &entry.lifetime, reason, true);
    SubscriptionClosure {
        label: entry.label.clone(),
        scope: entry.scope.clone(),
        reason,
    }
}

fn subscription_delivery(
    entry: &SubscriptionEntry,
    result: Result<UiValue, String>,
) -> AsyncDelivery {
    match result {
        Ok(value) => match entry.output.validate_ui_value(&value) {
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
    #[error("background worker pool is unavailable")]
    WorkerPoolUnavailable,
    #[error("background worker queue is full")]
    WorkerQueueFull,
    #[error("subscription is closed: {reason}")]
    Closed { reason: SubscriptionCloseReason },
    #[error("async output is invalid: {0}")]
    InvalidOutput(#[from] SchemaValidationError),
    #[error("subscription pending queue reached its capacity of {capacity}")]
    Backpressure { capacity: usize },
    #[error("subscription capacity must be between 1 and {MAX_SUBSCRIPTION_CAPACITY}, got {0}")]
    InvalidCapacity(usize),
    #[error("subscription throttle must not exceed 60 seconds, got {0:?}")]
    InvalidThrottle(Duration),
    #[error("subscription pending queue is poisoned")]
    Poisoned,
    #[error("suspended view delivery queue reached its capacity of {capacity}")]
    SuspendedBackpressure { capacity: usize },
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
    fn task_panic_becomes_an_error_delivery_and_releases_the_entry() {
        let (success, error, generation) = callbacks();
        let mut tasks = TaskRegistry::new();
        tasks
            .spawn(
                AsyncScope::App,
                generation,
                success,
                error.clone(),
                ValueSchema::Null,
                || panic!("task failed"),
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let deliveries = tasks.drain(generation);
            if let Some(delivery) = deliveries.into_iter().next() {
                assert_eq!(delivery.callback, error);
                assert_eq!(tasks.active_count(), 0);
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
    fn task_cancellation_is_reversible_until_transaction_commit() {
        let (success, error, generation) = callbacks();
        let (release, wait) = std::sync::mpsc::channel();
        let mut tasks = TaskRegistry::new();
        let handle = tasks
            .spawn_cancellable(
                AsyncScope::App,
                generation,
                success,
                error,
                ValueSchema::integer(),
                move |cancellation| {
                    wait.recv().unwrap();
                    assert!(!cancellation.is_cancelled());
                    Ok(UiValue::Integer(9))
                },
            )
            .unwrap();
        let snapshot = tasks.snapshot();
        tasks.begin_transaction();
        assert!(tasks.cancel(handle));
        tasks.restore(snapshot);
        assert_eq!(tasks.active_count(), 1);
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let deliveries = tasks.drain(generation);
            if !deliveries.is_empty() {
                assert_eq!(deliveries[0].payload, UiValue::Integer(9));
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
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
        .with_delivery_policy(SubscriptionDeliveryPolicy::Latest)
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
    fn subscription_defaults_to_bounded_ordered_delivery() {
        let (success, error, generation) = callbacks();
        let registration = SubscriptionRegistration::new(
            "app.stream.events",
            AsyncScope::App,
            generation,
            success,
            error,
            ValueSchema::integer(),
        )
        .with_capacity(2)
        .unwrap();
        let mut subscriptions = SubscriptionRegistry::new();
        let (_, emitter) = subscriptions.subscribe(registration);
        emitter.emit(UiValue::Integer(1)).unwrap();
        emitter.emit(UiValue::Integer(2)).unwrap();
        assert!(matches!(
            emitter.emit(UiValue::Integer(3)),
            Err(AsyncRuntimeError::Backpressure { capacity: 2 })
        ));
        assert_eq!(
            subscriptions
                .drain(generation)
                .into_iter()
                .map(|delivery| delivery.payload)
                .collect::<Vec<_>>(),
            vec![UiValue::Integer(1), UiValue::Integer(2)]
        );
        emitter.emit(UiValue::Integer(3)).unwrap();
        assert_eq!(
            subscriptions.drain(generation)[0].payload,
            UiValue::Integer(3)
        );
    }

    #[test]
    fn stale_generation_discards_buffer_and_reclaims_subscription() {
        let (success, error, generation) = callbacks();
        let mut subscriptions = SubscriptionRegistry::new();
        let (_, emitter) = subscriptions.subscribe(
            SubscriptionRegistration::new(
                "app.stream.stale",
                AsyncScope::App,
                generation,
                success,
                error,
                ValueSchema::integer(),
            )
            .with_capacity(1)
            .unwrap(),
        );
        emitter.emit(UiValue::Integer(1)).unwrap();
        let pending = Arc::clone(&emitter.pending);
        let blocked = emitter.clone();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let producer = std::thread::spawn(move || {
            done_tx
                .send(blocked.emit_blocking(UiValue::Integer(2)))
                .unwrap();
        });
        let deadline = Instant::now() + Duration::from_secs(1);
        while pending.blocked_producers.load(Ordering::Acquire) == 0 {
            assert!(
                Instant::now() < deadline,
                "producer did not reach the capacity wait"
            );
            std::thread::yield_now();
        }

        let current = generation.next();
        assert!(subscriptions.drain_up_to(current, 0).is_empty());
        assert!(matches!(
            done_rx.recv_timeout(Duration::from_secs(1)).unwrap(),
            Err(AsyncRuntimeError::Closed {
                reason: SubscriptionCloseReason::GenerationStale
            })
        ));
        producer.join().unwrap();
        assert_eq!(subscriptions.active_count(), 0);
        assert_eq!(
            subscriptions.take_closures()[0].reason,
            SubscriptionCloseReason::GenerationStale
        );
        assert!(matches!(
            emitter.emit(UiValue::Integer(2)),
            Err(AsyncRuntimeError::Closed {
                reason: SubscriptionCloseReason::GenerationStale
            })
        ));
    }

    #[test]
    fn cancellation_wakes_a_blocked_lossless_producer() {
        let (success, error, generation) = callbacks();
        let mut subscriptions = SubscriptionRegistry::new();
        let (handle, emitter) = subscriptions.subscribe(
            SubscriptionRegistration::new(
                "app.stream.blocked",
                AsyncScope::App,
                generation,
                success,
                error,
                ValueSchema::integer(),
            )
            .with_capacity(1)
            .unwrap(),
        );
        emitter.emit(UiValue::Integer(1)).unwrap();
        let pending = Arc::clone(&emitter.pending);
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let producer = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            done_tx
                .send(emitter.emit_blocking(UiValue::Integer(2)))
                .unwrap();
        });
        started_rx.recv().unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while pending.blocked_producers.load(Ordering::Acquire) == 0 {
            assert!(
                Instant::now() < deadline,
                "producer did not reach the capacity wait"
            );
            std::thread::yield_now();
        }

        assert!(subscriptions.cancel(handle));
        let result = done_rx.recv_timeout(Duration::from_secs(1));
        if result.is_err() {
            // Keep a failing regression from stranding the test process.
            pending.space.notify_all();
        }
        let result = result.expect("cancellation must wake the blocked producer");
        assert!(matches!(
            result,
            Err(AsyncRuntimeError::Closed {
                reason: SubscriptionCloseReason::Cancelled
            })
        ));
        producer.join().unwrap();
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
        emitter.emit(UiValue::Integer(1)).unwrap();
        emitter.close();
        assert_eq!(
            subscriptions.drain_up_to(generation, 1)[0].payload,
            UiValue::Integer(1)
        );
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
