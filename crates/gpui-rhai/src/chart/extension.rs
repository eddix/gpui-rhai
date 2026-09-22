use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};

use thiserror::Error;

use super::{
    ChartDataset, ChartFormatSpec, ChartMark, ChartMarkGeometry, ChartRect, ChartScale,
    ChartSeriesSpec, ChartTheme,
};

pub struct ChartCustomSeriesContext<'a> {
    pub spec: &'a ChartSeriesSpec,
    pub dataset: &'a ChartDataset,
    pub bounds: ChartRect,
    pub theme: &'a ChartTheme,
    pub x_scale: Option<&'a ChartScale>,
    pub y_scale: Option<&'a ChartScale>,
}

/// Compile-time trusted Rust custom series extension.
///
/// Returned marks enter the same paint, hit-test, tooltip, semantic, motion,
/// Inspector, and export scene as built-in series.
pub trait HostChartSeries: Send + Sync {
    /// Produce validated public chart marks for one prepared series.
    ///
    /// # Errors
    ///
    /// Returns an application-facing diagnostic; the candidate scene is then
    /// rejected or reports the bounded series error.
    fn layout(&self, context: ChartCustomSeriesContext<'_>) -> Result<Vec<ChartMark>, String>;
}

#[derive(Clone, Default)]
pub struct ChartSeriesRegistry {
    inner: Arc<RwLock<BTreeMap<String, Arc<dyn HostChartSeries>>>>,
}

impl std::fmt::Debug for ChartSeriesRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let registered = self
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        formatter
            .debug_struct("ChartSeriesRegistry")
            .field("registered", &registered)
            .finish()
    }
}

impl ChartSeriesRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a renderer under a stable compile-time ID.
    ///
    /// # Errors
    ///
    /// Returns for an unsafe or duplicate ID or a poisoned registry.
    pub fn register(
        &self,
        id: impl Into<String>,
        renderer: impl HostChartSeries + 'static,
    ) -> Result<(), ChartSeriesExtensionError> {
        let id = id.into();
        if !valid_identifier(&id) {
            return Err(ChartSeriesExtensionError::InvalidId(id));
        }
        let mut registry = self
            .inner
            .write()
            .map_err(|_| ChartSeriesExtensionError::Poisoned)?;
        if registry.insert(id.clone(), Arc::new(renderer)).is_some() {
            return Err(ChartSeriesExtensionError::Duplicate(id));
        }
        Ok(())
    }

    pub(crate) fn layout(
        &self,
        id: &str,
        context: ChartCustomSeriesContext<'_>,
    ) -> Result<Vec<ChartMark>, ChartSeriesExtensionError> {
        let renderer = self
            .inner
            .read()
            .map_err(|_| ChartSeriesExtensionError::Poisoned)?
            .get(id)
            .cloned()
            .ok_or_else(|| ChartSeriesExtensionError::Unknown(id.to_owned()))?;
        let series_key = context.spec.key.clone();
        let marks =
            renderer
                .layout(context)
                .map_err(|message| ChartSeriesExtensionError::Failed {
                    id: id.to_owned(),
                    message,
                })?;
        validate_custom_marks(&series_key, &marks)?;
        Ok(marks)
    }

    #[cfg(feature = "dev-reload")]
    pub(crate) fn copy_from(&self, source: &Self) {
        let source = source
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        *self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = source;
    }
}

fn validate_custom_marks(
    series_key: &str,
    marks: &[ChartMark],
) -> Result<(), ChartSeriesExtensionError> {
    if marks.len() > 100_000 {
        return Err(ChartSeriesExtensionError::InvalidMarks {
            series: series_key.to_owned(),
            message: "more than 100000 marks".to_owned(),
        });
    }
    let mut keys = BTreeSet::new();
    for mark in marks {
        if mark.series_key != series_key
            || mark.key.is_empty()
            || mark.datum_key.is_empty()
            || !keys.insert(mark.key.clone())
            || !geometry_is_finite(&mark.geometry)
        {
            return Err(ChartSeriesExtensionError::InvalidMarks {
                series: series_key.to_owned(),
                message: format!("invalid or duplicate mark `{}`", mark.key),
            });
        }
    }
    Ok(())
}

fn geometry_is_finite(geometry: &ChartMarkGeometry) -> bool {
    match geometry {
        ChartMarkGeometry::Rect(rect) => {
            rect.x.is_finite()
                && rect.y.is_finite()
                && rect.width.is_finite()
                && rect.height.is_finite()
                && rect.width >= 0.0
                && rect.height >= 0.0
        }
        ChartMarkGeometry::Circle { center, radius } => {
            center.x.is_finite() && center.y.is_finite() && radius.is_finite() && *radius >= 0.0
        }
        ChartMarkGeometry::Polyline { points, width } => {
            points.len() >= 2
                && width.is_finite()
                && *width > 0.0
                && points
                    .iter()
                    .all(|point| point.x.is_finite() && point.y.is_finite())
        }
        ChartMarkGeometry::Polygon(points) => {
            points.len() >= 3
                && points
                    .iter()
                    .all(|point| point.x.is_finite() && point.y.is_finite())
        }
        ChartMarkGeometry::CompoundPolygon(rings) => {
            !rings.is_empty()
                && rings.iter().all(|points| {
                    points.len() >= 3
                        && points
                            .iter()
                            .all(|point| point.x.is_finite() && point.y.is_finite())
                })
        }
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic())
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ChartSeriesExtensionError {
    #[error("chart series renderer ID `{0}` must be a safe identifier")]
    InvalidId(String),
    #[error("chart series renderer `{0}` is already registered")]
    Duplicate(String),
    #[error("chart series renderer `{0}` is not registered")]
    Unknown(String),
    #[error("chart series renderer `{id}` failed: {message}")]
    Failed { id: String, message: String },
    #[error("chart series registry lock was poisoned")]
    Poisoned,
    #[error("custom chart series `{series}` returned invalid marks: {message}")]
    InvalidMarks { series: String, message: String },
}

/// Trusted Rust formatter for application-specific numeric conventions.
pub trait HostChartFormatter: Send + Sync {
    /// Format one finite value for a declared locale and format specification.
    ///
    /// # Errors
    ///
    /// Returns an application-facing diagnostic; the built-in label is kept.
    fn format(&self, value: f64, locale: &str, spec: &ChartFormatSpec) -> Result<String, String>;
}

#[derive(Clone, Default)]
pub struct ChartFormatterRegistry {
    inner: Arc<RwLock<BTreeMap<String, Arc<dyn HostChartFormatter>>>>,
}

impl std::fmt::Debug for ChartFormatterRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let registered = self
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        formatter
            .debug_struct("ChartFormatterRegistry")
            .field("registered", &registered)
            .finish()
    }
}

impl ChartFormatterRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a formatter under a stable compile-time ID.
    ///
    /// # Errors
    ///
    /// Returns for an unsafe/duplicate ID or poisoned registry.
    pub fn register(
        &self,
        id: impl Into<String>,
        formatter: impl HostChartFormatter + 'static,
    ) -> Result<(), ChartFormatterError> {
        let id = id.into();
        if !valid_identifier(&id) {
            return Err(ChartFormatterError::InvalidId(id));
        }
        let mut registry = self
            .inner
            .write()
            .map_err(|_| ChartFormatterError::Poisoned)?;
        if registry.insert(id.clone(), Arc::new(formatter)).is_some() {
            return Err(ChartFormatterError::Duplicate(id));
        }
        Ok(())
    }

    pub(crate) fn format(
        &self,
        id: &str,
        value: f64,
        locale: &str,
        spec: &ChartFormatSpec,
    ) -> Result<String, ChartFormatterError> {
        let handler = self
            .inner
            .read()
            .map_err(|_| ChartFormatterError::Poisoned)?
            .get(id)
            .cloned()
            .ok_or_else(|| ChartFormatterError::Unknown(id.to_owned()))?;
        let formatted =
            handler
                .format(value, locale, spec)
                .map_err(|message| ChartFormatterError::Failed {
                    id: id.to_owned(),
                    message,
                })?;
        if formatted.len() > 1_024 || formatted.contains('\n') || formatted.contains('\r') {
            return Err(ChartFormatterError::InvalidOutput(id.to_owned()));
        }
        Ok(formatted)
    }

    #[cfg(feature = "dev-reload")]
    pub(crate) fn copy_from(&self, source: &Self) {
        let source = source
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        *self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = source;
    }
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ChartFormatterError {
    #[error("chart formatter ID `{0}` must be a safe identifier")]
    InvalidId(String),
    #[error("chart formatter `{0}` is already registered")]
    Duplicate(String),
    #[error("chart formatter `{0}` is not registered")]
    Unknown(String),
    #[error("chart formatter `{id}` failed: {message}")]
    Failed { id: String, message: String },
    #[error("chart formatter registry lock was poisoned")]
    Poisoned,
    #[error("chart formatter `{0}` returned an overlong or multiline label")]
    InvalidOutput(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CurrencyFormatter;

    impl HostChartFormatter for CurrencyFormatter {
        fn format(&self, value: f64, locale: &str, _: &ChartFormatSpec) -> Result<String, String> {
            Ok(format!("{locale}:{value:.2}"))
        }
    }

    #[test]
    fn host_formatter_registry_is_typed_and_duplicate_safe() {
        let registry = ChartFormatterRegistry::new();
        registry.register("currency", CurrencyFormatter).unwrap();
        assert_eq!(
            registry
                .format("currency", 12.5, "zh-CN", &ChartFormatSpec::default())
                .unwrap(),
            "zh-CN:12.50"
        );
        assert!(matches!(
            registry.register("currency", CurrencyFormatter),
            Err(ChartFormatterError::Duplicate(_))
        ));
    }
}
