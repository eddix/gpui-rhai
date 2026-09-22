#![allow(clippy::cast_precision_loss)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

use event_listener::EventListener;
use rhai::{CustomType, Engine, TypeBuilder};
use thiserror::Error;

use crate::{UiValue, async_runtime::AsyncWake};

/// Hard limits applied before chart data reaches transform or layout work.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ChartDataLimits {
    pub max_datasets: usize,
    pub max_dimensions: usize,
    pub max_rows: usize,
    pub max_string_bytes: usize,
}

impl Default for ChartDataLimits {
    fn default() -> Self {
        Self {
            max_datasets: 32,
            max_dimensions: 256,
            max_rows: 2_000_000,
            max_string_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ChartDataType {
    Number,
    Integer,
    Timestamp,
    Bool,
    String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChartValue {
    Null,
    Number(f64),
    Integer(i64),
    Timestamp(i64),
    Bool(bool),
    String(String),
}

impl ChartValue {
    #[must_use]
    pub const fn data_type(&self) -> Option<ChartDataType> {
        match self {
            Self::Null => None,
            Self::Number(_) => Some(ChartDataType::Number),
            Self::Integer(_) => Some(ChartDataType::Integer),
            Self::Timestamp(_) => Some(ChartDataType::Timestamp),
            Self::Bool(_) => Some(ChartDataType::Bool),
            Self::String(_) => Some(ChartDataType::String),
        }
    }

    #[must_use]
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Integer(value) | Self::Timestamp(value) => Some(*value as f64),
            Self::Null | Self::Bool(_) | Self::String(_) => None,
        }
    }

    #[must_use]
    pub fn display_text(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Number(value) => value.to_string(),
            Self::Integer(value) | Self::Timestamp(value) => value.to_string(),
            Self::Bool(value) => value.to_string(),
            Self::String(value) => value.clone(),
        }
    }
}

/// Compact null bitmap kept separately from the typed column payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChartNullBitmap {
    bits: Arc<[u64]>,
    len: usize,
}

impl ChartNullBitmap {
    #[must_use]
    pub fn from_validity(valid: impl IntoIterator<Item = bool>) -> Self {
        let validity = valid.into_iter().collect::<Vec<_>>();
        let mut bits = vec![0_u64; validity.len().div_ceil(64)];
        for (index, is_valid) in validity.iter().copied().enumerate() {
            if !is_valid {
                bits[index / 64] |= 1_u64 << (index % 64);
            }
        }
        Self {
            bits: bits.into(),
            len: validity.len(),
        }
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub fn is_null(&self, index: usize) -> bool {
        index >= self.len || self.bits[index / 64] & (1_u64 << (index % 64)) != 0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChartColumnValues {
    Number(Arc<[f64]>),
    Integer(Arc<[i64]>),
    Timestamp(Arc<[i64]>),
    Bool(Arc<[bool]>),
    String(Arc<[String]>),
}

impl ChartColumnValues {
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Number(values) => values.len(),
            Self::Integer(values) | Self::Timestamp(values) => values.len(),
            Self::Bool(values) => values.len(),
            Self::String(values) => values.len(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub const fn data_type(&self) -> ChartDataType {
        match self {
            Self::Number(_) => ChartDataType::Number,
            Self::Integer(_) => ChartDataType::Integer,
            Self::Timestamp(_) => ChartDataType::Timestamp,
            Self::Bool(_) => ChartDataType::Bool,
            Self::String(_) => ChartDataType::String,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartColumn {
    name: String,
    values: ChartColumnValues,
    nulls: ChartNullBitmap,
}

impl ChartColumn {
    /// Construct a validated typed column.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsafe name, a bitmap length mismatch, or a
    /// non-finite numeric value in a non-null row.
    pub fn new(
        name: impl Into<String>,
        values: ChartColumnValues,
        nulls: ChartNullBitmap,
    ) -> Result<Self, ChartDataError> {
        let name = name.into();
        validate_dimension_name(&name)?;
        if values.len() != nulls.len() {
            return Err(ChartDataError::ColumnLength {
                dimension: name,
                expected: values.len(),
                actual: nulls.len(),
            });
        }
        if let ChartColumnValues::Number(values) = &values
            && let Some(index) = values.iter().enumerate().find_map(|(index, value)| {
                (!nulls.is_null(index) && !value.is_finite()).then_some(index)
            })
        {
            return Err(ChartDataError::NonFinite {
                dimension: name,
                row: index,
            });
        }
        Ok(Self {
            name,
            values,
            nulls,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.len() == 0
    }

    #[must_use]
    pub const fn data_type(&self) -> ChartDataType {
        self.values.data_type()
    }

    #[must_use]
    pub fn value(&self, index: usize) -> Option<ChartValue> {
        if self.nulls.is_null(index) {
            return Some(ChartValue::Null);
        }
        match &self.values {
            ChartColumnValues::Number(values) => values.get(index).copied().map(ChartValue::Number),
            ChartColumnValues::Integer(values) => {
                values.get(index).copied().map(ChartValue::Integer)
            }
            ChartColumnValues::Timestamp(values) => {
                values.get(index).copied().map(ChartValue::Timestamp)
            }
            ChartColumnValues::Bool(values) => values.get(index).copied().map(ChartValue::Bool),
            ChartColumnValues::String(values) => values.get(index).cloned().map(ChartValue::String),
        }
    }

    #[must_use]
    pub const fn nulls(&self) -> &ChartNullBitmap {
        &self.nulls
    }

    fn append(&self, other: &Self, skip: usize) -> Result<Self, ChartDataError> {
        if self.name != other.name || self.data_type() != other.data_type() {
            return Err(ChartDataError::SchemaMismatch);
        }
        let validity = (0..self.len())
            .map(|index| !self.nulls.is_null(index))
            .chain((0..other.len()).map(|index| !other.nulls.is_null(index)))
            .skip(skip)
            .collect::<Vec<_>>();
        let values = match (&self.values, &other.values) {
            (ChartColumnValues::Number(left), ChartColumnValues::Number(right)) => {
                ChartColumnValues::Number(
                    left.iter()
                        .chain(right.iter())
                        .copied()
                        .skip(skip)
                        .collect::<Vec<_>>()
                        .into(),
                )
            }
            (ChartColumnValues::Integer(left), ChartColumnValues::Integer(right)) => {
                ChartColumnValues::Integer(
                    left.iter()
                        .chain(right.iter())
                        .copied()
                        .skip(skip)
                        .collect::<Vec<_>>()
                        .into(),
                )
            }
            (ChartColumnValues::Timestamp(left), ChartColumnValues::Timestamp(right)) => {
                ChartColumnValues::Timestamp(
                    left.iter()
                        .chain(right.iter())
                        .copied()
                        .skip(skip)
                        .collect::<Vec<_>>()
                        .into(),
                )
            }
            (ChartColumnValues::Bool(left), ChartColumnValues::Bool(right)) => {
                ChartColumnValues::Bool(
                    left.iter()
                        .chain(right.iter())
                        .copied()
                        .skip(skip)
                        .collect::<Vec<_>>()
                        .into(),
                )
            }
            (ChartColumnValues::String(left), ChartColumnValues::String(right)) => {
                ChartColumnValues::String(
                    left.iter()
                        .chain(right.iter())
                        .skip(skip)
                        .cloned()
                        .collect::<Vec<_>>()
                        .into(),
                )
            }
            _ => return Err(ChartDataError::SchemaMismatch),
        };
        Self::new(
            self.name.clone(),
            values,
            ChartNullBitmap::from_validity(validity),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartDataset {
    name: String,
    columns: BTreeMap<String, ChartColumn>,
    len: usize,
    key_dimension: Option<String>,
}

impl ChartDataset {
    /// Construct a named columnar dataset.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid identity, inconsistent column lengths,
    /// missing/duplicate keys, or configured data limits.
    pub fn new(
        name: impl Into<String>,
        columns: impl IntoIterator<Item = ChartColumn>,
        key_dimension: Option<String>,
        limits: ChartDataLimits,
    ) -> Result<Self, ChartDataError> {
        let name = name.into();
        validate_dataset_name(&name)?;
        let mut mapped = BTreeMap::new();
        let mut len = None;
        for column in columns {
            if mapped.contains_key(column.name()) {
                return Err(ChartDataError::DuplicateDimension(column.name().to_owned()));
            }
            if let Some(expected) = len {
                if column.len() != expected {
                    return Err(ChartDataError::ColumnLength {
                        dimension: column.name().to_owned(),
                        expected,
                        actual: column.len(),
                    });
                }
            } else {
                len = Some(column.len());
            }
            mapped.insert(column.name().to_owned(), column);
        }
        let len = len.unwrap_or(0);
        if mapped.len() > limits.max_dimensions {
            return Err(ChartDataError::TooManyDimensions {
                actual: mapped.len(),
                limit: limits.max_dimensions,
            });
        }
        if len > limits.max_rows {
            return Err(ChartDataError::TooManyRows {
                actual: len,
                limit: limits.max_rows,
            });
        }
        let string_bytes = mapped
            .values()
            .filter_map(|column| match &column.values {
                ChartColumnValues::String(values) => Some(
                    values
                        .iter()
                        .fold(0_usize, |total, value| total.saturating_add(value.len())),
                ),
                _ => None,
            })
            .fold(0_usize, usize::saturating_add);
        if string_bytes > limits.max_string_bytes {
            return Err(ChartDataError::TooManyStringBytes {
                actual: string_bytes,
                limit: limits.max_string_bytes,
            });
        }
        if let Some(key) = &key_dimension {
            let column = mapped
                .get(key)
                .ok_or_else(|| ChartDataError::MissingKeyDimension(key.clone()))?;
            let mut keys = BTreeSet::new();
            for row in 0..len {
                let value = column
                    .value(row)
                    .filter(|value| !matches!(value, ChartValue::Null))
                    .ok_or(ChartDataError::NullKey { row })?;
                let value = value.display_text();
                if value.is_empty() || !keys.insert(value.clone()) {
                    return Err(ChartDataError::DuplicateKey(value));
                }
            }
        }
        Ok(Self {
            name,
            columns: mapped,
            len,
            key_dimension,
        })
    }

    /// Infer a typed dataset from durable map rows.
    ///
    /// Integers promote to number when mixed with floats. Other mixed non-null
    /// types are rejected rather than stringified implicitly.
    ///
    /// # Errors
    ///
    /// Returns a schema, type, identity, or configured-limit diagnostic.
    pub fn from_rows(
        name: impl Into<String>,
        rows: &[BTreeMap<String, UiValue>],
        key_dimension: Option<String>,
        limits: ChartDataLimits,
    ) -> Result<Self, ChartDataError> {
        let rows = rows
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                row.iter()
                    .map(|(name, value)| ui_to_chart_value(name, value, row_index))
                    .collect::<Result<BTreeMap<_, _>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_chart_rows(name, &rows, key_dimension, limits)
    }

    /// Infer a typed dataset from already-normalized chart scalar rows.
    ///
    /// # Errors
    ///
    /// Returns a schema, type, identity, or configured-limit diagnostic.
    pub fn from_chart_rows(
        name: impl Into<String>,
        rows: &[BTreeMap<String, ChartValue>],
        key_dimension: Option<String>,
        limits: ChartDataLimits,
    ) -> Result<Self, ChartDataError> {
        if rows.len() > limits.max_rows {
            return Err(ChartDataError::TooManyRows {
                actual: rows.len(),
                limit: limits.max_rows,
            });
        }
        let dimensions = rows
            .iter()
            .flat_map(BTreeMap::keys)
            .cloned()
            .collect::<BTreeSet<_>>();
        let columns = dimensions
            .into_iter()
            .map(|dimension| infer_chart_column(&dimension, rows))
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(name, columns, key_dimension, limits)
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub fn column(&self, name: &str) -> Option<&ChartColumn> {
        self.columns.get(name)
    }

    pub fn columns(&self) -> impl Iterator<Item = &ChartColumn> {
        self.columns.values()
    }

    #[must_use]
    pub fn key(&self, row: usize) -> Option<String> {
        let key = self.key_dimension.as_ref()?;
        self.columns
            .get(key)?
            .value(row)
            .map(|value| value.display_text())
    }

    #[must_use]
    pub fn key_dimension(&self) -> Option<&str> {
        self.key_dimension.as_deref()
    }

    #[must_use]
    pub fn row(&self, index: usize) -> Option<BTreeMap<String, ChartValue>> {
        (index < self.len).then(|| {
            self.columns
                .iter()
                .filter_map(|(name, column)| column.value(index).map(|value| (name.clone(), value)))
                .collect()
        })
    }

    #[must_use]
    pub fn rows(&self) -> Vec<BTreeMap<String, ChartValue>> {
        (0..self.len).filter_map(|index| self.row(index)).collect()
    }

    fn validate_limits(&self, limits: ChartDataLimits) -> Result<(), ChartDataError> {
        if self.columns.len() > limits.max_dimensions {
            return Err(ChartDataError::TooManyDimensions {
                actual: self.columns.len(),
                limit: limits.max_dimensions,
            });
        }
        if self.len > limits.max_rows {
            return Err(ChartDataError::TooManyRows {
                actual: self.len,
                limit: limits.max_rows,
            });
        }
        let string_bytes = self
            .columns
            .values()
            .filter_map(|column| match &column.values {
                ChartColumnValues::String(values) => Some(
                    values
                        .iter()
                        .fold(0_usize, |total, value| total.saturating_add(value.len())),
                ),
                _ => None,
            })
            .fold(0_usize, usize::saturating_add);
        if string_bytes > limits.max_string_bytes {
            return Err(ChartDataError::TooManyStringBytes {
                actual: string_bytes,
                limit: limits.max_string_bytes,
            });
        }
        Ok(())
    }

    /// Select rows in the supplied order while preserving typed columns and
    /// the original key dimension.
    ///
    /// # Errors
    ///
    /// Returns when an index is outside this dataset or reconstruction fails.
    pub fn select_rows(&self, indices: &[usize]) -> Result<Self, ChartDataError> {
        if let Some(index) = indices.iter().copied().find(|index| *index >= self.len) {
            return Err(ChartDataError::RowOutOfBounds {
                index,
                len: self.len,
            });
        }
        let rows = indices
            .iter()
            .filter_map(|index| self.row(*index))
            .collect::<Vec<_>>();
        Self::from_chart_rows(
            self.name.clone(),
            &rows,
            self.key_dimension.clone(),
            ChartDataLimits::default(),
        )
    }

    pub(crate) fn with_replacement_identity(&self, series: &str) -> Result<Self, ChartDataError> {
        if self.key_dimension.is_some() {
            return Ok(self.clone());
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        series.hash(&mut hasher);
        for column in self.columns.values() {
            column.name.hash(&mut hasher);
            column.data_type().hash(&mut hasher);
            for row in 0..column.len() {
                match column.value(row).unwrap_or(ChartValue::Null) {
                    ChartValue::Null => 0_u8.hash(&mut hasher),
                    ChartValue::Number(value) => value.to_bits().hash(&mut hasher),
                    ChartValue::Integer(value) | ChartValue::Timestamp(value) => {
                        value.hash(&mut hasher);
                    }
                    ChartValue::Bool(value) => value.hash(&mut hasher),
                    ChartValue::String(value) => value.hash(&mut hasher),
                }
            }
        }
        let fingerprint = hasher.finish();
        let mut dimension = "chart_identity".to_owned();
        let mut suffix = 2_usize;
        while self.columns.contains_key(&dimension) {
            dimension = format!("chart_identity_{suffix}");
            suffix = suffix.saturating_add(1);
        }
        let mut rows = self.rows();
        for (row, values) in rows.iter_mut().enumerate() {
            values.insert(
                dimension.clone(),
                ChartValue::String(format!("{series}:{fingerprint:016x}:{row}")),
            );
        }
        Self::from_chart_rows(
            self.name.clone(),
            &rows,
            Some(dimension),
            ChartDataLimits::default(),
        )
    }

    fn append(
        &self,
        chunk: &Self,
        keep_last: Option<usize>,
        limits: ChartDataLimits,
    ) -> Result<Self, ChartDataError> {
        if self.columns.keys().ne(chunk.columns.keys()) {
            return Err(ChartDataError::SchemaMismatch);
        }
        for (name, column) in &self.columns {
            if column.data_type() != chunk.columns[name].data_type() {
                return Err(ChartDataError::SchemaMismatch);
            }
        }
        let combined = self.len.saturating_add(chunk.len);
        let skip = keep_last.map_or(0, |keep_last| combined.saturating_sub(keep_last));
        let columns = self
            .columns
            .iter()
            .map(|(name, column)| column.append(&chunk.columns[name], skip))
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(
            self.name.clone(),
            columns,
            self.key_dimension.clone(),
            limits,
        )
    }
}

#[derive(Clone, Debug)]
pub struct ChartDataSnapshot {
    revision: u64,
    datasets: Arc<BTreeMap<String, ChartDataset>>,
}

impl ChartDataSnapshot {
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn dataset(&self, name: &str) -> Option<&ChartDataset> {
        self.datasets.get(name)
    }

    pub fn datasets(&self) -> impl Iterator<Item = (&str, &ChartDataset)> {
        self.datasets
            .iter()
            .map(|(name, dataset)| (name.as_str(), dataset))
    }
}

struct NativeChartDataInner {
    snapshot: RwLock<ChartDataSnapshot>,
    wake: AsyncWake,
    limits: ChartDataLimits,
}

/// Thread-safe, revisioned chart data owned by a trusted Rust Host.
///
/// Updates replace one immutable snapshot under a lock and then issue a
/// coalescible edge notification. A mounted chart always reads the newest
/// revision, so producers cannot build an unbounded foreground delivery queue.
#[derive(Clone)]
pub struct NativeChartData {
    inner: Arc<NativeChartDataInner>,
}

impl fmt::Debug for NativeChartData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let snapshot = self.snapshot();
        formatter
            .debug_struct("NativeChartData")
            .field("revision", &snapshot.revision)
            .field("datasets", &snapshot.datasets.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl PartialEq for NativeChartData {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl NativeChartData {
    /// Construct a native data handle with one or more uniquely named datasets.
    ///
    /// # Errors
    ///
    /// Returns for an empty, duplicated, or over-limit dataset collection.
    pub fn new(
        datasets: impl IntoIterator<Item = ChartDataset>,
        limits: ChartDataLimits,
    ) -> Result<Self, ChartDataError> {
        let datasets = collect_datasets(datasets, limits)?;
        Ok(Self {
            inner: Arc::new(NativeChartDataInner {
                snapshot: RwLock::new(ChartDataSnapshot {
                    revision: 1,
                    datasets: Arc::new(datasets),
                }),
                wake: AsyncWake::default(),
                limits,
            }),
        })
    }

    #[must_use]
    pub fn snapshot(&self) -> ChartDataSnapshot {
        self.inner
            .snapshot
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.snapshot().revision
    }

    #[must_use]
    pub fn len(&self, dataset: &str) -> usize {
        self.snapshot()
            .dataset(dataset)
            .map_or(0, ChartDataset::len)
    }

    /// Atomically replace the complete named dataset set.
    ///
    /// # Errors
    ///
    /// Returns for invalid datasets, a poisoned lock, or exhausted revision ID.
    pub fn replace(
        &self,
        datasets: impl IntoIterator<Item = ChartDataset>,
    ) -> Result<u64, ChartDataError> {
        let datasets = collect_datasets(datasets, self.inner.limits)?;
        self.commit(datasets)
    }

    /// Append a schema-compatible chunk to one dataset.
    ///
    /// # Errors
    ///
    /// Returns for an unknown dataset, schema mismatch, or limit violation.
    pub fn append(&self, dataset: &str, chunk: &ChartDataset) -> Result<u64, ChartDataError> {
        self.append_inner(dataset, chunk, None)
    }

    /// Append and retain only the newest `keep_last` rows.
    ///
    /// # Errors
    ///
    /// Returns for an invalid window, unknown dataset, or schema mismatch.
    pub fn append_sliding(
        &self,
        dataset: &str,
        chunk: &ChartDataset,
        keep_last: usize,
    ) -> Result<u64, ChartDataError> {
        if keep_last == 0 || keep_last > self.inner.limits.max_rows {
            return Err(ChartDataError::InvalidWindow(keep_last));
        }
        self.append_inner(dataset, chunk, Some(keep_last))
    }

    fn append_inner(
        &self,
        dataset: &str,
        chunk: &ChartDataset,
        keep_last: Option<usize>,
    ) -> Result<u64, ChartDataError> {
        for _ in 0..32 {
            let current = self.snapshot();
            let source = current
                .dataset(dataset)
                .ok_or_else(|| ChartDataError::UnknownDataset(dataset.to_owned()))?;
            let appended = source.append(chunk, keep_last, self.inner.limits)?;
            if appended.len() > self.inner.limits.max_rows {
                return Err(ChartDataError::TooManyRows {
                    actual: appended.len(),
                    limit: self.inner.limits.max_rows,
                });
            }
            let mut datasets = (*current.datasets).clone();
            datasets.insert(dataset.to_owned(), appended);
            if let Some(revision) = self.commit_if_revision(current.revision, datasets)? {
                return Ok(revision);
            }
        }
        Err(ChartDataError::ConcurrentUpdate)
    }

    fn commit(&self, datasets: BTreeMap<String, ChartDataset>) -> Result<u64, ChartDataError> {
        let mut snapshot = self
            .inner
            .snapshot
            .write()
            .map_err(|_| ChartDataError::Poisoned)?;
        let revision = snapshot
            .revision
            .checked_add(1)
            .ok_or(ChartDataError::RevisionExhausted)?;
        *snapshot = ChartDataSnapshot {
            revision,
            datasets: Arc::new(datasets),
        };
        drop(snapshot);
        self.inner.wake.notify();
        Ok(revision)
    }

    fn commit_if_revision(
        &self,
        expected: u64,
        datasets: BTreeMap<String, ChartDataset>,
    ) -> Result<Option<u64>, ChartDataError> {
        let mut snapshot = self
            .inner
            .snapshot
            .write()
            .map_err(|_| ChartDataError::Poisoned)?;
        if snapshot.revision != expected {
            return Ok(None);
        }
        let revision = snapshot
            .revision
            .checked_add(1)
            .ok_or(ChartDataError::RevisionExhausted)?;
        *snapshot = ChartDataSnapshot {
            revision,
            datasets: Arc::new(datasets),
        };
        drop(snapshot);
        self.inner.wake.notify();
        Ok(Some(revision))
    }

    pub(crate) fn listen(&self) -> EventListener {
        self.inner.wake.listen()
    }
}

impl CustomType for NativeChartData {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("NativeChartData")
            .with_get("revision", |data: &mut Self| {
                i64::try_from(data.revision()).unwrap_or(i64::MAX)
            })
            .with_fn("len", |data: &mut Self, dataset: &str| {
                i64::try_from(data.len(dataset)).unwrap_or(i64::MAX)
            });
    }
}

pub(crate) fn register_chart_data_api(engine: &mut Engine) {
    engine.build_type::<NativeChartData>();
}

fn collect_datasets(
    datasets: impl IntoIterator<Item = ChartDataset>,
    limits: ChartDataLimits,
) -> Result<BTreeMap<String, ChartDataset>, ChartDataError> {
    let mut mapped = BTreeMap::new();
    for dataset in datasets {
        dataset.validate_limits(limits)?;
        if mapped.insert(dataset.name().to_owned(), dataset).is_some() {
            return Err(ChartDataError::DuplicateDataset);
        }
    }
    if mapped.is_empty() {
        return Err(ChartDataError::NoDatasets);
    }
    if mapped.len() > limits.max_datasets {
        return Err(ChartDataError::TooManyDatasets {
            actual: mapped.len(),
            limit: limits.max_datasets,
        });
    }
    Ok(mapped)
}

fn ui_to_chart_value(
    name: &str,
    value: &UiValue,
    row: usize,
) -> Result<(String, ChartValue), ChartDataError> {
    let value = match value {
        UiValue::Null => ChartValue::Null,
        UiValue::Float(value) if value.is_finite() => ChartValue::Number(*value),
        UiValue::Float(_) => {
            return Err(ChartDataError::NonFinite {
                dimension: name.to_owned(),
                row,
            });
        }
        UiValue::Integer(value) => ChartValue::Integer(*value),
        UiValue::Bool(value) => ChartValue::Bool(*value),
        UiValue::String(value) => ChartValue::String(value.clone()),
        _ => {
            return Err(ChartDataError::UnsupportedValue {
                dimension: name.to_owned(),
                row,
            });
        }
    };
    Ok((name.to_owned(), value))
}

fn infer_chart_column(
    dimension: &str,
    rows: &[BTreeMap<String, ChartValue>],
) -> Result<ChartColumn, ChartDataError> {
    validate_dimension_name(dimension)?;
    let mut kind = None;
    let mut valid = Vec::with_capacity(rows.len());
    for (row, values) in rows.iter().enumerate() {
        let value = values.get(dimension).unwrap_or(&ChartValue::Null);
        let next = match value {
            ChartValue::Null => None,
            ChartValue::Number(value) if value.is_finite() => Some(ChartDataType::Number),
            ChartValue::Number(_) => {
                return Err(ChartDataError::NonFinite {
                    dimension: dimension.to_owned(),
                    row,
                });
            }
            ChartValue::Integer(_) => Some(ChartDataType::Integer),
            ChartValue::Timestamp(_) => Some(ChartDataType::Timestamp),
            ChartValue::Bool(_) => Some(ChartDataType::Bool),
            ChartValue::String(_) => Some(ChartDataType::String),
        };
        valid.push(next.is_some());
        kind = merge_types(kind, next).map_err(|()| ChartDataError::MixedTypes {
            dimension: dimension.to_owned(),
            row,
        })?;
    }
    let kind = kind.unwrap_or(ChartDataType::String);
    let values = match kind {
        ChartDataType::Number => ChartColumnValues::Number(
            rows.iter()
                .map(|row| match row.get(dimension) {
                    Some(ChartValue::Number(value)) => *value,
                    Some(ChartValue::Integer(value)) => *value as f64,
                    _ => 0.0,
                })
                .collect::<Vec<_>>()
                .into(),
        ),
        ChartDataType::Integer => ChartColumnValues::Integer(
            rows.iter()
                .map(|row| match row.get(dimension) {
                    Some(ChartValue::Integer(value)) => *value,
                    _ => 0,
                })
                .collect::<Vec<_>>()
                .into(),
        ),
        ChartDataType::Bool => ChartColumnValues::Bool(
            rows.iter()
                .map(|row| match row.get(dimension) {
                    Some(ChartValue::Bool(value)) => *value,
                    _ => false,
                })
                .collect::<Vec<_>>()
                .into(),
        ),
        ChartDataType::String => ChartColumnValues::String(
            rows.iter()
                .map(|row| match row.get(dimension) {
                    Some(ChartValue::String(value)) => value.clone(),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .into(),
        ),
        ChartDataType::Timestamp => ChartColumnValues::Timestamp(
            rows.iter()
                .map(|row| match row.get(dimension) {
                    Some(ChartValue::Timestamp(value)) => *value,
                    _ => 0,
                })
                .collect::<Vec<_>>()
                .into(),
        ),
    };
    ChartColumn::new(dimension, values, ChartNullBitmap::from_validity(valid))
}

fn merge_types(
    current: Option<ChartDataType>,
    next: Option<ChartDataType>,
) -> Result<Option<ChartDataType>, ()> {
    match (current, next) {
        (current, None) => Ok(current),
        (None, next) => Ok(next),
        (Some(left), Some(right)) if left == right => Ok(Some(left)),
        (
            Some(ChartDataType::Integer | ChartDataType::Number),
            Some(ChartDataType::Integer | ChartDataType::Number),
        ) => Ok(Some(ChartDataType::Number)),
        _ => Err(()),
    }
}

fn validate_dataset_name(name: &str) -> Result<(), ChartDataError> {
    if valid_identifier(name) {
        Ok(())
    } else {
        Err(ChartDataError::InvalidDatasetName(name.to_owned()))
    }
}

fn validate_dimension_name(name: &str) -> Result<(), ChartDataError> {
    if valid_identifier(name) {
        Ok(())
    } else {
        Err(ChartDataError::InvalidDimensionName(name.to_owned()))
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
pub enum ChartDataError {
    #[error("chart dataset name `{0}` must be a safe identifier")]
    InvalidDatasetName(String),
    #[error("chart dimension name `{0}` must be a safe identifier")]
    InvalidDimensionName(String),
    #[error("chart dataset collection cannot be empty")]
    NoDatasets,
    #[error("chart dataset name is duplicated")]
    DuplicateDataset,
    #[error("chart dimension `{0}` is duplicated")]
    DuplicateDimension(String),
    #[error("chart dataset contains {actual} dimensions; limit is {limit}")]
    TooManyDimensions { actual: usize, limit: usize },
    #[error("chart data contains {actual} rows; limit is {limit}")]
    TooManyRows { actual: usize, limit: usize },
    #[error("chart data contains {actual} string bytes; limit is {limit}")]
    TooManyStringBytes { actual: usize, limit: usize },
    #[error("chart data contains {actual} datasets; limit is {limit}")]
    TooManyDatasets { actual: usize, limit: usize },
    #[error("chart column `{dimension}` has length {actual}; expected {expected}")]
    ColumnLength {
        dimension: String,
        expected: usize,
        actual: usize,
    },
    #[error("chart value in `{dimension}` row {row} is not finite")]
    NonFinite { dimension: String, row: usize },
    #[error("chart value in `{dimension}` row {row} is not a scalar")]
    UnsupportedValue { dimension: String, row: usize },
    #[error("chart dimension `{dimension}` mixes incompatible types at row {row}")]
    MixedTypes { dimension: String, row: usize },
    #[error("chart key dimension `{0}` does not exist")]
    MissingKeyDimension(String),
    #[error("chart key is null at row {row}")]
    NullKey { row: usize },
    #[error("chart key `{0}` is empty or duplicated")]
    DuplicateKey(String),
    #[error("chart append schema does not match the target dataset")]
    SchemaMismatch,
    #[error("chart dataset `{0}` does not exist")]
    UnknownDataset(String),
    #[error("chart sliding window size {0} is invalid")]
    InvalidWindow(usize),
    #[error("chart data lock was poisoned")]
    Poisoned,
    #[error("chart data revision identity is exhausted")]
    RevisionExhausted,
    #[error("chart data append could not commit after repeated concurrent updates")]
    ConcurrentUpdate,
    #[error("chart row index {index} is outside dataset length {len}")]
    RowOutOfBounds { index: usize, len: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(start: i64, count: i64) -> Vec<BTreeMap<String, UiValue>> {
        (start..start + count)
            .map(|index| {
                BTreeMap::from([
                    ("id".to_owned(), UiValue::Integer(index)),
                    ("x".to_owned(), UiValue::Integer(index)),
                    ("y".to_owned(), UiValue::Float(index as f64 * 1.5)),
                    ("group".to_owned(), UiValue::String("alpha".to_owned())),
                ])
            })
            .collect()
    }

    #[test]
    fn inferred_columns_are_typed_columnar_and_keyed() {
        let dataset = ChartDataset::from_rows(
            "main",
            &rows(0, 3),
            Some("id".to_owned()),
            ChartDataLimits::default(),
        )
        .unwrap();

        assert_eq!(dataset.len(), 3);
        assert_eq!(
            dataset.column("x").unwrap().data_type(),
            ChartDataType::Integer
        );
        assert_eq!(
            dataset.column("y").unwrap().data_type(),
            ChartDataType::Number
        );
        assert_eq!(dataset.key(2).as_deref(), Some("2"));
    }

    #[test]
    fn streaming_updates_are_atomic_revisioned_and_sliding() {
        let source = ChartDataset::from_rows(
            "main",
            &rows(0, 3),
            Some("id".to_owned()),
            ChartDataLimits::default(),
        )
        .unwrap();
        let data = NativeChartData::new([source], ChartDataLimits::default()).unwrap();
        let chunk = ChartDataset::from_rows(
            "main",
            &rows(3, 3),
            Some("id".to_owned()),
            ChartDataLimits::default(),
        )
        .unwrap();

        assert_eq!(data.append_sliding("main", &chunk, 4).unwrap(), 2);
        let snapshot = data.snapshot();
        let dataset = snapshot.dataset("main").unwrap();
        assert_eq!(dataset.len(), 4);
        assert_eq!(dataset.key(0).as_deref(), Some("2"));
        assert_eq!(dataset.key(3).as_deref(), Some("5"));
    }

    #[test]
    fn invalid_streaming_candidate_does_not_advance_revision() {
        let source = ChartDataset::from_rows(
            "main",
            &rows(0, 2),
            Some("id".to_owned()),
            ChartDataLimits::default(),
        )
        .unwrap();
        let data = NativeChartData::new([source], ChartDataLimits::default()).unwrap();
        let incompatible = ChartDataset::from_rows(
            "other",
            &[BTreeMap::from([(
                "name".to_owned(),
                UiValue::String("bad".to_owned()),
            )])],
            None,
            ChartDataLimits::default(),
        )
        .unwrap();

        assert!(matches!(
            data.append("main", &incompatible),
            Err(ChartDataError::SchemaMismatch)
        ));
        assert_eq!(data.revision(), 1);
        assert_eq!(data.len("main"), 2);
    }

    #[test]
    fn concurrent_appends_retry_without_losing_a_committed_revision() {
        let source = ChartDataset::from_rows(
            "main",
            &rows(0, 1),
            Some("id".to_owned()),
            ChartDataLimits::default(),
        )
        .unwrap();
        let data = NativeChartData::new([source], ChartDataLimits::default()).unwrap();
        let workers = (0..4)
            .map(|worker| {
                let data = data.clone();
                std::thread::spawn(move || {
                    for offset in 0..5 {
                        let start = 1 + worker * 5 + offset;
                        let chunk = ChartDataset::from_rows(
                            "main",
                            &rows(start, 1),
                            Some("id".to_owned()),
                            ChartDataLimits::default(),
                        )
                        .unwrap();
                        data.append("main", &chunk).unwrap();
                    }
                })
            })
            .collect::<Vec<_>>();
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(data.len("main"), 21);
        assert_eq!(data.revision(), 21);
    }

    #[test]
    fn unkeyed_data_uses_content_scoped_whole_series_identity() {
        let source =
            ChartDataset::from_rows("main", &rows(0, 3), None, ChartDataLimits::default()).unwrap();
        let same = source.with_replacement_identity("series").unwrap();
        let same_again = source.with_replacement_identity("series").unwrap();
        let changed =
            ChartDataset::from_rows("main", &rows(1, 3), None, ChartDataLimits::default())
                .unwrap()
                .with_replacement_identity("series")
                .unwrap();

        assert_eq!(same.key(0), same_again.key(0));
        assert_ne!(same.key(0), changed.key(0));
    }
}
