use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use thiserror::Error;

use crate::{ComponentInstancePath, NodeId};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeometryBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl GeometryBounds {
    /// Construct validated logical geometry.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::InvalidBounds`] for non-finite values or
    /// negative sizes.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Result<Self, GeometryError> {
        if [x, y, width, height].into_iter().all(f64::is_finite) && width >= 0.0 && height >= 0.0 {
            Ok(Self {
                x,
                y,
                width,
                height,
            })
        } else {
            Err(GeometryError::InvalidBounds {
                x,
                y,
                width,
                height,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ElementGeometry {
    pub layout: GeometryBounds,
    pub visual: GeometryBounds,
    pub clip: Option<GeometryBounds>,
}

#[derive(Clone, Debug, Default)]
struct GeometryState {
    committed: BTreeMap<NodeId, ElementGeometry>,
    readers: BTreeMap<NodeId, BTreeSet<ComponentInstancePath>>,
    dirty: BTreeSet<ComponentInstancePath>,
}

#[derive(Clone, Debug, Default)]
pub struct GeometryRegistry {
    inner: Rc<RefCell<GeometryState>>,
}

impl GeometryRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&self, node: NodeId, geometry: ElementGeometry) -> bool {
        let mut state = self.inner.borrow_mut();
        if state.committed.get(&node) == Some(&geometry) {
            return false;
        }
        state.committed.insert(node, geometry);
        let readers = state.readers.get(&node).cloned().unwrap_or_default();
        state.dirty.extend(readers);
        true
    }

    /// Read committed geometry and register one exact component dependency.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::Unavailable`] before first prepaint or after
    /// unmount.
    pub fn read(
        &self,
        node: NodeId,
        reader: &ComponentInstancePath,
    ) -> Result<ElementGeometry, GeometryError> {
        let mut state = self.inner.borrow_mut();
        let geometry = state
            .committed
            .get(&node)
            .copied()
            .ok_or(GeometryError::Unavailable(node))?;
        state
            .readers
            .entry(node)
            .or_default()
            .insert(reader.clone());
        Ok(geometry)
    }

    pub(crate) fn read_tracked(
        &self,
        node: NodeId,
        reader: &ComponentInstancePath,
    ) -> Option<ElementGeometry> {
        let mut state = self.inner.borrow_mut();
        state
            .readers
            .entry(node)
            .or_default()
            .insert(reader.clone());
        state.committed.get(&node).copied()
    }

    pub(crate) fn get(&self, node: NodeId) -> Option<ElementGeometry> {
        self.inner.borrow().committed.get(&node).copied()
    }

    pub(crate) fn retain_nodes(&self, active: &BTreeSet<NodeId>) {
        let mut state = self.inner.borrow_mut();
        state.committed.retain(|node, _| active.contains(node));
        state.readers.retain(|node, _| active.contains(node));
    }

    pub(crate) fn take_dirty(&self) -> BTreeSet<ComponentInstancePath> {
        std::mem::take(&mut self.inner.borrow_mut().dirty)
    }

    pub(crate) fn snapshot(&self) -> GeometrySnapshot {
        GeometrySnapshot(self.inner.borrow().clone())
    }

    pub(crate) fn restore(&self, snapshot: GeometrySnapshot) {
        *self.inner.borrow_mut() = snapshot.0;
    }
}

#[derive(Clone, Debug)]
pub(crate) struct GeometrySnapshot(GeometryState);

#[derive(Clone, Debug, Error, PartialEq)]
pub enum GeometryError {
    #[error("invalid geometry x={x}, y={y}, width={width}, height={height}")]
    InvalidBounds {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    #[error("geometry for retained node {0} is not committed")]
    Unavailable(NodeId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_geometry_invalidates_exact_readers_and_snapshot_restores() {
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(crate::UiNode::text("field")).unwrap();
        let node = tree.root_id().unwrap();
        let reader = ComponentInstancePath::root("Panel", "main");
        let registry = GeometryRegistry::new();
        let initial = ElementGeometry {
            layout: GeometryBounds::new(0.0, 0.0, 100.0, 20.0).unwrap(),
            visual: GeometryBounds::new(0.0, 0.0, 100.0, 20.0).unwrap(),
            clip: None,
        };
        registry.update(node, initial);
        assert_eq!(registry.read(node, &reader).unwrap(), initial);
        let snapshot = registry.snapshot();
        registry.update(
            node,
            ElementGeometry {
                layout: GeometryBounds::new(0.0, 0.0, 120.0, 20.0).unwrap(),
                visual: GeometryBounds::new(0.0, 0.0, 120.0, 20.0).unwrap(),
                clip: None,
            },
        );
        assert!(registry.take_dirty().contains(&reader));
        registry.restore(snapshot);
        assert_eq!(registry.read(node, &reader).unwrap(), initial);
    }
}
