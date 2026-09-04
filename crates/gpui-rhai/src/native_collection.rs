use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Arc, Mutex};

use rhai::{
    CustomType, Dynamic, EvalAltResult, FuncRegistration, ImmutableString, Map, Position,
    TypeBuilder,
};
use thiserror::Error;

use crate::{ComponentInstancePath, UiValue};

const MAX_TABLE_ORDER_CACHE_ENTRIES: usize = 64;

#[derive(Clone)]
pub struct NativeCollection {
    source: Arc<NativeCollectionSource>,
    order: Arc<Vec<NativeCollectionEntry>>,
    sticky_headers: Arc<BTreeSet<usize>>,
    projection: Option<Arc<TableProjection>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum NativeCollectionEntry {
    Row(usize),
    Group(GroupEntry),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GroupEntry {
    key: String,
    value: String,
    count: usize,
    collapsed: bool,
}

#[derive(Clone)]
struct TableOrder {
    entries: Arc<Vec<NativeCollectionEntry>>,
    sticky_headers: Arc<BTreeSet<usize>>,
}

struct NativeCollectionSource {
    key_field: String,
    rows: Vec<UiValue>,
    keys: Vec<String>,
    sorted_orders: Mutex<BTreeMap<SortSpec, Arc<Vec<usize>>>>,
    table_orders: Mutex<BTreeMap<TableOrderSpec, TableOrder>>,
}

impl fmt::Debug for NativeCollectionSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeCollectionSource")
            .field("key_field", &self.key_field)
            .field("rows", &self.rows.len())
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for NativeCollection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeCollection")
            .field("key_field", &self.source.key_field)
            .field("len", &self.order.len())
            .field("projected", &self.projection.is_some())
            .finish_non_exhaustive()
    }
}

impl PartialEq for NativeCollection {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.source, &other.source)
            && (Arc::ptr_eq(&self.order, &other.order) || self.order == other.order)
            && (Arc::ptr_eq(&self.sticky_headers, &other.sticky_headers)
                || self.sticky_headers == other.sticky_headers)
            && self.projection == other.projection
    }
}

impl NativeCollection {
    /// Build an immutable Rust-owned keyed collection.
    ///
    /// # Errors
    ///
    /// Returns an error unless every row contains one unique scalar key.
    pub fn new(
        key_field: impl Into<String>,
        rows: impl IntoIterator<Item = BTreeMap<String, UiValue>>,
    ) -> Result<Self, NativeCollectionError> {
        Self::from_values(key_field, rows.into_iter().map(UiValue::Map).collect())
    }

    /// Build a collection from already-normalized [`UiValue`] maps.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsafe key field, non-map rows, missing keys,
    /// unsupported key values, or duplicate keys.
    pub fn from_values(
        key_field: impl Into<String>,
        rows: Vec<UiValue>,
    ) -> Result<Self, NativeCollectionError> {
        let key_field = key_field.into();
        validate_name(&key_field, "key field")?;
        let mut seen = BTreeSet::new();
        let keys = rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let UiValue::Map(row) = row else {
                    return Err(NativeCollectionError::RowNotMap(index));
                };
                let key = row
                    .get(&key_field)
                    .ok_or_else(|| NativeCollectionError::MissingKey {
                        index,
                        field: key_field.clone(),
                    })?;
                let key = scalar_text(key).ok_or_else(|| NativeCollectionError::InvalidKey {
                    index,
                    field: key_field.clone(),
                })?;
                if key.is_empty() {
                    return Err(NativeCollectionError::InvalidKey {
                        index,
                        field: key_field.clone(),
                    });
                }
                if !seen.insert(key.clone()) {
                    return Err(NativeCollectionError::DuplicateKey(key));
                }
                Ok(key)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let order = Arc::new((0..rows.len()).map(NativeCollectionEntry::Row).collect());
        Ok(Self {
            source: Arc::new(NativeCollectionSource {
                key_field,
                rows,
                keys,
                sorted_orders: Mutex::new(BTreeMap::new()),
                table_orders: Mutex::new(BTreeMap::new()),
            }),
            order,
            sticky_headers: Arc::new(BTreeSet::new()),
            projection: None,
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.order.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    #[must_use]
    pub fn key_field(&self) -> &str {
        &self.source.key_field
    }

    pub(crate) fn key(&self, index: usize) -> Option<&str> {
        match self.order.get(index)? {
            NativeCollectionEntry::Row(source) => self.source.keys.get(*source).map(String::as_str),
            NativeCollectionEntry::Group(group) => Some(group.key.as_str()),
        }
    }

    pub(crate) fn item(&self, index: usize) -> Result<Option<UiValue>, NativeCollectionError> {
        let Some(entry) = self.order.get(index) else {
            return Ok(None);
        };
        match entry {
            NativeCollectionEntry::Row(source_index) => {
                let row = self
                    .source
                    .rows
                    .get(*source_index)
                    .ok_or(NativeCollectionError::CorruptOrder(*source_index))?;
                self.projection.as_ref().map_or_else(
                    || Ok(Some(row.clone())),
                    |projection| projection.project_row(row, index).map(Some),
                )
            }
            NativeCollectionEntry::Group(group) => self
                .projection
                .as_ref()
                .ok_or(NativeCollectionError::UnexpectedGroupEntry)
                .map(|projection| Some(projection.project_group(group))),
        }
    }

    pub(crate) fn sticky_headers(&self) -> Arc<BTreeSet<usize>> {
        Arc::clone(&self.sticky_headers)
    }

    pub(crate) fn table_view(&self, config: Map) -> Result<Self, NativeCollectionError> {
        let config = UiValue::from_dynamic(Dynamic::from_map(config))
            .map_err(|error| NativeCollectionError::InvalidTableConfig(error.to_string()))?;
        let projection = TableProjection::decode(config, self.key_field())?;
        let order = self.table_order(&projection)?;
        Ok(Self {
            source: Arc::clone(&self.source),
            order: order.entries,
            sticky_headers: order.sticky_headers,
            projection: Some(Arc::new(projection)),
        })
    }

    fn table_order(
        &self,
        projection: &TableProjection,
    ) -> Result<TableOrder, NativeCollectionError> {
        if self.projection.is_none() && projection.sort.is_none() && projection.group_by.is_none() {
            return Ok(TableOrder {
                entries: Arc::clone(&self.order),
                sticky_headers: Arc::clone(&self.sticky_headers),
            });
        }
        let spec = TableOrderSpec::from(projection);
        if let Some(order) = self
            .source
            .table_orders
            .lock()
            .map_err(|_| NativeCollectionError::Poisoned)?
            .get(&spec)
            .cloned()
        {
            return Ok(order);
        }
        let rows = projection.sort.as_ref().map_or_else(
            || Ok(Arc::new((0..self.source.rows.len()).collect())),
            |sort| self.sorted_order(sort),
        )?;
        let entries = Arc::new(projection.grouped_order(&self.source, rows.as_ref())?);
        let sticky_headers = Arc::new(
            entries
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| {
                    matches!(entry, NativeCollectionEntry::Group(_)).then_some(index)
                })
                .collect(),
        );
        let order = TableOrder {
            entries,
            sticky_headers,
        };
        let mut cache = self
            .source
            .table_orders
            .lock()
            .map_err(|_| NativeCollectionError::Poisoned)?;
        if cache.len() >= MAX_TABLE_ORDER_CACHE_ENTRIES
            && let Some(victim) = cache.keys().next().cloned()
        {
            cache.remove(&victim);
        }
        cache.insert(spec, order.clone());
        Ok(order)
    }

    fn sorted_order(&self, sort: &SortSpec) -> Result<Arc<Vec<usize>>, NativeCollectionError> {
        if let Some(cached) = self
            .source
            .sorted_orders
            .lock()
            .map_err(|_| NativeCollectionError::Poisoned)?
            .get(sort)
            .cloned()
        {
            return Ok(cached);
        }
        let mut order = (0..self.source.rows.len()).collect::<Vec<_>>();
        let mut failure = None;
        order.sort_by(|left, right| {
            let result = compare_rows(
                &self.source.rows[*left],
                &self.source.rows[*right],
                &sort.key,
            );
            match result {
                Ok(ordering) => {
                    let ordering = if sort.descending {
                        ordering.reverse()
                    } else {
                        ordering
                    };
                    ordering.then_with(|| left.cmp(right))
                }
                Err(error) => {
                    failure.get_or_insert(error);
                    left.cmp(right)
                }
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
        let order = Arc::new(order);
        self.source
            .sorted_orders
            .lock()
            .map_err(|_| NativeCollectionError::Poisoned)?
            .insert(sort.clone(), Arc::clone(&order));
        Ok(order)
    }
}

impl CustomType for NativeCollection {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("NativeCollection")
            .with_get("len", |collection: &mut Self| {
                i64::try_from(collection.len()).unwrap_or(i64::MAX)
            })
            .with_get("key_field", |collection: &mut Self| {
                ImmutableString::from(collection.key_field().to_owned())
            })
            .with_fn("to_string", |collection: &mut Self| {
                format!("NativeCollection(len={})", collection.len())
            });
    }
}

pub(crate) fn register_native_collection_api(engine: &mut rhai::Engine) {
    engine.build_type::<NativeCollection>();
    FuncRegistration::new("is_native_collection")
        .in_global_namespace()
        .register_into_engine(engine, |value: Dynamic| value.is::<NativeCollection>());
    FuncRegistration::new("native_table_view")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |collection: NativeCollection,
             config: Map|
             -> Result<NativeCollection, Box<EvalAltResult>> {
                collection.table_view(config).map_err(|error| {
                    Box::new(EvalAltResult::ErrorRuntime(
                        error.to_string().into(),
                        Position::NONE,
                    ))
                })
            },
        );
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SortSpec {
    key: String,
    descending: bool,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TableOrderSpec {
    sort: Option<SortSpec>,
    group_by: Option<String>,
    collapsed_groups: BTreeSet<String>,
}

impl From<&TableProjection> for TableOrderSpec {
    fn from(projection: &TableProjection) -> Self {
        Self {
            sort: projection.sort.clone(),
            group_by: projection.group_by.clone(),
            collapsed_groups: projection.collapsed_groups.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct TableProjection {
    label: String,
    row_key: String,
    columns: Vec<TableColumn>,
    selected_keys: Vec<String>,
    selected: BTreeSet<String>,
    selection_mode: SelectionMode,
    striped: bool,
    row_height: f64,
    sort: Option<SortSpec>,
    group_by: Option<String>,
    collapsed_groups: BTreeSet<String>,
    group_toggle: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct TableColumn {
    key: String,
    width: UiValue,
    align: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectionMode {
    None,
    Single,
    Multiple,
}

impl TableProjection {
    fn decode(config: UiValue, source_key: &str) -> Result<Self, NativeCollectionError> {
        let UiValue::Map(mut config) = config else {
            return Err(NativeCollectionError::InvalidTableConfig(
                "configuration must be a map".to_owned(),
            ));
        };
        let row_key = take_string(&mut config, "row_key")?;
        if row_key != source_key {
            return Err(NativeCollectionError::KeyFieldMismatch {
                source_key: source_key.to_owned(),
                requested: row_key,
            });
        }
        let label = take_string(&mut config, "label")?;
        let columns = take_array(&mut config, "columns")?
            .into_iter()
            .map(TableColumn::decode)
            .collect::<Result<Vec<_>, _>>()?;
        let selected_keys = take_array(&mut config, "selected_keys")?
            .into_iter()
            .enumerate()
            .map(|(index, value)| match value {
                UiValue::String(value) => Ok(value),
                _ => Err(NativeCollectionError::InvalidTableConfig(format!(
                    "selected_keys[{index}] must be a string"
                ))),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let selected = selected_keys.iter().cloned().collect();
        let selection_mode = match take_string(&mut config, "selection_mode")?.as_str() {
            "none" => SelectionMode::None,
            "single" => SelectionMode::Single,
            "multiple" => SelectionMode::Multiple,
            value => {
                return Err(NativeCollectionError::InvalidTableConfig(format!(
                    "unknown selection mode `{value}`"
                )));
            }
        };
        let striped = take_bool(&mut config, "striped")?;
        let row_height = take_number(&mut config, "row_height")?;
        if !row_height.is_finite() || row_height <= 0.0 {
            return Err(NativeCollectionError::InvalidTableConfig(
                "row_height must be finite and positive".to_owned(),
            ));
        }
        let sort = decode_sort(config.remove("sort"))?;
        let (group_by, collapsed_groups, group_toggle) = decode_grouping(&mut config, &columns)?;
        if !config.is_empty() {
            return Err(NativeCollectionError::InvalidTableConfig(format!(
                "unknown configuration fields: {}",
                config.keys().cloned().collect::<Vec<_>>().join(", ")
            )));
        }
        Ok(Self {
            label,
            row_key,
            columns,
            selected_keys,
            selected,
            selection_mode,
            striped,
            row_height,
            sort,
            group_by,
            collapsed_groups,
            group_toggle,
        })
    }

    fn grouped_order(
        &self,
        source: &NativeCollectionSource,
        rows: &[usize],
    ) -> Result<Vec<NativeCollectionEntry>, NativeCollectionError> {
        let Some(group_by) = &self.group_by else {
            return Ok(rows
                .iter()
                .copied()
                .map(NativeCollectionEntry::Row)
                .collect());
        };
        let mut order = Vec::new();
        let mut groups = BTreeMap::<String, Vec<usize>>::new();
        for source_index in rows {
            let row = source
                .rows
                .get(*source_index)
                .ok_or(NativeCollectionError::CorruptOrder(*source_index))?;
            let value = match row_field(row, group_by) {
                Some(UiValue::String(value)) if !value.is_empty() => value.clone(),
                _ => {
                    return Err(NativeCollectionError::InvalidGroupField {
                        index: *source_index,
                        field: group_by.clone(),
                    });
                }
            };
            if !groups.contains_key(&value) {
                order.push(value.clone());
            }
            groups.entry(value).or_default().push(*source_index);
        }
        let source_keys = source
            .keys
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut header_keys = BTreeSet::new();
        let mut entries = Vec::with_capacity(rows.len().saturating_add(order.len()));
        for value in order {
            let rows = groups.remove(&value).unwrap_or_default();
            let key = unique_group_key(&value, &source_keys, &mut header_keys);
            let collapsed = self.collapsed_groups.contains(&value);
            entries.push(NativeCollectionEntry::Group(GroupEntry {
                key,
                value,
                count: rows.len(),
                collapsed,
            }));
            if !collapsed {
                entries.extend(rows.into_iter().map(NativeCollectionEntry::Row));
            }
        }
        Ok(entries)
    }

    fn project_row(&self, row: &UiValue, index: usize) -> Result<UiValue, NativeCollectionError> {
        let UiValue::Map(row) = row else {
            return Err(NativeCollectionError::RowNotMap(index));
        };
        let key = row.get(&self.row_key).and_then(scalar_text);
        let key = key.ok_or_else(|| {
            NativeCollectionError::InvalidTableConfig("projected row key is unavailable".to_owned())
        })?;
        let selected = self.selected.contains(&key);
        let cells = self
            .columns
            .iter()
            .map(|column| {
                let text = row
                    .get(&column.key)
                    .map_or_else(String::new, display_scalar);
                UiValue::Map(BTreeMap::from([
                    ("key".to_owned(), UiValue::String(column.key.clone())),
                    ("text".to_owned(), UiValue::String(text)),
                    ("width".to_owned(), column.width.clone()),
                    ("align".to_owned(), UiValue::String(column.align.clone())),
                ]))
            })
            .collect();
        let selection = match self.selection_mode {
            SelectionMode::None => UiValue::Null,
            SelectionMode::Single if selected => UiValue::Array(Vec::new()),
            SelectionMode::Single => UiValue::Array(vec![UiValue::String(key.clone())]),
            SelectionMode::Multiple => {
                let mut next = self
                    .selected_keys
                    .iter()
                    .filter(|candidate| *candidate != &key)
                    .cloned()
                    .map(UiValue::String)
                    .collect::<Vec<_>>();
                if !selected {
                    next.push(UiValue::String(key.clone()));
                }
                UiValue::Array(next)
            }
        };
        Ok(UiValue::Map(BTreeMap::from([
            ("kind".to_owned(), UiValue::String("row".to_owned())),
            ("key".to_owned(), UiValue::String(key.clone())),
            (
                "label".to_owned(),
                UiValue::String(format!("{} row {}", self.label, index + 1)),
            ),
            ("cells".to_owned(), UiValue::Array(cells)),
            ("selected".to_owned(), UiValue::Bool(selected)),
            ("selection".to_owned(), selection),
            (
                "striped".to_owned(),
                UiValue::Bool(self.striped && index % 2 == 1),
            ),
            ("height".to_owned(), UiValue::Float(self.row_height)),
        ])))
    }

    fn project_group(&self, group: &GroupEntry) -> UiValue {
        UiValue::Map(BTreeMap::from([
            ("kind".to_owned(), UiValue::String("group".to_owned())),
            ("key".to_owned(), UiValue::String(group.key.clone())),
            ("group".to_owned(), UiValue::String(group.value.clone())),
            (
                "count".to_owned(),
                UiValue::Integer(i64::try_from(group.count).unwrap_or(i64::MAX)),
            ),
            ("collapsed".to_owned(), UiValue::Bool(group.collapsed)),
            ("toggle".to_owned(), UiValue::Bool(self.group_toggle)),
            ("height".to_owned(), UiValue::Float(self.row_height)),
        ]))
    }
}

fn decode_sort(value: Option<UiValue>) -> Result<Option<SortSpec>, NativeCollectionError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let UiValue::Map(mut sort) = value else {
        if value == UiValue::Null {
            return Ok(None);
        }
        return Err(NativeCollectionError::InvalidTableConfig(
            "sort must be null or a map".to_owned(),
        ));
    };
    let key = take_string(&mut sort, "key")?;
    let direction = take_string(&mut sort, "direction")?;
    if !sort.is_empty() {
        return Err(NativeCollectionError::InvalidTableConfig(
            "sort contains unknown fields".to_owned(),
        ));
    }
    let descending = match direction.as_str() {
        "ascending" => false,
        "descending" => true,
        _ => {
            return Err(NativeCollectionError::InvalidTableConfig(format!(
                "unknown sort direction `{direction}`"
            )));
        }
    };
    Ok(Some(SortSpec { key, descending }))
}

fn decode_grouping(
    config: &mut BTreeMap<String, UiValue>,
    columns: &[TableColumn],
) -> Result<(Option<String>, BTreeSet<String>, bool), NativeCollectionError> {
    let group_by = take_optional_string(config, "group_by")?;
    if let Some(group_by) = &group_by
        && !columns.iter().any(|column| &column.key == group_by)
    {
        return Err(NativeCollectionError::InvalidTableConfig(format!(
            "group_by `{group_by}` is not a declared column"
        )));
    }
    let collapsed = if config.contains_key("collapsed_groups") {
        take_string_set(config, "collapsed_groups")?
    } else {
        BTreeSet::new()
    };
    if group_by.is_none() && !collapsed.is_empty() {
        return Err(NativeCollectionError::InvalidTableConfig(
            "collapsed_groups requires group_by".to_owned(),
        ));
    }
    let toggle = if config.contains_key("group_toggle") {
        take_bool(config, "group_toggle")?
    } else {
        false
    };
    Ok((group_by, collapsed, toggle))
}

fn unique_group_key(
    value: &str,
    source_keys: &BTreeSet<&str>,
    header_keys: &mut BTreeSet<String>,
) -> String {
    let mut key = format!("__gpui_rhai_group__:{value}");
    while source_keys.contains(key.as_str()) || !header_keys.insert(key.clone()) {
        key.insert(0, '_');
    }
    key
}

impl TableColumn {
    fn decode(value: UiValue) -> Result<Self, NativeCollectionError> {
        let UiValue::Map(mut column) = value else {
            return Err(NativeCollectionError::InvalidTableConfig(
                "each column must be a map".to_owned(),
            ));
        };
        let key = take_string(&mut column, "key")?;
        let width = column.remove("width").ok_or_else(|| {
            NativeCollectionError::InvalidTableConfig("column.width is required".to_owned())
        })?;
        let align = match column.remove("align") {
            None => "start".to_owned(),
            Some(UiValue::String(value)) => value,
            Some(_) => {
                return Err(NativeCollectionError::InvalidTableConfig(
                    "column.align must be a string".to_owned(),
                ));
            }
        };
        Ok(Self { key, width, align })
    }
}

fn compare_rows(
    left: &UiValue,
    right: &UiValue,
    key: &str,
) -> Result<Ordering, NativeCollectionError> {
    let left = row_field(left, key)
        .ok_or_else(|| NativeCollectionError::MissingSortField(key.to_owned()))?;
    let right = row_field(right, key)
        .ok_or_else(|| NativeCollectionError::MissingSortField(key.to_owned()))?;
    compare_values(left, right)
        .ok_or_else(|| NativeCollectionError::UnsortableField(key.to_owned()))
}

fn row_field<'a>(row: &'a UiValue, key: &str) -> Option<&'a UiValue> {
    match row {
        UiValue::Map(row) => row.get(key),
        _ => None,
    }
}

fn compare_values(left: &UiValue, right: &UiValue) -> Option<Ordering> {
    match (left, right) {
        (UiValue::Null, UiValue::Null) => Some(Ordering::Equal),
        (UiValue::Bool(left), UiValue::Bool(right)) => Some(left.cmp(right)),
        (UiValue::Integer(left), UiValue::Integer(right)) => Some(left.cmp(right)),
        (UiValue::Float(left), UiValue::Float(right)) => left.partial_cmp(right),
        (UiValue::Integer(left), UiValue::Float(right)) => integer_float(*left).partial_cmp(right),
        (UiValue::Float(left), UiValue::Integer(right)) => left.partial_cmp(&integer_float(*right)),
        (UiValue::String(left), UiValue::String(right)) => Some(left.cmp(right)),
        _ => None,
    }
}

fn scalar_text(value: &UiValue) -> Option<String> {
    match value {
        UiValue::Bool(value) => Some(value.to_string()),
        UiValue::Integer(value) => Some(value.to_string()),
        UiValue::Float(value) if value.is_finite() => Some(value.to_string()),
        UiValue::String(value) => Some(value.clone()),
        UiValue::Null
        | UiValue::Float(_)
        | UiValue::Array(_)
        | UiValue::Map(_)
        | UiValue::Handle(_) => None,
    }
}

fn display_scalar(value: &UiValue) -> String {
    scalar_text(value).unwrap_or_default()
}

fn take_string(
    values: &mut BTreeMap<String, UiValue>,
    name: &str,
) -> Result<String, NativeCollectionError> {
    match values.remove(name) {
        Some(UiValue::String(value)) => Ok(value),
        _ => Err(NativeCollectionError::InvalidTableConfig(format!(
            "{name} must be a string"
        ))),
    }
}

fn take_optional_string(
    values: &mut BTreeMap<String, UiValue>,
    name: &str,
) -> Result<Option<String>, NativeCollectionError> {
    match values.remove(name) {
        None | Some(UiValue::Null) => Ok(None),
        Some(UiValue::String(value)) if !value.is_empty() => Ok(Some(value)),
        _ => Err(NativeCollectionError::InvalidTableConfig(format!(
            "{name} must be null or a non-empty string"
        ))),
    }
}

fn take_string_set(
    values: &mut BTreeMap<String, UiValue>,
    name: &str,
) -> Result<BTreeSet<String>, NativeCollectionError> {
    take_array(values, name)?
        .into_iter()
        .enumerate()
        .map(|(index, value)| match value {
            UiValue::String(value) if !value.is_empty() => Ok(value),
            _ => Err(NativeCollectionError::InvalidTableConfig(format!(
                "{name}[{index}] must be a non-empty string"
            ))),
        })
        .collect()
}

fn take_array(
    values: &mut BTreeMap<String, UiValue>,
    name: &str,
) -> Result<Vec<UiValue>, NativeCollectionError> {
    match values.remove(name) {
        Some(UiValue::Array(value)) => Ok(value),
        _ => Err(NativeCollectionError::InvalidTableConfig(format!(
            "{name} must be an array"
        ))),
    }
}

fn take_bool(
    values: &mut BTreeMap<String, UiValue>,
    name: &str,
) -> Result<bool, NativeCollectionError> {
    match values.remove(name) {
        Some(UiValue::Bool(value)) => Ok(value),
        _ => Err(NativeCollectionError::InvalidTableConfig(format!(
            "{name} must be a bool"
        ))),
    }
}

fn take_number(
    values: &mut BTreeMap<String, UiValue>,
    name: &str,
) -> Result<f64, NativeCollectionError> {
    match values.remove(name) {
        Some(UiValue::Float(value)) => Ok(value),
        Some(UiValue::Integer(value)) => Ok(integer_float(value)),
        _ => Err(NativeCollectionError::InvalidTableConfig(format!(
            "{name} must be a number"
        ))),
    }
}

fn integer_float(value: i64) -> f64 {
    value.to_string().parse().unwrap_or_else(|_| {
        if value.is_negative() {
            f64::MIN
        } else {
            f64::MAX
        }
    })
}

fn validate_name(name: &str, label: &'static str) -> Result<(), NativeCollectionError> {
    if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        Err(NativeCollectionError::InvalidName(label, name.to_owned()))
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct NativeCollectionRegistry {
    collections: BTreeMap<String, NativeCollection>,
    readers: BTreeMap<String, BTreeSet<ComponentInstancePath>>,
}

impl NativeCollectionRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one application-owned collection before initial render.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsafe or duplicate name.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        collection: NativeCollection,
    ) -> Result<(), NativeCollectionError> {
        let name = name.into();
        validate_name(&name, "collection name")?;
        if self.collections.contains_key(&name) {
            return Err(NativeCollectionError::DuplicateCollection(name));
        }
        self.collections.insert(name, collection);
        Ok(())
    }

    /// Replace an existing collection and return its exact subscribed readers.
    ///
    /// # Errors
    ///
    /// Returns an error when the collection is unknown.
    pub fn replace(
        &mut self,
        name: &str,
        collection: NativeCollection,
    ) -> Result<BTreeSet<ComponentInstancePath>, NativeCollectionError> {
        let current = self
            .collections
            .get_mut(name)
            .ok_or_else(|| NativeCollectionError::UnknownCollection(name.to_owned()))?;
        if current == &collection {
            return Ok(BTreeSet::new());
        }
        *current = collection;
        Ok(self.readers.get(name).cloned().unwrap_or_default())
    }

    pub(crate) fn read_tracked(
        &mut self,
        reader: &ComponentInstancePath,
        name: &str,
    ) -> Result<NativeCollection, NativeCollectionError> {
        let collection = self
            .collections
            .get(name)
            .cloned()
            .ok_or_else(|| NativeCollectionError::UnknownCollection(name.to_owned()))?;
        self.readers
            .entry(name.to_owned())
            .or_default()
            .insert(reader.clone());
        Ok(collection)
    }

    pub(crate) fn reset_reader(&mut self, reader: &ComponentInstancePath) {
        for readers in self.readers.values_mut() {
            readers.remove(reader);
        }
        self.readers.retain(|_, readers| !readers.is_empty());
    }

    pub(crate) fn retain_reader_scope(
        &mut self,
        root: &ComponentInstancePath,
        active: &BTreeSet<ComponentInstancePath>,
    ) {
        for readers in self.readers.values_mut() {
            readers.retain(|reader| {
                !reader.is_within(root) || reader == root || active.contains(reader)
            });
        }
        self.readers.retain(|_, readers| !readers.is_empty());
    }

    pub(crate) fn remove_reader_scope(&mut self, root: &ComponentInstancePath) {
        for readers in self.readers.values_mut() {
            readers.retain(|reader| !reader.is_within(root));
        }
        self.readers.retain(|_, readers| !readers.is_empty());
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum VirtualCollectionData {
    Values(Vec<UiValue>),
    Native(NativeCollection),
}

impl From<Vec<UiValue>> for VirtualCollectionData {
    fn from(values: Vec<UiValue>) -> Self {
        Self::Values(values)
    }
}

impl From<NativeCollection> for VirtualCollectionData {
    fn from(collection: NativeCollection) -> Self {
        Self::Native(collection)
    }
}

impl FromIterator<UiValue> for VirtualCollectionData {
    fn from_iter<T: IntoIterator<Item = UiValue>>(iter: T) -> Self {
        Self::Values(iter.into_iter().collect())
    }
}

impl VirtualCollectionData {
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Values(values) => values.len(),
            Self::Native(collection) => collection.len(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn sticky_headers(&self) -> Arc<BTreeSet<usize>> {
        match self {
            Self::Values(_) => Arc::new(BTreeSet::new()),
            Self::Native(collection) => collection.sticky_headers(),
        }
    }

    pub(crate) fn item(&self, index: usize) -> Result<Option<UiValue>, NativeCollectionError> {
        match self {
            Self::Values(values) => Ok(values.get(index).cloned()),
            Self::Native(collection) => collection.item(index),
        }
    }

    pub(crate) fn key(&self, index: usize) -> Option<&str> {
        match self {
            Self::Values(values) => values.get(index).and_then(|item| match item {
                UiValue::Map(item) => match item.get("key") {
                    Some(UiValue::String(key)) => Some(key.as_str()),
                    _ => None,
                },
                _ => None,
            }),
            Self::Native(collection) => collection.key(index),
        }
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum NativeCollectionError {
    #[error("invalid {0} `{1}`")]
    InvalidName(&'static str, String),
    #[error("native collection row {0} must be a map")]
    RowNotMap(usize),
    #[error("native collection row {index} is missing key field `{field}`")]
    MissingKey { index: usize, field: String },
    #[error("native collection row {index} key field `{field}` must be a non-empty scalar")]
    InvalidKey { index: usize, field: String },
    #[error("native collection key `{0}` is duplicated")]
    DuplicateKey(String),
    #[error("native collection `{0}` is already registered")]
    DuplicateCollection(String),
    #[error("native collection `{0}` is not registered")]
    UnknownCollection(String),
    #[error(
        "native collection key field `{source_key}` does not match requested row key `{requested}`"
    )]
    KeyFieldMismatch {
        source_key: String,
        requested: String,
    },
    #[error("native collection sort field `{0}` is missing")]
    MissingSortField(String),
    #[error("native collection sort field `{0}` is not a consistently comparable scalar")]
    UnsortableField(String),
    #[error("native collection order references missing source row {0}")]
    CorruptOrder(usize),
    #[error("native collection contains a group header outside a table projection")]
    UnexpectedGroupEntry,
    #[error("native collection row {index} group field `{field}` must be a non-empty string")]
    InvalidGroupField { index: usize, field: String },
    #[error("invalid native Table configuration: {0}")]
    InvalidTableConfig(String),
    #[error("native collection cache is poisoned")]
    Poisoned,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> NativeCollection {
        NativeCollection::new(
            "id",
            [
                BTreeMap::from([
                    ("id".to_owned(), UiValue::String("b".to_owned())),
                    ("score".to_owned(), UiValue::Integer(2)),
                ]),
                BTreeMap::from([
                    ("id".to_owned(), UiValue::String("a".to_owned())),
                    ("score".to_owned(), UiValue::Integer(1)),
                ]),
            ],
        )
        .unwrap()
    }

    fn grouped_rows() -> NativeCollection {
        NativeCollection::new(
            "id",
            [
                BTreeMap::from([
                    ("id".to_owned(), UiValue::String("b".to_owned())),
                    ("track".to_owned(), UiValue::String("track-b".to_owned())),
                    ("score".to_owned(), UiValue::Integer(2)),
                ]),
                BTreeMap::from([
                    ("id".to_owned(), UiValue::String("a".to_owned())),
                    ("track".to_owned(), UiValue::String("track-a".to_owned())),
                    ("score".to_owned(), UiValue::Integer(3)),
                ]),
                BTreeMap::from([
                    ("id".to_owned(), UiValue::String("c".to_owned())),
                    ("track".to_owned(), UiValue::String("track-b".to_owned())),
                    ("score".to_owned(), UiValue::Integer(1)),
                ]),
            ],
        )
        .unwrap()
    }

    fn table_config(collapsed: &[&str]) -> Map {
        let columns = ["id", "track", "score"]
            .into_iter()
            .map(|key| {
                UiValue::Map(BTreeMap::from([
                    ("key".to_owned(), UiValue::String(key.to_owned())),
                    ("width".to_owned(), UiValue::Integer(80)),
                ]))
            })
            .collect();
        UiValue::Map(BTreeMap::from([
            ("row_key".to_owned(), UiValue::String("id".to_owned())),
            ("label".to_owned(), UiValue::String("Clusters".to_owned())),
            ("columns".to_owned(), UiValue::Array(columns)),
            ("selected_keys".to_owned(), UiValue::Array(Vec::new())),
            (
                "selection_mode".to_owned(),
                UiValue::String("multiple".to_owned()),
            ),
            ("striped".to_owned(), UiValue::Bool(true)),
            ("row_height".to_owned(), UiValue::Float(30.0)),
            (
                "sort".to_owned(),
                UiValue::Map(BTreeMap::from([
                    ("key".to_owned(), UiValue::String("score".to_owned())),
                    (
                        "direction".to_owned(),
                        UiValue::String("ascending".to_owned()),
                    ),
                ])),
            ),
            ("group_by".to_owned(), UiValue::String("track".to_owned())),
            (
                "collapsed_groups".to_owned(),
                UiValue::Array(
                    collapsed
                        .iter()
                        .map(|value| UiValue::String((*value).to_owned()))
                        .collect(),
                ),
            ),
            ("group_toggle".to_owned(), UiValue::Bool(true)),
        ]))
        .into_dynamic()
        .cast::<Map>()
    }

    #[test]
    fn validates_keys_and_caches_rust_sort_order() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NativeCollection>();
        let collection = rows();
        let sort = SortSpec {
            key: "score".to_owned(),
            descending: false,
        };
        let first = collection.sorted_order(&sort).unwrap();
        let second = collection.sorted_order(&sort).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(collection.source.keys[first[0]], "a");
    }

    #[test]
    fn tracked_replacement_invalidates_only_collection_readers() {
        let mut registry = NativeCollectionRegistry::new();
        registry.register("accounts", rows()).unwrap();
        let reader = ComponentInstancePath::root("View", "main");
        let _ = registry.read_tracked(&reader, "accounts").unwrap();
        let changed = NativeCollection::new(
            "id",
            [BTreeMap::from([(
                "id".to_owned(),
                UiValue::String("next".to_owned()),
            )])],
        )
        .unwrap();
        assert_eq!(
            registry.replace("accounts", changed).unwrap(),
            [reader].into()
        );
    }

    #[test]
    fn grouped_table_projection_flattens_headers_and_sorted_rows() {
        let source = grouped_rows();
        let grouped = source.table_view(table_config(&[])).unwrap();
        let mut selected_config = table_config(&[]);
        selected_config.insert(
            "selected_keys".into(),
            Dynamic::from_array(vec![Dynamic::from("a")]),
        );
        let reused = source.table_view(selected_config).unwrap();
        assert!(Arc::ptr_eq(&grouped.order, &reused.order));
        assert_eq!(grouped.len(), 5);
        assert_eq!(grouped.sticky_headers().as_ref(), &BTreeSet::from([0, 3]));
        let values = (0..grouped.len())
            .map(|index| grouped.item(index).unwrap().unwrap())
            .collect::<Vec<_>>();
        let field = |value: &UiValue, name: &str| match value {
            UiValue::Map(value) => value[name].clone(),
            _ => panic!("projected table item must be a map"),
        };
        assert_eq!(field(&values[0], "kind"), UiValue::String("group".into()));
        assert_eq!(
            field(&values[0], "group"),
            UiValue::String("track-b".into())
        );
        assert_eq!(field(&values[0], "count"), UiValue::Integer(2));
        assert_eq!(field(&values[1], "key"), UiValue::String("c".into()));
        assert_eq!(field(&values[2], "key"), UiValue::String("b".into()));
        assert_eq!(
            field(&values[3], "group"),
            UiValue::String("track-a".into())
        );
        assert_eq!(field(&values[4], "key"), UiValue::String("a".into()));
    }

    #[test]
    fn grouped_table_projection_collapses_rows_but_retains_header_counts() {
        let grouped = grouped_rows()
            .table_view(table_config(&["track-b"]))
            .unwrap();
        assert_eq!(grouped.len(), 3);
        assert_eq!(grouped.sticky_headers().as_ref(), &BTreeSet::from([0, 1]));
        let UiValue::Map(first) = grouped.item(0).unwrap().unwrap() else {
            panic!("group header must be a map");
        };
        assert_eq!(first["group"], UiValue::String("track-b".into()));
        assert_eq!(first["count"], UiValue::Integer(2));
        assert_eq!(first["collapsed"], UiValue::Bool(true));
        assert_eq!(grouped.key(2), Some("a"));
    }
}
