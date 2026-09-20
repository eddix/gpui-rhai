use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::time::{Duration, Instant};

use thiserror::Error;

use crate::{ComponentInstancePath, NodeId, UiValue};

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

    #[must_use]
    pub fn into_value(self) -> UiValue {
        UiValue::Map(BTreeMap::from([
            ("x".to_owned(), UiValue::Float(self.x)),
            ("y".to_owned(), UiValue::Float(self.y)),
            ("width".to_owned(), UiValue::Float(self.width)),
            ("height".to_owned(), UiValue::Float(self.height)),
        ]))
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
    presented: BTreeSet<NodeId>,
    readers: BTreeMap<NodeId, BTreeSet<ComponentInstancePath>>,
    dirty: BTreeSet<ComponentInstancePath>,
    layout_motion: BTreeMap<NodeId, LayoutMotionState>,
    shared_layout: BTreeMap<(String, String), (NodeId, GeometryBounds)>,
    motion_progress: BTreeMap<(NodeId, crate::MotionProperty), f64>,
    motion_trigger_targets: BTreeMap<(NodeId, crate::MotionProgressDriver), bool>,
    motion_triggers: BTreeMap<(NodeId, crate::MotionProgressDriver), TriggerMotionState>,
}

#[derive(Clone, Debug)]
struct LayoutMotionState {
    from: GeometryBounds,
    to: GeometryBounds,
    started: Instant,
    duration: Duration,
    easing: crate::MotionEasing,
}

#[derive(Clone, Debug)]
struct TriggerMotionState {
    from: f64,
    to: f64,
    started: Instant,
    duration: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LayoutMotionSample {
    pub offset_x: f64,
    pub offset_y: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub active: bool,
}

impl Default for LayoutMotionSample {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            active: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TriggerMotionSample {
    pub value: f64,
    pub active: bool,
}

#[derive(Clone, Debug, Default)]
pub struct GeometryRegistry {
    inner: Rc<RefCell<Rc<GeometryState>>>,
}

impl GeometryRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.borrow().committed.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.borrow().committed.is_empty()
    }

    pub fn update(&self, node: NodeId, geometry: ElementGeometry) -> bool {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        state.presented.insert(node);
        if state.committed.get(&node) == Some(&geometry) {
            return false;
        }
        state.committed.insert(node, geometry);
        let readers = state.readers.get(&node).cloned().unwrap_or_default();
        state.dirty.extend(readers);
        true
    }

    pub(crate) fn sample_layout_motion(
        &self,
        node: NodeId,
        layout: GeometryBounds,
        shared: Option<(&str, &str)>,
        duration: Duration,
        easing: crate::MotionEasing,
        now: Instant,
    ) -> LayoutMotionSample {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        let shared_previous = shared.and_then(|(group, id)| {
            state
                .shared_layout
                .get(&(group.to_owned(), id.to_owned()))
                .copied()
                .filter(|(previous, _)| *previous != node)
                .map(|(_, bounds)| bounds)
        });
        let previous = state
            .committed
            .get(&node)
            .map(|geometry| geometry.layout)
            .or(shared_previous);
        let target_changed = state
            .layout_motion
            .get(&node)
            .is_none_or(|motion| motion.to != layout);
        if let Some(previous) = previous
            && previous != layout
            && target_changed
            && duration > Duration::ZERO
        {
            let from = state
                .layout_motion
                .get(&node)
                .map_or(previous, |motion| sample_layout_bounds(motion, now));
            state.layout_motion.insert(
                node,
                LayoutMotionState {
                    from,
                    to: layout,
                    started: now,
                    duration,
                    easing,
                },
            );
        }
        if let Some((group, id)) = shared {
            state
                .shared_layout
                .insert((group.to_owned(), id.to_owned()), (node, layout));
            while state.shared_layout.len() > 1_024 {
                let Some(first) = state.shared_layout.keys().next().cloned() else {
                    break;
                };
                state.shared_layout.remove(&first);
            }
        }
        let Some(motion) = state.layout_motion.get(&node) else {
            return LayoutMotionSample::default();
        };
        let sampled = sample_layout_bounds(motion, now);
        let done = now.saturating_duration_since(motion.started) >= motion.duration;
        let result = LayoutMotionSample {
            offset_x: sampled.x - layout.x,
            offset_y: sampled.y - layout.y,
            // GPUI 0.2.2 exposes an element offset but no public arbitrary
            // subtree scale transform. Keep visual/hit geometry honest and
            // snap size while animating position.
            scale_x: 1.0,
            scale_y: 1.0,
            active: !done,
        };
        if done {
            state.layout_motion.remove(&node);
        }
        result
    }

    pub(crate) fn update_motion_progress(
        &self,
        node: NodeId,
        property: crate::MotionProperty,
        value: f64,
    ) -> bool {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        let key = (node, property);
        if state
            .motion_progress
            .get(&key)
            .is_some_and(|previous| (*previous - value).abs() <= f64::EPSILON)
        {
            false
        } else {
            state.motion_progress.insert(key, value);
            true
        }
    }

    pub(crate) fn motion_progress(
        &self,
        node: NodeId,
        property: crate::MotionProperty,
    ) -> Option<f64> {
        self.inner
            .borrow()
            .motion_progress
            .get(&(node, property))
            .copied()
    }

    pub(crate) fn set_motion_trigger(
        &self,
        node: NodeId,
        driver: crate::MotionProgressDriver,
        active: bool,
    ) {
        Rc::make_mut(&mut self.inner.borrow_mut())
            .motion_trigger_targets
            .insert((node, driver), active);
    }

    pub(crate) fn sample_motion_trigger(
        &self,
        node: NodeId,
        binding: &crate::MotionProgressBinding,
        now: Instant,
    ) -> TriggerMotionSample {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        let key = (node, binding.driver);
        let target = if state
            .motion_trigger_targets
            .get(&key)
            .copied()
            .unwrap_or(false)
        {
            1.0
        } else {
            0.0
        };
        let duration = crate::motion::progress_source_duration(&binding.source);
        let trigger = state
            .motion_triggers
            .entry(key)
            .or_insert(TriggerMotionState {
                from: 0.0,
                to: 0.0,
                started: now,
                duration,
            });
        let sampled = sample_trigger_progress(trigger, now);
        if (trigger.to - target).abs() > f64::EPSILON {
            *trigger = TriggerMotionState {
                from: sampled,
                to: target,
                started: now,
                duration,
            };
        } else {
            trigger.duration = duration;
        }
        let progress = sample_trigger_progress(trigger, now);
        TriggerMotionSample {
            value: crate::motion::sample_progress_source(&binding.source, progress),
            active: (progress - trigger.to).abs() > 0.000_1,
        }
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
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
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
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        state
            .readers
            .entry(node)
            .or_default()
            .insert(reader.clone());
        state.committed.get(&node).copied()
    }

    pub(crate) fn register_readers(&self, node: NodeId, readers: BTreeSet<ComponentInstancePath>) {
        if readers.is_empty() {
            return;
        }
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        let committed = state.committed.contains_key(&node);
        for reader in readers {
            if state
                .readers
                .entry(node)
                .or_default()
                .insert(reader.clone())
                && committed
            {
                state.dirty.insert(reader);
            }
        }
    }

    pub(crate) fn get(&self, node: NodeId) -> Option<ElementGeometry> {
        self.inner.borrow().committed.get(&node).copied()
    }

    pub(crate) fn begin_frame(&self) {
        Rc::make_mut(&mut self.inner.borrow_mut()).presented.clear();
    }

    pub(crate) fn is_presented(&self, node: NodeId) -> bool {
        self.inner.borrow().presented.contains(&node)
    }

    pub(crate) fn retain_nodes(&self, active: &BTreeSet<NodeId>) {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        state.committed.retain(|node, _| active.contains(node));
        state.presented.retain(|node| active.contains(node));
        state.readers.retain(|node, _| active.contains(node));
        state.layout_motion.retain(|node, _| active.contains(node));
        state
            .motion_progress
            .retain(|(node, _), _| active.contains(node));
        state
            .motion_trigger_targets
            .retain(|(node, _), _| active.contains(node));
        state
            .motion_triggers
            .retain(|(node, _), _| active.contains(node));
    }

    pub(crate) fn take_dirty(&self) -> BTreeSet<ComponentInstancePath> {
        std::mem::take(&mut Rc::make_mut(&mut self.inner.borrow_mut()).dirty)
    }

    pub(crate) fn snapshot(&self) -> GeometrySnapshot {
        GeometrySnapshot(Rc::clone(&self.inner.borrow()))
    }

    pub(crate) fn restore(&self, snapshot: GeometrySnapshot) {
        *self.inner.borrow_mut() = snapshot.0;
    }
}

fn sample_layout_bounds(motion: &LayoutMotionState, now: Instant) -> GeometryBounds {
    let progress = if motion.duration.is_zero() {
        1.0
    } else {
        (now.saturating_duration_since(motion.started).as_secs_f64()
            / motion.duration.as_secs_f64())
        .clamp(0.0, 1.0)
    };
    let progress = match motion.easing {
        crate::MotionEasing::Linear => progress,
        crate::MotionEasing::EaseIn => progress * progress,
        crate::MotionEasing::EaseOut => 1.0 - (1.0 - progress).powi(2),
        crate::MotionEasing::EaseInOut if progress < 0.5 => 2.0 * progress * progress,
        crate::MotionEasing::EaseInOut => 1.0 - (-2.0 * progress + 2.0).powi(2) / 2.0,
    };
    let interpolate = |from: f64, to: f64| from + (to - from) * progress;
    GeometryBounds {
        x: interpolate(motion.from.x, motion.to.x),
        y: interpolate(motion.from.y, motion.to.y),
        width: interpolate(motion.from.width, motion.to.width),
        height: interpolate(motion.from.height, motion.to.height),
    }
}

fn sample_trigger_progress(motion: &TriggerMotionState, now: Instant) -> f64 {
    let progress = if motion.duration.is_zero() {
        1.0
    } else {
        (now.saturating_duration_since(motion.started).as_secs_f64()
            / motion.duration.as_secs_f64())
        .clamp(0.0, 1.0)
    };
    motion.from + (motion.to - motion.from) * progress
}

#[derive(Clone, Debug)]
pub(crate) struct GeometrySnapshot(Rc<GeometryState>);

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
        assert!(registry.is_presented(node));
        registry.begin_frame();
        assert!(!registry.is_presented(node));
        assert!(!registry.update(node, initial));
        assert!(registry.is_presented(node));
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

    #[test]
    fn layout_and_trigger_motion_sample_from_committed_native_state() {
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(crate::UiNode::text("target")).unwrap();
        let node = tree.root_id().unwrap();
        let registry = GeometryRegistry::new();
        let start = Instant::now();
        let old = GeometryBounds::new(0.0, 0.0, 100.0, 20.0).unwrap();
        registry.update(
            node,
            ElementGeometry {
                layout: old,
                visual: old,
                clip: None,
            },
        );
        let next = GeometryBounds::new(100.0, 40.0, 200.0, 40.0).unwrap();
        let initial = registry.sample_layout_motion(
            node,
            next,
            None,
            Duration::from_millis(100),
            crate::MotionEasing::Linear,
            start,
        );
        assert!((initial.offset_x + 100.0).abs() < 0.01);
        let middle = registry.sample_layout_motion(
            node,
            next,
            None,
            Duration::from_millis(100),
            crate::MotionEasing::Linear,
            start + Duration::from_millis(50),
        );
        assert!((middle.offset_x + 50.0).abs() < 0.01);

        let binding = crate::MotionProgressBinding::new(
            crate::MotionProgressDriver::Hover,
            crate::MotionSource::Transition(crate::MotionTransition::new(
                crate::MotionProperty::Opacity,
                0.5,
                1.0,
                100,
            )),
        )
        .unwrap();
        registry.set_motion_trigger(node, crate::MotionProgressDriver::Hover, true);
        let sample = registry.sample_motion_trigger(node, &binding, start);
        assert!(sample.active);
        let sample =
            registry.sample_motion_trigger(node, &binding, start + Duration::from_millis(100));
        assert!((sample.value - 1.0).abs() < 0.01);
    }
}
