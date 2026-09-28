//! Shared foreground ownership and lifecycle for native pointer gestures.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use gpui::{
    App, DispatchPhase, EntityId, MouseButton, MouseMoveEvent, MouseUpEvent, Pixels, Point, Window,
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct InteractionOwner {
    view: String,
    key: String,
}

impl InteractionOwner {
    pub(crate) fn new(view: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            view: view.into(),
            key: key.into(),
        }
    }

    fn belongs_to(&self, view: &str) -> bool {
        self.view == view
    }

    pub(crate) fn child(&self, key: impl AsRef<str>) -> Self {
        Self {
            view: self.view.clone(),
            key: format!("{}:{}", self.key, key.as_ref()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GesturePhase {
    Armed,
    Active,
    Finished,
    Cancelled,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GestureUpdate {
    start: Point<Pixels>,
    previous: Point<Pixels>,
    current: Point<Pixels>,
    moved: bool,
}

impl GestureUpdate {
    pub(crate) fn start(self) -> Point<Pixels> {
        self.start
    }

    pub(crate) fn current(self) -> Point<Pixels> {
        self.current
    }

    pub(crate) fn delta(self) -> (f64, f64) {
        (
            f64::from(self.current.x - self.start.x),
            f64::from(self.current.y - self.start.y),
        )
    }

    pub(crate) fn step(self) -> (f64, f64) {
        (
            f64::from(self.current.x - self.previous.x),
            f64::from(self.current.y - self.previous.y),
        )
    }

    pub(crate) fn moved(self) -> bool {
        self.moved
    }
}

#[derive(Clone, Debug)]
struct GestureSession {
    start: Point<Pixels>,
    previous: Point<Pixels>,
    current: Point<Pixels>,
    phase: GesturePhase,
    moved: bool,
}

impl GestureSession {
    fn new(start: Point<Pixels>) -> Self {
        Self {
            start,
            previous: start,
            current: start,
            phase: GesturePhase::Armed,
            moved: false,
        }
    }

    fn update(&mut self, position: Point<Pixels>) -> GestureUpdate {
        self.previous = self.current;
        self.current = position;
        if self.phase == GesturePhase::Armed {
            let dx = f64::from(position.x - self.start.x);
            let dy = f64::from(position.y - self.start.y);
            if dx.abs() > f64::EPSILON || dy.abs() > f64::EPSILON {
                self.phase = GesturePhase::Active;
                self.moved = true;
            }
        }
        self.snapshot()
    }

    fn finish(&mut self, position: Point<Pixels>) -> GestureUpdate {
        self.update(position);
        self.phase = GesturePhase::Finished;
        self.snapshot()
    }

    fn cancel(&mut self) {
        self.phase = GesturePhase::Cancelled;
    }

    fn snapshot(&self) -> GestureUpdate {
        GestureUpdate {
            start: self.start,
            previous: self.previous,
            current: self.current,
            moved: self.moved,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InteractionFlow {
    Continue,
    Cancel,
}

type UpdateHandler = dyn Fn(GestureUpdate, &mut Window, &mut App) -> InteractionFlow;
type FinishHandler = dyn Fn(GestureUpdate, &mut Window, &mut App);
type CancelHandler = dyn Fn(&mut Window, &mut App);
type CapturedMoveHandler = dyn Fn(&MouseMoveEvent, &mut Window, &mut App) -> bool;
type CapturedUpHandler = dyn Fn(&MouseUpEvent, &mut Window, &mut App) -> bool;

pub(crate) struct NativeGesture {
    owner: InteractionOwner,
    button: MouseButton,
    session: GestureSession,
    notify: EntityId,
    update: Rc<UpdateHandler>,
    finish: Rc<FinishHandler>,
    cancel: Rc<CancelHandler>,
}

impl NativeGesture {
    pub(crate) fn new(
        owner: InteractionOwner,
        start: Point<Pixels>,
        notify: EntityId,
        update: impl Fn(GestureUpdate, &mut Window, &mut App) -> InteractionFlow + 'static,
        finish: impl Fn(GestureUpdate, &mut Window, &mut App) + 'static,
        cancel: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            owner,
            button: MouseButton::Left,
            session: GestureSession::new(start),
            notify,
            update: Rc::new(update),
            finish: Rc::new(finish),
            cancel: Rc::new(cancel),
        }
    }

    pub(crate) fn with_button(mut self, button: MouseButton) -> Self {
        self.button = button;
        self
    }
}

#[derive(Clone, Default)]
pub(crate) struct WindowInteractionCoordinator(Rc<RefCell<InteractionState>>);

#[derive(Default)]
struct InteractionState {
    active: Option<NativeGesture>,
    presented: BTreeSet<InteractionOwner>,
    captured_move: Option<Rc<CapturedMoveHandler>>,
    captured_up: Option<Rc<CapturedUpHandler>>,
}

impl WindowInteractionCoordinator {
    pub(crate) fn begin_frame(&self) {
        let mut state = self.0.borrow_mut();
        state.presented.clear();
        state.captured_move = None;
        state.captured_up = None;
    }

    pub(crate) fn set_pointer_routes(
        &self,
        move_handler: impl Fn(&MouseMoveEvent, &mut Window, &mut App) -> bool + 'static,
        up_handler: impl Fn(&MouseUpEvent, &mut Window, &mut App) -> bool + 'static,
    ) {
        let mut state = self.0.borrow_mut();
        state.captured_move = Some(Rc::new(move_handler));
        state.captured_up = Some(Rc::new(up_handler));
    }

    pub(crate) fn present(&self, owner: InteractionOwner) {
        self.0.borrow_mut().presented.insert(owner);
    }

    pub(crate) fn finish_frame(&self, window: &mut Window, cx: &mut App) {
        let stale = {
            let mut state = self.0.borrow_mut();
            let should_cancel = state
                .active
                .as_ref()
                .is_some_and(|active| !state.presented.contains(&active.owner));
            should_cancel.then(|| state.active.take()).flatten()
        };
        if let Some(active) = stale {
            window.defer(cx, move |window, cx| cancel_active(active, window, cx));
        }
    }

    pub(crate) fn begin(&self, gesture: NativeGesture, window: &mut Window, cx: &mut App) {
        self.cancel(window, cx);
        let notify = gesture.notify;
        self.0.borrow_mut().active = Some(gesture);
        cx.notify(notify);
    }

    pub(crate) fn is_active(&self, owner: &InteractionOwner) -> bool {
        self.0
            .borrow()
            .active
            .as_ref()
            .is_some_and(|active| active.owner == *owner)
    }

    pub(crate) fn cancel_owner(
        &self,
        owner: &InteractionOwner,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        if !self.is_active(owner) {
            return false;
        }
        self.cancel(window, cx)
    }

    pub(crate) fn cancel_view(&self, view: &str, window: &mut Window, cx: &mut App) -> bool {
        let owned = self
            .0
            .borrow()
            .active
            .as_ref()
            .is_some_and(|active| active.owner.belongs_to(view));
        if owned {
            self.cancel(window, cx)
        } else {
            false
        }
    }

    pub(crate) fn discard_view(&self, view: &str) {
        let owned = self
            .0
            .borrow()
            .active
            .as_ref()
            .is_some_and(|active| active.owner.belongs_to(view));
        if owned {
            self.0.borrow_mut().active = None;
        }
    }

    pub(crate) fn cancel(&self, window: &mut Window, cx: &mut App) -> bool {
        let Some(active) = self.0.borrow_mut().active.take() else {
            return false;
        };
        cancel_active(active, window, cx);
        true
    }

    pub(crate) fn install(&self, window: &mut Window) {
        let move_coordinator = self.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase != DispatchPhase::Capture {
                return;
            }
            let native = {
                let mut state = move_coordinator.0.borrow_mut();
                state.active.as_mut().map(|active| {
                    (
                        active.session.update(event.position),
                        Rc::clone(&active.update),
                        active.notify,
                    )
                })
            };
            if let Some((update, handler, notify)) = native {
                if !event.dragging() || handler(update, window, cx) == InteractionFlow::Cancel {
                    move_coordinator.cancel(window, cx);
                } else {
                    cx.notify(notify);
                }
                cx.stop_propagation();
                return;
            }
            let route = move_coordinator.0.borrow().captured_move.clone();
            if route.is_some_and(|route| route(event, window, cx)) {
                cx.stop_propagation();
            }
        });

        let up_coordinator = self.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
            if phase != DispatchPhase::Capture {
                return;
            }
            let matches_active = up_coordinator
                .0
                .borrow()
                .active
                .as_ref()
                .is_some_and(|active| active.button == event.button);
            if !matches_active {
                let route = up_coordinator.0.borrow().captured_up.clone();
                if route.is_some_and(|route| route(event, window, cx)) {
                    cx.stop_propagation();
                }
                return;
            }
            let Some(mut active) = up_coordinator.0.borrow_mut().active.take() else {
                return;
            };
            let update = active.session.finish(event.position);
            (active.finish)(update, window, cx);
            cx.notify(active.notify);
            cx.stop_propagation();
        });
    }
}

fn cancel_active(mut active: NativeGesture, window: &mut Window, cx: &mut App) {
    active.session.cancel();
    (active.cancel)(window, cx);
    cx.notify(active.notify);
}

#[cfg(test)]
mod tests {
    use gpui::{point, px};

    use super::GestureSession;

    #[test]
    fn gesture_session_owns_activation_and_total_delta() {
        let mut session = GestureSession::new(point(px(10.0), px(20.0)));
        let unchanged = session.update(point(px(10.0), px(20.0)));
        assert!(!unchanged.moved());
        assert!(!session.finish(point(px(10.0), px(20.0))).moved());

        let mut session = GestureSession::new(point(px(10.0), px(20.0)));
        let active = session.update(point(px(15.0), px(24.0)));
        assert!(active.moved());
        assert_eq!(active.delta(), (5.0, 4.0));

        let finished = session.finish(point(px(16.0), px(25.0)));
        assert!(finished.moved());
        assert_eq!(finished.delta(), (6.0, 5.0));
    }
}
