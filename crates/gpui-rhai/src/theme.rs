use std::collections::BTreeMap;

use rhai::{Dynamic, Engine, Scope};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ColorResolver, ColorValue, ComponentInstancePath, Length, Rgba8};

const REQUIRED_COLORS: &[&str] = &[
    "surface",
    "surface_raised",
    "surface_hover",
    "text_primary",
    "text_muted",
    "accent",
    "accent_hover",
    "on_accent",
    "danger",
    "on_danger",
    "warning",
    "on_warning",
    "success",
    "on_success",
    "border",
    "focus_ring",
    "selection",
    "disabled",
];
const REQUIRED_SPACING: &[&str] = &["xs", "sm", "md", "lg"];
const REQUIRED_RADII: &[&str] = &["sm", "md", "lg"];
pub const REQUIRED_TYPOGRAPHY: &[&str] = &[
    "caption",
    "body_small",
    "body",
    "subtitle",
    "title",
    "heading",
    "display",
    "display_large",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThemeTokens {
    pub colors: BTreeMap<String, Rgba8>,
    pub spacing: BTreeMap<String, Length>,
    pub radii: BTreeMap<String, Length>,
    pub typography: ThemeTypography,
    #[serde(default)]
    pub namespaces: BTreeMap<String, BTreeMap<String, ThemeTokenValue>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThemeTypography {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallbacks: Vec<String>,
    pub roles: BTreeMap<String, TypographyToken>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TypographyToken {
    pub size: Length,
    pub line_height: Length,
    pub weight: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedTypography {
    pub family: Option<String>,
    pub fallbacks: Vec<String>,
    pub size: Length,
    pub line_height: Length,
    pub weight: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ThemeTokenValue {
    Color(Rgba8),
    Length(Length),
    Number(f64),
    String(String),
}

impl ThemeTokens {
    fn install_document_defaults(&mut self) {
        let color = |name: &str| self.colors.get(name).copied();
        let syntax = self.namespaces.entry("syntax".to_owned()).or_default();
        for (name, value) in [
            ("comment", color("text_muted")),
            ("string", color("success")),
            ("number", color("warning")),
            ("keyword", color("accent")),
            ("function", color("accent_hover")),
            ("type", color("warning")),
            ("variable", color("text_primary")),
            ("constant", color("danger")),
            ("operator", color("accent")),
            ("punctuation", color("text_muted")),
            ("tag", color("danger")),
            ("attribute", color("warning")),
        ] {
            if let Some(value) = value {
                syntax
                    .entry(name.to_owned())
                    .or_insert(ThemeTokenValue::Color(value));
            }
        }
        let document = self.namespaces.entry("document".to_owned()).or_default();
        if let Some(value) = color("warning") {
            document
                .entry("search_match".to_owned())
                .or_insert(ThemeTokenValue::Color(with_alpha(value, 0x55)));
            document
                .entry("search_current".to_owned())
                .or_insert(ThemeTokenValue::Color(with_alpha(value, 0xaa)));
        }
        let diff = self.namespaces.entry("diff".to_owned()).or_default();
        for (name, value) in [
            (
                "left_only",
                color("danger").map(|value| with_alpha(value, 0x24)),
            ),
            (
                "right_only",
                color("success").map(|value| with_alpha(value, 0x24)),
            ),
            (
                "modified",
                color("accent").map(|value| with_alpha(value, 0x18)),
            ),
            (
                "inline_left",
                color("danger").map(|value| with_alpha(value, 0x66)),
            ),
            (
                "inline_right",
                color("success").map(|value| with_alpha(value, 0x66)),
            ),
            ("gutter", color("surface_raised")),
            ("fold", color("surface_hover")),
        ] {
            if let Some(value) = value {
                diff.entry(name.to_owned())
                    .or_insert(ThemeTokenValue::Color(value));
            }
        }
    }

    /// Validate the initial semantic token contract.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] for missing tokens or invalid lengths.
    pub fn validate(&self) -> Result<(), ThemeError> {
        require_tokens("color", REQUIRED_COLORS, &self.colors)?;
        require_tokens("spacing", REQUIRED_SPACING, &self.spacing)?;
        require_tokens("radius", REQUIRED_RADII, &self.radii)?;
        self.typography.validate()?;
        for (name, value) in self.spacing.iter().chain(&self.radii) {
            if value.is_theme_token() {
                return Err(ThemeError::NestedLengthToken(name.clone()));
            }
            value
                .validate()
                .map_err(|source| ThemeError::InvalidLength {
                    token: name.clone(),
                    source,
                })?;
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
                        if length.is_theme_token() {
                            return Err(ThemeError::NestedNamespacedLength {
                                namespace: namespace.clone(),
                                name: name.clone(),
                            });
                        }
                        length
                            .validate()
                            .map_err(|source| ThemeError::InvalidLength {
                                token: format!("{namespace}.{name}"),
                                source,
                            })?;
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
        Ok(())
    }

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
}

impl ThemeTypography {
    /// Validate the shared family/fallback stack and every required type role.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] for missing roles or invalid font metrics.
    pub fn validate(&self) -> Result<(), ThemeError> {
        require_tokens("typography", REQUIRED_TYPOGRAPHY, &self.roles)?;
        if self
            .family
            .as_ref()
            .is_some_and(|family| !valid_font_family(family))
        {
            return Err(ThemeError::InvalidTypographyFamily);
        }
        let mut families = std::collections::BTreeSet::new();
        for family in &self.fallbacks {
            if !valid_font_family(family) || !families.insert(family) {
                return Err(ThemeError::InvalidTypographyFallbacks);
            }
        }
        if self
            .family
            .as_ref()
            .is_some_and(|family| families.contains(family))
        {
            return Err(ThemeError::InvalidTypographyFallbacks);
        }
        for (role, token) in &self.roles {
            if !REQUIRED_TYPOGRAPHY.contains(&role.as_str()) {
                return Err(ThemeError::UnknownTypographyRole(role.clone()));
            }
            validate_typography_length(role, "size", token.size)?;
            validate_typography_length(role, "line_height", token.line_height)?;
            if !(1..=1_000).contains(&token.weight) {
                return Err(ThemeError::InvalidTypographyWeight {
                    role: role.clone(),
                    weight: token.weight,
                });
            }
            match (token.size, token.line_height) {
                (Length::Pixels(size), Length::Pixels(line_height))
                | (Length::Rems(size), Length::Rems(line_height))
                    if line_height < size =>
                {
                    return Err(ThemeError::InvalidTypographyLineHeight(role.clone()));
                }
                _ => {}
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn resolve(&self, role: &str) -> Option<ResolvedTypography> {
        let token = self.roles.get(role)?;
        Some(ResolvedTypography {
            family: self.family.clone(),
            fallbacks: self.fallbacks.clone(),
            size: token.size,
            line_height: token.line_height,
            weight: token.weight,
        })
    }
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
        Length::Relative(_) | Length::ThemeSpacing(_) | Length::ThemeRadius(_) => false,
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

fn valid_token_segment(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('_')
        && !value.ends_with('_')
        && !value.contains("__")
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThemeVariant {
    pub family: String,
    pub name: String,
    pub mode: ThemeMode,
    pub tokens: ThemeTokens,
}

impl ThemeVariant {
    /// Validate identity and semantic tokens.
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

    #[must_use]
    pub fn typography(&self, role: &str) -> Option<ResolvedTypography> {
        self.tokens.typography.resolve(role)
    }
}

impl ColorResolver for ThemeVariant {
    fn resolve(&self, color: &ColorValue) -> Option<Rgba8> {
        match color {
            ColorValue::Literal(color) => Some(*color),
            ColorValue::Token(token) => self.tokens.color(token),
        }
    }

    fn resolve_length(&self, length: Length) -> Option<Length> {
        match length {
            Length::ThemeSpacing(token) => self.tokens.spacing.get(token.as_str()).copied(),
            Length::ThemeRadius(token) => self.tokens.radii.get(token.as_str()).copied(),
            Length::Pixels(_) | Length::Rems(_) | Length::Relative(_) => Some(length),
        }
    }

    fn resolve_typography(&self, role: &str) -> Option<ResolvedTypography> {
        self.typography(role)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
        for mut variant in variants {
            variant.tokens.install_document_defaults();
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
    pub fn register_family(&mut self, mut family: ThemeFamily) -> Result<(), ThemeError> {
        for variant in family.variants.values_mut() {
            variant.tokens.install_document_defaults();
        }
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
    pub fn replace_variant(&mut self, mut variant: ThemeVariant) -> Result<(), ThemeError> {
        variant.tokens.install_document_defaults();
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
    fn resolve(&self, color: &ColorValue) -> Option<Rgba8> {
        match color {
            ColorValue::Literal(color) => Some(*color),
            ColorValue::Token(token) => self.variant.tokens.color(token),
        }
    }

    fn resolve_length(&self, length: Length) -> Option<Length> {
        self.variant.resolve_length(length)
    }

    fn resolve_typography(&self, role: &str) -> Option<ResolvedTypography> {
        self.variant.typography(role)
    }
}

/// Compile and evaluate a Rhai theme source exporting `theme() -> map`.
///
/// # Errors
///
/// Returns [`ThemeError`] for compilation, evaluation, decoding, or semantic
/// token validation failures.
pub fn load_theme_source(
    engine: &Engine,
    source_name: &str,
    source: &str,
) -> Result<ThemeVariant, ThemeError> {
    let mut ast = engine
        .compile(source)
        .map_err(|error| ThemeError::Script(error.to_string()))?;
    ast.set_source(source_name);
    let raw: Dynamic = engine
        .call_fn(&mut Scope::new(), &ast, "theme", ())
        .map_err(|error| ThemeError::Script(error.to_string()))?;
    let mut theme = rhai::serde::from_dynamic::<ThemeVariant>(&raw)
        .map_err(|error| ThemeError::Decode(error.to_string()))?;
    theme.tokens.install_document_defaults();
    theme.validate()?;
    Ok(theme)
}

const fn with_alpha(color: Rgba8, alpha: u8) -> Rgba8 {
    Rgba8::from_rgba_hex((color.as_rgba_hex() & 0xffff_ff00) | alpha as u32)
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
    #[error("theme length token `{namespace}.{name}` cannot reference another theme token")]
    NestedNamespacedLength { namespace: String, name: String },
    #[error("theme number token `{namespace}.{name}` must be finite")]
    NonFiniteNumber { namespace: String, name: String },
    #[error("theme typography family must be a non-empty name no longer than 256 bytes")]
    InvalidTypographyFamily,
    #[error("theme typography fallbacks must contain unique non-empty family names")]
    InvalidTypographyFallbacks,
    #[error("unknown theme typography role `{0}`")]
    UnknownTypographyRole(String),
    #[error("theme typography `{role}.{field}` must be a positive px or rem length")]
    InvalidTypographyLength { role: String, field: &'static str },
    #[error("theme typography `{0}` line height cannot be smaller than its font size")]
    InvalidTypographyLineHeight(String),
    #[error("theme typography `{role}` weight must be between 1 and 1000, got {weight}")]
    InvalidTypographyWeight { role: String, weight: u16 },
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

    fn typography() -> ThemeTypography {
        ThemeTypography {
            family: None,
            fallbacks: Vec::new(),
            roles: BTreeMap::from([
                ("caption".to_owned(), type_token(10.0, 14.0, 400)),
                ("body_small".to_owned(), type_token(11.0, 14.0, 400)),
                ("body".to_owned(), type_token(12.0, 16.0, 400)),
                ("subtitle".to_owned(), type_token(13.0, 18.0, 400)),
                ("title".to_owned(), type_token(14.0, 20.0, 700)),
                ("heading".to_owned(), type_token(16.0, 22.0, 700)),
                ("display".to_owned(), type_token(24.0, 30.0, 700)),
                ("display_large".to_owned(), type_token(28.0, 34.0, 700)),
            ]),
        }
    }

    fn type_token(size: f64, line_height: f64, weight: u16) -> TypographyToken {
        TypographyToken {
            size: Length::Pixels(size),
            line_height: Length::Pixels(line_height),
            weight,
        }
    }

    fn tokens(accent: u32) -> ThemeTokens {
        ThemeTokens {
            colors: REQUIRED_COLORS
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
            spacing: BTreeMap::from([
                ("xs".to_owned(), Length::Pixels(4.0)),
                ("sm".to_owned(), Length::Pixels(8.0)),
                ("md".to_owned(), Length::Pixels(12.0)),
                ("lg".to_owned(), Length::Pixels(16.0)),
            ]),
            radii: BTreeMap::from([
                ("sm".to_owned(), Length::Pixels(4.0)),
                ("md".to_owned(), Length::Pixels(8.0)),
                ("lg".to_owned(), Length::Pixels(12.0)),
            ]),
            typography: typography(),
            namespaces: BTreeMap::new(),
        }
    }

    fn family() -> ThemeFamily {
        ThemeFamily {
            name: "Default".to_owned(),
            variants: BTreeMap::from([
                (
                    "Light".to_owned(),
                    ThemeVariant {
                        family: "Default".to_owned(),
                        name: "Light".to_owned(),
                        mode: ThemeMode::Light,
                        tokens: tokens(0x0033_66ff),
                    },
                ),
                (
                    "Dark".to_owned(),
                    ThemeVariant {
                        family: "Default".to_owned(),
                        name: "Dark".to_owned(),
                        mode: ThemeMode::Dark,
                        tokens: tokens(0x0066_99ff),
                    },
                ),
            ]),
            default_light: "Light".to_owned(),
            default_dark: "Dark".to_owned(),
        }
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
    }

    #[test]
    fn namespaced_typed_tokens_validate_and_resolve_colors() {
        let mut tokens = tokens(0x0033_66ff);
        tokens.namespaces.insert(
            "charts".to_owned(),
            BTreeMap::from([
                (
                    "series_a".to_owned(),
                    ThemeTokenValue::Color(Rgba8::from_rgb_hex(0x00ff_5500)),
                ),
                (
                    "stroke".to_owned(),
                    ThemeTokenValue::Length(Length::Pixels(2.0)),
                ),
                ("muted_alpha".to_owned(), ThemeTokenValue::Number(0.6)),
            ]),
        );
        tokens.validate().unwrap();
        assert_eq!(
            tokens.color("charts.series_a"),
            Some(Rgba8::from_rgb_hex(0x00ff_5500))
        );
        tokens
            .namespaces
            .get_mut("charts")
            .unwrap()
            .insert("bad_number".to_owned(), ThemeTokenValue::Number(f64::NAN));
        assert!(matches!(
            tokens.validate(),
            Err(ThemeError::NonFiniteNumber { .. })
        ));
    }

    #[test]
    fn document_palette_defaults_preserve_explicit_theme_tuning() {
        let explicit = Rgba8::from_rgb_hex(0x00ab_cdef);
        let mut tokens = tokens(0x0033_66ff);
        tokens.namespaces.insert(
            "syntax".to_owned(),
            BTreeMap::from([("keyword".to_owned(), ThemeTokenValue::Color(explicit))]),
        );
        let manager = ThemeManager::from_variants(
            [ThemeVariant {
                family: "Tuned".to_owned(),
                name: "Dark".to_owned(),
                mode: ThemeMode::Dark,
                tokens,
            }],
            ThemeSelection::new("Tuned", "Dark"),
        )
        .unwrap();
        let resolved = manager.resolve(None, None, SystemAppearance::Dark).unwrap();
        assert_eq!(
            resolved.variant().tokens.color("syntax.keyword"),
            Some(explicit)
        );
        assert!(resolved.variant().tokens.color("diff.left_only").is_some());
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
            .replace_variant(ThemeVariant {
                family: "Default".to_owned(),
                name: "Dark".to_owned(),
                mode: ThemeMode::Dark,
                tokens: tokens(0x00ff_00ff),
            })
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
        assert!(matches!(
            manager.replace_variant(ThemeVariant {
                family: "Missing".to_owned(),
                name: "Dark".to_owned(),
                mode: ThemeMode::Dark,
                tokens: tokens(0x0000_00ff),
            }),
            Err(ThemeError::UnknownSelection(_))
        ));
    }

    #[test]
    fn missing_semantic_token_is_rejected() {
        let mut tokens = tokens(0x0033_66ff);
        tokens.colors.remove("focus_ring");
        assert!(matches!(
            tokens.validate(),
            Err(ThemeError::MissingTokens {
                category: "color",
                ..
            })
        ));
    }

    #[test]
    fn typography_roles_are_required_validated_and_resolved() {
        let mut theme_tokens = tokens(0x0033_66ff);
        theme_tokens.typography.family = Some("JetBrains Mono".to_owned());
        theme_tokens.typography.fallbacks = vec!["PingFang SC".to_owned()];
        let body = theme_tokens.typography.resolve("body").unwrap();
        assert_eq!(body.family.as_deref(), Some("JetBrains Mono"));
        assert_eq!(body.fallbacks, ["PingFang SC"]);
        assert_eq!(body.size, Length::Pixels(12.0));
        assert_eq!(body.line_height, Length::Pixels(16.0));
        assert_eq!(body.weight, 400);

        theme_tokens.typography.roles.remove("caption");
        assert!(matches!(
            theme_tokens.validate(),
            Err(ThemeError::MissingTokens {
                category: "typography",
                ..
            })
        ));

        let mut invalid = tokens(0x0033_66ff);
        invalid
            .typography
            .roles
            .insert("body".to_owned(), type_token(16.0, 12.0, 400));
        assert!(matches!(
            invalid.validate(),
            Err(ThemeError::InvalidTypographyLineHeight(role)) if role == "body"
        ));

        let mut invalid = tokens(0x0033_66ff);
        invalid
            .typography
            .roles
            .insert("bodyish".to_owned(), type_token(12.0, 16.0, 400));
        assert!(matches!(
            invalid.validate(),
            Err(ThemeError::UnknownTypographyRole(role)) if role == "bodyish"
        ));
    }

    #[test]
    fn semantic_spacing_and_radius_lengths_resolve_without_recursion() {
        let mut theme_family = family();
        let variant = theme_family.variants.remove("Dark").unwrap();
        assert_eq!(
            variant.resolve_length(Length::ThemeSpacing(crate::SpacingToken::Sm)),
            Some(Length::Pixels(8.0))
        );
        assert_eq!(
            variant.resolve_length(Length::ThemeRadius(crate::RadiusToken::Md)),
            Some(Length::Pixels(8.0))
        );
        let mut invalid = variant.tokens;
        invalid.spacing.insert(
            "sm".to_owned(),
            Length::ThemeSpacing(crate::SpacingToken::Sm),
        );
        assert!(matches!(
            invalid.validate(),
            Err(ThemeError::NestedLengthToken(token)) if token == "sm"
        ));
    }

    #[test]
    fn rhai_theme_source_is_typed_and_validated() {
        let theme = ThemeVariant {
            family: "Default".to_owned(),
            name: "Dark".to_owned(),
            mode: ThemeMode::Dark,
            tokens: tokens(0x0066_99ff),
        };
        let dynamic = rhai::serde::to_dynamic(theme.clone()).unwrap();
        let mut scope = Scope::new();
        scope.push_dynamic("THEME", dynamic);
        let engine = Engine::new();
        let ast = engine
            .compile_with_scope(&scope, "fn theme() { THEME }")
            .unwrap();
        let raw: Dynamic = engine.call_fn(&mut scope, &ast, "theme", ()).unwrap();
        let decoded: ThemeVariant = rhai::serde::from_dynamic(&raw).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded, theme);
    }
}
