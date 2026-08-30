use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use gpui::{App, SharedString, Window};
use rhai::{CustomType, TypeBuilder};

use crate::{ComponentInstancePath, ScriptCallback, UiValue};

#[derive(Clone, Debug, PartialEq)]
pub struct UiEvent {
    pub name: String,
    pub payload: UiValue,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalPoint {
    pub x: f64,
    pub y: f64,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EventModifiers {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub platform: bool,
    pub function: bool,
}

impl EventModifiers {
    #[must_use]
    pub fn into_value(self) -> UiValue {
        UiValue::Map(BTreeMap::from([
            ("control".to_owned(), UiValue::Bool(self.control)),
            ("alt".to_owned(), UiValue::Bool(self.alt)),
            ("shift".to_owned(), UiValue::Bool(self.shift)),
            ("platform".to_owned(), UiValue::Bool(self.platform)),
            ("function".to_owned(), UiValue::Bool(self.function)),
        ]))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointerEventData {
    pub pointer_id: u64,
    pub pointer_type: String,
    pub window: LogicalPoint,
    pub local: LogicalPoint,
    pub content: LogicalPoint,
    pub movement: LogicalPoint,
    pub button: Option<String>,
    pub buttons: Vec<String>,
    pub modifiers: EventModifiers,
    pub click_count: usize,
    pub timestamp_ms: f64,
    pub captured: bool,
}

impl PointerEventData {
    #[must_use]
    pub fn into_value(self) -> UiValue {
        UiValue::Map(BTreeMap::from([
            (
                "pointer_id".to_owned(),
                UiValue::Integer(i64::try_from(self.pointer_id).unwrap_or(i64::MAX)),
            ),
            (
                "pointer_type".to_owned(),
                UiValue::String(self.pointer_type),
            ),
            ("window".to_owned(), point_value(self.window)),
            ("local".to_owned(), point_value(self.local)),
            ("content".to_owned(), point_value(self.content)),
            ("movement".to_owned(), point_value(self.movement)),
            (
                "button".to_owned(),
                self.button.map_or(UiValue::Null, UiValue::String),
            ),
            (
                "buttons".to_owned(),
                UiValue::Array(self.buttons.into_iter().map(UiValue::String).collect()),
            ),
            ("modifiers".to_owned(), self.modifiers.into_value()),
            (
                "click_count".to_owned(),
                UiValue::Integer(i64::try_from(self.click_count).unwrap_or(i64::MAX)),
            ),
            ("timestamp_ms".to_owned(), UiValue::Float(self.timestamp_ms)),
            ("captured".to_owned(), UiValue::Bool(self.captured)),
            ("pressure".to_owned(), UiValue::Null),
            ("tilt_x".to_owned(), UiValue::Null),
            ("tilt_y".to_owned(), UiValue::Null),
        ]))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WheelEventData {
    pub window: LogicalPoint,
    pub local: LogicalPoint,
    pub content: LogicalPoint,
    pub delta: LogicalPoint,
    pub precise: bool,
    pub modifiers: EventModifiers,
    pub timestamp_ms: f64,
}

impl WheelEventData {
    #[must_use]
    pub fn into_value(self) -> UiValue {
        UiValue::Map(BTreeMap::from([
            ("window".to_owned(), point_value(self.window)),
            ("local".to_owned(), point_value(self.local)),
            ("content".to_owned(), point_value(self.content)),
            ("delta".to_owned(), point_value(self.delta)),
            ("precise".to_owned(), UiValue::Bool(self.precise)),
            ("modifiers".to_owned(), self.modifiers.into_value()),
            ("timestamp_ms".to_owned(), UiValue::Float(self.timestamp_ms)),
        ]))
    }
}

fn point_value(point: LogicalPoint) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("x".to_owned(), UiValue::Float(point.x)),
        ("y".to_owned(), UiValue::Float(point.y)),
    ]))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventPropagation {
    Handled,
    Propagate,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PropagationControl {
    #[default]
    Continue,
    Stop,
    StopImmediate,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PointerCaptureDirective {
    #[default]
    None,
    Capture,
    Release,
}

/// Orthogonal response controls returned by Script, Host, and native handlers.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EventResponse {
    prevent_default: bool,
    propagation: PropagationControl,
    pointer_capture: PointerCaptureDirective,
}

impl EventResponse {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            prevent_default: false,
            propagation: PropagationControl::Continue,
            pointer_capture: PointerCaptureDirective::None,
        }
    }

    #[must_use]
    pub const fn prevent_default(mut self) -> Self {
        self.prevent_default = true;
        self
    }

    #[must_use]
    pub const fn stop(mut self) -> Self {
        self.propagation = PropagationControl::Stop;
        self
    }

    #[must_use]
    pub const fn stop_immediate(mut self) -> Self {
        self.propagation = PropagationControl::StopImmediate;
        self
    }

    #[must_use]
    pub const fn capture_pointer(mut self) -> Self {
        self.pointer_capture = PointerCaptureDirective::Capture;
        self
    }

    #[must_use]
    pub const fn release_pointer(mut self) -> Self {
        self.pointer_capture = PointerCaptureDirective::Release;
        self
    }

    #[must_use]
    pub const fn default_prevented(self) -> bool {
        self.prevent_default
    }

    #[must_use]
    pub const fn propagation(self) -> PropagationControl {
        self.propagation
    }

    #[must_use]
    pub const fn pointer_capture(self) -> PointerCaptureDirective {
        self.pointer_capture
    }

    #[must_use]
    pub const fn stops_propagation(self) -> bool {
        !matches!(self.propagation, PropagationControl::Continue)
    }

    pub(crate) fn merge(&mut self, response: Self) {
        self.prevent_default |= response.prevent_default;
        if !matches!(response.propagation, PropagationControl::Continue) {
            self.propagation = response.propagation;
        }
        if !matches!(response.pointer_capture, PointerCaptureDirective::None) {
            self.pointer_capture = response.pointer_capture;
        }
    }
}

impl From<EventPropagation> for EventResponse {
    fn from(value: EventPropagation) -> Self {
        match value {
            EventPropagation::Handled => Self::new().stop(),
            EventPropagation::Propagate => Self::new(),
        }
    }
}

impl From<EventResponse> for EventPropagation {
    fn from(value: EventResponse) -> Self {
        if value.stops_propagation() {
            Self::Handled
        } else {
            Self::Propagate
        }
    }
}

impl CustomType for EventResponse {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("EventResponse")
            .with_fn("prevent_default", |response: &mut Self| {
                (*response).prevent_default()
            })
            .with_fn("stop", |response: &mut Self| (*response).stop())
            .with_fn("stop_immediate", |response: &mut Self| {
                (*response).stop_immediate()
            })
            .with_fn("capture_pointer", |response: &mut Self| {
                (*response).capture_pointer()
            })
            .with_fn("release_pointer", |response: &mut Self| {
                (*response).release_pointer()
            });
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

#[derive(Clone, Debug, Default)]
pub struct PointerCaptureRegistry {
    active: Rc<RefCell<BTreeMap<u64, crate::NodeId>>>,
}

impl PointerCaptureRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn capture(&self, pointer_id: u64, node: crate::NodeId) {
        self.active.borrow_mut().insert(pointer_id, node);
    }

    pub fn release(&self, pointer_id: u64) -> Option<crate::NodeId> {
        self.active.borrow_mut().remove(&pointer_id)
    }

    #[must_use]
    pub fn captured(&self, pointer_id: u64) -> Option<crate::NodeId> {
        self.active.borrow().get(&pointer_id).copied()
    }

    pub(crate) fn retain_nodes(&self, nodes: &std::collections::BTreeSet<crate::NodeId>) {
        self.active
            .borrow_mut()
            .retain(|_, node| nodes.contains(node));
    }

    pub(crate) fn snapshot(&self) -> BTreeMap<u64, crate::NodeId> {
        self.active.borrow().clone()
    }

    pub(crate) fn restore(&self, snapshot: BTreeMap<u64, crate::NodeId>) {
        *self.active.borrow_mut() = snapshot;
    }
}

type HostCallbackFn = dyn Fn(UiValue, &mut Window, &mut App) -> EventResponse;

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
    pub fn new<R>(
        label: impl Into<SharedString>,
        handler: impl Fn(UiValue, &mut Window, &mut App) -> R + 'static,
    ) -> Self
    where
        R: Into<EventResponse>,
    {
        let label = label.into();
        assert!(
            !label.as_ref().trim().is_empty(),
            "HostCallback label cannot be empty"
        );
        Self {
            label,
            handler: Rc::new(move |payload, window, app| handler(payload, window, app).into()),
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
    ) -> EventResponse {
        (self.handler)(payload, window, app)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiEventBinding {
    phase: EventPhase,
    handler: UiEventHandler,
}

impl UiEventBinding {
    #[must_use]
    pub fn new(phase: EventPhase, handler: impl Into<UiEventHandler>) -> Self {
        Self {
            phase,
            handler: handler.into(),
        }
    }

    #[must_use]
    pub const fn phase(&self) -> EventPhase {
        self.phase
    }

    #[must_use]
    pub const fn handler(&self) -> &UiEventHandler {
        &self.handler
    }

    pub(crate) const fn handler_mut(&mut self) -> &mut UiEventHandler {
        &mut self.handler
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

#[derive(Clone, Debug, PartialEq)]
pub enum UiEventHandler {
    Script(ScriptCallback),
    Host(HostCallback),
    Native(crate::NativeHandlerRef),
}

impl UiEventHandler {
    #[must_use]
    pub fn diagnostic_label(&self) -> String {
        match self {
            Self::Script(callback) => format!("script:{}", callback.name()),
            Self::Host(callback) => format!("host:{}", callback.label()),
            Self::Native(reference) => {
                format!("native:{}", reference.descriptor().id)
            }
        }
    }

    pub(crate) const fn as_script_mut(&mut self) -> Option<&mut ScriptCallback> {
        match self {
            Self::Script(callback) => Some(callback),
            Self::Host(_) | Self::Native(_) => None,
        }
    }

    #[must_use]
    pub const fn as_script(&self) -> Option<&ScriptCallback> {
        match self {
            Self::Script(callback) => Some(callback),
            Self::Host(_) | Self::Native(_) => None,
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

impl From<crate::NativeHandlerRef> for UiEventHandler {
    fn from(reference: crate::NativeHandlerRef) -> Self {
        Self::Native(reference)
    }
}

#[derive(Clone, Debug, Default)]
pub struct EventRouter {
    handlers: BTreeMap<ComponentInstancePath, BTreeMap<String, Vec<UiEventBinding>>>,
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
        self.bind_phase(target, event, EventPhase::Bubble, callback);
    }

    pub fn bind_phase(
        &mut self,
        target: ComponentInstancePath,
        event: impl Into<String>,
        phase: EventPhase,
        handler: impl Into<UiEventHandler>,
    ) {
        self.handlers
            .entry(target)
            .or_default()
            .entry(event.into())
            .or_default()
            .push(UiEventBinding::new(phase, handler));
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
            &UiEventHandler,
            &UiEvent,
        ) -> Result<EventResponse, E>,
    ) -> Result<EventDispatchReport, E> {
        let mut route = Vec::new();
        let mut current = Some(target.clone());
        while let Some(path) = current {
            route.push(path.clone());
            current = path.parent();
        }
        route.reverse();
        let mut report = EventDispatchReport::default();
        for path in &route {
            if self.dispatch_phase(path, event, EventPhase::Capture, &mut invoke, &mut report)? {
                return Ok(report);
            }
        }
        if self.dispatch_phase(target, event, EventPhase::Target, &mut invoke, &mut report)? {
            return Ok(report);
        }
        for path in route.iter().rev() {
            if self.dispatch_phase(path, event, EventPhase::Bubble, &mut invoke, &mut report)? {
                return Ok(report);
            }
        }
        Ok(report)
    }

    fn dispatch_phase<E>(
        &self,
        path: &ComponentInstancePath,
        event: &UiEvent,
        phase: EventPhase,
        invoke: &mut impl FnMut(
            &ComponentInstancePath,
            &UiEventHandler,
            &UiEvent,
        ) -> Result<EventResponse, E>,
        report: &mut EventDispatchReport,
    ) -> Result<bool, E> {
        report.visited.push((path.clone(), phase));
        let Some(bindings) = self
            .handlers
            .get(path)
            .and_then(|handlers| handlers.get(&event.name))
        else {
            return Ok(false);
        };
        let mut stop_route = false;
        for binding in bindings.iter().filter(|binding| binding.phase == phase) {
            let response = invoke(path, &binding.handler, event)?;
            report.response.merge(response);
            report.invoked = report.invoked.saturating_add(1);
            if matches!(response.propagation, PropagationControl::StopImmediate) {
                report.stopped_at = Some(path.clone());
                return Ok(true);
            }
            if matches!(response.propagation, PropagationControl::Stop) {
                report.stopped_at = Some(path.clone());
                stop_route = true;
            }
        }
        Ok(stop_route)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EventDispatchReport {
    pub visited: Vec<(ComponentInstancePath, EventPhase)>,
    pub stopped_at: Option<ComponentInstancePath>,
    pub invoked: usize,
    pub response: EventResponse,
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
                Ok::<_, ()>(
                    if path == &root {
                        EventPropagation::Handled
                    } else {
                        EventPropagation::Propagate
                    }
                    .into(),
                )
            })
            .unwrap();

        assert_eq!(order, vec![child, root.clone()]);
        assert_eq!(report.stopped_at, Some(root));
        assert_eq!(report.invoked, 2);
    }

    #[test]
    fn capture_target_bubble_and_immediate_stop_are_ordered() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile("fn view() { text(\"event\") } fn callback(payload) { payload }")
            .unwrap();
        let callback = runtime.callback(&compiled, "callback").unwrap();
        let root = ComponentInstancePath::root("App", "root");
        let child = root.child("Box", "child");
        let mut router = EventRouter::new();
        router.bind_phase(
            root.clone(),
            "pointer_down",
            EventPhase::Capture,
            callback.clone(),
        );
        router.bind_phase(
            child.clone(),
            "pointer_down",
            EventPhase::Target,
            callback.clone(),
        );
        router.bind_phase(
            child.clone(),
            "pointer_down",
            EventPhase::Target,
            callback.clone(),
        );
        router.bind_phase(root.clone(), "pointer_down", EventPhase::Bubble, callback);
        let event = UiEvent {
            name: "pointer_down".to_owned(),
            payload: UiValue::Null,
        };
        let mut order = Vec::new();
        let report = router
            .dispatch(&child, &event, |path, _, _| {
                order.push(path.clone());
                Ok::<_, ()>(if order.len() == 2 {
                    EventResponse::new().prevent_default().stop_immediate()
                } else {
                    EventResponse::new()
                })
            })
            .unwrap();
        assert_eq!(order, vec![root, child.clone()]);
        assert_eq!(report.stopped_at, Some(child));
        assert_eq!(report.invoked, 2);
        assert!(report.response.default_prevented());
    }

    #[test]
    fn stop_allows_remaining_same_phase_handlers_but_blocks_ancestors() {
        let mut runtime = RuntimeEngine::new();
        let compiled = runtime
            .compile("fn view() { text(\"event\") } fn callback(payload) { payload }")
            .unwrap();
        let callback = runtime.callback(&compiled, "callback").unwrap();
        let root = ComponentInstancePath::root("App", "root");
        let child = root.child("Box", "child");
        let mut router = EventRouter::new();
        router.bind(child.clone(), "click", callback.clone());
        router.bind(child.clone(), "click", callback.clone());
        router.bind(root, "click", callback);
        let mut invoked = 0;
        let report = router
            .dispatch(
                &child,
                &UiEvent {
                    name: "click".to_owned(),
                    payload: UiValue::Null,
                },
                |_, _, _| {
                    invoked += 1;
                    Ok::<_, ()>(if invoked == 1 {
                        EventResponse::new().stop()
                    } else {
                        EventResponse::new()
                    })
                },
            )
            .unwrap();
        assert_eq!(invoked, 2);
        assert_eq!(report.stopped_at, Some(child));
    }

    #[test]
    fn pointer_capture_is_node_scoped_and_clears_on_unmount() {
        let mut tree = crate::RetainedUiTree::new();
        tree.reconcile(crate::UiNode::text("drag")).unwrap();
        let node = tree.root_id().unwrap();
        let captures = PointerCaptureRegistry::new();
        captures.capture(0, node);
        assert_eq!(captures.captured(0), Some(node));
        captures.retain_nodes(&std::collections::BTreeSet::new());
        assert_eq!(captures.captured(0), None);
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
