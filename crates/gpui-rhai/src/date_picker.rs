use std::collections::BTreeSet;

use crate::{AssetId, CalendarMetadata, GregorianDate, NumberMetadata, OverlayPlacement};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatePickerPreset {
    pub label: String,
    pub value: GregorianDate,
    pub disabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatePickerCell {
    pub date: Option<GregorianDate>,
    pub states: BTreeSet<DatePickerCellState>,
}

impl DatePickerCell {
    #[must_use]
    pub fn has(&self, state: DatePickerCellState) -> bool {
        self.states.contains(&state)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DatePickerCellState {
    OutsideMonth,
    Today,
    Selected,
    Disabled,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DatePickerNodeSpec {
    pub id: String,
    pub parent_overlay: Option<String>,
    pub value: Option<GregorianDate>,
    pub min_date: Option<GregorianDate>,
    pub max_date: Option<GregorianDate>,
    pub today: GregorianDate,
    pub display_value: String,
    pub placeholder: String,
    pub open_label: String,
    pub previous_label: String,
    pub next_label: String,
    pub clear_label: String,
    pub calendar: CalendarMetadata,
    pub number: NumberMetadata,
    pub presets: Vec<DatePickerPreset>,
    pub clearable: bool,
    pub disabled: bool,
    pub placement: OverlayPlacement,
    pub cell_size: f64,
    pub trigger_height: f64,
    pub panel_width: f64,
    pub overlay_gap: f64,
    pub previous_asset: AssetId,
    pub next_asset: AssetId,
    pub trigger_asset: AssetId,
    pub clear_asset: AssetId,
}

impl DatePickerNodeSpec {
    /// Validate cross-field date and metric invariants.
    ///
    /// # Errors
    ///
    /// Returns a stable diagnostic for an inverted range, an out-of-range
    /// controlled value, duplicate presets, or invalid geometry.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("DatePicker ID cannot be empty".to_owned());
        }
        if self
            .min_date
            .zip(self.max_date)
            .is_some_and(|(min, max)| min > max)
        {
            return Err("DatePicker min_date cannot be after max_date".to_owned());
        }
        if self.value.is_some_and(|value| !self.date_enabled(value)) {
            return Err("DatePicker controlled value is outside min_date/max_date".to_owned());
        }
        let mut values = std::collections::BTreeSet::new();
        if [
            &self.open_label,
            &self.previous_label,
            &self.next_label,
            &self.clear_label,
        ]
        .into_iter()
        .any(|label| label.trim().is_empty())
        {
            return Err("DatePicker semantic labels cannot be empty".to_owned());
        }
        for preset in &self.presets {
            if preset.label.trim().is_empty() {
                return Err("DatePicker preset labels cannot be empty".to_owned());
            }
            if !values.insert(preset.value) {
                return Err(format!(
                    "DatePicker preset date `{}` is duplicated",
                    preset.value
                ));
            }
        }
        if ![self.cell_size, self.trigger_height, self.panel_width]
            .into_iter()
            .all(|value| value.is_finite() && value > 0.0)
        {
            return Err("DatePicker geometry must be finite and positive".to_owned());
        }
        if !self.overlay_gap.is_finite() || self.overlay_gap < 0.0 {
            return Err("DatePicker overlay gap must be finite and non-negative".to_owned());
        }
        Ok(())
    }

    #[must_use]
    pub fn date_enabled(&self, date: GregorianDate) -> bool {
        self.min_date.is_none_or(|min| date >= min) && self.max_date.is_none_or(|max| date <= max)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DatePickerKey {
    PreviousDay,
    NextDay,
    PreviousWeek,
    NextWeek,
    WeekStart,
    WeekEnd,
    PreviousMonth,
    NextMonth,
    PreviousYear,
    NextYear,
    Commit,
    Escape,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DatePickerOutcome {
    pub open_changed: bool,
    pub value_changed: bool,
}

#[derive(Clone, Debug)]
pub struct DatePickerState {
    spec: DatePickerNodeSpec,
    open: bool,
    visible_month: GregorianDate,
    focused: GregorianDate,
}

impl DatePickerState {
    /// Create state after validating the public spec.
    ///
    /// # Errors
    ///
    /// Returns the same cross-field diagnostics as [`DatePickerNodeSpec`].
    pub fn new(spec: DatePickerNodeSpec) -> Result<Self, String> {
        spec.validate()?;
        let anchor = spec.value.unwrap_or(spec.today);
        let focused = clamp_enabled(&spec, anchor);
        Ok(Self {
            visible_month: first_of_month(focused),
            focused,
            spec,
            open: false,
        })
    }

    /// Synchronize controlled and locale data while retaining an active
    /// browsing month only while the panel remains open.
    ///
    /// # Errors
    ///
    /// Returns public spec validation errors.
    pub fn synchronize(&mut self, spec: DatePickerNodeSpec) -> Result<bool, String> {
        spec.validate()?;
        let changed = self.spec != spec;
        let anchor_changed = self.spec.value != spec.value || self.spec.today != spec.today;
        self.spec = spec;
        if anchor_changed || !self.open {
            let anchor = self.spec.value.unwrap_or(self.spec.today);
            self.focused = clamp_enabled(&self.spec, anchor);
            self.visible_month = first_of_month(self.focused);
        } else {
            self.focused = clamp_enabled(&self.spec, self.focused);
            if !month_has_enabled_date(&self.spec, self.visible_month) {
                self.visible_month = first_of_month(self.focused);
            }
        }
        Ok(changed)
    }

    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.open
    }

    #[must_use]
    pub const fn focused(&self) -> GregorianDate {
        self.focused
    }

    #[must_use]
    pub const fn visible_month(&self) -> GregorianDate {
        self.visible_month
    }

    #[must_use]
    pub fn value(&self) -> Option<GregorianDate> {
        self.spec.value
    }

    pub fn set_open(&mut self, open: bool) -> bool {
        if self.open == open {
            return false;
        }
        self.open = open;
        if open {
            let anchor = self.spec.value.unwrap_or(self.spec.today);
            self.focused = clamp_enabled(&self.spec, anchor);
            self.visible_month = first_of_month(self.focused);
        }
        true
    }

    pub fn clear(&mut self) -> DatePickerOutcome {
        let value_changed = self.spec.value.take().is_some();
        let open_changed = self.set_open(false);
        DatePickerOutcome {
            open_changed,
            value_changed,
        }
    }

    pub fn select(&mut self, date: GregorianDate) -> DatePickerOutcome {
        if !self.spec.date_enabled(date) {
            return DatePickerOutcome::default();
        }
        let value_changed = self.spec.value != Some(date);
        self.spec.value = Some(date);
        self.focused = date;
        self.visible_month = first_of_month(date);
        let open_changed = self.set_open(false);
        DatePickerOutcome {
            open_changed,
            value_changed,
        }
    }

    /// Apply one calendar keyboard command.
    ///
    /// # Errors
    ///
    /// Returns date arithmetic errors only at the supported year boundary.
    pub fn handle_key(&mut self, key: DatePickerKey) -> Result<DatePickerOutcome, String> {
        match key {
            DatePickerKey::PreviousDay => self.move_days(-1)?,
            DatePickerKey::NextDay => self.move_days(1)?,
            DatePickerKey::PreviousWeek => self.move_days(-7)?,
            DatePickerKey::NextWeek => self.move_days(7)?,
            DatePickerKey::WeekStart => self.move_week_edge(false)?,
            DatePickerKey::WeekEnd => self.move_week_edge(true)?,
            DatePickerKey::PreviousMonth => self.move_months(-1)?,
            DatePickerKey::NextMonth => self.move_months(1)?,
            DatePickerKey::PreviousYear => self.move_months(-12)?,
            DatePickerKey::NextYear => self.move_months(12)?,
            DatePickerKey::Commit => return Ok(self.select(self.focused)),
            DatePickerKey::Escape => {
                return Ok(DatePickerOutcome {
                    open_changed: self.set_open(false),
                    value_changed: false,
                });
            }
        }
        Ok(DatePickerOutcome::default())
    }

    /// Move the browsed month when its range intersects enabled dates.
    ///
    /// # Errors
    ///
    /// Returns date arithmetic errors at the supported year boundary.
    pub fn move_visible_month(&mut self, months: i32) -> Result<bool, String> {
        let candidate = self
            .visible_month
            .checked_add_months(months)
            .map_err(|error| error.to_string())?;
        let candidate = first_of_month(candidate);
        if !month_has_enabled_date(&self.spec, candidate) {
            return Ok(false);
        }
        self.visible_month = candidate;
        let max_day = GregorianDate::days_in_month(candidate.year(), candidate.month())
            .map_err(|error| error.to_string())?;
        self.focused = clamp_enabled(
            &self.spec,
            GregorianDate::new(
                candidate.year(),
                candidate.month(),
                self.focused.day().min(max_day),
            )
            .map_err(|error| error.to_string())?,
        );
        Ok(true)
    }

    #[must_use]
    pub fn can_move_visible_month(&self, months: i32) -> bool {
        self.visible_month
            .checked_add_months(months)
            .ok()
            .map(first_of_month)
            .is_some_and(|month| month_has_enabled_date(&self.spec, month))
    }

    /// Return the stable 42-cell month grid. Positions outside the absolute
    /// four-digit ISO range are disabled blanks.
    #[must_use]
    pub fn cells(&self) -> Vec<DatePickerCell> {
        let weekday = self.visible_month.weekday().sunday_index();
        let first = self.spec.calendar.first_weekday.sunday_index();
        let leading = (weekday + 7 - first) % 7;
        (0..42)
            .map(|offset| {
                let delta = i64::from(offset) - i64::try_from(leading).unwrap_or(0);
                let date = self.visible_month.checked_add_days(delta).ok();
                let mut states = BTreeSet::new();
                if date.is_none_or(|date| {
                    date.month() != self.visible_month.month()
                        || date.year() != self.visible_month.year()
                }) {
                    states.insert(DatePickerCellState::OutsideMonth);
                }
                if date == Some(self.spec.today) {
                    states.insert(DatePickerCellState::Today);
                }
                if date == self.spec.value {
                    states.insert(DatePickerCellState::Selected);
                }
                if date.is_none_or(|date| !self.spec.date_enabled(date)) {
                    states.insert(DatePickerCellState::Disabled);
                }
                DatePickerCell { date, states }
            })
            .collect()
    }

    #[must_use]
    pub fn presets(&self) -> &[DatePickerPreset] {
        &self.spec.presets
    }

    #[must_use]
    pub fn spec(&self) -> &DatePickerNodeSpec {
        &self.spec
    }

    fn move_days(&mut self, days: i64) -> Result<(), String> {
        let date = self
            .focused
            .checked_add_days(days)
            .map_err(|error| error.to_string())?;
        self.focused = clamp_enabled(&self.spec, date);
        self.visible_month = first_of_month(self.focused);
        Ok(())
    }

    fn move_months(&mut self, months: i32) -> Result<(), String> {
        let date = self
            .focused
            .checked_add_months(months)
            .map_err(|error| error.to_string())?;
        self.focused = clamp_enabled(&self.spec, date);
        self.visible_month = first_of_month(self.focused);
        Ok(())
    }

    fn move_week_edge(&mut self, end: bool) -> Result<(), String> {
        let weekday = self.focused.weekday().sunday_index();
        let first = self.spec.calendar.first_weekday.sunday_index();
        let from_start = (weekday + 7 - first) % 7;
        let signed = if end {
            i64::try_from(6 - from_start).unwrap_or(0)
        } else {
            -i64::try_from(from_start).unwrap_or(0)
        };
        self.move_days(signed)
    }
}

fn first_of_month(date: GregorianDate) -> GregorianDate {
    GregorianDate::new(date.year(), date.month(), 1).expect("validated date has valid month")
}

fn clamp_enabled(spec: &DatePickerNodeSpec, date: GregorianDate) -> GregorianDate {
    if let Some(min) = spec.min_date
        && date < min
    {
        return min;
    }
    if let Some(max) = spec.max_date
        && date > max
    {
        return max;
    }
    date
}

fn month_has_enabled_date(spec: &DatePickerNodeSpec, month: GregorianDate) -> bool {
    let first = first_of_month(month);
    let last = GregorianDate::days_in_month(first.year(), first.month())
        .ok()
        .and_then(|day| GregorianDate::new(first.year(), first.month(), day).ok());
    last.is_some_and(|last| {
        spec.min_date.is_none_or(|min| last >= min) && spec.max_date.is_none_or(|max| first <= max)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CalendarNames, DatePatterns, Weekday};

    fn calendar(first_weekday: Weekday) -> CalendarMetadata {
        CalendarMetadata {
            first_weekday,
            months: CalendarNames {
                short: (1..=12).map(|month| format!("M{month}")).collect(),
                long: (1..=12).map(|month| format!("Month {month}")).collect(),
            },
            weekdays: CalendarNames {
                short: (0..7).map(|day| format!("D{day}")).collect(),
                long: (0..7).map(|day| format!("Day {day}")).collect(),
            },
            date_patterns: DatePatterns {
                month_year: "MMMM yyyy".to_owned(),
                short: "yyyy-MM-dd".to_owned(),
                medium: "yyyy-MM-dd".to_owned(),
                long: "yyyy-MM-dd".to_owned(),
            },
        }
    }

    fn number() -> NumberMetadata {
        NumberMetadata {
            digits: (0..10).map(|digit| digit.to_string()).collect(),
            decimal_separator: ".".to_owned(),
            grouping_separator: ",".to_owned(),
            primary_group_size: 3,
            secondary_group_size: 3,
            minus_sign: "-".to_owned(),
        }
    }

    fn spec(value: Option<&str>, first_weekday: Weekday) -> DatePickerNodeSpec {
        DatePickerNodeSpec {
            id: "appointment".to_owned(),
            parent_overlay: None,
            value: value.map(|value| GregorianDate::parse_iso(value).unwrap()),
            min_date: Some(GregorianDate::parse_iso("2024-01-10").unwrap()),
            max_date: Some(GregorianDate::parse_iso("2025-02-20").unwrap()),
            today: GregorianDate::parse_iso("2024-02-15").unwrap(),
            display_value: String::new(),
            placeholder: "Date".to_owned(),
            open_label: "Open calendar".to_owned(),
            previous_label: "Previous month".to_owned(),
            next_label: "Next month".to_owned(),
            clear_label: "Clear date".to_owned(),
            calendar: calendar(first_weekday),
            number: number(),
            presets: Vec::new(),
            clearable: true,
            disabled: false,
            placement: OverlayPlacement::Bottom,
            cell_size: 32.0,
            trigger_height: 32.0,
            panel_width: 280.0,
            overlay_gap: 4.0,
            previous_asset: AssetId::parse("app/icons/date_previous").unwrap(),
            next_asset: AssetId::parse("app/icons/date_next").unwrap(),
            trigger_asset: AssetId::parse("app/icons/calendar").unwrap(),
            clear_asset: AssetId::parse("app/icons/close").unwrap(),
        }
    }

    #[test]
    fn month_grid_is_always_42_cells_and_respects_week_start() {
        let mut sunday = DatePickerState::new(spec(Some("2024-02-15"), Weekday::Sunday)).unwrap();
        sunday.set_open(true);
        let cells = sunday.cells();
        assert_eq!(cells.len(), 42);
        assert_eq!(cells[0].date.unwrap().to_iso(), "2024-01-28");

        let mut monday = DatePickerState::new(spec(Some("2024-02-15"), Weekday::Monday)).unwrap();
        monday.set_open(true);
        assert_eq!(monday.cells()[0].date.unwrap().to_iso(), "2024-01-29");

        let mut boundary = DatePickerState::new(spec(Some("2024-02-15"), Weekday::Sunday)).unwrap();
        boundary.visible_month = GregorianDate::parse_iso("0001-01-01").unwrap();
        let cells = boundary.cells();
        assert_eq!(cells.len(), 42);
        assert!(cells.iter().any(|cell| cell.date.is_none()));
    }

    #[test]
    fn keyboard_crosses_month_year_and_clamps_range() {
        let mut state = DatePickerState::new(spec(Some("2024-12-31"), Weekday::Monday)).unwrap();
        state.set_open(true);
        state.handle_key(DatePickerKey::NextDay).unwrap();
        assert_eq!(state.focused().to_iso(), "2025-01-01");
        state.handle_key(DatePickerKey::NextYear).unwrap();
        assert_eq!(state.focused().to_iso(), "2025-02-20");
    }

    #[test]
    fn selecting_commits_and_closes_while_escape_does_not_commit() {
        let mut state = DatePickerState::new(spec(None, Weekday::Sunday)).unwrap();
        state.set_open(true);
        let outcome = state.select(GregorianDate::parse_iso("2024-03-01").unwrap());
        assert_eq!(
            outcome,
            DatePickerOutcome {
                open_changed: true,
                value_changed: true,
            }
        );
        assert_eq!(state.value().unwrap().to_iso(), "2024-03-01");
        assert!(!state.is_open());

        state.set_open(true);
        let outcome = state.handle_key(DatePickerKey::Escape).unwrap();
        assert!(outcome.open_changed);
        assert!(!outcome.value_changed);
        assert_eq!(state.value().unwrap().to_iso(), "2024-03-01");
    }

    #[test]
    fn month_navigation_disables_outside_range() {
        let mut state = DatePickerState::new(spec(Some("2024-01-10"), Weekday::Sunday)).unwrap();
        state.set_open(true);
        assert!(!state.can_move_visible_month(-1));
        assert!(state.can_move_visible_month(1));
        assert!(!state.move_visible_month(-1).unwrap());
    }

    #[test]
    fn external_clock_and_range_changes_resynchronize_open_browsing_state() {
        let mut state = DatePickerState::new(spec(None, Weekday::Sunday)).unwrap();
        state.set_open(true);
        state.move_visible_month(5).unwrap();

        let mut changed = state.spec().clone();
        changed.today = GregorianDate::parse_iso("2024-04-03").unwrap();
        state.synchronize(changed).unwrap();
        assert_eq!(state.focused().to_iso(), "2024-04-03");
        assert_eq!(state.visible_month().to_iso(), "2024-04-01");

        let mut changed = state.spec().clone();
        changed.min_date = Some(GregorianDate::parse_iso("2024-09-15").unwrap());
        state.synchronize(changed).unwrap();
        assert_eq!(state.focused().to_iso(), "2024-09-15");
        assert_eq!(state.visible_month().to_iso(), "2024-09-01");
    }
}
