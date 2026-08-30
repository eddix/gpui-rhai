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
    active: BTreeMap<EffectId, EffectDescriptor>,
}

impl EffectRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&EffectId, &EffectDescriptor)> {
        self.active.iter()
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
        next.extend(candidate.clone());

        let mut cleanup = self
            .active
            .iter()
            .filter(|(id, active)| {
                id.component.is_within(root) && candidate.get(*id) != Some(*active)
            })
            .map(|(_, descriptor)| descriptor.clone())
            .collect::<Vec<_>>();
        cleanup.sort_by(|left, right| right.id.cmp(&left.id));

        let mut start = candidate
            .into_iter()
            .filter(|(id, descriptor)| self.active.get(id) != Some(descriptor))
            .map(|(_, descriptor)| descriptor)
            .collect::<Vec<_>>();
        start.sort_by(|left, right| left.id.cmp(&right.id));

        EffectPlan {
            cleanup,
            start,
            next,
        }
    }

    pub(crate) fn commit(&mut self, plan: EffectPlan) {
        self.active = plan.next;
    }

    pub(crate) fn remove_scope(&mut self, root: &ComponentInstancePath) {
        self.active.retain(|id, _| !id.component.is_within(root));
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EffectPlan {
    cleanup: Vec<EffectDescriptor>,
    start: Vec<EffectDescriptor>,
    next: BTreeMap<EffectId, EffectDescriptor>,
}

impl EffectPlan {
    pub(crate) fn cleanup(&self) -> &[EffectDescriptor] {
        &self.cleanup
    }

    pub(crate) fn start(&self) -> &[EffectDescriptor] {
        &self.start
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
        assert!(initial.cleanup().is_empty());
        assert_eq!(initial.start(), std::slice::from_ref(&active));
        registry.commit(initial);

        let unchanged = registry.plan(
            &root,
            BTreeMap::from([(active.id().clone(), active.clone())]),
        );
        assert!(unchanged.cleanup().is_empty());
        assert!(unchanged.start().is_empty());

        let removed = registry.plan(&root, BTreeMap::new());
        assert_eq!(removed.cleanup(), std::slice::from_ref(&active));
        assert!(removed.start().is_empty());
    }
}
