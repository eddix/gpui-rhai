use std::collections::BTreeMap;

use thiserror::Error;

use crate::{ComponentInstancePath, ScriptCallback, ScriptGeneration, UiValue};

/// Stable identity of one declarative component effect.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EffectId {
    component: ComponentInstancePath,
    key: String,
}

impl EffectId {
    /// Construct a component-scoped effect identity.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidKey`] for an unsafe or unstable key.
    pub fn new(
        component: ComponentInstancePath,
        key: impl Into<String>,
    ) -> Result<Self, EffectError> {
        let key = key.into();
        if key.is_empty()
            || key.len() > 128
            || !key.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':')
            })
        {
            return Err(EffectError::InvalidKey(key));
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

/// One pure-render effect declaration retained until its dependencies change.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectDescriptor {
    id: EffectId,
    dependencies: UiValue,
    start: ScriptCallback,
    cleanup: ScriptCallback,
}

impl EffectDescriptor {
    #[must_use]
    pub fn new(
        id: EffectId,
        dependencies: UiValue,
        start: ScriptCallback,
        cleanup: ScriptCallback,
    ) -> Self {
        Self {
            id,
            dependencies,
            start,
            cleanup,
        }
    }

    #[must_use]
    pub const fn id(&self) -> &EffectId {
        &self.id
    }

    #[must_use]
    pub const fn dependencies(&self) -> &UiValue {
        &self.dependencies
    }

    #[must_use]
    pub const fn start(&self) -> &ScriptCallback {
        &self.start
    }

    #[must_use]
    pub const fn cleanup(&self) -> &ScriptCallback {
        &self.cleanup
    }

    #[must_use]
    pub fn generation(&self) -> ScriptGeneration {
        self.start.generation()
    }
}

#[derive(Clone, Debug, Default)]
pub struct EffectRegistry {
    active: BTreeMap<EffectId, ActiveEffect>,
    next_activation: u64,
}

#[derive(Clone, Debug)]
struct ActiveEffect {
    descriptor: EffectDescriptor,
    activation: u64,
}

impl ActiveEffect {
    fn scope(&self) -> crate::AsyncScope {
        crate::AsyncScope::Effect {
            component: self.descriptor.id.component.clone(),
            key: self.descriptor.id.key.clone(),
            activation: self.activation,
        }
    }
}

impl EffectRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&EffectId, &EffectDescriptor)> {
        self.active
            .iter()
            .map(|(id, active)| (id, &active.descriptor))
    }

    pub(crate) fn iter_active(
        &self,
    ) -> impl ExactSizeIterator<Item = (&EffectId, &EffectDescriptor, u64)> {
        self.active
            .iter()
            .map(|(id, active)| (id, &active.descriptor, active.activation))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.active.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    pub(crate) fn plan(
        &self,
        root: &ComponentInstancePath,
        candidate: BTreeMap<EffectId, EffectDescriptor>,
    ) -> EffectPlan {
        let mut next = self.active.clone();
        next.retain(|id, _| !id.component.is_within(root));

        let mut cleanup = self
            .active
            .iter()
            .filter(|(id, active)| {
                id.component.is_within(root) && candidate.get(*id) != Some(&active.descriptor)
            })
            .map(|(_, active)| active.clone())
            .collect::<Vec<_>>();
        cleanup.sort_by(|left, right| right.descriptor.id.cmp(&left.descriptor.id));

        let mut activation = self.next_activation.max(1);
        let mut start = Vec::new();
        for (id, descriptor) in candidate {
            if let Some(active) = self
                .active
                .get(&id)
                .filter(|active| active.descriptor == descriptor)
            {
                next.insert(id, active.clone());
            } else {
                let active = ActiveEffect {
                    descriptor,
                    activation,
                };
                activation = activation.saturating_add(1);
                start.push(active.clone());
                next.insert(id, active);
            }
        }
        start.sort_by(|left, right| left.descriptor.id.cmp(&right.descriptor.id));

        EffectPlan {
            cleanup,
            start,
            next,
            next_activation: activation,
        }
    }

    pub(crate) fn commit(&mut self, plan: EffectPlan) {
        self.active = plan.next;
        self.next_activation = plan.next_activation;
    }

    pub(crate) fn remove_scope(&mut self, root: &ComponentInstancePath) {
        self.active.retain(|id, _| !id.component.is_within(root));
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EffectPlan {
    cleanup: Vec<ActiveEffect>,
    start: Vec<ActiveEffect>,
    next: BTreeMap<EffectId, ActiveEffect>,
    next_activation: u64,
}

impl EffectPlan {
    pub(crate) fn transition_count(&self) -> usize {
        self.cleanup.len().saturating_add(self.start.len())
    }

    pub(crate) fn has_cleanup(&self) -> bool {
        !self.cleanup.is_empty()
    }

    pub(crate) fn cleanup_descriptors(
        &self,
    ) -> impl ExactSizeIterator<Item = (&EffectDescriptor, crate::AsyncScope)> {
        self.cleanup
            .iter()
            .map(|active| (&active.descriptor, active.scope()))
    }

    pub(crate) fn start_descriptors(
        &self,
    ) -> impl ExactSizeIterator<Item = (&EffectDescriptor, crate::AsyncScope)> {
        self.start
            .iter()
            .map(|active| (&active.descriptor, active.scope()))
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum EffectError {
    #[error("effect key `{0}` must be 1-128 ASCII letters, digits, `_`, `-`, `.`, or `:`")]
    InvalidKey(String),
    #[error("effect `{key}` is not declared by component `{component}`")]
    Undeclared {
        component: ComponentInstancePath,
        key: String,
    },
    #[error("effect `{key}` is declared more than once by component `{component}`")]
    Duplicate {
        component: ComponentInstancePath,
        key: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeEngine;

    fn callback(name: &str) -> ScriptCallback {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(&format!(
                "fn {name}(ctx, dependencies) {{}} fn view(ctx) {{ text(\"ok\") }}"
            ))
            .unwrap();
        engine.callback(&compiled, name).unwrap()
    }

    fn descriptor(component: &ComponentInstancePath, key: &str, value: i64) -> EffectDescriptor {
        EffectDescriptor::new(
            EffectId::new(component.clone(), key).unwrap(),
            UiValue::Integer(value),
            callback("start"),
            callback("cleanup"),
        )
    }

    #[test]
    fn unchanged_effects_do_not_restart_and_removed_effects_cleanup() {
        let root = ComponentInstancePath::root("View", "main");
        let child = root.child("Panel", "primary");
        let active = descriptor(&child, "load", 1);
        let mut registry = EffectRegistry::new();
        let initial = registry.plan(
            &root,
            BTreeMap::from([(active.id().clone(), active.clone())]),
        );
        assert!(!initial.has_cleanup());
        assert_eq!(
            initial
                .start_descriptors()
                .map(|(descriptor, _)| descriptor)
                .collect::<Vec<_>>(),
            vec![&active]
        );
        registry.commit(initial);

        let unchanged = registry.plan(
            &root,
            BTreeMap::from([(active.id().clone(), active.clone())]),
        );
        assert_eq!(unchanged.transition_count(), 0);

        let removed = registry.plan(&root, BTreeMap::new());
        assert_eq!(
            removed
                .cleanup_descriptors()
                .map(|(descriptor, _)| descriptor)
                .collect::<Vec<_>>(),
            vec![&active]
        );
        assert_eq!(removed.start_descriptors().len(), 0);
    }

    #[test]
    fn restarted_effect_uses_a_distinct_async_activation_scope() {
        let root = ComponentInstancePath::root("View", "main");
        let old = descriptor(&root, "watch", 1);
        let mut registry = EffectRegistry::new();
        let initial = registry.plan(&root, BTreeMap::from([(old.id().clone(), old.clone())]));
        let old_scope = initial.start_descriptors().next().unwrap().1;
        registry.commit(initial);

        let changed = descriptor(&root, "watch", 2);
        let restart = registry.plan(&root, BTreeMap::from([(changed.id().clone(), changed)]));
        let cleanup_scope = restart.cleanup_descriptors().next().unwrap().1;
        let start_scope = restart.start_descriptors().next().unwrap().1;
        assert_eq!(cleanup_scope, old_scope);
        assert_ne!(start_scope, old_scope);
    }
}
