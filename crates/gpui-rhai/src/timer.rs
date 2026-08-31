use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use thiserror::Error;

use crate::{
    AsyncDelivery, AsyncScope, ComponentInstancePath, ScriptCallback, ScriptGeneration, UiValue,
};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TimerId {
    component: ComponentInstancePath,
    key: String,
}

impl TimerId {
    /// Create a component-scoped declarative timer identity.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::InvalidKey`] for an empty, oversized, or unsafe key.
    pub fn new(
        component: ComponentInstancePath,
        key: impl Into<String>,
    ) -> Result<Self, TimerError> {
        let key = key.into();
        if !(1..=128).contains(&key.len())
            || !key.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | ':' | '.')
            })
        {
            return Err(TimerError::InvalidKey(key));
        }
        Ok(Self { component, key })
    }

    #[must_use]
    pub const fn component(&self) -> &ComponentInstancePath {
        &self.component
    }

    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimerDescriptor {
    id: TimerId,
    delay: Duration,
    paused: bool,
    callback: ScriptCallback,
    payload: UiValue,
}

impl TimerDescriptor {
    /// Create a bounded one-shot timer declaration.
    ///
    /// # Errors
    ///
    /// Returns [`TimerError::InvalidDelay`] for zero or more than one day.
    pub fn new(
        id: TimerId,
        delay: Duration,
        paused: bool,
        callback: ScriptCallback,
        payload: UiValue,
    ) -> Result<Self, TimerError> {
        if delay.is_zero() || delay > Duration::from_hours(24) {
            return Err(TimerError::InvalidDelay(delay));
        }
        Ok(Self {
            id,
            delay,
            paused,
            callback,
            payload,
        })
    }

    #[must_use]
    pub const fn id(&self) -> &TimerId {
        &self.id
    }

    #[must_use]
    pub const fn delay(&self) -> Duration {
        self.delay
    }

    #[must_use]
    pub const fn paused(&self) -> bool {
        self.paused
    }
}

#[derive(Clone, Debug, PartialEq)]
struct TimerSignature {
    delay: Duration,
    callback: String,
    payload: UiValue,
}

impl From<&TimerDescriptor> for TimerSignature {
    fn from(descriptor: &TimerDescriptor) -> Self {
        Self {
            delay: descriptor.delay,
            callback: descriptor.callback.name().to_owned(),
            payload: descriptor.payload.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct TimerEntry {
    descriptor: TimerDescriptor,
    signature: TimerSignature,
    deadline: Instant,
    remaining: Option<Duration>,
    declaration_paused: bool,
    interaction_paused: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimerSnapshot {
    pub id: TimerId,
    pub delay: Duration,
    pub remaining: Duration,
    pub declaration_paused: bool,
    pub interaction_paused: bool,
    pub callback: String,
    pub generation: ScriptGeneration,
}

impl TimerEntry {
    fn new(descriptor: TimerDescriptor, now: Instant) -> Self {
        let signature = TimerSignature::from(&descriptor);
        let declaration_paused = descriptor.paused;
        let remaining = descriptor.paused.then_some(descriptor.delay);
        Self {
            deadline: now + descriptor.delay,
            descriptor,
            signature,
            remaining,
            declaration_paused,
            interaction_paused: false,
        }
    }

    fn synchronize(&mut self, descriptor: TimerDescriptor, now: Instant) {
        let was_paused = self.is_paused();
        self.declaration_paused = descriptor.paused;
        self.transition_pause(was_paused, self.is_paused(), now);
        self.descriptor = descriptor;
    }

    fn is_paused(&self) -> bool {
        self.declaration_paused || self.interaction_paused
    }

    fn transition_pause(&mut self, was_paused: bool, paused: bool, now: Instant) {
        if !was_paused && paused {
            self.remaining = Some(self.deadline.saturating_duration_since(now));
        } else if was_paused && !paused {
            self.deadline = now + self.remaining.take().unwrap_or_default();
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct TimerRegistry {
    entries: BTreeMap<TimerId, TimerEntry>,
    completed: BTreeMap<TimerId, TimerSignature>,
}

impl TimerRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reconcile one component subtree's timer declarations atomically.
    pub fn reconcile(
        &mut self,
        root: &ComponentInstancePath,
        declarations: BTreeMap<TimerId, TimerDescriptor>,
        now: Instant,
    ) {
        self.entries
            .retain(|id, _| !id.component.is_within(root) || declarations.contains_key(id));
        self.completed
            .retain(|id, _| !id.component.is_within(root) || declarations.contains_key(id));
        for (id, descriptor) in declarations {
            let signature = TimerSignature::from(&descriptor);
            if self.completed.get(&id) == Some(&signature) {
                continue;
            }
            self.completed.remove(&id);
            match self.entries.get_mut(&id) {
                Some(entry) if entry.signature == signature => entry.synchronize(descriptor, now),
                Some(entry) => *entry = TimerEntry::new(descriptor, now),
                None => {
                    self.entries.insert(id, TimerEntry::new(descriptor, now));
                }
            }
        }
    }

    #[must_use]
    pub fn pause(&mut self, id: &TimerId, now: Instant) -> bool {
        let Some(entry) = self.entries.get_mut(id) else {
            return false;
        };
        let was_paused = entry.is_paused();
        entry.interaction_paused = true;
        entry.transition_pause(was_paused, entry.is_paused(), now);
        true
    }

    #[must_use]
    pub fn resume(&mut self, id: &TimerId, now: Instant) -> bool {
        let Some(entry) = self.entries.get_mut(id) else {
            return false;
        };
        let was_paused = entry.is_paused();
        entry.interaction_paused = false;
        entry.transition_pause(was_paused, entry.is_paused(), now);
        true
    }

    #[must_use]
    pub fn cancel(&mut self, id: &TimerId) -> bool {
        let Some(entry) = self.entries.remove(id) else {
            return false;
        };
        self.completed.insert(id.clone(), entry.signature);
        true
    }

    #[must_use]
    pub fn drain(&mut self, now: Instant, generation: ScriptGeneration) -> Vec<AsyncDelivery> {
        let due = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.remaining.is_none() && entry.deadline <= now)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        due.into_iter()
            .filter_map(|id| {
                let entry = self.entries.remove(&id)?;
                self.completed.insert(id, entry.signature);
                (entry.descriptor.callback.generation() == generation).then(|| AsyncDelivery {
                    callback: entry.descriptor.callback,
                    payload: entry.descriptor.payload,
                    scope: AsyncScope::Component(entry.descriptor.id.component),
                })
            })
            .collect()
    }

    pub fn cancel_component_scope(&mut self, component: &ComponentInstancePath) {
        self.entries
            .retain(|id, _| !id.component.is_within(component));
        self.completed
            .retain(|id, _| !id.component.is_within(component));
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn inspect(&self, now: Instant) -> Vec<TimerSnapshot> {
        self.entries
            .iter()
            .map(|(id, entry)| TimerSnapshot {
                id: id.clone(),
                delay: entry.descriptor.delay,
                remaining: entry
                    .remaining
                    .unwrap_or_else(|| entry.deadline.saturating_duration_since(now)),
                declaration_paused: entry.declaration_paused,
                interaction_paused: entry.interaction_paused,
                callback: entry.descriptor.callback.name().to_owned(),
                generation: entry.descriptor.callback.generation(),
            })
            .collect()
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum TimerError {
    #[error("timer key `{0}` must be 1-128 safe identifier characters")]
    InvalidKey(String),
    #[error("timer delay must be between 1ms and 24h, got {0:?}")]
    InvalidDelay(Duration),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventSchema, ScriptGeneration};
    use rhai::FnPtr;

    fn descriptor(
        component: &ComponentInstancePath,
        key: &str,
        delay: Duration,
        paused: bool,
    ) -> TimerDescriptor {
        let mut callback = ScriptCallback::try_from_fn_ptr(
            FnPtr::new("fired").unwrap(),
            ScriptGeneration::initial(),
        )
        .unwrap();
        callback.bind_component_if_unset(component.clone(), BTreeMap::<String, EventSchema>::new());
        TimerDescriptor::new(
            TimerId::new(component.clone(), key).unwrap(),
            delay,
            paused,
            callback,
            UiValue::String(key.to_owned()),
        )
        .unwrap()
    }

    #[test]
    fn reconcile_preserves_deadline_pause_and_one_shot_completion() {
        let root = ComponentInstancePath::root("App", "root");
        let now = Instant::now();
        let mut timers = TimerRegistry::new();
        let running = descriptor(&root, "toast:a", Duration::from_millis(100), false);
        timers.reconcile(
            &root,
            BTreeMap::from([(running.id.clone(), running.clone())]),
            now,
        );
        assert!(timers.pause(running.id(), now + Duration::from_millis(40)));
        let snapshot = timers.inspect(now + Duration::from_millis(90));
        assert_eq!(snapshot[0].remaining, Duration::from_millis(60));
        assert!(snapshot[0].interaction_paused);
        assert!(timers.resume(running.id(), now + Duration::from_millis(90)));
        assert!(
            timers
                .drain(
                    now + Duration::from_millis(149),
                    ScriptGeneration::initial(),
                )
                .is_empty()
        );
        assert_eq!(
            timers
                .drain(
                    now + Duration::from_millis(150),
                    ScriptGeneration::initial(),
                )
                .len(),
            1
        );
        timers.reconcile(&root, BTreeMap::from([(running.id.clone(), running)]), now);
        assert_eq!(
            timers.active_count(),
            0,
            "completed declaration stays one-shot"
        );
    }

    #[test]
    fn changing_signature_restarts_and_removal_cleans_scope() {
        let root = ComponentInstancePath::root("App", "root");
        let now = Instant::now();
        let mut timers = TimerRegistry::new();
        let first = descriptor(&root, "one", Duration::from_millis(10), false);
        timers.reconcile(&root, BTreeMap::from([(first.id.clone(), first)]), now);
        assert_eq!(timers.active_count(), 1);
        let changed = descriptor(&root, "one", Duration::from_millis(20), false);
        timers.reconcile(&root, BTreeMap::from([(changed.id.clone(), changed)]), now);
        assert!(
            timers
                .drain(now + Duration::from_millis(19), ScriptGeneration::initial(),)
                .is_empty()
        );
        timers.reconcile(&root, BTreeMap::new(), now);
        assert_eq!(timers.active_count(), 0);
    }
}
