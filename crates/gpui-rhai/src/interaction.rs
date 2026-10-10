//! Shared foreground ownership and lifecycle for native pointer gestures.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    App, DispatchPhase, EntityId, Hitbox, ListState, MouseButton, MouseMoveEvent, MouseUpEvent,
    Pixels, Point, ScrollHandle, Window, WindowId, point, px,
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
type AuxiliaryCancelHandler = dyn Fn(&mut Window, &mut App);

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
    source_snapshot: Option<UiValue>,
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
            source_snapshot: None,
        }
    }

    pub(crate) fn with_collection(
        mut self,
        collection: String,
        source_index: Option<usize>,
        source_snapshot: Option<UiValue>,
    ) -> Self {
        self.collection = Some(collection);
        self.source_index = source_index;
        self.source_snapshot = source_snapshot;
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

/// Runs when a press is released before it became a drag.
pub(crate) type TapHandler = Rc<dyn Fn(&mut Window, &mut App)>;

pub(crate) fn application_drag_gesture(
    coordinator: WindowInteractionCoordinator,
    start: Point<Pixels>,
    notify: EntityId,
    spec: ApplicationDragSpec,
    threshold: f64,
    on_end: impl Fn(ApplicationDropResult, bool, &mut Window, &mut App) + 'static,
    on_tap: Option<TapHandler>,
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
        // A release before the threshold was crossed is a tap, not a drag.
        if !finish_started.get() {
            if let Some(tap) = &on_tap {
                tap(window, cx);
            }
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
    scroll_destination: Option<(String, crate::NodeId)>,
}

#[derive(Clone)]
struct AuxiliaryInteraction {
    owner: InteractionOwner,
    window: WindowId,
    notify: EntityId,
    cancel: Rc<AuxiliaryCancelHandler>,
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
    active_window: Option<WindowId>,
    presented: BTreeSet<InteractionOwner>,
    logical_sources: BTreeSet<InteractionOwner>,
    captured_move: BTreeMap<String, Rc<CapturedMoveHandler>>,
    captured_up: BTreeMap<String, Rc<CapturedUpHandler>>,
    drag: Option<ActiveApplicationDrag>,
    drag_window: Option<WindowId>,
    auxiliary: Option<AuxiliaryInteraction>,
    drop_targets: BTreeMap<InteractionOwner, DropTargetRegistration>,
    next_drop_order: u64,
    virtual_scrolls: BTreeMap<String, VirtualScrollTarget>,
    parents: BTreeMap<(String, crate::NodeId), Option<crate::NodeId>>,
    scroll_tick_queued: bool,
}

#[derive(Clone)]
struct VirtualScrollTarget {
    view: String,
    node: Option<crate::NodeId>,
    state: ListState,
    notify: EntityId,
    hitbox: Hitbox,
}

impl WindowInteractionCoordinator {
    pub(crate) fn set_retained_tree(&self, view: &str, tree: &crate::RetainedUiTree) {
        let mut state = self.0.borrow_mut();
        state.parents.retain(|(owner, _), _| owner != view);
        state.parents.extend(
            tree.nodes()
                .map(|node| ((view.to_owned(), node.id()), node.parent())),
        );
    }
    pub(crate) fn begin_frame(&self) {
        let mut state = self.0.borrow_mut();
        state.presented.clear();
        state.logical_sources.clear();
        state.captured_move.clear();
        state.captured_up.clear();
        state.drop_targets.clear();
        state.next_drop_order = 0;
        state.virtual_scrolls.clear();
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
        let expired_session = {
            let mut state = self.0.borrow_mut();
            let should_cancel = state.active.as_ref().is_some_and(|active| {
                !state.presented.contains(&active.owner)
                    && !state.logical_sources.contains(&active.owner)
            });
            let expired = should_cancel.then(|| state.active.take()).flatten();
            if expired.is_some() {
                state.active_window = None;
            }
            expired
        };
        if let Some(active) = expired_session {
            window.defer(cx, move |window, cx| cancel_active(active, window, cx));
        }
        let schedule_tick = {
            let mut state = self.0.borrow_mut();
            let schedule = state.drag.is_some() && !state.scroll_tick_queued;
            state.scroll_tick_queued |= schedule;
            schedule
        };
        if schedule_tick {
            // GPUI hit testing reads the presented frame. Delay the stationary
            // sample until the current paint has published its fresh hitboxes.
            let coordinator = self.clone();
            window
                .spawn(cx, async move |cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(16))
                        .await;
                    coordinator.0.borrow_mut().scroll_tick_queued = false;
                    let _ = cx.update(|window, cx| {
                        let position = {
                            let state = coordinator.0.borrow();
                            state.drag.as_ref().map(|drag| drag.position)
                        };
                        if let Some(position) = position {
                            coordinator.update_app_drag(position, window, cx);
                        }
                    });
                })
                .detach();
        }
    }

    pub(crate) fn register_drop_target(&self, mut target: DropTargetRegistration) {
        let mut state = self.0.borrow_mut();
        target.paint_order = state.next_drop_order;
        state.next_drop_order = state.next_drop_order.saturating_add(1);
        state.drop_targets.insert(target.owner.clone(), target);
    }

    pub(crate) fn register_virtual_scroll(
        &self,
        collection: String,
        view: String,
        node: Option<crate::NodeId>,
        state: ListState,
        notify: EntityId,
        hitbox: Hitbox,
    ) {
        self.0.borrow_mut().virtual_scrolls.insert(
            collection,
            VirtualScrollTarget {
                view,
                node,
                state,
                notify,
                hitbox,
            },
        );
    }

    pub(crate) fn start_app_drag(
        &self,
        spec: ApplicationDragSpec,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.clear_app_drag(cx);
        let notify = spec.notify;
        self.0.borrow_mut().drag = Some(ActiveApplicationDrag {
            spec,
            position,
            target: None,
            scroll_destination: None,
        });
        self.0.borrow_mut().drag_window = Some(window.window_handle().window_id());
        cx.notify(notify);
        self.update_app_drag(position, window, cx);
    }

    pub(crate) fn update_app_drag(
        &self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let (old_target, new_target, target, virtual_scroll, mut notifications) = {
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
            let target = next
                .as_ref()
                .and_then(|owner| state.drop_targets.get(owner))
                .cloned();
            let virtual_scroll = resolve_virtual_scroll_target(
                target.as_ref(),
                &state.virtual_scrolls,
                &state.parents,
                position,
            )
            .or_else(|| {
                if target.is_some() {
                    return None;
                }
                let (view, node) = drag.scroll_destination.as_ref()?;
                state
                    .virtual_scrolls
                    .values()
                    .find(|scroll| {
                        scroll.view == *view
                            && scroll.node == Some(*node)
                            && scroll.hitbox.is_hovered_at(position, window)
                    })
                    .cloned()
            });
            let drag = state.drag.as_mut().expect("active drag was checked");
            drag.scroll_destination = virtual_scroll
                .as_ref()
                .and_then(|scroll| scroll.node.map(|node| (scroll.view.clone(), node)));
            drag.position = position;
            drag.target.clone_from(&next);
            (old, next, target, virtual_scroll, notifications)
        };
        let scrolled = target
            .as_ref()
            .is_some_and(|target| auto_scroll_drop_target(target, position));
        if scrolled && let Some(target) = target {
            notifications.insert(target.notify);
        }
        let virtual_scrolled = virtual_scroll
            .as_ref()
            .is_some_and(|target| auto_scroll_virtual_target(target, position));
        if virtual_scrolled && let Some(target) = virtual_scroll {
            notifications.insert(target.notify);
            window.refresh();
        }
        if old_target != new_target || scrolled || virtual_scrolled {
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
            state.drag_window = None;
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
            state.drag_window = None;
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

    pub(crate) fn retain_virtual_drag_source(
        &self,
        collection: &str,
        index: usize,
        value: &UiValue,
    ) -> bool {
        let mut state = self.0.borrow_mut();
        let Some(drag) = state.drag.as_ref() else {
            return false;
        };
        if drag.spec.collection.as_deref() != Some(collection)
            || drag.spec.source_index != Some(index)
            || drag.spec.source_snapshot.as_ref() != Some(value)
        {
            return false;
        }
        let owner = drag.spec.source.clone();
        state.logical_sources.insert(owner);
        true
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
        self.cancel_auxiliary_in_window(window, cx);
        let notify = gesture.notify;
        let mut state = self.0.borrow_mut();
        state.active = Some(gesture);
        state.active_window = Some(window.window_handle().window_id());
        drop(state);
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
        if self.is_active(owner) {
            return self.cancel(window, cx);
        }
        let auxiliary = {
            let mut state = self.0.borrow_mut();
            if state
                .auxiliary
                .as_ref()
                .is_some_and(|active| active.owner == *owner)
            {
                state.auxiliary.take()
            } else {
                None
            }
        };
        if let Some(active) = auxiliary {
            (active.cancel)(window, cx);
            cx.notify(active.notify);
            true
        } else {
            false
        }
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
        if active.is_some() {
            self.0.borrow_mut().active_window = None;
        }
        let auxiliary = {
            let mut state = self.0.borrow_mut();
            if state
                .auxiliary
                .as_ref()
                .is_some_and(|active| active.owner.belongs_to(view))
            {
                state.auxiliary.take()
            } else {
                None
            }
        };
        let drag_owned = self
            .0
            .borrow()
            .drag
            .as_ref()
            .is_some_and(|drag| drag.spec.source.belongs_to(view));
        let mut consumed = false;
        if let Some(mut active) = active {
            active.session.cancel();
            (active.cancel)(window, cx);
            consumed = true;
        }
        if let Some(active) = auxiliary {
            (active.cancel)(window, cx);
            consumed = true;
        }
        if drag_owned && self.cancel_app_drag(cx).is_some() {
            consumed = true;
        }
        consumed
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
            self.0.borrow_mut().active_window = None;
        }
        let drag_owned = self
            .0
            .borrow()
            .drag
            .as_ref()
            .is_some_and(|drag| drag.spec.source.belongs_to(view));
        if drag_owned {
            self.0.borrow_mut().drag = None;
            self.0.borrow_mut().drag_window = None;
        }
        let mut state = self.0.borrow_mut();
        if state
            .auxiliary
            .as_ref()
            .is_some_and(|active| active.owner.belongs_to(view))
        {
            state.auxiliary = None;
        }
        state.captured_move.remove(view);
        state.captured_up.remove(view);
        state.parents.retain(|(owner, _), _| owner != view);
        state
            .drop_targets
            .retain(|owner, _| !owner.belongs_to(view));
    }

    pub(crate) fn cancel(&self, window: &mut Window, cx: &mut App) -> bool {
        let active = {
            let mut state = self.0.borrow_mut();
            let active = state.active.take();
            if active.is_some() {
                state.active_window = None;
            }
            active
        };
        let Some(active) = active else {
            return false;
        };
        cancel_active(active, window, cx);
        true
    }

    pub(crate) fn register_auxiliary(
        &self,
        owner: InteractionOwner,
        notify: EntityId,
        cancel: impl Fn(&mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut App,
    ) {
        let previous = {
            let mut state = self.0.borrow_mut();
            let same_owner = state
                .auxiliary
                .as_ref()
                .is_some_and(|active| active.owner == owner);
            let previous = (!same_owner).then(|| state.auxiliary.take()).flatten();
            state.auxiliary = Some(AuxiliaryInteraction {
                owner,
                window: window.window_handle().window_id(),
                notify,
                cancel: Rc::new(cancel),
            });
            previous
        };
        if let Some(previous) = previous {
            (previous.cancel)(window, cx);
            cx.notify(previous.notify);
        }
    }

    pub(crate) fn clear_auxiliary(&self, owner: &InteractionOwner) {
        let mut state = self.0.borrow_mut();
        if state
            .auxiliary
            .as_ref()
            .is_some_and(|active| active.owner == *owner)
        {
            state.auxiliary = None;
        }
    }

    fn cancel_auxiliary_in_window(&self, window: &mut Window, cx: &mut App) -> bool {
        let window_id = window.window_handle().window_id();
        let active = {
            let mut state = self.0.borrow_mut();
            if state
                .auxiliary
                .as_ref()
                .is_some_and(|active| active.window == window_id)
            {
                state.auxiliary.take()
            } else {
                None
            }
        };
        if let Some(active) = active {
            (active.cancel)(window, cx);
            cx.notify(active.notify);
            true
        } else {
            false
        }
    }

    pub(crate) fn cancel_window(&self, window: &mut Window, cx: &mut App) -> bool {
        let window_id = window.window_handle().window_id();
        let owns_pointer = self.0.borrow().active_window == Some(window_id);
        let owns_drag = self.0.borrow().drag_window == Some(window_id);
        let mut consumed = owns_pointer && self.cancel(window, cx);
        consumed |= self.cancel_auxiliary_in_window(window, cx);
        if owns_drag && self.cancel_app_drag(cx).is_some() {
            consumed = true;
        }
        consumed
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
            up_coordinator.0.borrow_mut().active_window = None;
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

fn resolve_virtual_scroll_target(
    active_target: Option<&DropTargetRegistration>,
    virtual_scrolls: &BTreeMap<String, VirtualScrollTarget>,
    parents: &BTreeMap<(String, crate::NodeId), Option<crate::NodeId>>,
    position: Point<Pixels>,
) -> Option<VirtualScrollTarget> {
    let target = active_target?;
    let view = &target.owner.view;
    let mut cursor = target.owner.retained;
    while let Some(node) = cursor {
        if let Some(scroll) = virtual_scrolls.values().find(|scroll| {
            scroll.view == *view
                && scroll.node == Some(node)
                && scroll.state.viewport_bounds().contains(&position)
        }) {
            return Some(scroll.clone());
        }
        cursor = parents.get(&(view.clone(), node)).copied().flatten();
    }
    None
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

fn auto_scroll_virtual_target(target: &VirtualScrollTarget, position: Point<Pixels>) -> bool {
    let bounds = target.state.viewport_bounds();
    if !bounds.contains(&position) {
        return false;
    }
    let margin = (bounds.size.height / 4.0).min(px(28.0));
    let distance = if position.y < bounds.top() + margin {
        px(-12.0)
    } else if position.y > bounds.bottom() - margin {
        px(12.0)
    } else {
        px(0.0)
    };
    if distance == px(0.0) {
        return false;
    }
    let before = target.state.logical_scroll_top();
    target.state.scroll_by(distance);
    let after = target.state.logical_scroll_top();
    after.item_ix != before.item_ix || after.offset_in_item != before.offset_in_item
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
