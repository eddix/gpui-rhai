use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use gpui::{App, SharedString, Window};

use crate::{ComponentInstancePath, ScriptCallback, UiValue};

#[derive(Clone, Debug, PartialEq)]
pub struct UiEvent {
    pub name: String,
    pub payload: UiValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventPropagation {
    Handled,
    Propagate,
}

type HostCallbackFn = dyn Fn(UiValue, &mut Window, &mut App) -> EventPropagation;

#[derive(Clone)]
pub struct HostCallback {
    label: SharedString,
    handler: Rc<HostCallbackFn>,
}

impl HostCallback {
    /// Construct one foreground Host event callback.
    ///
    /// # Panics
    ///
    /// Panics when the diagnostic label is empty or whitespace-only.
    #[must_use]
    pub fn new(
        label: impl Into<SharedString>,
        handler: impl Fn(UiValue, &mut Window, &mut App) -> EventPropagation + 'static,
    ) -> Self {
        let label = label.into();
        assert!(
            !label.as_ref().trim().is_empty(),
            "HostCallback label cannot be empty"
        );
        Self {
            label,
            handler: Rc::new(handler),
        }
    }

    #[must_use]
    pub fn label(&self) -> &str {
        self.label.as_ref()
    }

    pub(crate) fn invoke(
        &self,
        payload: UiValue,
        window: &mut Window,
        app: &mut App,
    ) -> EventPropagation {
        (self.handler)(payload, window, app)
    }
}

impl fmt::Debug for HostCallback {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostCallback")
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl PartialEq for HostCallback {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label && Rc::ptr_eq(&self.handler, &other.handler)
    }
}

impl Eq for HostCallback {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiEventHandler {
    Script(ScriptCallback),
    Host(HostCallback),
}

impl UiEventHandler {
    #[must_use]
    pub fn diagnostic_label(&self) -> String {
        match self {
            Self::Script(callback) => format!("script:{}", callback.name()),
            Self::Host(callback) => format!("host:{}", callback.label()),
        }
    }

    pub(crate) const fn as_script_mut(&mut self) -> Option<&mut ScriptCallback> {
        match self {
            Self::Script(callback) => Some(callback),
            Self::Host(_) => None,
        }
    }

    #[must_use]
    pub const fn as_script(&self) -> Option<&ScriptCallback> {
        match self {
            Self::Script(callback) => Some(callback),
            Self::Host(_) => None,
        }
    }
}

impl From<ScriptCallback> for UiEventHandler {
    fn from(callback: ScriptCallback) -> Self {
        Self::Script(callback)
    }
}

impl From<HostCallback> for UiEventHandler {
    fn from(callback: HostCallback) -> Self {
        Self::Host(callback)
    }
}

#[derive(Clone, Debug, Default)]
pub struct EventRouter {
    handlers: BTreeMap<ComponentInstancePath, BTreeMap<String, ScriptCallback>>,
}

impl EventRouter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind(
        &mut self,
        target: ComponentInstancePath,
        event: impl Into<String>,
        callback: ScriptCallback,
    ) {
        self.handlers
            .entry(target)
            .or_default()
            .insert(event.into(), callback);
    }

    pub fn unbind_component(&mut self, target: &ComponentInstancePath) {
        self.handlers.remove(target);
    }

    /// Bubble a normalized event from its target toward the root.
    ///
    /// # Errors
    ///
    /// Propagates errors returned by the callback invoker and stops dispatch.
    pub fn dispatch<E>(
        &self,
        target: &ComponentInstancePath,
        event: &UiEvent,
        mut invoke: impl FnMut(
            &ComponentInstancePath,
            &ScriptCallback,
            &UiEvent,
        ) -> Result<EventPropagation, E>,
    ) -> Result<EventDispatchReport, E> {
        let mut current = Some(target.clone());
        let mut visited = Vec::new();
        while let Some(path) = current {
            visited.push(path.clone());
            if let Some(callback) = self
                .handlers
                .get(&path)
                .and_then(|handlers| handlers.get(&event.name))
                && invoke(&path, callback, event)? == EventPropagation::Handled
            {
                return Ok(EventDispatchReport {
                    visited,
                    handled_by: Some(path),
                });
            }
            current = path.parent();
        }
        Ok(EventDispatchReport {
            visited,
            handled_by: None,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventDispatchReport {
    pub visited: Vec<ComponentInstancePath>,
    pub handled_by: Option<ComponentInstancePath>,
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::{RuntimeEngine, UiNode};

    struct DropSentinel(Rc<Cell<bool>>);

    impl Drop for DropSentinel {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    #[test]
    fn events_bubble_until_explicitly_handled() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn view() { text("event") }
                    fn child_click(payload) { payload }
                    fn root_click(payload) { payload }
                "#,
            )
            .unwrap();
        let root = ComponentInstancePath::root("App", "root");
        let child = root.child("Button", "save");
        let mut router = EventRouter::new();
        router.bind(
            child.clone(),
            "click",
            runtime.callback(&compiled, "child_click").unwrap(),
        );
        router.bind(
            root.clone(),
            "click",
            runtime.callback(&compiled, "root_click").unwrap(),
        );
        let event = UiEvent {
            name: "click".to_owned(),
            payload: UiValue::Null,
        };
        let mut order = Vec::new();
        let report = router
            .dispatch(&child, &event, |path, _, _| {
                order.push(path.clone());
                Ok::<_, ()>(if path == &root {
                    EventPropagation::Handled
                } else {
                    EventPropagation::Propagate
                })
            })
            .unwrap();

        assert_eq!(order, vec![child, root.clone()]);
        assert_eq!(report.handled_by, Some(root));
    }

    #[test]
    fn host_callback_identity_debug_and_tree_drop_follow_rc_ownership() {
        let callback = HostCallback::new("widget.input", |_, _, _| EventPropagation::Handled);
        let cloned = callback.clone();
        let other = HostCallback::new("widget.input", |_, _, _| EventPropagation::Handled);
        assert_eq!(callback, cloned);
        assert_ne!(callback, other);
        let debug = format!("{callback:?}");
        assert!(debug.contains("widget.input"));
        assert!(!debug.contains("0x"));

        let dropped = Rc::new(Cell::new(false));
        {
            let sentinel = DropSentinel(Rc::clone(&dropped));
            let callback = HostCallback::new("widget.drop", move |_, _, _| {
                let _ = &sentinel;
                EventPropagation::Handled
            });
            let _tree = UiNode::text("host-owned").with_host_handler("click", callback);
        }
        assert!(dropped.get());
    }

    #[test]
    fn rhai_cannot_construct_host_callbacks() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile(
                r#"
                    fn view() {
                        text("host").with_host_handler("click", ())
                    }
                "#,
            )
            .unwrap();
        assert!(runtime.render(&compiled).is_err());
    }
}
