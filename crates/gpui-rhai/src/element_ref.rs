use std::collections::BTreeMap;

use rhai::{CustomType, ImmutableString, TypeBuilder};
use thiserror::Error;

use crate::{ComponentInstancePath, NodeId};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ElementRefId {
    component: ComponentInstancePath,
    key: String,
}

impl ElementRefId {
    /// Construct a stable component-local reference identity.
    ///
    /// # Errors
    ///
    /// Returns [`ElementRefError::InvalidKey`] for unsafe keys.
    pub fn new(
        component: ComponentInstancePath,
        key: impl Into<String>,
    ) -> Result<Self, ElementRefError> {
        let key = key.into();
        if key.is_empty()
            || key.len() > 128
            || !key.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':')
            })
        {
            return Err(ElementRefError::InvalidKey(key));
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

/// Non-owning reference to a retained node in one formal component scope.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ElementRef {
    id: ElementRefId,
}

impl ElementRef {
    #[must_use]
    pub const fn new(id: ElementRefId) -> Self {
        Self { id }
    }

    #[must_use]
    pub const fn id(&self) -> &ElementRefId {
        &self.id
    }
}

impl CustomType for ElementRef {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("ElementRef")
            .with_get("key", |reference: &mut Self| {
                ImmutableString::from(reference.id.key.as_str())
            });
    }
}

#[derive(Clone, Debug, Default)]
pub struct ElementRefRegistry {
    active: BTreeMap<ElementRefId, NodeId>,
}

impl ElementRefRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.active.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&ElementRefId, &NodeId)> {
        self.active.iter()
    }

    /// Resolve the last committed retained identity.
    ///
    /// # Errors
    ///
    /// Returns [`ElementRefError::Stale`] after unmount or before commit.
    pub fn resolve(&self, reference: &ElementRef) -> Result<NodeId, ElementRefError> {
        self.active
            .get(reference.id())
            .copied()
            .ok_or_else(|| ElementRefError::Stale(reference.id().clone()))
    }

    /// Resolve a mounted component-local ref key.
    ///
    /// # Errors
    ///
    /// Returns [`ElementRefError::UnknownKey`] when no binding is committed.
    pub fn resolve_key(
        &self,
        component: &ComponentInstancePath,
        key: &str,
    ) -> Result<ElementRef, ElementRefError> {
        self.active
            .keys()
            .find(|id| id.component() == component && id.key() == key)
            .cloned()
            .map(ElementRef::new)
            .ok_or_else(|| ElementRefError::UnknownKey {
                component: component.clone(),
                key: key.to_owned(),
            })
    }

    pub(crate) fn reconcile(
        &mut self,
        root: &ComponentInstancePath,
        candidate: BTreeMap<ElementRefId, NodeId>,
    ) {
        self.active.retain(|id, _| !id.component.is_within(root));
        self.active.extend(candidate);
    }

    pub(crate) fn remove_scope(&mut self, root: &ComponentInstancePath) {
        self.active.retain(|id, _| !id.component.is_within(root));
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ElementRefError {
    #[error("element-ref key `{0}` must be 1-128 ASCII letters, digits, `_`, `-`, `.`, or `:`")]
    InvalidKey(String),
    #[error("element ref `{0:?}` is stale or unmounted")]
    Stale(ElementRefId),
    #[error("element ref `{0:?}` was not declared by the active formal component render")]
    Undeclared(ElementRefId),
    #[error("element ref `{0:?}` is bound to more than one node")]
    DuplicateBinding(ElementRefId),
    #[error("element ref `{0:?}` requires a stable node key")]
    MissingNodeKey(ElementRefId),
    #[error("component `{component}` has no mounted element ref `{key}`")]
    UnknownKey {
        component: ComponentInstancePath,
        key: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ElementCommand {
    Focus {
        window: String,
        node: NodeId,
    },
    ScrollTo {
        window: String,
        node: NodeId,
        x: f64,
        y: f64,
    },
    ScrollIntoView {
        window: String,
        node: NodeId,
    },
}

impl ElementCommand {
    pub(crate) fn window(&self) -> &str {
        match self {
            Self::Focus { window, .. }
            | Self::ScrollTo { window, .. }
            | Self::ScrollIntoView { window, .. } => window,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_refs_do_not_rebind_after_scope_removal() {
        let component = ComponentInstancePath::root("Panel", "main");
        let reference = ElementRef::new(ElementRefId::new(component.clone(), "field").unwrap());
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(crate::UiNode::text("field")).unwrap();
        let mut registry = ElementRefRegistry::new();
        registry.reconcile(
            &component,
            BTreeMap::from([(reference.id().clone(), tree.root_id().unwrap())]),
        );
        assert!(registry.resolve(&reference).is_ok());
        registry.remove_scope(&component);
        assert!(matches!(
            registry.resolve(&reference),
            Err(ElementRefError::Stale(_))
        ));
    }
}
