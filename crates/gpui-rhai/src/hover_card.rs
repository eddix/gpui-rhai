//! A hover disclosure owns intent and one cancellable deadline, not the
//! authoritative open value. Placement, dismiss ordering and rendering are
//! shared with every other Overlay.
use std::{cell::RefCell, rc::Rc, time::Duration};

use gpui::{App, FocusHandle, Pixels, Point, Subscription, Task, WeakFocusHandle, Window};

use crate::{OverlayBounds, OverlayId, TooltipDelays, overlay_element::WindowOverlayCoordinator};

#[derive(Clone)]
pub(crate) struct HoverBinding {
    pub(crate) id: OverlayId,
    pub(crate) session: Rc<RefCell<HoverSession>>,
    coordinator: WindowOverlayCoordinator,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum Zone {
    #[default]
    Outside,
    PointerTrigger,
    KeyboardTrigger,
    Panel,
}

impl Zone {
    fn is_trigger(self) -> bool {
        matches!(self, Self::PointerTrigger | Self::KeyboardTrigger)
    }
    fn holds_card(self) -> bool {
        matches!(self, Self::KeyboardTrigger | Self::Panel)
    }
}

#[derive(Default)]
struct PointerPresence {
    trigger: bool,
    panel: bool,
}

pub(crate) struct HoverSession {
    open: bool,
    show: Duration,
    hide: Duration,
    trigger: Option<WeakFocusHandle>,
    panel: Option<WeakFocusHandle>,
    opener: Option<WeakFocusHandle>,
    focus: Zone,
    pointer: PointerPresence,
    blocked: bool,
    restore: bool,
    trigger_bounds: OverlayBounds,
    panel_bounds: Option<OverlayBounds>,
    last_trigger_point: Option<Point<Pixels>>,
    corridor: Option<Point<Pixels>>,
    focus_subscriptions: Vec<Subscription>,
    serial: u64,
    pending: Option<bool>,
    task: Option<Task<()>>,
}

impl Default for HoverSession {
    fn default() -> Self {
        Self {
            open: false,
            show: Duration::from_millis(600),
            hide: Duration::from_millis(300),
            trigger: None,
            panel: None,
            opener: None,
            focus: Zone::Outside,
            pointer: PointerPresence::default(),
            blocked: false,
            restore: false,
            trigger_bounds: OverlayBounds::default(),
            panel_bounds: None,
            last_trigger_point: None,
            corridor: None,
            focus_subscriptions: Vec::new(),
            serial: 0,
            pending: None,
            task: None,
        }
    }
}

impl HoverSession {
    fn cancel(&mut self) {
        self.pending = None;
        self.serial = self.serial.wrapping_add(1);
        self.task.take();
    }

    fn trigger(&self) -> Option<FocusHandle> {
        self.trigger.as_ref().and_then(WeakFocusHandle::upgrade)
    }

    fn panel(&self) -> Option<FocusHandle> {
        self.panel.as_ref().and_then(WeakFocusHandle::upgrade)
    }

    fn return_target(&self, window: &Window) -> Option<FocusHandle> {
        let trigger = self.trigger()?;
        self.opener
            .as_ref()
            .and_then(WeakFocusHandle::upgrade)
            .filter(|focus| trigger.contains(focus, window))
            .or(Some(trigger))
    }

    fn unblock_if_outside(&mut self) {
        if !self.pointer.trigger && !self.focus.is_trigger() {
            self.blocked = false;
        }
    }

    fn desired_open(&self) -> bool {
        !self.blocked
            && (self.pointer.trigger
                || self.pointer.panel
                || self.corridor.is_some()
                || self.focus.holds_card())
    }

    pub(crate) fn force_close(&mut self, escape: bool, window: &Window, cx: &App) {
        self.cancel();
        self.blocked = true;
        self.corridor = None;
        self.restore = escape
            && self
                .panel()
                .is_some_and(|panel| panel.contains_focused(window, cx));
    }
}

impl HoverBinding {
    pub(crate) fn new(
        id: OverlayId,
        session: Rc<RefCell<HoverSession>>,
        coordinator: WindowOverlayCoordinator,
    ) -> Self {
        Self {
            id,
            session,
            coordinator,
        }
    }

    pub(crate) fn sync(
        &self,
        open: bool,
        delays: TooltipDelays,
        trigger: &FocusHandle,
        panel: &FocusHandle,
        window: &mut Window,
        cx: &mut App,
    ) {
        let (needs_watch, restore) = {
            let mut state = self.session.borrow_mut();
            let mut restore = None;
            if open != state.open {
                state.cancel();
                state.open = open;
                if !open {
                    state.pointer.panel = false;
                    state.panel_bounds = None;
                    state.corridor = None;
                    if std::mem::take(&mut state.restore) {
                        restore = state.return_target(window);
                    }
                }
            }
            state.show = Duration::from_millis(delays.show_ms);
            state.hide = Duration::from_millis(delays.hide_ms);
            let watch =
                state.trigger().as_ref() != Some(trigger) || state.panel().as_ref() != Some(panel);
            if watch {
                state.trigger = Some(trigger.downgrade());
                state.panel = Some(panel.downgrade());
            }
            (watch, restore)
        };
        if needs_watch {
            let mut watches = Vec::with_capacity(4);
            for handle in [trigger, panel] {
                let weak = Rc::downgrade(&self.session);
                let id = self.id.clone();
                let coord = self.coordinator.clone();
                watches.push(window.on_focus_in(handle, cx, move |window, cx| {
                    if let Some(session) = weak.upgrade() {
                        Self::new(id.clone(), session, coord.clone()).focus_changed(window, cx);
                    }
                }));
                let weak = Rc::downgrade(&self.session);
                let id = self.id.clone();
                let coord = self.coordinator.clone();
                watches.push(window.on_focus_out(handle, cx, move |_, window, cx| {
                    if let Some(session) = weak.upgrade() {
                        Self::new(id.clone(), session, coord.clone()).focus_changed(window, cx);
                    }
                }));
            }
            self.session.borrow_mut().focus_subscriptions = watches;
        }
        if let Some(target) = restore {
            let trigger = self.session.borrow().trigger();
            Self::focus_opener(&target, trigger.as_ref(), window, cx);
        }
    }

    pub(crate) fn set_bounds(&self, trigger: OverlayBounds, panel: Option<OverlayBounds>) {
        let mut state = self.session.borrow_mut();
        state.trigger_bounds = trigger;
        state.panel_bounds = panel;
    }

    fn focus_changed(&self, window: &mut Window, cx: &mut App) {
        if !self.coordinator.current_hover_card(&self.id, &self.session) {
            return;
        }
        let immediate = {
            let mut state = self.session.borrow_mut();
            let next = if state
                .panel()
                .is_some_and(|p| p.contains_focused(window, cx))
            {
                Zone::Panel
            } else if state
                .trigger()
                .is_some_and(|t| t.contains_focused(window, cx))
            {
                if window.last_input_was_keyboard() {
                    Zone::KeyboardTrigger
                } else {
                    Zone::PointerTrigger
                }
            } else {
                Zone::Outside
            };
            if state.focus == next {
                return;
            }
            let focus_owned = state.focus.holds_card();
            state.focus = next;
            if next.is_trigger() {
                state.opener = window.focused(cx).map(|f| f.downgrade());
            }
            if next == Zone::Outside && focus_owned {
                state.blocked = state.pointer.trigger;
            }
            (next == Zone::Outside && focus_owned).then_some(Duration::ZERO)
        };
        self.schedule(immediate, window, cx);
    }

    pub(crate) fn hover(&self, panel: bool, inside: bool, window: &mut Window, cx: &mut App) {
        if !self.coordinator.current_hover_card(&self.id, &self.session) {
            return;
        }
        if window.last_input_was_touch() {
            return;
        }
        {
            let mut state = self.session.borrow_mut();
            let point = window.mouse_position();
            if panel {
                if state.pointer.panel == inside {
                    return;
                }
                state.pointer.panel = inside;
                if inside {
                    state.corridor = None;
                }
            } else {
                if state.pointer.trigger == inside {
                    return;
                }
                state.pointer.trigger = inside;
                if inside {
                    state.last_trigger_point = Some(point);
                    state.corridor = None;
                } else {
                    state.corridor = None;
                    if state.open
                        && !state.pointer.panel
                        && let (Some(origin), Some(card)) =
                            (state.last_trigger_point, state.panel_bounds)
                        && toward_card(origin, point, card)
                    {
                        state.corridor = Some(origin);
                    }
                }
            }
            state.unblock_if_outside();
        }
        self.schedule(None, window, cx);
    }

    pub(crate) fn pointer_moved(&self, point: Point<Pixels>, window: &mut Window, cx: &mut App) {
        if !self.coordinator.current_hover_card(&self.id, &self.session) {
            return;
        }
        let left_corridor = {
            let mut state = self.session.borrow_mut();
            if state.pointer.trigger
                && state
                    .trigger_bounds
                    .contains(f64::from(point.x), f64::from(point.y))
            {
                state.last_trigger_point = Some(point);
            }
            if let (Some(origin), Some(card)) = (state.corridor, state.panel_bounds)
                && !toward_card(origin, point, card)
            {
                state.corridor = None;
                true
            } else {
                false
            }
        };
        if left_corridor {
            self.schedule(None, window, cx);
        }
    }

    pub(crate) fn press(&self, window: &mut Window, cx: &mut App) {
        if !self.coordinator.current_hover_card(&self.id, &self.session) {
            return;
        }
        self.session.borrow_mut().force_close(false, window, cx);
        self.schedule(Some(Duration::ZERO), window, cx);
    }

    pub(crate) fn tab_from_trigger(&self, window: &mut Window, cx: &mut App) -> bool {
        let panel = {
            let state = self.session.borrow();
            if !state.open {
                return false;
            }
            state.panel()
        };
        let Some(panel) = panel else {
            return false;
        };
        let previous = window.focused(cx);
        panel.focus(window, cx);
        window.focus_next(cx);
        let moved = panel.contains_focused(window, cx) && !panel.is_focused(window);
        if !moved && let Some(previous) = previous {
            previous.focus(window, cx);
        }
        moved
    }

    fn focus_opener(
        target: &FocusHandle,
        trigger: Option<&FocusHandle>,
        window: &mut Window,
        cx: &mut App,
    ) {
        target.focus(window, cx);
        if trigger == Some(target) {
            // A pointer-opened preview may never have focused its anchor.
            // If the wrapper contains a real control, restore the real stop.
            window.focus_next(cx);
            if !target.contains_focused(window, cx) {
                target.focus(window, cx);
            }
        }
    }

    pub(crate) fn tab_from_panel(&self, back: bool, window: &mut Window, cx: &mut App) -> bool {
        let (panel, trigger, opener) = {
            let state = self.session.borrow();
            (state.panel(), state.trigger(), state.return_target(window))
        };
        let Some(panel) = panel.filter(|panel| panel.contains_focused(window, cx)) else {
            return false;
        };
        if back {
            window.focus_prev(cx);
        } else {
            window.focus_next(cx);
        }
        if panel.contains_focused(window, cx) && !panel.is_focused(window) {
            return true;
        }
        let natural = window.focused(cx);
        if let Some(opener) = opener {
            Self::focus_opener(&opener, trigger.as_ref(), window, cx);
            if !back {
                window.focus_next(cx);
                if panel.contains_focused(window, cx)
                    && let Some(natural) = natural
                    && !panel.contains(&natural, window)
                {
                    natural.focus(window, cx);
                }
            }
        }
        true
    }

    fn schedule(&self, override_delay: Option<Duration>, window: &Window, cx: &App) {
        let (target, delay, serial) = {
            let mut state = self.session.borrow_mut();
            let target = state.desired_open();
            if target == state.open {
                state.cancel();
                return;
            }
            if state.pending == Some(target) {
                return;
            }
            state.cancel();
            state.pending = Some(target);
            (
                target,
                override_delay.unwrap_or(if target { state.show } else { state.hide }),
                state.serial,
            )
        };
        let weak = Rc::downgrade(&self.session);
        let coordinator = self.coordinator.clone();
        let id = self.id.clone();
        let timer = cx.background_executor().timer(delay);
        let task = window.spawn(cx, async move |cx| {
            timer.await;
            let _ = cx.update(|window, cx| {
                let Some(session) = weak.upgrade() else {
                    return;
                };
                {
                    let mut state = session.borrow_mut();
                    if state.serial != serial
                        || state.pending != Some(target)
                        || state.desired_open() != target
                    {
                        return;
                    }
                    state.pending = None;
                }
                coordinator.propose_hover_card(&id, &session, target, window, cx);
            });
        });
        self.session.borrow_mut().task = Some(task);
    }
}

/// A point is protected only if the ray from the last point inside the
/// trigger passes through it and then meets the panel. Works on any side,
/// with translated/unequal rectangles, without also protecting the rest of
/// the trigger's row.
fn toward_card(origin: Point<Pixels>, point: Point<Pixels>, card: OverlayBounds) -> bool {
    let mut start = 1.0_f64;
    let mut end = f64::INFINITY;
    for (from, to, min, extent) in [
        (origin.x, point.x, card.x, card.width),
        (origin.y, point.y, card.y, card.height),
    ] {
        let from = f64::from(from);
        let delta = f64::from(to) - from;
        let max = min + extent;
        if delta == 0. {
            if !(min..=max).contains(&from) {
                return false;
            }
        } else {
            let a = (min - from) / delta;
            let b = (max - from) / delta;
            start = start.max(a.min(b));
            end = end.min(a.max(b));
        }
    }
    start <= end
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corridor_follows_the_placed_panel_not_the_original_side() {
        let p = |x, y| gpui::point(gpui::px(x), gpui::px(y));
        let window = OverlayBounds {
            x: 118.,
            y: 20.,
            width: 140.,
            height: 70.,
        };
        let origin = p(107., 50.);
        assert!(toward_card(origin, p(113., 59.), window));
        assert!(!toward_card(origin, p(113., 100.), window));
        assert!(!toward_card(origin, p(100., 52.), window));
        assert!(!toward_card(origin, p(261., 52.), window));
    }
}
