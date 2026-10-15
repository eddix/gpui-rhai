use std::collections::BTreeMap;

use thiserror::Error;

use crate::{ComponentInstancePath, ScriptCallback, ScriptGeneration, UiValue};

/// The existing semantic effect-transition budget also bounds a post-commit
/// cascade; yielding to the foreground pump must not reset its allowance.
pub(crate) const EFFECT_TRANSITION_CALLBACK_BUDGET: usize = 64;

#[derive(Clone, Debug, Default)]
pub(crate) struct EffectCascadeCheckpoint {
    callbacks: BTreeMap<ComponentInstancePath, usize>,
    accepted_rejections: BTreeMap<ComponentInstancePath, u64>,
    generations: BTreeMap<ComponentInstancePath, ScriptGeneration>,
    quiescent: std::collections::BTreeSet<ComponentInstancePath>,
    unowned_dispatch: std::collections::BTreeSet<ComponentInstancePath>,
}

/// Foreground delivery bookkeeping, not a second scheduler. Rejection epochs
/// deliberately survive rollback: an already queued wake cannot restart a
/// rejected graph. Accepting a new external turn is itself transactional.
#[derive(Debug, Default)]
pub(crate) struct EffectCascadeBudget {
    checkpoint: EffectCascadeCheckpoint,
    rejections: BTreeMap<ComponentInstancePath, BTreeMap<ScriptGeneration, u64>>,
    dispatches:
        BTreeMap<ComponentInstancePath, Vec<std::rc::Weak<crate::context::SemanticDispatchSource>>>,
    live_dispatches: std::collections::BTreeSet<ComponentInstancePath>,
}

impl EffectCascadeBudget {
    pub(crate) fn checkpoint(&self) -> EffectCascadeCheckpoint {
        self.checkpoint.clone()
    }

    pub(crate) fn restore(&mut self, checkpoint: EffectCascadeCheckpoint) {
        self.checkpoint = checkpoint;
    }

    pub(crate) fn restart(&mut self, root: &ComponentInstancePath) {
        self.checkpoint.callbacks.remove(root);
        self.checkpoint.quiescent.remove(root);
        self.checkpoint.unowned_dispatch.remove(root);
        let epoch = self.rejection_epoch(root);
        self.checkpoint
            .accepted_rejections
            .insert(root.clone(), epoch);
    }

    pub(crate) fn restart_containing(&mut self, component: &ComponentInstancePath) {
        let roots = self
            .checkpoint
            .accepted_rejections
            .keys()
            .filter(|root| component.is_within(root) || root.is_within(component))
            .cloned()
            .collect::<Vec<_>>();
        for root in roots {
            self.restart(&root);
        }
    }

    pub(crate) fn blocked(&self, root: &ComponentInstancePath) -> bool {
        self.rejection_epoch(root)
            > self
                .checkpoint
                .accepted_rejections
                .get(root)
                .copied()
                .unwrap_or_default()
    }

    pub(crate) fn blocked_component(&self, component: &ComponentInstancePath) -> bool {
        self.rejections
            .keys()
            .any(|root| component.is_within(root) && self.blocked(root))
    }

    pub(crate) fn dispatch_token(
        &self,
        component: &ComponentInstancePath,
    ) -> Option<(ComponentInstancePath, u64)> {
        self.checkpoint
            .accepted_rejections
            .keys()
            .filter(|root| component.is_within(root))
            .max()
            .map(|root| (root.clone(), self.rejection_epoch(root)))
    }

    pub(crate) fn accepts_dispatch(&self, root: &ComponentInstancePath, epoch: u64) -> bool {
        !self.blocked(root) && self.rejection_epoch(root) == epoch
    }

    pub(crate) fn register_generation(
        &mut self,
        root: &ComponentInstancePath,
        generation: ScriptGeneration,
    ) {
        if self.checkpoint.generations.insert(root.clone(), generation) != Some(generation) {
            self.restart(root);
        }
    }

    fn rejection_epoch(&self, root: &ComponentInstancePath) -> u64 {
        let generation = self
            .checkpoint
            .generations
            .get(root)
            .copied()
            .unwrap_or_default();
        self.rejections
            .get(root)
            .and_then(|epochs| epochs.get(&generation))
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn track_dispatch(
        &mut self,
        root: &ComponentInstancePath,
        owner: &std::rc::Rc<crate::context::SemanticDispatchSource>,
    ) {
        self.dispatches
            .entry(root.clone())
            .or_default()
            .push(std::rc::Rc::downgrade(owner));
    }

    pub(crate) fn prune_dispatches(
        &mut self,
        incarnations: &BTreeMap<ComponentInstancePath, crate::ComponentIncarnation>,
        effects: &EffectRegistry,
    ) {
        let mut live = std::collections::BTreeSet::new();
        self.dispatches = std::mem::take(&mut self.dispatches)
            .into_iter()
            .filter_map(|(root, mut owners)| {
                owners.retain(|owner| {
                    owner.upgrade().is_some_and(|owner| {
                        if !owner.publication_valid() {
                            return false;
                        }
                        if owner.active(incarnations, effects, self) {
                            live.insert(root.clone());
                        }
                        // Candidate generation/activation changes are transactional;
                        // keep weak identity until rollback can restore its source.
                        true
                    })
                });
                (!owners.is_empty()).then_some((root, owners))
            })
            .collect();
        self.live_dispatches = live;
    }

    pub(crate) fn retain_unowned_dispatch(&mut self, root: &ComponentInstancePath) {
        self.checkpoint.unowned_dispatch.insert(root.clone());
    }

    pub(crate) fn generation_for(
        &self,
        component: &ComponentInstancePath,
    ) -> Option<ScriptGeneration> {
        self.checkpoint
            .generations
            .iter()
            .filter(|(root, _)| component.is_within(root))
            .max_by_key(|(root, _)| *root)
            .map(|(_, generation)| *generation)
    }

    pub(crate) fn charge(
        &mut self,
        root: &ComponentInstancePath,
        count: usize,
    ) -> Result<(), usize> {
        if self.checkpoint.quiescent.contains(root)
            && !self.live_dispatches.contains(root)
            && !self.checkpoint.unowned_dispatch.contains(root)
        {
            self.checkpoint.callbacks.remove(root);
            self.checkpoint.quiescent.remove(root);
        }
        self.checkpoint
            .accepted_rejections
            .entry(root.clone())
            .or_default();
        let total = self
            .checkpoint
            .callbacks
            .get(root)
            .copied()
            .unwrap_or_default()
            .saturating_add(count);
        if total > EFFECT_TRANSITION_CALLBACK_BUDGET {
            self.reject(root);
            return Err(total);
        }
        self.checkpoint.callbacks.insert(root.clone(), total);
        Ok(())
    }

    pub(crate) fn reject(&mut self, root: &ComponentInstancePath) {
        let generation = self
            .checkpoint
            .generations
            .get(root)
            .copied()
            .unwrap_or_default();
        let epoch = self
            .rejections
            .entry(root.clone())
            .or_default()
            .entry(generation)
            .or_default();
        *epoch = epoch.saturating_add(1);
    }

    pub(crate) fn settle<'a>(
        &mut self,
        dirty: &std::collections::BTreeSet<ComponentInstancePath>,
        pending_effect_scopes: impl IntoIterator<Item = &'a ComponentInstancePath>,
    ) {
        let pending = pending_effect_scopes.into_iter().collect::<Vec<_>>();
        let quiescent = &mut self.checkpoint.quiescent;
        let unowned = &self.checkpoint.unowned_dispatch;
        let live = &self.live_dispatches;
        self.checkpoint.callbacks.retain(|root, _| {
            let unsettled = dirty.iter().any(|path| path.is_within(root))
                || pending.iter().any(|path| path.is_within(root));
            if unsettled {
                quiescent.remove(root);
            } else {
                quiescent.insert(root.clone());
            }
            unsettled || live.contains(root) || unowned.contains(root)
        });
    }

    pub(crate) fn remove_scope(&mut self, root: &ComponentInstancePath) {
        self.checkpoint
            .callbacks
            .retain(|path, _| !path.is_within(root));
        self.checkpoint
            .accepted_rejections
            .retain(|path, _| !path.is_within(root));
        self.checkpoint
            .generations
            .retain(|path, _| !path.is_within(root));
        self.checkpoint
            .quiescent
            .retain(|path| !path.is_within(root));
        self.checkpoint
            .unowned_dispatch
            .retain(|path| !path.is_within(root));
        self.dispatches.retain(|path, _| !path.is_within(root));
        self.live_dispatches.retain(|path| !path.is_within(root));
        self.rejections.retain(|path, _| !path.is_within(root));
    }
}

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

    pub(crate) fn contains_scope(&self, scope: &crate::AsyncScope) -> bool {
        let crate::AsyncScope::Effect {
            component,
            key,
            activation,
        } = scope
        else {
            return true;
        };
        self.active
            .get(&EffectId {
                component: component.clone(),
                key: key.clone(),
            })
            .is_some_and(|active| active.activation == *activation)
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

    #[test]
    fn cascade_budget_survives_yields_and_rejection_survives_rollback() {
        let root = ComponentInstancePath::root("View", "main");
        let sibling = ComponentInstancePath::root("View", "other");
        let mut budget = EffectCascadeBudget::default();
        budget.restart(&root);
        budget
            .charge(&root, EFFECT_TRANSITION_CALLBACK_BUDGET)
            .unwrap();
        let last_good = budget.checkpoint();
        budget.settle(
            &std::collections::BTreeSet::from([root.child("Child", "a")]),
            [],
        );
        assert_eq!(budget.charge(&root, 1), Err(65));
        budget.restore(last_good.clone());
        assert!(
            budget.blocked(&root),
            "rollback must not retry a rejected cascade"
        );
        assert!(budget.blocked_component(&root.child("Child", "a")));
        assert!(!budget.blocked_component(&sibling));
        budget.charge(&sibling, 1).unwrap();

        budget.restart(&root);
        assert!(!budget.blocked(&root));
        budget.charge(&root, 1).unwrap();
        budget.restore(last_good);
        assert!(
            budget.blocked(&root),
            "failed recovery must stay quarantined"
        );
        budget.restart(&root);
        budget.charge(&root, 1).unwrap();
        budget.settle(&std::collections::BTreeSet::new(), []);
        budget
            .charge(&root, EFFECT_TRANSITION_CALLBACK_BUDGET)
            .unwrap();
        budget.remove_scope(&root);
        assert!(!budget.blocked(&root));
    }

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
        registry.commit(restart);
        assert!(!registry.contains_scope(&old_scope));
        assert!(registry.contains_scope(&start_scope));
    }
}
