//! Display and spoken names for individual keys, independent of key binding parsing.

use std::collections::BTreeMap;

use rhai::{Dynamic, Engine, EvalAltResult, FuncRegistration, ImmutableString, Position};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::UiValue;

/// The naming convention used to display a key. This never installs a binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyLegendPlatform {
    Mac,
    Other,
}

impl Default for KeyLegendPlatform {
    fn default() -> Self {
        if cfg!(target_os = "macos") {
            Self::Mac
        } else {
            Self::Other
        }
    }
}

impl KeyLegendPlatform {
    /// Parse `mac` or `other`; an omitted platform should use [`Self::default`].
    ///
    /// # Errors
    /// Returns [`KeyLegendError`] for any other value.
    pub fn parse(value: &str) -> Result<Self, KeyLegendError> {
        match value {
            "mac" => Ok(Self::Mac),
            "other" => Ok(Self::Other),
            _ => Err(KeyLegendError(value.to_owned())),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Mac => "mac",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("key legend platform must be `mac` or `other`, got `{0}`")]
pub struct KeyLegendError(String);

/// One visual key and its readable accessible name.
///
/// Unlike an [`crate::ActionShortcut`], a legend does not parse a chord or
/// reorder modifiers. Callers may compose keys in the order they intend.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KeyLegend {
    pub label: String,
    pub spoken: String,
    /// A single symbolic glyph that benefits from optical sizing beside letters.
    pub glyph: bool,
    pub platform: KeyLegendPlatform,
}

impl KeyLegend {
    /// Describe a named key or literal legend. Blank input stays blank.
    #[must_use]
    pub fn new(key: &str, platform: KeyLegendPlatform) -> Self {
        let key = key.trim();
        if let Some(named) = Self::named(key, platform) {
            return named;
        }
        let label = if key.chars().count() == 1 {
            key.to_uppercase()
        } else {
            key.to_owned()
        };
        let spoken = match key {
            "." => "Period",
            "," => "Comma",
            "/" => "Slash",
            "\\" => "Backslash",
            "?" => "Question mark",
            "[" => "Left bracket",
            "]" => "Right bracket",
            "-" => "Minus",
            "+" => "Plus",
            "=" => "Equals",
            ";" => "Semicolon",
            "'" => "Quote",
            "`" => "Backtick",
            _ => &label,
        }
        .to_owned();
        Self {
            label,
            spoken,
            glyph: false,
            platform,
        }
    }

    pub(crate) fn named(key: &str, platform: KeyLegendPlatform) -> Option<Self> {
        let id = key.trim().to_lowercase();
        let (mac, other, mac_spoken, other_spoken, symbol) = match id.as_str() {
            "mod" => ("⌘", "Ctrl", "Command", "Control", true),
            "cmd" | "command" | "meta" | "⌘" => ("⌘", "⌘", "Command", "Command", true),
            "ctrl" | "control" | "⌃" => ("⌃", "Ctrl", "Control", "Control", true),
            "alt" | "option" | "opt" | "⌥" => ("⌥", "Alt", "Option", "Alt", true),
            "shift" | "⇧" => ("⇧", "Shift", "Shift", "Shift", true),
            "enter" | "return" | "↵" | "↩" => ("↵", "Enter", "Return", "Enter", true),
            "esc" | "escape" | "⎋" => ("Esc", "Esc", "Escape", "Escape", false),
            "tab" | "⇥" => ("Tab", "Tab", "Tab", "Tab", false),
            "space" => ("Space", "Space", "Space", "Space", false),
            "backspace" | "⌫" => ("⌫", "Backspace", "Delete", "Backspace", true),
            "delete" | "del" | "⌦" => ("⌦", "Del", "Forward delete", "Delete", true),
            "up" | "arrowup" | "↑" => ("↑", "↑", "Up arrow", "Up arrow", true),
            "down" | "arrowdown" | "↓" => ("↓", "↓", "Down arrow", "Down arrow", true),
            "left" | "arrowleft" | "←" => ("←", "←", "Left arrow", "Left arrow", true),
            "right" | "arrowright" | "→" => ("→", "→", "Right arrow", "Right arrow", true),
            "pageup" | "pgup" => ("PgUp", "PgUp", "Page up", "Page up", false),
            "pagedown" | "pgdn" => ("PgDn", "PgDn", "Page down", "Page down", false),
            "home" => ("Home", "Home", "Home", "Home", false),
            "end" => ("End", "End", "End", "End", false),
            _ => return None,
        };
        let (label, spoken) = if platform == KeyLegendPlatform::Mac {
            (mac, mac_spoken)
        } else {
            (other, other_spoken)
        };
        Some(Self {
            label: label.to_owned(),
            spoken: spoken.to_owned(),
            glyph: symbol && label.chars().count() == 1,
            platform,
        })
    }

    /// The same value used by Rhai's `key_legend`: `#{ label, spoken, glyph, platform }`.
    #[must_use]
    pub fn to_ui_value(&self) -> UiValue {
        UiValue::Map(BTreeMap::from([
            ("label".to_owned(), UiValue::String(self.label.clone())),
            ("spoken".to_owned(), UiValue::String(self.spoken.clone())),
            ("glyph".to_owned(), UiValue::Bool(self.glyph)),
            (
                "platform".to_owned(),
                UiValue::String(self.platform.as_str().to_owned()),
            ),
        ]))
    }
}

pub(crate) fn register_key_legend_api(engine: &mut Engine) {
    FuncRegistration::new("key_legend")
        .in_global_namespace()
        .register_into_engine(engine, |key: ImmutableString| {
            KeyLegend::new(&key, KeyLegendPlatform::default())
                .to_ui_value()
                .into_dynamic()
        });
    FuncRegistration::new("key_legend")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |key: ImmutableString,
             platform: ImmutableString|
             -> Result<Dynamic, Box<EvalAltResult>> {
                let platform = KeyLegendPlatform::parse(&platform).map_err(|error| {
                    Box::new(EvalAltResult::ErrorRuntime(
                        error.to_string().into(),
                        Position::NONE,
                    ))
                })?;
                Ok(KeyLegend::new(&key, platform).to_ui_value().into_dynamic())
            },
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platforms_aliases_and_spoken_names_are_explicit() {
        for platform in [KeyLegendPlatform::Mac, KeyLegendPlatform::Other] {
            for (name, alias) in [
                ("command", "⌘"),
                ("control", "ctrl"),
                ("escape", "esc"),
                ("return", "enter"),
                ("arrowup", "up"),
                ("pgdn", "pagedown"),
            ] {
                assert_eq!(
                    KeyLegend::new(name, platform),
                    KeyLegend::new(alias, platform)
                );
            }
        }
        assert_eq!(KeyLegend::new("mod", KeyLegendPlatform::Mac).label, "⌘");
        assert_eq!(
            KeyLegend::new("mod", KeyLegendPlatform::Other).spoken,
            "Control"
        );
        assert!(KeyLegend::new("shift", KeyLegendPlatform::Mac).glyph);
        assert!(!KeyLegend::new("shift", KeyLegendPlatform::Other).glyph);
        assert_eq!(
            KeyLegend::new("delete", KeyLegendPlatform::Mac).spoken,
            "Forward delete"
        );
    }

    #[test]
    fn literals_punctuation_and_empty_keys_do_not_parse_as_bindings() {
        let platform = KeyLegendPlatform::Other;
        for (raw, display, spoken) in [
            (" k ", "K", "K"),
            ("?", "?", "Question mark"),
            ("custom key", "custom key", "custom key"),
            ("cmd-k", "cmd-k", "cmd-k"),
            ("   ", "", ""),
        ] {
            let item = KeyLegend::new(raw, platform);
            assert_eq!(
                (item.label.as_str(), item.spoken.as_str()),
                (display, spoken)
            );
        }
        assert!(KeyLegendPlatform::parse("linux").is_err());
    }

    #[test]
    fn rhai_overloads_agree_with_the_native_value_and_reject_invalid_platforms() {
        let runtime = crate::RuntimeEngine::new();
        let api = crate::script_docs::ScriptApi::from_engine(runtime.engine()).unwrap();
        let functions: Vec<_> = api
            .functions
            .iter()
            .filter(|function| function.name == "key_legend")
            .collect();
        assert!(
            functions.iter().all(|function| function.doc.is_some()),
            "{functions:?}"
        );
        for platform in [KeyLegendPlatform::Mac, KeyLegendPlatform::Other] {
            let value = runtime
                .engine()
                .eval::<Dynamic>(&format!("key_legend(\"mod\", \"{}\")", platform.as_str()))
                .unwrap();
            assert_eq!(
                UiValue::from_dynamic(value).unwrap(),
                KeyLegend::new("mod", platform).to_ui_value()
            );
        }
        assert!(
            runtime
                .engine()
                .eval::<Dynamic>("key_legend(\"mod\", \"linux\")")
                .unwrap_err()
                .to_string()
                .contains("mac` or `other")
        );
    }
}
