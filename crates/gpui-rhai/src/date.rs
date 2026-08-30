use std::fmt;
use std::rc::Rc;

use rhai::{
    Array, Dynamic, Engine, EvalAltResult, FuncRegistration, ImmutableString, Map, Position,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GregorianDate {
    year: i32,
    month: u8,
    day: u8,
}

impl GregorianDate {
    /// Construct a Gregorian date in the supported four-digit ISO range.
    ///
    /// # Errors
    ///
    /// Returns [`DateError`] for an unsupported year/month or a day outside the
    /// selected month.
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, DateError> {
        if !(1..=9_999).contains(&year) {
            return Err(DateError::Year(year));
        }
        if !(1..=12).contains(&month) {
            return Err(DateError::Month(month));
        }
        let max = Self::days_in_month(year, month)?;
        if !(1..=max).contains(&day) {
            return Err(DateError::Day { year, month, day });
        }
        Ok(Self { year, month, day })
    }

    /// Parse exactly `YYYY-MM-DD`.
    ///
    /// # Errors
    ///
    /// Returns [`DateError`] for malformed text or an invalid calendar date.
    pub fn parse_iso(value: &str) -> Result<Self, DateError> {
        let bytes = value.as_bytes();
        if bytes.len() != 10
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || !bytes
                .iter()
                .enumerate()
                .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
        {
            return Err(DateError::Format(value.to_owned()));
        }
        let year = parse_ascii_number(&bytes[0..4]);
        let month = parse_ascii_number(&bytes[5..7]);
        let day = parse_ascii_number(&bytes[8..10]);
        Self::new(
            i32::from(year),
            u8::try_from(month).unwrap_or(u8::MAX),
            u8::try_from(day).unwrap_or(u8::MAX),
        )
    }

    #[must_use]
    pub const fn year(self) -> i32 {
        self.year
    }

    #[must_use]
    pub const fn month(self) -> u8 {
        self.month
    }

    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }

    #[must_use]
    pub const fn is_leap_year(year: i32) -> bool {
        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }

    /// Return the number of days in a Gregorian month.
    ///
    /// # Errors
    ///
    /// Returns [`DateError`] for an unsupported year/month.
    pub fn days_in_month(year: i32, month: u8) -> Result<u8, DateError> {
        if !(1..=9_999).contains(&year) {
            return Err(DateError::Year(year));
        }
        let days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if Self::is_leap_year(year) => 29,
            2 => 28,
            _ => return Err(DateError::Month(month)),
        };
        Ok(days)
    }

    #[must_use]
    pub fn weekday(self) -> Weekday {
        Weekday::from_sunday_index(days_from_civil(self.year, self.month, self.day) + 4)
    }

    /// Add signed calendar days.
    ///
    /// # Errors
    ///
    /// Returns [`DateError::ArithmeticRange`] when the result leaves years
    /// 0001..=9999.
    pub fn checked_add_days(self, days: i64) -> Result<Self, DateError> {
        let serial = days_from_civil(self.year, self.month, self.day)
            .checked_add(days)
            .ok_or(DateError::ArithmeticRange)?;
        civil_from_days(serial).ok_or(DateError::ArithmeticRange)
    }

    /// Add signed calendar months, clamping the day into the target month.
    ///
    /// # Errors
    ///
    /// Returns [`DateError::ArithmeticRange`] when the result leaves years
    /// 0001..=9999.
    pub fn checked_add_months(self, months: i32) -> Result<Self, DateError> {
        let index = self
            .year
            .checked_mul(12)
            .and_then(|value| value.checked_add(i32::from(self.month) - 1))
            .and_then(|value| value.checked_add(months))
            .ok_or(DateError::ArithmeticRange)?;
        let year = index.div_euclid(12);
        let month =
            u8::try_from(index.rem_euclid(12) + 1).map_err(|_| DateError::ArithmeticRange)?;
        if !(1..=9_999).contains(&year) {
            return Err(DateError::ArithmeticRange);
        }
        let day = self.day.min(Self::days_in_month(year, month)?);
        Self::new(year, month, day)
    }

    #[must_use]
    pub fn to_iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl fmt::Display for GregorianDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_iso())
    }
}

fn parse_ascii_number(bytes: &[u8]) -> u16 {
    bytes.iter().fold(0_u16, |value, byte| {
        value
            .saturating_mul(10)
            .saturating_add(u16::from(*byte - b'0'))
    })
}

// Howard Hinnant's civil calendar transform, shifted so 1970-01-01 is day 0.
fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let mut year = i64::from(year);
    let month = i64::from(month);
    year -= i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> Option<GregorianDate> {
    let days = days.checked_add(719_468)?;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    GregorianDate::new(
        i32::try_from(year).ok()?,
        u8::try_from(month).ok()?,
        u8::try_from(day).ok()?,
    )
    .ok()
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Weekday {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl Weekday {
    #[must_use]
    pub const fn sunday_index(self) -> usize {
        match self {
            Self::Sunday => 0,
            Self::Monday => 1,
            Self::Tuesday => 2,
            Self::Wednesday => 3,
            Self::Thursday => 4,
            Self::Friday => 5,
            Self::Saturday => 6,
        }
    }

    #[must_use]
    pub fn from_sunday_index(index: i64) -> Self {
        match index.rem_euclid(7) {
            0 => Self::Sunday,
            1 => Self::Monday,
            2 => Self::Tuesday,
            3 => Self::Wednesday,
            4 => Self::Thursday,
            5 => Self::Friday,
            _ => Self::Saturday,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sunday => "sunday",
            Self::Monday => "monday",
            Self::Tuesday => "tuesday",
            Self::Wednesday => "wednesday",
            Self::Thursday => "thursday",
            Self::Friday => "friday",
            Self::Saturday => "saturday",
        }
    }

    /// Parse the stable lowercase weekday vocabulary used by locale metadata.
    ///
    /// # Errors
    ///
    /// Returns [`DateError::Weekday`] for another value.
    pub fn parse(value: &str) -> Result<Self, DateError> {
        match value {
            "sunday" => Ok(Self::Sunday),
            "monday" => Ok(Self::Monday),
            "tuesday" => Ok(Self::Tuesday),
            "wednesday" => Ok(Self::Wednesday),
            "thursday" => Ok(Self::Thursday),
            "friday" => Ok(Self::Friday),
            "saturday" => Ok(Self::Saturday),
            _ => Err(DateError::Weekday(value.to_owned())),
        }
    }
}

pub(crate) fn register_date_api(engine: &mut Engine) {
    FuncRegistration::new("date_info")
        .in_global_namespace()
        .register_into_engine(engine, date_info);
    FuncRegistration::new("date_month_start")
        .in_global_namespace()
        .register_into_engine(engine, date_month_start);
    FuncRegistration::new("date_checked_add_days")
        .in_global_namespace()
        .register_into_engine(engine, date_checked_add_days);
    FuncRegistration::new("date_checked_add_months")
        .in_global_namespace()
        .register_into_engine(engine, date_checked_add_months);
    FuncRegistration::new("date_week_edge")
        .in_global_namespace()
        .register_into_engine(engine, date_week_edge);
    FuncRegistration::new("date_month_grid")
        .in_global_namespace()
        .register_into_engine(engine, date_month_grid);
    FuncRegistration::new("date_clamp")
        .in_global_namespace()
        .register_into_engine(engine, date_clamp);
    FuncRegistration::new("date_month_intersects")
        .in_global_namespace()
        .register_into_engine(engine, date_month_intersects);
}

fn date_info(value: ImmutableString) -> Result<Map, Box<EvalAltResult>> {
    let value: String = value.into();
    let date = parse_script_date(&value)?;
    Ok(Map::from_iter([
        ("iso".into(), Dynamic::from(date.to_iso())),
        ("year".into(), Dynamic::from_int(i64::from(date.year()))),
        ("month".into(), Dynamic::from_int(i64::from(date.month()))),
        ("day".into(), Dynamic::from_int(i64::from(date.day()))),
        (
            "weekday".into(),
            Dynamic::from(date.weekday().as_str().to_owned()),
        ),
    ]))
}

fn date_month_start(value: ImmutableString) -> Result<ImmutableString, Box<EvalAltResult>> {
    let value: String = value.into();
    let date = parse_script_date(&value)?;
    Ok(GregorianDate::new(date.year(), date.month(), 1)
        .expect("parsed date has a valid month")
        .to_iso()
        .into())
}

fn date_checked_add_days(
    value: ImmutableString,
    days: rhai::INT,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let value: String = value.into();
    let date = parse_script_date(&value)?;
    Ok(date
        .checked_add_days(days)
        .map_or(Dynamic::UNIT, |date| Dynamic::from(date.to_iso())))
}

fn date_checked_add_months(
    value: ImmutableString,
    months: rhai::INT,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let value: String = value.into();
    let date = parse_script_date(&value)?;
    let months = i32::try_from(months)
        .map_err(|_| Box::new(date_script_error(&DateError::ArithmeticRange)))?;
    Ok(date
        .checked_add_months(months)
        .map_or(Dynamic::UNIT, |date| Dynamic::from(date.to_iso())))
}

fn date_week_edge(
    value: ImmutableString,
    first_weekday: ImmutableString,
    end: bool,
) -> Result<Dynamic, Box<EvalAltResult>> {
    let value: String = value.into();
    let first_weekday: String = first_weekday.into();
    let date = parse_script_date(&value)?;
    let first =
        Weekday::parse(&first_weekday).map_err(|error| Box::new(date_script_error(&error)))?;
    let weekday = date.weekday().sunday_index();
    let from_start = (weekday + 7 - first.sunday_index()) % 7;
    let delta = if end {
        i64::try_from(6 - from_start).unwrap_or(0)
    } else {
        -i64::try_from(from_start).unwrap_or(0)
    };
    Ok(date
        .checked_add_days(delta)
        .map_or(Dynamic::UNIT, |date| Dynamic::from(date.to_iso())))
}

fn date_month_grid(
    value: ImmutableString,
    first_weekday: ImmutableString,
) -> Result<Array, Box<EvalAltResult>> {
    let value: String = value.into();
    let first_weekday: String = first_weekday.into();
    let date = parse_script_date(&value)?;
    let month =
        GregorianDate::new(date.year(), date.month(), 1).expect("parsed date has a valid month");
    let first =
        Weekday::parse(&first_weekday).map_err(|error| Box::new(date_script_error(&error)))?;
    let leading = (month.weekday().sunday_index() + 7 - first.sunday_index()) % 7;
    Ok((0_i64..42)
        .map(|offset| {
            let date = month.checked_add_days(offset - i64::try_from(leading).unwrap_or(0));
            let (iso, day, outside, weekday) = date.map_or_else(
                |_| (Dynamic::UNIT, Dynamic::UNIT, true, Dynamic::UNIT),
                |date| {
                    (
                        Dynamic::from(date.to_iso()),
                        Dynamic::from_int(i64::from(date.day())),
                        date.year() != month.year() || date.month() != month.month(),
                        Dynamic::from(date.weekday().as_str().to_owned()),
                    )
                },
            );
            Dynamic::from_map(Map::from_iter([
                ("date".into(), iso),
                ("day".into(), day),
                ("outside".into(), Dynamic::from_bool(outside)),
                ("weekday".into(), weekday),
            ]))
        })
        .collect())
}

fn date_clamp(
    value: ImmutableString,
    min: Dynamic,
    max: Dynamic,
) -> Result<ImmutableString, Box<EvalAltResult>> {
    let value: String = value.into();
    let date = parse_script_date(&value)?;
    let min = optional_script_date(min, "min")?;
    let max = optional_script_date(max, "max")?;
    validate_script_range(min, max)?;
    Ok(min
        .filter(|min| date < *min)
        .or_else(|| max.filter(|max| date > *max))
        .unwrap_or(date)
        .to_iso()
        .into())
}

fn date_month_intersects(
    value: ImmutableString,
    min: Dynamic,
    max: Dynamic,
) -> Result<bool, Box<EvalAltResult>> {
    let value: String = value.into();
    let date = parse_script_date(&value)?;
    let first =
        GregorianDate::new(date.year(), date.month(), 1).expect("parsed date has a valid month");
    let last = GregorianDate::new(
        date.year(),
        date.month(),
        GregorianDate::days_in_month(date.year(), date.month()).expect("parsed valid month"),
    )
    .expect("last day is valid");
    let min = optional_script_date(min, "min")?;
    let max = optional_script_date(max, "max")?;
    validate_script_range(min, max)?;
    Ok(min.is_none_or(|min| last >= min) && max.is_none_or(|max| first <= max))
}

fn parse_script_date(value: &str) -> Result<GregorianDate, Box<EvalAltResult>> {
    GregorianDate::parse_iso(value).map_err(|error| Box::new(date_script_error(&error)))
}

fn optional_script_date(
    value: Dynamic,
    name: &str,
) -> Result<Option<GregorianDate>, Box<EvalAltResult>> {
    if value.is_unit() {
        return Ok(None);
    }
    value
        .try_cast::<ImmutableString>()
        .ok_or_else(|| {
            Box::new(date_runtime_error(format!(
                "date range `{name}` must be a string or ()"
            )))
        })
        .and_then(|value| parse_script_date(value.as_str()).map(Some))
}

fn validate_script_range(
    min: Option<GregorianDate>,
    max: Option<GregorianDate>,
) -> Result<(), Box<EvalAltResult>> {
    if min.zip(max).is_some_and(|(min, max)| min > max) {
        Err(Box::new(date_runtime_error(
            "date range min cannot be after max",
        )))
    } else {
        Ok(())
    }
}

fn date_script_error(error: &DateError) -> EvalAltResult {
    date_runtime_error(error.to_string())
}

fn date_runtime_error(message: impl Into<String>) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(message.into().into(), Position::NONE)
}

pub trait CalendarClockSource: fmt::Debug {
    fn today(&self) -> GregorianDate;
}

#[derive(Clone)]
pub struct CalendarClock(Rc<dyn CalendarClockSource>);

impl CalendarClock {
    #[must_use]
    pub fn system() -> Self {
        Self(Rc::new(SystemCalendarClock))
    }

    #[must_use]
    pub fn fixed(date: GregorianDate) -> Self {
        Self(Rc::new(FixedCalendarClock(date)))
    }

    #[must_use]
    pub fn from_source(source: impl CalendarClockSource + 'static) -> Self {
        Self(Rc::new(source))
    }

    #[must_use]
    pub fn today(&self) -> GregorianDate {
        self.0.today()
    }
}

impl Default for CalendarClock {
    fn default() -> Self {
        Self::system()
    }
}

impl fmt::Debug for CalendarClock {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("CalendarClock")
            .field(&self.0)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct SystemCalendarClock;

impl CalendarClockSource for SystemCalendarClock {
    fn today(&self) -> GregorianDate {
        let date = jiff::Zoned::now().date();
        GregorianDate::new(
            i32::from(date.year()),
            u8::try_from(date.month()).expect("Jiff months fit u8"),
            u8::try_from(date.day()).expect("Jiff days fit u8"),
        )
        .expect("Jiff system dates fit the supported Gregorian range")
    }
}

#[derive(Clone, Copy, Debug)]
struct FixedCalendarClock(GregorianDate);

impl CalendarClockSource for FixedCalendarClock {
    fn today(&self) -> GregorianDate {
        self.0
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum DateError {
    #[error("date `{0}` must use strict YYYY-MM-DD format")]
    Format(String),
    #[error("Gregorian year {0} must be between 1 and 9999")]
    Year(i32),
    #[error("Gregorian month {0} must be between 1 and 12")]
    Month(u8),
    #[error("day {day} is invalid for {year:04}-{month:02}")]
    Day { year: i32, month: u8, day: u8 },
    #[error("date arithmetic left the supported years 0001 through 9999")]
    ArithmeticRange,
    #[error("weekday `{0}` is not in the stable lowercase weekday vocabulary")]
    Weekday(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_iso_and_leap_year_validation() {
        assert_eq!(
            GregorianDate::parse_iso("2024-02-29").unwrap().to_iso(),
            "2024-02-29"
        );
        assert!(GregorianDate::parse_iso("2023-02-29").is_err());
        assert!(GregorianDate::parse_iso("2024-2-29").is_err());
        assert!(GregorianDate::parse_iso("0000-01-01").is_err());
        assert!(GregorianDate::parse_iso("10000-01-01").is_err());
    }

    #[test]
    fn arithmetic_crosses_month_year_and_century_boundaries() {
        assert_eq!(
            GregorianDate::parse_iso("2024-12-31")
                .unwrap()
                .checked_add_days(1)
                .unwrap()
                .to_iso(),
            "2025-01-01"
        );
        assert_eq!(
            GregorianDate::parse_iso("2024-01-31")
                .unwrap()
                .checked_add_months(1)
                .unwrap()
                .to_iso(),
            "2024-02-29"
        );
        assert_eq!(
            GregorianDate::parse_iso("2100-01-31")
                .unwrap()
                .checked_add_months(1)
                .unwrap()
                .to_iso(),
            "2100-02-28"
        );
        let epoch = GregorianDate::parse_iso("1970-01-01").unwrap();
        assert_eq!(
            epoch.checked_add_days(i64::MAX),
            Err(DateError::ArithmeticRange)
        );
        assert_eq!(
            epoch.checked_add_days(i64::MIN),
            Err(DateError::ArithmeticRange)
        );
    }

    #[test]
    fn weekdays_match_known_dates() {
        assert_eq!(
            GregorianDate::parse_iso("1970-01-01").unwrap().weekday(),
            Weekday::Thursday
        );
        assert_eq!(
            GregorianDate::parse_iso("2024-02-29").unwrap().weekday(),
            Weekday::Thursday
        );
    }

    #[test]
    fn fixed_clock_is_deterministic() {
        let date = GregorianDate::parse_iso("2026-08-29").unwrap();
        assert_eq!(CalendarClock::fixed(date).today(), date);
    }

    #[test]
    fn rhai_date_data_api_is_checked_and_month_grid_is_stable() {
        let runtime = crate::RuntimeEngine::new();
        let info = runtime
            .engine()
            .eval::<Map>(r#"date_info("2024-02-29")"#)
            .unwrap();
        assert_eq!(info["weekday"].clone_cast::<ImmutableString>(), "thursday");
        let grid = runtime
            .engine()
            .eval::<Array>(r#"date_month_grid("2024-02-15", "sunday")"#)
            .unwrap();
        assert_eq!(grid.len(), 42);
        let first = grid[0].clone_cast::<Map>();
        assert_eq!(first["date"].clone_cast::<ImmutableString>(), "2024-01-28");
        assert!(first["outside"].clone_cast::<bool>());
        assert!(
            runtime
                .engine()
                .eval::<Dynamic>(r#"date_checked_add_days("9999-12-31", 1)"#)
                .unwrap()
                .is_unit()
        );
        assert!(
            runtime
                .engine()
                .eval::<bool>(r#"date_month_intersects("2024-02-01", "2024-02-10", "2024-02-20")"#)
                .unwrap()
        );
        assert!(
            runtime
                .engine()
                .eval::<Map>(r#"date_info("2023-02-29")"#)
                .is_err()
        );
    }
}
