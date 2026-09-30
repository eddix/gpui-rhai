//! Shared foreground ownership and lifecycle for native pointer gestures.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use gpui::{
    App, DispatchPhase, EntityId, Hitbox, MouseButton, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    ScrollHandle, Window, point, px,
};

use crate::{GeometryBounds, UiValue};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct InteractionOwner {
    view: String,
    retained: Option<crate::NodeId>,
    key: String,
}

impl InteractionOwner {
    pub(crate) fn new(view: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            view: view.into(),
            retained: None,
            key: key.into(),
        }
    }

    pub(crate) fn with_retained(mut self, retained: crate::NodeId) -> Self {
        self.retained = Some(retained);
        self
    }

    fn belongs_to(&self, view: &str) -> bool {
        self.view == view
    }

    pub(crate) fn child(&self, key: impl AsRef<str>) -> Self {
        Self {
            view: self.view.clone(),
            retained: self.retained,
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
    #[cfg(feature = "charts")]
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

    #[cfg(feature = "charts")]
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
    #[cfg(feature = "charts")]
    previous: Point<Pixels>,
    current: Point<Pixels>,
    phase: GesturePhase,
    moved: bool,
    threshold_squared: f64,
}

impl GestureSession {
    fn new(start: Point<Pixels>) -> Self {
        Self {
            start,
            #[cfg(feature = "charts")]
            previous: start,
            current: start,
            phase: GesturePhase::Armed,
            moved: false,
            threshold_squared: 0.0,
        }
    }

    fn update(&mut self, position: Point<Pixels>) -> GestureUpdate {
        #[cfg(feature = "charts")]
        {
            self.previous = self.current;
        }
        self.current = position;
        if self.phase == GesturePhase::Armed {
            let dx = f64::from(position.x - self.start.x);
            let dy = f64::from(position.y - self.start.y);
            if dx.mul_add(dx, dy * dy) > self.threshold_squared {
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
            #[cfg(feature = "charts")]
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

    pub(crate) fn with_threshold(mut self, threshold: f64) -> Self {
        self.session.threshold_squared = threshold.max(0.0).powi(2);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum DragOperation {
    Copy,
    Move,
}

impl DragOperation {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "copy" => Some(Self::Copy),
            "move" => Some(Self::Move),
            _ => None,
        }
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Move => "move",
        }
    }
}

#[derive(Clone)]
pub(crate) struct ApplicationDragSpec {
    source: InteractionOwner,
    source_id: String,
    payload_type: String,
    payload: UiValue,
    operation: DragOperation,
    notify: EntityId,
    collection: Option<String>,
    source_index: Option<usize>,
}

impl ApplicationDragSpec {
    pub(crate) fn new(
        source: InteractionOwner,
        source_id: String,
        payload_type: String,
        payload: UiValue,
        operation: DragOperation,
        notify: EntityId,
    ) -> Self {
        Self {
            source,
            source_id,
            payload_type,
            payload,
            operation,
            notify,
            collection: None,
            source_index: None,
        }
    }

    pub(crate) fn with_collection(
        mut self,
        collection: String,
        source_index: Option<usize>,
    ) -> Self {
        self.collection = Some(collection);
        self.source_index = source_index;
        self
    }

    pub(crate) fn source_id(&self) -> &str {
        &self.source_id
    }

    pub(crate) fn payload_type(&self) -> &str {
        &self.payload_type
    }

    pub(crate) fn payload(&self) -> &UiValue {
        &self.payload
    }

    pub(crate) const fn operation(&self) -> DragOperation {
        self.operation
    }

    pub(crate) const fn notify(&self) -> EntityId {
        self.notify
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ApplicationDropResult {
    pub accepted: bool,
    pub target_id: Option<String>,
    pub operation: DragOperation,
}

type ApplicationDragEndHandler = dyn Fn(ApplicationDropResult, bool, &mut Window, &mut App);

pub(crate) fn application_drag_gesture(
    coordinator: WindowInteractionCoordinator,
    start: Point<Pixels>,
    notify: EntityId,
    spec: ApplicationDragSpec,
    threshold: f64,
    on_end: impl Fn(ApplicationDropResult, bool, &mut Window, &mut App) + 'static,
) -> NativeGesture {
    let owner = spec.source.clone();
    let started = Rc::new(std::cell::Cell::new(false));
    let update_started = Rc::clone(&started);
    let update_coordinator = coordinator.clone();
    let update_spec = spec;
    let update = move |gesture: GestureUpdate, window: &mut Window, cx: &mut App| {
        if gesture.moved() && !update_started.replace(true) {
            update_coordinator.start_app_drag(update_spec.clone(), gesture.current(), window, cx);
        }
        if update_started.get() {
            update_coordinator.update_app_drag(gesture.current(), window, cx);
        }
        InteractionFlow::Continue
    };
    let on_end: Rc<ApplicationDragEndHandler> = Rc::new(on_end);
    let finish_started = Rc::clone(&started);
    let finish_coordinator = coordinator.clone();
    let finish_handler = Rc::clone(&on_end);
    let finish = move |gesture: GestureUpdate, window: &mut Window, cx: &mut App| {
        if !finish_started.get() {
            return;
        }
        if let Some(result) = finish_coordinator.finish_app_drag(gesture.current(), window, cx) {
            finish_handler(result, false, window, cx);
        }
    };
    let cancel_started = Rc::clone(&started);
    let cancel_coordinator = coordinator;
    let cancel = move |window: &mut Window, cx: &mut App| {
        if !cancel_started.get() {
            return;
        }
        if let Some(result) = cancel_coordinator.cancel_app_drag(cx) {
            on_end(result, true, window, cx);
        }
    };
    NativeGesture::new(owner, start, notify, update, finish, cancel).with_threshold(threshold)
}

type DropCommitHandler = dyn Fn(&ApplicationDragSpec, Point<Pixels>, &mut Window, &mut App);

#[derive(Clone)]
pub(crate) struct DropTargetRegistration {
    owner: InteractionOwner,
    target_id: String,
    bounds: GeometryBounds,
    payload_types: BTreeSet<String>,
    operations: BTreeSet<DragOperation>,
    priority: i64,
    hitbox: Hitbox,
    paint_order: u64,
    scroll_handles: Vec<ScrollHandle>,
    notify: EntityId,
    commit: Rc<DropCommitHandler>,
}

impl DropTargetRegistration {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        owner: InteractionOwner,
        target_id: String,
        bounds: GeometryBounds,
        payload_types: BTreeSet<String>,
        operations: BTreeSet<DragOperation>,
        priority: i64,
        hitbox: Hitbox,
        scroll_handles: Vec<ScrollHandle>,
        notify: EntityId,
        commit: impl Fn(&ApplicationDragSpec, Point<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            owner,
            target_id,
            bounds,
            payload_types,
            operations,
            priority,
            hitbox,
            paint_order: 0,
            scroll_handles,
            notify,
            commit: Rc::new(commit),
        }
    }

    fn accepts(&self, drag: &ApplicationDragSpec) -> bool {
        self.payload_types.contains(&drag.payload_type)
            && self.operations.contains(&drag.operation)
            && self.owner != drag.source
    }
}

#[derive(Clone)]
struct ActiveApplicationDrag {
    spec: ApplicationDragSpec,
    position: Point<Pixels>,
    target: Option<InteractionOwner>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DropTargetState {
    Idle,
    Eligible,
    Active,
    Invalid,
}

#[derive(Clone, Default)]
pub(crate) struct WindowInteractionCoordinator(Rc<RefCell<InteractionState>>);

#[derive(Default)]
struct InteractionState {
    active: Option<NativeGesture>,
    presented: BTreeSet<InteractionOwner>,
    captured_move: BTreeMap<String, Rc<CapturedMoveHandler>>,
    captured_up: BTreeMap<String, Rc<CapturedUpHandler>>,
    drag: Option<ActiveApplicationDrag>,
    drop_targets: BTreeMap<InteractionOwner, DropTargetRegistration>,
    next_drop_order: u64,
}

impl WindowInteractionCoordinator {
    pub(crate) fn begin_frame(&self) {
        let mut state = self.0.borrow_mut();
        state.presented.clear();
        state.captured_move.clear();
        state.captured_up.clear();
        state.drop_targets.clear();
        state.next_drop_order = 0;
    }

    pub(crate) fn set_pointer_routes(
        &self,
        view: impl Into<String>,
        move_handler: impl Fn(&MouseMoveEvent, &mut Window, &mut App) -> bool + 'static,
        up_handler: impl Fn(&MouseUpEvent, &mut Window, &mut App) -> bool + 'static,
    ) {
        let mut state = self.0.borrow_mut();
        let view = view.into();
        state
            .captured_move
            .insert(view.clone(), Rc::new(move_handler));
        state.captured_up.insert(view, Rc::new(up_handler));
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
        let position = self.0.borrow().drag.as_ref().map(|drag| drag.position);
        if let Some(position) = position {
            self.update_app_drag(position, window, cx);
        }
    }

    pub(crate) fn register_drop_target(&self, mut target: DropTargetRegistration) {
        let mut state = self.0.borrow_mut();
        target.paint_order = state.next_drop_order;
        state.next_drop_order = state.next_drop_order.saturating_add(1);
        state.drop_targets.insert(target.owner.clone(), target);
    }

    pub(crate) fn start_app_drag(
        &self,
        spec: ApplicationDragSpec,
        position: Point<Pixels>,
        window: &Window,
        cx: &mut App,
    ) {
        self.clear_app_drag(cx);
        let notify = spec.notify;
        self.0.borrow_mut().drag = Some(ActiveApplicationDrag {
            spec,
            position,
            target: None,
        });
        cx.notify(notify);
        self.update_app_drag(position, window, cx);
    }

    pub(crate) fn update_app_drag(&self, position: Point<Pixels>, window: &Window, cx: &mut App) {
        let (old_target, new_target, target, mut notifications) = {
            let mut state = self.0.borrow_mut();
            let Some(drag) = state.drag.as_ref() else {
                return;
            };
            let next = resolve_drop_target(drag, &state.drop_targets, position, window);
            let old = drag.target.clone();
            let mut notifications = BTreeSet::new();
            if old != next {
                if let Some(owner) = old.as_ref().and_then(|owner| state.drop_targets.get(owner)) {
                    notifications.insert(owner.notify);
                }
                if let Some(owner) = next
                    .as_ref()
                    .and_then(|owner| state.drop_targets.get(owner))
                {
                    notifications.insert(owner.notify);
                }
            }
            notifications.insert(drag.spec.notify);
            let drag = state.drag.as_mut().expect("active drag was checked");
            drag.position = position;
            drag.target.clone_from(&next);
            let target = next
                .as_ref()
                .and_then(|owner| state.drop_targets.get(owner))
                .cloned();
            (old, next, target, notifications)
        };
        let scrolled = target
            .as_ref()
            .is_some_and(|target| auto_scroll_drop_target(target, position));
        if scrolled && let Some(target) = target {
            notifications.insert(target.notify);
        }
        if old_target != new_target || scrolled {
            for notify in notifications {
                cx.notify(notify);
            }
        }
    }

    pub(crate) fn finish_app_drag(
        &self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<ApplicationDropResult> {
        self.update_app_drag(position, window, cx);
        let (drag, target) = {
            let mut state = self.0.borrow_mut();
            let drag = state.drag.take()?;
            let target = drag
                .target
                .as_ref()
                .and_then(|owner| state.drop_targets.get(owner))
                .cloned();
            (drag, target)
        };
        cx.notify(drag.spec.notify);
        if let Some(target) = target {
            cx.notify(target.notify);
            (target.commit)(&drag.spec, position, window, cx);
            Some(ApplicationDropResult {
                accepted: true,
                target_id: Some(target.target_id),
                operation: drag.spec.operation,
            })
        } else {
            Some(ApplicationDropResult {
                accepted: false,
                target_id: None,
                operation: drag.spec.operation,
            })
        }
    }

    pub(crate) fn perform_keyboard_drop(
        &self,
        spec: &ApplicationDragSpec,
        target_id: &str,
        window: &mut Window,
        cx: &mut App,
    ) -> ApplicationDropResult {
        let target = {
            let state = self.0.borrow();
            state
                .drop_targets
                .values()
                .filter(|target| target.target_id == target_id && target.accepts(spec))
                .max_by(|left, right| compare_drop_targets(left, right))
                .cloned()
        };
        cx.notify(spec.notify);
        if let Some(target) = target {
            let position = point(
                px(f64_to_f32(target.bounds.x + target.bounds.width / 2.0)),
                px(f64_to_f32(target.bounds.y + target.bounds.height / 2.0)),
            );
            cx.notify(target.notify);
            (target.commit)(spec, position, window, cx);
            ApplicationDropResult {
                accepted: true,
                target_id: Some(target.target_id),
                operation: spec.operation,
            }
        } else {
            ApplicationDropResult {
                accepted: false,
                target_id: None,
                operation: spec.operation,
            }
        }
    }

    pub(crate) fn cancel_app_drag(&self, cx: &mut App) -> Option<ApplicationDropResult> {
        let drag = self.clear_app_drag(cx)?;
        Some(ApplicationDropResult {
            accepted: false,
            target_id: None,
            operation: drag.spec.operation,
        })
    }

    fn clear_app_drag(&self, cx: &mut App) -> Option<ActiveApplicationDrag> {
        let (drag, target_notify) = {
            let mut state = self.0.borrow_mut();
            let drag = state.drag.take()?;
            let target_notify = drag
                .target
                .as_ref()
                .and_then(|owner| state.drop_targets.get(owner))
                .map(|target| target.notify);
            (drag, target_notify)
        };
        cx.notify(drag.spec.notify);
        if let Some(notify) = target_notify {
            cx.notify(notify);
        }
        Some(drag)
    }

    pub(crate) fn app_drag_source_active(&self, owner: &InteractionOwner) -> bool {
        self.0
            .borrow()
            .drag
            .as_ref()
            .is_some_and(|drag| drag.spec.source == *owner)
    }

    pub(crate) fn app_drag_pin(&self, collection: &str) -> Option<(String, usize)> {
        self.0.borrow().drag.as_ref().and_then(|drag| {
            (drag.spec.collection.as_deref() == Some(collection))
                .then(|| {
                    drag.spec
                        .source_index
                        .map(|index| (drag.spec.source_id.clone(), index))
                })
                .flatten()
        })
    }

    pub(crate) fn drop_target_state(&self, owner: &InteractionOwner) -> DropTargetState {
        let state = self.0.borrow();
        let Some(drag) = state.drag.as_ref() else {
            return DropTargetState::Idle;
        };
        let Some(target) = state.drop_targets.get(owner) else {
            return DropTargetState::Idle;
        };
        if !target.accepts(&drag.spec) {
            return DropTargetState::Invalid;
        }
        if drag.target.as_ref() == Some(owner) {
            DropTargetState::Active
        } else {
            DropTargetState::Eligible
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
        let active = {
            let mut state = self.0.borrow_mut();
            if state
                .active
                .as_ref()
                .is_some_and(|active| active.owner.belongs_to(view))
            {
                state.active.take()
            } else {
                None
            }
        };
        let Some(mut active) = active else {
            return false;
        };
        // The dispatcher owns a direct runtime signal fallback for this
        // lifecycle phase, so cleanup is complete before suspension commits.
        active.session.cancel();
        (active.cancel)(window, cx);
        true
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
        let drag_owned = self
            .0
            .borrow()
            .drag
            .as_ref()
            .is_some_and(|drag| drag.spec.source.belongs_to(view));
        if drag_owned {
            self.0.borrow_mut().drag = None;
        }
        let mut state = self.0.borrow_mut();
        state.captured_move.remove(view);
        state.captured_up.remove(view);
        state
            .drop_targets
            .retain(|owner, _| !owner.belongs_to(view));
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
            let routes = move_coordinator
                .0
                .borrow()
                .captured_move
                .values()
                .cloned()
                .collect::<Vec<_>>();
            if routes.into_iter().any(|route| route(event, window, cx)) {
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
                let routes = up_coordinator
                    .0
                    .borrow()
                    .captured_up
                    .values()
                    .cloned()
                    .collect::<Vec<_>>();
                if routes.into_iter().any(|route| route(event, window, cx)) {
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

fn resolve_drop_target(
    drag: &ActiveApplicationDrag,
    targets: &BTreeMap<InteractionOwner, DropTargetRegistration>,
    position: Point<Pixels>,
    window: &Window,
) -> Option<InteractionOwner> {
    targets
        .values()
        .filter(|target| {
            target.accepts(&drag.spec) && target.hitbox.is_hovered_at(position, window)
        })
        .max_by(|left, right| compare_drop_targets(left, right))
        .map(|target| target.owner.clone())
}

fn compare_drop_targets(
    left: &DropTargetRegistration,
    right: &DropTargetRegistration,
) -> std::cmp::Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| {
            let left_area = left.bounds.width * left.bounds.height;
            let right_area = right.bounds.width * right.bounds.height;
            right_area
                .partial_cmp(&left_area)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| left.paint_order.cmp(&right.paint_order))
}

fn auto_scroll_drop_target(target: &DropTargetRegistration, position: Point<Pixels>) -> bool {
    target.scroll_handles.iter().any(|handle| {
        let bounds = handle.bounds();
        if !bounds.contains(&position) {
            return false;
        }
        let current = handle.offset();
        let maximum = handle.max_offset();
        let margin_x = (bounds.size.width / 4.0).min(px(28.0));
        let margin_y = (bounds.size.height / 4.0).min(px(28.0));
        let step = px(12.0);
        let delta_x = if position.x < bounds.left() + margin_x {
            step
        } else if position.x > bounds.right() - margin_x {
            -step
        } else {
            px(0.0)
        };
        let delta_y = if position.y < bounds.top() + margin_y {
            step
        } else if position.y > bounds.bottom() - margin_y {
            -step
        } else {
            px(0.0)
        };
        let next = point(
            (current.x + delta_x).clamp(-maximum.x, px(0.0)),
            (current.y + delta_y).clamp(-maximum.y, px(0.0)),
        );
        if next == current {
            false
        } else {
            handle.set_offset(next);
            true
        }
    })
}

fn f64_to_f32(value: f64) -> f32 {
    value.to_string().parse().unwrap_or_else(|_| {
        if value.is_sign_negative() {
            f32::MIN
        } else {
            f32::MAX
        }
    })
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
