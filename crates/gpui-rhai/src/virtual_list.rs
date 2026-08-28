use std::collections::BTreeSet;
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
    pub row_height: f64,
    pub height: f64,
    pub overscan: usize,
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
    fn rhai_builds_keyed_native_virtual_list_node() {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile(
                r#"
                    fn view() {
                        virtual_list(#{
                            key: "files", label: "Files", row_height: 28,
                            height: 280, overscan: 2,
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
                if spec.items.len() == 2 && (spec.row_height - 28.0).abs() < f64::EPSILON
        ));
    }
}
