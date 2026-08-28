use std::collections::{BTreeMap, VecDeque};

use thiserror::Error;

use crate::ScriptCallback;

const MAX_WINDOWS: usize = 16;
const MAX_PENDING_COMMANDS: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct ScriptWindowSpec {
    pub id: String,
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub focus: bool,
}

impl ScriptWindowSpec {
    /// Validate a script-visible window declaration.
    ///
    /// # Errors
    ///
    /// Returns [`WindowCommandError`] for unsafe identifiers, titles, or bounds.
    pub fn validate(&self) -> Result<(), WindowCommandError> {
        if !valid_window_id(&self.id) {
            return Err(WindowCommandError::InvalidId(self.id.clone()));
        }
        if self.title.trim().is_empty() || self.title.chars().count() > 256 {
            return Err(WindowCommandError::InvalidTitle);
        }
        if !valid_dimension(self.width) || !valid_dimension(self.height) {
            return Err(WindowCommandError::InvalidBounds {
                width: self.width,
                height: self.height,
            });
        }
        Ok(())
    }
}

fn valid_window_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

fn valid_dimension(value: f64) -> bool {
    value.is_finite() && (200.0..=4096.0).contains(&value)
}

#[derive(Clone, Debug, PartialEq)]
pub enum WindowCommand {
    Open(ScriptWindowSpec),
    Focus(String),
    Close(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WindowStatus {
    Pending,
    Open,
}

#[derive(Clone, Debug)]
struct WindowRecord {
    status: WindowStatus,
    close_handler: Option<ScriptCallback>,
}

#[derive(Clone, Debug, Default)]
pub struct WindowCommandRegistry {
    windows: BTreeMap<String, WindowRecord>,
    commands: VecDeque<WindowCommand>,
}

impl WindowCommandRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a window already created by the Rust host.
    ///
    /// # Errors
    ///
    /// Returns duplicate or invalid-ID errors.
    pub fn register_open(&mut self, id: impl Into<String>) -> Result<(), WindowCommandError> {
        let id = id.into();
        if !valid_window_id(&id) {
            return Err(WindowCommandError::InvalidId(id));
        }
        if self.windows.contains_key(&id) {
            return Err(WindowCommandError::Duplicate(id));
        }
        if self.windows.len() >= MAX_WINDOWS {
            return Err(WindowCommandError::WindowLimit(MAX_WINDOWS));
        }
        self.windows.insert(
            id,
            WindowRecord {
                status: WindowStatus::Open,
                close_handler: None,
            },
        );
        Ok(())
    }

    /// Validate and queue creation of another instance of the script entry.
    ///
    /// # Errors
    ///
    /// Returns duplicate, limit, queue, or spec validation errors.
    pub fn request_open(&mut self, spec: ScriptWindowSpec) -> Result<(), WindowCommandError> {
        spec.validate()?;
        if self.windows.contains_key(&spec.id) {
            return Err(WindowCommandError::Duplicate(spec.id));
        }
        if self.windows.len() >= MAX_WINDOWS {
            return Err(WindowCommandError::WindowLimit(MAX_WINDOWS));
        }
        self.require_queue_capacity()?;
        self.windows.insert(
            spec.id.clone(),
            WindowRecord {
                status: WindowStatus::Pending,
                close_handler: None,
            },
        );
        self.commands.push_back(WindowCommand::Open(spec));
        Ok(())
    }

    /// Mark successful native creation of a queued window.
    ///
    /// # Errors
    ///
    /// Returns unknown-window errors or an invalid transition.
    pub fn mark_open(&mut self, id: &str) -> Result<(), WindowCommandError> {
        let record = self
            .windows
            .get_mut(id)
            .ok_or_else(|| WindowCommandError::Unknown(id.to_owned()))?;
        if record.status != WindowStatus::Pending {
            return Err(WindowCommandError::AlreadyOpen(id.to_owned()));
        }
        record.status = WindowStatus::Open;
        Ok(())
    }

    pub fn fail_open(&mut self, id: &str) {
        if self
            .windows
            .get(id)
            .is_some_and(|record| record.status == WindowStatus::Pending)
        {
            self.windows.remove(id);
        }
    }

    /// Queue native focus for an existing window.
    ///
    /// # Errors
    ///
    /// Returns unknown-window or queue-limit errors.
    pub fn request_focus(&mut self, id: &str) -> Result<(), WindowCommandError> {
        self.require_open(id)?;
        self.require_queue_capacity()?;
        self.commands.push_back(WindowCommand::Focus(id.to_owned()));
        Ok(())
    }

    /// Queue forced close after script confirmation.
    ///
    /// # Errors
    ///
    /// Returns unknown-window or queue-limit errors.
    pub fn request_close(&mut self, id: &str) -> Result<(), WindowCommandError> {
        self.require_open(id)?;
        self.require_queue_capacity()?;
        self.commands.push_back(WindowCommand::Close(id.to_owned()));
        Ok(())
    }

    /// Install or clear the current generation's close-request callback.
    ///
    /// # Errors
    ///
    /// Returns [`WindowCommandError::Unknown`] for an absent window.
    pub fn set_close_handler(
        &mut self,
        id: &str,
        handler: Option<ScriptCallback>,
    ) -> Result<(), WindowCommandError> {
        let record = self
            .windows
            .get_mut(id)
            .ok_or_else(|| WindowCommandError::Unknown(id.to_owned()))?;
        record.close_handler = handler;
        Ok(())
    }

    #[must_use]
    pub fn close_handler(&self, id: &str) -> Option<ScriptCallback> {
        self.windows
            .get(id)
            .and_then(|record| record.close_handler.clone())
    }

    #[must_use]
    pub fn drain_commands(&mut self) -> Vec<WindowCommand> {
        self.commands.drain(..).collect()
    }

    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.windows.contains_key(id)
    }

    #[must_use]
    pub fn open_ids(&self) -> Vec<String> {
        self.windows
            .iter()
            .filter(|(_, record)| record.status == WindowStatus::Open)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn remove(&mut self, id: &str) -> bool {
        self.windows.remove(id).is_some()
    }

    fn require_open(&self, id: &str) -> Result<(), WindowCommandError> {
        match self.windows.get(id).map(|record| record.status) {
            Some(WindowStatus::Open) => Ok(()),
            Some(WindowStatus::Pending) => Err(WindowCommandError::NotOpen(id.to_owned())),
            None => Err(WindowCommandError::Unknown(id.to_owned())),
        }
    }

    fn require_queue_capacity(&self) -> Result<(), WindowCommandError> {
        if self.commands.len() < MAX_PENDING_COMMANDS {
            Ok(())
        } else {
            Err(WindowCommandError::QueueLimit(MAX_PENDING_COMMANDS))
        }
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum WindowCommandError {
    #[error("invalid window ID `{0}`")]
    InvalidId(String),
    #[error("window title must contain 1 to 256 characters")]
    InvalidTitle,
    #[error("window bounds must be finite and between 200 and 4096, got {width}x{height}")]
    InvalidBounds { width: f64, height: f64 },
    #[error("window `{0}` is already registered")]
    Duplicate(String),
    #[error("window `{0}` is not registered")]
    Unknown(String),
    #[error("window `{0}` is still pending native creation")]
    NotOpen(String),
    #[error("window `{0}` is already open")]
    AlreadyOpen(String),
    #[error("at most {0} script windows may be active")]
    WindowLimit(usize),
    #[error("at most {0} window commands may be pending")]
    QueueLimit(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str) -> ScriptWindowSpec {
        ScriptWindowSpec {
            id: id.to_owned(),
            title: "Settings".to_owned(),
            width: 640.0,
            height: 480.0,
            focus: true,
        }
    }

    #[test]
    fn commands_validate_identity_lifecycle_and_duplicates() {
        let mut windows = WindowCommandRegistry::new();
        windows.register_open("main").unwrap();
        windows.request_open(spec("settings")).unwrap();
        assert!(matches!(
            windows.request_open(spec("settings")),
            Err(WindowCommandError::Duplicate(_))
        ));
        assert!(matches!(
            windows.request_focus("settings"),
            Err(WindowCommandError::NotOpen(_))
        ));
        assert_eq!(
            windows.drain_commands(),
            vec![WindowCommand::Open(spec("settings"))]
        );
        windows.mark_open("settings").unwrap();
        windows.request_focus("settings").unwrap();
        windows.request_close("settings").unwrap();
        assert_eq!(
            windows.drain_commands(),
            vec![
                WindowCommand::Focus("settings".to_owned()),
                WindowCommand::Close("settings".to_owned()),
            ]
        );
    }

    #[test]
    fn invalid_window_specs_never_reserve_an_id() {
        let mut windows = WindowCommandRegistry::new();
        let mut invalid = spec("../escape");
        invalid.width = 20.0;
        assert!(windows.request_open(invalid).is_err());
        assert!(!windows.contains("../escape"));
    }
}
