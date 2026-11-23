use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub(crate) const MAX_ELEMENT_WIDTH_BREAKPOINTS: usize = 32;

/// The interval in which the last observed width-class result remains stable.
/// Fixed-size storage also covers multiple class reads: their stable intervals
/// intersect, rather than retaining an unbounded set of caller thresholds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WidthClassInterval {
    lower_exclusive: Option<u64>,
    upper_inclusive: Option<u64>,
}

impl WidthClassInterval {
    fn lower(self) -> f64 {
        self.lower_exclusive
            .map_or(f64::NEG_INFINITY, f64::from_bits)
    }

    fn upper(self) -> f64 {
        self.upper_inclusive.map_or(f64::INFINITY, f64::from_bits)
    }

    pub(crate) fn contains(self, width: f64) -> bool {
        width > self.lower() && width <= self.upper()
    }

    pub(crate) fn intersection(self, other: Self) -> Option<Self> {
        let lower = self.lower().max(other.lower());
        let upper = self.upper().min(other.upper());
        (lower < upper).then_some(Self {
            lower_exclusive: lower.is_finite().then_some(lower.to_bits()),
            upper_inclusive: upper.is_finite().then_some(upper.to_bits()),
        })
    }

    /// A narrower stable interval observes at least as many changes.
    pub(crate) fn covers(self, other: Self) -> bool {
        self.lower() >= other.lower() && self.upper() <= other.upper()
    }
}

pub(crate) fn validate_element_width_breakpoints(values: &[f64]) -> Result<(), ResponsiveError> {
    if values.len() > MAX_ELEMENT_WIDTH_BREAKPOINTS
        || values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0 || *value > f64::from(f32::MAX))
        || values.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(ResponsiveError::InvalidElementWidthBreakpoints);
    }
    Ok(())
}

pub(crate) fn classify_element_width(
    width: f64,
    breakpoints: &[f64],
) -> Result<(usize, WidthClassInterval), ResponsiveError> {
    validate_element_width_breakpoints(breakpoints)?;
    if !width.is_finite() || width < 0.0 {
        return Err(ResponsiveError::InvalidWidth(width));
    }
    let class = breakpoints.partition_point(|boundary| width > *boundary);
    Ok((
        class,
        WidthClassInterval {
            lower_exclusive: class
                .checked_sub(1)
                .map(|index| breakpoints[index].to_bits()),
            upper_inclusive: breakpoints.get(class).map(|value| value.to_bits()),
        },
    ))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewportClass {
    Compact,
    Regular,
    Wide,
}

impl ViewportClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Regular => "regular",
            Self::Wide => "wide",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewportBreakpoints {
    compact_max: f64,
    regular_max: f64,
}

impl ViewportBreakpoints {
    /// Create increasing finite viewport boundaries in logical pixels.
    ///
    /// # Errors
    ///
    /// Returns [`ResponsiveError::InvalidBreakpoints`] for non-positive,
    /// non-finite, or non-increasing values.
    pub fn new(compact_max: f64, regular_max: f64) -> Result<Self, ResponsiveError> {
        if compact_max.is_finite()
            && regular_max.is_finite()
            && compact_max > 0.0
            && regular_max > compact_max
        {
            Ok(Self {
                compact_max,
                regular_max,
            })
        } else {
            Err(ResponsiveError::InvalidBreakpoints {
                compact_max,
                regular_max,
            })
        }
    }

    /// Classify a finite non-negative viewport width.
    ///
    /// # Errors
    ///
    /// Returns [`ResponsiveError::InvalidWidth`] for invalid geometry.
    pub fn classify(self, width: f64) -> Result<ViewportClass, ResponsiveError> {
        if !width.is_finite() || width < 0.0 {
            return Err(ResponsiveError::InvalidWidth(width));
        }
        Ok(if width <= self.compact_max {
            ViewportClass::Compact
        } else if width <= self.regular_max {
            ViewportClass::Regular
        } else {
            ViewportClass::Wide
        })
    }

    #[must_use]
    pub const fn compact_max(self) -> f64 {
        self.compact_max
    }

    #[must_use]
    pub const fn regular_max(self) -> f64 {
        self.regular_max
    }
}

impl Default for ViewportBreakpoints {
    fn default() -> Self {
        Self {
            compact_max: 600.0,
            regular_max: 1000.0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ResponsiveRuntime {
    breakpoints: ViewportBreakpoints,
    windows: BTreeMap<String, ViewportClass>,
}

impl ResponsiveRuntime {
    #[must_use]
    pub fn new(breakpoints: ViewportBreakpoints) -> Self {
        Self {
            breakpoints,
            windows: BTreeMap::new(),
        }
    }

    /// Update one window and report whether its discrete class changed.
    ///
    /// # Errors
    ///
    /// Returns invalid-width errors.
    pub fn update_window(&mut self, window: &str, width: f64) -> Result<bool, ResponsiveError> {
        let class = self.breakpoints.classify(width)?;
        Ok(self.windows.insert(window.to_owned(), class) != Some(class))
    }

    /// Check a width without mutating the committed viewport class.
    ///
    /// # Errors
    ///
    /// Returns [`ResponsiveError::InvalidWidth`] for invalid geometry.
    pub fn would_update_window(&self, window: &str, width: f64) -> Result<bool, ResponsiveError> {
        let class = self.breakpoints.classify(width)?;
        Ok(self.windows.get(window).copied() != Some(class))
    }

    #[must_use]
    pub fn class(&self, window: &str) -> ViewportClass {
        self.windows
            .get(window)
            .copied()
            .unwrap_or(ViewportClass::Regular)
    }

    pub fn remove_window(&mut self, window: &str) -> bool {
        self.windows.remove(window).is_some()
    }

    #[must_use]
    pub const fn breakpoints(&self) -> ViewportBreakpoints {
        self.breakpoints
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum ResponsiveError {
    #[error(
        "element width breakpoints must contain at most 32 finite, non-negative, strictly increasing logical-pixel values representable by GPUI"
    )]
    InvalidElementWidthBreakpoints,
    #[error(
        "viewport breakpoints must be finite, positive, and increasing; got {compact_max}, {regular_max}"
    )]
    InvalidBreakpoints { compact_max: f64, regular_max: f64 },
    #[error("viewport width must be finite and non-negative, got {0}")]
    InvalidWidth(f64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_width_classes_have_inclusive_boundaries_and_exact_stable_intervals() {
        for boundaries in [
            &[][..],
            &[0.0][..],
            &[480.0][..],
            &[100.0, 480.0, 720.0][..],
        ] {
            for width in [
                0.0, 1.0, 100.0, 100.5, 479.5, 480.0, 480.5, 720.0, 721.0, 1200.0,
            ] {
                let (class, interval) = classify_element_width(width, boundaries).unwrap();
                assert_eq!(
                    class,
                    boundaries
                        .iter()
                        .filter(|threshold| width > **threshold)
                        .count()
                );
                for next in [
                    0.0, 1.0, 100.0, 100.5, 479.5, 480.0, 480.5, 720.0, 721.0, 1200.0,
                ] {
                    assert_eq!(
                        interval.contains(next),
                        classify_element_width(next, boundaries).unwrap().0 == class,
                        "{boundaries:?}: {width} -> {next}"
                    );
                }
            }
        }
    }

    #[test]
    fn element_width_intervals_combine_without_storing_query_history() {
        let (_, first) = classify_element_width(400.0, &[480.0]).unwrap();
        let (_, second) = classify_element_width(400.0, &[300.0, 600.0]).unwrap();
        let combined = first.intersection(second).unwrap();
        assert!(combined.contains(400.0) && combined.contains(480.0));
        assert!(!combined.contains(300.0) && !combined.contains(480.5));
        assert!(combined.covers(first) && combined.covers(second));
        assert_eq!(combined, second.intersection(first).unwrap());
        assert_eq!(
            std::mem::size_of_val(&first),
            std::mem::size_of_val(&combined)
        );
    }

    #[test]
    fn element_width_queries_reject_invalid_or_unbounded_threshold_lists() {
        for values in [
            vec![f64::NAN],
            vec![f64::INFINITY],
            vec![-1.0],
            vec![480.0, 480.0],
            vec![480.0, 300.0],
            vec![0.0, -0.0],
            vec![f64::from(f32::MAX) * 2.0],
            vec![1.0; 33],
        ] {
            assert!(
                validate_element_width_breakpoints(&values).is_err(),
                "{values:?}"
            );
        }
        assert!(
            validate_element_width_breakpoints(&(0..32).map(f64::from).collect::<Vec<_>>()).is_ok()
        );
    }

    #[test]
    fn discrete_classes_change_only_at_configured_boundaries() {
        let breakpoints = ViewportBreakpoints::new(500.0, 900.0).unwrap();
        let mut runtime = ResponsiveRuntime::new(breakpoints);
        assert!(runtime.update_window("main", 480.0).unwrap());
        assert!(!runtime.update_window("main", 499.0).unwrap());
        assert!(runtime.update_window("main", 700.0).unwrap());
        assert_eq!(runtime.class("main"), ViewportClass::Regular);
        assert!(runtime.update_window("main", 1200.0).unwrap());
        assert_eq!(runtime.class("main"), ViewportClass::Wide);
    }
}
