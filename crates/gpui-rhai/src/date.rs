use std::fmt;
use std::rc::Rc;

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
}
