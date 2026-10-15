//! Wall-clock instants and timezone conversion, independent of charts and motion.
use std::{fmt, rc::Rc};

use jiff::{Timestamp, tz::AmbiguousOffset};
use rhai::{Dynamic, Engine, EvalAltResult, FuncRegistration, ImmutableString, Map, Position};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{CalendarMetadata, DateStyle, NumberMetadata, format_date_with_metadata};

/// Injectable presentation wall clock. It is deliberately not the animation clock.
pub trait WallClockSource: fmt::Debug {
    fn now_ms(&self) -> i64;
}

#[derive(Clone, Debug)]
pub struct WallClock(Rc<dyn WallClockSource>);
impl WallClock {
    #[must_use]
    pub fn system() -> Self {
        Self::from_source(SystemWallClock)
    }
    #[must_use]
    pub fn fixed(epoch_ms: i64) -> Self {
        Self::from_source(FixedWallClock(epoch_ms))
    }
    #[must_use]
    pub fn from_source(source: impl WallClockSource + 'static) -> Self {
        Self(Rc::new(source))
    }
    #[must_use]
    pub fn now_ms(&self) -> i64 {
        self.0.now_ms()
    }
}
impl Default for WallClock {
    fn default() -> Self {
        Self::system()
    }
}
#[derive(Debug)]
struct FixedWallClock(i64);
impl WallClockSource for FixedWallClock {
    fn now_ms(&self) -> i64 {
        self.0
    }
}
#[derive(Debug)]
struct SystemWallClock;
impl WallClockSource for SystemWallClock {
    fn now_ms(&self) -> i64 {
        Timestamp::now().as_millisecond()
    }
}

/// The shared chart/picker timezone grammar. `Local` resolves on the Host.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum TimeZone {
    Local,
    #[default]
    Utc,
    FixedOffsetMinutes(i32),
    Iana(String),
}
impl TimeZone {
    /// Parse `local`, `UTC`, `offset:<minutes>` or an available IANA zone.
    ///
    /// # Errors
    /// Returns [`TimeError::Zone`] for malformed or unavailable zones.
    pub fn parse(value: &str) -> Result<Self, TimeError> {
        let zone = if value == "local" {
            Self::Local
        } else if value.eq_ignore_ascii_case("utc") {
            Self::Utc
        } else if let Some(offset) = value.strip_prefix("offset:") {
            let minutes = offset
                .parse::<i32>()
                .map_err(|_| TimeError::Zone(value.into()))?;
            if !(-1440..=1440).contains(&minutes) {
                return Err(TimeError::Zone(value.into()));
            }
            Self::FixedOffsetMinutes(minutes)
        } else if value.contains('/') {
            Self::Iana(value.into())
        } else {
            return Err(TimeError::Zone(value.into()));
        };
        zone.resolve()?;
        Ok(zone)
    }
    pub(crate) fn resolve(&self) -> Result<jiff::tz::TimeZone, TimeError> {
        match self {
            Self::Local => Ok(jiff::tz::TimeZone::system()),
            Self::Utc => Ok(jiff::tz::TimeZone::UTC),
            Self::FixedOffsetMinutes(minutes) => {
                jiff::tz::Offset::from_seconds(minutes.saturating_mul(60))
                    .map(jiff::tz::TimeZone::fixed)
                    .map_err(|_| TimeError::Zone(format!("offset:{minutes}")))
            }
            Self::Iana(name) => {
                jiff::tz::TimeZone::get(name).map_err(|_| TimeError::Zone(name.clone()))
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TimeMetadata {
    pub hour_cycle: String,
    pub short: String,
    pub medium: String,
    pub am: String,
    pub pm: String,
    /// Localized compact day/hour/minute/second suffixes.
    pub units: Vec<String>,
    /// Patterns contain `{value}` and can be placed before or after it.
    pub past: String,
    pub future: String,
    pub now: String,
}
impl Default for TimeMetadata {
    fn default() -> Self {
        Self {
            hour_cycle: "h23".into(),
            short: "HH:mm".into(),
            medium: "HH:mm:ss".into(),
            am: "AM".into(),
            pm: "PM".into(),
            units: ["d", "h", "m", "s"].map(str::to_owned).into(),
            past: "{value} ago".into(),
            future: "in {value}".into(),
            now: "now".into(),
        }
    }
}
impl TimeMetadata {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !["h12", "h23"].contains(&self.hour_cycle.as_str())
            || self.units.len() != 4
            || self.units.iter().any(String::is_empty)
            || self.am.is_empty()
            || self.pm.is_empty()
            || !self.past.contains("{value}")
            || !self.future.contains("{value}")
            || self.now.is_empty()
        {
            return Err(
                "time requires h12/h23, four unit names, day periods and relative patterns".into(),
            );
        }
        for pattern in [&self.short, &self.medium] {
            if pattern.is_empty()
                || !pattern.contains("mm")
                || !(pattern.contains("HH") || pattern.contains("hh"))
            {
                return Err("time patterns require HH or hh and mm".into());
            }
            if (self.hour_cycle == "h12" && (!pattern.contains("hh") || !pattern.contains('a')))
                || (self.hour_cycle == "h23" && (!pattern.contains("HH") || pattern.contains('a')))
            {
                return Err("time patterns must match the declared hour cycle".into());
            }
            let literal = pattern
                .replace("HH", "")
                .replace("hh", "")
                .replace("mm", "")
                .replace("ss", "")
                .replace('a', "");
            if literal.chars().any(|c| c.is_ascii_alphabetic()) {
                return Err("unsupported time pattern token".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum TimeError {
    #[error("invalid or unavailable timezone `{0}`")]
    Zone(String),
    #[error("instant is outside the supported timestamp range")]
    Instant,
    #[error("invalid local date/time: {0}")]
    Local(String),
    #[error("{0}")]
    Range(String),
}

fn timestamp(ms: i64) -> Result<Timestamp, TimeError> {
    Timestamp::from_millisecond(ms).map_err(|_| TimeError::Instant)
}

#[derive(Clone, Debug, Serialize)]
pub struct TimeParts {
    pub date: String,
    pub time: String,
    pub offset_minutes: i32,
    pub zone: String,
}
/// Convert an instant once through Jiff, never through duplicated civil arithmetic.
///
/// # Errors
/// Returns an invalid instant or timezone error.
pub fn time_parts(ms: i64, zone: &str) -> Result<TimeParts, TimeError> {
    let zoned = timestamp(ms)?.to_zoned(TimeZone::parse(zone)?.resolve()?);
    Ok(TimeParts {
        date: zoned.date().to_string(),
        time: zoned.strftime("%H:%M:%S").to_string(),
        offset_minutes: zoned.offset().seconds() / 60,
        zone: format!("{}", zoned.offset()),
    })
}

/// Return zero (gap), one, or two (fold) candidate instants. No default disambiguation.
///
/// # Errors
/// Returns an invalid local date/time, instant or timezone error.
pub fn local_time_candidates(date: &str, time: &str, zone: &str) -> Result<Vec<i64>, TimeError> {
    crate::GregorianDate::parse_iso(date).map_err(|e| TimeError::Local(e.to_string()))?;
    let dt = format!("{date}T{time}")
        .parse::<jiff::civil::DateTime>()
        .map_err(|e| TimeError::Local(e.to_string()))?;
    let ambiguous = TimeZone::parse(zone)?.resolve()?.to_ambiguous_timestamp(dt);
    let values = match ambiguous.offset() {
        AmbiguousOffset::Gap { .. } => return Ok(vec![]),
        AmbiguousOffset::Fold { .. } => vec![ambiguous.clone().earlier(), ambiguous.later()],
        AmbiguousOffset::Unambiguous { .. } => vec![ambiguous.unambiguous()],
    };
    values
        .into_iter()
        .map(|v| {
            v.map(|v| v.as_millisecond())
                .map_err(|_| TimeError::Instant)
        })
        .collect()
}

fn localize_digits(value: &str, number: &NumberMetadata) -> String {
    value
        .chars()
        .map(|c| {
            c.to_digit(10)
                .map_or_else(|| c.to_string(), |n| number.digits[n as usize].clone())
        })
        .collect()
}
/// Parse localized clock text without guessing a date or zone.
pub(crate) fn parse_time(
    value: &str,
    precision: &str,
    time: &TimeMetadata,
    number: &NumberMetadata,
) -> Option<String> {
    let mut text = value.trim().to_owned();
    for (n, digit) in number.digits.iter().enumerate() {
        text = text.replace(digit, &n.to_string());
    }
    let period = if time.hour_cycle == "h12" {
        if text.ends_with(&time.am) {
            text.truncate(text.len() - time.am.len());
            Some(false)
        } else if text.ends_with(&time.pm) {
            text.truncate(text.len() - time.pm.len());
            Some(true)
        } else {
            return None;
        }
    } else {
        None
    };
    let pieces = text.trim().split(':').collect::<Vec<_>>();
    if pieces.len() != if precision == "second" { 3 } else { 2 } {
        return None;
    }
    if pieces
        .iter()
        .any(|p| p.len() != 2 || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let mut hour = pieces[0].parse::<u8>().ok()?;
    if let Some(pm) = period {
        if !(1..=12).contains(&hour) {
            return None;
        }
        hour = hour % 12 + if pm { 12 } else { 0 };
    }
    let minute = pieces[1].parse::<u8>().ok()?;
    let second = if pieces.len() == 3 {
        pieces[2].parse::<u8>().ok()?
    } else {
        0
    };
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    Some(format!("{hour:02}:{minute:02}:{second:02}"))
}
pub(crate) fn format_time(
    ms: i64,
    style: &str,
    zone: &str,
    time: &TimeMetadata,
    calendar: &CalendarMetadata,
    number: &NumberMetadata,
) -> Result<String, TimeError> {
    let zoned = timestamp(ms)?.to_zoned(TimeZone::parse(zone)?.resolve()?);
    let pattern = match style {
        "short" => &time.short,
        "medium" | "long" => &time.medium,
        _ => {
            return Err(TimeError::Range(
                "time style must be short, medium or long".into(),
            ));
        }
    };
    let hour = zoned.hour();
    let mut text = pattern
        .replace("HH", &format!("{hour:02}"))
        .replace(
            "hh",
            &format!("{:02}", if hour % 12 == 0 { 12 } else { hour % 12 }),
        )
        .replace("mm", &format!("{:02}", zoned.minute()))
        .replace("ss", &format!("{:02}", zoned.second()))
        .replace('a', if hour < 12 { &time.am } else { &time.pm });
    text = localize_digits(&text, number);
    if style == "long" {
        let date = format_date_with_metadata(
            &zoned.date().to_string(),
            DateStyle::Medium,
            calendar,
            number,
        )
        .map_err(|e| TimeError::Local(e.to_string()))?;
        text = format!("{date}, {text} {}", zoned.offset());
    }
    Ok(text)
}
#[must_use]
pub(crate) fn format_duration(ms: i64, time: &TimeMetadata, number: &NumberMetadata) -> String {
    let mut seconds = ms.unsigned_abs() / 1000;
    let mut pieces = Vec::new();
    for (unit, scale) in [86400, 3600, 60, 1].into_iter().enumerate() {
        let value = seconds / scale;
        seconds %= scale;
        if value > 0 || (unit == 3 && pieces.is_empty()) {
            pieces.push(format!(
                "{}{}",
                localize_digits(&value.to_string(), number),
                time.units[unit]
            ));
        }
        if pieces.len() == 2 {
            break;
        }
    }
    format!(
        "{}{}",
        if ms < 0 { &number.minus_sign } else { "" },
        pieces.join(" ")
    )
}
#[must_use]
pub(crate) fn format_relative(
    ms: i64,
    now: i64,
    time: &TimeMetadata,
    number: &NumberMetadata,
) -> String {
    let diff = ms.saturating_sub(now);
    if diff.unsigned_abs() < 1000 {
        return time.now.clone();
    }
    let value = format_duration(diff.saturating_abs(), time, number);
    if diff < 0 { &time.past } else { &time.future }.replace("{value}", &value)
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TimeRange {
    Relative { duration_ms: i64 },
    Absolute { start: i64, end: i64 },
}
impl TimeRange {
    /// Resolve relative durations at query time, with checked integer arithmetic.
    ///
    /// # Errors
    /// Returns an invalid duration, ordering, arithmetic or timestamp-range error.
    pub fn resolve(&self, now_ms: i64) -> Result<(i64, i64), TimeError> {
        let pair = match *self {
            Self::Relative { duration_ms } if duration_ms > 0 => (
                now_ms.checked_sub(duration_ms).ok_or(TimeError::Instant)?,
                now_ms,
            ),
            Self::Relative { .. } => {
                return Err(TimeError::Range("duration_ms must be positive".into()));
            }
            Self::Absolute { start, end } => (start, end),
        };
        if pair.0 >= pair.1 {
            return Err(TimeError::Range("End must be after start".into()));
        }
        timestamp(pair.0)?;
        timestamp(pair.1)?;
        Ok(pair)
    }
    /// Validate range ordering and explicit inclusive bounds, without a wall-clock policy.
    ///
    /// # Errors
    /// Returns invalid bound/span configuration, ordering, instant or range violations.
    pub fn validate(
        &self,
        min: Option<i64>,
        max: Option<i64>,
        max_span: Option<i64>,
    ) -> Result<(), TimeError> {
        if min.zip(max).is_some_and(|(a, b)| a > b) {
            return Err(TimeError::Range("min must not exceed max".into()));
        }
        if max_span.is_some_and(|span| span <= 0) {
            return Err(TimeError::Range("max_span_ms must be positive".into()));
        }
        let span = match *self {
            Self::Relative { duration_ms } => {
                self.resolve(0)?;
                duration_ms
            }
            Self::Absolute { start, end } => {
                self.resolve(0)?;
                if min.is_some_and(|min| start < min) || max.is_some_and(|max| end > max) {
                    return Err(TimeError::Range("Range is outside min/max".into()));
                }
                end.checked_sub(start).ok_or(TimeError::Instant)?
            }
        };
        if max_span.is_some_and(|max| span > max) {
            return Err(TimeError::Range("Range exceeds maximum span".into()));
        }
        Ok(())
    }
}
fn script_error(error: impl fmt::Display) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(
        error.to_string().into(),
        Position::NONE,
    ))
}
fn map_dynamic(value: impl Serialize) -> Result<Map, Box<EvalAltResult>> {
    rhai::serde::to_dynamic(value)
        .map(|v| v.cast::<Map>())
        .map_err(script_error)
}
pub(crate) fn register_time_api(engine: &mut Engine) {
    FuncRegistration::new("time_parts")
        .in_global_namespace()
        .register_into_engine(engine, |ms: i64, zone: ImmutableString| {
            time_parts(ms, &zone)
                .map_err(script_error)
                .and_then(map_dynamic)
        });
    FuncRegistration::new("time_local_candidates")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |date: ImmutableString,
             time: ImmutableString,
             zone: ImmutableString|
             -> Result<rhai::Array, Box<EvalAltResult>> {
                local_time_candidates(&date, &time, &zone)
                    .map(|v| v.into_iter().map(Dynamic::from).collect())
                    .map_err(script_error)
            },
        );
    FuncRegistration::new("time_range_resolve")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |value: Map, now_ms: i64| -> Result<Map, Box<EvalAltResult>> {
                let value: TimeRange =
                    rhai::serde::from_dynamic(&value.into()).map_err(script_error)?;
                let (start, end) = value.resolve(now_ms).map_err(script_error)?;
                Ok(Map::from_iter([
                    ("start".into(), start.into()),
                    ("end".into(), end.into()),
                ]))
            },
        );
    FuncRegistration::new("time_range_error")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |value: Map, min: Dynamic, max: Dynamic, span: Dynamic| -> String {
                if [&min, &max, &span]
                    .iter()
                    .any(|v| !v.is_unit() && !v.is::<i64>())
                {
                    return "bounds and span must be integer milliseconds or ()".into();
                }
                let value = rhai::serde::from_dynamic::<TimeRange>(&value.into());
                match value.map_err(|e| e.to_string()).and_then(|v| {
                    v.validate(min.try_cast(), max.try_cast(), span.try_cast())
                        .map_err(|e| e.to_string())
                }) {
                    Ok(()) => String::new(),
                    Err(error) => error,
                }
            },
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_ranges_and_dst_are_explicit() {
        assert_eq!(WallClock::fixed(123).now_ms(), 123);
        assert!(
            local_time_candidates("2026-03-08", "02:30:00", "America/New_York")
                .unwrap()
                .is_empty()
        );
        let fold = local_time_candidates("2026-11-01", "01:30:00", "America/New_York").unwrap();
        assert_eq!(fold.len(), 2);
        assert_eq!(fold[1] - fold[0], 3_600_000);
        assert_eq!(
            time_parts(fold[0], "America/New_York")
                .unwrap()
                .offset_minutes,
            -240
        );
        assert_eq!(
            time_parts(fold[1], "America/New_York")
                .unwrap()
                .offset_minutes,
            -300
        );
        assert_eq!(
            local_time_candidates("2026-10-14", "08:00:00", "offset:480").unwrap(),
            local_time_candidates("2026-10-14", "00:00:00", "UTC").unwrap()
        );
        let range = TimeRange::Relative {
            duration_ms: 900000,
        };
        assert_eq!(range.resolve(1_000_000).unwrap(), (100000, 1000000));
        assert_eq!(range.resolve(2_000_000).unwrap(), (1100000, 2000000));
        assert!(range.validate(None, None, Some(1000)).is_err());
        assert!(
            TimeRange::Absolute {
                start: 0,
                end: 1000
            }
            .validate(None, Some(999), None)
            .is_err()
        );
        assert!(
            TimeRange::Absolute {
                start: 0,
                end: 1000
            }
            .validate(None, None, None)
            .is_ok()
        );
        assert!(range.resolve(i64::MIN).is_err());
        assert!(
            TimeRange::Absolute {
                start: 1000,
                end: 1000
            }
            .resolve(0)
            .is_err()
        );
    }

    #[test]
    fn default_calendar_uses_wall_instant_and_explicit_calendar_override_wins() {
        let clock = WallClock::fixed(0);
        let calendar = crate::CalendarClock::default();
        assert_eq!(
            calendar.presentation_today(&clock).unwrap().to_iso(),
            time_parts(0, "local").unwrap().date
        );
        assert_eq!(time_parts(0, "UTC").unwrap().date, "1970-01-01");
        assert_eq!(time_parts(0, "offset:-60").unwrap().date, "1969-12-31");
        let fixed =
            crate::CalendarClock::fixed(crate::GregorianDate::parse_iso("2026-10-14").unwrap());
        assert_eq!(
            fixed.presentation_today(&clock).unwrap().to_iso(),
            "2026-10-14"
        );
    }
}
