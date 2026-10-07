//! Design-language-neutral token primitives.
//!
//! The runtime knows how to store, inherit and resolve tokens, but not which
//! tokens exist. Token paths, environment names and environment values are
//! application data: they are interned into small `Copy` symbols so styles and
//! render environments stay cheap to copy.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::{Arc, OnceLock, RwLock};

use serde::{Serialize, Serializer};
use thiserror::Error;

/// An interned token path, environment name or environment value.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Symbol(u32);

#[derive(Default)]
struct SymbolTable {
    ids: HashMap<Arc<str>, u32>,
    names: Vec<Arc<str>>,
}

fn symbols() -> &'static RwLock<SymbolTable> {
    static TABLE: OnceLock<RwLock<SymbolTable>> = OnceLock::new();
    TABLE.get_or_init(|| RwLock::new(SymbolTable::default()))
}

impl Symbol {
    /// Intern a name. Interning is process-wide, so symbols compare equal
    /// across threads and views.
    #[must_use]
    pub fn intern(name: &str) -> Self {
        if let Some(id) = symbols()
            .read()
            .ok()
            .and_then(|table| table.ids.get(name).copied())
        {
            return Self(id);
        }
        let mut table = symbols()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(id) = table.ids.get(name) {
            return Self(*id);
        }
        let id = u32::try_from(table.names.len()).unwrap_or(u32::MAX);
        let name: Arc<str> = Arc::from(name);
        table.names.push(Arc::clone(&name));
        table.ids.insert(name, id);
        Self(id)
    }

    /// The interned text.
    #[must_use]
    pub fn as_str(self) -> Arc<str> {
        symbols()
            .read()
            .ok()
            .and_then(|table| table.names.get(self.0 as usize).cloned())
            .unwrap_or_else(|| Arc::from(""))
    }
}

impl fmt::Debug for Symbol {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}", self.as_str())
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.as_str())
    }
}

impl Serialize for Symbol {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.as_str())
    }
}

/// A reference to a named length token, optionally scaled.
///
/// `theme_length("metrics.row") * 8` produces a token with scale 8, resolved
/// to eight times the row height of the inherited environment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LengthToken {
    path: Symbol,
    scale: f64,
}

impl LengthToken {
    /// Reference a token path such as `spacing.sm` or `metrics.row`.
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::InvalidPath`] unless the path is `namespace.name`
    /// with `snake_case` segments.
    pub fn new(path: &str) -> Result<Self, TokenError> {
        validate_token_path(path)?;
        Ok(Self {
            path: Symbol::intern(path),
            scale: 1.0,
        })
    }

    #[must_use]
    pub const fn path(self) -> Symbol {
        self.path
    }

    #[must_use]
    pub const fn scale(self) -> f64 {
        self.scale
    }

    /// Multiply the resolved length by `factor`.
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::InvalidScale`] for negative or non-finite factors.
    pub fn scaled(self, factor: f64) -> Result<Self, TokenError> {
        let scale = self.scale * factor;
        if scale.is_finite() && scale >= 0.0 && scale <= f64::from(f32::MAX) {
            Ok(Self { scale, ..self })
        } else {
            Err(TokenError::InvalidScale(factor))
        }
    }
}

impl Serialize for LengthToken {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("LengthToken", 2)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("scale", &self.scale)?;
        state.end()
    }
}

/// Maximum number of distinct environment values one subtree can carry.
pub const MAX_ENVIRONMENT_VALUES: usize = 8;

/// Inherited environment values for one rendered subtree.
///
/// The environment is a small sorted array of `(name, value)` symbols so it can
/// be copied with every render environment.
#[derive(Clone, Copy)]
pub struct Environment {
    entries: [(Symbol, Symbol); MAX_ENVIRONMENT_VALUES],
    len: u8,
}

impl fmt::Debug for Environment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_map()
            .entries(
                self.iter()
                    .map(|(name, value)| (name.as_str(), value.as_str())),
            )
            .finish()
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl PartialEq for Environment {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl Eq for Environment {}

impl std::hash::Hash for Environment {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for entry in self.iter() {
            entry.hash(state);
        }
    }
}

impl Environment {
    pub const EMPTY: Self = Self {
        entries: [(Symbol(0), Symbol(0)); MAX_ENVIRONMENT_VALUES],
        len: 0,
    };

    #[must_use]
    pub fn get(&self, name: Symbol) -> Option<Symbol> {
        self.iter()
            .find_map(|(candidate, value)| (candidate == name).then_some(value))
    }

    pub fn iter(&self) -> impl Iterator<Item = (Symbol, Symbol)> + '_ {
        self.entries[..usize::from(self.len)].iter().copied()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return a copy with `name` set to `value`; the nearest value wins.
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::TooManyEnvironmentValues`] when a ninth distinct
    /// name is added.
    pub fn with(mut self, name: Symbol, value: Symbol) -> Result<Self, TokenError> {
        let len = usize::from(self.len);
        if let Some(entry) = self.entries[..len]
            .iter_mut()
            .find(|(candidate, _)| *candidate == name)
        {
            entry.1 = value;
            return Ok(self);
        }
        if len == MAX_ENVIRONMENT_VALUES {
            return Err(TokenError::TooManyEnvironmentValues);
        }
        self.entries[len] = (name, value);
        self.len += 1;
        self.entries[..=len].sort_by_key(|(candidate, _)| *candidate);
        Ok(self)
    }

    /// Apply every override in order.
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::TooManyEnvironmentValues`] when the result has
    /// more than [`MAX_ENVIRONMENT_VALUES`] names.
    pub fn with_all(
        self,
        overrides: impl IntoIterator<Item = (Symbol, Symbol)>,
    ) -> Result<Self, TokenError> {
        overrides
            .into_iter()
            .try_fold(self, |environment, (name, value)| {
                environment.with(name, value)
            })
    }
}

/// Declaration of one environment value: its allowed values and default.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EnvironmentDeclaration {
    pub values: Vec<Symbol>,
    pub default: Symbol,
}

impl EnvironmentDeclaration {
    /// Build a declaration from strings.
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::InvalidDeclaration`] for empty or duplicate
    /// values or a default that is not one of the values.
    pub fn new(name: &str, values: &[String], default: &str) -> Result<Self, TokenError> {
        let mut seen = std::collections::BTreeSet::new();
        if values.is_empty()
            || values
                .iter()
                .any(|value| !valid_token_segment(value) || !seen.insert(value.as_str()))
            || !seen.contains(default)
        {
            return Err(TokenError::InvalidDeclaration(name.to_owned()));
        }
        Ok(Self {
            values: values.iter().map(|value| Symbol::intern(value)).collect(),
            default: Symbol::intern(default),
        })
    }

    #[must_use]
    pub fn allows(&self, value: Symbol) -> bool {
        self.values.contains(&value)
    }
}

/// Declared environment values, keyed by name.
pub type EnvironmentDeclarations = BTreeMap<Symbol, EnvironmentDeclaration>;

/// A value that depends on inherited environment values.
///
/// `keys` lists the environment names in lookup order; each entry is keyed by
/// the corresponding values.
#[derive(Clone, Debug, PartialEq)]
pub struct EnvTable<T> {
    keys: Vec<Symbol>,
    entries: BTreeMap<Vec<Symbol>, T>,
}

impl<T> EnvTable<T> {
    /// Build a table and check that every key path has the declared depth.
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::InvalidEnvTable`] for empty keys or entries whose
    /// path length differs from the key count.
    pub fn new(keys: Vec<Symbol>, entries: BTreeMap<Vec<Symbol>, T>) -> Result<Self, TokenError> {
        if keys.is_empty() || entries.keys().any(|path| path.len() != keys.len()) {
            return Err(TokenError::InvalidEnvTable);
        }
        Ok(Self { keys, entries })
    }

    #[must_use]
    pub fn keys(&self) -> &[Symbol] {
        &self.keys
    }

    /// The value at one key path (one value per key, in key order).
    #[must_use]
    pub fn get(&self, path: &[Symbol]) -> Option<&T> {
        self.entries.get(path)
    }

    pub fn entries(&self) -> impl Iterator<Item = (&[Symbol], &T)> {
        self.entries
            .iter()
            .map(|(path, value)| (path.as_slice(), value))
    }

    /// Resolve against the inherited environment, using declared defaults for
    /// names the environment does not set.
    #[must_use]
    pub fn resolve(
        &self,
        environment: &Environment,
        declarations: &EnvironmentDeclarations,
    ) -> Option<&T> {
        let path = self
            .keys
            .iter()
            .map(|name| {
                let declaration = declarations.get(name);
                match (environment.get(*name), declaration) {
                    // An undeclared value falls back to the declared default.
                    (Some(value), Some(declaration)) if !declaration.allows(value) => {
                        Some(declaration.default)
                    }
                    (Some(value), _) => Some(value),
                    (None, declaration) => declaration.map(|declaration| declaration.default),
                }
            })
            .collect::<Option<Vec<_>>>()?;
        self.entries.get(&path)
    }

    /// Map every entry while keeping the key structure.
    ///
    /// # Errors
    ///
    /// Propagates the first mapping error.
    pub fn try_map<U, E>(&self, mut map: impl FnMut(&T) -> Result<U, E>) -> Result<EnvTable<U>, E> {
        Ok(EnvTable {
            keys: self.keys.clone(),
            entries: self
                .entries
                .iter()
                .map(|(path, value)| Ok((path.clone(), map(value)?)))
                .collect::<Result<_, E>>()?,
        })
    }
}

impl<T: Serialize> Serialize for EnvTable<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let entries = self
            .entries
            .iter()
            .map(|(path, value)| {
                (
                    path.iter()
                        .map(|symbol| symbol.as_str().to_string())
                        .collect::<Vec<_>>()
                        .join("."),
                    value,
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut state = serializer.serialize_struct("EnvTable", 2)?;
        state.serialize_field("keys", &self.keys)?;
        state.serialize_field("entries", &entries)?;
        state.end()
    }
}

/// A value that is either fixed or depends on the environment.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Variable<T> {
    Fixed(T),
    ByEnv(EnvTable<T>),
}

impl<T> Variable<T> {
    #[must_use]
    pub fn resolve(
        &self,
        environment: &Environment,
        declarations: &EnvironmentDeclarations,
    ) -> Option<&T> {
        match self {
            Self::Fixed(value) => Some(value),
            Self::ByEnv(table) => table.resolve(environment, declarations),
        }
    }

    /// Every value this variable can take.
    pub fn values(&self) -> Box<dyn Iterator<Item = &T> + '_> {
        match self {
            Self::Fixed(value) => Box::new(std::iter::once(value)),
            Self::ByEnv(table) => Box::new(table.entries.values()),
        }
    }
}

/// Validate a `namespace.name` token path.
///
/// # Errors
///
/// Returns [`TokenError::InvalidPath`] for anything else.
pub fn validate_token_path(path: &str) -> Result<(), TokenError> {
    match path.split_once('.') {
        Some((namespace, name)) if valid_token_segment(namespace) && valid_token_segment(name) => {
            Ok(())
        }
        _ => Err(TokenError::InvalidPath(path.to_owned())),
    }
}

/// A `snake_case` token segment: lowercase ASCII letters, digits and single
/// underscores, not starting or ending with an underscore.
#[must_use]
pub fn valid_token_segment(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('_')
        && !value.ends_with('_')
        && !value.contains("__")
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum TokenError {
    #[error("token path `{0}` must be `namespace.name` with snake_case segments")]
    InvalidPath(String),
    #[error("token scale must be finite and non-negative, got {0}")]
    InvalidScale(f64),
    #[error("a subtree can carry at most {MAX_ENVIRONMENT_VALUES} environment values")]
    TooManyEnvironmentValues,
    #[error(
        "environment declaration `{0}` needs unique snake_case values and a default among them"
    )]
    InvalidDeclaration(String),
    #[error("an environment table needs keys and entries of matching depth")]
    InvalidEnvTable,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol(name: &str) -> Symbol {
        Symbol::intern(name)
    }

    #[test]
    fn symbols_are_stable_and_round_trip() {
        let first = symbol("metrics.row");
        assert_eq!(first, symbol("metrics.row"));
        assert_eq!(&*first.as_str(), "metrics.row");
        assert_ne!(first, symbol("metrics.inset"));
    }

    #[test]
    fn environment_keeps_the_nearest_value_and_bounds_its_size() {
        let density = symbol("density");
        let environment = Environment::EMPTY
            .with(density, symbol("comfortable"))
            .and_then(|environment| environment.with(density, symbol("compact")))
            .expect("environment");
        assert_eq!(environment.get(density), Some(symbol("compact")));
        let mut full = Environment::EMPTY;
        for index in 0..MAX_ENVIRONMENT_VALUES {
            full = full
                .with(symbol(&format!("name_{index}")), symbol("value"))
                .expect("within bound");
        }
        assert_eq!(
            full.with(symbol("overflow"), symbol("value")),
            Err(TokenError::TooManyEnvironmentValues)
        );
    }

    #[test]
    fn env_tables_resolve_with_declared_defaults() {
        let density = symbol("density");
        let size = symbol("size");
        let declarations = EnvironmentDeclarations::from([
            (
                density,
                EnvironmentDeclaration::new(
                    "density",
                    &["comfortable".to_owned(), "compact".to_owned()],
                    "comfortable",
                )
                .expect("density"),
            ),
            (
                size,
                EnvironmentDeclaration::new("size", &["sm".to_owned(), "md".to_owned()], "md")
                    .expect("size"),
            ),
        ]);
        let table = EnvTable::new(
            vec![density, size],
            BTreeMap::from([
                (vec![symbol("comfortable"), symbol("md")], 32),
                (vec![symbol("comfortable"), symbol("sm")], 28),
                (vec![symbol("compact"), symbol("md")], 28),
                (vec![symbol("compact"), symbol("sm")], 24),
            ]),
        )
        .expect("table");
        assert_eq!(table.resolve(&Environment::EMPTY, &declarations), Some(&32));
        let compact = Environment::EMPTY
            .with(density, symbol("compact"))
            .and_then(|environment| environment.with(size, symbol("sm")))
            .expect("environment");
        assert_eq!(table.resolve(&compact, &declarations), Some(&24));
        assert_eq!(
            table.resolve(&compact, &EnvironmentDeclarations::new()),
            Some(&24)
        );
        assert_eq!(
            table.resolve(&Environment::EMPTY, &EnvironmentDeclarations::new()),
            None
        );
    }

    #[test]
    fn token_paths_are_validated_and_scaled() {
        assert!(LengthToken::new("metrics.row").is_ok());
        assert!(LengthToken::new("row").is_err());
        assert!(LengthToken::new("Metrics.row").is_err());
        let token = LengthToken::new("metrics.row")
            .and_then(|token| token.scaled(8.0))
            .expect("scaled");
        assert!((token.scale() - 8.0).abs() < f64::EPSILON);
        assert!(token.scaled(-1.0).is_err());
    }
}
