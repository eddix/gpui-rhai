//! The operating system's notification center and Dock badge, for Hosts.
//!
//! GPUI posts system notifications (`App::show_system_notification`, with
//! `on_system_notification_response` for clicks and actions) and rings the
//! bell (`Window::play_system_bell`, `Window::request_attention`); this module
//! adds what it lacks: whether notifications can reach the user at all, and
//! the macOS Dock badge.
//!
//! ```no_run
//! use gpui_rhai::gpui::App;
//! use gpui_rhai::system::{self, SystemNotification};
//!
//! fn session_waits(waiting: usize, cx: &mut App) {
//!     let label = (waiting > 0).then(|| waiting.to_string());
//!     let _ = system::set_badge_label(label.as_deref());
//!     if waiting > 0 && system::notifications_available() {
//!         cx.show_system_notification(SystemNotification {
//!             tag: "waiting".into(),
//!             title: "A session needs you".into(),
//!             body: format!("{waiting} sessions wait for approval").into(),
//!             actions: Vec::new(),
//!         });
//!     }
//! }
//! ```

pub use crate::gpui::{SystemNotification, SystemNotificationAction, SystemNotificationResponse};

/// A system service the platform does not offer, or not from where it was
/// asked for.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SystemServiceError {
    #[error("the Dock badge exists on macOS only")]
    Unsupported,
    #[error("the Dock badge is set from the main thread, where GPUI runs callbacks")]
    OffMainThread,
}

/// Whether system notifications can reach the user. macOS posts them only for
/// an application bundle with a bundle identifier, so a binary started by
/// `cargo run` shows none and GPUI drops them silently; build a bundle, or run
/// the binary from inside one. Other platforms always may.
#[must_use]
pub fn notifications_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        objc2_foundation::NSBundle::mainBundle()
            .bundleIdentifier()
            .is_some()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Shows `label` on the application's Dock icon, such as a count of sessions
/// that wait, or clears it with `None`.
///
/// # Errors
///
/// [`SystemServiceError::Unsupported`] off macOS, and
/// [`SystemServiceError::OffMainThread`] when not called on the main thread.
pub fn set_badge_label(label: Option<&str>) -> Result<(), SystemServiceError> {
    #[cfg(target_os = "macos")]
    {
        let main_thread =
            objc2::MainThreadMarker::new().ok_or(SystemServiceError::OffMainThread)?;
        let tile = objc2_app_kit::NSApplication::sharedApplication(main_thread).dockTile();
        let label = label.map(objc2_foundation::NSString::from_str);
        tile.setBadgeLabel(label.as_deref());
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = label;
        Err(SystemServiceError::Unsupported)
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn a_test_binary_is_no_bundle_and_runs_off_the_main_thread() {
        // Test threads are not the main thread, and `cargo test` binaries carry no bundle.
        assert!(!notifications_available());
        assert_eq!(
            set_badge_label(Some("3")),
            Err(SystemServiceError::OffMainThread)
        );
    }
}

/// Gives scripts the system services through capabilities, for a Host that
/// mounts its views with `.extension(SystemServices::default())` and an app
/// manifest that declares them:
///
/// - `system.notification@1`: `show(#{ tag, title, body?, actions? })`,
///   `dismiss(#{ tag })` and `available()`, and the subscription `responses`,
///   which delivers `#{ tag, action_id }` when the user clicks a notification
///   (`action_id` is `()` for its body) or one of its `actions`
///   (`#{ id, label }`).
/// - `system.badge@1`: `set(#{ label })` and `clear()`, the macOS Dock badge.
/// - `system.attention@1`: `bell()` and `request()`, on the view's window.
///
/// A capability handler gets no `App`, so the calls queue and run at the
/// view's next delivery poll (within a frame). GPUI keeps one notification
/// response handler per application: with these services mounted, that
/// handler is theirs.
#[derive(Clone, Default)]
pub struct SystemServices {
    shared: std::rc::Rc<SystemQueue>,
}

#[derive(Default)]
struct SystemQueue {
    commands: std::cell::RefCell<Vec<SystemCommand>>,
    responders: std::sync::Arc<std::sync::Mutex<Vec<std::sync::mpsc::Sender<crate::UiValue>>>>,
    response_handler: std::cell::Cell<bool>,
}

enum SystemCommand {
    Show(SystemNotification),
    Dismiss(String),
    Badge(Option<String>),
    Bell,
    Attention,
}

struct QueueHandler(std::rc::Rc<SystemQueue>);
struct ResponseHandler(
    std::sync::Arc<std::sync::Mutex<Vec<std::sync::mpsc::Sender<crate::UiValue>>>>,
);

fn text_field(input: &crate::UiValue, name: &str) -> Option<String> {
    match input {
        crate::UiValue::Map(map) => match map.get(name) {
            Some(crate::UiValue::String(value)) => Some(value.clone()),
            _ => None,
        },
        _ => None,
    }
}

impl crate::CapabilityHandler for QueueHandler {
    fn call(&mut self, method: &str, input: crate::UiValue) -> Result<crate::UiValue, String> {
        let command = match method {
            "show" => {
                let actions = match &input {
                    crate::UiValue::Map(map) => match map.get("actions") {
                        Some(crate::UiValue::Array(actions)) => actions
                            .iter()
                            .map(|action| SystemNotificationAction {
                                id: text_field(action, "id").unwrap_or_default().into(),
                                label: text_field(action, "label").unwrap_or_default().into(),
                            })
                            .collect(),
                        _ => Vec::new(),
                    },
                    _ => Vec::new(),
                };
                SystemCommand::Show(SystemNotification {
                    tag: text_field(&input, "tag").unwrap_or_default().into(),
                    title: text_field(&input, "title").unwrap_or_default().into(),
                    body: text_field(&input, "body").unwrap_or_default().into(),
                    actions,
                })
            }
            "dismiss" => SystemCommand::Dismiss(text_field(&input, "tag").unwrap_or_default()),
            "available" => return Ok(crate::UiValue::Bool(notifications_available())),
            "set" => SystemCommand::Badge(text_field(&input, "label")),
            "clear" => SystemCommand::Badge(None),
            "bell" => SystemCommand::Bell,
            "request" => SystemCommand::Attention,
            other => return Err(format!("unknown system service method `{other}`")),
        };
        self.0.commands.borrow_mut().push(command);
        Ok(crate::UiValue::Null)
    }
}

impl crate::SubscriptionCapabilityHandler for ResponseHandler {
    fn subscribe(
        &mut self,
        method: &str,
        _input: crate::UiValue,
    ) -> Result<crate::SubscriptionWork, String> {
        if method != "responses" {
            return Err(format!(
                "system.notification has no subscription `{method}`"
            ));
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        self.0
            .lock()
            .map_err(|_| "the notification responders are poisoned".to_owned())?
            .push(sender);
        Ok(crate::SubscriptionWork::from_receiver(receiver))
    }
}

fn method(input: crate::ValueSchema, output: crate::ValueSchema) -> crate::CapabilityMethod {
    crate::CapabilityMethod { input, output }
}

fn object(fields: &[(&str, crate::ObjectField)]) -> crate::ValueSchema {
    crate::ValueSchema::object(
        fields
            .iter()
            .map(|(name, field)| ((*name).to_owned(), field.clone()))
            .collect(),
    )
}

fn descriptor(
    id: &str,
    methods: Vec<(&str, crate::CapabilityMethod)>,
) -> Result<crate::CapabilityDescriptor, String> {
    Ok(crate::CapabilityDescriptor {
        id: crate::CapabilityId::parse(id).map_err(|error| error.to_string())?,
        version: semver::Version::new(1, 0, 0),
        methods: methods
            .into_iter()
            .map(|(name, method)| (name.to_owned(), method))
            .collect(),
    })
}

impl crate::ScriptViewExtension for SystemServices {
    fn configure_runtime(&self, runtime: &mut crate::UiRuntimeState) -> Result<(), String> {
        use crate::{ObjectField, ValueSchema};
        let string = || ObjectField::required(ValueSchema::string());
        let action = object(&[("id", string()), ("label", string())]);
        let notification = descriptor(
            "system.notification",
            vec![
                (
                    "show",
                    method(
                        object(&[
                            ("tag", string()),
                            ("title", string()),
                            ("body", ObjectField::optional(ValueSchema::string())),
                            (
                                "actions",
                                ObjectField::optional(ValueSchema::Array {
                                    items: Box::new(action),
                                    max_items: Some(8),
                                }),
                            ),
                        ]),
                        ValueSchema::Null,
                    ),
                ),
                (
                    "dismiss",
                    method(object(&[("tag", string())]), ValueSchema::Null),
                ),
                ("available", method(ValueSchema::Null, ValueSchema::Bool)),
                (
                    "responses",
                    method(
                        ValueSchema::Null,
                        object(&[
                            ("tag", string()),
                            (
                                "action_id",
                                ObjectField::required(ValueSchema::optional(ValueSchema::string())),
                            ),
                        ]),
                    ),
                ),
            ],
        )?;
        let capabilities = &mut runtime.capabilities;
        capabilities
            .register(notification.clone(), QueueHandler(self.shared.clone()))
            .map_err(|error| error.to_string())?;
        capabilities
            .register_subscription(
                notification,
                ResponseHandler(self.shared.responders.clone()),
            )
            .map_err(|error| error.to_string())?;
        capabilities
            .register(
                descriptor(
                    "system.badge",
                    vec![
                        (
                            "set",
                            method(object(&[("label", string())]), ValueSchema::Null),
                        ),
                        ("clear", method(ValueSchema::Null, ValueSchema::Null)),
                    ],
                )?,
                QueueHandler(self.shared.clone()),
            )
            .map_err(|error| error.to_string())?;
        capabilities
            .register(
                descriptor(
                    "system.attention",
                    vec![
                        ("bell", method(ValueSchema::Null, ValueSchema::Null)),
                        ("request", method(ValueSchema::Null, ValueSchema::Null)),
                    ],
                )?,
                QueueHandler(self.shared.clone()),
            )
            .map_err(|error| error.to_string())
    }

    fn poll(&self, window: Option<crate::gpui::AnyWindowHandle>, cx: &mut crate::gpui::App) {
        if !self.shared.response_handler.replace(true) {
            let responders = self.shared.responders.clone();
            cx.on_system_notification_response(move |response, _| {
                let value = crate::UiValue::Map(std::collections::BTreeMap::from([
                    (
                        "tag".to_owned(),
                        crate::UiValue::String(response.tag.to_string()),
                    ),
                    (
                        "action_id".to_owned(),
                        response.action_id.map_or(crate::UiValue::Null, |id| {
                            crate::UiValue::String(id.to_string())
                        }),
                    ),
                ]));
                if let Ok(mut senders) = responders.lock() {
                    senders.retain(|sender| sender.send(value.clone()).is_ok());
                }
            });
        }
        let commands = std::mem::take(&mut *self.shared.commands.borrow_mut());
        for command in commands {
            match command {
                SystemCommand::Show(notification) => cx.show_system_notification(notification),
                SystemCommand::Dismiss(tag) => cx.dismiss_system_notification(&tag),
                SystemCommand::Badge(label) => {
                    let _ = set_badge_label(label.as_deref());
                }
                SystemCommand::Bell | SystemCommand::Attention => {
                    let bell = matches!(command, SystemCommand::Bell);
                    if let Some(window) = window {
                        let _ =
                            crate::gpui::AppContext::update_window(cx, window, |_, window, _| {
                                if bell {
                                    window.play_system_bell();
                                } else {
                                    window.request_attention();
                                }
                            });
                    }
                }
            }
        }
    }
}
