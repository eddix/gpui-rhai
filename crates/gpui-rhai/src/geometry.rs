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

/// Finite two-dimensional affine transform using column-vector coordinates.
///
/// Points map as `x' = m11*x + m21*y + tx` and
/// `y' = m12*x + m22*y + ty`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine2D {
    m11: f64,
    m12: f64,
    m21: f64,
    m22: f64,
    tx: f64,
    ty: f64,
}

impl Default for Affine2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Affine2D {
    pub const IDENTITY: Self = Self {
        m11: 1.0,
        m12: 0.0,
        m21: 0.0,
        m22: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    /// Construct one validated affine matrix.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::InvalidTransform`] for non-finite members.
    pub fn new(
        m11: f64,
        m12: f64,
        m21: f64,
        m22: f64,
        tx: f64,
        ty: f64,
    ) -> Result<Self, GeometryError> {
        let transform = Self {
            m11,
            m12,
            m21,
            m22,
            tx,
            ty,
        };
        if transform.is_finite() {
            Ok(transform)
        } else {
            Err(GeometryError::InvalidTransform)
        }
    }

    /// Construct a translation.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::InvalidTransform`] for non-finite offsets.
    pub fn translation(x: f64, y: f64) -> Result<Self, GeometryError> {
        Self::new(1.0, 0.0, 0.0, 1.0, x, y)
    }

    /// Construct an axis-aligned scale.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::InvalidTransform`] for non-finite scales.
    pub fn scale(x: f64, y: f64) -> Result<Self, GeometryError> {
        Self::new(x, 0.0, 0.0, y, 0.0, 0.0)
    }

    /// Construct a rotation in degrees.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::InvalidTransform`] for a non-finite angle.
    pub fn rotation_degrees(degrees: f64) -> Result<Self, GeometryError> {
        if !degrees.is_finite() {
            return Err(GeometryError::InvalidTransform);
        }
        let radians = degrees.to_radians();
        let (sin, cos) = radians.sin_cos();
        Self::new(cos, sin, -sin, cos, 0.0, 0.0)
    }

    /// Construct independent X/Y skews in degrees.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::InvalidTransform`] for invalid angles.
    pub fn skew_degrees(x: f64, y: f64) -> Result<Self, GeometryError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(GeometryError::InvalidTransform);
        }
        Self::new(
            1.0,
            y.to_radians().tan(),
            x.to_radians().tan(),
            1.0,
            0.0,
            0.0,
        )
    }

    /// Apply `self`, then `next`.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::InvalidTransform`] if composition overflows.
    pub fn then(self, next: Self) -> Result<Self, GeometryError> {
        Self::new(
            next.m11.mul_add(self.m11, next.m21 * self.m12),
            next.m12.mul_add(self.m11, next.m22 * self.m12),
            next.m11.mul_add(self.m21, next.m21 * self.m22),
            next.m12.mul_add(self.m21, next.m22 * self.m22),
            next.m11
                .mul_add(self.tx, next.m21.mul_add(self.ty, next.tx)),
            next.m12
                .mul_add(self.tx, next.m22.mul_add(self.ty, next.ty)),
        )
    }

    /// Rebase this transform around an origin.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::InvalidTransform`] for invalid coordinates.
    pub fn around(self, origin: (f64, f64)) -> Result<Self, GeometryError> {
        Self::translation(-origin.0, -origin.1)?
            .then(self)?
            .then(Self::translation(origin.0, origin.1)?)
    }

    #[must_use]
    pub fn components(self) -> (f64, f64, f64, f64, f64, f64) {
        (self.m11, self.m12, self.m21, self.m22, self.tx, self.ty)
    }

    #[must_use]
    pub fn map_point(self, point: (f64, f64)) -> (f64, f64) {
        (
            self.m11
                .mul_add(point.0, self.m21.mul_add(point.1, self.tx)),
            self.m12
                .mul_add(point.0, self.m22.mul_add(point.1, self.ty)),
        )
    }

    #[must_use]
    pub fn inverse(self) -> Option<Self> {
        let scale = self
            .m11
            .abs()
            .max(self.m12.abs())
            .max(self.m21.abs())
            .max(self.m22.abs());
        if scale == 0.0 || !scale.is_finite() {
            return None;
        }
        let (a, b, c, d) = (
            self.m11 / scale,
            self.m12 / scale,
            self.m21 / scale,
            self.m22 / scale,
        );
        let determinant = a.mul_add(d, -(b * c));
        if !determinant.is_finite() || determinant.abs() <= f64::EPSILON {
            return None;
        }
        let m11 = (d / determinant) / scale;
        let m12 = (-b / determinant) / scale;
        let m21 = (-c / determinant) / scale;
        let m22 = (a / determinant) / scale;
        Self::new(
            m11,
            m12,
            m21,
            m22,
            -(m11.mul_add(self.tx, m21 * self.ty)),
            -(m12.mul_add(self.tx, m22 * self.ty)),
        )
        .ok()
    }

    #[must_use]
    pub fn transform_bounds(self, bounds: GeometryBounds) -> GeometryBounds {
        let points = [
            self.map_point((bounds.x, bounds.y)),
            self.map_point((bounds.x + bounds.width, bounds.y)),
            self.map_point((bounds.x, bounds.y + bounds.height)),
            self.map_point((bounds.x + bounds.width, bounds.y + bounds.height)),
        ];
        let (min_x, max_x) = points
            .iter()
            .map(|point| point.0)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
                (min.min(value), max.max(value))
            });
        let (min_y, max_y) = points
            .iter()
            .map(|point| point.1)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
                (min.min(value), max.max(value))
            });
        GeometryBounds {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }

    #[must_use]
    pub fn is_finite(self) -> bool {
        [self.m11, self.m12, self.m21, self.m22, self.tx, self.ty]
            .into_iter()
            .all(f64::is_finite)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresentedGeometry {
    pub local: GeometryBounds,
    pub layout: GeometryBounds,
    pub visual: GeometryBounds,
    pub local_to_window: Affine2D,
    pub window_to_local: Option<Affine2D>,
    pub clip: Option<GeometryBounds>,
}

impl PresentedGeometry {
    /// Build bidirectional presented coordinates from committed element facts.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::InvalidTransform`] if derived scale or
    /// translation overflows.
    pub fn from_element(geometry: ElementGeometry) -> Result<Self, GeometryError> {
        let scale_x = if geometry.layout.width.abs() <= f64::EPSILON {
            1.0
        } else {
            geometry.visual.width / geometry.layout.width
        };
        let scale_y = if geometry.layout.height.abs() <= f64::EPSILON {
            1.0
        } else {
            geometry.visual.height / geometry.layout.height
        };
        let local_to_window = Affine2D::scale(scale_x, scale_y)?
            .then(Affine2D::translation(geometry.visual.x, geometry.visual.y)?)?;
        Ok(Self {
            local: GeometryBounds {
                x: 0.0,
                y: 0.0,
                width: geometry.layout.width,
                height: geometry.layout.height,
            },
            layout: geometry.layout,
            visual: geometry.visual,
            local_to_window,
            window_to_local: local_to_window.inverse(),
            clip: geometry.clip,
        })
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
    readers: BTreeMap<NodeId, BTreeSet<crate::read_dependency::ReadDependency>>,
    // Keep direct NodeId dependencies separate: detaching a ref must not detach
    // an independent direct read of the same node by the same contribution.
    ref_readers: crate::element_ref::ElementRefGeometryBindings,
    dirty: BTreeSet<ComponentInstancePath>,
    layout_motion: BTreeMap<NodeId, LayoutMotionState>,
    shared_layout: BTreeMap<(String, String), (NodeId, GeometryBounds)>,
    motion_progress: BTreeMap<(NodeId, crate::MotionProperty), f64>,
    motion_trigger_targets: BTreeMap<(NodeId, crate::MotionProgressDriver), bool>,
    motion_triggers: BTreeMap<(NodeId, crate::MotionProgressDriver), TriggerMotionState>,
    canvas_transforms: BTreeMap<NodeId, CanvasMotionTransform>,
    canvas_drawables: BTreeMap<NodeId, ElementGeometry>,
    element_offsets: BTreeMap<NodeId, (f64, f64)>,
    /// This frame's hitbox of each node with an element ref, so native code
    /// can ask whether the pointer is over its visible, unoccluded part.
    hitboxes: BTreeMap<NodeId, gpui::HitboxId>,
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

#[derive(Clone, Copy)]
pub(crate) struct LayoutMotionRequest<'a> {
    pub shared: Option<(&'a str, &'a str)>,
    pub duration: Duration,
    pub easing: crate::MotionEasing,
    pub preference: crate::MotionPreference,
    pub now: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CanvasMotionTransform {
    pub rotate: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub skew_x: f64,
    pub skew_y: f64,
    pub path_progress: Option<f64>,
}

impl Default for CanvasMotionTransform {
    fn default() -> Self {
        Self {
            rotate: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            skew_x: 0.0,
            skew_y: 0.0,
            path_progress: None,
        }
    }
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
        let owners = geometry_read_owners(state, node);
        state.dirty.extend(owners);
        true
    }

    pub(crate) fn sample_layout_motion(
        &self,
        node: NodeId,
        layout: GeometryBounds,
        request: LayoutMotionRequest<'_>,
    ) -> LayoutMotionSample {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        let shared_previous = request.shared.and_then(|(group, id)| {
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
            && request.duration > Duration::ZERO
        {
            let from = state
                .layout_motion
                .get(&node)
                .map_or(previous, |motion| sample_layout_bounds(motion, request.now));
            state.layout_motion.insert(
                node,
                LayoutMotionState {
                    from,
                    to: layout,
                    started: request.now,
                    duration: request.duration,
                    easing: request.easing,
                },
            );
        }
        if let Some((group, id)) = request.shared {
            state
                .shared_layout
                .insert((group.to_owned(), id.to_owned()), (node, layout));
        }
        if request.preference != crate::MotionPreference::Normal {
            state.layout_motion.remove(&node);
            return LayoutMotionSample::default();
        }
        let Some(motion) = state.layout_motion.get(&node) else {
            return LayoutMotionSample::default();
        };
        let sampled = sample_layout_bounds(motion, request.now);
        let done = request.now.saturating_duration_since(motion.started) >= motion.duration;
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

    pub(crate) fn delay_motion(&self, delay: Duration) {
        if delay.is_zero() {
            return;
        }
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        for motion in state.layout_motion.values_mut() {
            motion.started = motion.started.checked_add(delay).unwrap_or(motion.started);
        }
        for motion in state.motion_triggers.values_mut() {
            motion.started = motion.started.checked_add(delay).unwrap_or(motion.started);
        }
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

    pub(crate) fn update_canvas_transform(&self, node: NodeId, transform: CanvasMotionTransform) {
        Rc::make_mut(&mut self.inner.borrow_mut())
            .canvas_transforms
            .insert(node, transform);
    }

    pub(crate) fn canvas_transform(&self, node: NodeId) -> CanvasMotionTransform {
        self.inner
            .borrow()
            .canvas_transforms
            .get(&node)
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn record_hitbox(&self, node: NodeId, hitbox: gpui::HitboxId) {
        Rc::make_mut(&mut self.inner.borrow_mut())
            .hitboxes
            .insert(node, hitbox);
    }

    pub(crate) fn hitbox(&self, node: NodeId) -> Option<gpui::HitboxId> {
        self.inner.borrow().hitboxes.get(&node).copied()
    }

    pub(crate) fn record_element_offset(&self, node: NodeId, offset: (f64, f64)) {
        Rc::make_mut(&mut self.inner.borrow_mut())
            .element_offsets
            .insert(node, offset);
    }

    pub(crate) fn update_canvas_drawable(
        &self,
        node: NodeId,
        visual: GeometryBounds,
        offset: (f64, f64),
    ) {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        let Some(outer) = state.committed.get(&node) else {
            return;
        };
        let base_offset = state
            .element_offsets
            .get(&node)
            .copied()
            .unwrap_or_default();
        let geometry = ElementGeometry {
            layout: GeometryBounds {
                x: visual.x - (offset.0 - base_offset.0),
                y: visual.y - (offset.1 - base_offset.1),
                ..visual
            },
            visual,
            clip: outer.clip,
        };
        if state.canvas_drawables.get(&node) != Some(&geometry) {
            state.canvas_drawables.insert(node, geometry);
            let owners = geometry_read_owners(state, node);
            state.dirty.extend(owners);
        }
    }

    pub(crate) fn canvas_drawable(&self, node: NodeId) -> Option<ElementGeometry> {
        self.inner.borrow().canvas_drawables.get(&node).copied()
    }

    pub(crate) fn sample_motion_trigger(
        &self,
        node: NodeId,
        binding: &crate::MotionProgressBinding,
        preference: crate::MotionPreference,
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
        if preference != crate::MotionPreference::Normal {
            state.motion_triggers.remove(&key);
            return TriggerMotionSample {
                value: crate::motion::sample_progress_source(&binding.source, target),
                active: false,
            };
        }
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
            .insert(crate::read_dependency::ReadDependency::component(reader));
        Ok(geometry)
    }

    #[cfg(test)]
    pub(crate) fn read_tracked(
        &self,
        node: NodeId,
        reader: impl Into<crate::read_dependency::ReadDependency>,
    ) -> Option<ElementGeometry> {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        state.readers.entry(node).or_default().insert(reader.into());
        state.committed.get(&node).copied()
    }

    pub(crate) fn read_ref_tracked(
        &self,
        reference: &crate::ElementRefId,
        node: NodeId,
        reader: impl Into<crate::read_dependency::ReadDependency>,
    ) -> Option<ElementGeometry> {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        state
            .ref_readers
            .entry(node)
            .or_default()
            .entry(reference.clone())
            .or_default()
            .insert(reader.into());
        state.committed.get(&node).copied()
    }

    /// Replace the logical-ref observation targets for one committed view scope.
    /// The snapshot is authoritative even when no binding currently exists.
    pub(crate) fn sync_ref_readers(
        &self,
        scope: &ComponentInstancePath,
        bindings: crate::element_ref::ElementRefGeometryBindings,
    ) {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        let previous = state.ref_readers.clone();
        for refs in state.ref_readers.values_mut() {
            refs.retain(|reference, _| !reference.component().is_within(scope));
        }
        state.ref_readers.retain(|_, refs| !refs.is_empty());
        for (node, refs) in bindings {
            for (reference, readers) in refs {
                if state.committed.contains_key(&node) {
                    let previous_readers =
                        previous.get(&node).and_then(|refs| refs.get(&reference));
                    state.dirty.extend(
                        readers
                            .iter()
                            .filter(|reader| {
                                previous_readers.is_none_or(|previous| !previous.contains(*reader))
                            })
                            .map(|reader| reader.owner.clone()),
                    );
                }
                state
                    .ref_readers
                    .entry(node)
                    .or_default()
                    .insert(reference, readers);
            }
        }
    }

    pub(crate) fn get(&self, node: NodeId) -> Option<ElementGeometry> {
        self.inner.borrow().committed.get(&node).copied()
    }

    pub(crate) fn reset_contribution(&self, reader: &crate::read_dependency::ReadDependency) {
        let mut current = self.inner.borrow_mut();
        retain_geometry_readers(Rc::make_mut(&mut current), |existing| existing != reader);
    }
    pub(crate) fn retain_contributions(
        &self,
        scope: &ComponentInstancePath,
        active: &BTreeSet<crate::read_dependency::ReadContribution>,
    ) {
        let mut current = self.inner.borrow_mut();
        retain_geometry_readers(Rc::make_mut(&mut current), |reader| {
            reader.retained_in_contribution_scope(scope, active)
        });
    }

    pub(crate) fn retain_reader_scope(
        &self,
        scope: &ComponentInstancePath,
        active: &BTreeSet<ComponentInstancePath>,
    ) {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        retain_geometry_readers(state, |reader| {
            reader.retained_in_owner_scope(scope, active)
        });
        for refs in state.ref_readers.values_mut() {
            refs.retain(|reference, _| {
                let provider = reference.component();
                !provider.is_within(scope) || provider == scope || active.contains(provider)
            });
        }
        state.ref_readers.retain(|_, refs| !refs.is_empty());
        state
            .dirty
            .retain(|owner| !owner.is_within(scope) || owner == scope || active.contains(owner));
    }

    pub(crate) fn remove_scope(&self, scope: &ComponentInstancePath) {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        retain_geometry_readers(state, |reader| !reader.owner.is_within(scope));
        for refs in state.ref_readers.values_mut() {
            refs.retain(|reference, _| !reference.component().is_within(scope));
        }
        state.ref_readers.retain(|_, refs| !refs.is_empty());
        state.dirty.retain(|owner| !owner.is_within(scope));
    }

    pub(crate) fn presented(&self, node: NodeId) -> Result<PresentedGeometry, GeometryError> {
        self.get(node)
            .ok_or(GeometryError::Unavailable(node))
            .and_then(PresentedGeometry::from_element)
    }

    pub(crate) fn begin_frame(&self) {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        state
            .shared_layout
            .retain(|_, (node, _)| state.presented.contains(node));
        state.presented.clear();
        // A node that is not drawn this frame has no hitbox.
        state.hitboxes.clear();
    }

    pub(crate) fn finish_frame(&self) {
        let mut current = self.inner.borrow_mut();
        let state = Rc::make_mut(&mut current);
        state
            .shared_layout
            .retain(|_, (node, _)| state.presented.contains(node));
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
        state.ref_readers.retain(|node, _| active.contains(node));
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
        state
            .canvas_transforms
            .retain(|node, _| active.contains(node));
        state
            .canvas_drawables
            .retain(|node, _| active.contains(node));
        state
            .element_offsets
            .retain(|node, _| active.contains(node));
        state.hitboxes.retain(|node, _| active.contains(node));
    }

    pub(crate) fn retain_shared_layout_ids(&self, active: &BTreeSet<(String, String)>) {
        Rc::make_mut(&mut self.inner.borrow_mut())
            .shared_layout
            .retain(|identity, _| active.contains(identity));
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

fn geometry_read_owners(state: &GeometryState, node: NodeId) -> BTreeSet<ComponentInstancePath> {
    state
        .readers
        .get(&node)
        .into_iter()
        .flatten()
        .chain(
            state
                .ref_readers
                .get(&node)
                .into_iter()
                .flat_map(BTreeMap::values)
                .flatten(),
        )
        .map(|reader| reader.owner.clone())
        .collect()
}

fn retain_geometry_readers(
    state: &mut GeometryState,
    predicate: impl Fn(&crate::read_dependency::ReadDependency) -> bool,
) {
    crate::read_dependency::retain_readers(&mut state.readers, &predicate);
    for refs in state.ref_readers.values_mut() {
        crate::read_dependency::retain_readers(refs, &predicate);
    }
    state.ref_readers.retain(|_, refs| !refs.is_empty());
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
    #[error("affine transform members must be finite")]
    InvalidTransform,
    #[error("geometry for retained node {0} is not committed")]
    Unavailable(NodeId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_inverse_is_scale_relative_and_rejects_singular_matrices() {
        for scale in [1e-200, 1e-10, 1e-5, 1.0, 1e5, 1e200] {
            let transform = Affine2D::scale(scale, scale * 2.0)
                .unwrap()
                .then(Affine2D::rotation_degrees(31.0).unwrap())
                .unwrap();
            let inverse = transform.inverse().unwrap();
            for point in [(0.0, 0.0), (1.0, 2.0), (-10.0, 40.0)] {
                let restored = inverse.map_point(transform.map_point(point));
                assert!((restored.0 - point.0).abs() < 1e-10);
                assert!((restored.1 - point.1).abs() < 1e-10);
            }
        }
        assert!(Affine2D::scale(0.0, 1.0).unwrap().inverse().is_none());
        assert!(
            Affine2D::new(1.0, 2.0, 2.0, 4.0, 0.0, 0.0)
                .unwrap()
                .inverse()
                .is_none()
        );
    }

    #[test]
    fn affine_composition_inverse_and_bounds_share_one_coordinate_fact() {
        let transform = Affine2D::scale(2.0, 3.0)
            .unwrap()
            .then(Affine2D::rotation_degrees(90.0).unwrap())
            .unwrap()
            .then(Affine2D::translation(10.0, 20.0).unwrap())
            .unwrap();
        let mapped = transform.map_point((1.0, 2.0));
        assert!((mapped.0 - 4.0).abs() < 0.000_001);
        assert!((mapped.1 - 22.0).abs() < 0.000_001);
        let restored = transform.inverse().unwrap().map_point(mapped);
        assert!((restored.0 - 1.0).abs() < 0.000_001);
        assert!((restored.1 - 2.0).abs() < 0.000_001);

        let rotated = Affine2D::rotation_degrees(90.0)
            .unwrap()
            .transform_bounds(GeometryBounds::new(0.0, 0.0, 10.0, 5.0).unwrap());
        assert!((rotated.x + 5.0).abs() < 0.000_001);
        assert!(rotated.y.abs() < 0.000_001);
        assert!((rotated.width - 5.0).abs() < 0.000_001);
        assert!((rotated.height - 10.0).abs() < 0.000_001);
    }

    #[test]
    fn presented_geometry_maps_local_and_window_coordinates_both_ways() {
        let presented = PresentedGeometry::from_element(ElementGeometry {
            layout: GeometryBounds::new(10.0, 20.0, 100.0, 50.0).unwrap(),
            visual: GeometryBounds::new(30.0, 40.0, 200.0, 25.0).unwrap(),
            clip: Some(GeometryBounds::new(0.0, 0.0, 300.0, 200.0).unwrap()),
        })
        .unwrap();
        let window = presented.local_to_window.map_point((25.0, 10.0));
        assert_eq!(window, (80.0, 45.0));
        let local = presented.window_to_local.unwrap().map_point(window);
        assert!((local.0 - 25.0).abs() < 0.000_001);
        assert!((local.1 - 10.0).abs() < 0.000_001);
    }

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
            LayoutMotionRequest {
                shared: None,
                duration: Duration::from_millis(100),
                easing: crate::MotionEasing::Linear,
                preference: crate::MotionPreference::Normal,
                now: start,
            },
        );
        assert!((initial.offset_x + 100.0).abs() < 0.01);
        let middle = registry.sample_layout_motion(
            node,
            next,
            LayoutMotionRequest {
                shared: None,
                duration: Duration::from_millis(100),
                easing: crate::MotionEasing::Linear,
                preference: crate::MotionPreference::Normal,
                now: start + Duration::from_millis(50),
            },
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
        let sample =
            registry.sample_motion_trigger(node, &binding, crate::MotionPreference::Normal, start);
        assert!(sample.active);
        let sample = registry.sample_motion_trigger(
            node,
            &binding,
            crate::MotionPreference::Normal,
            start + Duration::from_millis(100),
        );
        assert!((sample.value - 1.0).abs() < 0.01);
    }

    #[test]
    fn shared_layout_history_expires_after_its_last_presented_frame() {
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(crate::UiNode::text("card")).unwrap();
        let node = tree.root_id().unwrap();
        let registry = GeometryRegistry::new();
        let bounds = GeometryBounds::new(0.0, 0.0, 100.0, 20.0).unwrap();
        registry.update(
            node,
            ElementGeometry {
                layout: bounds,
                visual: bounds,
                clip: None,
            },
        );
        registry.sample_layout_motion(
            node,
            bounds,
            LayoutMotionRequest {
                shared: Some(("cards", "alpha")),
                duration: Duration::from_millis(100),
                easing: crate::MotionEasing::Linear,
                preference: crate::MotionPreference::Normal,
                now: Instant::now(),
            },
        );
        assert_eq!(registry.inner.borrow().shared_layout.len(), 1);
        registry.begin_frame();
        assert_eq!(registry.inner.borrow().shared_layout.len(), 1);
        registry.begin_frame();
        assert!(registry.inner.borrow().shared_layout.is_empty());
    }
}
