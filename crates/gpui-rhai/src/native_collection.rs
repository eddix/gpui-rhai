use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

use rhai::{
    CustomType, Dynamic, EvalAltResult, FuncRegistration, ImmutableString, Map, Position,
    TypeBuilder,
};
use thiserror::Error;

use crate::{ComponentInstancePath, UiValue};

const MAX_TABLE_ORDER_CACHE_ENTRIES: usize = 64;
const MAX_FUZZY_ORDER_CACHE_ENTRIES: usize = 64;

#[derive(Clone)]
pub struct NativeCollection {
    source: Arc<NativeCollectionSource>,
    order: Arc<Vec<NativeCollectionEntry>>,
    sticky_headers: Arc<BTreeSet<usize>>,
    projection: Option<Arc<CollectionProjection>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum NativeCollectionEntry {
    Row(usize),
    Group(GroupEntry),
}

#[derive(Clone, Debug, PartialEq)]
enum CollectionProjection {
    Table(TableProjection),
    List(BTreeSet<String>),
    Fuzzy(FuzzyProjection),
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
    fuzzy_orders: Mutex<BTreeMap<FuzzyOrderSpec, Arc<Vec<NativeCollectionEntry>>>>,
    list_validation: OnceLock<Result<(), NativeCollectionError>>,
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
                fuzzy_orders: Mutex::new(BTreeMap::new()),
                list_validation: OnceLock::new(),
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
                    |projection| {
                        projection
                            .project_row(row, index, self.key(index))
                            .map(Some)
                    },
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
            projection: Some(Arc::new(CollectionProjection::Table(projection))),
        })
    }

    fn list_view(&self, selected: BTreeSet<String>) -> Result<Self, NativeCollectionError> {
        self.source
            .list_validation
            .get_or_init(|| {
                for row in &self.source.rows {
                    list_row(row, "", &BTreeSet::new())?;
                }
                Ok(())
            })
            .clone()?;
        Ok(Self {
            source: Arc::clone(&self.source),
            order: Arc::clone(&self.order),
            sticky_headers: Arc::new(BTreeSet::new()),
            projection: Some(Arc::new(CollectionProjection::List(selected))),
        })
    }

    pub(crate) fn fuzzy_view(&self, config: Map) -> Result<Self, NativeCollectionError> {
        let projection = FuzzyProjection::decode(config)?;
        let order = self.fuzzy_order(&projection)?;
        Ok(Self {
            source: Arc::clone(&self.source),
            order,
            sticky_headers: Arc::new(BTreeSet::new()),
            projection: Some(Arc::new(CollectionProjection::Fuzzy(projection))),
        })
    }

    fn fuzzy_order(
        &self,
        projection: &FuzzyProjection,
    ) -> Result<Arc<Vec<NativeCollectionEntry>>, NativeCollectionError> {
        let spec = FuzzyOrderSpec::from(projection);
        if let Some(order) = self
            .source
            .fuzzy_orders
            .lock()
            .map_err(|_| NativeCollectionError::Poisoned)?
            .get(&spec)
            .cloned()
        {
            return Ok(order);
        }
        let order = Arc::new(projection.grouped_order(&self.source)?);
        let mut cache = self
            .source
            .fuzzy_orders
            .lock()
            .map_err(|_| NativeCollectionError::Poisoned)?;
        if cache.len() >= MAX_FUZZY_ORDER_CACHE_ENTRIES
            && let Some(victim) = cache.keys().next().cloned()
        {
            cache.remove(&victim);
        }
        cache.insert(spec, Arc::clone(&order));
        Ok(order)
    }

    pub(crate) fn fuzzy_edge(&self, last: bool) -> Result<String, NativeCollectionError> {
        let projection = self.fuzzy_projection()?;
        let rows = if last {
            Box::new(self.order.iter().rev()) as Box<dyn Iterator<Item = &NativeCollectionEntry>>
        } else {
            Box::new(self.order.iter()) as Box<dyn Iterator<Item = &NativeCollectionEntry>>
        };
        for entry in rows {
            if let NativeCollectionEntry::Row(index) = entry
                && !projection.row_disabled(&self.source, *index)?
            {
                return self
                    .source
                    .keys
                    .get(*index)
                    .cloned()
                    .ok_or(NativeCollectionError::CorruptOrder(*index));
            }
        }
        Ok(String::new())
    }

    pub(crate) fn fuzzy_adjacent(
        &self,
        active: &str,
        step: i64,
    ) -> Result<String, NativeCollectionError> {
        let projection = self.fuzzy_projection()?;
        let mut enabled = Vec::new();
        for entry in self.order.iter() {
            if let NativeCollectionEntry::Row(index) = entry
                && !projection.row_disabled(&self.source, *index)?
            {
                enabled.push(*index);
            }
        }
        if enabled.is_empty() {
            return Ok(String::new());
        }
        let current = enabled.iter().position(|index| {
            self.source
                .keys
                .get(*index)
                .is_some_and(|key| key == active)
        });
        let next = match step.cmp(&0) {
            Ordering::Equal => current.unwrap_or(0),
            Ordering::Less => (current.unwrap_or(0) + enabled.len() - 1) % enabled.len(),
            Ordering::Greater => (current.unwrap_or(0) + 1) % enabled.len(),
        };
        self.source
            .keys
            .get(enabled[next])
            .cloned()
            .ok_or(NativeCollectionError::CorruptOrder(enabled[next]))
    }

    /// Keyboard targets of a table view, by the same rule as an array Table:
    /// the current row is the first displayed row in `selected`; Up and Down
    /// move one row and stop at the ends; with no current row both go to the
    /// first. Group headers are skipped. One scan in native code, so a key
    /// press does not enumerate the collection in Rhai.
    pub(crate) fn table_neighbors(&self, selected: &BTreeSet<String>) -> TableNeighbors {
        // (display index, key) of each displayed row; group headers take display
        // indices too.
        let rows = self
            .order
            .iter()
            .enumerate()
            .filter_map(|(display, entry)| match entry {
                NativeCollectionEntry::Row(index) => {
                    if matches!(
                        self.projection.as_deref(),
                        Some(CollectionProjection::List(_))
                    ) && row_field(&self.source.rows[*index], "disabled")
                        == Some(&UiValue::Bool(true))
                    {
                        return None;
                    }
                    self.source.keys.get(*index).map(|key| (display, key))
                }
                NativeCollectionEntry::Group(_) => None,
            })
            .collect::<Vec<_>>();
        let current = rows
            .iter()
            .position(|(_, key)| selected.contains(key.as_str()));
        let at = |index: usize| rows.get(index).map(|(_, key)| (*key).clone());
        let last = rows.len().saturating_sub(1);
        TableNeighbors {
            first: at(0),
            last: at(last),
            previous: at(current.map_or(0, |current| current.saturating_sub(1))),
            next: at(current.map_or(0, |current| (current + 1).min(last))),
            current: current.and_then(at),
            current_index: current
                .and_then(|current| rows.get(current).map(|(display, _)| *display)),
        }
    }

    fn fuzzy_projection(&self) -> Result<&FuzzyProjection, NativeCollectionError> {
        match self.projection.as_deref() {
            Some(CollectionProjection::Fuzzy(projection)) => Ok(projection),
            _ => Err(NativeCollectionError::NotFuzzyProjection),
        }
    }

    fn table_order(
        &self,
        projection: &TableProjection,
    ) -> Result<TableOrder, NativeCollectionError> {
        if self.projection.is_none()
            && projection.sort.is_none()
            && projection.group_by.is_none()
            && projection.query.is_empty()
            && projection.page_size.is_none()
        {
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
        let rows = projection.visible_rows(&self.source, rows.as_ref());
        let entries = Arc::new(projection.grouped_order(&self.source, &rows)?);
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
        // Validate even one-row views, before the comparator can short-circuit.
        let mut sample = None;
        for row in &self.source.rows {
            if let Some(value) = row_field(row, &sort.key).filter(|value| **value != UiValue::Null)
            {
                let reference = sample.get_or_insert(value);
                if compare_values(reference, value).is_none() {
                    return Err(NativeCollectionError::UnsortableField(sort.key.clone()));
                }
            }
        }
        let mut order = (0..self.source.rows.len()).collect::<Vec<_>>();
        order.sort_by(|left, right| {
            let field = |index: usize| {
                row_field(&self.source.rows[index], &sort.key)
                    .filter(|value| **value != UiValue::Null)
            };
            let ordering = match (field(*left), field(*right)) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(left), Some(right)) => {
                    let ordering = compare_values(left, right).expect("validated scalar sort type");
                    if sort.descending {
                        ordering.reverse()
                    } else {
                        ordering
                    }
                }
            };
            ordering.then_with(|| left.cmp(right))
        });
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

fn register_list_collection_api(engine: &mut rhai::Engine) {
    register_list_navigation_api(engine);
    FuncRegistration::new("native_list_view")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |collection: NativeCollection,
             selected: rhai::Array|
             -> Result<NativeCollection, Box<EvalAltResult>> {
                collection
                    .list_view(
                        selected
                            .into_iter()
                            .filter_map(|value| value.into_string().ok())
                            .collect(),
                    )
                    .map_err(|error| Box::new(native_collection_runtime_error(&error)))
            },
        );
    FuncRegistration::new("native_list_view")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |rows: rhai::Array,
             selected: rhai::Array|
             -> Result<NativeCollection, Box<EvalAltResult>> {
                let rows = rows
                    .into_iter()
                    .map(|value| {
                        UiValue::from_dynamic(value).map_err(|error| {
                            Box::new(EvalAltResult::ErrorRuntime(
                                error.to_string().into(),
                                Position::NONE,
                            ))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                NativeCollection::from_values("key", rows)
                    .and_then(|collection| {
                        collection.list_view(
                            selected
                                .into_iter()
                                .filter_map(|value| value.into_string().ok())
                                .collect(),
                        )
                    })
                    .map_err(|error| Box::new(native_collection_runtime_error(&error)))
            },
        );
}

pub(crate) fn register_native_collection_api(engine: &mut rhai::Engine) {
    engine.build_type::<NativeCollection>();
    register_list_collection_api(engine);
    register_table_collection_api(engine);
    register_fuzzy_collection_api(engine);
    FuncRegistration::new("is_native_collection")
        .in_global_namespace()
        .register_into_engine(engine, |value: Dynamic| value.is::<NativeCollection>());
}

fn register_table_collection_api(engine: &mut rhai::Engine) {
    register_array_table_api(engine);
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
    FuncRegistration::new("native_table_neighbors")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |collection: NativeCollection, selected: rhai::Array| -> Map {
                let selected = selected
                    .into_iter()
                    .filter_map(|key| key.into_string().ok())
                    .collect::<BTreeSet<_>>();
                collection.table_neighbors(&selected).into_map()
            },
        );
}

fn register_list_navigation_api(engine: &mut rhai::Engine) {
    FuncRegistration::new("native_list_neighbors")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |collection: NativeCollection,
             selected: rhai::Array|
             -> Result<Map, Box<EvalAltResult>> {
                let selected: BTreeSet<_> = selected
                    .into_iter()
                    .filter_map(|value| value.into_string().ok())
                    .collect();
                let collection = collection
                    .list_view(selected.clone())
                    .map_err(|error| Box::new(native_collection_runtime_error(&error)))?;
                let neighbors = collection.table_neighbors(&selected);
                let children = neighbors
                    .current_index
                    .and_then(|index| collection.item(index).ok().flatten())
                    .is_some_and(|item| {
                        row_field(&item, "has_children") == Some(&UiValue::Bool(true))
                    });
                let mut result = neighbors.into_map();
                result.insert("has_children".into(), Dynamic::from(children));
                Ok(result)
            },
        );
}

fn register_array_table_api(engine: &mut rhai::Engine) {
    FuncRegistration::new("native_table_view")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |rows: rhai::Array, config: Map| -> Result<NativeCollection, Box<EvalAltResult>> {
                let key = config
                    .get("row_key")
                    .and_then(|value| value.clone().try_cast::<ImmutableString>())
                    .ok_or_else(|| {
                        Box::new(native_collection_runtime_error(
                            &NativeCollectionError::InvalidTableConfig(
                                "row_key must be a string".into(),
                            ),
                        ))
                    })?;
                let values = rows
                    .into_iter()
                    .map(|value| {
                        UiValue::from_dynamic(value).map_err(|error| {
                            Box::new(EvalAltResult::ErrorRuntime(
                                error.to_string().into(),
                                Position::NONE,
                            ))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                NativeCollection::from_values(key.as_str(), values)
                    .and_then(|rows| rows.table_view(config))
                    .map_err(|error| Box::new(native_collection_runtime_error(&error)))
            },
        );
}

fn register_fuzzy_collection_api(engine: &mut rhai::Engine) {
    FuncRegistration::new("native_fuzzy_view")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |collection: NativeCollection,
             config: Map|
             -> Result<NativeCollection, Box<EvalAltResult>> {
                collection
                    .fuzzy_view(config)
                    .map_err(|error| Box::new(native_collection_runtime_error(&error)))
            },
        );
    FuncRegistration::new("native_fuzzy_edge")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |collection: NativeCollection,
             last: bool|
             -> Result<ImmutableString, Box<EvalAltResult>> {
                collection
                    .fuzzy_edge(last)
                    .map(ImmutableString::from)
                    .map_err(|error| Box::new(native_collection_runtime_error(&error)))
            },
        );
    FuncRegistration::new("native_fuzzy_adjacent")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |collection: NativeCollection,
             active: ImmutableString,
             step: i64|
             -> Result<ImmutableString, Box<EvalAltResult>> {
                collection
                    .fuzzy_adjacent(active.as_str(), step)
                    .map(ImmutableString::from)
                    .map_err(|error| Box::new(native_collection_runtime_error(&error)))
            },
        );
}

/// The row keys a Table's navigation keys move to; `None` when the view has
/// no rows (or no current row).
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct TableNeighbors {
    pub first: Option<String>,
    pub last: Option<String>,
    pub previous: Option<String>,
    pub next: Option<String>,
    pub current: Option<String>,
    /// The current row's index among the displayed items.
    pub current_index: Option<usize>,
}

impl TableNeighbors {
    fn into_map(self) -> Map {
        let key = |value: Option<String>| value.map_or(Dynamic::UNIT, Dynamic::from);
        Map::from([
            ("first".into(), key(self.first)),
            ("last".into(), key(self.last)),
            ("previous".into(), key(self.previous)),
            ("next".into(), key(self.next)),
            ("current".into(), key(self.current)),
            (
                "current_index".into(),
                self.current_index
                    .and_then(|index| rhai::INT::try_from(index).ok())
                    .map_or(Dynamic::UNIT, Dynamic::from),
            ),
        ])
    }
}

fn native_collection_runtime_error(error: &NativeCollectionError) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(error.to_string().into(), Position::NONE)
}

// Shared List/ColumnBrowser row normalization. Work remains native for large
// Host collections; Rhai sees only the realized virtual rows and key targets.
fn list_row(
    row: &UiValue,
    key: &str,
    selected: &BTreeSet<String>,
) -> Result<UiValue, NativeCollectionError> {
    let UiValue::Map(row) = row else {
        return Err(NativeCollectionError::RowNotMap(0));
    };
    if !matches!(row.get("title"), Some(UiValue::String(_))) {
        return Err(NativeCollectionError::InvalidTableConfig(
            "List title must be a string".into(),
        ));
    }
    let mut item = row.clone();
    item.insert("key".into(), UiValue::String(key.into()));
    for name in ["secondary", "meta", "badge"] {
        item.entry(name.into()).or_insert(UiValue::Null);
    }
    for name in ["disabled", "has_children"] {
        let value = item.entry(name.into()).or_insert(UiValue::Bool(false));
        if !matches!(value, UiValue::Bool(_)) {
            return Err(NativeCollectionError::InvalidTableConfig(format!(
                "List {name} must be a bool"
            )));
        }
    }
    for name in ["secondary", "meta"] {
        if !matches!(item[name], UiValue::Null | UiValue::String(_)) {
            return Err(NativeCollectionError::InvalidTableConfig(format!(
                "List {name} must be a string or null"
            )));
        }
    }
    if let Some(UiValue::Map(badge)) = item.get_mut("badge") {
        if !matches!(badge.get("text"), Some(UiValue::String(_))) {
            return Err(NativeCollectionError::InvalidTableConfig(
                "List badge.text must be a string".into(),
            ));
        }
        badge
            .entry("variant".into())
            .or_insert(UiValue::String("neutral".into()));
        badge.entry("dot".into()).or_insert(UiValue::Bool(true));
        let Some(UiValue::String(variant)) = badge.get("variant") else {
            return Err(NativeCollectionError::InvalidTableConfig(
                "List badge.variant must be a string".into(),
            ));
        };
        validate_badge_variant(variant)?;
        if !matches!(badge.get("dot"), Some(UiValue::Bool(_))) {
            return Err(NativeCollectionError::InvalidTableConfig(
                "List badge.dot must be a bool".into(),
            ));
        }
    } else if item["badge"] != UiValue::Null {
        return Err(NativeCollectionError::InvalidTableConfig(
            "List badge must be a map or null".into(),
        ));
    }
    item.insert("selected".into(), UiValue::Bool(selected.contains(key)));
    Ok(UiValue::Map(item))
}

impl CollectionProjection {
    fn project_row(
        &self,
        row: &UiValue,
        index: usize,
        key: Option<&str>,
    ) -> Result<UiValue, NativeCollectionError> {
        match self {
            Self::Table(projection) => projection.project_row(row, index),
            Self::List(selected) => list_row(row, key.unwrap_or_default(), selected),
            Self::Fuzzy(projection) => projection.project_row(row, index, key),
        }
    }

    fn project_group(&self, group: &GroupEntry) -> UiValue {
        match self {
            Self::Table(projection) => projection.project_group(group),
            Self::List(_) => UiValue::Null,
            Self::Fuzzy(projection) => projection.project_group(group),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct FuzzyOrderSpec {
    query: String,
    label_field: String,
    keywords_field: String,
    group_field: String,
}

impl From<&FuzzyProjection> for FuzzyOrderSpec {
    fn from(projection: &FuzzyProjection) -> Self {
        Self {
            query: projection.query.clone(),
            label_field: projection.label_field.clone(),
            keywords_field: projection.keywords_field.clone(),
            group_field: projection.group_field.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct FuzzyProjection {
    query: String,
    label_field: String,
    keywords_field: String,
    group_field: String,
    shortcut_field: String,
    disabled_field: String,
    active: String,
    row_height: f64,
}

impl FuzzyProjection {
    fn decode(config: Map) -> Result<Self, NativeCollectionError> {
        let UiValue::Map(mut config) = UiValue::from_dynamic(Dynamic::from_map(config))
            .map_err(|error| NativeCollectionError::InvalidFuzzyConfig(error.to_string()))?
        else {
            return Err(NativeCollectionError::InvalidFuzzyConfig(
                "configuration must be a map".to_owned(),
            ));
        };
        let query = fuzzy_take_string(&mut config, "query")?.to_lowercase();
        let label_field = fuzzy_take_string(&mut config, "label_field")?;
        let keywords_field = fuzzy_take_string(&mut config, "keywords_field")?;
        let group_field = fuzzy_take_string(&mut config, "group_field")?;
        let shortcut_field = fuzzy_take_string(&mut config, "shortcut_field")?;
        let disabled_field = fuzzy_take_string(&mut config, "disabled_field")?;
        let active = fuzzy_take_string(&mut config, "active")?;
        let row_height = fuzzy_take_number(&mut config, "row_height")?;
        if !row_height.is_finite() || row_height <= 0.0 {
            return Err(NativeCollectionError::InvalidFuzzyConfig(
                "row_height must be finite and positive".to_owned(),
            ));
        }
        for (field, label) in [
            (&label_field, "label field"),
            (&keywords_field, "keywords field"),
            (&group_field, "group field"),
            (&shortcut_field, "shortcut field"),
            (&disabled_field, "disabled field"),
        ] {
            validate_name(field, label)?;
        }
        if !config.is_empty() {
            return Err(NativeCollectionError::InvalidFuzzyConfig(format!(
                "unknown configuration fields: {}",
                config.keys().cloned().collect::<Vec<_>>().join(", ")
            )));
        }
        Ok(Self {
            query,
            label_field,
            keywords_field,
            group_field,
            shortcut_field,
            disabled_field,
            active,
            row_height,
        })
    }

    fn grouped_order(
        &self,
        source: &NativeCollectionSource,
    ) -> Result<Vec<NativeCollectionEntry>, NativeCollectionError> {
        let mut grouped = Vec::new();
        for (index, row) in source.rows.iter().enumerate() {
            let label = Self::row_string(row, index, &self.label_field, true)?;
            let keywords = self.row_keywords(row, index)?;
            let score = fuzzy_score(&label, &keywords, &self.query);
            if score < 0 {
                continue;
            }
            let group = Self::row_string(row, index, &self.group_field, false)?;
            grouped.push((group, (index, score)));
        }
        let source_keys = source
            .keys
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut header_keys = BTreeSet::new();
        let mut entries = Vec::new();
        for (group, mut rows) in crate::collection_projection::stable_groups(grouped) {
            rows.sort_by(|(left_index, left_score), (right_index, right_score)| {
                right_score
                    .cmp(left_score)
                    .then_with(|| left_index.cmp(right_index))
            });
            if !group.is_empty() {
                let key =
                    unique_group_key(&format!("command:{group}"), &source_keys, &mut header_keys);
                entries.push(NativeCollectionEntry::Group(GroupEntry {
                    key,
                    value: group,
                    count: rows.len(),
                    collapsed: false,
                }));
            }
            entries.extend(
                rows.into_iter()
                    .map(|(index, _)| NativeCollectionEntry::Row(index)),
            );
        }
        Ok(entries)
    }

    fn project_row(
        &self,
        row: &UiValue,
        index: usize,
        key: Option<&str>,
    ) -> Result<UiValue, NativeCollectionError> {
        let key = key.ok_or(NativeCollectionError::CorruptOrder(index))?;
        let label = Self::row_string(row, index, &self.label_field, true)?;
        let shortcut = Self::row_string(row, index, &self.shortcut_field, false)?;
        let disabled = self.row_disabled_value(row, index)?;
        Ok(UiValue::Map(BTreeMap::from([
            ("kind".to_owned(), UiValue::String("item".to_owned())),
            ("key".to_owned(), UiValue::String(format!("item:{key}"))),
            ("value".to_owned(), UiValue::String(key.to_owned())),
            ("label".to_owned(), UiValue::String(label)),
            ("shortcut".to_owned(), UiValue::String(shortcut)),
            ("disabled".to_owned(), UiValue::Bool(disabled)),
            ("active".to_owned(), UiValue::Bool(self.active == key)),
            ("height".to_owned(), UiValue::Float(self.row_height)),
        ])))
    }

    fn project_group(&self, group: &GroupEntry) -> UiValue {
        UiValue::Map(BTreeMap::from([
            ("kind".to_owned(), UiValue::String("group".to_owned())),
            ("key".to_owned(), UiValue::String(group.key.clone())),
            ("label".to_owned(), UiValue::String(group.value.clone())),
            ("height".to_owned(), UiValue::Float(self.row_height)),
        ]))
    }

    fn row_disabled(
        &self,
        source: &NativeCollectionSource,
        index: usize,
    ) -> Result<bool, NativeCollectionError> {
        let row = source
            .rows
            .get(index)
            .ok_or(NativeCollectionError::CorruptOrder(index))?;
        self.row_disabled_value(row, index)
    }

    fn row_disabled_value(
        &self,
        row: &UiValue,
        index: usize,
    ) -> Result<bool, NativeCollectionError> {
        match row_field(row, &self.disabled_field) {
            None | Some(UiValue::Null) => Ok(false),
            Some(UiValue::Bool(value)) => Ok(*value),
            Some(_) => Err(NativeCollectionError::InvalidFuzzyConfig(format!(
                "row {index} field `{}` must be bool when present",
                self.disabled_field
            ))),
        }
    }

    fn row_string(
        row: &UiValue,
        index: usize,
        field: &str,
        required: bool,
    ) -> Result<String, NativeCollectionError> {
        match row_field(row, field) {
            Some(UiValue::String(value)) if !required || !value.is_empty() => Ok(value.clone()),
            None | Some(UiValue::Null) if !required => Ok(String::new()),
            _ => Err(NativeCollectionError::InvalidFuzzyConfig(format!(
                "row {index} field `{field}` must be {}string",
                if required { "a non-empty " } else { "a " }
            ))),
        }
    }

    fn row_keywords(
        &self,
        row: &UiValue,
        index: usize,
    ) -> Result<Vec<String>, NativeCollectionError> {
        match row_field(row, &self.keywords_field) {
            None | Some(UiValue::Null) => Ok(Vec::new()),
            Some(UiValue::Array(values)) => values
                .iter()
                .enumerate()
                .map(|(keyword, value)| match value {
                    UiValue::String(value) => Ok(value.clone()),
                    _ => Err(NativeCollectionError::InvalidFuzzyConfig(format!(
                        "row {index} field `{}` item {keyword} must be a string",
                        self.keywords_field
                    ))),
                })
                .collect(),
            Some(_) => Err(NativeCollectionError::InvalidFuzzyConfig(format!(
                "row {index} field `{}` must be an array of strings",
                self.keywords_field
            ))),
        }
    }
}

fn fuzzy_score(label: &str, keywords: &[String], query: &str) -> i64 {
    std::iter::once((label, 0))
        .chain(keywords.iter().map(|keyword| (keyword.as_str(), -100)))
        .map(|(candidate, penalty)| fuzzy_candidate_score(candidate, query) + penalty)
        .max()
        .unwrap_or(-1)
}

fn fuzzy_candidate_score(candidate: &str, query: &str) -> i64 {
    if query.is_empty() {
        return 0;
    }
    let candidate = candidate.to_lowercase();
    if candidate == query {
        return 10_000;
    }
    let length = i64::try_from(candidate.chars().count()).unwrap_or(i64::MAX);
    if candidate.starts_with(query) {
        return 8_000 - length;
    }
    if let Some(position) = candidate.find(query) {
        let position = i64::try_from(candidate[..position].chars().count()).unwrap_or(i64::MAX);
        return 6_000 - position.saturating_mul(8) - length;
    }
    let chars = candidate.chars().collect::<Vec<_>>();
    let mut position = 0usize;
    let mut gaps = 0i64;
    for needle in query.chars() {
        let Some(found) = chars[position..]
            .iter()
            .position(|candidate| *candidate == needle)
        else {
            return -1;
        };
        gaps = gaps.saturating_add(i64::try_from(found).unwrap_or(i64::MAX));
        position = position.saturating_add(found).saturating_add(1);
    }
    4_000 - gaps.saturating_mul(12) - length
}

fn fuzzy_take_string(
    values: &mut BTreeMap<String, UiValue>,
    name: &str,
) -> Result<String, NativeCollectionError> {
    match values.remove(name) {
        Some(UiValue::String(value)) => Ok(value),
        _ => Err(NativeCollectionError::InvalidFuzzyConfig(format!(
            "{name} must be a string"
        ))),
    }
}

fn fuzzy_take_number(
    values: &mut BTreeMap<String, UiValue>,
    name: &str,
) -> Result<f64, NativeCollectionError> {
    match values.remove(name) {
        Some(UiValue::Float(value)) => Ok(value),
        Some(UiValue::Integer(value)) => Ok(integer_float(value)),
        _ => Err(NativeCollectionError::InvalidFuzzyConfig(format!(
            "{name} must be a number"
        ))),
    }
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
    query: String,
    search_fields: Vec<String>,
    page: usize,
    page_size: Option<usize>,
}

impl From<&TableProjection> for TableOrderSpec {
    fn from(projection: &TableProjection) -> Self {
        Self {
            sort: projection.sort.clone(),
            group_by: projection.group_by.clone(),
            collapsed_groups: projection.collapsed_groups.clone(),
            query: projection.query.clone(),
            search_fields: projection.search_fields.clone(),
            page: projection.page,
            page_size: projection.page_size,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct TableProjection {
    label: String,
    row_key: String,
    columns: Vec<TableColumn>,
    selected: BTreeSet<String>,
    striped: bool,
    row_height: f64,
    sort: Option<SortSpec>,
    group_by: Option<String>,
    collapsed_groups: BTreeSet<String>,
    group_toggle: bool,
    query: String,
    search_fields: Vec<String>,
    page: usize,
    page_size: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
struct TableColumn {
    key: String,
    sort_key: String,
    width: UiValue,
    align: String,
    resize_signal_key: Option<String>,
    adornments: Vec<TableAdornment>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TableAdornment {
    text_key: String,
    variant: String,
    variant_key: Option<String>,
    dot: bool,
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
        let selected = take_array(&mut config, "selected_keys")?
            .into_iter()
            .enumerate()
            .map(|(index, value)| match value {
                UiValue::String(value) => Ok(value),
                _ => Err(NativeCollectionError::InvalidTableConfig(format!(
                    "selected_keys[{index}] must be a string"
                ))),
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        // A row is drawn from its selected flag; a click builds the next
        // selection in Rhai, so the mode is only validated here.
        let selection_mode = take_string(&mut config, "selection_mode")?;
        if !matches!(selection_mode.as_str(), "none" | "single" | "multiple") {
            return Err(NativeCollectionError::InvalidTableConfig(format!(
                "unknown selection mode `{selection_mode}`"
            )));
        }
        let striped = take_bool(&mut config, "striped")?;
        let row_height = take_number(&mut config, "row_height")?;
        if !row_height.is_finite() || row_height <= 0.0 {
            return Err(NativeCollectionError::InvalidTableConfig(
                "row_height must be finite and positive".to_owned(),
            ));
        }
        let sort = decode_sort(config.remove("sort"))?
            .map(|mut sort| {
                let column = columns
                    .iter()
                    .find(|column| column.key == sort.key)
                    .ok_or_else(|| {
                        NativeCollectionError::InvalidTableConfig(format!(
                            "sort key `{}` is not a declared column",
                            sort.key
                        ))
                    })?;
                sort.key.clone_from(&column.sort_key);
                Ok(sort)
            })
            .transpose()?;
        let (group_by, collapsed_groups, group_toggle) = decode_grouping(&mut config, &columns)?;
        let (query, search_fields, page, page_size) =
            decode_table_window(&mut config, &row_key, &columns)?;
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
            selected,
            striped,
            row_height,
            sort,
            group_by,
            collapsed_groups,
            group_toggle,
            query,
            search_fields,
            page,
            page_size,
        })
    }

    fn visible_rows(&self, source: &NativeCollectionSource, rows: &[usize]) -> Vec<usize> {
        let mut visible = if self.query.is_empty() {
            rows.to_vec()
        } else {
            rows.iter()
                .copied()
                .filter(|index| {
                    source.rows.get(*index).is_some_and(|row| {
                        self.search_fields.iter().any(|field| {
                            row_field(row, field)
                                .and_then(scalar_text)
                                .is_some_and(|value| value.to_lowercase().contains(&self.query))
                        })
                    })
                })
                .collect()
        };
        if let Some(page_size) = self.page_size {
            let start = self.page.saturating_sub(1).saturating_mul(page_size);
            if start >= visible.len() {
                visible.clear();
            } else {
                let end = start.saturating_add(page_size).min(visible.len());
                visible = visible[start..end].to_vec();
            }
        }
        visible
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
        let mut grouped = Vec::with_capacity(rows.len());
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
            grouped.push((value, *source_index));
        }
        let source_keys = source
            .keys
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut header_keys = BTreeSet::new();
        let groups = crate::collection_projection::stable_groups(grouped);
        let mut entries = Vec::with_capacity(rows.len().saturating_add(groups.len()));
        for (value, rows) in groups {
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
                let adornments = column
                    .adornments
                    .iter()
                    .map(|adornment| adornment.project(row))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect();
                let mut cell = BTreeMap::from([
                    ("key".to_owned(), UiValue::String(column.key.clone())),
                    ("text".to_owned(), UiValue::String(text)),
                    ("width".to_owned(), column.width.clone()),
                    ("align".to_owned(), UiValue::String(column.align.clone())),
                    ("adornments".to_owned(), UiValue::Array(adornments)),
                ]);
                if let Some(signal_key) = &column.resize_signal_key {
                    cell.insert(
                        "resize_signal_key".to_owned(),
                        UiValue::String(signal_key.clone()),
                    );
                }
                Ok(UiValue::Map(cell))
            })
            .collect::<Result<Vec<_>, NativeCollectionError>>()?;
        Ok(UiValue::Map(BTreeMap::from([
            ("kind".to_owned(), UiValue::String("row".to_owned())),
            ("key".to_owned(), UiValue::String(key)),
            (
                "label".to_owned(),
                UiValue::String(format!("{} row {}", self.label, index + 1)),
            ),
            ("cells".to_owned(), UiValue::Array(cells)),
            ("selected".to_owned(), UiValue::Bool(selected)),
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

fn decode_table_window(
    config: &mut BTreeMap<String, UiValue>,
    row_key: &str,
    columns: &[TableColumn],
) -> Result<(String, Vec<String>, usize, Option<usize>), NativeCollectionError> {
    let query = match config.remove("query") {
        None => String::new(),
        Some(UiValue::String(value)) => value.to_lowercase(),
        _ => {
            return Err(NativeCollectionError::InvalidTableConfig(
                "query must be a string".to_owned(),
            ));
        }
    };
    let search_fields = match config.remove("search_fields") {
        None => Vec::new(),
        Some(UiValue::Array(values)) => values,
        _ => {
            return Err(NativeCollectionError::InvalidTableConfig(
                "search_fields must be an array".to_owned(),
            ));
        }
    }
    .into_iter()
    .enumerate()
    .map(|(index, value)| match value {
        UiValue::String(value) if !value.is_empty() => Ok(value),
        _ => Err(NativeCollectionError::InvalidTableConfig(format!(
            "search_fields[{index}] must be a non-empty string"
        ))),
    })
    .collect::<Result<Vec<_>, _>>()?;
    if !query.is_empty() && search_fields.is_empty() {
        return Err(NativeCollectionError::InvalidTableConfig(
            "non-empty query requires search_fields".to_owned(),
        ));
    }
    if let Some(field) = search_fields.iter().find(|field| {
        field.as_str() != row_key && !columns.iter().any(|column| column.key == field.as_str())
    }) {
        return Err(NativeCollectionError::InvalidTableConfig(format!(
            "search field `{field}` is not the row key or a declared column"
        )));
    }
    let page = if config.contains_key("page") {
        take_positive_usize(config, "page")?
    } else {
        1
    };
    let page_size = take_optional_positive_usize(config, "page_size")?;
    Ok((query, search_fields, page, page_size))
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
        let sort_key =
            take_optional_string(&mut column, "sort_key")?.unwrap_or_else(|| key.clone());
        validate_name(&sort_key, "sort field")?;
        let width = column.remove("width").ok_or_else(|| {
            NativeCollectionError::InvalidTableConfig("column.width is required".to_owned())
        })?;
        let align = match column.remove("align") {
            None | Some(UiValue::Null) => "start".to_owned(),
            Some(UiValue::String(value)) => value,
            Some(_) => {
                return Err(NativeCollectionError::InvalidTableConfig(
                    "column.align must be a string".to_owned(),
                ));
            }
        };
        let resize_signal_key = match column.remove("resize_signal_key") {
            Some(UiValue::String(value)) => Some(value),
            Some(_) => {
                return Err(NativeCollectionError::InvalidTableConfig(
                    "resize_signal_key must be a string".to_owned(),
                ));
            }
            None => None,
        };
        let adornments = match column.remove("adornments") {
            Some(UiValue::Array(values)) => values
                .into_iter()
                .map(TableAdornment::decode)
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => {
                return Err(NativeCollectionError::InvalidTableConfig(
                    "column.adornments must be an array".to_owned(),
                ));
            }
            None => Vec::new(),
        };
        for normalized_only in [
            "title",
            "numeric",
            "typography",
            "sortable",
            "resizable",
            "resize_enabled",
            "resize_ref_key",
            "min_width",
            "max_width",
        ] {
            column.remove(normalized_only);
        }
        if !column.is_empty() {
            return Err(NativeCollectionError::InvalidTableConfig(format!(
                "unknown column fields: {}",
                column.keys().cloned().collect::<Vec<_>>().join(", ")
            )));
        }
        Ok(Self {
            key,
            sort_key,
            width,
            align,
            resize_signal_key,
            adornments,
        })
    }
}

impl TableAdornment {
    fn decode(value: UiValue) -> Result<Self, NativeCollectionError> {
        let UiValue::Map(mut value) = value else {
            return Err(NativeCollectionError::InvalidTableConfig(
                "each column adornment must be a map".to_owned(),
            ));
        };
        let text_key = take_string(&mut value, "text_key")?;
        let variant = match value.remove("variant") {
            Some(UiValue::String(value)) => value,
            Some(_) => {
                return Err(NativeCollectionError::InvalidTableConfig(
                    "adornment.variant must be a string".to_owned(),
                ));
            }
            None => "neutral".to_owned(),
        };
        validate_badge_variant(&variant)?;
        let variant_key = match value.remove("variant_key") {
            Some(UiValue::String(value)) => Some(value),
            Some(UiValue::Null) | None => None,
            Some(_) => {
                return Err(NativeCollectionError::InvalidTableConfig(
                    "adornment.variant_key must be a string".to_owned(),
                ));
            }
        };
        let dot = match value.remove("dot") {
            Some(UiValue::Bool(value)) => value,
            Some(_) => {
                return Err(NativeCollectionError::InvalidTableConfig(
                    "adornment.dot must be a bool".to_owned(),
                ));
            }
            None => false,
        };
        if !value.is_empty() {
            return Err(NativeCollectionError::InvalidTableConfig(format!(
                "unknown adornment fields: {}",
                value.keys().cloned().collect::<Vec<_>>().join(", ")
            )));
        }
        Ok(Self {
            text_key,
            variant,
            variant_key,
            dot,
        })
    }

    fn project(
        &self,
        row: &BTreeMap<String, UiValue>,
    ) -> Result<Option<UiValue>, NativeCollectionError> {
        let text = row
            .get(&self.text_key)
            .map_or_else(String::new, display_scalar);
        if text.is_empty() {
            return Ok(None);
        }
        let variant = self.variant_key.as_ref().map_or_else(
            || Ok(self.variant.clone()),
            |key| match row.get(key) {
                Some(UiValue::String(value)) => Ok(value.clone()),
                _ => Err(NativeCollectionError::InvalidTableConfig(format!(
                    "adornment variant field `{key}` must be a string"
                ))),
            },
        )?;
        validate_badge_variant(&variant)?;
        Ok(Some(UiValue::Map(BTreeMap::from([
            ("text".to_owned(), UiValue::String(text)),
            ("variant".to_owned(), UiValue::String(variant)),
            ("dot".to_owned(), UiValue::Bool(self.dot)),
        ]))))
    }
}

fn validate_badge_variant(value: &str) -> Result<(), NativeCollectionError> {
    if matches!(
        value,
        "neutral" | "accent" | "success" | "warning" | "danger"
    ) {
        Ok(())
    } else {
        Err(NativeCollectionError::InvalidTableConfig(format!(
            "unknown adornment variant `{value}`"
        )))
    }
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
        (UiValue::Float(left), UiValue::Float(right)) if left.is_finite() && right.is_finite() => {
            left.partial_cmp(right)
        }
        (UiValue::Integer(left), UiValue::Float(right)) => compare_integer_float(*left, *right),
        (UiValue::Float(left), UiValue::Integer(right)) => {
            compare_integer_float(*right, *left).map(Ordering::reverse)
        }
        (UiValue::String(left), UiValue::String(right)) => Some(left.cmp(right)),
        _ => None,
    }
}

#[allow(clippy::cast_possible_truncation)]
fn compare_integer_float(integer: i64, float: f64) -> Option<Ordering> {
    const I64_END: f64 = 9_223_372_036_854_775_808.0;
    if !float.is_finite() {
        return None;
    }
    // The upper endpoint is exclusive: i64::MAX rounds up to 2^63 as f64.
    // Converting the integer instead would collapse distinct ranks above 2^53
    // and make the stable source-order tie break violate transitivity.
    if float >= I64_END {
        return Some(Ordering::Less);
    }
    if float < -I64_END {
        return Some(Ordering::Greater);
    }
    // The range checks make the truncated integral part representable as i64.
    let truncated = float as i64;
    Some(integer.cmp(&truncated).then_with(|| {
        if float.fract() > 0.0 {
            Ordering::Less
        } else if float.fract() < 0.0 {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }))
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

fn take_positive_usize(
    values: &mut BTreeMap<String, UiValue>,
    name: &str,
) -> Result<usize, NativeCollectionError> {
    match values.remove(name) {
        Some(UiValue::Integer(value)) if value > 0 => usize::try_from(value).map_err(|_| {
            NativeCollectionError::InvalidTableConfig(format!(
                "{name} exceeds the supported platform size"
            ))
        }),
        _ => Err(NativeCollectionError::InvalidTableConfig(format!(
            "{name} must be a positive integer"
        ))),
    }
}

fn take_optional_positive_usize(
    values: &mut BTreeMap<String, UiValue>,
    name: &str,
) -> Result<Option<usize>, NativeCollectionError> {
    match values.remove(name) {
        None | Some(UiValue::Null) => Ok(None),
        Some(UiValue::Integer(value)) if value > 0 => {
            usize::try_from(value).map(Some).map_err(|_| {
                NativeCollectionError::InvalidTableConfig(format!(
                    "{name} exceeds the supported platform size"
                ))
            })
        }
        _ => Err(NativeCollectionError::InvalidTableConfig(format!(
            "{name} must be null or a positive integer"
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

const MAX_MISSING_NAMES_PER_READER: usize = 64;
const MAX_MISSING_READER_PAIRS: usize = 4_096;
const MAX_MISSING_NAME_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Default)]
pub struct NativeCollectionRegistry {
    collections: BTreeMap<String, NativeCollection>,
    readers: BTreeMap<String, BTreeSet<crate::read_dependency::ReadDependency>>,
    missing_readers: BTreeMap<String, BTreeSet<crate::read_dependency::ReadDependency>>,
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
        self.register_with_invalidated(name.into(), collection)?;
        Ok(())
    }

    pub(crate) fn register_with_invalidated(
        &mut self,
        name: String,
        collection: NativeCollection,
    ) -> Result<BTreeSet<ComponentInstancePath>, NativeCollectionError> {
        validate_name(&name, "collection name")?;
        if self.collections.contains_key(&name) {
            return Err(NativeCollectionError::DuplicateCollection(name));
        }
        self.collections.insert(name.clone(), collection);
        Ok(crate::read_dependency::owners(
            self.missing_readers.remove(&name).unwrap_or_default(),
        ))
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
        Ok(crate::read_dependency::owners(
            self.readers.get(name).cloned().unwrap_or_default(),
        ))
    }

    #[cfg(test)]
    pub(crate) fn read_tracked(
        &mut self,
        reader: &ComponentInstancePath,
        name: &str,
    ) -> Result<NativeCollection, NativeCollectionError> {
        self.read_tracked_with_missing(reader, name, true)
    }

    #[cfg(test)]
    pub(crate) fn read_tracked_with_missing(
        &mut self,
        reader: &ComponentInstancePath,
        name: &str,
        track_missing: bool,
    ) -> Result<NativeCollection, NativeCollectionError> {
        self.read_dependency(
            Some(&crate::read_dependency::ReadDependency::component(reader)),
            name,
            track_missing,
        )
    }

    /// Read a collection, subscribing `reader` when there is one.
    pub(crate) fn read_dependency(
        &mut self,
        reader: Option<&crate::read_dependency::ReadDependency>,
        name: &str,
        track_missing: bool,
    ) -> Result<NativeCollection, NativeCollectionError> {
        validate_name(name, "collection name")?;
        let Some(collection) = self.collections.get(name).cloned() else {
            if track_missing && let Some(reader) = reader {
                self.track_missing_reader(reader, name)?;
            }
            return Err(NativeCollectionError::UnknownCollection(name.to_owned()));
        };
        let Some(reader) = reader else {
            return Ok(collection);
        };
        if let Some(readers) = self.missing_readers.get_mut(name) {
            readers.remove(reader);
            if readers.is_empty() {
                self.missing_readers.remove(name);
            }
        }
        self.readers
            .entry(name.to_owned())
            .or_default()
            .insert(reader.clone());
        Ok(collection)
    }

    fn track_missing_reader(
        &mut self,
        reader: &crate::read_dependency::ReadDependency,
        name: &str,
    ) -> Result<(), NativeCollectionError> {
        if self
            .missing_readers
            .get(name)
            .is_some_and(|readers| readers.contains(reader))
        {
            return Ok(());
        }
        let reader_names = self
            .missing_readers
            .values()
            .filter(|readers| readers.contains(reader))
            .count();
        if reader_names >= MAX_MISSING_NAMES_PER_READER {
            return Err(NativeCollectionError::MissingDependencyBudget {
                kind: "names per component",
                limit: MAX_MISSING_NAMES_PER_READER,
            });
        }
        let pairs = self
            .missing_readers
            .values()
            .map(BTreeSet::len)
            .sum::<usize>();
        if pairs >= MAX_MISSING_READER_PAIRS {
            return Err(NativeCollectionError::MissingDependencyBudget {
                kind: "component/name pairs",
                limit: MAX_MISSING_READER_PAIRS,
            });
        }
        if !self.missing_readers.contains_key(name) {
            let bytes = self.missing_readers.keys().map(String::len).sum::<usize>();
            if bytes.saturating_add(name.len()) > MAX_MISSING_NAME_BYTES {
                return Err(NativeCollectionError::MissingDependencyBudget {
                    kind: "name bytes",
                    limit: MAX_MISSING_NAME_BYTES,
                });
            }
        }
        self.missing_readers
            .entry(name.to_owned())
            .or_default()
            .insert(reader.clone());
        Ok(())
    }

    pub(crate) fn reset_reader(&mut self, reader: &ComponentInstancePath) {
        self.reset_contribution(&crate::read_dependency::ReadDependency::component(reader));
    }

    pub(crate) fn reset_contribution(&mut self, reader: &crate::read_dependency::ReadDependency) {
        for readers in self.readers.values_mut() {
            readers.remove(reader);
        }
        self.readers.retain(|_, readers| !readers.is_empty());
        for readers in self.missing_readers.values_mut() {
            readers.remove(reader);
        }
        self.missing_readers
            .retain(|_, readers| !readers.is_empty());
    }

    pub(crate) fn retain_reader_scope(
        &mut self,
        root: &ComponentInstancePath,
        active: &BTreeSet<ComponentInstancePath>,
    ) {
        for readers in self.readers.values_mut() {
            readers.retain(|reader| reader.retained_in_owner_scope(root, active));
        }
        self.readers.retain(|_, readers| !readers.is_empty());
        for readers in self.missing_readers.values_mut() {
            readers.retain(|reader| reader.retained_in_owner_scope(root, active));
        }
        self.missing_readers
            .retain(|_, readers| !readers.is_empty());
    }

    pub(crate) fn remove_reader_scope(&mut self, root: &ComponentInstancePath) {
        for readers in self.readers.values_mut() {
            readers.retain(|reader| !reader.owner.is_within(root));
        }
        self.readers.retain(|_, readers| !readers.is_empty());
        for readers in self.missing_readers.values_mut() {
            readers.retain(|reader| !reader.owner.is_within(root));
        }
        self.missing_readers
            .retain(|_, readers| !readers.is_empty());
    }

    pub(crate) fn retain_contributions(
        &mut self,
        scope: &ComponentInstancePath,
        active: &BTreeSet<crate::read_dependency::ReadContribution>,
    ) {
        crate::read_dependency::retain_readers(&mut self.readers, |reader| {
            reader.retained_in_contribution_scope(scope, active)
        });
        crate::read_dependency::retain_readers(&mut self.missing_readers, |reader| {
            reader.retained_in_contribution_scope(scope, active)
        });
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

    /// Read one projected row without converting the full native source.
    ///
    /// # Errors
    /// Returns a native projection error for an invalid source/view configuration.
    pub fn item(&self, index: usize) -> Result<Option<UiValue>, NativeCollectionError> {
        match self {
            Self::Values(values) => Ok(values.get(index).cloned()),
            Self::Native(collection) => collection.item(index),
        }
    }

    /// Stable key at an ordered collection index, without materializing its row.
    #[must_use]
    pub fn key(&self, index: usize) -> Option<&str> {
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
    #[error("native collection missing-dependency {kind} exceeds limit {limit}")]
    MissingDependencyBudget { kind: &'static str, limit: usize },
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
    #[error("native collection is not a fuzzy projection")]
    NotFuzzyProjection,
    #[error("native collection row {index} group field `{field}` must be a non-empty string")]
    InvalidGroupField { index: usize, field: String },
    #[error("invalid native Table configuration: {0}")]
    InvalidTableConfig(String),
    #[error("invalid native fuzzy-view configuration: {0}")]
    InvalidFuzzyConfig(String),
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

    fn fuzzy_rows() -> NativeCollection {
        NativeCollection::new(
            "id",
            [
                BTreeMap::from([
                    ("id".to_owned(), UiValue::String("new".to_owned())),
                    ("label".to_owned(), UiValue::String("New file".to_owned())),
                    ("group".to_owned(), UiValue::String("File".to_owned())),
                    (
                        "keywords".to_owned(),
                        UiValue::Array(vec![UiValue::String("create document".to_owned())]),
                    ),
                    ("shortcut".to_owned(), UiValue::String("⌘N".to_owned())),
                    ("disabled".to_owned(), UiValue::Bool(false)),
                ]),
                BTreeMap::from([
                    ("id".to_owned(), UiValue::String("open".to_owned())),
                    ("label".to_owned(), UiValue::String("Open file".to_owned())),
                    ("group".to_owned(), UiValue::String("File".to_owned())),
                    (
                        "keywords".to_owned(),
                        UiValue::Array(vec![UiValue::String("load document".to_owned())]),
                    ),
                    ("shortcut".to_owned(), UiValue::String("⌘O".to_owned())),
                    ("disabled".to_owned(), UiValue::Bool(false)),
                ]),
                BTreeMap::from([
                    ("id".to_owned(), UiValue::String("close".to_owned())),
                    (
                        "label".to_owned(),
                        UiValue::String("Close window".to_owned()),
                    ),
                    ("group".to_owned(), UiValue::String("Window".to_owned())),
                    ("keywords".to_owned(), UiValue::Array(Vec::new())),
                    ("shortcut".to_owned(), UiValue::String("⌘W".to_owned())),
                    ("disabled".to_owned(), UiValue::Bool(true)),
                ]),
            ],
        )
        .unwrap()
    }

    fn fuzzy_config(query: &str, active: &str) -> Map {
        UiValue::Map(BTreeMap::from([
            ("query".to_owned(), UiValue::String(query.to_owned())),
            (
                "label_field".to_owned(),
                UiValue::String("label".to_owned()),
            ),
            (
                "keywords_field".to_owned(),
                UiValue::String("keywords".to_owned()),
            ),
            (
                "group_field".to_owned(),
                UiValue::String("group".to_owned()),
            ),
            (
                "shortcut_field".to_owned(),
                UiValue::String("shortcut".to_owned()),
            ),
            (
                "disabled_field".to_owned(),
                UiValue::String("disabled".to_owned()),
            ),
            ("active".to_owned(), UiValue::String(active.to_owned())),
            ("row_height".to_owned(), UiValue::Float(32.0)),
        ]))
        .into_dynamic()
        .cast::<Map>()
    }

    #[test]
    fn fuzzy_projection_filters_groups_projects_and_reuses_structural_order() {
        let source = fuzzy_rows();
        let view = source.fuzzy_view(fuzzy_config("opn", "")).unwrap();
        assert_eq!(view.len(), 2);
        assert_eq!(view.fuzzy_edge(false).unwrap(), "open");
        assert_eq!(view.fuzzy_adjacent("", 0).unwrap(), "open");
        let UiValue::Map(group) = view.item(0).unwrap().unwrap() else {
            panic!("first fuzzy entry must be a group");
        };
        assert_eq!(group["kind"], UiValue::String("group".to_owned()));
        let UiValue::Map(item) = view.item(1).unwrap().unwrap() else {
            panic!("second fuzzy entry must be a command");
        };
        assert_eq!(item["value"], UiValue::String("open".to_owned()));
        assert_eq!(item["active"], UiValue::Bool(false));

        let active = source.fuzzy_view(fuzzy_config("opn", "open")).unwrap();
        assert!(Arc::ptr_eq(&view.order, &active.order));
        let UiValue::Map(item) = active.item(1).unwrap().unwrap() else {
            unreachable!()
        };
        assert_eq!(item["active"], UiValue::Bool(true));

        let all = source.fuzzy_view(fuzzy_config("", "new")).unwrap();
        assert_eq!(all.fuzzy_edge(true).unwrap(), "open");
        assert_eq!(all.fuzzy_adjacent("new", 1).unwrap(), "open");
        assert_eq!(all.fuzzy_adjacent("open", 1).unwrap(), "new");
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

    fn ranked_config(direction: &str) -> Map {
        let mut config = table_config(&[]);
        config.insert("group_by".into(), Dynamic::UNIT);
        config.insert(
            "columns".into(),
            Dynamic::from_array(vec![Dynamic::from_map(Map::from([
                ("key".into(), Dynamic::from("score")),
                ("sort_key".into(), Dynamic::from("rank")),
                ("width".into(), Dynamic::from(80_i64)),
            ]))]),
        );
        config.insert(
            "sort".into(),
            Dynamic::from_map(Map::from([
                ("key".into(), Dynamic::from("score")),
                ("direction".into(), Dynamic::from(direction.to_owned())),
            ])),
        );
        config
    }

    #[test]
    fn mixed_numeric_comparison_is_exact_at_integer_and_float_boundaries() {
        const TWO_TO_53: i64 = 9_007_199_254_740_992;
        let float_53 = 9_007_199_254_740_992.0_f64;
        let float_63 = 9_223_372_036_854_775_808.0_f64;
        let tiny = f64::from_bits(1);
        for (integer, float, expected) in [
            (TWO_TO_53 - 1, float_53, Ordering::Less),
            (TWO_TO_53, float_53, Ordering::Equal),
            (TWO_TO_53 + 1, float_53, Ordering::Greater),
            (TWO_TO_53 + 1, float_53.next_up(), Ordering::Less),
            (-TWO_TO_53 - 1, -float_53, Ordering::Less),
            (-TWO_TO_53, -float_53, Ordering::Equal),
            (-TWO_TO_53 + 1, -float_53, Ordering::Greater),
            (i64::MAX, float_63, Ordering::Less),
            (i64::MAX, float_63.next_down(), Ordering::Greater),
            (i64::MAX, float_63.next_up(), Ordering::Less),
            (i64::MIN, -float_63, Ordering::Equal),
            (i64::MIN + 1, -float_63, Ordering::Greater),
            (i64::MIN, (-float_63).next_up(), Ordering::Less),
            (i64::MIN, (-float_63).next_down(), Ordering::Greater),
            (i64::MAX, f64::MAX, Ordering::Less),
            (i64::MIN, -f64::MAX, Ordering::Greater),
            (1, 1.5, Ordering::Less),
            (2, 1.5, Ordering::Greater),
            (-1, -1.5, Ordering::Greater),
            (-2, -1.5, Ordering::Less),
            (0, 0.5, Ordering::Less),
            (0, -0.5, Ordering::Greater),
            (0, tiny, Ordering::Less),
            (0, -tiny, Ordering::Greater),
            (0, 0.0, Ordering::Equal),
            (0, -0.0, Ordering::Equal),
        ] {
            let integer = UiValue::Integer(integer);
            let float = UiValue::Float(float);
            assert_eq!(
                compare_values(&integer, &float),
                Some(expected),
                "{integer:?} versus {float:?}"
            );
            assert_eq!(compare_values(&float, &integer), Some(expected.reverse()));
        }
        for float in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                compare_values(&UiValue::Integer(0), &UiValue::Float(float)),
                None
            );
            assert_eq!(
                compare_values(&UiValue::Float(float), &UiValue::Integer(0)),
                None
            );
        }
    }

    #[test]
    fn mixed_numeric_comparison_is_a_total_order_with_stable_ties() {
        let values = [
            UiValue::Integer(9_007_199_254_740_993),
            UiValue::Float(9_007_199_254_740_992.0),
            UiValue::Integer(9_007_199_254_740_992),
            UiValue::Integer(9_007_199_254_740_991),
            UiValue::Float(9_007_199_254_740_994.0),
            UiValue::Integer(-9_007_199_254_740_993),
            UiValue::Float(-9_007_199_254_740_992.0),
            UiValue::Integer(-9_007_199_254_740_992),
            UiValue::Integer(i64::MIN),
            UiValue::Integer(i64::MIN + 1),
            UiValue::Float(-9_223_372_036_854_775_808.0),
            UiValue::Integer(i64::MAX),
            UiValue::Float(9_223_372_036_854_775_808.0),
            UiValue::Float(f64::MAX),
            UiValue::Float(-f64::MAX),
            UiValue::Integer(-2),
            UiValue::Float(-1.5),
            UiValue::Integer(-1),
            UiValue::Float(-f64::from_bits(1)),
            UiValue::Integer(0),
            UiValue::Float(-0.0),
            UiValue::Float(0.0),
            UiValue::Float(f64::from_bits(1)),
            UiValue::Float(0.5),
            UiValue::Integer(1),
            UiValue::Float(1.5),
            UiValue::Integer(2),
        ];
        // Include the source-position tie break used by sorted_order: rounding
        // 2^53 + 1 down to 2^53 previously produced a cycle in the first triple.
        for (left_index, left) in values.iter().enumerate() {
            assert_eq!(compare_values(left, left), Some(Ordering::Equal));
            for (right_index, right) in values.iter().enumerate() {
                let forward = compare_values(left, right).unwrap();
                assert_eq!(compare_values(right, left), Some(forward.reverse()));
                for (last_index, last) in values.iter().enumerate() {
                    let middle = compare_values(right, last).unwrap();
                    let end = compare_values(left, last).unwrap();
                    assert!(
                        forward == Ordering::Greater
                            || middle == Ordering::Greater
                            || end != Ordering::Greater,
                        "non-transitive numeric order: {left:?}, {right:?}, {last:?}"
                    );
                    let forward = forward.then_with(|| left_index.cmp(&right_index));
                    let middle = middle.then_with(|| right_index.cmp(&last_index));
                    let end = end.then_with(|| left_index.cmp(&last_index));
                    assert!(
                        forward == Ordering::Greater
                            || middle == Ordering::Greater
                            || end != Ordering::Greater,
                        "non-transitive stable order: {left:?}, {right:?}, {last:?}"
                    );
                }
            }
        }
    }

    fn mixed_numeric_source() -> NativeCollection {
        let float_63 = 9_223_372_036_854_775_808.0_f64;
        let ranks = [
            ("large-int", Some(UiValue::Integer(9_007_199_254_740_993))),
            ("large-float", Some(UiValue::Float(9_007_199_254_740_992.0))),
            ("same-int", Some(UiValue::Integer(9_007_199_254_740_992))),
            ("max-int", Some(UiValue::Integer(i64::MAX))),
            ("upper-float", Some(UiValue::Float(float_63))),
            ("min-int", Some(UiValue::Integer(i64::MIN))),
            ("lower-float", Some(UiValue::Float(-float_63))),
            ("below-min", Some(UiValue::Float((-float_63).next_down()))),
            ("negative-frac", Some(UiValue::Float(-1.5))),
            ("negative-int", Some(UiValue::Integer(-1))),
            ("negative-tiny", Some(UiValue::Float(-f64::from_bits(1)))),
            ("zero-int", Some(UiValue::Integer(0))),
            ("zero-negative", Some(UiValue::Float(-0.0))),
            ("zero-positive", Some(UiValue::Float(0.0))),
            ("positive-tiny", Some(UiValue::Float(f64::from_bits(1)))),
            ("positive-frac", Some(UiValue::Float(0.5))),
            ("null", Some(UiValue::Null)),
            ("missing", None),
        ];
        NativeCollection::new(
            "id",
            ranks.into_iter().map(|(key, rank)| {
                let mut row = BTreeMap::from([
                    ("id".into(), UiValue::String(key.into())),
                    ("score".into(), UiValue::String(format!("formatted {key}"))),
                ]);
                if let Some(rank) = rank {
                    row.insert("rank".into(), rank);
                }
                row
            }),
        )
        .unwrap()
    }

    #[test]
    fn array_and_native_sort_key_share_exact_mixed_numeric_order() {
        let source = mixed_numeric_source();
        let array = source
            .source
            .rows
            .iter()
            .cloned()
            .map(UiValue::into_dynamic)
            .collect::<rhai::Array>();
        let mut engine = rhai::Engine::new_raw();
        register_native_collection_api(&mut engine);
        for (direction, expected) in [
            (
                "ascending",
                [
                    "below-min",
                    "min-int",
                    "lower-float",
                    "negative-frac",
                    "negative-int",
                    "negative-tiny",
                    "zero-int",
                    "zero-negative",
                    "zero-positive",
                    "positive-tiny",
                    "positive-frac",
                    "large-float",
                    "same-int",
                    "large-int",
                    "max-int",
                    "upper-float",
                    "null",
                    "missing",
                ],
            ),
            (
                "descending",
                [
                    "upper-float",
                    "max-int",
                    "large-int",
                    "large-float",
                    "same-int",
                    "positive-frac",
                    "positive-tiny",
                    "zero-int",
                    "zero-negative",
                    "zero-positive",
                    "negative-tiny",
                    "negative-int",
                    "negative-frac",
                    "min-int",
                    "lower-float",
                    "below-min",
                    "null",
                    "missing",
                ],
            ),
        ] {
            for input in [
                Dynamic::from(source.clone()),
                Dynamic::from_array(array.clone()),
            ] {
                let mut scope = rhai::Scope::new();
                scope.push_dynamic("rows", input);
                scope.push("config", ranked_config(direction));
                let view = engine
                    .eval_with_scope::<NativeCollection>(
                        &mut scope,
                        "native_table_view(rows, config)",
                    )
                    .unwrap();
                assert_eq!(
                    (0..view.len())
                        .map(|index| view.key(index).unwrap())
                        .collect::<Vec<_>>(),
                    expected
                );
            }
        }
    }

    #[test]
    fn sort_key_resolves_column_identity_and_nulls_stay_last_with_stable_ties() {
        let rows = [
            Some(UiValue::Integer(2)),
            None,
            Some(UiValue::Null),
            Some(UiValue::Integer(1)),
            Some(UiValue::Integer(2)),
        ];
        let source = NativeCollection::new(
            "id",
            rows.into_iter().enumerate().map(|(index, rank)| {
                let mut row = BTreeMap::from([
                    ("id".into(), UiValue::String(index.to_string())),
                    (
                        "score".into(),
                        UiValue::String(format!("formatted {index}")),
                    ),
                ]);
                if let Some(rank) = rank {
                    row.insert("rank".into(), rank);
                }
                row
            }),
        )
        .unwrap();
        for (direction, expected) in [
            ("ascending", ["3", "0", "4", "1", "2"]),
            ("descending", ["0", "4", "3", "1", "2"]),
        ] {
            let view = source.table_view(ranked_config(direction)).unwrap();
            assert_eq!(
                (0..view.len())
                    .map(|index| view.key(index).unwrap())
                    .collect::<Vec<_>>(),
                expected
            );
            let UiValue::Map(item) = view.item(0).unwrap().unwrap() else {
                panic!()
            };
            assert!(format!("{item:?}").contains("formatted"));
        }
        let mut invalid = ranked_config("ascending");
        invalid.insert(
            "sort".into(),
            Dynamic::from_map(Map::from([
                ("key".into(), Dynamic::from("rank")),
                ("direction".into(), Dynamic::from("ascending")),
            ])),
        );
        assert!(
            source.table_view(invalid).is_err(),
            "sort must name the declared display column"
        );
    }

    #[test]
    fn sort_validates_every_non_null_value_before_mutating_or_caching() {
        for rank in [
            UiValue::Array(vec![]),
            UiValue::Float(f64::NAN),
            UiValue::Float(f64::INFINITY),
        ] {
            let source = NativeCollection::new(
                "id",
                [BTreeMap::from([
                    ("id".into(), UiValue::String("only".into())),
                    ("rank".into(), rank),
                ])],
            )
            .unwrap();
            assert!(matches!(
                source.table_view(ranked_config("ascending")),
                Err(NativeCollectionError::UnsortableField(_))
            ));
            assert!(source.source.sorted_orders.lock().unwrap().is_empty());
            assert_eq!(source.key(0), Some("only"));
        }
        let source = NativeCollection::new(
            "id",
            [UiValue::Integer(1), UiValue::String("bad".into())]
                .into_iter()
                .enumerate()
                .map(|(index, rank)| {
                    BTreeMap::from([
                        ("id".into(), UiValue::String(index.to_string())),
                        ("rank".into(), rank),
                    ])
                }),
        )
        .unwrap();
        assert!(source.table_view(ranked_config("ascending")).is_err());
    }

    #[test]
    fn thirty_thousand_formatted_native_rows_sort_without_rhai_enumeration() {
        let source = NativeCollection::new(
            "id",
            (0..30_000).map(|index| {
                BTreeMap::from([
                    ("id".into(), UiValue::String(index.to_string())),
                    (
                        "score".into(),
                        UiValue::String(format!("{} min", 30_000 - index)),
                    ),
                    ("rank".into(), UiValue::Integer(30_000 - index)),
                ])
            }),
        )
        .unwrap();
        let view = source.table_view(ranked_config("ascending")).unwrap();
        assert_eq!(view.len(), 30_000);
        assert_eq!(view.key(0), Some("29999"));
        assert_eq!(view.key(29_999), Some("0"));
        let cached = source.table_view(ranked_config("ascending")).unwrap();
        assert!(Arc::ptr_eq(&view.order, &cached.order));
    }

    #[test]
    fn native_list_neighbors_skip_disabled_and_missing_selections() {
        let source = NativeCollection::new(
            "id",
            ["a", "b", "c"].into_iter().map(|key| {
                BTreeMap::from([
                    ("id".into(), UiValue::String(key.into())),
                    ("title".into(), UiValue::String(key.into())),
                    ("disabled".into(), UiValue::Bool(key == "b")),
                    ("has_children".into(), UiValue::Bool(key == "a")),
                ])
            }),
        )
        .unwrap();
        let selected = BTreeSet::from(["a".into()]);
        let view = source.list_view(selected.clone()).unwrap();
        let neighbors = view.table_neighbors(&selected);
        assert_eq!(neighbors.next.as_deref(), Some("c"));
        let missing = view.table_neighbors(&BTreeSet::from(["gone".into()]));
        assert_eq!(missing.current, None);
        assert_eq!(missing.next.as_deref(), Some("a"));
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
    fn registering_a_missing_name_invalidates_its_exact_readers() {
        let mut registry = NativeCollectionRegistry::new();
        let reader = ComponentInstancePath::root("View", "late-child");
        assert!(registry.read_tracked(&reader, "late").is_err());
        assert_eq!(
            registry
                .register_with_invalidated("late".to_owned(), rows())
                .unwrap(),
            BTreeSet::from([reader])
        );
    }

    #[test]
    fn missing_dependencies_are_render_only_validated_and_bounded() {
        let mut registry = NativeCollectionRegistry::new();
        let reader = ComponentInstancePath::root("View", "missing-budget");
        assert!(matches!(
            registry.read_tracked_with_missing(&reader, &"x".repeat(257), true),
            Err(NativeCollectionError::InvalidName(_, _))
        ));
        assert!(registry.missing_readers.is_empty());

        for index in 0..128 {
            assert!(
                registry
                    .read_tracked_with_missing(&reader, &format!("event-{index}"), false)
                    .is_err()
            );
        }
        assert!(registry.missing_readers.is_empty());

        for index in 0..MAX_MISSING_NAMES_PER_READER {
            assert!(
                registry
                    .read_tracked_with_missing(&reader, &format!("render-{index}"), true)
                    .is_err()
            );
        }
        assert_eq!(registry.missing_readers.len(), MAX_MISSING_NAMES_PER_READER);
        assert!(matches!(
            registry.read_tracked_with_missing(&reader, "one-too-many", true),
            Err(NativeCollectionError::MissingDependencyBudget { .. })
        ));
        assert_eq!(registry.missing_readers.len(), MAX_MISSING_NAMES_PER_READER);
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
    fn table_rows_carry_their_selected_flag_but_not_the_next_selection() {
        let source = grouped_rows();
        for (mode, selected) in [
            ("multiple", vec!["a", "b"]),
            ("single", vec!["a"]),
            ("none", vec![]),
        ] {
            let mut config = table_config(&[]);
            config.insert("group_by".into(), Dynamic::UNIT);
            config.insert("selection_mode".into(), Dynamic::from(mode));
            config.insert(
                "selected_keys".into(),
                Dynamic::from_array(selected.iter().copied().map(Dynamic::from).collect()),
            );
            let view = source.table_view(config).unwrap();
            for index in 0..view.len() {
                let UiValue::Map(row) = view.item(index).unwrap().unwrap() else {
                    panic!("projected table row must be a map");
                };
                let UiValue::String(key) = &row["key"] else {
                    panic!("projected table row key must be a string");
                };
                assert_eq!(
                    row["selected"],
                    UiValue::Bool(selected.contains(&key.as_str())),
                    "{mode} {key}"
                );
                assert_eq!(
                    row.keys().map(String::as_str).collect::<Vec<_>>(),
                    [
                        "cells", "height", "key", "kind", "label", "selected", "striped"
                    ],
                    "{mode} {key}"
                );
            }
        }
        let mut config = table_config(&[]);
        config.insert("selection_mode".into(), Dynamic::from("several"));
        assert!(matches!(
            source.table_view(config),
            Err(NativeCollectionError::InvalidTableConfig(message))
                if message == "unknown selection mode `several`"
        ));
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

    #[test]
    fn table_projection_filters_then_pages_the_sorted_native_rows() {
        let source = grouped_rows();
        let page = |number: i64| {
            let mut config = table_config(&[]);
            config.insert("group_by".into(), Dynamic::UNIT);
            config.insert("query".into(), Dynamic::from("track-b"));
            config.insert(
                "search_fields".into(),
                Dynamic::from_array(vec![Dynamic::from("track")]),
            );
            config.insert("page".into(), Dynamic::from_int(number));
            config.insert("page_size".into(), Dynamic::from_int(1));
            source.table_view(config).unwrap()
        };
        let first = page(1);
        let second = page(2);
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert_eq!(first.key(0), Some("c"));
        assert_eq!(second.key(0), Some("b"));
        assert!(!Arc::ptr_eq(&first.order, &second.order));
    }
}
