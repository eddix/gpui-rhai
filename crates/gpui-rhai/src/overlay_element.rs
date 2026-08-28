use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    AnyElement, App, Bounds, BoxShadow, DispatchPhase, Display, Element, ElementId, FocusHandle,
    GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, KeyDownEvent, LayoutId,
    MouseDownEvent, ParentElement, Pixels, Point, SharedString, StatefulInteractiveElement,
    Style as GpuiStyle, Styled, WeakFocusHandle, Window, deferred, div, point, px, rgba, size,
};

use crate::{
    FocusToken, OverlayBounds, OverlayId, OverlayKind, OverlayManager, OverlayNodeSpec,
    OverlaySpec, PlacementResult, Rgba8, ToastError, ToastRegion,
};

pub(crate) type OpenChangeHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;
pub(crate) type PanelKeyHandler = Rc<dyn Fn(&KeyDownEvent, &mut Window, &mut App) -> bool>;
pub(crate) type BackdropStyleHandler = Rc<dyn Fn(gpui::Div) -> gpui::Div>;

#[derive(Clone)]
pub(crate) struct WindowOverlayCoordinator(Rc<RefCell<OverlayCoordinatorState>>);

struct OverlayCoordinatorState {
    manager: OverlayManager,
    callbacks: BTreeMap<OverlayId, OpenChangeHandler>,
    priorities: BTreeMap<OverlayId, usize>,
    next_priority: usize,
    outside_listener_claimed: bool,
}

impl Default for WindowOverlayCoordinator {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(OverlayCoordinatorState {
            manager: OverlayManager::new(OverlayBounds::default())
                .expect("zero-sized overlay viewport is valid"),
            callbacks: BTreeMap::new(),
            priorities: BTreeMap::new(),
            next_priority: 1,
            outside_listener_claimed: false,
        })))
    }
}

impl WindowOverlayCoordinator {
    pub(crate) fn begin_frame(&self, viewport: gpui::Size<Pixels>) {
        let viewport = overlay_viewport(viewport);
        let mut state = self.0.borrow_mut();
        state
            .manager
            .begin_frame(viewport)
            .expect("GPUI viewport geometry is valid");
        state.callbacks.clear();
        state.priorities.clear();
        state.next_priority = 1;
        state.outside_listener_claimed = false;
    }

    fn reserve(
        &self,
        id: OverlayId,
        _kind: OverlayKind,
        callback: Option<OpenChangeHandler>,
    ) -> usize {
        let mut state = self.0.borrow_mut();
        if let Some(callback) = callback {
            state.callbacks.insert(id.clone(), callback);
        }
        if let Some(priority) = state.priorities.get(&id) {
            return *priority;
        }
        let priority = state.next_priority;
        state.next_priority = state.next_priority.saturating_add(1);
        state.priorities.insert(id, priority);
        priority
    }

    fn register(
        &self,
        spec: OverlaySpec,
        viewport: gpui::Size<Pixels>,
    ) -> Result<PlacementResult, crate::OverlayError> {
        let mut state = self.0.borrow_mut();
        state.manager.set_viewport(overlay_viewport(viewport))?;
        state.manager.open(spec)
    }

    fn dismiss(&self, id: &OverlayId, window: &mut Window, cx: &mut App) -> bool {
        let callbacks = {
            let mut state = self.0.borrow_mut();
            let report = state.manager.dismiss(id);
            report
                .dismissed
                .into_iter()
                .filter_map(|id| state.callbacks.remove(&id))
                .collect::<Vec<_>>()
        };
        let handled = !callbacks.is_empty();
        for callback in callbacks {
            callback(false, window, cx);
        }
        handled
    }

    fn dismiss_escape(&self, window: &mut Window, cx: &mut App) -> bool {
        self.dismiss_report(true, 0.0, 0.0, window, cx)
    }

    fn dismiss_outside(&self, point: Point<Pixels>, window: &mut Window, cx: &mut App) -> bool {
        self.dismiss_report(false, f64::from(point.x), f64::from(point.y), window, cx)
    }

    fn dismiss_report(
        &self,
        escape: bool,
        x: f64,
        y: f64,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let callbacks = {
            let mut state = self.0.borrow_mut();
            let report = if escape {
                state.manager.handle_escape()
            } else {
                state.manager.handle_outside_click(x, y)
            };
            report
                .dismissed
                .into_iter()
                .filter_map(|id| state.callbacks.remove(&id))
                .collect::<Vec<_>>()
        };
        let handled = !callbacks.is_empty();
        for callback in callbacks {
            callback(false, window, cx);
        }
        handled
    }

    fn claim_outside_listener(&self) -> bool {
        let mut state = self.0.borrow_mut();
        if state.outside_listener_claimed {
            false
        } else {
            state.outside_listener_claimed = true;
            true
        }
    }

    pub(crate) fn tooltip_visible(&self, id: &OverlayId, now: Instant) -> bool {
        let mut state = self.0.borrow_mut();
        let _ = state.manager.tooltips_mut().tick(now);
        state.manager.tooltips().visible() == Some(id)
    }

    fn tooltip_hover(
        &self,
        id: OverlayId,
        hovered: bool,
        show_delay: Duration,
        hide_delay: Duration,
        window: &mut Window,
        cx: &mut App,
    ) {
        let delay = if hovered { show_delay } else { hide_delay };
        {
            let mut state = self.0.borrow_mut();
            let now = cx.background_executor().now();
            if hovered {
                state
                    .manager
                    .tooltips_mut()
                    .pointer_enter(id, now, show_delay);
            } else {
                state
                    .manager
                    .tooltips_mut()
                    .pointer_leave(&id, now, hide_delay);
            }
        }
        let timer = cx.background_executor().timer(delay);
        window
            .spawn(cx, async move |cx| {
                timer.await;
                let _ = cx.update(|window, _| window.refresh());
            })
            .detach();
    }

    pub(crate) fn toast_set_max_visible(&self, max_visible: usize) {
        self.0
            .borrow_mut()
            .manager
            .toasts_mut()
            .set_max_visible(max_visible);
    }

    pub(crate) fn toast_contains(&self, id: &OverlayId) -> bool {
        self.0.borrow().manager.toasts().contains(id)
    }

    pub(crate) fn toast_enqueue(
        &self,
        id: OverlayId,
        region: ToastRegion,
        duration: Duration,
        now: Instant,
    ) -> Result<Vec<OverlayId>, ToastError> {
        self.0
            .borrow_mut()
            .manager
            .toasts_mut()
            .enqueue(id, region, duration, now)
    }

    pub(crate) fn toast_dismiss(&self, id: &OverlayId) -> bool {
        self.0.borrow_mut().manager.toasts_mut().dismiss(id)
    }

    pub(crate) fn toast_pause(&self, id: &OverlayId, now: Instant) -> bool {
        self.0.borrow_mut().manager.toasts_mut().pause(id, now)
    }

    pub(crate) fn toast_resume(&self, id: &OverlayId, now: Instant) -> bool {
        self.0.borrow_mut().manager.toasts_mut().resume(id, now)
    }

    pub(crate) fn toast_remaining(&self, id: &OverlayId, now: Instant) -> Option<Duration> {
        self.0.borrow().manager.toasts().remaining(id, now)
    }

    pub(crate) fn toast_tick(&self, now: Instant) -> Vec<OverlayId> {
        self.0.borrow_mut().manager.toasts_mut().tick(now)
    }

    pub(crate) fn toast_visible(&self, region: ToastRegion) -> Vec<OverlayId> {
        self.0
            .borrow()
            .manager
            .toasts()
            .visible(region)
            .into_iter()
            .cloned()
            .collect()
    }
}

fn overlay_viewport(viewport: gpui::Size<Pixels>) -> OverlayBounds {
    OverlayBounds {
        x: 0.0,
        y: 0.0,
        width: f64::from(viewport.width),
        height: f64::from(viewport.height),
    }
}

pub(crate) struct ScriptOverlayElement {
    id: ElementId,
    trigger: Option<AnyElement>,
    content: Option<AnyElement>,
    spec: OverlayNodeSpec,
    open_change: Option<OpenChangeHandler>,
    panel_key: Option<PanelKeyHandler>,
    backdrop_style: Option<BackdropStyleHandler>,
    focus_ring: Rgba8,
    focus_surface: Rgba8,
    restore_focus_on_close: bool,
    coordinator: WindowOverlayCoordinator,
}

impl ScriptOverlayElement {
    pub(crate) fn new(
        path: &str,
        trigger: AnyElement,
        content: AnyElement,
        spec: OverlayNodeSpec,
        open_change: Option<OpenChangeHandler>,
        panel_key: Option<PanelKeyHandler>,
        coordinator: WindowOverlayCoordinator,
    ) -> Self {
        let restore_focus_on_close = spec.modal;
        Self {
            id: SharedString::from(format!("{path}/overlay")).into(),
            trigger: Some(trigger),
            content: Some(content),
            spec,
            open_change,
            panel_key,
            backdrop_style: None,
            focus_ring: Rgba8::from_rgb_hex(0x003b_82f6),
            focus_surface: Rgba8::from_rgb_hex(0x0018_181b),
            restore_focus_on_close,
            coordinator,
        }
    }

    pub(crate) fn with_backdrop_style(mut self, style: Option<BackdropStyleHandler>) -> Self {
        self.backdrop_style = style;
        self
    }

    pub(crate) fn with_focus_ring(mut self, color: Rgba8) -> Self {
        self.focus_ring = color;
        self
    }

    pub(crate) fn with_focus_surface(mut self, color: Rgba8) -> Self {
        self.focus_surface = color;
        self
    }

    pub(crate) fn restore_focus_on_close(mut self, restore: bool) -> Self {
        self.restore_focus_on_close = restore;
        self
    }

    fn update_focus(&self, state: &mut OverlayElementState, window: &mut Window, cx: &App) {
        if self.spec.open && !state.was_open {
            if self.restore_focus_on_close {
                state.previous_focus = window.focused(cx).map(|focus| focus.downgrade());
            }
            if self.spec.modal || self.spec.kind == OverlayKind::Menu {
                state.panel_focus.focus(window);
            }
        } else if state.was_open && !self.spec.open && self.restore_focus_on_close {
            if self.spec.modal {
                if let Some(previous) = state
                    .previous_focus
                    .take()
                    .and_then(|focus| focus.upgrade())
                {
                    previous.focus(window);
                }
            } else {
                state.trigger_focus.focus(window);
            }
        }
        state.was_open = self.spec.open;
    }

    fn build_trigger(&mut self, trigger_focus: &FocusHandle) -> AnyElement {
        let click_callback = self.open_change.clone();
        let key_callback = self.open_change.clone();
        let panel_key = self.panel_key.clone();
        let click_coordinator = self.coordinator.clone();
        let click_id = self.spec.id.clone();
        let key_coordinator = self.coordinator.clone();
        let hover_coordinator = self.coordinator.clone();
        let hover_id = self.spec.id.clone();
        let tooltip_delays = self.spec.tooltip_delays;
        let open = self.spec.open;
        let dismiss_on_escape = self.spec.dismiss.escape;
        let focus_ring = self.focus_ring;
        let focus_surface = self.focus_surface;
        div()
            .id(SharedString::from(format!("{}-trigger", self.id)))
            .track_focus(trigger_focus)
            .tab_stop(click_callback.is_some())
            .focus(move |style| {
                style.shadow(vec![
                    BoxShadow {
                        color: rgba(focus_ring.as_rgba_hex()).into(),
                        offset: point(px(0.0), px(0.0)),
                        blur_radius: px(0.0),
                        spread_radius: px(4.0),
                    },
                    BoxShadow {
                        color: rgba(focus_surface.as_rgba_hex()).into(),
                        offset: point(px(0.0), px(0.0)),
                        blur_radius: px(0.0),
                        spread_radius: px(2.0),
                    },
                ])
            })
            .child(self.trigger.take().expect("overlay trigger rendered once"))
            .on_click(move |_, window, cx| {
                if open {
                    let _ = click_coordinator.dismiss(&click_id, window, cx);
                } else {
                    dispatch_open_change(click_callback.as_ref(), true, window, cx);
                }
            })
            .on_key_down(move |event, window, cx| {
                let key = event.keystroke.key.as_str();
                if key == "tab" {
                    if event.keystroke.modifiers.shift {
                        window.focus_prev();
                    } else {
                        window.focus_next();
                    }
                    cx.stop_propagation();
                    return;
                }
                if (key == "enter" && open || !matches!(key, "enter" | "space"))
                    && panel_key
                        .as_ref()
                        .is_some_and(|handler| handler(event, window, cx))
                {
                    cx.stop_propagation();
                    return;
                }
                let next = if matches!(key, "enter" | "space") {
                    Some(!open)
                } else if key == "escape" && open && dismiss_on_escape {
                    if key_coordinator.dismiss_escape(window, cx) {
                        cx.stop_propagation();
                    }
                    None
                } else {
                    None
                };
                if let Some(next) = next {
                    if next {
                        dispatch_open_change(key_callback.as_ref(), true, window, cx);
                    } else {
                        let _ = key_coordinator.dismiss_escape(window, cx);
                    }
                    if key_callback.is_some() {
                        cx.stop_propagation();
                    }
                }
            })
            .on_hover(move |hovered, window, cx| {
                if let Some(delays) = tooltip_delays {
                    hover_coordinator.tooltip_hover(
                        hover_id.clone(),
                        *hovered,
                        Duration::from_millis(delays.show_ms),
                        Duration::from_millis(delays.hide_ms),
                        window,
                        cx,
                    );
                }
            })
            .into_any_element()
    }

    fn build_overlay(
        &mut self,
        panel_focus: &FocusHandle,
        viewport: gpui::Size<Pixels>,
        priority: usize,
    ) -> Option<AnyElement> {
        if !self.spec.open {
            return None;
        }
        let panel_key = self.panel_key.clone();
        let focus = panel_focus.clone();
        let modal = self.spec.modal;
        let dismiss_on_escape = self.spec.dismiss.escape;
        let escape_coordinator = self.coordinator.clone();
        let hover_coordinator = self.coordinator.clone();
        let hover_id = self.spec.id.clone();
        let tooltip_delays = self.spec.tooltip_delays;
        let panel = div()
            .id(SharedString::from(format!("{}-panel", self.id)))
            .track_focus(panel_focus)
            .tab_stop(true)
            .occlude()
            .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
            .on_key_down(move |event, window, cx| {
                if panel_key
                    .as_ref()
                    .is_some_and(|handler| handler(event, window, cx))
                {
                    cx.stop_propagation();
                    return;
                }
                let key = event.keystroke.key.as_str();
                if key == "escape" && dismiss_on_escape {
                    if escape_coordinator.dismiss_escape(window, cx) {
                        cx.stop_propagation();
                    }
                } else if key == "tab" && modal {
                    if event.keystroke.modifiers.shift {
                        window.focus_prev();
                    } else {
                        window.focus_next();
                    }
                    if !focus.contains_focused(window, cx) {
                        focus.focus(window);
                    }
                    cx.stop_propagation();
                }
            })
            .on_hover(move |hovered, window, cx| {
                if let Some(delays) = tooltip_delays {
                    hover_coordinator.tooltip_hover(
                        hover_id.clone(),
                        *hovered,
                        Duration::from_millis(delays.show_ms),
                        Duration::from_millis(delays.hide_ms),
                        window,
                        cx,
                    );
                }
            })
            .child(self.content.take().expect("overlay content rendered once"));

        let overlay = if self.spec.kind == OverlayKind::Dialog {
            self.build_dialog_backdrop(panel, viewport)
        } else {
            div().absolute().child(panel).into_any_element()
        };
        if self.spec.parent.is_some() {
            // GPUI 0.2.x forbids calling `defer_draw` while it is already
            // prepainting a deferred element. A nested overlay is rendered as
            // part of its parent's deferred subtree, while the shared overlay
            // coordinator still owns placement, ordering, and dismissal.
            Some(overlay)
        } else {
            Some(deferred(overlay).with_priority(priority).into_any_element())
        }
    }

    fn build_dialog_backdrop(
        &self,
        panel: impl IntoElement,
        viewport: gpui::Size<Pixels>,
    ) -> AnyElement {
        let dismiss_on_outside = self.spec.dismiss.outside;
        let modal = self.spec.modal;
        let coordinator = self.coordinator.clone();
        let id = self.spec.id.clone();
        let backdrop = div()
            .absolute()
            .w(viewport.width)
            .h(viewport.height)
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x0000_0066));
        let backdrop = if let Some(style) = &self.backdrop_style {
            style(backdrop)
        } else {
            backdrop
        };
        backdrop
            .on_any_mouse_down(move |_, window, cx| {
                if dismiss_on_outside {
                    let _ = coordinator.dismiss(&id, window, cx);
                }
                if modal {
                    cx.stop_propagation();
                }
            })
            .child(panel)
            .into_any_element()
    }
}

struct OverlayElementState {
    was_open: bool,
    trigger_focus: FocusHandle,
    panel_focus: FocusHandle,
    previous_focus: Option<WeakFocusHandle>,
}

pub(crate) struct OverlayFrame {
    trigger: AnyElement,
    trigger_layout: LayoutId,
    overlay: Option<AnyElement>,
    overlay_layout: Option<LayoutId>,
}

#[derive(Default)]
pub(crate) struct OverlayPrepaint {
    overlay_offset: Option<Point<Pixels>>,
}

impl Element for ScriptOverlayElement {
    type RequestLayoutState = OverlayFrame;
    type PrepaintState = OverlayPrepaint;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        window.with_element_state(
            global_id.expect("overlay element is keyed"),
            |state, window| {
                let mut state = state.unwrap_or_else(|| OverlayElementState {
                    was_open: false,
                    trigger_focus: cx.focus_handle().tab_stop(self.open_change.is_some()),
                    panel_focus: cx.focus_handle().tab_stop(true),
                    previous_focus: None,
                });
                state.trigger_focus = state
                    .trigger_focus
                    .clone()
                    .tab_stop(self.open_change.is_some());
                self.update_focus(&mut state, window, cx);
                let mut trigger = self.build_trigger(&state.trigger_focus);
                let trigger_layout = trigger.request_layout(window, cx);
                let priority = self.coordinator.reserve(
                    self.spec.id.clone(),
                    self.spec.kind,
                    self.open_change.clone(),
                );
                let mut overlay =
                    self.build_overlay(&state.panel_focus, window.viewport_size(), priority);
                let overlay_layout = overlay
                    .as_mut()
                    .map(|overlay| overlay.request_layout(window, cx));
                let children = std::iter::once(trigger_layout).chain(overlay_layout);
                let layout = window.request_layout(
                    GpuiStyle {
                        display: Display::Flex,
                        ..GpuiStyle::default()
                    },
                    children,
                    cx,
                );
                (
                    (
                        layout,
                        OverlayFrame {
                            trigger,
                            trigger_layout,
                            overlay,
                            overlay_layout,
                        },
                    ),
                    state,
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        frame: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        frame.trigger.prepaint(window, cx);
        let trigger_bounds = window.layout_bounds(frame.trigger_layout);
        let Some(overlay_layout) = frame.overlay_layout else {
            return OverlayPrepaint::default();
        };
        let overlay_bounds = window.layout_bounds(overlay_layout);
        let overlay_spec = native_overlay_spec(&self.spec, trigger_bounds, overlay_bounds.size);
        let placement = self
            .coordinator
            .register(overlay_spec.clone(), window.viewport_size())
            .unwrap_or_else(|_| fallback_placement(overlay_spec, window.viewport_size()));
        let desired = if self.spec.kind == OverlayKind::Dialog {
            Bounds {
                origin: Point::default(),
                size: window.viewport_size(),
            }
        } else {
            placement_bounds(placement)
        };
        let offset = desired.origin - overlay_bounds.origin;
        if let Some(overlay) = frame.overlay.as_mut() {
            window.with_element_offset(offset, |window| overlay.prepaint(window, cx));
        }
        OverlayPrepaint {
            overlay_offset: Some(offset),
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        frame: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.trigger.paint(window, cx);
        if let (Some(overlay), Some(_)) = (frame.overlay.as_mut(), prepaint.overlay_offset) {
            overlay.paint(window, cx);
        }

        if !self.coordinator.claim_outside_listener() {
            return;
        }
        let coordinator = self.coordinator.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase == DispatchPhase::Bubble
                && coordinator.dismiss_outside(event.position, window, cx)
            {
                cx.stop_propagation();
            }
        });
    }
}

impl IntoElement for ScriptOverlayElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

fn native_overlay_spec(
    node: &OverlayNodeSpec,
    trigger: Bounds<Pixels>,
    panel: gpui::Size<Pixels>,
) -> OverlaySpec {
    OverlaySpec {
        id: node.id.clone(),
        parent: node.parent.clone(),
        kind: node.kind,
        anchor: OverlayBounds {
            x: f64::from(trigger.origin.x),
            y: f64::from(trigger.origin.y),
            width: f64::from(trigger.size.width),
            height: f64::from(trigger.size.height),
        },
        width: f64::from(panel.width),
        height: f64::from(panel.height),
        preferred: node.placement,
        gap: node.gap,
        modal: node.modal,
        dismiss_on_escape: node.dismiss.escape,
        dismiss_on_outside: node.dismiss.outside,
        restore_focus: Some(FocusToken(format!("overlay:{}", node.id.as_str()))),
    }
}

fn fallback_placement(spec: OverlaySpec, viewport: gpui::Size<Pixels>) -> PlacementResult {
    let mut fallback = spec;
    fallback.parent = None;
    OverlayManager::new(overlay_viewport(viewport))
        .and_then(|mut manager| manager.open(fallback))
        .expect("validated overlay geometry has a standalone placement")
}

fn placement_bounds(placement: PlacementResult) -> Bounds<Pixels> {
    Bounds {
        origin: point(
            pixel_from_f64(placement.bounds.x),
            pixel_from_f64(placement.bounds.y),
        ),
        size: size(
            pixel_from_f64(placement.bounds.width),
            pixel_from_f64(placement.bounds.height),
        ),
    }
}

fn pixel_from_f64(value: f64) -> Pixels {
    px(value.to_string().parse::<f32>().unwrap_or(f32::MAX))
}

fn dispatch_open_change(
    callback: Option<&OpenChangeHandler>,
    open: bool,
    window: &mut Window,
    cx: &mut App,
) {
    if let Some(callback) = callback {
        callback(open, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn spec(id: &str, parent: Option<&str>) -> OverlaySpec {
        OverlaySpec {
            id: OverlayId::new(id),
            parent: parent.map(OverlayId::new),
            kind: OverlayKind::Popover,
            anchor: OverlayBounds {
                x: 40.0,
                y: 40.0,
                width: 80.0,
                height: 24.0,
            },
            width: 160.0,
            height: 100.0,
            preferred: crate::OverlayPlacement::Bottom,
            gap: 4.0,
            modal: false,
            dismiss_on_escape: true,
            dismiss_on_outside: true,
            restore_focus: None,
        }
    }

    #[test]
    fn window_coordinator_shares_parent_stack_and_resets_per_frame() {
        let coordinator = WindowOverlayCoordinator::default();
        let viewport = size(px(800.0), px(600.0));
        coordinator.begin_frame(viewport);
        let parent = OverlayId::new("dialog");
        let child = OverlayId::new("popover");
        let parent_priority = coordinator.reserve(parent.clone(), OverlayKind::Dialog, None);
        let child_priority = coordinator.reserve(child.clone(), OverlayKind::Popover, None);
        coordinator
            .register(spec("dialog", None), viewport)
            .unwrap();
        coordinator
            .register(spec("popover", Some("dialog")), viewport)
            .unwrap();
        assert!(parent_priority < child_priority);
        let report = coordinator.0.borrow_mut().manager.dismiss(&parent);
        assert_eq!(report.dismissed, vec![child, parent]);

        coordinator
            .0
            .borrow_mut()
            .manager
            .toasts_mut()
            .enqueue(
                OverlayId::new("saved"),
                crate::ToastRegion::TopRight,
                Duration::from_secs(2),
                Instant::now(),
            )
            .unwrap();
        coordinator.begin_frame(viewport);
        assert!(
            coordinator
                .0
                .borrow()
                .manager
                .z_index(&OverlayId::new("dialog"))
                .is_none()
        );
        assert_eq!(
            coordinator
                .0
                .borrow()
                .manager
                .toasts()
                .visible(crate::ToastRegion::TopRight)
                .into_iter()
                .map(OverlayId::as_str)
                .collect::<Vec<_>>(),
            vec!["saved"]
        );
    }
}
