use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use thiserror::Error;

use crate::{ComponentInstancePath, UiNode};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VirtualCollectionId {
    pub component: ComponentInstancePath,
    pub key: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualCollectionNodeSpec {
    pub id: VirtualCollectionId,
    pub label: String,
    pub data: crate::VirtualCollectionData,
    pub realized: BTreeMap<usize, UiNode>,
    pub estimated_height: f64,
    /// Fixed logical-pixel viewport height, or `None` to fill the resolved
    /// height offered by the parent flex layout.
    pub height: Option<f64>,
    pub overdraw_pixels: f64,
    pub bottom_align: bool,
    pub follow_tail: bool,
    /// Stable data key to reveal when this controlled target changes.
    pub reveal_key: Option<String>,
    /// Item indices whose realized nodes act as top-pinned section headers.
    pub sticky_headers: Arc<BTreeSet<usize>>,
}

#[derive(Clone, Debug, Default)]
pub struct VirtualRequestRegistry {
    requests: Rc<RefCell<BTreeMap<VirtualCollectionId, BTreeSet<usize>>>>,
    metrics: Rc<RefCell<BTreeMap<VirtualCollectionId, VirtualCollectionMetrics>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualCollectionMetrics {
    pub id: VirtualCollectionId,
    pub item_count: usize,
    pub realized_count: usize,
    pub realized_range: Range<usize>,
    pub requested_count: usize,
    pub requested_range: Range<usize>,
    pub visible_range: Range<usize>,
    pub viewport_height: f64,
    pub scroll_item: usize,
    pub scroll_offset: f64,
    pub is_scrolled: bool,
    pub sticky_header: Option<usize>,
    pub bottom_align: bool,
    pub follow_tail: bool,
}

impl VirtualCollectionMetrics {
    fn new(id: VirtualCollectionId) -> Self {
        Self {
            id,
            item_count: 0,
            realized_count: 0,
            realized_range: 0..0,
            requested_count: 0,
            requested_range: 0..0,
            visible_range: 0..0,
            viewport_height: 0.0,
            scroll_item: 0,
            scroll_offset: 0.0,
            is_scrolled: false,
            sticky_header: None,
            bottom_align: false,
            follow_tail: false,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct VirtualRequestSnapshot {
    requests: BTreeMap<VirtualCollectionId, BTreeSet<usize>>,
    metrics: BTreeMap<VirtualCollectionId, VirtualCollectionMetrics>,
}

impl VirtualRequestRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request(&self, id: VirtualCollectionId, indices: impl IntoIterator<Item = usize>) {
        let indices = indices.into_iter().collect::<Vec<_>>();
        let requested_count = {
            let mut requests = self.requests.borrow_mut();
            let requested = requests.entry(id.clone()).or_default();
            requested.extend(indices.iter().copied());
            requested.len()
        };
        let new_range = index_range(indices);
        let mut metrics = self.metrics.borrow_mut();
        let metrics = metrics
            .entry(id.clone())
            .or_insert_with(|| VirtualCollectionMetrics::new(id));
        metrics.requested_count = requested_count;
        if !new_range.is_empty() {
            metrics.requested_range = if metrics.requested_range.is_empty() {
                new_range
            } else {
                metrics.requested_range.start.min(new_range.start)
                    ..metrics.requested_range.end.max(new_range.end)
            };
        }
    }

    /// Replace one collection's pending request with a complete atomic target.
    ///
    /// Unlike [`Self::request`], this does not union partial layout callbacks.
    /// The virtual element calls it only after collecting a complete prepaint.
    pub(crate) fn request_target(
        &self,
        id: VirtualCollectionId,
        indices: impl IntoIterator<Item = usize>,
    ) {
        let indices = indices.into_iter().collect::<BTreeSet<_>>();
        let requested_count = indices.len();
        self.requests
            .borrow_mut()
            .insert(id.clone(), indices.clone());
        let requested_range = index_range(indices);
        let mut metrics = self.metrics.borrow_mut();
        let metrics = metrics
            .entry(id.clone())
            .or_insert_with(|| VirtualCollectionMetrics::new(id));
        metrics.requested_count = requested_count;
        metrics.requested_range = requested_range;
    }

    pub(crate) fn clear_target(&self, id: &VirtualCollectionId) {
        self.requests.borrow_mut().remove(id);
        if let Some(metrics) = self.metrics.borrow_mut().get_mut(id) {
            metrics.requested_count = 0;
            metrics.requested_range = 0..0;
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.requests.borrow().is_empty()
    }

    #[must_use]
    pub(crate) fn has_scope(&self, root: &ComponentInstancePath) -> bool {
        self.requests
            .borrow()
            .keys()
            .any(|id| id.component.is_within(root))
    }

    pub(crate) fn drain(&self) -> BTreeMap<VirtualCollectionId, BTreeSet<usize>> {
        let requests = std::mem::take(&mut *self.requests.borrow_mut());
        for metrics in self.metrics.borrow_mut().values_mut() {
            metrics.requested_count = 0;
            metrics.requested_range = 0..0;
        }
        requests
    }

    pub(crate) fn snapshot(&self) -> VirtualRequestSnapshot {
        VirtualRequestSnapshot {
            requests: self.requests.borrow().clone(),
            metrics: self.metrics.borrow().clone(),
        }
    }

    pub(crate) fn restore(&self, snapshot: VirtualRequestSnapshot) {
        *self.requests.borrow_mut() = snapshot.requests;
        *self.metrics.borrow_mut() = snapshot.metrics;
    }

    pub(crate) fn retain(&self, active: &BTreeSet<VirtualCollectionId>) {
        self.requests
            .borrow_mut()
            .retain(|id, _| active.contains(id));
        self.metrics
            .borrow_mut()
            .retain(|id, _| active.contains(id));
    }

    pub(crate) fn report_frame(
        &self,
        spec: &VirtualCollectionNodeSpec,
        viewport_height: f64,
        scroll_item: usize,
        scroll_offset: f64,
        measured_visible: Option<Range<usize>>,
    ) {
        let mut metrics = self.metrics.borrow_mut();
        let metrics = metrics
            .entry(spec.id.clone())
            .or_insert_with(|| VirtualCollectionMetrics::new(spec.id.clone()));
        metrics.item_count = spec.data.len();
        metrics.realized_count = spec.realized.len();
        metrics.realized_range = index_range(spec.realized.keys().copied());
        if let Some(visible_range) = measured_visible {
            metrics.visible_range = visible_range;
        } else if metrics.visible_range.is_empty() {
            metrics.visible_range = metrics.realized_range.clone();
        } else {
            metrics.visible_range = metrics.visible_range.start.min(spec.data.len())
                ..metrics.visible_range.end.min(spec.data.len());
        }
        metrics.viewport_height = viewport_height;
        metrics.scroll_item = scroll_item;
        metrics.scroll_offset = scroll_offset;
        metrics.sticky_header = spec
            .sticky_headers
            .range(..=scroll_item)
            .next_back()
            .copied();
        metrics.bottom_align = spec.bottom_align;
        metrics.follow_tail = spec.follow_tail;
    }

    pub(crate) fn report_scroll(
        &self,
        id: &VirtualCollectionId,
        visible_range: Range<usize>,
        is_scrolled: bool,
    ) {
        let mut metrics = self.metrics.borrow_mut();
        let metrics = metrics
            .entry(id.clone())
            .or_insert_with(|| VirtualCollectionMetrics::new(id.clone()));
        metrics.visible_range = visible_range;
        metrics.is_scrolled = is_scrolled;
    }

    #[must_use]
    pub fn inspect(&self) -> Vec<VirtualCollectionMetrics> {
        self.metrics.borrow().values().cloned().collect()
    }

    pub(crate) fn metrics(&self, id: &VirtualCollectionId) -> Option<VirtualCollectionMetrics> {
        self.metrics.borrow().get(id).cloned()
    }
}

fn index_range(indices: impl IntoIterator<Item = usize>) -> Range<usize> {
    let mut indices = indices.into_iter();
    let Some(first) = indices.next() else {
        return 0..0;
    };
    let (min, max) = indices.fold((first, first), |(min, max), index| {
        (min.min(index), max.max(index))
    });
    min..max.saturating_add(1)
}

fn validate_viewport(scroll_offset: f64, viewport_height: f64) -> Result<(), VirtualListError> {
    if !scroll_offset.is_finite() || scroll_offset < 0.0 {
        return Err(VirtualListError::InvalidScrollOffset(scroll_offset));
    }
    if !viewport_height.is_finite() || viewport_height < 0.0 {
        return Err(VirtualListError::InvalidViewport(viewport_height));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VariableListSpec {
    pub estimated_height: f64,
    pub overscan_pixels: f64,
}

impl VariableListSpec {
    /// Construct a variable-height one-dimensional virtualization policy.
    ///
    /// # Errors
    ///
    /// Returns for non-finite/non-positive estimates or negative overscan.
    pub fn new(estimated_height: f64, overscan_pixels: f64) -> Result<Self, VirtualListError> {
        if !estimated_height.is_finite() || estimated_height <= 0.0 {
            return Err(VirtualListError::InvalidEstimatedHeight(estimated_height));
        }
        if !overscan_pixels.is_finite() || overscan_pixels < 0.0 {
            return Err(VirtualListError::InvalidOverscan(overscan_pixels));
        }
        Ok(Self {
            estimated_height,
            overscan_pixels,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct VariableListState {
    keys: Vec<String>,
    indices: BTreeMap<String, usize>,
    measured: BTreeMap<String, f64>,
    heights: Vec<f64>,
    prefix: FenwickTree,
}

impl VariableListState {
    /// Install ordered keys while preserving measurements by identity.
    ///
    /// # Errors
    ///
    /// Returns for invalid policy or duplicate keys.
    pub fn set_keys(
        &mut self,
        keys: Vec<String>,
        spec: VariableListSpec,
    ) -> Result<(), VirtualListError> {
        VariableListSpec::new(spec.estimated_height, spec.overscan_pixels)?;
        let mut unique = BTreeSet::new();
        if let Some(key) = keys.iter().find(|key| !unique.insert((*key).clone())) {
            return Err(VirtualListError::DuplicateKey(key.clone()));
        }
        self.measured.retain(|key, _| unique.contains(key));
        self.indices = keys
            .iter()
            .enumerate()
            .map(|(index, key)| (key.clone(), index))
            .collect();
        self.heights = keys
            .iter()
            .map(|key| {
                self.measured
                    .get(key)
                    .copied()
                    .unwrap_or(spec.estimated_height)
            })
            .collect();
        self.prefix = FenwickTree::from_values(&self.heights);
        self.keys = keys;
        Ok(())
    }

    /// Record a measured item height and return the scroll correction required
    /// to keep an anchor key at the same viewport position.
    ///
    /// # Errors
    ///
    /// Returns for unknown keys or invalid measurements.
    pub fn measure(
        &mut self,
        key: &str,
        height: f64,
        anchor: Option<&str>,
    ) -> Result<f64, VirtualListError> {
        if !height.is_finite() || height <= 0.0 {
            return Err(VirtualListError::InvalidMeasurement(height));
        }
        let index = *self
            .indices
            .get(key)
            .ok_or_else(|| VirtualListError::UnknownKey(key.to_owned()))?;
        let anchor_index = anchor
            .map(|anchor| {
                self.indices
                    .get(anchor)
                    .copied()
                    .ok_or_else(|| VirtualListError::UnknownKey(anchor.to_owned()))
            })
            .transpose()?;
        let before = anchor_index.map_or(0.0, |anchor| self.prefix.sum(anchor));
        let delta = height - self.heights[index];
        self.heights[index] = height;
        self.measured.insert(key.to_owned(), height);
        self.prefix.add(index, delta);
        let after = anchor_index.map_or(0.0, |anchor| self.prefix.sum(anchor));
        Ok(after - before)
    }

    /// Compute the bounded realization window and spacer geometry.
    ///
    /// # Errors
    ///
    /// Returns viewport validation errors.
    pub fn window(
        &self,
        spec: VariableListSpec,
        scroll_offset: f64,
        viewport_height: f64,
    ) -> Result<VariableListWindow, VirtualListError> {
        validate_viewport(scroll_offset, viewport_height)?;
        VariableListSpec::new(spec.estimated_height, spec.overscan_pixels)?;
        if self.keys.is_empty() {
            return Ok(VariableListWindow::default());
        }
        let start_offset = (scroll_offset - spec.overscan_pixels).max(0.0);
        let end_offset = scroll_offset + viewport_height + spec.overscan_pixels;
        let start = self.prefix.lower_bound(start_offset).min(self.keys.len());
        let end = self
            .prefix
            .lower_bound(end_offset)
            .saturating_add(1)
            .min(self.keys.len());
        Ok(VariableListWindow {
            range: start..end,
            leading: self.prefix.sum(start),
            trailing: (self.prefix.total() - self.prefix.sum(end)).max(0.0),
            total: self.prefix.total(),
        })
    }

    /// Return the positive visible offset that keeps the bottom aligned.
    #[must_use]
    pub fn tail_offset(&self, viewport_height: f64) -> f64 {
        (self.prefix.total() - viewport_height.max(0.0)).max(0.0)
    }

    #[must_use]
    pub fn measured_count(&self) -> usize {
        self.measured.len()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VariableListWindow {
    pub range: Range<usize>,
    pub leading: f64,
    pub trailing: f64,
    pub total: f64,
}

#[derive(Clone, Debug, Default)]
struct FenwickTree {
    tree: Vec<f64>,
}

impl FenwickTree {
    fn from_values(values: &[f64]) -> Self {
        let mut tree = Self {
            tree: vec![0.0; values.len() + 1],
        };
        for (index, value) in values.iter().copied().enumerate() {
            tree.add(index, value);
        }
        tree
    }

    fn add(&mut self, index: usize, delta: f64) {
        let mut cursor = index + 1;
        while cursor < self.tree.len() {
            self.tree[cursor] += delta;
            cursor += cursor & cursor.wrapping_neg();
        }
    }

    fn sum(&self, end: usize) -> f64 {
        let mut cursor = end.min(self.tree.len().saturating_sub(1));
        let mut total = 0.0;
        while cursor > 0 {
            total += self.tree[cursor];
            cursor &= cursor - 1;
        }
        total
    }

    fn total(&self) -> f64 {
        self.sum(self.tree.len().saturating_sub(1))
    }

    fn lower_bound(&self, target: f64) -> usize {
        let mut index = 0usize;
        let mut accumulated = 0.0;
        let mut step = self.tree.len().next_power_of_two() / 2;
        while step > 0 {
            let next = index + step;
            if next < self.tree.len() && accumulated + self.tree[next] <= target {
                index = next;
                accumulated += self.tree[next];
            }
            step /= 2;
        }
        index.min(self.tree.len().saturating_sub(1))
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum VirtualListError {
    #[error("scroll offset must be finite and non-negative, got {0}")]
    InvalidScrollOffset(f64),
    #[error("viewport height must be finite and non-negative, got {0}")]
    InvalidViewport(f64),
    #[error("virtual list key `{0}` is duplicated")]
    DuplicateKey(String),
    #[error("virtual list key `{0}` is unknown")]
    UnknownKey(String),
    #[error("estimated item height must be finite and positive, got {0}")]
    InvalidEstimatedHeight(f64),
    #[error("virtual-list overscan pixels must be finite and non-negative, got {0}")]
    InvalidOverscan(f64),
    #[error("measured item height must be finite and positive, got {0}")]
    InvalidMeasurement(f64),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UiValue;

    #[test]
    fn variable_height_window_is_bounded_and_measurement_preserves_anchor() {
        let spec = VariableListSpec::new(20.0, 80.0).unwrap();
        let mut list = VariableListState::default();
        list.set_keys(
            (0..10_000).map(|index| format!("row-{index}")).collect(),
            spec,
        )
        .unwrap();
        let window = list.window(spec, 50_000.0, 600.0).unwrap();
        assert!(window.range.len() <= 40, "{window:?}");
        let correction = list.measure("row-0", 50.0, Some("row-2500")).unwrap();
        assert!((correction - 30.0).abs() < f64::EPSILON);
        let corrected = list.window(spec, 50_030.0, 600.0).unwrap();
        assert_eq!(corrected.range, window.range);
        assert_eq!(list.measured_count(), 1);
    }

    #[test]
    fn variable_height_measurements_survive_reorder_and_tail_alignment() {
        let spec = VariableListSpec::new(20.0, 0.0).unwrap();
        let mut list = VariableListState::default();
        list.set_keys(vec!["a".into(), "b".into(), "c".into()], spec)
            .unwrap();
        list.measure("b", 60.0, None).unwrap();
        list.set_keys(vec!["c".into(), "b".into(), "a".into()], spec)
            .unwrap();
        assert_eq!(list.measured_count(), 1);
        assert!((list.tail_offset(50.0) - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn virtual_collection_metrics_track_frame_scroll_requests_and_rollback() {
        let id = VirtualCollectionId {
            component: ComponentInstancePath::root("Chat", "main"),
            key: "messages".to_owned(),
        };
        let item = |key: &str| {
            UiValue::Map(BTreeMap::from([(
                "key".to_owned(),
                UiValue::String(key.to_owned()),
            )]))
        };
        let spec = VirtualCollectionNodeSpec {
            id: id.clone(),
            label: "Messages".to_owned(),
            data: (0..5).map(|index| item(&format!("row-{index}"))).collect(),
            realized: BTreeMap::from([(1, UiNode::text("one")), (2, UiNode::text("two"))]),
            estimated_height: 24.0,
            height: Some(120.0),
            overdraw_pixels: 48.0,
            bottom_align: true,
            follow_tail: true,
            reveal_key: None,
            sticky_headers: Arc::new(BTreeSet::new()),
        };
        let registry = VirtualRequestRegistry::new();
        registry.report_frame(&spec, 120.0, 1, 3.5, Some(1..3));
        registry.request(id.clone(), [3, 4]);
        registry.report_scroll(&id, 1..4, true);
        let metrics = &registry.inspect()[0];
        assert_eq!(metrics.realized_range, 1..3);
        assert_eq!(metrics.requested_range, 3..5);
        assert_eq!(metrics.visible_range, 1..4);
        assert!(metrics.bottom_align && metrics.follow_tail && metrics.is_scrolled);
        assert_eq!(metrics.sticky_header, None);

        let snapshot = registry.snapshot();
        let _ = registry.drain();
        assert_eq!(registry.inspect()[0].requested_count, 0);
        registry.restore(snapshot);
        assert_eq!(registry.inspect()[0].requested_count, 2);
        registry.retain(&BTreeSet::new());
        assert!(registry.inspect().is_empty());
    }
}
