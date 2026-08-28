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

    pub fn register_or_replace(&mut self, id: ActionId, callback: ScriptCallback) {
        self.actions.insert(
            id,
            ActionEntry {
                callback,
                enabled: true,
            },
        );
    }

    pub fn remove_component_scope(&mut self, component: &crate::ComponentInstancePath) {
        self.actions.retain(|_, entry| {
            !entry
                .callback
                .component()
                .is_some_and(|path| path.is_within(component))
        });
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
        })
    }
}

#[derive(Clone, Debug)]
pub struct ActionInvocation {
    pub id: ActionId,
    pub callback: ScriptCallback,
    pub payload: UiValue,
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
mod tests {
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
