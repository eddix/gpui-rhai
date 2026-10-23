//! Wall-clock instants and timezone conversion, independent of charts and motion.
use std::fmt::Write as _;
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
        for (style, pattern) in [("short", &self.short), ("medium", &self.medium)] {
            if pattern.is_empty()
                || !pattern.contains("mm")
                || !(pattern.contains("HH") || pattern.contains("hh"))
            {
                return Err("time patterns require HH or hh and mm".into());
            }
            let hour_token = if self.hour_cycle == "h12" { "hh" } else { "HH" };
            if pattern.matches(hour_token).count() != 1
                || pattern.matches("mm").count() != 1
                || pattern.matches("ss").count() != usize::from(style == "medium")
            {
                return Err("short time requires hours/minutes; medium additionally requires seconds, each exactly once".into());
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

fn offset_text(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let value = seconds.unsigned_abs();
    let mut text = format!("{sign}{:02}:{:02}", value / 3600, value % 3600 / 60);
    if !value.is_multiple_of(60) {
        let _ = write!(text, ":{:02}", value % 60);
    }
    text
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
        zone: offset_text(zoned.offset().seconds()),
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
        AmbiguousOffset::Fold { .. } => vec![ambiguous.earlier(), ambiguous.later()],
        AmbiguousOffset::Unambiguous { .. } => vec![ambiguous.unambiguous()],
    };
    values
        .into_iter()
        .map(|v| {
            v.map(Timestamp::as_millisecond)
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
        if text.contains(&time.am) {
            text = text.replacen(&time.am, "", 1);
            Some(false)
        } else if text.contains(&time.pm) {
            text = text.replacen(&time.pm, "", 1);
            Some(true)
        } else {
            return None;
        }
    } else {
        None
    };
    let pattern = match precision {
        "minute" => &time.short,
        "second" => &time.medium,
        _ => return None,
    }
    .replace('a', "");
    let mut pattern = pattern.trim();
    let mut input = text.trim();
    let (mut hour, mut minute, mut second) = (None, None, None);
    while !pattern.is_empty() {
        let token = if pattern.starts_with("HH") || pattern.starts_with("hh") {
            Some(0)
        } else if pattern.starts_with("mm") {
            Some(1)
        } else if pattern.starts_with("ss") {
            Some(2)
        } else {
            None
        };
        if let Some(token) = token {
            let digits = input.get(..2)?;
            if !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let value = digits.parse::<u8>().ok()?;
            let field = match token {
                0 => &mut hour,
                1 => &mut minute,
                _ => &mut second,
            };
            if field.replace(value).is_some() {
                return None;
            }
            input = input.get(2..)?;
            pattern = &pattern[2..];
        } else {
            let literal = pattern.chars().next()?;
            input = input.strip_prefix(literal)?;
            pattern = &pattern[literal.len_utf8()..];
        }
    }
    if !input.is_empty() {
        return None;
    }
    let mut hour = hour?;
    let minute = minute?;
    let second = if precision == "second" {
        second?
    } else {
        if second.is_some() {
            return None;
        }
        0
    };
    if let Some(pm) = period {
        if !(1..=12).contains(&hour) {
            return None;
        }
        hour = hour % 12 + if pm { 12 } else { 0 };
    }
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
        text = format!("{date}, {text} {}", offset_text(zoned.offset().seconds()));
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
#[allow(
    clippy::unnecessary_box_returns,
    reason = "Rhai native fallible function ABI uses boxed EvalAltResult"
)]
fn script_error(error: impl fmt::Display) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(
        error.to_string().into(),
        Position::NONE,
    ))
}
fn map_dynamic(value: impl Serialize) -> Result<Map, Box<EvalAltResult>> {
    rhai::serde::to_dynamic(value)
        .map(Dynamic::cast::<Map>)
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
        assert_eq!(
            local_time_candidates("2026-03-08", "02:30:00", "America/New_York").unwrap(),
            Vec::<i64>::new()
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
            time_parts(fold[0], "America/New_York").unwrap().zone,
            "-04:00"
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
            duration_ms: 900_000,
        };
        assert_eq!(range.resolve(1_000_000).unwrap(), (100_000, 1_000_000));
        assert_eq!(range.resolve(2_000_000).unwrap(), (1_100_000, 2_000_000));
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

    #[test]
    fn clock_parser_obeys_locale_pattern_literals_and_day_period_position() {
        let number = NumberMetadata {
            digits: (0..10).map(|n| n.to_string()).collect(),
            decimal_separator: ".".into(),
            grouping_separator: ",".into(),
            primary_group_size: 3,
            secondary_group_size: 3,
            minus_sign: "-".into(),
        };
        let time = TimeMetadata {
            hour_cycle: "h12".into(),
            short: "a hh.mm".into(),
            medium: "a hh.mm.ss".into(),
            ..TimeMetadata::default()
        };
        time.validate().unwrap();
        assert_eq!(
            parse_time("PM 11.59", "minute", &time, &number).as_deref(),
            Some("23:59:00")
        );
        assert_eq!(
            parse_time("AM 12.00.01", "second", &time, &number).as_deref(),
            Some("00:00:01")
        );
        assert!(parse_time("AM 12.00.99", "second", &time, &number).is_none());
        let no_seconds = TimeMetadata {
            medium: "HH:mm".into(),
            ..TimeMetadata::default()
        };
        assert!(no_seconds.validate().is_err());
        let no_seconds_12 = TimeMetadata {
            hour_cycle: "h12".into(),
            short: "hh:mm a".into(),
            medium: "hh:mm a".into(),
            ..TimeMetadata::default()
        };
        assert!(no_seconds_12.validate().is_err());
    }
}
