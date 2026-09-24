#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque, btree_map::Entry};
use std::sync::{Arc, RwLock};

use thiserror::Error;

use super::{
    ChartColumn, ChartColumnValues, ChartDataError, ChartDataLimits, ChartDataset, ChartNullBitmap,
    ChartTransformSpec, ChartValue,
};
use crate::UiValue;

#[derive(Clone, Copy, Debug, Default)]
pub struct ChartTransformContext {
    pub limits: ChartDataLimits,
}

/// Trusted Rust transform extension. Implementations receive only typed owned
/// chart data and durable options; no Rhai engine value crosses this boundary.
pub trait HostChartTransform: Send + Sync {
    /// Transform one immutable typed dataset.
    ///
    /// # Errors
    ///
    /// Returns an application-facing diagnostic; no partial output is kept.
    fn transform(
        &self,
        input: &ChartDataset,
        options: &UiValue,
        context: ChartTransformContext,
    ) -> Result<ChartDataset, String>;
}

#[derive(Clone, Default)]
pub struct ChartTransformRegistry {
    inner: Arc<RwLock<BTreeMap<String, Arc<dyn HostChartTransform>>>>,
}

impl std::fmt::Debug for ChartTransformRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let registered = self
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        formatter
            .debug_struct("ChartTransformRegistry")
            .field("registered", &registered)
            .finish()
    }
}

impl ChartTransformRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a trusted transform under a stable ID.
    ///
    /// # Errors
    ///
    /// Returns for an unsafe/duplicate ID or poisoned registry.
    pub fn register(
        &self,
        id: impl Into<String>,
        transform: impl HostChartTransform + 'static,
    ) -> Result<(), ChartTransformError> {
        let id = id.into();
        validate_identifier(&id)?;
        let mut registry = self
            .inner
            .write()
            .map_err(|_| ChartTransformError::Poisoned)?;
        match registry.entry(id.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(Arc::new(transform));
            }
            Entry::Occupied(_) => return Err(ChartTransformError::DuplicateHost(id)),
        }
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Arc<dyn HostChartTransform>, ChartTransformError> {
        self.inner
            .read()
            .map_err(|_| ChartTransformError::Poisoned)?
            .get(id)
            .cloned()
            .ok_or_else(|| ChartTransformError::UnknownHost(id.to_owned()))
    }

    #[cfg(feature = "dev-reload")]
    pub(crate) fn copy_from(&self, source: &Self) {
        let source = source
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        *self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = source;
    }
}

/// Apply a declared transform pipeline in order.
///
/// Each stage produces a fresh validated dataset. The input remains immutable,
/// so a failing stage cannot expose a partially transformed candidate.
///
/// # Errors
///
/// Returns the first data, transform, or Host-extension diagnostic.
pub fn apply_chart_transforms(
    input: &ChartDataset,
    transforms: &[ChartTransformSpec],
    registry: &ChartTransformRegistry,
    context: ChartTransformContext,
) -> Result<ChartDataset, ChartTransformError> {
    let mut current = input.clone();
    for transform in transforms {
        current = match transform {
            ChartTransformSpec::Filter {
                dimension,
                operator,
                value,
            } => filter(&current, dimension, operator, value)?,
            ChartTransformSpec::Sort {
                dimension,
                descending,
            } => sort(&current, dimension, *descending)?,
            ChartTransformSpec::Aggregate {
                group_by,
                dimension,
                operation,
                output,
            } => aggregate(
                &current,
                group_by,
                dimension,
                operation,
                output,
                context.limits,
            )?,
            ChartTransformSpec::Bin {
                dimension,
                bins,
                output_start,
                output_end,
            } => bin(
                &current,
                dimension,
                *bins,
                output_start,
                output_end,
                context.limits,
            )?,
            ChartTransformSpec::Stack {
                group_by,
                dimension,
                output_start,
                output_end,
            } => stack(
                &current,
                group_by,
                dimension,
                output_start,
                output_end,
                context.limits,
            )?,
            ChartTransformSpec::Normalize {
                group_by,
                dimension,
                output,
            } => normalize(&current, group_by, dimension, output, context.limits)?,
            ChartTransformSpec::MovingWindow {
                dimension,
                window,
                operation,
                output,
            } => moving_window(
                &current,
                dimension,
                *window,
                operation,
                output,
                context.limits,
            )?,
            ChartTransformSpec::Downsample { x, y, threshold } => {
                downsample_lttb(&current, x, y, *threshold)?
            }
            ChartTransformSpec::Host { id, options } => registry
                .get(id)?
                .transform(&current, options, context)
                .map_err(|message| ChartTransformError::HostFailed {
                    id: id.clone(),
                    message,
                })?,
        };
    }
    Ok(current)
}

fn filter(
    input: &ChartDataset,
    dimension: &str,
    operator: &str,
    expected: &UiValue,
) -> Result<ChartDataset, ChartTransformError> {
    if !matches!(
        operator,
        "eq" | "ne" | "lt" | "lte" | "gt" | "gte" | "contains"
    ) {
        return Err(ChartTransformError::UnknownFilterOperator(
            operator.to_owned(),
        ));
    }
    let column = require_column(input, dimension)?;
    let expected = ui_scalar(expected)?;
    let indices = (0..input.len())
        .filter(|index| {
            column
                .value(*index)
                .is_some_and(|actual| compare_filter(&actual, operator, &expected))
        })
        .collect::<Vec<_>>();
    Ok(input.select_rows(&indices)?)
}

fn compare_filter(actual: &ChartValue, operator: &str, expected: &ChartValue) -> bool {
    match operator {
        "eq" => compare_values(actual, expected) == Some(Ordering::Equal),
        "ne" => compare_values(actual, expected) != Some(Ordering::Equal),
        "lt" => compare_values(actual, expected) == Some(Ordering::Less),
        "lte" => matches!(
            compare_values(actual, expected),
            Some(Ordering::Less | Ordering::Equal)
        ),
        "gt" => compare_values(actual, expected) == Some(Ordering::Greater),
        "gte" => matches!(
            compare_values(actual, expected),
            Some(Ordering::Greater | Ordering::Equal)
        ),
        "contains" => matches!((actual, expected),
            (ChartValue::String(actual), ChartValue::String(expected)) if actual.contains(expected)),
        _ => false,
    }
}

fn sort(
    input: &ChartDataset,
    dimension: &str,
    descending: bool,
) -> Result<ChartDataset, ChartTransformError> {
    let column = require_column(input, dimension)?;
    let mut indices = (0..input.len()).collect::<Vec<_>>();
    indices.sort_by(|left, right| {
        let order = match (column.value(*left), column.value(*right)) {
            (Some(left), Some(right)) => compare_values(&left, &right).unwrap_or(Ordering::Equal),
            _ => Ordering::Equal,
        };
        if descending { order.reverse() } else { order }
    });
    Ok(input.select_rows(&indices)?)
}

fn aggregate(
    input: &ChartDataset,
    group_by: &[String],
    dimension: &str,
    operation: &str,
    output: &str,
    limits: ChartDataLimits,
) -> Result<ChartDataset, ChartTransformError> {
    validate_output(output)?;
    let value_column = require_column(input, dimension)?;
    let group_columns = group_columns(input, group_by)?;
    let mut groups = BTreeMap::<Vec<TypedGroupKey>, (Vec<ChartValue>, Vec<ChartValue>)>::new();
    for row in 0..input.len() {
        let values = group_columns
            .iter()
            .map(|column| column.value(row).unwrap_or(ChartValue::Null))
            .collect::<Vec<_>>();
        let key = values.iter().map(TypedGroupKey::from).collect::<Vec<_>>();
        let entry = groups.entry(key).or_insert_with(|| (values, Vec::new()));
        if let Some(value) = value_column.value(row)
            && !matches!(value, ChartValue::Null)
        {
            entry.1.push(value);
        }
    }
    let rows = groups
        .into_iter()
        .map(|(_, (key, values))| {
            let mut row = group_by
                .iter()
                .cloned()
                .zip(key)
                .collect::<BTreeMap<_, _>>();
            row.insert(
                output.to_owned(),
                ChartValue::Number(reduce_values(&values, operation)?),
            );
            Ok(row)
        })
        .collect::<Result<Vec<_>, ChartTransformError>>()?;
    if rows.is_empty() {
        let empty = input.select_rows(&[])?;
        let mut columns = group_by
            .iter()
            .filter_map(|name| empty.column(name).cloned())
            .collect::<Vec<_>>();
        columns.push(ChartColumn::new(
            output,
            ChartColumnValues::Number(Vec::<f64>::new().into()),
            ChartNullBitmap::from_validity(Vec::<bool>::new()),
        )?);
        return Ok(ChartDataset::new(input.name(), columns, None, limits)?);
    }
    Ok(ChartDataset::from_chart_rows(
        input.name(),
        &rows,
        None,
        limits,
    )?)
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TypedGroupKey {
    Null,
    Number(u64),
    Integer(i64),
    Timestamp(i64),
    Bool(bool),
    String(String),
}

impl From<&ChartValue> for TypedGroupKey {
    fn from(value: &ChartValue) -> Self {
        match value {
            ChartValue::Null => Self::Null,
            ChartValue::Number(value) => Self::Number(value.to_bits()),
            ChartValue::Integer(value) => Self::Integer(*value),
            ChartValue::Timestamp(value) => Self::Timestamp(*value),
            ChartValue::Bool(value) => Self::Bool(*value),
            ChartValue::String(value) => Self::String(value.clone()),
        }
    }
}

fn reduce_values(values: &[ChartValue], operation: &str) -> Result<f64, ChartTransformError> {
    if operation == "count" {
        return Ok(usize_to_f64(values.len()));
    }
    let numbers = values
        .iter()
        .filter_map(ChartValue::as_number)
        .collect::<Vec<_>>();
    reduce(&numbers, operation)
}

fn reduce(values: &[f64], operation: &str) -> Result<f64, ChartTransformError> {
    let value = match operation {
        "sum" => values.iter().sum(),
        "mean" => {
            if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / usize_to_f64(values.len())
            }
        }
        "min" => values.iter().copied().reduce(f64::min).unwrap_or(0.0),
        "max" => values.iter().copied().reduce(f64::max).unwrap_or(0.0),
        "count" => usize_to_f64(values.len()),
        other => return Err(ChartTransformError::UnknownOperation(other.to_owned())),
    };
    Ok(value)
}

fn bin(
    input: &ChartDataset,
    dimension: &str,
    bins: u16,
    output_start: &str,
    output_end: &str,
    limits: ChartDataLimits,
) -> Result<ChartDataset, ChartTransformError> {
    validate_output(output_start)?;
    validate_output(output_end)?;
    let column = require_column(input, dimension)?;
    let values = (0..input.len())
        .filter_map(|row| column.value(row).and_then(|value| value.as_number()))
        .collect::<Vec<_>>();
    let Some(min) = values.iter().copied().reduce(f64::min) else {
        return Ok(input.clone());
    };
    let max = values.iter().copied().reduce(f64::max).unwrap_or(min);
    let width = if max > min {
        (max - min) / f64::from(bins)
    } else {
        1.0
    };
    let mut rows = input.rows();
    for row in &mut rows {
        let value = row
            .get(dimension)
            .and_then(ChartValue::as_number)
            .unwrap_or(min);
        let index = ((value - min) / width)
            .floor()
            .clamp(0.0, f64::from(bins.saturating_sub(1)));
        let start = min + index * width;
        row.insert(output_start.to_owned(), ChartValue::Number(start));
        row.insert(output_end.to_owned(), ChartValue::Number(start + width));
    }
    Ok(ChartDataset::from_chart_rows(
        input.name(),
        &rows,
        input.key_dimension().map(ToOwned::to_owned),
        limits,
    )?)
}

fn stack(
    input: &ChartDataset,
    group_by: &[String],
    dimension: &str,
    output_start: &str,
    output_end: &str,
    limits: ChartDataLimits,
) -> Result<ChartDataset, ChartTransformError> {
    validate_output(output_start)?;
    validate_output(output_end)?;
    let column = require_column(input, dimension)?;
    let group_columns = group_columns(input, group_by)?;
    let mut positive = BTreeMap::<Vec<String>, f64>::new();
    let mut negative = BTreeMap::<Vec<String>, f64>::new();
    let mut rows = input.rows();
    for (index, row) in rows.iter_mut().enumerate() {
        let key = group_columns
            .iter()
            .map(|column| {
                column
                    .value(index)
                    .unwrap_or(ChartValue::Null)
                    .display_text()
            })
            .collect::<Vec<_>>();
        let value = column
            .value(index)
            .and_then(|value| value.as_number())
            .unwrap_or(0.0);
        let running = if value >= 0.0 {
            positive.entry(key).or_default()
        } else {
            negative.entry(key).or_default()
        };
        let start = *running;
        *running += value;
        row.insert(output_start.to_owned(), ChartValue::Number(start));
        row.insert(output_end.to_owned(), ChartValue::Number(*running));
    }
    Ok(ChartDataset::from_chart_rows(
        input.name(),
        &rows,
        input.key_dimension().map(ToOwned::to_owned),
        limits,
    )?)
}

fn normalize(
    input: &ChartDataset,
    group_by: &[String],
    dimension: &str,
    output: &str,
    limits: ChartDataLimits,
) -> Result<ChartDataset, ChartTransformError> {
    validate_output(output)?;
    let column = require_column(input, dimension)?;
    let group_columns = group_columns(input, group_by)?;
    let mut totals = BTreeMap::<Vec<String>, f64>::new();
    for row in 0..input.len() {
        let key = group_columns
            .iter()
            .map(|column| column.value(row).unwrap_or(ChartValue::Null).display_text())
            .collect::<Vec<_>>();
        let value = column
            .value(row)
            .and_then(|value| value.as_number())
            .unwrap_or(0.0)
            .abs();
        *totals.entry(key).or_default() += value;
    }
    let mut rows = input.rows();
    for (index, row) in rows.iter_mut().enumerate() {
        let key = group_columns
            .iter()
            .map(|column| {
                column
                    .value(index)
                    .unwrap_or(ChartValue::Null)
                    .display_text()
            })
            .collect::<Vec<_>>();
        let total = totals.get(&key).copied().unwrap_or(0.0);
        let value = column
            .value(index)
            .and_then(|value| value.as_number())
            .unwrap_or(0.0);
        row.insert(
            output.to_owned(),
            ChartValue::Number(if total > 0.0 { value / total } else { 0.0 }),
        );
    }
    Ok(ChartDataset::from_chart_rows(
        input.name(),
        &rows,
        input.key_dimension().map(ToOwned::to_owned),
        limits,
    )?)
}

fn moving_window(
    input: &ChartDataset,
    dimension: &str,
    window: usize,
    operation: &str,
    output: &str,
    limits: ChartDataLimits,
) -> Result<ChartDataset, ChartTransformError> {
    validate_output(output)?;
    if window == 0 || window > limits.max_rows {
        return Err(ChartTransformError::InvalidWindow(window));
    }
    let column = require_column(input, dimension)?;
    let mut active = VecDeque::with_capacity(window);
    let mut rows = input.rows();
    for (index, row) in rows.iter_mut().enumerate() {
        if let Some(value) = column.value(index).and_then(|value| value.as_number()) {
            active.push_back(value);
        }
        while active.len() > window {
            active.pop_front();
        }
        row.insert(
            output.to_owned(),
            ChartValue::Number(reduce(active.make_contiguous(), operation)?),
        );
    }
    Ok(ChartDataset::from_chart_rows(
        input.name(),
        &rows,
        input.key_dimension().map(ToOwned::to_owned),
        limits,
    )?)
}

/// Largest-Triangle-Three-Buckets downsampling. Returned rows keep their
/// original key values, so hit testing can resolve back to source identity.
fn downsample_lttb(
    input: &ChartDataset,
    x: &str,
    y: &str,
    threshold: usize,
) -> Result<ChartDataset, ChartTransformError> {
    if threshold < 3 || input.len() <= threshold {
        return Ok(input.clone());
    }
    let x = require_column(input, x)?;
    let y = require_column(input, y)?;
    let categorical_x = matches!(
        x.data_type(),
        super::ChartDataType::String | super::ChartDataType::Bool
    );
    let mut segments = Vec::<Vec<(usize, f64, f64)>>::new();
    let mut current = Vec::new();
    let mut separators = Vec::<usize>::new();
    let mut pending_separator = None;
    for index in 0..input.len() {
        let x = x.value(index).and_then(|value| {
            if categorical_x {
                (value != ChartValue::Null).then(|| usize_to_f64(index))
            } else {
                value.as_number()
            }
        });
        let y = y.value(index).and_then(|value| value.as_number());
        if let (Some(x), Some(y)) = (x, y) {
            if current.is_empty()
                && !segments.is_empty()
                && let Some(separator) = pending_separator.take()
            {
                separators.push(separator);
            }
            current.push((index, x, y));
        } else if !current.is_empty() {
            segments.push(std::mem::take(&mut current));
            pending_separator = Some(index);
        } else if !segments.is_empty() && pending_separator.is_none() {
            pending_separator = Some(index);
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }
    if segments.is_empty() {
        return Ok(input.select_rows(&[])?);
    }

    let max_segments = threshold.div_ceil(2).max(1);
    let kept = evenly_spaced_indexes(segments.len(), segments.len().min(max_segments));
    let separator_count = kept.len().saturating_sub(1);
    let point_budget = threshold.saturating_sub(separator_count).max(1);
    let lengths = kept
        .iter()
        .map(|index| segments[*index].len())
        .collect::<Vec<_>>();
    let allocations = allocate_segment_budget(&lengths, point_budget);
    let mut selected = Vec::with_capacity(threshold);
    for (position, (segment_index, allocation)) in kept.iter().copied().zip(allocations).enumerate()
    {
        selected.extend(sample_segment(&segments[segment_index], allocation));
        if let Some(next) = kept.get(position + 1) {
            let separator_index = segment_index.min(separators.len().saturating_sub(1));
            if segment_index < *next
                && let Some(separator) = separators.get(separator_index)
            {
                selected.push(*separator);
            }
        }
    }
    selected.sort_unstable();
    selected.dedup();
    debug_assert!(selected.len() <= threshold);
    Ok(input.select_rows(&selected)?)
}

fn evenly_spaced_indexes(len: usize, count: usize) -> Vec<usize> {
    if count >= len {
        return (0..len).collect();
    }
    if count <= 1 {
        return vec![len / 2];
    }
    (0..count)
        .map(|index| index * (len - 1) / (count - 1))
        .collect()
}

fn allocate_segment_budget(lengths: &[usize], budget: usize) -> Vec<usize> {
    let mut allocated = vec![1; lengths.len()];
    let requested = budget.saturating_sub(lengths.len());
    let capacities = lengths
        .iter()
        .map(|length| length.saturating_sub(1))
        .collect::<Vec<_>>();
    let total_capacity = capacities.iter().sum::<usize>();
    if requested == 0 || total_capacity == 0 {
        return allocated;
    }
    let remaining = requested.min(total_capacity);
    let mut remainders = Vec::with_capacity(lengths.len());
    let mut used = 0_usize;
    for (index, capacity) in capacities.iter().copied().enumerate() {
        let weighted = remaining as u128 * capacity as u128;
        let extra = usize::try_from(weighted / total_capacity as u128).unwrap_or(capacity);
        allocated[index] += extra.min(capacity);
        used += extra.min(capacity);
        remainders.push((index, weighted % total_capacity as u128));
    }
    remainders.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
    for (index, _) in remainders.into_iter().take(remaining.saturating_sub(used)) {
        if allocated[index] < lengths[index] {
            allocated[index] += 1;
        }
    }
    allocated
}

fn sample_segment(points: &[(usize, f64, f64)], threshold: usize) -> Vec<usize> {
    match threshold {
        0 => Vec::new(),
        1 => vec![points[points.len() / 2].0],
        2 if points.len() > 1 => vec![points[0].0, points[points.len() - 1].0],
        _ => lttb_segment(points, threshold),
    }
}

fn lttb_segment(points: &[(usize, f64, f64)], threshold: usize) -> Vec<usize> {
    if points.len() <= threshold || threshold < 3 {
        return points.iter().map(|point| point.0).collect();
    }
    let bucket_size =
        usize_to_f64(points.len().saturating_sub(2)) / usize_to_f64(threshold.saturating_sub(2));
    let mut selected = Vec::with_capacity(threshold);
    selected.push(points[0].0);
    let mut anchor = 0;
    for bucket in 0..threshold.saturating_sub(2) {
        let next_start =
            ((usize_to_f64(bucket + 1) * bucket_size).floor() as usize + 1).min(points.len() - 1);
        let next_end =
            ((usize_to_f64(bucket + 2) * bucket_size).floor() as usize + 1).min(points.len());
        let average = if next_start < next_end {
            let slice = &points[next_start..next_end];
            let count = usize_to_f64(slice.len());
            (
                slice.iter().map(|point| point.1).sum::<f64>() / count,
                slice.iter().map(|point| point.2).sum::<f64>() / count,
            )
        } else {
            (points[points.len() - 1].1, points[points.len() - 1].2)
        };
        let start =
            ((usize_to_f64(bucket) * bucket_size).floor() as usize + 1).min(points.len() - 1);
        let end = ((usize_to_f64(bucket + 1) * bucket_size).floor() as usize + 1)
            .min(points.len() - 1)
            .max(start + 1);
        let anchor_point = (points[anchor].1, points[anchor].2);
        let (candidate, _) = (start..end)
            .map(|index| {
                let point = (points[index].1, points[index].2);
                let area = ((anchor_point.0 - average.0) * (point.1 - anchor_point.1)
                    - (anchor_point.0 - point.0) * (average.1 - anchor_point.1))
                    .abs();
                (index, area)
            })
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .unwrap_or((start, 0.0));
        selected.push(points[candidate].0);
        anchor = candidate;
    }
    selected.push(points[points.len() - 1].0);
    selected
}

fn require_column<'a>(
    input: &'a ChartDataset,
    dimension: &str,
) -> Result<&'a super::ChartColumn, ChartTransformError> {
    input
        .column(dimension)
        .ok_or_else(|| ChartTransformError::MissingDimension(dimension.to_owned()))
}

fn group_columns<'a>(
    input: &'a ChartDataset,
    dimensions: &[String],
) -> Result<Vec<&'a super::ChartColumn>, ChartTransformError> {
    dimensions
        .iter()
        .map(|dimension| require_column(input, dimension))
        .collect()
}

fn compare_values(left: &ChartValue, right: &ChartValue) -> Option<Ordering> {
    match (left, right) {
        (ChartValue::Null, ChartValue::Null) => Some(Ordering::Equal),
        (ChartValue::Null, _) => Some(Ordering::Less),
        (_, ChartValue::Null) => Some(Ordering::Greater),
        (ChartValue::String(left), ChartValue::String(right)) => Some(left.cmp(right)),
        (ChartValue::Bool(left), ChartValue::Bool(right)) => Some(left.cmp(right)),
        _ => left
            .as_number()
            .zip(right.as_number())
            .map(|(left, right)| left.total_cmp(&right)),
    }
}

fn ui_scalar(value: &UiValue) -> Result<ChartValue, ChartTransformError> {
    Ok(match value {
        UiValue::Null => ChartValue::Null,
        UiValue::Bool(value) => ChartValue::Bool(*value),
        UiValue::Integer(value) => ChartValue::Integer(*value),
        UiValue::Float(value) if value.is_finite() => ChartValue::Number(*value),
        UiValue::String(value) => ChartValue::String(value.clone()),
        _ => return Err(ChartTransformError::InvalidFilterValue),
    })
}

fn validate_identifier(value: &str) -> Result<(), ChartTransformError> {
    if !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic())
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        Ok(())
    } else {
        Err(ChartTransformError::InvalidHostId(value.to_owned()))
    }
}

fn validate_output(value: &str) -> Result<(), ChartTransformError> {
    validate_identifier(value).map_err(|_| ChartTransformError::InvalidOutput(value.to_owned()))
}

#[allow(clippy::cast_precision_loss)]
fn usize_to_f64(value: usize) -> f64 {
    value as f64
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ChartTransformError {
    #[error(transparent)]
    Data(#[from] ChartDataError),
    #[error("chart transform dimension `{0}` does not exist")]
    MissingDimension(String),
    #[error("chart transform row {row} requires numeric x/y values")]
    NonNumeric { row: usize },
    #[error("chart filter value must be a durable scalar")]
    InvalidFilterValue,
    #[error("chart filter operator `{0}` is not supported")]
    UnknownFilterOperator(String),
    #[error("chart aggregate operation `{0}` is not supported")]
    UnknownOperation(String),
    #[error("chart transform output `{0}` must be a safe identifier")]
    InvalidOutput(String),
    #[error("chart moving-window size {0} is invalid")]
    InvalidWindow(usize),
    #[error("host chart transform ID `{0}` must be a safe identifier")]
    InvalidHostId(String),
    #[error("host chart transform `{0}` is already registered")]
    DuplicateHost(String),
    #[error("host chart transform `{0}` is not registered")]
    UnknownHost(String),
    #[error("host chart transform `{id}` failed: {message}")]
    HostFailed { id: String, message: String },
    #[error("chart transform registry lock was poisoned")]
    Poisoned,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn dataset(rows: usize) -> ChartDataset {
        let rows = (0..rows)
            .map(|index| {
                BTreeMap::from([
                    ("id".to_owned(), ChartValue::String(format!("p{index}"))),
                    ("x".to_owned(), ChartValue::Number(usize_to_f64(index))),
                    (
                        "y".to_owned(),
                        ChartValue::Number((usize_to_f64(index) / 5.0).sin()),
                    ),
                    (
                        "group".to_owned(),
                        ChartValue::String(if index % 2 == 0 { "a" } else { "b" }.to_owned()),
                    ),
                ])
            })
            .collect::<Vec<_>>();
        ChartDataset::from_chart_rows(
            "main",
            &rows,
            Some("id".to_owned()),
            ChartDataLimits::default(),
        )
        .unwrap()
    }

    #[test]
    fn lttb_preserves_first_last_and_original_datum_keys() {
        let source = dataset(1_000);
        let sampled = apply_chart_transforms(
            &source,
            &[ChartTransformSpec::Downsample {
                x: "x".to_owned(),
                y: "y".to_owned(),
                threshold: 100,
            }],
            &ChartTransformRegistry::new(),
            ChartTransformContext::default(),
        )
        .unwrap();

        assert_eq!(sampled.len(), 100);
        assert_eq!(sampled.key(0).as_deref(), Some("p0"));
        assert_eq!(sampled.key(99).as_deref(), Some("p999"));
        let keys = (0..sampled.len())
            .filter_map(|index| sampled.key(index))
            .collect::<BTreeSet<_>>();
        assert_eq!(keys.len(), sampled.len());
    }

    #[test]
    fn transform_failure_keeps_the_input_immutable() {
        let source = dataset(8);
        let result = apply_chart_transforms(
            &source,
            &[ChartTransformSpec::MovingWindow {
                dimension: "missing".to_owned(),
                window: 3,
                operation: "mean".to_owned(),
                output: "average".to_owned(),
            }],
            &ChartTransformRegistry::new(),
            ChartTransformContext::default(),
        );
        assert!(matches!(
            result,
            Err(ChartTransformError::MissingDimension(_))
        ));
        assert_eq!(source.len(), 8);
        assert!(source.column("average").is_none());
    }

    #[test]
    fn lttb_collapses_null_runs_and_respects_the_total_budget() {
        let rows = (0..1_000)
            .map(|index| {
                BTreeMap::from([
                    ("id".to_owned(), ChartValue::String(format!("p{index}"))),
                    ("x".to_owned(), ChartValue::Number(usize_to_f64(index))),
                    (
                        "y".to_owned(),
                        if index % 3 == 1 {
                            ChartValue::Null
                        } else {
                            ChartValue::Number(usize_to_f64(index))
                        },
                    ),
                ])
            })
            .collect::<Vec<_>>();
        let source = ChartDataset::from_chart_rows(
            "main",
            &rows,
            Some("id".to_owned()),
            ChartDataLimits::default(),
        )
        .unwrap();
        let sampled = apply_chart_transforms(
            &source,
            &[ChartTransformSpec::Downsample {
                x: "x".to_owned(),
                y: "y".to_owned(),
                threshold: 100,
            }],
            &ChartTransformRegistry::new(),
            ChartTransformContext::default(),
        )
        .unwrap();
        assert!(sampled.len() <= 100);
        assert!(
            (0..sampled.len()).any(|index| sampled
                .column("y")
                .unwrap()
                .value(index)
                .is_some_and(|value| value == ChartValue::Null)),
            "sampling removed every gap separator"
        );
    }
}
