use std::collections::{BTreeMap, BTreeSet};

use crate::{
    AssetId, CalendarMetadata, DateStyle, Length, NumberFormatOptions, NumberMetadata, UiNode,
    UiValue, VirtualListSpec, VirtualListState,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TableColumnWidth {
    Fixed(f64),
    Percent(f64),
    Flex(f64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableAlign {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableCellFormat {
    Text,
    Number(NumberFormatOptions),
    Date(DateStyle),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableColumnSpec {
    pub key: String,
    pub title: String,
    pub width: TableColumnWidth,
    pub align: TableAlign,
    pub format: TableCellFormat,
    pub sortable: bool,
    pub custom_cells: Option<Vec<UiNode>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableRowSpec {
    pub key: String,
    pub values: BTreeMap<String, UiValue>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableSelectionMode {
    None,
    Single,
    Multiple,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableSortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableSort {
    pub key: String,
    pub direction: TableSortDirection,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableNodeSpec {
    pub key: String,
    pub label: String,
    pub columns: Vec<TableColumnSpec>,
    pub rows: Vec<TableRowSpec>,
    pub height: Length,
    pub row_height: f64,
    pub flex_min_width: f64,
    pub selection_width: f64,
    pub selection_size: f64,
    pub horizontal_scrollbar_height: f64,
    pub horizontal_scrollbar_thumb_min_width: f64,
    pub horizontal_scrollbar_inset: f64,
    pub overscan: usize,
    pub loading: bool,
    pub loading_slot: Option<Box<UiNode>>,
    pub loading_rows: Vec<UiNode>,
    pub empty_slot: Option<Box<UiNode>>,
    pub empty_text: String,
    pub striped: bool,
    pub selection_mode: TableSelectionMode,
    pub selected_keys: BTreeSet<String>,
    pub sort: Option<TableSort>,
    pub calendar: CalendarMetadata,
    pub number: NumberMetadata,
    pub check_asset: AssetId,
    pub sort_ascending_asset: AssetId,
    pub sort_descending_asset: AssetId,
}

impl TableNodeSpec {
    /// Validate stable identity, column definitions, controlled sort/selection,
    /// scalar cells, and custom-renderer cardinality.
    ///
    /// # Errors
    ///
    /// Returns a path-aware table diagnostic for the first broken invariant.
    pub fn validate(&self) -> Result<(), TableError> {
        if self.key.trim().is_empty() {
            return Err(TableError::EmptyKey);
        }
        if self.label.trim().is_empty() {
            return Err(TableError::EmptyLabel);
        }
        if self.columns.is_empty() {
            return Err(TableError::NoColumns);
        }
        if !self.row_height.is_finite() || self.row_height <= 0.0 {
            return Err(TableError::InvalidRowHeight(self.row_height));
        }
        if !self.flex_min_width.is_finite() || self.flex_min_width < 0.0 {
            return Err(TableError::InvalidFlexMinimum(self.flex_min_width));
        }
        if !self.selection_width.is_finite()
            || self.selection_width <= 0.0
            || !self.selection_size.is_finite()
            || self.selection_size <= 0.0
            || self.selection_size > self.selection_width
        {
            return Err(TableError::InvalidSelectionGeometry {
                width: self.selection_width,
                size: self.selection_size,
            });
        }
        self.validate_horizontal_scrollbar_geometry()?;
        if self.loading_rows.len() > 100 {
            return Err(TableError::TooManyLoadingRows(self.loading_rows.len()));
        }
        self.height
            .validate()
            .map_err(|error| TableError::InvalidHeight(error.to_string()))?;
        let mut column_keys = BTreeSet::new();
        for column in &self.columns {
            validate_column(column, self.rows.len())?;
            if !column_keys.insert(column.key.clone()) {
                return Err(TableError::DuplicateColumn(column.key.clone()));
            }
        }
        let mut row_keys = BTreeSet::new();
        for row in &self.rows {
            if row.key.trim().is_empty() {
                return Err(TableError::EmptyRowKey);
            }
            if !row_keys.insert(row.key.clone()) {
                return Err(TableError::DuplicateRow(row.key.clone()));
            }
            for column in &self.columns {
                let value = row
                    .values
                    .get(&column.key)
                    .ok_or_else(|| TableError::MissingCell {
                        row: row.key.clone(),
                        column: column.key.clone(),
                    })?;
                if column.custom_cells.is_none() {
                    validate_default_cell(value, column)?;
                }
            }
        }
        if let Some(unknown) = self
            .selected_keys
            .iter()
            .find(|key| !row_keys.contains(*key))
        {
            return Err(TableError::UnknownSelection(unknown.clone()));
        }
        match self.selection_mode {
            TableSelectionMode::None if !self.selected_keys.is_empty() => {
                return Err(TableError::SelectionDisabled);
            }
            TableSelectionMode::Single if self.selected_keys.len() > 1 => {
                return Err(TableError::MultipleSingleSelection);
            }
            _ => {}
        }
        if let Some(sort) = &self.sort {
            let column = self
                .columns
                .iter()
                .find(|column| column.key == sort.key)
                .ok_or_else(|| TableError::UnknownSortColumn(sort.key.clone()))?;
            if !column.sortable {
                return Err(TableError::UnsortableColumn(sort.key.clone()));
            }
        }
        Ok(())
    }

    fn validate_horizontal_scrollbar_geometry(&self) -> Result<(), TableError> {
        if self.horizontal_scrollbar_height.is_finite()
            && self.horizontal_scrollbar_height > 0.0
            && self.horizontal_scrollbar_thumb_min_width.is_finite()
            && self.horizontal_scrollbar_thumb_min_width > 0.0
            && self.horizontal_scrollbar_inset.is_finite()
            && self.horizontal_scrollbar_inset >= 0.0
            && self.horizontal_scrollbar_inset * 2.0 < self.horizontal_scrollbar_height
        {
            Ok(())
        } else {
            Err(TableError::InvalidHorizontalScrollbarGeometry {
                height: self.horizontal_scrollbar_height,
                thumb_min_width: self.horizontal_scrollbar_thumb_min_width,
                inset: self.horizontal_scrollbar_inset,
            })
        }
    }

    #[must_use]
    pub fn row_keys(&self) -> Vec<String> {
        self.rows.iter().map(|row| row.key.clone()).collect()
    }

    /// Format one default scalar cell through the shared locale metadata.
    ///
    /// # Errors
    ///
    /// Returns missing row/column/cell or locale format diagnostics.
    pub fn display_cell(&self, row: usize, column: usize) -> Result<String, TableError> {
        let row = self.rows.get(row).ok_or(TableError::RowIndex(row))?;
        let column = self
            .columns
            .get(column)
            .ok_or(TableError::ColumnIndex(column))?;
        let value = row
            .values
            .get(&column.key)
            .ok_or_else(|| TableError::MissingCell {
                row: row.key.clone(),
                column: column.key.clone(),
            })?;
        match (value, column.format) {
            (UiValue::Null, TableCellFormat::Text) => Ok(String::new()),
            (UiValue::Bool(value), TableCellFormat::Text) => Ok(value.to_string()),
            (UiValue::Integer(value), TableCellFormat::Text) => Ok(value.to_string()),
            (UiValue::Float(value), TableCellFormat::Text) => Ok(value.to_string()),
            (UiValue::String(value), TableCellFormat::Text) => Ok(value.clone()),
            (UiValue::Integer(value), TableCellFormat::Number(options)) => {
                crate::format_integer_with_metadata(*value, options, &self.number)
                    .map_err(|error| TableError::Format(error.to_string()))
            }
            (UiValue::Float(value), TableCellFormat::Number(options)) => {
                crate::format_number_with_metadata(*value, options, &self.number)
                    .map_err(|error| TableError::Format(error.to_string()))
            }
            (UiValue::String(value), TableCellFormat::Date(style)) => {
                crate::format_date_with_metadata(value, style, &self.calendar, &self.number)
                    .map_err(|error| TableError::Format(error.to_string()))
            }
            _ => Err(TableError::InvalidCell {
                column: column.key.clone(),
                expected: format!("{:?}", column.format),
            }),
        }
    }

    /// Resolve fixed, viewport-percent, and weighted-flex tracks.
    ///
    /// # Errors
    ///
    /// Returns [`TableError::InvalidViewportWidth`] for a non-finite or negative
    /// measured viewport.
    pub fn resolve_widths(&self, viewport_width: f64) -> Result<TableLayout, TableError> {
        if !viewport_width.is_finite() || viewport_width < 0.0 {
            return Err(TableError::InvalidViewportWidth(viewport_width));
        }
        let committed = self
            .columns
            .iter()
            .map(|column| match column.width {
                TableColumnWidth::Fixed(value) => value,
                TableColumnWidth::Percent(value) => value * viewport_width,
                TableColumnWidth::Flex(_) => 0.0,
            })
            .sum::<f64>();
        let flex_weight = self
            .columns
            .iter()
            .filter_map(|column| match column.width {
                TableColumnWidth::Flex(weight) => Some(weight),
                _ => None,
            })
            .sum::<f64>();
        let flex_count = self
            .columns
            .iter()
            .filter(|column| matches!(column.width, TableColumnWidth::Flex(_)))
            .count();
        let remaining = (viewport_width - committed).max(0.0);
        let minimum_flex_space =
            self.flex_min_width * u32::try_from(flex_count).map_or(f64::from(u32::MAX), f64::from);
        let flex_space = remaining.max(minimum_flex_space);
        let weighted_space = (flex_space - minimum_flex_space).max(0.0);
        let widths = self
            .columns
            .iter()
            .map(|column| match column.width {
                TableColumnWidth::Fixed(value) => value,
                TableColumnWidth::Percent(value) => value * viewport_width,
                TableColumnWidth::Flex(weight) if flex_weight > 0.0 => {
                    self.flex_min_width + weighted_space * weight / flex_weight
                }
                TableColumnWidth::Flex(_) => 0.0,
            })
            .collect::<Vec<_>>();
        Ok(TableLayout {
            content_width: widths.iter().sum::<f64>().max(viewport_width),
            widths,
        })
    }
}

fn validate_column(column: &TableColumnSpec, row_count: usize) -> Result<(), TableError> {
    if column.key.trim().is_empty() {
        return Err(TableError::EmptyColumnKey);
    }
    let width = match column.width {
        TableColumnWidth::Fixed(value) | TableColumnWidth::Flex(value) => value,
        TableColumnWidth::Percent(value) => {
            if !(0.0..=1.0).contains(&value) {
                return Err(TableError::InvalidPercentWidth {
                    column: column.key.clone(),
                    value,
                });
            }
            value
        }
    };
    if !width.is_finite() || width <= 0.0 {
        return Err(TableError::InvalidColumnWidth {
            column: column.key.clone(),
            value: width,
        });
    }
    if let Some(cells) = &column.custom_cells
        && cells.len() != row_count
    {
        return Err(TableError::CustomCellCount {
            column: column.key.clone(),
            expected: row_count,
            actual: cells.len(),
        });
    }
    Ok(())
}

fn validate_default_cell(value: &UiValue, column: &TableColumnSpec) -> Result<(), TableError> {
    let valid = match column.format {
        TableCellFormat::Text => matches!(
            value,
            UiValue::Null
                | UiValue::Bool(_)
                | UiValue::Integer(_)
                | UiValue::Float(_)
                | UiValue::String(_)
        ),
        TableCellFormat::Number(_) => matches!(value, UiValue::Integer(_) | UiValue::Float(_)),
        TableCellFormat::Date(_) => {
            matches!(value, UiValue::String(value) if crate::GregorianDate::parse_iso(value).is_ok())
        }
    };
    if valid {
        Ok(())
    } else {
        Err(TableError::InvalidCell {
            column: column.key.clone(),
            expected: format!("{:?}", column.format),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableLayout {
    pub widths: Vec<f64>,
    pub content_width: f64,
}

#[derive(Clone, Debug)]
pub struct TableState {
    spec: TableNodeSpec,
    rows: VirtualListState,
}

impl TableState {
    /// Build validated state and install stable row keys.
    ///
    /// # Errors
    ///
    /// Returns table or virtual-list identity diagnostics.
    pub fn new(spec: TableNodeSpec) -> Result<Self, TableError> {
        spec.validate()?;
        let mut rows = VirtualListState::default();
        rows.set_keys(spec.row_keys())?;
        if !spec.rows.is_empty() {
            rows.focus_first();
        }
        Ok(Self { spec, rows })
    }

    /// Synchronize controlled data while preserving active-row identity.
    ///
    /// # Errors
    ///
    /// Returns table or virtual-list identity diagnostics.
    pub fn synchronize(&mut self, spec: TableNodeSpec) -> Result<bool, TableError> {
        spec.validate()?;
        let changed = self.spec != spec;
        self.rows.set_keys(spec.row_keys())?;
        if self.rows.focused().is_none() && !spec.rows.is_empty() {
            self.rows.focus_first();
        }
        self.spec = spec;
        Ok(changed)
    }

    #[must_use]
    pub fn focused(&self) -> Option<&str> {
        self.rows.focused()
    }

    pub fn focus_next(&mut self) {
        self.rows.focus_next();
    }

    pub fn focus_previous(&mut self) {
        self.rows.focus_previous();
    }

    pub fn focus_first(&mut self) {
        self.rows.focus_first();
    }

    pub fn focus_last(&mut self) {
        self.rows.focus_last();
    }

    /// Focus a supplied row by stable identity.
    ///
    /// # Errors
    ///
    /// Returns an unknown-row diagnostic when the key is not in the current
    /// controlled page.
    pub fn focus_key(&mut self, key: &str) -> Result<bool, TableError> {
        if !self.spec.rows.iter().any(|row| row.key == key) {
            return Err(TableError::UnknownSelection(key.to_owned()));
        }
        let changed = self.rows.focused() != Some(key);
        self.rows.focus(key)?;
        Ok(changed)
    }

    /// Cycle a sortable column through ascending, descending, and none.
    ///
    /// # Errors
    ///
    /// Returns unknown or non-sortable column diagnostics.
    pub fn toggle_sort(&self, column: &str) -> Result<Option<TableSort>, TableError> {
        let column_spec = self
            .spec
            .columns
            .iter()
            .find(|candidate| candidate.key == column)
            .ok_or_else(|| TableError::UnknownSortColumn(column.to_owned()))?;
        if !column_spec.sortable {
            return Err(TableError::UnsortableColumn(column.to_owned()));
        }
        Ok(match self.spec.sort.as_ref() {
            Some(sort) if sort.key == column && sort.direction == TableSortDirection::Ascending => {
                Some(TableSort {
                    key: column.to_owned(),
                    direction: TableSortDirection::Descending,
                })
            }
            Some(sort)
                if sort.key == column && sort.direction == TableSortDirection::Descending =>
            {
                None
            }
            _ => Some(TableSort {
                key: column.to_owned(),
                direction: TableSortDirection::Ascending,
            }),
        })
    }

    /// Compute the next controlled selection for one row.
    ///
    /// # Errors
    ///
    /// Returns unknown-row or disabled-selection diagnostics.
    pub fn select_row(&self, key: &str) -> Result<BTreeSet<String>, TableError> {
        if !self.spec.rows.iter().any(|row| row.key == key) {
            return Err(TableError::UnknownSelection(key.to_owned()));
        }
        let mut selected = self.spec.selected_keys.clone();
        match self.spec.selection_mode {
            TableSelectionMode::None => return Err(TableError::SelectionDisabled),
            TableSelectionMode::Single => {
                selected.clear();
                selected.insert(key.to_owned());
            }
            TableSelectionMode::Multiple => {
                if !selected.remove(key) {
                    selected.insert(key.to_owned());
                }
            }
        }
        Ok(selected)
    }

    /// Toggle every currently supplied row in multiple-selection mode.
    ///
    /// # Errors
    ///
    /// Returns [`TableError::SelectionDisabled`] outside multiple mode.
    pub fn select_all_current(&self) -> Result<BTreeSet<String>, TableError> {
        if self.spec.selection_mode != TableSelectionMode::Multiple {
            return Err(TableError::SelectionDisabled);
        }
        let all = self.spec.rows.iter().map(|row| row.key.clone()).collect();
        if self.spec.selected_keys == all {
            Ok(BTreeSet::new())
        } else {
            Ok(all)
        }
    }

    /// Return bounded visible-row metrics using the shared virtual-list model.
    ///
    /// # Errors
    ///
    /// Returns invalid viewport or row-height diagnostics.
    pub fn metrics(
        &self,
        scroll_offset: f64,
        viewport_height: f64,
    ) -> Result<crate::VirtualListMetrics, TableError> {
        let virtual_spec = VirtualListSpec::new(self.spec.row_height, self.spec.overscan)?;
        Ok(self
            .rows
            .metrics(virtual_spec, scroll_offset, viewport_height)?)
    }

    #[must_use]
    pub fn spec(&self) -> &TableNodeSpec {
        &self.spec
    }
}

#[derive(Clone, Debug, thiserror::Error, PartialEq)]
pub enum TableError {
    #[error("Table key cannot be empty")]
    EmptyKey,
    #[error("Table accessibility label cannot be empty")]
    EmptyLabel,
    #[error("Table requires at least one column")]
    NoColumns,
    #[error("Table column keys cannot be empty")]
    EmptyColumnKey,
    #[error("Table column `{0}` is duplicated")]
    DuplicateColumn(String),
    #[error("Table row keys cannot be empty")]
    EmptyRowKey,
    #[error("Table row key `{0}` is duplicated")]
    DuplicateRow(String),
    #[error("Table row `{row}` is missing column `{column}`")]
    MissingCell { row: String, column: String },
    #[error("Table column `{column}` expected {expected} cell data")]
    InvalidCell { column: String, expected: String },
    #[error("Table row height must be finite and positive, got {0}")]
    InvalidRowHeight(f64),
    #[error("Table flex minimum width must be finite and non-negative, got {0}")]
    InvalidFlexMinimum(f64),
    #[error("Table selection geometry requires 0 < size <= width, got size {size}, width {width}")]
    InvalidSelectionGeometry { width: f64, size: f64 },
    #[error(
        "Table horizontal scrollbar requires positive height/thumb width and 0 <= 2 * inset < height, got height {height}, thumb width {thumb_min_width}, inset {inset}"
    )]
    InvalidHorizontalScrollbarGeometry {
        height: f64,
        thumb_min_width: f64,
        inset: f64,
    },
    #[error("Table default loading state cannot contain more than 100 rows, got {0}")]
    TooManyLoadingRows(usize),
    #[error("Table height is invalid: {0}")]
    InvalidHeight(String),
    #[error("Table column `{column}` width must be finite and positive, got {value}")]
    InvalidColumnWidth { column: String, value: f64 },
    #[error("Table column `{column}` percent width must be between 0 and 1, got {value}")]
    InvalidPercentWidth { column: String, value: f64 },
    #[error("Table viewport width must be finite and non-negative, got {0}")]
    InvalidViewportWidth(f64),
    #[error("Table row index {0} is out of bounds")]
    RowIndex(usize),
    #[error("Table column index {0} is out of bounds")]
    ColumnIndex(usize),
    #[error("Table locale formatting failed: {0}")]
    Format(String),
    #[error("Table custom column `{column}` built {actual} cells for {expected} rows")]
    CustomCellCount {
        column: String,
        expected: usize,
        actual: usize,
    },
    #[error("Table selected row `{0}` does not exist")]
    UnknownSelection(String),
    #[error("Table selection is disabled for the current mode")]
    SelectionDisabled,
    #[error("single-selection Table cannot contain multiple selected keys")]
    MultipleSingleSelection,
    #[error("Table sort column `{0}` does not exist")]
    UnknownSortColumn(String),
    #[error("Table column `{0}` is not sortable")]
    UnsortableColumn(String),
    #[error(transparent)]
    VirtualList(#[from] crate::VirtualListError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(row_count: usize) -> TableNodeSpec {
        let engine = crate::RuntimeEngine::new();
        let locale = crate::load_locale_source(
            engine.engine(),
            "en.rhai",
            include_str!("../../../registry/locales/en.rhai"),
        )
        .unwrap();
        TableNodeSpec {
            key: "users".to_owned(),
            label: "Users".to_owned(),
            columns: vec![
                TableColumnSpec {
                    key: "name".to_owned(),
                    title: "Name".to_owned(),
                    width: TableColumnWidth::Fixed(120.0),
                    align: TableAlign::Start,
                    format: TableCellFormat::Text,
                    sortable: true,
                    custom_cells: None,
                },
                TableColumnSpec {
                    key: "score".to_owned(),
                    title: "Score".to_owned(),
                    width: TableColumnWidth::Flex(2.0),
                    align: TableAlign::End,
                    format: TableCellFormat::Number(NumberFormatOptions::default()),
                    sortable: true,
                    custom_cells: None,
                },
            ],
            rows: (0..row_count)
                .map(|index| TableRowSpec {
                    key: format!("user-{index}"),
                    values: BTreeMap::from([
                        ("name".to_owned(), UiValue::String(format!("User {index}"))),
                        (
                            "score".to_owned(),
                            UiValue::Integer(i64::try_from(index).unwrap()),
                        ),
                    ]),
                })
                .collect(),
            height: Length::Pixels(400.0),
            row_height: 32.0,
            flex_min_width: 80.0,
            selection_width: 32.0,
            selection_size: 16.0,
            horizontal_scrollbar_height: 12.0,
            horizontal_scrollbar_thumb_min_width: 64.0,
            horizontal_scrollbar_inset: 4.0,
            overscan: 2,
            loading: false,
            loading_slot: None,
            loading_rows: Vec::new(),
            empty_slot: None,
            empty_text: "No users".to_owned(),
            striped: true,
            selection_mode: TableSelectionMode::Multiple,
            selected_keys: BTreeSet::new(),
            sort: None,
            calendar: locale.calendar,
            number: locale.number,
            check_asset: crate::AssetId::parse("app/icons/check").unwrap(),
            sort_ascending_asset: crate::AssetId::parse("app/icons/sort_ascending").unwrap(),
            sort_descending_asset: crate::AssetId::parse("app/icons/sort_descending").unwrap(),
        }
    }

    #[test]
    fn width_tracks_preserve_overflow_and_weight_flex() {
        let mut spec = table(1);
        spec.columns.push(TableColumnSpec {
            key: "extra".to_owned(),
            title: "Extra".to_owned(),
            width: TableColumnWidth::Percent(0.5),
            align: TableAlign::Start,
            format: TableCellFormat::Text,
            sortable: false,
            custom_cells: None,
        });
        spec.rows[0]
            .values
            .insert("extra".to_owned(), UiValue::String("x".to_owned()));
        spec.validate().unwrap();
        let layout = spec.resolve_widths(400.0).unwrap();
        assert_eq!(layout.widths, vec![120.0, 80.0, 200.0]);
        assert!((layout.content_width - 400.0).abs() < f64::EPSILON);

        spec.columns[0].width = TableColumnWidth::Fixed(300.0);
        let layout = spec.resolve_widths(400.0).unwrap();
        assert!((layout.content_width - 580.0).abs() < f64::EPSILON);
        assert!((layout.widths[1] - 80.0).abs() < f64::EPSILON);
    }

    #[test]
    fn sorting_and_selection_are_controlled_outcomes() {
        let spec = table(3);
        let mut state = TableState::new(spec).unwrap();
        assert_eq!(
            state.toggle_sort("name").unwrap(),
            Some(TableSort {
                key: "name".to_owned(),
                direction: TableSortDirection::Ascending,
            })
        );
        assert_eq!(
            state.select_row("user-1").unwrap(),
            BTreeSet::from(["user-1".to_owned()])
        );
        assert_eq!(state.select_all_current().unwrap().len(), 3);
        assert!(state.focus_key("user-2").unwrap());
        assert_eq!(state.focused(), Some("user-2"));
        assert!(!state.focus_key("user-2").unwrap());
    }

    #[test]
    fn ten_thousand_scalar_rows_have_bounded_realization() {
        let state = TableState::new(table(10_000)).unwrap();
        let metrics = state.metrics(12_000.0, 480.0).unwrap();
        assert_eq!(metrics.item_count, 10_000);
        assert!(metrics.realized_count <= 20, "{metrics:?}");
    }

    #[test]
    fn duplicate_identity_and_complex_default_cells_fail() {
        let mut spec = table(2);
        spec.rows[1].key = spec.rows[0].key.clone();
        assert!(matches!(spec.validate(), Err(TableError::DuplicateRow(_))));
        let mut spec = table(1);
        spec.rows[0]
            .values
            .insert("name".to_owned(), UiValue::Array(Vec::new()));
        assert!(matches!(
            spec.validate(),
            Err(TableError::InvalidCell { .. })
        ));
    }
}
