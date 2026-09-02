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

#[derive(Clone)]
pub struct NativeCollection {
    source: Arc<NativeCollectionSource>,
    order: Arc<Vec<usize>>,
    projection: Option<Arc<TableProjection>>,
}

struct NativeCollectionSource {
    key_field: String,
    rows: Vec<UiValue>,
    keys: Vec<String>,
    sorted_orders: Mutex<BTreeMap<SortSpec, Arc<Vec<usize>>>>,
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
        let order = Arc::new((0..rows.len()).collect());
        Ok(Self {
            source: Arc::new(NativeCollectionSource {
                key_field,
                rows,
                keys,
                sorted_orders: Mutex::new(BTreeMap::new()),
            }),
            order,
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
        self.order
            .get(index)
            .and_then(|source| self.source.keys.get(*source))
            .map(String::as_str)
    }

    pub(crate) fn item(&self, index: usize) -> Result<Option<UiValue>, NativeCollectionError> {
        let Some(source_index) = self.order.get(index).copied() else {
            return Ok(None);
        };
        let row = self
            .source
            .rows
            .get(source_index)
            .ok_or(NativeCollectionError::CorruptOrder(source_index))?;
        self.projection.as_ref().map_or_else(
            || Ok(Some(row.clone())),
            |projection| projection.project(row, index).map(Some),
        )
    }

    pub(crate) fn table_view(&self, config: Map) -> Result<Self, NativeCollectionError> {
        let config = UiValue::from_dynamic(Dynamic::from_map(config))
            .map_err(|error| NativeCollectionError::InvalidTableConfig(error.to_string()))?;
        let projection = TableProjection::decode(config, self.key_field())?;
        let order = projection.sort.as_ref().map_or_else(
            || Ok(Arc::clone(&self.order)),
            |sort| self.sorted_order(sort),
        )?;
        Ok(Self {
            source: Arc::clone(&self.source),
            order,
            projection: Some(Arc::new(projection)),
        })
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
        let sort = match config.remove("sort") {
            None | Some(UiValue::Null) => None,
            Some(UiValue::Map(mut sort)) => {
                let key = take_string(&mut sort, "key")?;
                let direction = take_string(&mut sort, "direction")?;
                if !sort.is_empty() {
                    return Err(NativeCollectionError::InvalidTableConfig(
                        "sort contains unknown fields".to_owned(),
                    ));
                }
                Some(SortSpec {
                    key,
                    descending: match direction.as_str() {
                        "ascending" => false,
                        "descending" => true,
                        _ => {
                            return Err(NativeCollectionError::InvalidTableConfig(format!(
                                "unknown sort direction `{direction}`"
                            )));
                        }
                    },
                })
            }
            Some(_) => {
                return Err(NativeCollectionError::InvalidTableConfig(
                    "sort must be null or a map".to_owned(),
                ));
            }
        };
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
        })
    }

    fn project(&self, row: &UiValue, index: usize) -> Result<UiValue, NativeCollectionError> {
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
}
