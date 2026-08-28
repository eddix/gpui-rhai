use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use thiserror::Error;

use crate::{UiNode, VirtualListError, VirtualListMetrics, VirtualListSpec, VirtualListState};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DropdownOption {
    pub value: String,
    pub label: String,
    pub keywords: Vec<String>,
    pub disabled: bool,
}

impl DropdownOption {
    #[must_use]
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            keywords: Vec::new(),
            disabled: false,
        }
    }

    #[must_use]
    pub fn keywords(mut self, keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DropdownMode {
    Single,
    Multiple,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DropdownNodeSpec {
    pub id: String,
    pub parent_overlay: Option<String>,
    pub options: Vec<DropdownOption>,
    pub mode: DropdownMode,
    pub selected: Option<Vec<String>>,
    pub open: Option<bool>,
    pub searchable: bool,
    pub query: Option<String>,
    pub placeholder: String,
    pub search_placeholder: String,
    pub empty_text: String,
    pub trigger_slot: Option<Box<UiNode>>,
    pub header_slot: Option<Box<UiNode>>,
    pub footer_slot: Option<Box<UiNode>>,
    pub empty_slot: Option<Box<UiNode>>,
    pub disabled: bool,
    pub max_visible: usize,
    pub placement: crate::OverlayPlacement,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DropdownKey {
    ArrowUp,
    ArrowDown,
    Home,
    End,
    Enter,
    Escape,
    Character(char),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DropdownOutcome {
    pub selection_changed: bool,
    pub open_changed: bool,
    pub query_changed: bool,
}

#[derive(Clone, Debug)]
pub struct DropdownState {
    options: Vec<DropdownOption>,
    visible: Vec<usize>,
    list: VirtualListState,
    selected: BTreeSet<String>,
    mode: DropdownMode,
    query: String,
    open: bool,
    typeahead: String,
    typeahead_at: Option<Instant>,
    typeahead_timeout: Duration,
}

impl DropdownState {
    /// Create a validated dropdown state from controlled selection data.
    ///
    /// # Errors
    ///
    /// Returns duplicate/empty option, unknown selection, or single-select
    /// cardinality errors.
    pub fn new(
        options: Vec<DropdownOption>,
        mode: DropdownMode,
        selected: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, DropdownError> {
        validate_options(&options)?;
        let selected = selected.into_iter().map(Into::into).collect();
        validate_selection(&options, mode, &selected)?;
        let mut state = Self {
            options,
            visible: Vec::new(),
            list: VirtualListState::default(),
            selected,
            mode,
            query: String::new(),
            open: false,
            typeahead: String::new(),
            typeahead_at: None,
            typeahead_timeout: Duration::from_millis(700),
        };
        state.rebuild_visible()?;
        Ok(state)
    }

    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.open
    }

    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Filter visible options while preserving selection by value.
    ///
    /// # Errors
    ///
    /// Returns virtual-list identity errors if internal option invariants are
    /// broken.
    pub fn set_query(&mut self, query: impl Into<String>) -> Result<bool, DropdownError> {
        let query = query.into();
        if self.query == query {
            return Ok(false);
        }
        self.query = query;
        self.rebuild_visible()?;
        self.focus_initial();
        Ok(true)
    }

    /// Synchronize an externally controlled selection without replacing
    /// keyboard focus or search state.
    ///
    /// # Errors
    ///
    /// Returns unknown selection or single-select cardinality errors.
    pub fn set_controlled_selection(
        &mut self,
        selected: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<bool, DropdownError> {
        let selected = selected.into_iter().map(Into::into).collect();
        validate_selection(&self.options, self.mode, &selected)?;
        if self.selected == selected {
            return Ok(false);
        }
        self.selected = selected;
        Ok(true)
    }

    /// Replace option data and mode while preserving query, open state, and
    /// focus by stable option value whenever possible.
    ///
    /// # Errors
    ///
    /// Returns option or controlled-selection validation errors.
    pub fn synchronize(
        &mut self,
        options: Vec<DropdownOption>,
        mode: DropdownMode,
        selected: Option<&[String]>,
    ) -> Result<bool, DropdownError> {
        validate_options(&options)?;
        let mut next_selected: BTreeSet<String> = selected.map_or_else(
            || {
                self.selected
                    .iter()
                    .filter(|value| options.iter().any(|option| &option.value == *value))
                    .cloned()
                    .collect()
            },
            |values| values.iter().cloned().collect(),
        );
        if selected.is_none() && mode == DropdownMode::Single && next_selected.len() > 1 {
            next_selected = next_selected.into_iter().take(1).collect();
        }
        validate_selection(&options, mode, &next_selected)?;
        let changed =
            self.options != options || self.mode != mode || self.selected != next_selected;
        if !changed {
            return Ok(false);
        }
        let focused = self.list.focused().map(ToOwned::to_owned);
        self.options = options;
        self.mode = mode;
        self.selected = next_selected;
        self.rebuild_visible()?;
        if let Some(focused) = focused
            && self
                .options
                .iter()
                .any(|option| option.value == focused && !option.disabled)
        {
            self.list.focus(&focused)?;
        } else {
            self.focus_initial();
        }
        Ok(true)
    }

    pub fn set_controlled_open(&mut self, open: bool) -> bool {
        if open { self.open() } else { self.close() }
    }

    pub fn open(&mut self) -> bool {
        if self.open {
            return false;
        }
        self.open = true;
        self.focus_initial();
        true
    }

    pub fn close(&mut self) -> bool {
        if !self.open {
            return false;
        }
        self.open = false;
        self.typeahead.clear();
        self.typeahead_at = None;
        true
    }

    #[must_use]
    pub fn selected(&self) -> &BTreeSet<String> {
        &self.selected
    }

    #[must_use]
    pub fn focused(&self) -> Option<&str> {
        self.list.focused()
    }

    #[must_use]
    pub fn focused_visible_index(&self) -> Option<usize> {
        self.list.focused().and_then(|focused| {
            self.visible
                .iter()
                .position(|index| self.options[*index].value == focused)
        })
    }

    #[must_use]
    pub fn visible_options(&self) -> impl ExactSizeIterator<Item = &DropdownOption> {
        self.visible.iter().map(|index| &self.options[*index])
    }

    /// Return only rows within the equal-height virtual realization range.
    ///
    /// # Errors
    ///
    /// Returns viewport validation errors from [`VirtualListSpec`].
    pub fn realized_options(
        &self,
        spec: VirtualListSpec,
        scroll_offset: f64,
        viewport_height: f64,
    ) -> Result<(VirtualListMetrics, Vec<&DropdownOption>), DropdownError> {
        let metrics = self.list.metrics(spec, scroll_offset, viewport_height)?;
        let options = self.visible[metrics.range.clone()]
            .iter()
            .map(|index| &self.options[*index])
            .collect();
        Ok((metrics, options))
    }

    /// Apply a keyboard command and report only semantic state changes.
    ///
    /// # Errors
    ///
    /// Returns virtual-list identity errors if internal option invariants are
    /// broken.
    pub fn handle_key(
        &mut self,
        key: DropdownKey,
        now: Instant,
    ) -> Result<DropdownOutcome, DropdownError> {
        let mut outcome = DropdownOutcome::default();
        match key {
            DropdownKey::ArrowDown => {
                if self.open {
                    self.move_focus(1)?;
                } else {
                    outcome.open_changed = self.open();
                }
            }
            DropdownKey::ArrowUp => {
                if self.open {
                    self.move_focus(-1)?;
                } else {
                    outcome.open_changed = self.open();
                }
            }
            DropdownKey::Home => self.focus_edge(false)?,
            DropdownKey::End => self.focus_edge(true)?,
            DropdownKey::Enter => {
                if self.open {
                    outcome.selection_changed = self.select_focused();
                    if self.mode == DropdownMode::Single && outcome.selection_changed {
                        outcome.open_changed = self.close();
                    }
                } else {
                    outcome.open_changed = self.open();
                }
            }
            DropdownKey::Escape => outcome.open_changed = self.close(),
            DropdownKey::Character(character) if !character.is_control() => {
                self.typeahead(character, now)?;
            }
            DropdownKey::Character(_) => {}
        }
        Ok(outcome)
    }

    /// Select or toggle a known enabled option by stable value.
    ///
    /// # Errors
    ///
    /// Returns [`DropdownError::UnknownSelection`] or virtual-list identity
    /// errors for unknown values.
    pub fn select_value(&mut self, value: &str) -> Result<DropdownOutcome, DropdownError> {
        let option = self
            .options
            .iter()
            .find(|option| option.value == value)
            .ok_or_else(|| DropdownError::UnknownSelection(value.to_owned()))?;
        if option.disabled {
            return Ok(DropdownOutcome::default());
        }
        self.list.focus(value)?;
        let selection_changed = self.select_focused();
        let open_changed = self.mode == DropdownMode::Single && selection_changed && self.close();
        Ok(DropdownOutcome {
            selection_changed,
            open_changed,
            query_changed: false,
        })
    }

    fn rebuild_visible(&mut self) -> Result<(), DropdownError> {
        let query = self.query.to_lowercase();
        self.visible = self
            .options
            .iter()
            .enumerate()
            .filter(|(_, option)| {
                query.is_empty()
                    || option.label.to_lowercase().contains(&query)
                    || option
                        .keywords
                        .iter()
                        .any(|keyword| keyword.to_lowercase().contains(&query))
            })
            .map(|(index, _)| index)
            .collect();
        self.list.set_keys(
            self.visible
                .iter()
                .map(|index| self.options[*index].value.clone())
                .collect(),
        )?;
        Ok(())
    }

    fn focus_initial(&mut self) {
        let selected = self.selected.iter().find(|value| {
            self.visible.iter().any(|index| {
                let option = &self.options[*index];
                !option.disabled && &option.value == *value
            })
        });
        let candidate = selected.cloned().or_else(|| {
            self.visible
                .iter()
                .map(|index| &self.options[*index])
                .find(|option| !option.disabled)
                .map(|option| option.value.clone())
        });
        if let Some(candidate) = candidate {
            let _ = self.list.focus(&candidate);
        }
    }

    fn focus_edge(&mut self, end: bool) -> Result<(), DropdownError> {
        let mut options = self
            .visible
            .iter()
            .map(|index| &self.options[*index])
            .filter(|option| !option.disabled);
        let option = if end {
            options.next_back()
        } else {
            options.next()
        };
        if let Some(option) = option {
            self.list.focus(&option.value)?;
        }
        Ok(())
    }

    fn move_focus(&mut self, direction: isize) -> Result<(), DropdownError> {
        if self.visible.is_empty() {
            return Ok(());
        }
        let current = self
            .list
            .focused()
            .and_then(|focused| {
                self.visible
                    .iter()
                    .position(|index| self.options[*index].value == focused)
            })
            .unwrap_or(if direction > 0 {
                self.visible.len().saturating_sub(1)
            } else {
                0
            });
        for step in 1..=self.visible.len() {
            let index = if direction > 0 {
                (current + step) % self.visible.len()
            } else {
                (current + self.visible.len() - (step % self.visible.len())) % self.visible.len()
            };
            let option = &self.options[self.visible[index]];
            if !option.disabled {
                self.list.focus(&option.value)?;
                break;
            }
        }
        Ok(())
    }

    fn select_focused(&mut self) -> bool {
        let Some(focused) = self.list.focused() else {
            return false;
        };
        let Some(option) = self
            .options
            .iter()
            .find(|option| option.value == focused && !option.disabled)
        else {
            return false;
        };
        match self.mode {
            DropdownMode::Single => {
                let next = BTreeSet::from([option.value.clone()]);
                if self.selected == next {
                    false
                } else {
                    self.selected = next;
                    true
                }
            }
            DropdownMode::Multiple => {
                if !self.selected.remove(&option.value) {
                    self.selected.insert(option.value.clone());
                }
                true
            }
        }
    }

    fn typeahead(&mut self, character: char, now: Instant) -> Result<(), DropdownError> {
        if self
            .typeahead_at
            .is_none_or(|previous| now.duration_since(previous) > self.typeahead_timeout)
        {
            self.typeahead.clear();
        }
        self.typeahead.extend(character.to_lowercase());
        self.typeahead_at = Some(now);
        let start = self
            .list
            .focused()
            .and_then(|focused| {
                self.visible
                    .iter()
                    .position(|index| self.options[*index].value == focused)
            })
            .map_or(0, |index| index.saturating_add(1));
        for offset in 0..self.visible.len() {
            let index = (start + offset) % self.visible.len();
            let option = &self.options[self.visible[index]];
            if !option.disabled && option.label.to_lowercase().starts_with(&self.typeahead) {
                self.list.focus(&option.value)?;
                break;
            }
        }
        Ok(())
    }
}

fn validate_options(options: &[DropdownOption]) -> Result<(), DropdownError> {
    let mut values = BTreeSet::new();
    for option in options {
        if option.value.trim().is_empty() {
            return Err(DropdownError::EmptyValue);
        }
        if option.label.trim().is_empty() {
            return Err(DropdownError::EmptyLabel(option.value.clone()));
        }
        if !values.insert(option.value.clone()) {
            return Err(DropdownError::DuplicateValue(option.value.clone()));
        }
    }
    Ok(())
}

fn validate_selection(
    options: &[DropdownOption],
    mode: DropdownMode,
    selected: &BTreeSet<String>,
) -> Result<(), DropdownError> {
    if mode == DropdownMode::Single && selected.len() > 1 {
        return Err(DropdownError::MultipleValuesInSingleMode);
    }
    if let Some(value) = selected
        .iter()
        .find(|value| !options.iter().any(|option| &option.value == *value))
    {
        return Err(DropdownError::UnknownSelection(value.clone()));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum DropdownError {
    #[error("dropdown option values cannot be empty")]
    EmptyValue,
    #[error("dropdown option `{0}` has an empty label")]
    EmptyLabel(String),
    #[error("dropdown option value `{0}` is duplicated")]
    DuplicateValue(String),
    #[error("single-select dropdown cannot contain multiple selected values")]
    MultipleValuesInSingleMode,
    #[error("dropdown selection `{0}` does not match an option")]
    UnknownSelection(String),
    #[error(transparent)]
    VirtualList(#[from] VirtualListError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(value: &str) -> DropdownOption {
        DropdownOption::new(value, value.to_uppercase())
    }

    #[test]
    fn keyboard_skips_disabled_and_single_selection_closes() {
        let mut state = DropdownState::new(
            vec![
                option("alpha"),
                option("beta").disabled(true),
                option("gamma"),
            ],
            DropdownMode::Single,
            std::iter::empty::<String>(),
        )
        .unwrap();
        assert!(state.open());
        assert_eq!(state.focused(), Some("alpha"));
        state
            .handle_key(DropdownKey::ArrowDown, Instant::now())
            .unwrap();
        assert_eq!(state.focused(), Some("gamma"));
        let outcome = state
            .handle_key(DropdownKey::Enter, Instant::now())
            .unwrap();
        assert!(outcome.selection_changed && outcome.open_changed);
        assert_eq!(state.selected(), &BTreeSet::from(["gamma".to_owned()]));
        assert!(!state.is_open());
    }

    #[test]
    fn multiple_search_typeahead_and_controlled_sync_are_stable() {
        let mut state = DropdownState::new(
            vec![
                option("alpha").keywords(["first"]),
                option("beta").keywords(["second"]),
                option("gamma").keywords(["third"]),
            ],
            DropdownMode::Multiple,
            ["alpha"],
        )
        .unwrap();
        state.open();
        let now = Instant::now();
        state.handle_key(DropdownKey::Character('g'), now).unwrap();
        assert_eq!(state.focused(), Some("gamma"));
        state.handle_key(DropdownKey::Enter, now).unwrap();
        assert_eq!(
            state.selected(),
            &BTreeSet::from(["alpha".to_owned(), "gamma".to_owned()])
        );
        assert!(state.is_open());
        assert!(state.set_query("second").unwrap());
        assert_eq!(
            state
                .visible_options()
                .map(|option| option.value.as_str())
                .collect::<Vec<_>>(),
            vec!["beta"]
        );
        assert!(state.set_controlled_selection(["beta"]).unwrap());
        assert_eq!(state.selected(), &BTreeSet::from(["beta".to_owned()]));
    }

    #[test]
    fn five_thousand_search_results_remain_bounded() {
        let options = (0..5_000)
            .map(|index| DropdownOption::new(format!("row-{index}"), format!("Row {index}")))
            .collect();
        let state = DropdownState::new(options, DropdownMode::Single, std::iter::empty::<String>())
            .unwrap();
        let (metrics, realized) = state
            .realized_options(VirtualListSpec::new(28.0, 4).unwrap(), 42_000.0, 420.0)
            .unwrap();
        assert_eq!(metrics.item_count, 5_000);
        assert_eq!(metrics.realized_count, realized.len());
        assert!(realized.len() <= 25);
    }
}
