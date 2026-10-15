//! Shared themed overlay scrollbars for native and retained scrolling surfaces.
//!
//! The geometry and drag policy follow the public `ScrollHandle` contract. The
//! implementation is independent, with behavior informed by Longbridge's
//! Apache-2.0 gpui-component scrollbar.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    App, Axis, Background, Bounds, ContentMask, CursorStyle, DispatchPhase, Edges, Element,
    ElementId, GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId,
    MouseButton, MouseDownEvent, MouseMoveEvent, PaintQuad, Pixels, Point, Position, Rgba,
    ScrollHandle, ScrollWheelEvent, Style, Window, fill, point, px, relative, rgba, size,
};

use crate::{Rgba8, TextDirection, UiNode, UiValue};

/// Width of the pointer track. Hosts can reserve this much logical-end space
/// when row content must not appear beneath the overlay scrollbar.
pub const SCROLLBAR_TRACK_WIDTH: Pixels = px(12.0);
const TRACK_WIDTH: Pixels = SCROLLBAR_TRACK_WIDTH;
const THUMB_WIDTH: Pixels = px(6.0);
const THUMB_ACTIVE_WIDTH: Pixels = px(8.0);
const THUMB_INSET: Pixels = px(3.0);
const MIN_THUMB_LENGTH: Pixels = px(32.0);
const AUTO_IDLE: Duration = Duration::from_millis(1400);

/// A foreground-owned native scroll state, using GPUI's negative-offset
/// convention. The source remains authoritative: the scrollbar retains no
/// independent scroll position or follow policy.
pub trait ScrollbarSource: 'static {
    fn viewport(&self) -> Bounds<Pixels>;
    fn offset(&self) -> Point<Pixels>;
    fn max_offset(&self) -> Point<Pixels>;
    fn set_offset(&self, offset: Point<Pixels>);
    fn drag_started(&self) {}
    fn drag_ended(&self) {}
}

impl ScrollbarSource for ScrollHandle {
    fn viewport(&self) -> Bounds<Pixels> {
        self.bounds()
    }
    fn offset(&self) -> Point<Pixels> {
        self.offset()
    }
    fn max_offset(&self) -> Point<Pixels> {
        self.max_offset()
    }
    fn set_offset(&self, offset: Point<Pixels>) {
        self.set_offset(offset);
    }
}

impl ScrollbarSource for gpui::ListState {
    fn viewport(&self) -> Bounds<Pixels> {
        self.viewport_bounds()
    }
    fn offset(&self) -> Point<Pixels> {
        self.scroll_px_offset_for_scrollbar()
    }
    fn max_offset(&self) -> Point<Pixels> {
        self.max_offset_for_scrollbar()
    }
    fn set_offset(&self, offset: Point<Pixels>) {
        self.set_offset_from_scrollbar(offset);
    }
    fn drag_started(&self) {
        self.scrollbar_drag_started();
    }
    fn drag_ended(&self) {
        self.scrollbar_drag_ended();
    }
}

impl ScrollbarSource for gpui::UniformListScrollHandle {
    fn viewport(&self) -> Bounds<Pixels> {
        self.0.borrow().base_handle.bounds()
    }
    fn offset(&self) -> Point<Pixels> {
        self.0.borrow().base_handle.offset()
    }
    fn max_offset(&self) -> Point<Pixels> {
        self.0.borrow().base_handle.max_offset()
    }
    fn set_offset(&self, offset: Point<Pixels>) {
        let mut state = self.0.borrow_mut();
        state.deferred_scroll_to_item = None;
        state.base_handle.set_offset(offset);
    }
}

/// A themed scrollbar factory scoped to a Host surface or retained primitive.
/// Obtain it from [`crate::PrimitiveContext::scrollbars`] or
/// [`crate::ScriptViewHost::scrollbars`]. Render the viewport first and the
/// scrollbar last as siblings in a relative, clipped container.
#[derive(Clone)]
pub struct ScrollbarContext {
    owner: crate::interaction::InteractionOwner,
    interactions: crate::interaction::WindowInteractionCoordinator,
    direction: TextDirection,
    colors: (Rgba8, Rgba8, Rgba8),
    theme: crate::PrimitiveTheme,
    disabled: bool,
}

impl ScrollbarContext {
    pub(crate) fn new(
        owner: crate::interaction::InteractionOwner,
        interactions: crate::interaction::WindowInteractionCoordinator,
        theme: &crate::PrimitiveTheme,
    ) -> Self {
        let fallback = |name, legacy, rgba| {
            theme
                .color(name)
                .or_else(|| theme.color(legacy))
                .unwrap_or_else(|| Rgba8::from_rgba_hex(rgba))
        };
        Self {
            owner,
            interactions,
            direction: theme.direction(),
            colors: (
                Rgba8::from_rgba_hex(0),
                fallback("scrollbar.thumb", "text_muted", 0x7171_7a73),
                fallback("scrollbar.thumb_hover", "accent", 0x3b82_f6ff),
            ),
            theme: theme.clone(),
            disabled: theme.inherited_disabled(),
        }
    }

    /// Override the backgrounds of the three standard scrollbar parts. Empty
    /// styles retain the theme defaults; no new focus or keyboard owner is made.
    #[must_use]
    pub fn with_part_styles(
        mut self,
        track: &crate::Style,
        thumb: &crate::Style,
        thumb_hover: &crate::Style,
    ) -> Self {
        let resolve = |style: &crate::Style, fallback| {
            style
                .resolve(&crate::InteractionState::default())
                .background
                .as_ref()
                .and_then(|color| self.theme.resolve_color(color))
                .unwrap_or(fallback)
        };
        self.colors = (
            resolve(track, self.colors.0),
            resolve(thumb, self.colors.1),
            resolve(thumb_hover, self.colors.2),
        );
        self
    }

    /// Build a pointer-only overlay. `id` must be stable and unique within this
    /// scope. Wheel/keyboard input remains owned by the underlying viewport.
    #[must_use]
    pub fn element(
        &self,
        id: impl Into<ElementId>,
        source: impl ScrollbarSource,
        spec: ScrollbarSpec,
    ) -> gpui::AnyElement {
        let id = id.into();
        let mut scrollbar = ThemedScrollbar::new(
            id.clone(),
            self.owner.child(format!("{id:?}")),
            self.interactions.clone(),
            source,
            spec,
            self.direction,
            self.colors,
        );
        scrollbar.disabled = self.disabled;
        scrollbar.into_any_element()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScrollbarVisibility {
    Auto,
    Always,
    Hidden,
}

impl ScrollbarVisibility {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Hidden => "hidden",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "hidden" => Ok(Self::Hidden),
            _ => Err(format!(
                "scrollbar visibility must be auto, always, or hidden; got `{value}`"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScrollbarSpec {
    horizontal: ScrollbarVisibility,
    vertical: ScrollbarVisibility,
}

impl ScrollbarSpec {
    /// Parse per-axis visibility policies.
    ///
    /// # Errors
    ///
    /// Returns an error unless both values are `auto`, `always`, or `hidden`.
    pub fn new(horizontal: &str, vertical: &str) -> Result<Self, String> {
        Ok(Self {
            horizontal: ScrollbarVisibility::parse(horizontal)?,
            vertical: ScrollbarVisibility::parse(vertical)?,
        })
    }

    pub const fn horizontal(self) -> ScrollbarVisibility {
        self.horizontal
    }

    pub const fn vertical(self) -> ScrollbarVisibility {
        self.vertical
    }

    pub(crate) fn from_node(node: &UiNode) -> Option<Self> {
        let read = |name| match node.attributes().get(name) {
            Some(UiValue::String(value)) => ScrollbarVisibility::parse(value).ok(),
            _ => None,
        };
        Some(Self {
            horizontal: read("scrollbar_horizontal")?,
            vertical: read("scrollbar_vertical")?,
        })
    }

    fn visibility(self, axis: Axis) -> ScrollbarVisibility {
        match axis {
            Axis::Horizontal => self.horizontal,
            Axis::Vertical => self.vertical,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ScrollbarStateInner {
    hovered: bool,
    last_offset: Point<Pixels>,
    last_activity: Option<Instant>,
    idle_timer_scheduled: bool,
}

#[derive(Clone, Default)]
struct ScrollbarState(Rc<Cell<ScrollbarStateInner>>);

#[derive(Clone)]
struct AxisPrepaint {
    axis: Axis,
    track: Bounds<Pixels>,
    thumb_hitbox: Bounds<Pixels>,
    thumb_paint: Bounds<Pixels>,
    bar_hitbox: Hitbox,
    max_offset: Pixels,
    thumb_length: Pixels,
    visible: bool,
    dragged: bool,
}

pub(crate) struct ThemedScrollbar {
    id: ElementId,
    interaction_owner: crate::interaction::InteractionOwner,
    interactions: crate::interaction::WindowInteractionCoordinator,
    handle: Rc<dyn ScrollbarSource>,
    spec: ScrollbarSpec,
    direction: TextDirection,
    track: Rgba8,
    thumb: Rgba8,
    thumb_hover: Rgba8,
    disabled: bool,
}

impl ThemedScrollbar {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        interaction_owner: crate::interaction::InteractionOwner,
        interactions: crate::interaction::WindowInteractionCoordinator,
        handle: impl ScrollbarSource,
        spec: ScrollbarSpec,
        direction: TextDirection,
        colors: (Rgba8, Rgba8, Rgba8),
    ) -> Self {
        let (track, thumb, thumb_hover) = colors;
        Self {
            id: id.into(),
            interaction_owner,
            interactions,
            handle: Rc::new(handle),
            spec,
            direction,
            track,
            thumb,
            thumb_hover,
            disabled: false,
        }
    }
}

impl IntoElement for ThemedScrollbar {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

pub(crate) struct ScrollbarPrepaint {
    viewport: Bounds<Pixels>,
    state: ScrollbarState,
    axes: Vec<AxisPrepaint>,
}

impl Element for ThemedScrollbar {
    type RequestLayoutState = ();
    type PrepaintState = ScrollbarPrepaint;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            position: Position::Absolute,
            size: size(relative(1.0).into(), relative(1.0).into()),
            ..Style::default()
        };
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let handle_bounds = self.handle.viewport();
        let viewport = if handle_bounds.size.width > px(0.0) && handle_bounds.size.height > px(0.0)
        {
            handle_bounds
        } else {
            bounds
        };
        let state = window
            .use_state(cx, |_, _| ScrollbarState::default())
            .read(cx)
            .clone();
        let now = Instant::now();
        let offset = self.handle.offset();
        let mut inner = state.0.get();
        if inner.last_offset != offset {
            inner.last_offset = offset;
            inner.last_activity = Some(now);
        }
        state.0.set(inner);
        schedule_idle_refresh(&state, now, window, cx);

        let max = self.handle.max_offset();
        let mut axes = Vec::new();
        for axis in [Axis::Horizontal, Axis::Vertical] {
            let visibility = self.spec.visibility(axis);
            if visibility == ScrollbarVisibility::Hidden {
                continue;
            }
            let (container, max_offset, scroll_position) = match axis {
                Axis::Horizontal => (viewport.size.width, max.x, -offset.x),
                Axis::Vertical => (viewport.size.height, max.y, -offset.y),
            };
            if max_offset <= px(0.0) || container <= px(0.0) {
                continue;
            }
            let content = container + max_offset;
            let thumb_length = (container / content * container)
                .max(MIN_THUMB_LENGTH)
                .min(container);
            let travel = (container - thumb_length).max(px(0.0));
            let thumb_start = (scroll_position / max_offset * travel).clamp(px(0.0), travel);
            let track = axis_track(viewport, axis, self.direction);
            let thumb_hitbox = axis_thumb(track, axis, thumb_start, thumb_length, TRACK_WIDTH);
            let owner = axis_interaction_owner(&self.interaction_owner, axis);
            self.interactions.present(owner.clone());
            let dragged = self.interactions.is_active(&owner);
            let active_width = if dragged {
                THUMB_ACTIVE_WIDTH
            } else {
                THUMB_WIDTH
            };
            let thumb_paint = axis_thumb(track, axis, thumb_start, thumb_length, active_width);
            let bar_hitbox = window
                .with_content_mask(Some(ContentMask { bounds: viewport }), |w| {
                    w.insert_hitbox(track, HitboxBehavior::Normal)
                });
            let inner = state.0.get();
            let visible = visibility == ScrollbarVisibility::Always
                || inner.hovered
                || dragged
                || inner
                    .last_activity
                    .is_some_and(|last| now.saturating_duration_since(last) < AUTO_IDLE);
            axes.push(AxisPrepaint {
                axis,
                track,
                thumb_hitbox,
                thumb_paint,
                bar_hitbox,
                max_offset,
                thumb_length,
                visible,
                dragged,
            });
        }
        ScrollbarPrepaint {
            viewport,
            state,
            axes,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let view = window.current_view();
        let viewport = prepaint.viewport;
        let state = prepaint.state.clone();
        register_hover_listener(viewport, state.clone(), view, window);
        register_wheel_listener(
            viewport,
            self.handle.clone(),
            state.clone(),
            view,
            prepaint
                .axes
                .iter()
                .filter(|axis| axis.visible)
                .map(|axis| axis.bar_hitbox.clone())
                .collect(),
            self.disabled,
            window,
        );
        for axis in &prepaint.axes {
            if !axis.visible {
                continue;
            }
            let hovered = state.0.get().hovered || axis.dragged;
            let thumb = if hovered {
                self.thumb_hover
            } else {
                self.thumb
            };
            window.set_cursor_style(CursorStyle::PointingHand, &axis.bar_hitbox);
            window.with_content_mask(Some(ContentMask { bounds: viewport }), |window| {
                window.paint_quad(fill(axis.track, rgba(self.track.as_rgba_hex())));
                window.paint_quad(PaintQuad {
                    bounds: axis.thumb_paint,
                    corner_radii: px(4.0).into(),
                    background: Background::from(rgba(thumb.as_rgba_hex())),
                    border_widths: Edges::default(),
                    border_color: Rgba::default().into(),
                    border_style: gpui::BorderStyle::default(),
                });
            });
            if self.disabled {
                continue;
            }
            register_axis_listeners(
                axis,
                &self.handle,
                &state,
                view,
                &self.interaction_owner,
                &self.interactions,
                window,
            );
        }
        let _ = cx;
    }
}

fn axis_track(viewport: Bounds<Pixels>, axis: Axis, direction: TextDirection) -> Bounds<Pixels> {
    match axis {
        Axis::Horizontal => Bounds::new(
            point(viewport.origin.x, viewport.bottom() - TRACK_WIDTH),
            size(viewport.size.width, TRACK_WIDTH),
        ),
        Axis::Vertical => {
            let x = if direction == TextDirection::RightToLeft {
                viewport.origin.x
            } else {
                viewport.right() - TRACK_WIDTH
            };
            Bounds::new(
                point(x, viewport.origin.y),
                size(TRACK_WIDTH, viewport.size.height),
            )
        }
    }
}

fn axis_thumb(
    track: Bounds<Pixels>,
    axis: Axis,
    start: Pixels,
    length: Pixels,
    width: Pixels,
) -> Bounds<Pixels> {
    match axis {
        Axis::Horizontal => Bounds::new(
            point(
                track.origin.x + start + THUMB_INSET,
                track.bottom() - width - THUMB_INSET,
            ),
            size((length - THUMB_INSET * 2.0).max(px(1.0)), width),
        ),
        Axis::Vertical => Bounds::new(
            point(
                track.right() - width - THUMB_INSET,
                track.origin.y + start + THUMB_INSET,
            ),
            size(width, (length - THUMB_INSET * 2.0).max(px(1.0))),
        ),
    }
}

fn schedule_idle_refresh(state: &ScrollbarState, now: Instant, window: &mut Window, cx: &mut App) {
    let mut inner = state.0.get();
    let Some(last) = inner.last_activity else {
        return;
    };
    let elapsed = now.saturating_duration_since(last);
    if elapsed >= AUTO_IDLE || inner.idle_timer_scheduled {
        return;
    }
    inner.idle_timer_scheduled = true;
    state.0.set(inner);
    let state = state.clone();
    let view = window.current_view();
    window
        .spawn(cx, async move |cx| {
            cx.background_executor()
                .timer(AUTO_IDLE.saturating_sub(elapsed))
                .await;
            let mut inner = state.0.get();
            inner.idle_timer_scheduled = false;
            state.0.set(inner);
            cx.update(|_, cx| cx.notify(view)).ok();
        })
        .detach();
}

fn register_hover_listener(
    viewport: Bounds<Pixels>,
    state: ScrollbarState,
    view: gpui::EntityId,
    window: &mut Window,
) {
    window.on_mouse_event(move |event: &MouseMoveEvent, _, _, cx| {
        let hovered = viewport.contains(&event.position);
        let mut inner = state.0.get();
        if inner.hovered != hovered {
            inner.hovered = hovered;
            if hovered {
                inner.last_activity = Some(Instant::now());
            }
            state.0.set(inner);
            cx.notify(view);
        }
    });
}

fn register_wheel_listener(
    viewport: Bounds<Pixels>,
    handle: Rc<dyn ScrollbarSource>,
    state: ScrollbarState,
    view: gpui::EntityId,
    tracks: Vec<Hitbox>,
    disabled: bool,
    window: &mut Window,
) {
    window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
        if phase == DispatchPhase::Bubble && viewport.contains(&event.position) {
            if !disabled && tracks.iter().any(|track| track.is_hovered(window)) {
                let delta = event.delta.pixel_delta(px(16.0));
                let max = handle.max_offset();
                let offset = handle.offset() + delta;
                handle.set_offset(point(
                    offset.x.clamp(-max.x, px(0.0)),
                    offset.y.clamp(-max.y, px(0.0)),
                ));
                cx.stop_propagation();
            }
            let mut inner = state.0.get();
            inner.last_offset = handle.offset();
            inner.last_activity = Some(Instant::now());
            state.0.set(inner);
            cx.notify(view);
        }
    });
}

fn register_axis_listeners(
    axis: &AxisPrepaint,
    handle: &Rc<dyn ScrollbarSource>,
    state: &ScrollbarState,
    view: gpui::EntityId,
    interaction_owner: &crate::interaction::InteractionOwner,
    interactions: &crate::interaction::WindowInteractionCoordinator,
    window: &mut Window,
) {
    let axis = axis.clone();
    let handle = handle.clone();
    let down_handle = handle.clone();
    let down_state = state.clone();
    let owner = axis_interaction_owner(interaction_owner, axis.axis);
    let coordinator = interactions.clone();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble
            || event.button != MouseButton::Left
            || !axis.bar_hitbox.is_hovered(window)
        {
            return;
        }
        cx.stop_propagation();
        let mut inner = down_state.0.get();
        inner.last_activity = Some(Instant::now());
        if axis.thumb_hitbox.contains(&event.position) {
            let drag_position = event.position - axis.thumb_hitbox.origin;
            let update_axis = axis.clone();
            let update_handle = down_handle.clone();
            let update_state = down_state.clone();
            let update =
                move |gesture: crate::interaction::GestureUpdate, _: &mut Window, _: &mut App| {
                    let position = match update_axis.axis {
                        Axis::Horizontal => {
                            gesture.current().x - drag_position.x - update_axis.track.origin.x
                        }
                        Axis::Vertical => {
                            gesture.current().y - drag_position.y - update_axis.track.origin.y
                        }
                    };
                    let travel = match update_axis.axis {
                        Axis::Horizontal => update_axis.track.size.width - update_axis.thumb_length,
                        Axis::Vertical => update_axis.track.size.height - update_axis.thumb_length,
                    };
                    let percentage = (position / travel.max(px(1.0))).clamp(0.0, 1.0);
                    let offset = update_handle.offset();
                    match update_axis.axis {
                        Axis::Horizontal => update_handle
                            .set_offset(point(-update_axis.max_offset * percentage, offset.y)),
                        Axis::Vertical => update_handle
                            .set_offset(point(offset.x, -update_axis.max_offset * percentage)),
                    }
                    let mut inner = update_state.0.get();
                    inner.last_activity = Some(Instant::now());
                    update_state.0.set(inner);
                    crate::interaction::InteractionFlow::Continue
                };
            let finish_state = down_state.clone();
            let finish_handle = down_handle.clone();
            let finish =
                move |_: crate::interaction::GestureUpdate, _: &mut Window, _: &mut App| {
                    finish_handle.drag_ended();
                    let mut inner = finish_state.0.get();
                    inner.last_activity = Some(Instant::now());
                    finish_state.0.set(inner);
                };
            let cancel_state = down_state.clone();
            let cancel_handle = down_handle.clone();
            let cancel = move |_: &mut Window, _: &mut App| {
                cancel_handle.drag_ended();
                let mut inner = cancel_state.0.get();
                inner.last_activity = Some(Instant::now());
                cancel_state.0.set(inner);
            };
            coordinator.begin(
                crate::interaction::NativeGesture::new(
                    owner.clone(),
                    event.position,
                    view,
                    update,
                    finish,
                    cancel,
                ),
                window,
                cx,
            );
            // Begin first: replacing an older gesture invokes its end hook.
            down_handle.drag_started();
        } else {
            let position = match axis.axis {
                Axis::Horizontal => event.position.x - axis.track.origin.x,
                Axis::Vertical => event.position.y - axis.track.origin.y,
            };
            let travel = match axis.axis {
                Axis::Horizontal => axis.track.size.width - axis.thumb_length,
                Axis::Vertical => axis.track.size.height - axis.thumb_length,
            };
            let percentage =
                ((position - axis.thumb_length / 2.0) / travel.max(px(1.0))).clamp(0.0, 1.0);
            let offset = down_handle.offset();
            match axis.axis {
                Axis::Horizontal => {
                    down_handle.set_offset(point(-axis.max_offset * percentage, offset.y));
                }
                Axis::Vertical => {
                    down_handle.set_offset(point(offset.x, -axis.max_offset * percentage));
                }
            }
        }
        down_state.0.set(inner);
        cx.notify(view);
    });
}

fn axis_interaction_owner(
    owner: &crate::interaction::InteractionOwner,
    axis: Axis,
) -> crate::interaction::InteractionOwner {
    owner.child(match axis {
        Axis::Horizontal => "horizontal",
        Axis::Vertical => "vertical",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_contract_is_strict_and_round_trips() {
        let spec = ScrollbarSpec::new("auto", "always").unwrap();
        assert_eq!(spec.horizontal(), ScrollbarVisibility::Auto);
        assert_eq!(spec.vertical(), ScrollbarVisibility::Always);
        assert!(ScrollbarSpec::new("platform", "auto").is_err());
    }

    #[test]
    fn vertical_track_uses_the_logical_end_edge() {
        let viewport = Bounds::new(point(px(20.0), px(30.0)), size(px(200.0), px(100.0)));
        let ltr = axis_track(viewport, Axis::Vertical, TextDirection::LeftToRight);
        let rtl = axis_track(viewport, Axis::Vertical, TextDirection::RightToLeft);
        assert_eq!(ltr.origin.x, viewport.right() - TRACK_WIDTH);
        assert_eq!(rtl.origin.x, viewport.origin.x);
        assert_eq!(ltr.size, rtl.size);
    }
}
