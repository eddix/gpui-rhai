use std::collections::BTreeMap;
use std::sync::Arc;

use rhai::Engine;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::token::{
    EnvironmentDeclaration, EnvironmentDeclarations, Symbol, Variable, valid_token_segment,
};
use crate::{ColorResolver, ColorValue, ComponentInstancePath, Environment, Length, Rgba8};

const REQUIRED_MOTION_DURATIONS: &[&str] = &["instant", "fast", "normal", "slow", "ambient"];
const REQUIRED_MOTION_EASINGS: &[&str] = &["standard", "entrance", "exit", "emphasized"];
const REQUIRED_MOTION_SPRINGS: &[&str] = &["responsive", "gentle", "bouncy"];
const REQUIRED_MOTION_DISTANCES: &[&str] = &["subtle", "moderate", "large"];
const REQUIRED_MOTION_STAGGERS: &[&str] = &["tight", "normal", "relaxed"];
const MAX_TYPOGRAPHY_ALIAS_DEPTH: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    Light,
    Dark,
}

/// Lightweight resolved identity, distinct from a user preference or token snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThemeVariantInfo {
    pub family: String,
    pub name: String,
    pub mode: ThemeMode,
}

/// A length token value that may depend on inherited environment values.
pub type ThemeLength = Variable<Length>;

/// The resolved token set of one theme variant.
///
/// The runtime imposes no vocabulary: every map may be empty. Colors are
/// final values; color expressions are evaluated when the token layers are
/// merged. Lengths and typography aliases may depend on environment values
/// and are resolved during native rendering.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ThemeTokens {
    pub colors: BTreeMap<String, Rgba8>,
    pub spacing: BTreeMap<String, ThemeLength>,
    pub radii: BTreeMap<String, ThemeLength>,
    pub typography: ThemeTypography,
    pub motion: ThemeMotion,
    pub namespaces: BTreeMap<String, BTreeMap<String, ThemeTokenValue>>,
    pub environment: EnvironmentDeclarations,
}

/// Host-owned token preferences applied uniformly to every loaded theme.
///
/// Overrides are intentionally partial: absent entries inherit the value from
/// the token base and each theme, while present entries replace it. Theme
/// family, variant name, and color mode are never host-overridable.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThemeTokenOverrides {
    pub colors: BTreeMap<String, Rgba8>,
    pub spacing: BTreeMap<String, Length>,
    pub radii: BTreeMap<String, Length>,
    pub typography: ThemeTypographyOverrides,
    pub motion: ThemeMotionOverrides,
    pub namespaces: BTreeMap<String, BTreeMap<String, ThemeTokenValue>>,
    /// Replaces declared environment defaults, for example a compact density
    /// preference on small screens.
    pub environment_defaults: BTreeMap<String, String>,
}

/// Partial host preferences for the shared typography system.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThemeTypographyOverrides {
    /// Replaces the primary UI family when present.
    pub family: Option<String>,
    /// Replaces, rather than appends to, the fallback stack when present.
    pub fallbacks: Option<Vec<String>>,
    /// Replaces individual typography roles by name.
    pub roles: BTreeMap<String, TypographyToken>,
}

/// Partial host preferences for semantic motion tokens.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThemeMotionOverrides {
    pub durations_ms: BTreeMap<String, u64>,
    pub easings: BTreeMap<String, crate::MotionEasing>,
    pub springs: BTreeMap<String, ThemeMotionSpring>,
    pub distances: BTreeMap<String, f64>,
    pub staggers_ms: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThemeMotion {
    pub durations_ms: BTreeMap<String, u64>,
    pub easings: BTreeMap<String, crate::MotionEasing>,
    pub springs: BTreeMap<String, ThemeMotionSpring>,
    pub distances: BTreeMap<String, f64>,
    pub staggers_ms: BTreeMap<String, u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThemeMotionSpring {
    pub stiffness: f64,
    pub damping: f64,
    pub mass: f64,
}

impl Default for ThemeMotion {
    fn default() -> Self {
        Self {
            durations_ms: BTreeMap::from([
                ("instant".to_owned(), 1),
                ("fast".to_owned(), 120),
                ("normal".to_owned(), 180),
                ("slow".to_owned(), 320),
                ("ambient".to_owned(), 1_100),
            ]),
            easings: BTreeMap::from([
                ("standard".to_owned(), crate::MotionEasing::EaseInOut),
                ("entrance".to_owned(), crate::MotionEasing::EaseOut),
                ("exit".to_owned(), crate::MotionEasing::EaseIn),
                ("emphasized".to_owned(), crate::MotionEasing::EaseInOut),
            ]),
            springs: BTreeMap::from([
                (
                    "responsive".to_owned(),
                    ThemeMotionSpring {
                        stiffness: 240.0,
                        damping: 26.0,
                        mass: 1.0,
                    },
                ),
                (
                    "gentle".to_owned(),
                    ThemeMotionSpring {
                        stiffness: 140.0,
                        damping: 22.0,
                        mass: 1.0,
                    },
                ),
                (
                    "bouncy".to_owned(),
                    ThemeMotionSpring {
                        stiffness: 280.0,
                        damping: 16.0,
                        mass: 1.0,
                    },
                ),
            ]),
            distances: BTreeMap::from([
                ("subtle".to_owned(), 4.0),
                ("moderate".to_owned(), 12.0),
                ("large".to_owned(), 32.0),
            ]),
            staggers_ms: BTreeMap::from([
                ("tight".to_owned(), 24),
                ("normal".to_owned(), 48),
                ("relaxed".to_owned(), 80),
            ]),
        }
    }
}

impl ThemeMotion {
    /// Validate the semantic motion token contract.
    ///
    /// # Errors
    ///
    /// Returns a theme error for missing or physically invalid tokens.
    pub fn validate(&self) -> Result<(), ThemeError> {
        require_tokens(
            "motion duration",
            REQUIRED_MOTION_DURATIONS,
            &self.durations_ms,
        )?;
        require_tokens("motion easing", REQUIRED_MOTION_EASINGS, &self.easings)?;
        require_tokens("motion spring", REQUIRED_MOTION_SPRINGS, &self.springs)?;
        require_tokens(
            "motion distance",
            REQUIRED_MOTION_DISTANCES,
            &self.distances,
        )?;
        require_tokens(
            "motion stagger",
            REQUIRED_MOTION_STAGGERS,
            &self.staggers_ms,
        )?;
        if self.durations_ms.values().any(|duration| *duration == 0)
            || self.staggers_ms.values().any(|duration| *duration == 0)
            || self
                .distances
                .values()
                .any(|distance| !distance.is_finite() || *distance < 0.0)
            || self.springs.values().any(|spring| {
                !spring.stiffness.is_finite()
                    || spring.stiffness <= 0.0
                    || !spring.damping.is_finite()
                    || spring.damping < 0.0
                    || !spring.mass.is_finite()
                    || spring.mass <= 0.0
            })
        {
            return Err(ThemeError::InvalidMotionTokens);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ThemeTypography {
    /// Primary UI family; `None` keeps the platform font.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fallbacks: Vec<String>,
    pub roles: BTreeMap<String, TypographyRole>,
}

/// A named typography role: a concrete style or an alias to another role.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TypographyRole {
    Style(TypographyToken),
    /// Another role's name, optionally chosen by environment values (for
    /// example a `control` role that maps `xs` to `body_small`).
    Alias(Variable<String>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TypographyToken {
    pub size: Length,
    pub line_height: Length,
    pub weight: u16,
    /// Role-specific family, replacing the shared UI family (for example a
    /// monospace label role).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// Role-specific fallbacks; used only together with `family`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallbacks: Vec<String>,
}

impl TypographyToken {
    /// A role in the shared UI family.
    #[must_use]
    pub const fn new(size: Length, line_height: Length, weight: u16) -> Self {
        Self {
            size,
            line_height,
            weight,
            family: None,
            fallbacks: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedTypography {
    pub family: Option<String>,
    pub fallbacks: Vec<String>,
    pub size: Length,
    pub line_height: Length,
    pub weight: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ThemeTokenValue {
    Color(Rgba8),
    Length(ThemeLength),
    Number(f64),
    String(String),
}

impl ThemeTokens {
    #[must_use]
    pub fn token(&self, path: &str) -> Option<&ThemeTokenValue> {
        let (namespace, name) = path.split_once('.')?;
        self.namespaces.get(namespace)?.get(name)
    }

    #[must_use]
    pub fn color(&self, token: &str) -> Option<Rgba8> {
        self.colors
            .get(token)
            .copied()
            .or_else(|| match self.token(token) {
                Some(ThemeTokenValue::Color(color)) => Some(*color),
                _ => None,
            })
    }

    /// The length token at `path` (`spacing.sm`, `radius.md`, `metrics.row`)
    /// before environment resolution.
    #[must_use]
    pub fn length_token(&self, path: &str) -> Option<&ThemeLength> {
        let (namespace, name) = path.split_once('.')?;
        match namespace {
            "spacing" => self.spacing.get(name),
            "radius" => self.radii.get(name),
            _ => match self.namespaces.get(namespace)?.get(name)? {
                ThemeTokenValue::Length(length) => Some(length),
                _ => None,
            },
        }
    }

    /// Resolve a length against the inherited environment. Literal lengths
    /// pass through; scaled tokens multiply their resolved value.
    #[must_use]
    pub fn resolve_length(&self, length: Length, environment: &Environment) -> Option<Length> {
        let Length::Token(token) = length else {
            return Some(length);
        };
        let value = *self
            .length_token(&token.path().as_str())?
            .resolve(environment, &self.environment)?;
        let scale = token.scale();
        match value {
            Length::Pixels(value) => Some(Length::Pixels(value * scale)),
            Length::Rems(value) => Some(Length::Rems(value * scale)),
            Length::Relative(value) => Some(Length::Relative((value * scale).min(1.0))),
            Length::Token(_) => None,
        }
    }

    /// Every length token resolved for one environment, keyed by path.
    #[must_use]
    pub fn length_snapshot(&self, environment: &Environment) -> BTreeMap<String, Length> {
        let mut lengths = BTreeMap::new();
        let mut insert = |path: String, value: &ThemeLength| {
            if let Some(value) = value.resolve(environment, &self.environment) {
                lengths.insert(path, *value);
            }
        };
        for (name, value) in &self.spacing {
            insert(format!("spacing.{name}"), value);
        }
        for (name, value) in &self.radii {
            insert(format!("radius.{name}"), value);
        }
        for (namespace, values) in &self.namespaces {
            for (name, value) in values {
                if let ThemeTokenValue::Length(value) = value {
                    insert(format!("{namespace}.{name}"), value);
                }
            }
        }
        lengths
    }

    /// Resolve a typography role, following aliases with the environment.
    #[must_use]
    pub fn resolve_typography(
        &self,
        role: &str,
        environment: &Environment,
    ) -> Option<ResolvedTypography> {
        let mut current = role.to_owned();
        for _ in 0..MAX_TYPOGRAPHY_ALIAS_DEPTH {
            match self.typography.roles.get(&current)? {
                TypographyRole::Style(token) => {
                    let (family, fallbacks) = if token.family.is_some() {
                        (token.family.clone(), token.fallbacks.clone())
                    } else {
                        (
                            self.typography.family.clone(),
                            self.typography.fallbacks.clone(),
                        )
                    };
                    return Some(ResolvedTypography {
                        family,
                        fallbacks,
                        size: token.size,
                        line_height: token.line_height,
                        weight: token.weight,
                    });
                }
                TypographyRole::Alias(target) => {
                    current = target.resolve(environment, &self.environment)?.clone();
                }
            }
        }
        None
    }

    /// Every typography role resolved for one environment.
    #[must_use]
    pub fn typography_snapshot(
        &self,
        environment: &Environment,
    ) -> BTreeMap<String, ResolvedTypography> {
        self.typography
            .roles
            .keys()
            .filter_map(|role| {
                self.resolve_typography(role, environment)
                    .map(|value| (role.clone(), value))
            })
            .collect()
    }

    /// Every color, with namespaced colors keyed `namespace.name`.
    #[must_use]
    pub fn color_snapshot(&self) -> BTreeMap<String, Rgba8> {
        let mut colors = self.colors.clone();
        for (namespace, values) in &self.namespaces {
            for (name, value) in values {
                if let ThemeTokenValue::Color(color) = value {
                    colors.insert(format!("{namespace}.{name}"), *color);
                }
            }
        }
        colors
    }

    /// Whether a declared token requirement is satisfied.
    ///
    /// A bare name is a semantic color; `spacing.*`, `radius.*` and other
    /// `namespace.name` paths are lengths or namespaced tokens;
    /// `typography.<role>` is a role; `environment.<name>` is a declaration.
    #[must_use]
    pub fn provides(&self, requirement: &str) -> bool {
        match requirement.split_once('.') {
            None => self.colors.contains_key(requirement),
            Some(("typography", role)) => self.typography.roles.contains_key(role),
            Some(("environment", name)) => self.environment.contains_key(&Symbol::intern(name)),
            Some(_) => {
                self.length_token(requirement).is_some() || self.token(requirement).is_some()
            }
        }
    }

    /// Validate token names and values. No token is required here; required
    /// tokens are declared by components and checked by
    /// [`ThemeVariant::require`].
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] for malformed names, invalid lengths, invalid
    /// typography, invalid motion or environment tables that reference
    /// undeclared names or values.
    pub fn validate(&self) -> Result<(), ThemeError> {
        for name in self.colors.keys() {
            if !valid_token_segment(name) {
                return Err(ThemeError::InvalidTokenName {
                    namespace: "colors".to_owned(),
                    name: name.clone(),
                });
            }
        }
        for (category, values) in [("spacing", &self.spacing), ("radius", &self.radii)] {
            for (name, value) in values {
                if !valid_token_segment(name) {
                    return Err(ThemeError::InvalidTokenName {
                        namespace: category.to_owned(),
                        name: name.clone(),
                    });
                }
                self.validate_length(&format!("{category}.{name}"), value)?;
            }
        }
        for (namespace, tokens) in &self.namespaces {
            if !valid_token_segment(namespace) {
                return Err(ThemeError::InvalidNamespace(namespace.clone()));
            }
            for (name, value) in tokens {
                if !valid_token_segment(name) {
                    return Err(ThemeError::InvalidTokenName {
                        namespace: namespace.clone(),
                        name: name.clone(),
                    });
                }
                match value {
                    ThemeTokenValue::Length(length) => {
                        self.validate_length(&format!("{namespace}.{name}"), length)?;
                    }
                    ThemeTokenValue::Number(number) if !number.is_finite() => {
                        return Err(ThemeError::NonFiniteNumber {
                            namespace: namespace.clone(),
                            name: name.clone(),
                        });
                    }
                    ThemeTokenValue::Color(_)
                    | ThemeTokenValue::Number(_)
                    | ThemeTokenValue::String(_) => {}
                }
            }
        }
        self.validate_typography()?;
        self.motion.validate()
    }

    fn validate_length(&self, token: &str, value: &ThemeLength) -> Result<(), ThemeError> {
        self.validate_variable(token, value)?;
        for length in value.values() {
            if length.is_theme_token() {
                return Err(ThemeError::NestedLengthToken(token.to_owned()));
            }
            length
                .validate()
                .map_err(|source| ThemeError::InvalidLength {
                    token: token.to_owned(),
                    source,
                })?;
        }
        Ok(())
    }

    fn validate_variable<T>(&self, token: &str, value: &Variable<T>) -> Result<(), ThemeError> {
        let Variable::ByEnv(table) = value else {
            return Ok(());
        };
        for (index, name) in table.keys().iter().enumerate() {
            let declaration =
                self.environment
                    .get(name)
                    .ok_or_else(|| ThemeError::UndeclaredEnvironment {
                        token: token.to_owned(),
                        name: name.to_string(),
                    })?;
            for (path, _) in table.entries() {
                if !declaration.allows(path[index]) {
                    return Err(ThemeError::UndeclaredEnvironmentValue {
                        token: token.to_owned(),
                        name: name.to_string(),
                        value: path[index].to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    fn validate_typography(&self) -> Result<(), ThemeError> {
        let typography = &self.typography;
        if typography
            .family
            .as_ref()
            .is_some_and(|family| !valid_font_family(family))
        {
            return Err(ThemeError::InvalidTypographyFamily);
        }
        validate_fallbacks(typography.family.as_deref(), &typography.fallbacks)?;
        for (role, value) in &typography.roles {
            if !valid_token_segment(role) {
                return Err(ThemeError::InvalidTypographyRole(role.clone()));
            }
            match value {
                TypographyRole::Style(token) => validate_typography_token(role, token)?,
                TypographyRole::Alias(target) => {
                    self.validate_variable(&format!("typography.{role}"), target)?;
                    for target in target.values() {
                        if !typography.roles.contains_key(target) {
                            return Err(ThemeError::UnknownTypographyAlias {
                                role: role.clone(),
                                target: target.clone(),
                            });
                        }
                    }
                }
            }
        }
        for role in typography.roles.keys() {
            if self.alias_cycle(role) {
                return Err(ThemeError::TypographyAliasCycle(role.clone()));
            }
        }
        Ok(())
    }

    fn alias_cycle(&self, role: &str) -> bool {
        let mut frontier = vec![(role.to_owned(), 0_usize)];
        while let Some((current, depth)) = frontier.pop() {
            if depth > MAX_TYPOGRAPHY_ALIAS_DEPTH {
                return true;
            }
            if let Some(TypographyRole::Alias(target)) = self.typography.roles.get(&current) {
                frontier.extend(target.values().map(|next| (next.clone(), depth + 1)));
            }
        }
        false
    }
}

fn validate_typography_token(role: &str, token: &TypographyToken) -> Result<(), ThemeError> {
    validate_typography_length(role, "size", token.size)?;
    validate_typography_length(role, "line_height", token.line_height)?;
    if !(1..=1_000).contains(&token.weight) {
        return Err(ThemeError::InvalidTypographyWeight {
            role: role.to_owned(),
            weight: token.weight,
        });
    }
    if token
        .family
        .as_ref()
        .is_some_and(|family| !valid_font_family(family))
    {
        return Err(ThemeError::InvalidTypographyFamily);
    }
    validate_fallbacks(token.family.as_deref(), &token.fallbacks)?;
    match (token.size, token.line_height) {
        (Length::Pixels(size), Length::Pixels(line_height))
        | (Length::Rems(size), Length::Rems(line_height))
            if line_height < size =>
        {
            Err(ThemeError::InvalidTypographyLineHeight(role.to_owned()))
        }
        _ => Ok(()),
    }
}

fn validate_fallbacks(family: Option<&str>, fallbacks: &[String]) -> Result<(), ThemeError> {
    let mut families = std::collections::BTreeSet::new();
    for fallback in fallbacks {
        if !valid_font_family(fallback) || !families.insert(fallback.as_str()) {
            return Err(ThemeError::InvalidTypographyFallbacks);
        }
    }
    if family.is_some_and(|family| families.contains(family)) {
        return Err(ThemeError::InvalidTypographyFallbacks);
    }
    Ok(())
}

fn valid_font_family(family: &str) -> bool {
    let trimmed = family.trim();
    !trimmed.is_empty() && trimmed.len() <= 256
}

fn validate_typography_length(
    role: &str,
    field: &'static str,
    value: Length,
) -> Result<(), ThemeError> {
    let positive = match value {
        Length::Pixels(value) | Length::Rems(value) => value.is_finite() && value > 0.0,
        Length::Relative(_) | Length::Token(_) => false,
    };
    if positive {
        Ok(())
    } else {
        Err(ThemeError::InvalidTypographyLength {
            role: role.to_owned(),
            field,
        })
    }
}

fn require_tokens<T>(
    category: &'static str,
    required: &[&str],
    actual: &BTreeMap<String, T>,
) -> Result<(), ThemeError> {
    let missing = required
        .iter()
        .filter(|name| !actual.contains_key(**name))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(ThemeError::MissingTokens { category, missing })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ThemeVariant {
    pub family: String,
    pub name: String,
    pub mode: ThemeMode,
    pub tokens: Arc<ThemeTokens>,
}

/// Owned, host-readable snapshot of the effective theme for one mounted view.
///
/// `revision` advances whenever the resolved variant changes, including a
/// system light/dark transition. The complete token table is available
/// through [`Self::variant`].
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeSnapshot {
    pub revision: u64,
    pub variant: ThemeVariant,
}

impl ThemeSnapshot {
    pub(crate) const fn new(revision: u64, variant: ThemeVariant) -> Self {
        Self { revision, variant }
    }
}

impl ThemeVariant {
    /// Validate identity and token format.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] for empty names or an invalid token set.
    pub fn validate(&self) -> Result<(), ThemeError> {
        if self.family.trim().is_empty() || self.name.trim().is_empty() {
            return Err(ThemeError::EmptyName);
        }
        self.tokens.validate()
    }

    /// Check that every requirement declared by a consumer is provided.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError::MissingRequiredTokens`] listing what is absent.
    pub fn require<'a>(
        &self,
        consumer: &str,
        requirements: impl IntoIterator<Item = &'a str>,
    ) -> Result<(), ThemeError> {
        let missing = requirements
            .into_iter()
            .filter(|requirement| !self.tokens.provides(requirement))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(ThemeError::MissingRequiredTokens {
                theme: format!("{}/{}", self.family, self.name),
                consumer: consumer.to_owned(),
                missing,
            })
        }
    }

    #[must_use]
    pub fn typography(&self, role: &str) -> Option<ResolvedTypography> {
        self.tokens.resolve_typography(role, &Environment::EMPTY)
    }
}

impl ThemeTokenOverrides {
    fn into_layer(self) -> TokenLayer {
        let mut layer = TokenLayer {
            colors: self
                .colors
                .into_iter()
                .map(|(name, color)| (name, ColorValue::Literal(color)))
                .collect(),
            spacing: self
                .spacing
                .into_iter()
                .map(|(name, value)| (name, Variable::Fixed(value)))
                .collect(),
            radii: self
                .radii
                .into_iter()
                .map(|(name, value)| (name, Variable::Fixed(value)))
                .collect(),
            typography_family: self.typography.family.map(FamilyChoice::Named),
            typography_fallbacks: self.typography.fallbacks,
            typography_roles: self
                .typography
                .roles
                .into_iter()
                .map(|(name, token)| (name, TypographyRole::Style(token)))
                .collect(),
            motion: self.motion,
            namespaces: self.namespaces,
            environment: BTreeMap::new(),
            environment_defaults: self.environment_defaults,
        };
        layer.normalize_colors();
        layer
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// A layer's choice of UI family: keep the platform font or name one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FamilyChoice {
    Platform,
    Named(String),
}

/// One unresolved token layer: the token base, a palette theme, or Host
/// overrides. Layers merge entry by entry; later layers win.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TokenLayer {
    pub(crate) colors: BTreeMap<String, ColorValue>,
    pub(crate) spacing: BTreeMap<String, ThemeLength>,
    pub(crate) radii: BTreeMap<String, ThemeLength>,
    pub(crate) typography_family: Option<FamilyChoice>,
    pub(crate) typography_fallbacks: Option<Vec<String>>,
    pub(crate) typography_roles: BTreeMap<String, TypographyRole>,
    pub(crate) motion: ThemeMotionOverrides,
    pub(crate) namespaces: BTreeMap<String, BTreeMap<String, ThemeTokenValue>>,
    pub(crate) environment: BTreeMap<String, EnvironmentDeclaration>,
    pub(crate) environment_defaults: BTreeMap<String, String>,
}

impl TokenLayer {
    /// Move namespaced literal colors into the color map under their dotted
    /// path, so precedence between layers and expression references work
    /// the same for every color.
    pub(crate) fn normalize_colors(&mut self) {
        for (namespace, tokens) in &mut self.namespaces {
            let colors = &mut self.colors;
            tokens.retain(|name, value| {
                if let ThemeTokenValue::Color(color) = value {
                    colors.insert(format!("{namespace}.{name}"), ColorValue::Literal(*color));
                    false
                } else {
                    true
                }
            });
        }
        self.namespaces.retain(|_, tokens| !tokens.is_empty());
    }

    fn merge(&mut self, other: Self) {
        self.colors.extend(other.colors);
        self.spacing.extend(other.spacing);
        self.radii.extend(other.radii);
        if other.typography_family.is_some() {
            self.typography_family = other.typography_family;
        }
        if other.typography_fallbacks.is_some() {
            self.typography_fallbacks = other.typography_fallbacks;
        }
        self.typography_roles.extend(other.typography_roles);
        self.motion.durations_ms.extend(other.motion.durations_ms);
        self.motion.easings.extend(other.motion.easings);
        self.motion.springs.extend(other.motion.springs);
        self.motion.distances.extend(other.motion.distances);
        self.motion.staggers_ms.extend(other.motion.staggers_ms);
        for (namespace, tokens) in other.namespaces {
            self.namespaces.entry(namespace).or_default().extend(tokens);
        }
        self.environment.extend(other.environment);
        self.environment_defaults.extend(other.environment_defaults);
    }

    /// Evaluate color expressions and produce the final token set.
    fn finalize(self) -> Result<ThemeTokens, ThemeError> {
        let mut environment = EnvironmentDeclarations::new();
        for (name, mut declaration) in self.environment {
            if !valid_token_segment(&name) {
                return Err(ThemeError::InvalidTokenName {
                    namespace: "environment".to_owned(),
                    name,
                });
            }
            if let Some(default) = self.environment_defaults.get(&name) {
                let default = Symbol::intern(default);
                if !declaration.allows(default) {
                    return Err(ThemeError::UndeclaredEnvironmentValue {
                        token: "environment default".to_owned(),
                        name,
                        value: default.to_string(),
                    });
                }
                declaration.default = default;
            }
            environment.insert(Symbol::intern(&name), declaration);
        }
        if let Some(name) = self
            .environment_defaults
            .keys()
            .find(|name| !environment.contains_key(&Symbol::intern(name)))
        {
            return Err(ThemeError::UndeclaredEnvironment {
                token: "environment default".to_owned(),
                name: name.clone(),
            });
        }

        let resolved = resolve_color_expressions(&self.colors)?;
        let mut colors = BTreeMap::new();
        let mut namespaces = self.namespaces;
        for (name, color) in resolved {
            match name.split_once('.') {
                Some((namespace, token)) => {
                    namespaces
                        .entry(namespace.to_owned())
                        .or_default()
                        .insert(token.to_owned(), ThemeTokenValue::Color(color));
                }
                None => {
                    colors.insert(name, color);
                }
            }
        }
        let mut motion = ThemeMotion::default();
        motion.durations_ms.extend(self.motion.durations_ms);
        motion.easings.extend(self.motion.easings);
        motion.springs.extend(self.motion.springs);
        motion.distances.extend(self.motion.distances);
        motion.staggers_ms.extend(self.motion.staggers_ms);
        Ok(ThemeTokens {
            colors,
            spacing: self.spacing,
            radii: self.radii,
            typography: ThemeTypography {
                family: match self.typography_family {
                    Some(FamilyChoice::Named(family)) => Some(family),
                    Some(FamilyChoice::Platform) | None => None,
                },
                fallbacks: self.typography_fallbacks.unwrap_or_default(),
                roles: self.typography_roles,
            },
            motion,
            namespaces,
            environment,
        })
    }
}

/// Evaluate every color, following token references between colors.
fn resolve_color_expressions(
    colors: &BTreeMap<String, ColorValue>,
) -> Result<BTreeMap<String, Rgba8>, ThemeError> {
    fn visit(
        name: &str,
        colors: &BTreeMap<String, ColorValue>,
        resolved: &mut BTreeMap<String, Rgba8>,
        stack: &mut Vec<String>,
    ) -> Result<Option<Rgba8>, ThemeError> {
        if let Some(color) = resolved.get(name) {
            return Ok(Some(*color));
        }
        let Some(expression) = colors.get(name) else {
            return Ok(None);
        };
        if stack.iter().any(|entry| entry == name) {
            stack.push(name.to_owned());
            return Err(ThemeError::ColorCycle(stack.join(" -> ")));
        }
        stack.push(name.to_owned());
        let mut failure = None;
        let value = expression.resolve_with(&mut |token: &str| match visit(
            token, colors, resolved, stack,
        ) {
            Ok(value) => value,
            Err(error) => {
                failure.get_or_insert(error);
                None
            }
        });
        stack.pop();
        if let Some(error) = failure {
            return Err(error);
        }
        let value = value.ok_or_else(|| ThemeError::UnresolvedColor {
            token: name.to_owned(),
            expression: format!("{expression:?}"),
        })?;
        resolved.insert(name.to_owned(), value);
        Ok(Some(value))
    }

    let mut resolved = BTreeMap::new();
    for name in colors.keys() {
        visit(name, colors, &mut resolved, &mut Vec::new())?;
    }
    Ok(resolved)
}

impl ColorResolver for ThemeVariant {
    fn resolve_token(&self, token: &str) -> Option<Rgba8> {
        self.tokens.color(token)
    }

    fn resolve_length_in(&self, length: Length, environment: &Environment) -> Option<Length> {
        self.tokens.resolve_length(length, environment)
    }

    fn resolve_typography_in(
        &self,
        role: &str,
        environment: &Environment,
    ) -> Option<ResolvedTypography> {
        self.tokens.resolve_typography(role, environment)
    }

    fn token_set(&self) -> Option<Arc<ThemeTokens>> {
        Some(Arc::clone(&self.tokens))
    }

    fn color_snapshot(&self) -> BTreeMap<String, Rgba8> {
        self.tokens.color_snapshot()
    }

    fn resolve_motion(&self) -> ThemeMotion {
        self.tokens.motion.clone()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ThemeFamily {
    pub name: String,
    pub variants: BTreeMap<String, ThemeVariant>,
    pub default_light: String,
    pub default_dark: String,
}

impl ThemeFamily {
    /// Validate variants and system-mode defaults.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] when variants disagree on family, map keys do not
    /// match variant names, or defaults are absent/wrong-mode.
    pub fn validate(&self) -> Result<(), ThemeError> {
        if self.name.trim().is_empty() {
            return Err(ThemeError::EmptyName);
        }
        for (key, variant) in &self.variants {
            variant.validate()?;
            if variant.family != self.name || &variant.name != key {
                return Err(ThemeError::VariantIdentity {
                    family: self.name.clone(),
                    key: key.clone(),
                });
            }
        }
        self.require_default(&self.default_light, ThemeMode::Light)?;
        self.require_default(&self.default_dark, ThemeMode::Dark)?;
        Ok(())
    }

    fn require_default(&self, name: &str, mode: ThemeMode) -> Result<(), ThemeError> {
        let variant = self
            .variants
            .get(name)
            .ok_or_else(|| ThemeError::MissingDefault {
                family: self.name.clone(),
                variant: name.to_owned(),
            })?;
        let family_supports_mode = self
            .variants
            .values()
            .any(|candidate| candidate.mode == mode);
        if variant.mode == mode || !family_supports_mode {
            Ok(())
        } else {
            Err(ThemeError::WrongDefaultMode {
                family: self.name.clone(),
                variant: name.to_owned(),
                expected: mode,
            })
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ThemeSelection {
    pub family: String,
    pub variant: String,
}

impl ThemeSelection {
    #[must_use]
    pub fn new(family: impl Into<String>, variant: impl Into<String>) -> Self {
        Self {
            family: family.into(),
            variant: variant.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "preference", rename_all = "snake_case")]
pub enum ThemePreference {
    Fixed { selection: ThemeSelection },
    System { family: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemAppearance {
    Light,
    Dark,
}

#[derive(Clone, Debug)]
pub struct ThemeManager {
    families: BTreeMap<String, ThemeFamily>,
    app: ThemePreference,
    windows: BTreeMap<String, ThemePreference>,
    scopes: BTreeMap<ComponentInstancePath, ThemePreference>,
    generation: u64,
}

impl ThemeManager {
    /// Build families from independent variants and select one fixed variant.
    /// Families that only provide one color mode use their first variant as the
    /// fallback for the unavailable system appearance.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] for duplicates, invalid variants, or an unknown
    /// initial selection.
    pub fn from_variants(
        variants: impl IntoIterator<Item = ThemeVariant>,
        selection: ThemeSelection,
    ) -> Result<Self, ThemeError> {
        let mut grouped = BTreeMap::<String, BTreeMap<String, ThemeVariant>>::new();
        for variant in variants {
            variant.validate()?;
            let family = grouped.entry(variant.family.clone()).or_default();
            if family
                .insert(variant.name.clone(), variant.clone())
                .is_some()
            {
                return Err(ThemeError::DuplicateVariant {
                    family: variant.family,
                    variant: variant.name,
                });
            }
        }
        let families = grouped
            .into_iter()
            .map(|(name, variants)| {
                let fallback = variants
                    .keys()
                    .next()
                    .cloned()
                    .ok_or(ThemeError::NoVariants)?;
                let default_light = variants
                    .values()
                    .find(|variant| variant.mode == ThemeMode::Light)
                    .map_or_else(|| fallback.clone(), |variant| variant.name.clone());
                let default_dark = variants
                    .values()
                    .find(|variant| variant.mode == ThemeMode::Dark)
                    .map(|variant| variant.name.clone())
                    .unwrap_or(fallback);
                Ok(ThemeFamily {
                    name,
                    variants,
                    default_light,
                    default_dark,
                })
            })
            .collect::<Result<Vec<_>, ThemeError>>()?;
        Self::new(families, ThemePreference::Fixed { selection })
    }

    /// Create a manager with a validated app preference.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] if the preference cannot resolve in the supplied
    /// family set.
    pub fn new(
        families: impl IntoIterator<Item = ThemeFamily>,
        app: ThemePreference,
    ) -> Result<Self, ThemeError> {
        let mut manager = Self {
            families: BTreeMap::new(),
            app,
            windows: BTreeMap::new(),
            scopes: BTreeMap::new(),
            generation: 1,
        };
        for family in families {
            manager.register_family(family)?;
        }
        manager.validate_preference(&manager.app)?;
        Ok(manager)
    }

    /// Add a validated theme family.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] for invalid or duplicate families.
    pub fn register_family(&mut self, family: ThemeFamily) -> Result<(), ThemeError> {
        family.validate()?;
        if self.families.contains_key(&family.name) {
            return Err(ThemeError::DuplicateFamily(family.name));
        }
        self.families.insert(family.name.clone(), family);
        Ok(())
    }

    /// Replace one already-registered variant and advance the theme generation.
    ///
    /// This is the host-facing path for live theme editors. Identity cannot be
    /// changed in place; callers create a new manager when families are added or
    /// removed.
    ///
    /// # Errors
    ///
    /// Returns validation or unknown-selection errors.
    pub fn replace_variant(&mut self, variant: ThemeVariant) -> Result<(), ThemeError> {
        variant.validate()?;
        let selection = ThemeSelection::new(variant.family.clone(), variant.name.clone());
        let family = self
            .families
            .get_mut(&variant.family)
            .ok_or_else(|| ThemeError::UnknownSelection(selection.clone()))?;
        if !family.variants.contains_key(&variant.name) {
            return Err(ThemeError::UnknownSelection(selection));
        }
        family.variants.insert(variant.name.clone(), variant);
        let fallback = family
            .variants
            .keys()
            .next()
            .cloned()
            .ok_or(ThemeError::NoVariants)?;
        family.default_light = family
            .variants
            .values()
            .find(|candidate| candidate.mode == ThemeMode::Light)
            .map_or_else(|| fallback.clone(), |candidate| candidate.name.clone());
        family.default_dark = family
            .variants
            .values()
            .find(|candidate| candidate.mode == ThemeMode::Dark)
            .map_or(fallback, |candidate| candidate.name.clone());
        family.validate()?;
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }

    /// Change the application fallback preference.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] when the selection cannot resolve.
    pub fn set_app(&mut self, preference: ThemePreference) -> Result<(), ThemeError> {
        self.validate_preference(&preference)?;
        if self.app != preference {
            self.app = preference;
            self.generation = self.generation.saturating_add(1);
        }
        Ok(())
    }

    /// Set a per-window theme preference.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] when the preference cannot resolve.
    pub fn set_window(
        &mut self,
        window: impl Into<String>,
        preference: ThemePreference,
    ) -> Result<(), ThemeError> {
        self.validate_preference(&preference)?;
        self.windows.insert(window.into(), preference);
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

    /// Set a local theme preference for a component subtree.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] when the preference cannot resolve.
    pub fn set_scope(
        &mut self,
        scope: ComponentInstancePath,
        preference: ThemePreference,
    ) -> Result<(), ThemeError> {
        self.validate_preference(&preference)?;
        self.scopes.insert(scope, preference);
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }

    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Every registered variant.
    pub fn variants(&self) -> impl Iterator<Item = &ThemeVariant> {
        self.families
            .values()
            .flat_map(|family| family.variants.values())
    }

    #[must_use]
    pub fn app_preference(&self) -> &ThemePreference {
        &self.app
    }

    /// Resolve the nearest active theme without changing scripts or UI state.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] if an internally stored preference no longer
    /// resolves after malformed deserialization.
    pub fn resolve(
        &self,
        window: Option<&str>,
        component: Option<&ComponentInstancePath>,
        system: SystemAppearance,
    ) -> Result<ResolvedTheme<'_>, ThemeError> {
        let preference = component
            .and_then(|component| self.nearest_scope(component))
            .or_else(|| window.and_then(|window| self.windows.get(window)))
            .unwrap_or(&self.app);
        let selection = self.selection_for(preference, system)?;
        let variant = self
            .families
            .get(&selection.family)
            .and_then(|family| family.variants.get(&selection.variant))
            .ok_or_else(|| ThemeError::UnknownSelection(selection.clone()))?;
        Ok(ResolvedTheme { variant })
    }

    fn nearest_scope(&self, component: &ComponentInstancePath) -> Option<&ThemePreference> {
        let mut current = Some(component.clone());
        while let Some(path) = current {
            if let Some(preference) = self.scopes.get(&path) {
                return Some(preference);
            }
            current = path.parent();
        }
        None
    }

    fn validate_preference(&self, preference: &ThemePreference) -> Result<(), ThemeError> {
        self.selection_for(preference, SystemAppearance::Light)?;
        self.selection_for(preference, SystemAppearance::Dark)?;
        Ok(())
    }

    fn selection_for(
        &self,
        preference: &ThemePreference,
        system: SystemAppearance,
    ) -> Result<ThemeSelection, ThemeError> {
        match preference {
            ThemePreference::Fixed { selection } => {
                let exists = self
                    .families
                    .get(&selection.family)
                    .is_some_and(|family| family.variants.contains_key(&selection.variant));
                if exists {
                    Ok(selection.clone())
                } else {
                    Err(ThemeError::UnknownSelection(selection.clone()))
                }
            }
            ThemePreference::System { family } => {
                let family = self
                    .families
                    .get(family)
                    .ok_or_else(|| ThemeError::UnknownFamily(family.clone()))?;
                Ok(ThemeSelection::new(
                    family.name.clone(),
                    match system {
                        SystemAppearance::Light => family.default_light.clone(),
                        SystemAppearance::Dark => family.default_dark.clone(),
                    },
                ))
            }
        }
    }
}

pub struct ResolvedTheme<'a> {
    variant: &'a ThemeVariant,
}

impl ResolvedTheme<'_> {
    #[must_use]
    pub fn variant(&self) -> &ThemeVariant {
        self.variant
    }
}

impl ColorResolver for ResolvedTheme<'_> {
    fn resolve_token(&self, token: &str) -> Option<Rgba8> {
        self.variant.resolve_token(token)
    }

    fn resolve_length_in(&self, length: Length, environment: &Environment) -> Option<Length> {
        self.variant.resolve_length_in(length, environment)
    }

    fn resolve_typography_in(
        &self,
        role: &str,
        environment: &Environment,
    ) -> Option<ResolvedTypography> {
        self.variant.resolve_typography_in(role, environment)
    }

    fn token_set(&self) -> Option<Arc<ThemeTokens>> {
        self.variant.token_set()
    }

    fn color_snapshot(&self) -> BTreeMap<String, Rgba8> {
        self.variant.color_snapshot()
    }

    fn resolve_motion(&self) -> ThemeMotion {
        self.variant.tokens.motion.clone()
    }
}

/// Compile and evaluate a Rhai theme source exporting `theme() -> map`,
/// without a token base or Host overrides.
///
/// # Errors
///
/// Returns [`ThemeError`] for compilation, evaluation, decoding, color
/// expression or token validation failures.
pub fn load_theme_source(
    engine: &Engine,
    source_name: &str,
    source: &str,
) -> Result<ThemeVariant, ThemeError> {
    load_theme_with_layers(
        engine,
        None,
        source_name,
        source,
        &ThemeTokenOverrides::default(),
    )
}

/// Compile and evaluate a token base source exporting `tokens() -> map`.
///
/// The base is the lowest token layer: palette themes and Host overrides
/// replace individual entries of it.
///
/// # Errors
///
/// Returns [`ThemeError`] for compilation, evaluation or decoding failures.
pub fn load_token_base(
    engine: &Engine,
    source_name: &str,
    source: &str,
) -> Result<TokenLayer, ThemeError> {
    crate::theme_source::decode_token_base(engine, source_name, source)
}

/// Load a palette theme on top of an optional token base and Host overrides.
///
/// # Errors
///
/// Returns [`ThemeError`] for any decoding, merge or validation failure.
pub fn load_theme_with_layers(
    engine: &Engine,
    base: Option<&TokenLayer>,
    source_name: &str,
    source: &str,
    overrides: &ThemeTokenOverrides,
) -> Result<ThemeVariant, ThemeError> {
    let theme = crate::theme_source::decode_theme(engine, source_name, source)?;
    let mut layer = base.cloned().unwrap_or_default();
    layer.merge(theme.tokens);
    layer.merge(overrides.clone().into_layer());
    let variant = ThemeVariant {
        family: theme.family,
        name: theme.name,
        mode: theme.mode,
        tokens: Arc::new(layer.finalize()?),
    };
    variant.validate()?;
    Ok(variant)
}

/// Relative luminance contrast ratio between two opaque colors.
#[must_use]
pub fn contrast_ratio(left: Rgba8, right: Rgba8) -> f64 {
    left.contrast_ratio(right)
}

#[derive(Debug, Error)]
pub enum ThemeError {
    #[error("theme family and variant names cannot be empty")]
    EmptyName,
    #[error("missing {category} tokens: {missing:?}")]
    MissingTokens {
        category: &'static str,
        missing: Vec<String>,
    },
    #[error("length token `{token}` is invalid: {source}")]
    InvalidLength {
        token: String,
        source: crate::LengthError,
    },
    #[error("theme length token `{0}` cannot reference another theme token")]
    NestedLengthToken(String),
    #[error("theme token namespace `{0}` must be snake_case")]
    InvalidNamespace(String),
    #[error("theme token `{namespace}.{name}` must use a snake_case name")]
    InvalidTokenName { namespace: String, name: String },
    #[error("theme number token `{namespace}.{name}` must be finite")]
    NonFiniteNumber { namespace: String, name: String },
    #[error("theme typography family must be a non-empty name no longer than 256 bytes")]
    InvalidTypographyFamily,
    #[error("theme typography fallbacks must contain unique non-empty family names")]
    InvalidTypographyFallbacks,
    #[error("theme typography role `{0}` must use a snake_case name")]
    InvalidTypographyRole(String),
    #[error("theme typography role `{role}` aliases unknown role `{target}`")]
    UnknownTypographyAlias { role: String, target: String },
    #[error("theme typography role `{0}` has an alias cycle or is nested too deeply")]
    TypographyAliasCycle(String),
    #[error("theme token `{token}` depends on undeclared environment value `{name}`")]
    UndeclaredEnvironment { token: String, name: String },
    #[error(
        "theme token `{token}` uses `{value}`, which environment value `{name}` does not declare"
    )]
    UndeclaredEnvironmentValue {
        token: String,
        name: String,
        value: String,
    },
    #[error("theme color expressions form a cycle: {0}")]
    ColorCycle(String),
    #[error("theme color `{token}` could not be resolved from {expression}")]
    UnresolvedColor { token: String, expression: String },
    #[error("theme `{theme}` lacks tokens required by {consumer}: {missing:?}")]
    MissingRequiredTokens {
        theme: String,
        consumer: String,
        missing: Vec<String>,
    },
    #[error("theme token `{token}` is invalid: {reason}")]
    InvalidTokenValue { token: String, reason: String },
    #[error("theme typography `{role}.{field}` must be a positive px or rem length")]
    InvalidTypographyLength { role: String, field: &'static str },
    #[error("theme typography `{0}` line height cannot be smaller than its font size")]
    InvalidTypographyLineHeight(String),
    #[error("theme typography `{role}` weight must be between 1 and 1000, got {weight}")]
    InvalidTypographyWeight { role: String, weight: u16 },
    #[error("theme motion tokens must use positive durations and physically valid values")]
    InvalidMotionTokens,
    #[error("variant `{key}` does not match family `{family}` or its map key")]
    VariantIdentity { family: String, key: String },
    #[error("family `{family}` has no default variant `{variant}`")]
    MissingDefault { family: String, variant: String },
    #[error("family `{family}` default `{variant}` has wrong mode; expected {expected:?}")]
    WrongDefaultMode {
        family: String,
        variant: String,
        expected: ThemeMode,
    },
    #[error("theme family `{0}` is already registered")]
    DuplicateFamily(String),
    #[error("theme family `{family}` already contains variant `{variant}`")]
    DuplicateVariant { family: String, variant: String },
    #[error("at least one theme variant is required")]
    NoVariants,
    #[error("theme family `{0}` is not registered")]
    UnknownFamily(String),
    #[error("theme selection `{0:?}` does not exist")]
    UnknownSelection(ThemeSelection),
    #[error("theme script failed: {0}")]
    Script(String),
    #[error("theme source could not be decoded: {0}")]
    Decode(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(accent: u32) -> ThemeTokens {
        ThemeTokens {
            colors: ["surface", "text_primary", "accent"]
                .iter()
                .map(|name| {
                    (
                        (*name).to_owned(),
                        Rgba8::from_rgb_hex(if *name == "accent" {
                            accent
                        } else {
                            0x0011_1111
                        }),
                    )
                })
                .collect(),
            ..ThemeTokens::default()
        }
    }

    fn variant(name: &str, mode: ThemeMode, accent: u32) -> ThemeVariant {
        ThemeVariant {
            family: "Default".to_owned(),
            name: name.to_owned(),
            mode,
            tokens: Arc::new(tokens(accent)),
        }
    }

    fn family() -> ThemeFamily {
        ThemeFamily {
            name: "Default".to_owned(),
            variants: BTreeMap::from([
                (
                    "Light".to_owned(),
                    variant("Light", ThemeMode::Light, 0x0033_66ff),
                ),
                (
                    "Dark".to_owned(),
                    variant("Dark", ThemeMode::Dark, 0x0066_99ff),
                ),
            ]),
            default_light: "Light".to_owned(),
            default_dark: "Dark".to_owned(),
        }
    }

    fn engine() -> crate::RuntimeEngine {
        crate::RuntimeEngine::new()
    }

    const BASE: &str = r#"
        fn tokens() {
            #{
                environment: #{
                    density: #{ values: ["comfortable", "compact"], "default": "comfortable" },
                    size: #{ values: ["sm", "md"], "default": "md" },
                },
                spacing: #{
                    sm: px(8),
                    lg: by_env("density", #{ comfortable: px(16), compact: px(12) }),
                },
                radius: #{ md: px(0) },
                metrics: #{
                    control: by_env(["density", "size"], #{
                        comfortable: #{ sm: px(28), md: px(32) },
                        compact: #{ sm: px(24), md: px(28) },
                    }),
                },
                typography: #{
                    roles: #{
                        body: #{ size: px(14), line_height: px(22), weight: 400 },
                        body_small: #{ size: px(13), line_height: px(20), weight: 400 },
                        label: #{ size: px(12), line_height: px(16), weight: 400,
                            family: "Menlo", fallbacks: ["DejaVu Sans Mono"] },
                        control: by_env("size", #{ sm: "body_small", md: "body" }),
                    },
                },
                colors: #{
                    "text.accent": readable(theme_color("accent"), theme_color("text_primary"),
                        theme_color("surface"), 4.5),
                    "table.selection": mix(theme_color("surface"), theme_color("accent"), 0.25),
                },
            }
        }
    "#;

    const PALETTE: &str = r#"
        fn theme() {
            #{
                family: "Paper", name: "Dark", mode: "dark",
                tokens: #{ colors: #{
                    surface: 0x151412ff, text_primary: 0xe8e4daff, accent: 0x3d5fe0ff,
                } },
            }
        }
    "#;

    fn layered() -> ThemeVariant {
        let engine = engine();
        let base = load_token_base(engine.engine(), "tokens.rhai", BASE).expect("base");
        load_theme_with_layers(
            engine.engine(),
            Some(&base),
            "theme.rhai",
            PALETTE,
            &ThemeTokenOverrides::default(),
        )
        .expect("layered theme")
    }

    fn environment(pairs: &[(&str, &str)]) -> Environment {
        pairs
            .iter()
            .fold(Environment::EMPTY, |environment, (name, value)| {
                environment
                    .with(Symbol::intern(name), Symbol::intern(value))
                    .expect("environment")
            })
    }

    #[test]
    fn scope_precedes_window_and_app_preferences() {
        let mut manager = ThemeManager::new(
            [family()],
            ThemePreference::System {
                family: "Default".to_owned(),
            },
        )
        .unwrap();
        manager
            .set_window(
                "main",
                ThemePreference::Fixed {
                    selection: ThemeSelection::new("Default", "Light"),
                },
            )
            .unwrap();
        let root = ComponentInstancePath::root("App", "root");
        manager
            .set_scope(
                root.clone(),
                ThemePreference::Fixed {
                    selection: ThemeSelection::new("Default", "Dark"),
                },
            )
            .unwrap();
        let child = root.child("Preview", "preview");
        assert_eq!(
            manager
                .resolve(Some("main"), Some(&child), SystemAppearance::Light)
                .unwrap()
                .variant()
                .name,
            "Dark"
        );
        assert_eq!(manager.variants().count(), 2);
    }

    #[test]
    fn switching_theme_only_advances_theme_generation() {
        let mut manager = ThemeManager::new(
            [family()],
            ThemePreference::System {
                family: "Default".to_owned(),
            },
        )
        .unwrap();
        let initial = manager.generation();
        manager
            .set_app(ThemePreference::Fixed {
                selection: ThemeSelection::new("Default", "Dark"),
            })
            .unwrap();
        assert_eq!(manager.generation(), initial + 1);
    }

    #[test]
    fn replacing_a_variant_preserves_identity_and_advances_generation() {
        let mut manager = ThemeManager::new(
            [family()],
            ThemePreference::Fixed {
                selection: ThemeSelection::new("Default", "Dark"),
            },
        )
        .unwrap();
        let before = manager.generation();
        manager
            .replace_variant(variant("Dark", ThemeMode::Dark, 0x00ff_00ff))
            .unwrap();
        assert_eq!(manager.generation(), before + 1);
        assert_eq!(
            manager
                .resolve(None, None, SystemAppearance::Dark)
                .unwrap()
                .variant()
                .tokens
                .colors["accent"],
            Rgba8::from_rgb_hex(0x00ff_00ff)
        );
        let mut missing = variant("Dark", ThemeMode::Dark, 0);
        missing.family = "Missing".to_owned();
        assert!(matches!(
            manager.replace_variant(missing),
            Err(ThemeError::UnknownSelection(_))
        ));
    }

    #[test]
    fn the_runtime_requires_no_vocabulary() {
        let engine = engine();
        let theme = load_theme_source(
            engine.engine(),
            "custom.rhai",
            r#"fn theme() { #{ family: "Tree", name: "Night", mode: "dark",
                tokens: #{ colors: #{ inset: 0x1a1b26ff, bright: 0xc0caf5ff } } } }"#,
        )
        .expect("a theme with its own vocabulary");
        assert_eq!(theme.tokens.colors.len(), 2);
        assert!(theme.tokens.typography.roles.is_empty());
        assert!(theme.require("app", ["inset", "bright"]).is_ok());
        let error = theme
            .require(
                "components/button",
                ["surface", "metrics.control", "typography.body"],
            )
            .expect_err("missing design-language tokens");
        assert!(
            matches!(error, ThemeError::MissingRequiredTokens { ref missing, .. } if missing.len() == 3),
            "{error}"
        );
    }

    #[test]
    fn layers_merge_and_color_expressions_follow_the_palette() {
        let theme = layered();
        let tokens = &theme.tokens;
        assert_eq!(tokens.colors["accent"], Rgba8::from_rgba_hex(0x3d5f_e0ff));
        let readable = tokens.color("text.accent").expect("derived");
        assert!(readable.contrast_ratio(tokens.colors["surface"]) >= 4.5);
        let selection = tokens.color("table.selection").expect("mixed");
        assert_eq!(
            selection,
            tokens.colors["surface"].mix(tokens.colors["accent"], 0.25)
        );
        assert!(
            theme
                .require("components/table", ["table.selection", "spacing.sm"])
                .is_ok()
        );
    }

    #[test]
    fn host_overrides_replace_tokens_and_environment_defaults() {
        let engine = engine();
        let base = load_token_base(engine.engine(), "tokens.rhai", BASE).unwrap();
        let theme = load_theme_with_layers(
            engine.engine(),
            Some(&base),
            "theme.rhai",
            PALETTE,
            &ThemeTokenOverrides {
                colors: BTreeMap::from([("accent".to_owned(), Rgba8::from_rgb_hex(0x0000_ff00))]),
                radii: BTreeMap::from([("md".to_owned(), Length::Pixels(4.0))]),
                environment_defaults: BTreeMap::from([(
                    "density".to_owned(),
                    "compact".to_owned(),
                )]),
                ..ThemeTokenOverrides::default()
            },
        )
        .unwrap();
        // Derived colors resolve after the override.
        assert_eq!(
            theme.tokens.color("table.selection"),
            Some(theme.tokens.colors["surface"].mix(Rgba8::from_rgb_hex(0x0000_ff00), 0.25))
        );
        assert_eq!(
            theme.resolve_length(Length::theme_radius("md").unwrap()),
            Some(Length::Pixels(4.0))
        );
        // The compact default applies when the subtree sets nothing.
        assert_eq!(
            theme.resolve_length(Length::token("metrics.control").unwrap()),
            Some(Length::Pixels(28.0))
        );
        let invalid = load_theme_with_layers(
            engine.engine(),
            Some(&base),
            "theme.rhai",
            PALETTE,
            &ThemeTokenOverrides {
                environment_defaults: BTreeMap::from([("density".to_owned(), "dense".to_owned())]),
                ..ThemeTokenOverrides::default()
            },
        );
        assert!(matches!(
            invalid,
            Err(ThemeError::UndeclaredEnvironmentValue { .. })
        ));
    }

    #[test]
    fn lengths_resolve_against_the_inherited_environment_and_scale() {
        let theme = layered();
        let control = Length::token("metrics.control").unwrap();
        assert_eq!(theme.resolve_length(control), Some(Length::Pixels(32.0)));
        assert_eq!(
            theme.resolve_length_in(
                control,
                &environment(&[("density", "compact"), ("size", "sm")])
            ),
            Some(Length::Pixels(24.0))
        );
        // An undeclared environment value falls back to the declared default.
        assert_eq!(
            theme.resolve_length_in(control, &environment(&[("size", "huge")])),
            Some(Length::Pixels(32.0))
        );
        assert_eq!(
            theme.resolve_length_in(
                Length::theme_spacing("lg").unwrap(),
                &environment(&[("density", "compact")])
            ),
            Some(Length::Pixels(12.0))
        );
        assert_eq!(
            theme.resolve_length(control.scaled(8.0).unwrap()),
            Some(Length::Pixels(256.0))
        );
        assert_eq!(
            theme.resolve_length(Length::token("metrics.missing").unwrap()),
            None
        );
    }

    #[test]
    fn typography_aliases_follow_the_environment_and_role_families() {
        let theme = layered();
        let control = theme
            .resolve_typography_in("control", &environment(&[("size", "sm")]))
            .expect("alias");
        assert_eq!(control.size, Length::Pixels(13.0));
        assert_eq!(
            theme.resolve_typography("control").map(|role| role.size),
            Some(Length::Pixels(14.0))
        );
        let label = theme.resolve_typography("label").expect("label role");
        assert_eq!(label.family.as_deref(), Some("Menlo"));
        assert_eq!(label.fallbacks, ["DejaVu Sans Mono"]);
        assert!(theme.resolve_typography("missing").is_none());
    }

    #[test]
    fn invalid_token_sources_are_rejected_with_paths() {
        let engine = engine();
        let decode = |source: &str| load_token_base(engine.engine(), "tokens.rhai", source);
        let cycle = load_theme_source(
            engine.engine(),
            "cycle.rhai",
            r#"fn theme() { #{ family: "F", name: "N", mode: "dark", tokens: #{ colors: #{
                a: theme_color("b"), b: mix(theme_color("a"), theme_color("a"), 0.5) } } } }"#,
        );
        assert!(matches!(cycle, Err(ThemeError::ColorCycle(_))), "{cycle:?}");
        let undeclared = engine.engine();
        let undeclared = load_theme_with_layers(
            undeclared,
            Some(
                &decode(
                    r#"fn tokens() { #{ metrics: #{ row: by_env("density",
                    #{ comfortable: px(32) }) } } }"#,
                )
                .unwrap(),
            ),
            "theme.rhai",
            PALETTE,
            &ThemeTokenOverrides::default(),
        );
        assert!(
            matches!(undeclared, Err(ThemeError::UndeclaredEnvironment { .. })),
            "{undeclared:?}"
        );
        let bare_number = decode(r"fn tokens() { #{ spacing: #{ sm: 8 } } }");
        assert!(
            matches!(bare_number, Err(ThemeError::InvalidTokenValue { ref token, .. }) if token == "spacing.sm"),
            "{bare_number:?}"
        );
        let unknown_alias = load_theme_with_layers(
            engine.engine(),
            Some(
                &decode(r#"fn tokens() { #{ typography: #{ roles: #{ control: "body" } } } }"#)
                    .unwrap(),
            ),
            "theme.rhai",
            PALETTE,
            &ThemeTokenOverrides::default(),
        );
        assert!(matches!(
            unknown_alias,
            Err(ThemeError::UnknownTypographyAlias { .. })
        ));
    }

    #[test]
    fn verbose_serialized_lengths_and_legacy_namespaces_remain_valid() {
        let engine = engine();
        let theme = load_theme_source(
            engine.engine(),
            "legacy.rhai",
            r#"fn theme() { #{ family: "F", name: "N", mode: "light", tokens: #{
                colors: #{ surface: 0xffffffff },
                spacing: #{ sm: #{ unit: "pixels", value: 8.0 } },
                radii: #{ sm: #{ unit: "pixels", value: 0.0 } },
                namespaces: #{ brand: #{ tint: #{ type: "color", value: 0xff0000ff },
                    stroke: #{ type: "length", value: #{ unit: "pixels", value: 2.0 } } } },
            } } }"#,
        )
        .expect("legacy shape");
        assert_eq!(
            theme.resolve_length(Length::theme_spacing("sm").unwrap()),
            Some(Length::Pixels(8.0))
        );
        assert_eq!(
            theme.tokens.color("brand.tint"),
            Some(Rgba8::from_rgba_hex(0xff00_00ff))
        );
        assert_eq!(
            theme.resolve_length(Length::token("brand.stroke").unwrap()),
            Some(Length::Pixels(2.0))
        );
    }

    #[test]
    fn semantic_motion_tokens_default_validate_and_reject_invalid_physics() {
        assert!(ThemeMotion::default().validate().is_ok());
        let mut motion = ThemeMotion::default();
        motion.springs.insert(
            "bouncy".to_owned(),
            ThemeMotionSpring {
                stiffness: 0.0,
                damping: 1.0,
                mass: 1.0,
            },
        );
        assert!(matches!(
            motion.validate(),
            Err(ThemeError::InvalidMotionTokens)
        ));
    }
}
