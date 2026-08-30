use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use rhai::{
    CustomType, Engine, EvalAltResult, FLOAT, FuncRegistration, INT, Position, TypeBuilder,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::UiNode;

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualListItem {
    pub key: String,
    pub node: UiNode,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualListNodeSpec {
    pub key: String,
    pub label: String,
    pub items: Vec<VirtualListItem>,
    pub estimated_height: f64,
    pub height: f64,
    pub overdraw_pixels: f64,
    pub bottom_align: bool,
    pub follow_tail: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct VirtualListSpec {
    pub row_height: f64,
    pub overscan: usize,
}

impl VirtualListSpec {
    /// Create an equal-height one-dimensional virtualization specification.
    ///
    /// # Errors
    ///
    /// Returns [`VirtualListError`] unless row height is finite and positive.
    pub fn new(row_height: f64, overscan: usize) -> Result<Self, VirtualListError> {
        if row_height.is_finite() && row_height > 0.0 {
            Ok(Self {
                row_height,
                overscan,
            })
        } else {
            Err(VirtualListError::InvalidRowHeight(row_height))
        }
    }

    /// Compute the only item range that a renderer should realize.
    ///
    /// # Errors
    ///
    /// Returns [`VirtualListError`] for non-finite or negative viewport values.
    pub fn visible_range(
        self,
        item_count: usize,
        scroll_offset: f64,
        viewport_height: f64,
    ) -> Result<Range<usize>, VirtualListError> {
        validate_viewport(scroll_offset, viewport_height)?;
        if item_count == 0 {
            return Ok(0..0);
        }
        let first_visible = nonnegative_to_usize((scroll_offset / self.row_height).floor());
        let visible_count =
            nonnegative_to_usize((viewport_height / self.row_height).ceil()).saturating_add(1);
        let start = first_visible.saturating_sub(self.overscan).min(item_count);
        let end = first_visible
            .saturating_add(visible_count)
            .saturating_add(self.overscan)
            .min(item_count);
        Ok(start..end)
    }
}

fn nonnegative_to_usize(value: f64) -> usize {
    value.to_string().parse().unwrap_or(usize::MAX)
}

impl CustomType for VirtualListSpec {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("VirtualListSpec");
    }
}

pub(crate) fn register_virtual_list_api(engine: &mut Engine) {
    engine.build_type::<VirtualListSpec>();
    FuncRegistration::new("virtual_list_spec")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |row_height: FLOAT, overscan: INT| -> Result<VirtualListSpec, Box<EvalAltResult>> {
                let overscan = usize::try_from(overscan).map_err(|_| {
                    Box::new(EvalAltResult::ErrorRuntime(
                        "virtual list overscan must be non-negative".into(),
                        Position::NONE,
                    ))
                })?;
                VirtualListSpec::new(row_height, overscan).map_err(|error| {
                    Box::new(EvalAltResult::ErrorRuntime(
                        error.to_string().into(),
                        Position::NONE,
                    ))
                })
            },
        );
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

#[derive(Clone, Debug, Default)]
pub struct VirtualListState {
    keys: Vec<String>,
    focused: Option<String>,
}

impl VirtualListState {
    /// Install ordered stable keys and preserve focus by identity.
    ///
    /// If the focused item was removed, focus moves to the nearest surviving
    /// index instead of silently jumping to unrelated state by position.
    ///
    /// # Errors
    ///
    /// Returns [`VirtualListError::DuplicateKey`] for duplicate keys.
    pub fn set_keys(&mut self, keys: Vec<String>) -> Result<(), VirtualListError> {
        let mut unique = BTreeSet::new();
        if let Some(duplicate) = keys.iter().find(|key| !unique.insert((*key).clone())) {
            return Err(VirtualListError::DuplicateKey(duplicate.clone()));
        }
        let previous_index = self
            .focused
            .as_ref()
            .and_then(|focused| self.keys.iter().position(|key| key == focused));
        if let Some(focused) = &self.focused
            && !keys.contains(focused)
        {
            self.focused = previous_index
                .and_then(|index| keys.get(index.min(keys.len().saturating_sub(1))))
                .cloned();
        }
        self.keys = keys;
        Ok(())
    }

    /// Focus a known item key.
    ///
    /// # Errors
    ///
    /// Returns [`VirtualListError::UnknownKey`] for absent keys.
    pub fn focus(&mut self, key: &str) -> Result<(), VirtualListError> {
        if self.keys.iter().any(|candidate| candidate == key) {
            self.focused = Some(key.to_owned());
            Ok(())
        } else {
            Err(VirtualListError::UnknownKey(key.to_owned()))
        }
    }

    pub fn focus_next(&mut self) {
        if self.keys.is_empty() {
            self.focused = None;
            return;
        }
        let next = self
            .focused
            .as_ref()
            .and_then(|focused| self.keys.iter().position(|key| key == focused))
            .map_or(0, |index| (index + 1).min(self.keys.len() - 1));
        self.focused = self.keys.get(next).cloned();
    }

    pub fn focus_previous(&mut self) {
        if self.keys.is_empty() {
            self.focused = None;
            return;
        }
        let previous = self
            .focused
            .as_ref()
            .and_then(|focused| self.keys.iter().position(|key| key == focused))
            .map_or(0, |index| index.saturating_sub(1));
        self.focused = self.keys.get(previous).cloned();
    }

    pub fn focus_first(&mut self) {
        self.focused = self.keys.first().cloned();
    }

    pub fn focus_last(&mut self) {
        self.focused = self.keys.last().cloned();
    }

    #[must_use]
    pub fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    /// Return realization metrics for diagnostics and performance assertions.
    ///
    /// # Errors
    ///
    /// Returns viewport validation errors from [`VirtualListSpec`].
    pub fn metrics(
        &self,
        spec: VirtualListSpec,
        scroll_offset: f64,
        viewport_height: f64,
    ) -> Result<VirtualListMetrics, VirtualListError> {
        let range = spec.visible_range(self.keys.len(), scroll_offset, viewport_height)?;
        Ok(VirtualListMetrics {
            item_count: self.keys.len(),
            realized_count: range.len(),
            range,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VirtualListMetrics {
    pub item_count: usize,
    pub realized_count: usize,
    pub range: Range<usize>,
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
    #[error("virtual row height must be finite and positive, got {0}")]
    InvalidRowHeight(f64),
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
    use crate::{RuntimeEngine, UiNodeKind};

    #[test]
    fn five_thousand_items_have_bounded_realization() {
        let mut list = VirtualListState::default();
        list.set_keys((0..5_000).map(|index| format!("row-{index}")).collect())
            .unwrap();
        let metrics = list
            .metrics(VirtualListSpec::new(24.0, 4).unwrap(), 48_000.0, 480.0)
            .unwrap();
        assert_eq!(metrics.item_count, 5_000);
        assert!(metrics.realized_count <= 30, "{metrics:?}");
    }

    #[test]
    fn focus_survives_reorder_filter_and_keyboard_moves() {
        let mut list = VirtualListState::default();
        list.set_keys(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()])
            .unwrap();
        list.focus("b").unwrap();
        list.set_keys(vec!["c".to_owned(), "b".to_owned(), "a".to_owned()])
            .unwrap();
        assert_eq!(list.focused(), Some("b"));
        list.set_keys(vec!["c".to_owned(), "a".to_owned()]).unwrap();
        assert_eq!(list.focused(), Some("a"));
        list.focus_previous();
        assert_eq!(list.focused(), Some("c"));
        list.focus_next();
        assert_eq!(list.focused(), Some("a"));
    }

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
    fn rhai_builds_keyed_native_virtual_list_node() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn view() {
                        virtual_list(#{
                            key: "files", label: "Files", estimated_height: 28,
                            height: 280, overdraw_pixels: 56,
                            alignment: "top", follow_tail: false,
                            items: [
                                #{ key: "a", node: text("A") },
                                #{ key: "b", node: text("B") }
                            ]
                        })
                    }
                "#,
            )
            .unwrap();
        let root = engine.render(&compiled).unwrap();
        assert!(matches!(
            root.kind(),
            UiNodeKind::VirtualList { spec }
                if spec.items.len() == 2
                    && (spec.estimated_height - 28.0).abs() < f64::EPSILON
        ));
    }
}
