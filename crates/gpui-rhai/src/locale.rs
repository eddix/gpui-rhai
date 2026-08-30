use std::collections::{BTreeMap, BTreeSet};

use rhai::{Dynamic, Engine, Scope};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ComponentInstancePath, DateError, GregorianDate, Weekday};

const REQUIRED_MESSAGES: &[&str] = &[
    "calendar.clear",
    "calendar.next_month",
    "calendar.open",
    "calendar.previous_month",
    "common.clear",
    "common.close",
    "common.loading",
    "common.no_results",
    "pagination.items",
    "pagination.next",
    "pagination.page",
    "pagination.per_page",
    "pagination.previous",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CalendarNames {
    pub short: Vec<String>,
    pub long: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatePatterns {
    pub month_year: String,
    pub short: String,
    pub medium: String,
    pub long: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CalendarMetadata {
    pub first_weekday: Weekday,
    pub months: CalendarNames,
    pub weekdays: CalendarNames,
    pub date_patterns: DatePatterns,
}

impl CalendarMetadata {
    fn validate(&self) -> Result<(), LocaleError> {
        validate_names("calendar.months.short", &self.months.short, 12)?;
        validate_names("calendar.months.long", &self.months.long, 12)?;
        validate_names("calendar.weekdays.short", &self.weekdays.short, 7)?;
        validate_names("calendar.weekdays.long", &self.weekdays.long, 7)?;
        for (name, pattern) in [
            ("month_year", &self.date_patterns.month_year),
            ("short", &self.date_patterns.short),
            ("medium", &self.date_patterns.medium),
            ("long", &self.date_patterns.long),
        ] {
            validate_date_pattern(pattern).map_err(|message| LocaleError::InvalidCalendar {
                field: format!("calendar.date_patterns.{name}"),
                message,
            })?;
        }
        let month_year = &self.date_patterns.month_year;
        if !month_year.contains("yyyy")
            || !month_year.contains('M')
            || month_year.contains('d')
            || month_year.contains('E')
        {
            return Err(LocaleError::InvalidCalendar {
                field: "calendar.date_patterns.month_year".to_owned(),
                message: "month_year must contain year and month tokens only".to_owned(),
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NumberMetadata {
    pub digits: Vec<String>,
    pub decimal_separator: String,
    pub grouping_separator: String,
    pub primary_group_size: u8,
    pub secondary_group_size: u8,
    pub minus_sign: String,
}

impl NumberMetadata {
    fn validate(&self) -> Result<(), LocaleError> {
        validate_names("number.digits", &self.digits, 10).map_err(|error| match error {
            LocaleError::InvalidCalendar { field, message } => {
                LocaleError::InvalidNumber { field, message }
            }
            other => other,
        })?;
        for (field, value) in [
            ("number.decimal_separator", &self.decimal_separator),
            ("number.grouping_separator", &self.grouping_separator),
            ("number.minus_sign", &self.minus_sign),
        ] {
            if value.is_empty() {
                return Err(LocaleError::InvalidNumber {
                    field: field.to_owned(),
                    message: "value cannot be empty".to_owned(),
                });
            }
        }
        if self.decimal_separator == self.grouping_separator {
            return Err(LocaleError::InvalidNumber {
                field: "number.grouping_separator".to_owned(),
                message: "grouping and decimal separators must differ".to_owned(),
            });
        }
        for (field, size) in [
            ("number.primary_group_size", self.primary_group_size),
            ("number.secondary_group_size", self.secondary_group_size),
        ] {
            if !(1..=9).contains(&size) {
                return Err(LocaleError::InvalidNumber {
                    field: field.to_owned(),
                    message: "group size must be between 1 and 9".to_owned(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LocaleBundle {
    pub locale: String,
    pub direction: TextDirection,
    pub messages: BTreeMap<String, String>,
    pub calendar: CalendarMetadata,
    pub number: NumberMetadata,
}

impl LocaleBundle {
    /// Validate locale identity, internal messages, calendar data, and number
    /// presentation metadata.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError`] for an incomplete or malformed bundle.
    pub fn validate(&self) -> Result<(), LocaleError> {
        if self.locale.trim().is_empty() {
            return Err(LocaleError::EmptyLocale);
        }
        let missing = REQUIRED_MESSAGES
            .iter()
            .filter(|key| !self.messages.contains_key(**key))
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(LocaleError::MissingMessages {
                locale: self.locale.clone(),
                missing,
            });
        }
        if let Some((key, _)) = self
            .messages
            .iter()
            .find(|(key, value)| key.trim().is_empty() || value.is_empty())
        {
            return Err(LocaleError::InvalidMessage(key.clone()));
        }
        self.calendar.validate()?;
        self.number.validate()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DateStyle {
    Short,
    Medium,
    Long,
}

impl DateStyle {
    /// Parse the stable public date presentation vocabulary.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError::UnknownDateStyle`] for another value.
    pub fn parse(value: &str) -> Result<Self, LocaleError> {
        match value {
            "short" => Ok(Self::Short),
            "medium" => Ok(Self::Medium),
            "long" => Ok(Self::Long),
            _ => Err(LocaleError::UnknownDateStyle(value.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NumberFormatOptions {
    pub min_fraction_digits: u8,
    pub max_fraction_digits: u8,
    pub grouping: bool,
}

impl NumberFormatOptions {
    /// Validate fraction digit ordering and the deliberately bounded formatter
    /// surface.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError::InvalidNumberOptions`] above twelve digits or for
    /// an inverted min/max pair.
    pub fn validate(self) -> Result<(), LocaleError> {
        if self.max_fraction_digits > 12 || self.min_fraction_digits > self.max_fraction_digits {
            Err(LocaleError::InvalidNumberOptions)
        } else {
            Ok(())
        }
    }
}

impl Default for NumberFormatOptions {
    fn default() -> Self {
        Self {
            min_fraction_digits: 0,
            max_fraction_digits: 3,
            grouping: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LocaleManager {
    bundles: BTreeMap<String, LocaleBundle>,
    fallback: String,
    app: String,
    windows: BTreeMap<String, String>,
    scopes: BTreeMap<ComponentInstancePath, String>,
    generation: u64,
}

impl LocaleManager {
    /// Create a locale manager from validated bundles.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError`] for invalid, duplicate, or unknown defaults.
    pub fn new(
        bundles: impl IntoIterator<Item = LocaleBundle>,
        fallback: impl Into<String>,
        app: impl Into<String>,
    ) -> Result<Self, LocaleError> {
        let mut map = BTreeMap::new();
        for bundle in bundles {
            bundle.validate()?;
            if map.insert(bundle.locale.clone(), bundle).is_some() {
                return Err(LocaleError::DuplicateLocale);
            }
        }
        let manager = Self {
            bundles: map,
            fallback: fallback.into(),
            app: app.into(),
            windows: BTreeMap::new(),
            scopes: BTreeMap::new(),
            generation: 1,
        };
        manager.require_locale(&manager.fallback)?;
        manager.require_locale(&manager.app)?;
        Ok(manager)
    }

    /// Change the app locale without touching component state.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError::UnknownLocale`] for absent bundles.
    pub fn set_app(&mut self, locale: impl Into<String>) -> Result<(), LocaleError> {
        let locale = locale.into();
        self.require_locale(&locale)?;
        if self.app != locale {
            self.app = locale;
            self.generation = self.generation.saturating_add(1);
        }
        Ok(())
    }

    /// Set a window locale override.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError::UnknownLocale`] for absent bundles.
    pub fn set_window(
        &mut self,
        window: impl Into<String>,
        locale: impl Into<String>,
    ) -> Result<(), LocaleError> {
        let locale = locale.into();
        self.require_locale(&locale)?;
        self.windows.insert(window.into(), locale);
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }

    pub fn remove_window(&mut self, window: &str) -> bool {
        let removed = self.windows.remove(window).is_some();
        if removed {
            self.generation = self.generation.saturating_add(1);
        }
        removed
    }

    pub fn remove_scope(&mut self, scope: &ComponentInstancePath) -> bool {
        let previous = self.scopes.len();
        self.scopes.retain(|path, _| !path.is_within(scope));
        let removed = self.scopes.len() != previous;
        if removed {
            self.generation = self.generation.saturating_add(1);
        }
        removed
    }

    /// Set a component-subtree locale override.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError::UnknownLocale`] for absent bundles.
    pub fn set_scope(
        &mut self,
        scope: ComponentInstancePath,
        locale: impl Into<String>,
    ) -> Result<(), LocaleError> {
        let locale = locale.into();
        self.require_locale(&locale)?;
        self.scopes.insert(scope, locale);
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }

    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    #[must_use]
    pub fn app_locale(&self) -> &str {
        &self.app
    }

    /// Resolve a message using nearest scope and fallback bundle.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError::UnknownMessage`] when neither selected nor
    /// fallback bundle defines the key.
    pub fn text(
        &self,
        window: Option<&str>,
        component: Option<&ComponentInstancePath>,
        key: &str,
    ) -> Result<String, LocaleError> {
        let locale = self.selected_locale(window, component);
        self.bundles
            .get(locale)
            .and_then(|bundle| bundle.messages.get(key))
            .or_else(|| {
                self.bundles
                    .get(&self.fallback)
                    .and_then(|bundle| bundle.messages.get(key))
            })
            .cloned()
            .ok_or_else(|| LocaleError::UnknownMessage(key.to_owned()))
    }

    /// Resolve selected text direction.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError`] if internal selection data is invalid.
    pub fn direction(
        &self,
        window: Option<&str>,
        component: Option<&ComponentInstancePath>,
    ) -> Result<TextDirection, LocaleError> {
        Ok(self.selected_bundle(window, component)?.direction)
    }

    /// Resolve immutable calendar metadata for the selected scope.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError`] if internal selection data is invalid.
    pub fn calendar(
        &self,
        window: Option<&str>,
        component: Option<&ComponentInstancePath>,
    ) -> Result<&CalendarMetadata, LocaleError> {
        Ok(&self.selected_bundle(window, component)?.calendar)
    }

    /// Resolve immutable number metadata for the selected scope.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError`] if internal selection data is invalid.
    pub fn number(
        &self,
        window: Option<&str>,
        component: Option<&ComponentInstancePath>,
    ) -> Result<&NumberMetadata, LocaleError> {
        Ok(&self.selected_bundle(window, component)?.number)
    }

    /// Format a strict ISO Gregorian date through the selected locale.
    ///
    /// # Errors
    ///
    /// Returns date, pattern, or locale-selection errors.
    pub fn format_date(
        &self,
        window: Option<&str>,
        component: Option<&ComponentInstancePath>,
        iso_date: &str,
        style: DateStyle,
    ) -> Result<String, LocaleError> {
        let bundle = self.selected_bundle(window, component)?;
        format_date_with_metadata(iso_date, style, &bundle.calendar, &bundle.number)
    }

    /// Format an integer without lossy float conversion.
    ///
    /// # Errors
    ///
    /// Returns invalid options or locale-selection errors.
    pub fn format_integer(
        &self,
        window: Option<&str>,
        component: Option<&ComponentInstancePath>,
        value: i64,
        options: NumberFormatOptions,
    ) -> Result<String, LocaleError> {
        let bundle = self.selected_bundle(window, component)?;
        format_integer_with_metadata(value, options, &bundle.number)
    }

    /// Format a finite decimal number through the selected locale.
    ///
    /// # Errors
    ///
    /// Returns invalid options, non-finite input, or locale-selection errors.
    pub fn format_number(
        &self,
        window: Option<&str>,
        component: Option<&ComponentInstancePath>,
        value: f64,
        options: NumberFormatOptions,
    ) -> Result<String, LocaleError> {
        let bundle = self.selected_bundle(window, component)?;
        format_number_with_metadata(value, options, &bundle.number)
    }

    fn selected_bundle(
        &self,
        window: Option<&str>,
        component: Option<&ComponentInstancePath>,
    ) -> Result<&LocaleBundle, LocaleError> {
        let locale = self.selected_locale(window, component);
        self.bundles
            .get(locale)
            .ok_or_else(|| LocaleError::UnknownLocale(locale.to_owned()))
    }

    fn selected_locale<'a>(
        &'a self,
        window: Option<&str>,
        component: Option<&ComponentInstancePath>,
    ) -> &'a str {
        component
            .and_then(|component| self.nearest_scope(component))
            .or_else(|| window.and_then(|window| self.windows.get(window)))
            .map_or(self.app.as_str(), String::as_str)
    }

    fn nearest_scope(&self, component: &ComponentInstancePath) -> Option<&String> {
        let mut current = Some(component.clone());
        while let Some(path) = current {
            if let Some(locale) = self.scopes.get(&path) {
                return Some(locale);
            }
            current = path.parent();
        }
        None
    }

    fn require_locale(&self, locale: &str) -> Result<(), LocaleError> {
        if self.bundles.contains_key(locale) {
            Ok(())
        } else {
            Err(LocaleError::UnknownLocale(locale.to_owned()))
        }
    }
}

fn validate_names(field: &str, values: &[String], expected: usize) -> Result<(), LocaleError> {
    if values.len() != expected {
        return Err(LocaleError::InvalidCalendar {
            field: field.to_owned(),
            message: format!("expected {expected} entries, got {}", values.len()),
        });
    }
    let mut unique = BTreeSet::new();
    if let Some(value) = values
        .iter()
        .find(|value| value.is_empty() || !unique.insert(value.as_str()))
    {
        return Err(LocaleError::InvalidCalendar {
            field: field.to_owned(),
            message: if value.is_empty() {
                "entries cannot be empty".to_owned()
            } else {
                format!("entry `{value}` is duplicated")
            },
        });
    }
    Ok(())
}

const DATE_TOKENS: &[&str] = &["MMMM", "yyyy", "EEEE", "MMM", "EEE", "MM", "dd", "M", "d"];

fn validate_date_pattern(pattern: &str) -> Result<(), String> {
    if pattern.is_empty() {
        return Err("pattern cannot be empty".to_owned());
    }
    let mut remaining = pattern;
    while !remaining.is_empty() {
        if let Some(token) = DATE_TOKENS
            .iter()
            .find(|token| remaining.starts_with(**token))
        {
            remaining = &remaining[token.len()..];
            continue;
        }
        let character = remaining.chars().next().expect("remaining is non-empty");
        if character.is_ascii_alphabetic() {
            return Err(format!(
                "unsupported pattern token beginning with `{character}`"
            ));
        }
        remaining = &remaining[character.len_utf8()..];
    }
    Ok(())
}

fn render_date_pattern(
    date: GregorianDate,
    pattern: &str,
    calendar: &CalendarMetadata,
    number: &NumberMetadata,
) -> Result<String, LocaleError> {
    let mut output = String::new();
    let mut remaining = pattern;
    while !remaining.is_empty() {
        let Some(token) = DATE_TOKENS
            .iter()
            .find(|token| remaining.starts_with(**token))
            .copied()
        else {
            let character = remaining.chars().next().expect("remaining is non-empty");
            output.push(character);
            remaining = &remaining[character.len_utf8()..];
            continue;
        };
        let ascii = match token {
            "yyyy" => format!("{:04}", date.year()),
            "MM" => format!("{:02}", date.month()),
            "M" => date.month().to_string(),
            "dd" => format!("{:02}", date.day()),
            "d" => date.day().to_string(),
            "MMM" => calendar.months.short[usize::from(date.month() - 1)].clone(),
            "MMMM" => calendar.months.long[usize::from(date.month() - 1)].clone(),
            "EEE" => calendar.weekdays.short[date.weekday().sunday_index()].clone(),
            "EEEE" => calendar.weekdays.long[date.weekday().sunday_index()].clone(),
            _ => return Err(LocaleError::InvalidDatePattern(pattern.to_owned())),
        };
        if matches!(token, "yyyy" | "MM" | "M" | "dd" | "d") {
            output.push_str(&localize_ascii_digits(&ascii, &number.digits));
        } else {
            output.push_str(&ascii);
        }
        remaining = &remaining[token.len()..];
    }
    Ok(output)
}

fn render_number_parts(
    negative: bool,
    integer: &str,
    fraction: &str,
    number: &NumberMetadata,
    options: NumberFormatOptions,
) -> String {
    let integer = if options.grouping {
        group_ascii_digits(
            integer,
            usize::from(number.primary_group_size),
            usize::from(number.secondary_group_size),
            &number.grouping_separator,
        )
    } else {
        integer.to_owned()
    };
    let mut output = String::new();
    if negative {
        output.push_str(&number.minus_sign);
    }
    output.push_str(&localize_ascii_digits(&integer, &number.digits));
    if !fraction.is_empty() {
        output.push_str(&number.decimal_separator);
        output.push_str(&localize_ascii_digits(fraction, &number.digits));
    }
    output
}

fn group_ascii_digits(integer: &str, primary: usize, secondary: usize, separator: &str) -> String {
    if integer.len() <= primary {
        return integer.to_owned();
    }
    let mut groups = Vec::new();
    let mut end = integer.len();
    let mut size = primary;
    while end > 0 {
        let start = end.saturating_sub(size);
        groups.push(&integer[start..end]);
        end = start;
        size = secondary;
    }
    groups.reverse();
    groups.join(separator)
}

fn localize_ascii_digits(value: &str, digits: &[String]) -> String {
    let mut output = String::new();
    for character in value.chars() {
        if let Some(index) = character.to_digit(10) {
            output.push_str(&digits[usize::try_from(index).expect("decimal digit fits usize")]);
        } else {
            output.push(character);
        }
    }
    output
}

/// Format one strict ISO date with already validated locale metadata.
///
/// # Errors
///
/// Returns strict date or pattern errors.
pub fn format_date_with_metadata(
    iso_date: &str,
    style: DateStyle,
    calendar: &CalendarMetadata,
    number: &NumberMetadata,
) -> Result<String, LocaleError> {
    let date = GregorianDate::parse_iso(iso_date)?;
    let pattern = match style {
        DateStyle::Short => &calendar.date_patterns.short,
        DateStyle::Medium => &calendar.date_patterns.medium,
        DateStyle::Long => &calendar.date_patterns.long,
    };
    render_date_pattern(date, pattern, calendar, number)
}

pub(crate) fn format_month_year_with_metadata(
    date: GregorianDate,
    calendar: &CalendarMetadata,
    number: &NumberMetadata,
) -> Result<String, LocaleError> {
    render_date_pattern(date, &calendar.date_patterns.month_year, calendar, number)
}

/// Format one integer with already validated locale metadata.
///
/// # Errors
///
/// Returns invalid formatter options.
pub fn format_integer_with_metadata(
    value: i64,
    options: NumberFormatOptions,
    number: &NumberMetadata,
) -> Result<String, LocaleError> {
    options.validate()?;
    Ok(render_number_parts(
        value.is_negative(),
        &value.unsigned_abs().to_string(),
        "",
        number,
        options,
    ))
}

/// Format one finite decimal with already validated locale metadata.
///
/// # Errors
///
/// Returns invalid formatter options or non-finite input.
pub fn format_number_with_metadata(
    value: f64,
    options: NumberFormatOptions,
    number: &NumberMetadata,
) -> Result<String, LocaleError> {
    options.validate()?;
    if !value.is_finite() {
        return Err(LocaleError::NonFiniteNumber);
    }
    let rendered = format!(
        "{:.*}",
        usize::from(options.max_fraction_digits),
        value.abs()
    );
    let (integer, mut fraction) = rendered
        .split_once('.')
        .map_or((rendered.as_str(), String::new()), |(integer, fraction)| {
            (integer, fraction.to_owned())
        });
    while fraction.len() > usize::from(options.min_fraction_digits) && fraction.ends_with('0') {
        fraction.pop();
    }
    Ok(render_number_parts(
        value.is_sign_negative() && value != 0.0,
        integer,
        &fraction,
        number,
        options,
    ))
}

/// Compile and evaluate `locale() -> map` from Rhai source.
///
/// # Errors
///
/// Returns script, decode, or semantic validation errors.
pub fn load_locale_source(
    engine: &Engine,
    source_name: &str,
    source: &str,
) -> Result<LocaleBundle, LocaleError> {
    let mut ast = engine
        .compile(source)
        .map_err(|error| LocaleError::Script(error.to_string()))?;
    ast.set_source(source_name);
    let raw: Dynamic = engine
        .call_fn(&mut Scope::new(), &ast, "locale", ())
        .map_err(|error| LocaleError::Script(error.to_string()))?;
    let bundle = rhai::serde::from_dynamic::<LocaleBundle>(&raw)
        .map_err(|error| LocaleError::Decode(error.to_string()))?;
    bundle.validate()?;
    Ok(bundle)
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum LocaleError {
    #[error("locale ID cannot be empty")]
    EmptyLocale,
    #[error("locale bundle is duplicated")]
    DuplicateLocale,
    #[error("locale `{locale}` is missing messages: {missing:?}")]
    MissingMessages {
        locale: String,
        missing: Vec<String>,
    },
    #[error("locale message `{0}` has an empty key or value")]
    InvalidMessage(String),
    #[error("invalid {field}: {message}")]
    InvalidCalendar { field: String, message: String },
    #[error("invalid {field}: {message}")]
    InvalidNumber { field: String, message: String },
    #[error("locale `{0}` is not loaded")]
    UnknownLocale(String),
    #[error("locale message `{0}` is not defined")]
    UnknownMessage(String),
    #[error("date style `{0}` is unknown")]
    UnknownDateStyle(String),
    #[error("date pattern `{0}` could not be rendered")]
    InvalidDatePattern(String),
    #[error("number format options require 0 <= min_fraction_digits <= max_fraction_digits <= 12")]
    InvalidNumberOptions,
    #[error("cannot format a non-finite number")]
    NonFiniteNumber,
    #[error(transparent)]
    Date(#[from] DateError),
    #[error("locale script failed: {0}")]
    Script(String),
    #[error("locale source could not be decoded: {0}")]
    Decode(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calendar() -> CalendarMetadata {
        CalendarMetadata {
            first_weekday: Weekday::Sunday,
            months: CalendarNames {
                short: [
                    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
                    "Dec",
                ]
                .map(str::to_owned)
                .to_vec(),
                long: [
                    "January",
                    "February",
                    "March",
                    "April",
                    "May",
                    "June",
                    "July",
                    "August",
                    "September",
                    "October",
                    "November",
                    "December",
                ]
                .map(str::to_owned)
                .to_vec(),
            },
            weekdays: CalendarNames {
                short: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
                    .map(str::to_owned)
                    .to_vec(),
                long: [
                    "Sunday",
                    "Monday",
                    "Tuesday",
                    "Wednesday",
                    "Thursday",
                    "Friday",
                    "Saturday",
                ]
                .map(str::to_owned)
                .to_vec(),
            },
            date_patterns: DatePatterns {
                month_year: "MMMM yyyy".to_owned(),
                short: "MM/dd/yyyy".to_owned(),
                medium: "MMM d, yyyy".to_owned(),
                long: "EEEE, MMMM d, yyyy".to_owned(),
            },
        }
    }

    fn number() -> NumberMetadata {
        NumberMetadata {
            digits: ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]
                .map(str::to_owned)
                .to_vec(),
            decimal_separator: ".".to_owned(),
            grouping_separator: ",".to_owned(),
            primary_group_size: 3,
            secondary_group_size: 3,
            minus_sign: "-".to_owned(),
        }
    }

    fn bundle(locale: &str, loading: &str, direction: TextDirection) -> LocaleBundle {
        LocaleBundle {
            locale: locale.to_owned(),
            direction,
            messages: BTreeMap::from([
                ("calendar.clear".to_owned(), "Clear date".to_owned()),
                ("calendar.next_month".to_owned(), "Next month".to_owned()),
                ("calendar.open".to_owned(), "Open calendar".to_owned()),
                (
                    "calendar.previous_month".to_owned(),
                    "Previous month".to_owned(),
                ),
                ("common.clear".to_owned(), "Clear".to_owned()),
                ("common.close".to_owned(), "Close".to_owned()),
                ("common.loading".to_owned(), loading.to_owned()),
                ("common.no_results".to_owned(), "No results".to_owned()),
                ("pagination.items".to_owned(), "items".to_owned()),
                ("pagination.next".to_owned(), "Next page".to_owned()),
                ("pagination.page".to_owned(), "Page".to_owned()),
                (
                    "pagination.per_page".to_owned(),
                    "Items per page".to_owned(),
                ),
                ("pagination.previous".to_owned(), "Previous page".to_owned()),
            ]),
            calendar: calendar(),
            number: number(),
        }
    }

    #[test]
    fn scope_precedence_and_fallback_are_deterministic() {
        let mut locales = LocaleManager::new(
            [
                bundle("en", "Loading", TextDirection::LeftToRight),
                bundle("zh-CN", "加载中", TextDirection::LeftToRight),
            ],
            "en",
            "en",
        )
        .unwrap();
        locales.set_window("main", "zh-CN").unwrap();
        let root = ComponentInstancePath::root("App", "root");
        locales.set_scope(root.clone(), "en").unwrap();
        assert_eq!(
            locales
                .text(
                    Some("main"),
                    Some(&root.child("Button", "save")),
                    "common.loading",
                )
                .unwrap(),
            "Loading"
        );
    }

    #[test]
    fn live_locale_switch_advances_only_locale_generation() {
        let mut locales = LocaleManager::new(
            [
                bundle("en", "Loading", TextDirection::LeftToRight),
                bundle("zh-CN", "加载中", TextDirection::LeftToRight),
            ],
            "en",
            "en",
        )
        .unwrap();
        let generation = locales.generation();
        locales.set_app("zh-CN").unwrap();
        assert_eq!(locales.generation(), generation + 1);
    }

    #[test]
    fn rtl_direction_follows_scope_window_and_app_precedence() {
        let mut locales = LocaleManager::new(
            [
                bundle("en", "Loading", TextDirection::LeftToRight),
                bundle("ar", "جارٍ التحميل", TextDirection::RightToLeft),
            ],
            "en",
            "ar",
        )
        .unwrap();
        let root = ComponentInstancePath::root("App", "root");
        assert_eq!(
            locales.direction(Some("main"), Some(&root)).unwrap(),
            TextDirection::RightToLeft
        );
        locales.set_window("main", "en").unwrap();
        assert_eq!(
            locales.direction(Some("main"), Some(&root)).unwrap(),
            TextDirection::LeftToRight
        );
        locales.set_scope(root.clone(), "ar").unwrap();
        assert_eq!(
            locales
                .direction(Some("main"), Some(&root.child("Button", "save")))
                .unwrap(),
            TextDirection::RightToLeft
        );
    }

    #[test]
    fn dates_and_numbers_use_selected_bundle_metadata() {
        let locales = LocaleManager::new(
            [bundle("en", "Loading", TextDirection::LeftToRight)],
            "en",
            "en",
        )
        .unwrap();
        assert_eq!(
            locales
                .format_date(None, None, "2024-02-29", DateStyle::Long)
                .unwrap(),
            "Thursday, February 29, 2024"
        );
        assert_eq!(
            format_month_year_with_metadata(
                GregorianDate::parse_iso("2024-02-29").unwrap(),
                &calendar(),
                &number(),
            )
            .unwrap(),
            "February 2024"
        );
        assert_eq!(
            locales
                .format_number(
                    None,
                    None,
                    -12345.6,
                    NumberFormatOptions {
                        min_fraction_digits: 2,
                        max_fraction_digits: 2,
                        grouping: true,
                    },
                )
                .unwrap(),
            "-12,345.60"
        );
        assert_eq!(
            locales
                .format_integer(None, None, i64::MIN, NumberFormatOptions::default())
                .unwrap(),
            "-9,223,372,036,854,775,808"
        );
    }

    #[test]
    fn month_year_pattern_controls_date_picker_header_order() {
        let engine = crate::RuntimeEngine::new();
        let zh = load_locale_source(
            engine.engine(),
            "zh_cn.rhai",
            include_str!("../../../registry/locales/zh_cn.rhai"),
        )
        .unwrap();
        assert_eq!(
            format_month_year_with_metadata(
                GregorianDate::parse_iso("2024-02-29").unwrap(),
                &zh.calendar,
                &zh.number,
            )
            .unwrap(),
            "2024年二月"
        );
    }

    #[test]
    fn malformed_calendar_and_number_contracts_fail_load() {
        let mut invalid = bundle("en", "Loading", TextDirection::LeftToRight);
        invalid.calendar.months.short.pop();
        assert!(matches!(
            invalid.validate(),
            Err(LocaleError::InvalidCalendar { .. })
        ));
        let mut invalid = bundle("en", "Loading", TextDirection::LeftToRight);
        invalid.calendar.date_patterns.month_year = "MMMM d, yyyy".to_owned();
        assert!(matches!(
            invalid.validate(),
            Err(LocaleError::InvalidCalendar { .. })
        ));
        let mut invalid = bundle("en", "Loading", TextDirection::LeftToRight);
        invalid.number.decimal_separator = ",".to_owned();
        assert!(matches!(
            invalid.validate(),
            Err(LocaleError::InvalidNumber { .. })
        ));
    }
}
