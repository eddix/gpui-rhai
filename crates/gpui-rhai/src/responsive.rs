use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

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
