use std::collections::BTreeMap;

use rhai::{Dynamic, Engine, Scope};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ComponentInstancePath;

const REQUIRED_MESSAGES: &[&str] = &[
    "common.clear",
    "common.close",
    "common.loading",
    "common.no_results",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LocaleBundle {
    pub locale: String,
    pub direction: TextDirection,
    pub messages: BTreeMap<String, String>,
}

impl LocaleBundle {
    /// Validate locale identity and required internal messages.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError`] for empty locale IDs or missing messages.
    pub fn validate(&self) -> Result<(), LocaleError> {
        if self.locale.trim().is_empty() {
            return Err(LocaleError::EmptyLocale);
        }
        let missing = REQUIRED_MESSAGES
            .iter()
            .filter(|key| !self.messages.contains_key(**key))
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(LocaleError::MissingMessages {
                locale: self.locale.clone(),
                missing,
            })
        }
    }
}

#[derive(Clone, Debug)]
pub struct LocaleManager {
    bundles: BTreeMap<String, LocaleBundle>,
    fallback: String,
    app: String,
    windows: BTreeMap<String, String>,
    scopes: BTreeMap<ComponentInstancePath, String>,
    generation: u64,
}

impl LocaleManager {
    /// Create a locale manager from validated bundles.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError`] for invalid, duplicate, or unknown defaults.
    pub fn new(
        bundles: impl IntoIterator<Item = LocaleBundle>,
        fallback: impl Into<String>,
        app: impl Into<String>,
    ) -> Result<Self, LocaleError> {
        let mut map = BTreeMap::new();
        for bundle in bundles {
            bundle.validate()?;
            if map.insert(bundle.locale.clone(), bundle).is_some() {
                return Err(LocaleError::DuplicateLocale);
            }
        }
        let manager = Self {
            bundles: map,
            fallback: fallback.into(),
            app: app.into(),
            windows: BTreeMap::new(),
            scopes: BTreeMap::new(),
            generation: 1,
        };
        manager.require_locale(&manager.fallback)?;
        manager.require_locale(&manager.app)?;
        Ok(manager)
    }

    /// Change the app locale without touching component state.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError::UnknownLocale`] for absent bundles.
    pub fn set_app(&mut self, locale: impl Into<String>) -> Result<(), LocaleError> {
        let locale = locale.into();
        self.require_locale(&locale)?;
        if self.app != locale {
            self.app = locale;
            self.generation = self.generation.saturating_add(1);
        }
        Ok(())
    }

    /// Set a window locale override.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError::UnknownLocale`] for absent bundles.
    pub fn set_window(
        &mut self,
        window: impl Into<String>,
        locale: impl Into<String>,
    ) -> Result<(), LocaleError> {
        let locale = locale.into();
        self.require_locale(&locale)?;
        self.windows.insert(window.into(), locale);
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

    /// Set a component-subtree locale override.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError::UnknownLocale`] for absent bundles.
    pub fn set_scope(
        &mut self,
        scope: ComponentInstancePath,
        locale: impl Into<String>,
    ) -> Result<(), LocaleError> {
        let locale = locale.into();
        self.require_locale(&locale)?;
        self.scopes.insert(scope, locale);
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }

    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    #[must_use]
    pub fn app_locale(&self) -> &str {
        &self.app
    }

    /// Resolve a message using nearest scope and fallback bundle.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError::UnknownMessage`] when neither selected nor
    /// fallback bundle defines the key.
    pub fn text(
        &self,
        window: Option<&str>,
        component: Option<&ComponentInstancePath>,
        key: &str,
    ) -> Result<String, LocaleError> {
        let locale = component
            .and_then(|component| self.nearest_scope(component))
            .or_else(|| window.and_then(|window| self.windows.get(window)))
            .unwrap_or(&self.app);
        self.bundles
            .get(locale)
            .and_then(|bundle| bundle.messages.get(key))
            .or_else(|| {
                self.bundles
                    .get(&self.fallback)
                    .and_then(|bundle| bundle.messages.get(key))
            })
            .cloned()
            .ok_or_else(|| LocaleError::UnknownMessage(key.to_owned()))
    }

    /// Resolve selected text direction.
    ///
    /// # Errors
    ///
    /// Returns [`LocaleError`] if internal selection data is invalid.
    pub fn direction(
        &self,
        window: Option<&str>,
        component: Option<&ComponentInstancePath>,
    ) -> Result<TextDirection, LocaleError> {
        let locale = component
            .and_then(|component| self.nearest_scope(component))
            .or_else(|| window.and_then(|window| self.windows.get(window)))
            .unwrap_or(&self.app);
        self.bundles
            .get(locale)
            .map(|bundle| bundle.direction)
            .ok_or_else(|| LocaleError::UnknownLocale(locale.clone()))
    }

    fn nearest_scope(&self, component: &ComponentInstancePath) -> Option<&String> {
        let mut current = Some(component.clone());
        while let Some(path) = current {
            if let Some(locale) = self.scopes.get(&path) {
                return Some(locale);
            }
            current = path.parent();
        }
        None
    }

    fn require_locale(&self, locale: &str) -> Result<(), LocaleError> {
        if self.bundles.contains_key(locale) {
            Ok(())
        } else {
            Err(LocaleError::UnknownLocale(locale.to_owned()))
        }
    }
}

/// Compile and evaluate `locale() -> map` from Rhai source.
///
/// # Errors
///
/// Returns script, decode, or semantic validation errors.
pub fn load_locale_source(
    engine: &Engine,
    source_name: &str,
    source: &str,
) -> Result<LocaleBundle, LocaleError> {
    let mut ast = engine
        .compile(source)
        .map_err(|error| LocaleError::Script(error.to_string()))?;
    ast.set_source(source_name);
    let raw: Dynamic = engine
        .call_fn(&mut Scope::new(), &ast, "locale", ())
        .map_err(|error| LocaleError::Script(error.to_string()))?;
    let bundle = rhai::serde::from_dynamic::<LocaleBundle>(&raw)
        .map_err(|error| LocaleError::Decode(error.to_string()))?;
    bundle.validate()?;
    Ok(bundle)
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum LocaleError {
    #[error("locale ID cannot be empty")]
    EmptyLocale,
    #[error("locale bundle is duplicated")]
    DuplicateLocale,
    #[error("locale `{locale}` is missing messages: {missing:?}")]
    MissingMessages {
        locale: String,
        missing: Vec<String>,
    },
    #[error("locale `{0}` is not loaded")]
    UnknownLocale(String),
    #[error("locale message `{0}` is not defined")]
    UnknownMessage(String),
    #[error("locale script failed: {0}")]
    Script(String),
    #[error("locale source could not be decoded: {0}")]
    Decode(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(locale: &str, loading: &str, direction: TextDirection) -> LocaleBundle {
        LocaleBundle {
            locale: locale.to_owned(),
            direction,
            messages: BTreeMap::from([
                ("common.clear".to_owned(), "Clear".to_owned()),
                ("common.close".to_owned(), "Close".to_owned()),
                ("common.loading".to_owned(), loading.to_owned()),
                ("common.no_results".to_owned(), "No results".to_owned()),
            ]),
        }
    }

    #[test]
    fn scope_precedence_and_fallback_are_deterministic() {
        let mut locales = LocaleManager::new(
            [
                bundle("en", "Loading", TextDirection::LeftToRight),
                bundle("zh-CN", "加载中", TextDirection::LeftToRight),
            ],
            "en",
            "en",
        )
        .unwrap();
        locales.set_window("main", "zh-CN").unwrap();
        let root = ComponentInstancePath::root("App", "root");
        locales.set_scope(root.clone(), "en").unwrap();
        assert_eq!(
            locales
                .text(
                    Some("main"),
                    Some(&root.child("Button", "save")),
                    "common.loading",
                )
                .unwrap(),
            "Loading"
        );
    }

    #[test]
    fn live_locale_switch_advances_only_locale_generation() {
        let mut locales = LocaleManager::new(
            [
                bundle("en", "Loading", TextDirection::LeftToRight),
                bundle("zh-CN", "加载中", TextDirection::LeftToRight),
            ],
            "en",
            "en",
        )
        .unwrap();
        let generation = locales.generation();
        locales.set_app("zh-CN").unwrap();
        assert_eq!(locales.generation(), generation + 1);
    }

    #[test]
    fn rtl_direction_follows_scope_window_and_app_precedence() {
        let mut locales = LocaleManager::new(
            [
                bundle("en", "Loading", TextDirection::LeftToRight),
                bundle("ar", "جارٍ التحميل", TextDirection::RightToLeft),
            ],
            "en",
            "ar",
        )
        .unwrap();
        let root = ComponentInstancePath::root("App", "root");
        assert_eq!(
            locales.direction(Some("main"), Some(&root)).unwrap(),
            TextDirection::RightToLeft
        );
        locales.set_window("main", "en").unwrap();
        assert_eq!(
            locales.direction(Some("main"), Some(&root)).unwrap(),
            TextDirection::LeftToRight
        );
        locales.set_scope(root.clone(), "ar").unwrap();
        assert_eq!(
            locales
                .direction(Some("main"), Some(&root.child("Button", "save")))
                .unwrap(),
            TextDirection::RightToLeft
        );
    }
}
