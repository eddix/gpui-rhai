use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::{Rc, Weak};

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

impl WindowCommand {
    fn target(&self) -> &str {
        match self {
            Self::Open(spec) => &spec.id,
            Self::Focus(id) | Self::Close(id) => id,
        }
    }
}

/// A command and the qualifications captured when it was enqueued.
///
/// A native pump must not replace this origin with its own authority. Rust
/// request methods without a source explicitly use trusted Host origin.
#[derive(Clone, Debug)]
pub struct QueuedWindowCommand {
    command: WindowCommand,
    origin: WindowCommandOrigin,
    target: Rc<WindowIdentity>,
}

impl QueuedWindowCommand {
    #[must_use]
    pub const fn command(&self) -> &WindowCommand {
        &self.command
    }

    pub(crate) fn source_binding(&self) -> Option<NativeWindowBinding> {
        match &self.origin {
            WindowCommandOrigin::Host => None,
            WindowCommandOrigin::View { identity, .. } => identity.native.borrow().clone(),
        }
    }

    pub(crate) fn is_host_origin(&self) -> bool {
        matches!(self.origin, WindowCommandOrigin::Host)
    }

    pub(crate) fn target_binding(&self) -> Option<NativeWindowBinding> {
        self.target.native.borrow().clone()
    }
}

#[derive(Clone, Debug)]
enum WindowCommandOrigin {
    Host,
    View {
        id: String,
        identity: Rc<WindowIdentity>,
    },
}

#[derive(Debug, Default)]
struct WindowIdentity {
    mount: RefCell<Option<Weak<()>>>,
    native: RefCell<Option<NativeWindowBinding>>,
}

#[derive(Clone, Debug)]
pub(crate) struct NativeWindowBinding {
    pub(crate) window: gpui::WindowId,
    pub(crate) lease: Weak<()>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowCommandPolicy {
    ApplicationOwned,
    Disabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WindowStatus {
    Pending,
    Open,
}

#[derive(Clone, Debug)]
struct WindowRecord {
    status: WindowStatus,
    policy: WindowCommandPolicy,
    view_id: String,
    close_handler: Option<ScriptCallback>,
    identity: Rc<WindowIdentity>,
}

#[derive(Clone, Debug, Default)]
pub struct WindowCommandRegistry {
    windows: BTreeMap<String, WindowRecord>,
    commands: VecDeque<QueuedWindowCommand>,
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
        self.register_open_with_policy(id, WindowCommandPolicy::ApplicationOwned)
    }

    /// Register an existing host window with an explicit script command policy.
    ///
    /// # Errors
    ///
    /// Returns duplicate, limit, or invalid-ID errors.
    pub fn register_open_with_policy(
        &mut self,
        id: impl Into<String>,
        policy: WindowCommandPolicy,
    ) -> Result<(), WindowCommandError> {
        let id = id.into();
        self.register_open_for_view(id.clone(), policy, id)
    }

    /// Register an existing host window and its owning mounted view.
    ///
    /// # Errors
    ///
    /// Returns duplicate, limit, or invalid-ID errors.
    pub fn register_open_for_view(
        &mut self,
        id: impl Into<String>,
        policy: WindowCommandPolicy,
        view_id: impl Into<String>,
    ) -> Result<(), WindowCommandError> {
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
                policy,
                view_id: view_id.into(),
                close_handler: None,
                identity: Rc::new(WindowIdentity::default()),
            },
        );
        Ok(())
    }

    /// Validate and queue creation of another instance of the script entry.
    ///
    /// # Errors
    ///
    /// Returns duplicate, limit, queue, or spec validation errors. This Rust
    /// entry explicitly uses trusted Host origin; scripts use `request_open_from`.
    pub fn request_open(&mut self, spec: ScriptWindowSpec) -> Result<(), WindowCommandError> {
        self.enqueue_open(spec, WindowCommandOrigin::Host)
    }

    fn enqueue_open(
        &mut self,
        spec: ScriptWindowSpec,
        origin: WindowCommandOrigin,
    ) -> Result<(), WindowCommandError> {
        spec.validate()?;
        if self.windows.contains_key(&spec.id) {
            return Err(WindowCommandError::Duplicate(spec.id));
        }
        if self.windows.len() >= MAX_WINDOWS {
            return Err(WindowCommandError::WindowLimit(MAX_WINDOWS));
        }
        self.require_queue_capacity()?;
        let identity = Rc::new(WindowIdentity::default());
        self.windows.insert(
            spec.id.clone(),
            WindowRecord {
                status: WindowStatus::Pending,
                policy: WindowCommandPolicy::ApplicationOwned,
                view_id: spec.id.clone(),
                close_handler: None,
                identity: Rc::clone(&identity),
            },
        );
        self.commands.push_back(QueuedWindowCommand {
            command: WindowCommand::Open(spec),
            origin,
            target: identity,
        });
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

    /// Queue native focus with trusted Rust Host origin. A pending Open in the
    /// same queue is a valid target and retains the same reservation identity.
    ///
    /// # Errors
    ///
    /// Returns unknown-window or queue-limit errors.
    pub fn request_focus(&mut self, id: &str) -> Result<(), WindowCommandError> {
        self.enqueue_target(
            WindowCommand::Focus(id.to_owned()),
            WindowCommandOrigin::Host,
        )
    }

    /// Queue focus after verifying that the source view owns window commands.
    ///
    /// # Errors
    ///
    /// Returns disabled-policy, unknown-window, or queue errors.
    pub fn request_focus_from(&mut self, source: &str, id: &str) -> Result<(), WindowCommandError> {
        self.require_commands(source, "focus_window")?;
        self.enqueue_target(
            WindowCommand::Focus(id.to_owned()),
            self.view_origin(source),
        )
    }

    /// Queue forced close with trusted Rust Host origin.
    ///
    /// # Errors
    ///
    /// Returns unknown-window or queue-limit errors.
    pub fn request_close(&mut self, id: &str) -> Result<(), WindowCommandError> {
        self.enqueue_target(
            WindowCommand::Close(id.to_owned()),
            WindowCommandOrigin::Host,
        )
    }

    /// Queue close after verifying that the source view owns window commands.
    ///
    /// # Errors
    ///
    /// Returns disabled-policy, unknown-window, or queue errors.
    pub fn request_close_from(&mut self, source: &str, id: &str) -> Result<(), WindowCommandError> {
        self.require_commands(source, "close_window")?;
        self.enqueue_target(
            WindowCommand::Close(id.to_owned()),
            self.view_origin(source),
        )
    }

    /// Queue open after verifying that the source view owns window commands.
    ///
    /// # Errors
    ///
    /// Returns disabled-policy, validation, duplicate, limit, or queue errors.
    pub fn request_open_from(
        &mut self,
        source: &str,
        spec: ScriptWindowSpec,
    ) -> Result<(), WindowCommandError> {
        self.require_commands(source, "open_window")?;
        self.enqueue_open(spec, self.view_origin(source))
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
        self.require_commands(id, "set_close_handler")?;
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
    pub fn drain_commands(&mut self) -> Vec<QueuedWindowCommand> {
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
        let Some(removed) = self.windows.remove(id) else {
            return false;
        };
        let queued = std::mem::take(&mut self.commands);
        for command in queued {
            let revoked = matches!(&command.origin, WindowCommandOrigin::View { identity, .. }
                if Rc::ptr_eq(identity, &removed.identity));
            if revoked || Rc::ptr_eq(&command.target, &removed.identity) {
                self.cancel_open(&command);
            } else {
                self.commands.push_back(command);
            }
        }
        true
    }

    fn view_origin(&self, id: &str) -> WindowCommandOrigin {
        WindowCommandOrigin::View {
            id: id.to_owned(),
            identity: Rc::clone(&self.windows[id].identity),
        }
    }

    fn enqueue_target(
        &mut self,
        command: WindowCommand,
        origin: WindowCommandOrigin,
    ) -> Result<(), WindowCommandError> {
        let target = self
            .windows
            .get(command.target())
            .ok_or_else(|| WindowCommandError::Unknown(command.target().to_owned()))?;
        let identity = Rc::clone(&target.identity);
        self.require_queue_capacity()?;
        self.commands.push_back(QueuedWindowCommand {
            command,
            origin,
            target: identity,
        });
        Ok(())
    }

    pub(crate) fn qualify_mount(
        &mut self,
        id: &str,
        lease: &Weak<()>,
    ) -> Result<(), WindowCommandError> {
        let identity = &self
            .windows
            .get(id)
            .ok_or_else(|| WindowCommandError::Unknown(id.to_owned()))?
            .identity;
        let mut mount = identity.mount.borrow_mut();
        if mount
            .as_ref()
            .is_some_and(|previous| !previous.ptr_eq(lease))
        {
            return Err(WindowCommandError::Duplicate(id.to_owned()));
        }
        *mount = Some(lease.clone());
        Ok(())
    }

    pub(crate) fn bind_native(
        &mut self,
        id: &str,
        window: gpui::WindowId,
    ) -> Result<(), WindowCommandError> {
        let identity = &self
            .windows
            .get(id)
            .ok_or_else(|| WindowCommandError::Unknown(id.to_owned()))?
            .identity;
        let lease = identity
            .mount
            .borrow()
            .clone()
            .ok_or_else(|| WindowCommandError::Unknown(id.to_owned()))?;
        let mut native = identity.native.borrow_mut();
        if native
            .as_ref()
            .is_some_and(|previous| previous.window != window || !previous.lease.ptr_eq(&lease))
        {
            return Err(WindowCommandError::Duplicate(id.to_owned()));
        }
        *native = Some(NativeWindowBinding { window, lease });
        Ok(())
    }

    pub(crate) fn is_current_target(&self, command: &QueuedWindowCommand) -> bool {
        self.windows
            .get(command.command.target())
            .is_some_and(|record| Rc::ptr_eq(&record.identity, &command.target))
    }

    pub(crate) fn is_current_origin(&self, command: &QueuedWindowCommand) -> bool {
        match &command.origin {
            WindowCommandOrigin::Host => true,
            WindowCommandOrigin::View { id, identity } => {
                self.windows.get(id).is_some_and(|record| {
                    Rc::ptr_eq(&record.identity, identity)
                        && identity
                            .mount
                            .borrow()
                            .as_ref()
                            .is_some_and(|lease| lease.strong_count() > 0)
                })
            }
        }
    }

    pub(crate) fn cancel_open(&mut self, command: &QueuedWindowCommand) {
        if matches!(command.command, WindowCommand::Open(_))
            && self.is_current_target(command)
            && self.windows[command.command.target()].status == WindowStatus::Pending
        {
            self.windows.remove(command.command.target());
        }
    }

    fn require_commands(
        &self,
        source: &str,
        command: &'static str,
    ) -> Result<(), WindowCommandError> {
        let record = self
            .windows
            .get(source)
            .ok_or_else(|| WindowCommandError::Unknown(source.to_owned()))?;
        if record.policy == WindowCommandPolicy::Disabled {
            Err(WindowCommandError::UnsupportedInEmbeddedView {
                window: source.to_owned(),
                view: record.view_id.clone(),
                command,
            })
        } else {
            Ok(())
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
    #[error("window `{0}` is already open")]
    AlreadyOpen(String),
    #[error("at most {0} script windows may be active")]
    WindowLimit(usize),
    #[error("at most {0} window commands may be pending")]
    QueueLimit(usize),
    #[error(
        "window command `{command}` is unavailable for embedded view `{view}` in window `{window}`"
    )]
    UnsupportedInEmbeddedView {
        window: String,
        view: String,
        command: &'static str,
    },
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
        windows.request_focus("settings").unwrap();
        windows.request_close("settings").unwrap();
        assert_eq!(
            windows
                .drain_commands()
                .into_iter()
                .map(|queued| queued.command)
                .collect::<Vec<_>>(),
            vec![
                WindowCommand::Open(spec("settings")),
                WindowCommand::Focus("settings".into()),
                WindowCommand::Close("settings".into())
            ]
        );
        windows.mark_open("settings").unwrap();
        windows.request_focus("settings").unwrap();
        windows.request_close("settings").unwrap();
        assert_eq!(
            windows
                .drain_commands()
                .into_iter()
                .map(|queued| queued.command)
                .collect::<Vec<_>>(),
            vec![
                WindowCommand::Focus("settings".to_owned()),
                WindowCommand::Close("settings".to_owned()),
            ]
        );
    }

    #[test]
    fn queued_origin_and_target_keep_their_registration_identity() {
        let mut windows = WindowCommandRegistry::new();
        let lease = Rc::new(());
        windows.register_open("source").unwrap();
        windows
            .qualify_mount("source", &Rc::downgrade(&lease))
            .unwrap();
        windows.request_open_from("source", spec("next")).unwrap();
        windows.request_focus_from("source", "next").unwrap();
        let queued = windows.drain_commands();
        assert!(queued.iter().all(
            |command| windows.is_current_origin(command) && windows.is_current_target(command)
        ));
        windows.remove("source");
        windows.register_open("source").unwrap();
        let replacement = Rc::new(());
        windows
            .qualify_mount("source", &Rc::downgrade(&replacement))
            .unwrap();
        assert!(
            queued
                .iter()
                .all(|command| !windows.is_current_origin(command))
        );
        windows.cancel_open(&queued[0]);
        windows.request_open_from("source", spec("next")).unwrap();
        assert!(!windows.is_current_target(&queued[1]));
        windows.cancel_open(&queued[0]);
        assert!(
            windows.contains("next"),
            "old cancellation must not release a newer reservation"
        );
    }

    #[test]
    fn revocation_releases_only_its_queued_open_and_host_origin_is_explicit() {
        let mut windows = WindowCommandRegistry::new();
        let lease = Rc::new(());
        windows.register_open("source").unwrap();
        windows
            .qualify_mount("source", &Rc::downgrade(&lease))
            .unwrap();
        windows
            .request_open_from("source", spec("cancelled"))
            .unwrap();
        windows.request_open(spec("trusted")).unwrap();
        windows.remove("source");
        assert!(!windows.contains("cancelled"));
        let queued = windows.drain_commands();
        assert_eq!(queued.len(), 1);
        assert!(queued[0].is_host_origin());
        assert!(windows.is_current_origin(&queued[0]));
        assert!(windows.is_current_target(&queued[0]));
    }

    #[test]
    fn invalid_window_specs_never_reserve_an_id() {
        let mut windows = WindowCommandRegistry::new();
        let mut invalid = spec("../escape");
        invalid.width = 20.0;
        assert!(windows.request_open(invalid).is_err());
        assert!(!windows.contains("../escape"));
    }

    #[test]
    fn embedded_view_policy_rejects_window_commands_at_call_site() {
        let mut windows = WindowCommandRegistry::new();
        windows
            .register_open_with_policy("host", WindowCommandPolicy::Disabled)
            .unwrap();
        assert!(matches!(
            windows.request_open_from("host", spec("settings")),
            Err(WindowCommandError::UnsupportedInEmbeddedView {
                command: "open_window",
                ..
            })
        ));
        assert!(matches!(
            windows.request_focus_from("host", "host"),
            Err(WindowCommandError::UnsupportedInEmbeddedView {
                command: "focus_window",
                ..
            })
        ));
        assert!(matches!(
            windows.request_close_from("host", "host"),
            Err(WindowCommandError::UnsupportedInEmbeddedView {
                command: "close_window",
                ..
            })
        ));
        assert!(windows.drain_commands().is_empty());
    }
}
