#![allow(clippy::cast_possible_truncation, clippy::float_cmp)]

use std::collections::BTreeMap;

use thiserror::Error;

use super::{ChartAxisDirection, ChartTimeZone};

#[derive(Clone, Debug, PartialEq)]
pub struct ChartTick {
    pub value: f64,
    pub position: f64,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChartScale {
    Linear(ContinuousScale),
    Log(ContinuousScale),
    Time {
        scale: ContinuousScale,
        timezone: ChartTimeZone,
    },
    Category(CategoryScale),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousScale {
    domain_min: f64,
    domain_max: f64,
    range_start: f64,
    range_end: f64,
    reversed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CategoryScale {
    categories: Vec<String>,
    positions: BTreeMap<String, usize>,
    range_start: f64,
    range_end: f64,
    reversed: bool,
}

impl ChartScale {
    /// Construct a finite continuous scale.
    ///
    /// # Errors
    ///
    /// Returns for an invalid domain or range.
    pub fn linear(
        domain_min: f64,
        domain_max: f64,
        range_start: f64,
        range_end: f64,
        direction: ChartAxisDirection,
    ) -> Result<Self, ChartScaleError> {
        Ok(Self::Linear(ContinuousScale::new(
            domain_min,
            domain_max,
            range_start,
            range_end,
            direction,
        )?))
    }

    /// Construct a positive base-10 logarithmic scale.
    ///
    /// # Errors
    ///
    /// Returns for a non-positive domain or invalid range.
    pub fn logarithmic(
        domain_min: f64,
        domain_max: f64,
        range_start: f64,
        range_end: f64,
        direction: ChartAxisDirection,
    ) -> Result<Self, ChartScaleError> {
        if domain_min <= 0.0 || domain_max <= 0.0 {
            return Err(ChartScaleError::LogDomain);
        }
        Ok(Self::Log(ContinuousScale::new(
            domain_min.log10(),
            domain_max.log10(),
            range_start,
            range_end,
            direction,
        )?))
    }

    /// Construct a typed epoch-millisecond scale.
    ///
    /// # Errors
    ///
    /// Returns for an invalid domain or range.
    pub fn time(
        domain_min_millis: i64,
        domain_max_millis: i64,
        range_start: f64,
        range_end: f64,
        direction: ChartAxisDirection,
    ) -> Result<Self, ChartScaleError> {
        Self::time_with_zone(
            domain_min_millis,
            domain_max_millis,
            range_start,
            range_end,
            direction,
            ChartTimeZone::Utc,
        )
    }

    /// Construct a typed epoch-millisecond scale with an explicit timezone.
    ///
    /// # Errors
    ///
    /// Returns for an invalid domain/range or unavailable IANA timezone.
    pub fn time_with_zone(
        domain_min_millis: i64,
        domain_max_millis: i64,
        range_start: f64,
        range_end: f64,
        direction: ChartAxisDirection,
        timezone: ChartTimeZone,
    ) -> Result<Self, ChartScaleError> {
        resolve_timezone(&timezone)?;
        Ok(Self::Time {
            scale: ContinuousScale::new(
                i64_to_f64(domain_min_millis),
                i64_to_f64(domain_max_millis),
                range_start,
                range_end,
                direction,
            )?,
            timezone,
        })
    }

    /// Construct a stable unique category scale.
    ///
    /// # Errors
    ///
    /// Returns for empty/duplicate categories or an invalid range.
    pub fn category(
        categories: impl IntoIterator<Item = String>,
        range_start: f64,
        range_end: f64,
        direction: ChartAxisDirection,
    ) -> Result<Self, ChartScaleError> {
        let categories = categories.into_iter().collect::<Vec<_>>();
        if categories.is_empty() {
            return Err(ChartScaleError::EmptyCategory);
        }
        let mut positions = BTreeMap::new();
        for (index, category) in categories.iter().enumerate() {
            if positions.insert(category.clone(), index).is_some() {
                return Err(ChartScaleError::DuplicateCategory(category.clone()));
            }
        }
        validate_range(range_start, range_end)?;
        Ok(Self::Category(CategoryScale {
            categories,
            positions,
            range_start,
            range_end,
            reversed: direction == ChartAxisDirection::Reversed,
        }))
    }

    #[must_use]
    pub fn map_number(&self, value: f64) -> Option<f64> {
        if !value.is_finite() {
            return None;
        }
        match self {
            Self::Linear(scale) | Self::Time { scale, .. } => Some(scale.map(value)),
            Self::Log(scale) if value > 0.0 => Some(scale.map(value.log10())),
            Self::Log(_) | Self::Category(_) => None,
        }
    }

    #[must_use]
    pub fn map_category(&self, value: &str) -> Option<f64> {
        match self {
            Self::Category(scale) => scale.map(value),
            Self::Linear(_) | Self::Log(_) | Self::Time { .. } => None,
        }
    }

    #[must_use]
    pub fn invert(&self, position: f64) -> Option<f64> {
        if !position.is_finite() {
            return None;
        }
        match self {
            Self::Linear(scale) | Self::Time { scale, .. } => Some(scale.invert(position)),
            Self::Log(scale) => Some(10_f64.powf(scale.invert(position))),
            Self::Category(_) => None,
        }
    }

    #[must_use]
    pub fn band_width(&self) -> Option<f64> {
        match self {
            Self::Category(scale) => Some(
                (scale.range_end - scale.range_start).abs() / usize_to_f64(scale.categories.len()),
            ),
            Self::Linear(_) | Self::Log(_) | Self::Time { .. } => None,
        }
    }

    #[must_use]
    pub fn ticks(&self, target: usize) -> Vec<ChartTick> {
        match self {
            Self::Linear(scale) => continuous_ticks(scale, target),
            Self::Time { scale, timezone } => time_ticks(scale, target, timezone),
            Self::Log(scale) => log_ticks(scale),
            Self::Category(scale) => scale
                .categories
                .iter()
                .filter_map(|category| {
                    scale.map(category).map(|position| ChartTick {
                        value: position,
                        position,
                        label: category.clone(),
                    })
                })
                .collect(),
        }
    }
}

impl ContinuousScale {
    fn new(
        domain_min: f64,
        domain_max: f64,
        range_start: f64,
        range_end: f64,
        direction: ChartAxisDirection,
    ) -> Result<Self, ChartScaleError> {
        if !domain_min.is_finite() || !domain_max.is_finite() || domain_max <= domain_min {
            return Err(ChartScaleError::Domain);
        }
        validate_range(range_start, range_end)?;
        Ok(Self {
            domain_min,
            domain_max,
            range_start,
            range_end,
            reversed: direction == ChartAxisDirection::Reversed,
        })
    }

    fn map(&self, value: f64) -> f64 {
        let ratio = (value - self.domain_min) / (self.domain_max - self.domain_min);
        self.range_start
            + if self.reversed { 1.0 - ratio } else { ratio } * (self.range_end - self.range_start)
    }

    fn invert(&self, position: f64) -> f64 {
        let ratio = (position - self.range_start) / (self.range_end - self.range_start);
        self.domain_min
            + if self.reversed { 1.0 - ratio } else { ratio } * (self.domain_max - self.domain_min)
    }
}

impl CategoryScale {
    fn map(&self, value: &str) -> Option<f64> {
        let index = *self.positions.get(value)?;
        let count = usize_to_f64(self.categories.len());
        let logical = (usize_to_f64(index) + 0.5) / count;
        let ratio = if self.reversed {
            1.0 - logical
        } else {
            logical
        };
        Some(self.range_start + ratio * (self.range_end - self.range_start))
    }
}

fn continuous_ticks(scale: &ContinuousScale, target: usize) -> Vec<ChartTick> {
    let target = target.clamp(2, 16);
    let span = scale.domain_max - scale.domain_min;
    let step = nice_step(span / usize_to_f64(target.saturating_sub(1)));
    let start = (scale.domain_min / step).ceil() * step;
    let end = (scale.domain_max / step).floor() * step;
    let mut ticks = Vec::new();
    let mut value = start;
    while value <= end + step * 0.001 && ticks.len() < 64 {
        ticks.push(ChartTick {
            value,
            position: scale.map(value),
            label: format_number_tick(value, step),
        });
        value += step;
    }
    ticks
}

fn time_ticks(scale: &ContinuousScale, target: usize, timezone: &ChartTimeZone) -> Vec<ChartTick> {
    let mut ticks = continuous_ticks(scale, target);
    let step = if ticks.len() >= 2 {
        (ticks[1].value - ticks[0].value).abs()
    } else {
        scale.domain_max - scale.domain_min
    };
    for tick in &mut ticks {
        tick.label = format_time_tick(tick.value, step, timezone);
    }
    ticks
}

fn log_ticks(scale: &ContinuousScale) -> Vec<ChartTick> {
    let start = scale.domain_min.ceil() as i32;
    let end = scale.domain_max.floor() as i32;
    (start..=end)
        .take(32)
        .map(|power| {
            let value = 10_f64.powi(power);
            ChartTick {
                value,
                position: scale.map(f64::from(power)),
                label: format_number_tick(value, value),
            }
        })
        .collect()
}

fn nice_step(value: f64) -> f64 {
    let power = 10_f64.powf(value.abs().max(f64::MIN_POSITIVE).log10().floor());
    let fraction = value / power;
    let nice = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * power
}

fn format_number_tick(value: f64, step: f64) -> String {
    if value.abs() >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value.abs() >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else {
        let decimals = if step.abs() >= 1.0 {
            0
        } else {
            usize::try_from((-step.abs().log10().floor()) as i64)
                .unwrap_or(0)
                .min(6)
        };
        format!("{value:.decimals$}")
    }
}

fn format_time_tick(value: f64, step: f64, timezone: &ChartTimeZone) -> String {
    let millis = value.round() as i64;
    let seconds = millis.div_euclid(1_000);
    let nanos = i32::try_from(millis.rem_euclid(1_000) * 1_000_000).unwrap_or(0);
    let Some(timestamp) = jiff::Timestamp::new(seconds, nanos).ok() else {
        return millis.to_string();
    };
    let Ok(timezone) = resolve_timezone(timezone) else {
        return millis.to_string();
    };
    let zoned = timestamp.to_zoned(timezone);
    let format = if step >= 31_536_000_000.0 {
        "%Y"
    } else if step >= 86_400_000.0 {
        "%Y-%m-%d"
    } else if step >= 3_600_000.0 {
        "%m-%d %H:%M"
    } else if step >= 1_000.0 {
        "%H:%M:%S"
    } else {
        "%H:%M:%S%.3f"
    };
    zoned.strftime(format).to_string()
}

fn resolve_timezone(timezone: &ChartTimeZone) -> Result<jiff::tz::TimeZone, ChartScaleError> {
    match timezone {
        ChartTimeZone::Utc => Ok(jiff::tz::TimeZone::UTC),
        ChartTimeZone::FixedOffsetMinutes(minutes) => {
            let seconds = minutes.saturating_mul(60);
            jiff::tz::Offset::from_seconds(seconds)
                .map(jiff::tz::TimeZone::fixed)
                .map_err(|_| ChartScaleError::TimeZone(format!("offset:{minutes}")))
        }
        ChartTimeZone::Iana(name) => {
            jiff::tz::TimeZone::get(name).map_err(|_| ChartScaleError::TimeZone(name.clone()))
        }
    }
}

fn validate_range(start: f64, end: f64) -> Result<(), ChartScaleError> {
    if start.is_finite() && end.is_finite() && start != end {
        Ok(())
    } else {
        Err(ChartScaleError::Range)
    }
}

#[allow(clippy::cast_precision_loss)]
fn usize_to_f64(value: usize) -> f64 {
    value as f64
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ChartScaleError {
    #[error("chart continuous scale domain must be finite and increasing")]
    Domain,
    #[error("chart logarithmic scale domain must be positive")]
    LogDomain,
    #[error("chart scale range must be finite and non-empty")]
    Range,
    #[error("chart category scale cannot be empty")]
    EmptyCategory,
    #[error("chart category `{0}` is duplicated")]
    DuplicateCategory(String),
    #[error("chart timezone `{0}` is unavailable")]
    TimeZone(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuous_mapping_inverts_and_reversal_is_explicit() {
        let normal =
            ChartScale::linear(0.0, 100.0, 10.0, 210.0, ChartAxisDirection::Normal).unwrap();
        let reversed =
            ChartScale::linear(0.0, 100.0, 10.0, 210.0, ChartAxisDirection::Reversed).unwrap();
        assert_eq!(normal.map_number(25.0), Some(60.0));
        assert_eq!(normal.invert(60.0), Some(25.0));
        assert_eq!(reversed.map_number(25.0), Some(160.0));
    }

    #[test]
    fn categories_occupy_band_centers_and_reject_duplicates() {
        let scale = ChartScale::category(
            ["a".to_owned(), "b".to_owned()],
            0.0,
            100.0,
            ChartAxisDirection::Normal,
        )
        .unwrap();
        assert_eq!(scale.map_category("a"), Some(25.0));
        assert_eq!(scale.map_category("b"), Some(75.0));
        assert_eq!(scale.band_width(), Some(50.0));
        assert!(matches!(
            ChartScale::category(
                ["a".to_owned(), "a".to_owned()],
                0.0,
                1.0,
                ChartAxisDirection::Normal
            ),
            Err(ChartScaleError::DuplicateCategory(_))
        ));
    }

    #[test]
    fn time_ticks_use_the_explicit_timezone_and_reject_unknown_iana_names() {
        let scale = ChartScale::time_with_zone(
            0,
            86_400_000,
            0.0,
            100.0,
            ChartAxisDirection::Normal,
            ChartTimeZone::Iana("Asia/Shanghai".to_owned()),
        )
        .unwrap();
        let labels = scale
            .ticks(3)
            .into_iter()
            .map(|tick| tick.label)
            .collect::<Vec<_>>();
        assert!(
            labels.iter().any(|label| label.contains("01-01")),
            "{labels:?}"
        );
        assert!(matches!(
            ChartScale::time_with_zone(
                0,
                1_000,
                0.0,
                1.0,
                ChartAxisDirection::Normal,
                ChartTimeZone::Iana("Mars/Olympus".to_owned())
            ),
            Err(ChartScaleError::TimeZone(_))
        ));
    }
}
