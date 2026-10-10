use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{DummyKeyboardMapper, KeyBinding, KeyBindingContextPredicate};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ScriptCallback, UiValue};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ActionId(String);

impl ActionId {
    /// Parse a namespaced semantic action identifier such as `document.save`.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError::InvalidId`] unless both segments are non-empty
    /// `snake_case` identifiers.
    pub fn parse(value: impl Into<String>) -> Result<Self, ActionError> {
        let value = value.into();
        if value
            .split_once('.')
            .is_some_and(|(namespace, action)| is_identifier(namespace) && is_identifier(action))
        {
            Ok(Self(value))
        } else {
            Err(ActionError::InvalidId(value))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('_')
        && !value.ends_with('_')
        && !value.contains("__")
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

#[derive(Clone, Debug)]
struct ActionEntry {
    callback: ScriptCallback,
    enabled: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ActionRegistry {
    actions: BTreeMap<ActionId, ActionEntry>,
}

impl ActionRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a semantic action and its Rhai callback.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError::Duplicate`] when the ID already exists.
    pub fn register(&mut self, id: ActionId, callback: ScriptCallback) -> Result<(), ActionError> {
        if self.actions.contains_key(&id) {
            return Err(ActionError::Duplicate(id));
        }
        self.actions.insert(
            id,
            ActionEntry {
                callback,
                enabled: true,
            },
        );
        Ok(())
    }

    /// Register a semantic action enabled, or replace the callback of a
    /// registered one; a replaced action keeps its enabled state, so running
    /// the registration again (a hot reload) does not re-enable it.
    pub fn register_or_replace(&mut self, id: ActionId, callback: ScriptCallback) {
        let enabled = self.actions.get(&id).is_none_or(|entry| entry.enabled);
        self.actions.insert(id, ActionEntry { callback, enabled });
    }

    pub fn remove_component_scope(&mut self, component: &crate::ComponentInstancePath) {
        self.actions.retain(|_, entry| {
            !entry
                .callback
                .component()
                .is_some_and(|path| path.is_within(component))
        });
    }

    /// Whether a registered action may dispatch; `None` when unregistered.
    #[must_use]
    pub fn is_enabled(&self, id: &ActionId) -> Option<bool> {
        self.actions.get(id).map(|entry| entry.enabled)
    }

    /// Registered action IDs in order.
    pub fn ids(&self) -> impl Iterator<Item = &ActionId> {
        self.actions.keys()
    }

    /// Change whether an action may dispatch.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError::Unknown`] when the ID is not registered.
    pub fn set_enabled(&mut self, id: &ActionId, enabled: bool) -> Result<(), ActionError> {
        self.actions
            .get_mut(id)
            .ok_or_else(|| ActionError::Unknown(id.clone()))?
            .enabled = enabled;
        Ok(())
    }

    /// Resolve a semantic action into a callback invocation.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError::Unknown`] or [`ActionError::Disabled`].
    pub fn dispatch(
        &self,
        id: &ActionId,
        payload: UiValue,
    ) -> Result<ActionInvocation, ActionError> {
        let entry = self
            .actions
            .get(id)
            .ok_or_else(|| ActionError::Unknown(id.clone()))?;
        if !entry.enabled {
            return Err(ActionError::Disabled(id.clone()));
        }
        Ok(ActionInvocation {
            id: id.clone(),
            callback: entry.callback.clone(),
            payload,
            origin: crate::InvocationOrigin::default(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct ActionInvocation {
    pub id: ActionId,
    pub callback: ScriptCallback,
    pub payload: UiValue,
    /// What the invocation that dispatched the action responds to; a queued
    /// action runs with it.
    pub origin: crate::InvocationOrigin,
}

#[derive(Clone, Debug, PartialEq, gpui::Action)]
#[action(namespace = gpui_rhai, no_json)]
pub struct DispatchScriptAction {
    pub id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KeyBindingSpec {
    pub keystrokes: String,
    pub action: ActionId,
    pub context: Option<String>,
}

impl KeyBindingSpec {
    /// Validate a key binding without panicking on malformed user input.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError::InvalidKeystroke`] or
    /// [`ActionError::InvalidContext`].
    pub fn new(
        keystrokes: impl Into<String>,
        action: ActionId,
        context: Option<String>,
    ) -> Result<Self, ActionError> {
        let keystrokes = keystrokes.into();
        if keystrokes.trim().is_empty() {
            return Err(ActionError::InvalidKeystroke(
                "key binding cannot be empty".to_owned(),
            ));
        }
        for keystroke in keystrokes.split_whitespace() {
            gpui::Keystroke::parse(keystroke)
                .map_err(|error| ActionError::InvalidKeystroke(error.to_string()))?;
        }
        if let Some(context) = &context {
            KeyBindingContextPredicate::parse(context)
                .map_err(|error| ActionError::InvalidContext(error.to_string()))?;
        }
        Ok(Self {
            keystrokes,
            action,
            context,
        })
    }

    /// The binding as a display label plus structured chords, formatted for
    /// the current platform (`⌃⌥⇧⌘K` on macOS, `Ctrl+Shift+K` elsewhere).
    #[must_use]
    pub fn shortcut(&self) -> ActionShortcut {
        ActionShortcut::from_keystrokes(&self.keystrokes, cfg!(target_os = "macos"))
    }

    /// Convert the validated specification into a GPUI key binding.
    ///
    /// # Errors
    ///
    /// Returns an [`ActionError`] if data was deserialized without passing
    /// through [`KeyBindingSpec::new`] and is invalid.
    pub fn to_gpui(&self) -> Result<KeyBinding, ActionError> {
        let context = self
            .context
            .as_deref()
            .map(KeyBindingContextPredicate::parse)
            .transpose()
            .map_err(|error| ActionError::InvalidContext(error.to_string()))?
            .map(Rc::new);
        KeyBinding::load(
            &self.keystrokes,
            Box::new(DispatchScriptAction {
                id: self.action.as_str().to_owned(),
            }),
            context,
            false,
            None,
            &DummyKeyboardMapper,
        )
        .map_err(|error| ActionError::InvalidKeystroke(error.to_string()))
    }
}

/// One key chord: a key and its modifiers in canonical order
/// (`ctrl`, `alt`, `shift`, `cmd`, `fn`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct KeyChord {
    pub key: String,
    pub modifiers: Vec<String>,
}

/// A displayable shortcut derived from a declared key binding.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ActionShortcut {
    /// Platform-formatted text, chords separated by a space.
    pub label: String,
    /// Structured chords, so custom UIs can draw their own key caps.
    pub chords: Vec<KeyChord>,
    /// The original binding text, such as `cmd-shift-k`.
    pub keystrokes: String,
}

impl ActionShortcut {
    fn from_keystrokes(keystrokes: &str, mac: bool) -> Self {
        let chords = keystrokes
            .split_whitespace()
            .filter_map(|chord| gpui::Keystroke::parse(chord).ok())
            .map(|keystroke| {
                let modifiers = keystroke.modifiers;
                let mut names = Vec::new();
                if modifiers.control {
                    names.push("ctrl".to_owned());
                }
                if modifiers.alt {
                    names.push("alt".to_owned());
                }
                if modifiers.shift {
                    names.push("shift".to_owned());
                }
                if modifiers.platform {
                    names.push("cmd".to_owned());
                }
                if modifiers.function {
                    names.push("fn".to_owned());
                }
                KeyChord {
                    key: keystroke.key,
                    modifiers: names,
                }
            })
            .collect::<Vec<_>>();
        let label = chords
            .iter()
            .map(|chord| format_chord(chord, mac))
            .collect::<Vec<_>>()
            .join(" ");
        Self {
            label,
            chords,
            keystrokes: keystrokes.to_owned(),
        }
    }

    /// A shortcut a caller wrote: GPUI keystrokes (`cmd-p`, `ctrl-shift-k`,
    /// `f6`) get the platform legend, like a bound action's shortcut; any
    /// other text (`⌘K`, `Ctrl+K`) is already a legend and is kept.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let keystrokes = !text.trim().is_empty()
            && text.split_whitespace().all(|chord| {
                let spelled = chord.contains('-') && !chord.ends_with('-')
                    || chord.chars().all(|character| {
                        character.is_ascii_lowercase() || character.is_ascii_digit()
                    });
                spelled && gpui::Keystroke::parse(chord).is_ok()
            });
        if keystrokes {
            Self::from_keystrokes(text, cfg!(target_os = "macos"))
        } else {
            Self {
                label: text.to_owned(),
                chords: Vec::new(),
                keystrokes: text.to_owned(),
            }
        }
    }

    /// The shortcut as a script value: `#{ label, keystrokes, chords }`.
    #[must_use]
    pub fn to_ui_value(&self) -> UiValue {
        UiValue::Map(BTreeMap::from([
            ("label".to_owned(), UiValue::String(self.label.clone())),
            (
                "keystrokes".to_owned(),
                UiValue::String(self.keystrokes.clone()),
            ),
            (
                "chords".to_owned(),
                UiValue::Array(
                    self.chords
                        .iter()
                        .map(|chord| {
                            UiValue::Map(BTreeMap::from([
                                ("key".to_owned(), UiValue::String(chord.key.clone())),
                                (
                                    "modifiers".to_owned(),
                                    UiValue::Array(
                                        chord
                                            .modifiers
                                            .iter()
                                            .cloned()
                                            .map(UiValue::String)
                                            .collect(),
                                    ),
                                ),
                            ]))
                        })
                        .collect(),
                ),
            ),
        ]))
    }
}

fn format_chord(chord: &KeyChord, mac: bool) -> String {
    let key = format_key(&chord.key, mac);
    if mac {
        let mut label = String::new();
        for modifier in &chord.modifiers {
            label.push_str(match modifier.as_str() {
                "ctrl" => "⌃",
                "alt" => "⌥",
                "shift" => "⇧",
                "cmd" => "⌘",
                _ => "fn ",
            });
        }
        label.push_str(&key);
        label
    } else {
        let mut parts = chord
            .modifiers
            .iter()
            .map(|modifier| match modifier.as_str() {
                "ctrl" => "Ctrl",
                "alt" => "Alt",
                "shift" => "Shift",
                "cmd" => "Super",
                _ => "Fn",
            })
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        parts.push(key);
        parts.join("+")
    }
}

fn format_key(key: &str, mac: bool) -> String {
    let named = match key {
        "enter" => Some(if mac { "↩" } else { "Enter" }),
        "escape" => Some(if mac { "⎋" } else { "Esc" }),
        "backspace" => Some(if mac { "⌫" } else { "Backspace" }),
        "delete" => Some(if mac { "⌦" } else { "Delete" }),
        "tab" => Some(if mac { "⇥" } else { "Tab" }),
        "space" => Some("Space"),
        "up" => Some("↑"),
        "down" => Some("↓"),
        "left" => Some("←"),
        "right" => Some("→"),
        "home" => Some("Home"),
        "end" => Some("End"),
        "pageup" => Some("PgUp"),
        "pagedown" => Some("PgDn"),
        _ => None,
    };
    named.map_or_else(
        || {
            let mut characters = key.chars();
            if let (Some(character), None) = (characters.next(), characters.next()) {
                character.to_uppercase().collect()
            } else {
                let mut text = key.to_owned();
                if let Some(first) = text.get_mut(0..1) {
                    first.make_ascii_uppercase();
                }
                text
            }
        },
        ToOwned::to_owned,
    )
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ActionError {
    #[error("action ID `{0}` must be `namespace.snake_case_action`")]
    InvalidId(String),
    #[error("action `{0:?}` is already registered")]
    Duplicate(ActionId),
    #[error("action `{0:?}` is not registered")]
    Unknown(ActionId),
    #[error("action `{0:?}` is disabled")]
    Disabled(ActionId),
    #[error("invalid key binding: {0}")]
    InvalidKeystroke(String),
    #[error("invalid key context: {0}")]
    InvalidContext(String),
}

#[cfg(test)]
mod shortcut_tests {
    use super::*;

    #[test]
    fn shortcuts_format_for_both_platforms_in_canonical_order() {
        let mac = ActionShortcut::from_keystrokes("cmd-shift-k", true);
        assert_eq!(mac.label, "⇧⌘K");
        assert_eq!(mac.chords[0].modifiers, ["shift", "cmd"]);
        let other = ActionShortcut::from_keystrokes("ctrl-alt-enter", false);
        assert_eq!(other.label, "Ctrl+Alt+Enter");
        let chord = ActionShortcut::from_keystrokes("cmd-k cmd-s", true);
        assert_eq!(chord.label, "⌘K ⌘S");
        assert_eq!(chord.chords.len(), 2);
        assert_eq!(ActionShortcut::from_keystrokes("f6", true).label, "F6");
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn caller_written_shortcuts_are_formatted_only_when_they_are_keystrokes() {
        for text in ["cmd-p", "ctrl-shift-k", "f6", "escape", "cmd-k cmd-s"] {
            let shortcut = ActionShortcut::from_text(text);
            assert!(!shortcut.chords.is_empty(), "{text} is keystrokes");
            assert_eq!(shortcut.keystrokes, text);
            assert_ne!(shortcut.label, text, "{text} gets a platform legend");
        }
        for legend in ["⌘K", "Ctrl+K", "⌘⇧P", ""] {
            let shortcut = ActionShortcut::from_text(legend);
            assert!(shortcut.chords.is_empty(), "{legend:?} is already a legend");
            assert_eq!(shortcut.label, legend);
        }
        assert_eq!(ActionShortcut::from_keystrokes("cmd-p", true).label, "⌘P");
        assert_eq!(
            ActionShortcut::from_keystrokes("cmd-p", false).label,
            "Super+P"
        );
    }
    use super::*;
    use crate::RuntimeEngine;

    fn callback() -> ScriptCallback {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn view() { text("action") }
                    fn save(payload) { payload }
                "#,
            )
            .unwrap();
        runtime.callback(&compiled, "save").unwrap()
    }

    #[test]
    fn disabled_actions_do_not_dispatch() {
        let id = ActionId::parse("document.save").unwrap();
        let mut registry = ActionRegistry::new();
        registry.register(id.clone(), callback()).unwrap();
        registry.set_enabled(&id, false).unwrap();
        assert!(matches!(
            registry.dispatch(&id, UiValue::Null),
            Err(ActionError::Disabled(_))
        ));
    }

    #[test]
    fn replacing_an_action_keeps_its_enabled_state() {
        let id = ActionId::parse("document.save").unwrap();
        let mut registry = ActionRegistry::new();
        registry.register_or_replace(id.clone(), callback());
        assert_eq!(registry.is_enabled(&id), Some(true));
        registry.set_enabled(&id, false).unwrap();
        registry.register_or_replace(id.clone(), callback());
        assert_eq!(registry.is_enabled(&id), Some(false));
        assert!(matches!(
            registry.dispatch(&id, UiValue::Null),
            Err(ActionError::Disabled(_))
        ));
        registry.set_enabled(&id, true).unwrap();
        registry.register_or_replace(id.clone(), callback());
        assert_eq!(registry.is_enabled(&id), Some(true));
    }

    #[test]
    fn key_bindings_validate_and_convert_to_gpui() {
        let binding = KeyBindingSpec::new(
            "cmd-s",
            ActionId::parse("document.save").unwrap(),
            Some("Editor && mode == full".to_owned()),
        )
        .unwrap();
        binding.to_gpui().unwrap();

        assert!(matches!(
            KeyBindingSpec::new("", ActionId::parse("document.save").unwrap(), None,),
            Err(ActionError::InvalidKeystroke(_))
        ));
    }
}
