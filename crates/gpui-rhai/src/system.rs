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
