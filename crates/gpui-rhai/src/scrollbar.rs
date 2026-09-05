//! Generic themed overlay scrollbars for retained scrollable nodes.
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
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, Position, Rgba,
    ScrollHandle, ScrollWheelEvent, Style, Window, fill, point, px, relative, rgba, size,
};

use crate::{Rgba8, TextDirection, UiNode, UiValue};

const TRACK_WIDTH: Pixels = px(12.0);
const THUMB_WIDTH: Pixels = px(6.0);
const THUMB_ACTIVE_WIDTH: Pixels = px(8.0);
const THUMB_INSET: Pixels = px(3.0);
const MIN_THUMB_LENGTH: Pixels = px(32.0);
const AUTO_IDLE: Duration = Duration::from_millis(1400);

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
    dragged: Option<Axis>,
    drag_position: Point<Pixels>,
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
}

pub(crate) struct ThemedScrollbar {
    id: ElementId,
    handle: ScrollHandle,
    spec: ScrollbarSpec,
    direction: TextDirection,
    track: Rgba8,
    thumb: Rgba8,
    thumb_hover: Rgba8,
}

impl ThemedScrollbar {
    pub(crate) fn new(
        id: String,
        handle: ScrollHandle,
        spec: ScrollbarSpec,
        direction: TextDirection,
        track: Rgba8,
        thumb: Rgba8,
        thumb_hover: Rgba8,
    ) -> Self {
        Self {
            id: ElementId::Name(id.into()),
            handle,
            spec,
            direction,
            track,
            thumb,
            thumb_hover,
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
        let handle_bounds = self.handle.bounds();
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
                Axis::Horizontal => (viewport.size.width, max.width, -offset.x),
                Axis::Vertical => (viewport.size.height, max.height, -offset.y),
            };
            if max_offset <= px(0.0) || container <= px(0.0) {
                continue;
            }
            let content = container + max_offset;
            let thumb_length = (container / content * container).max(MIN_THUMB_LENGTH);
            let travel = (container - thumb_length).max(px(0.0));
            let thumb_start = (scroll_position / max_offset * travel).clamp(px(0.0), travel);
            let track = axis_track(viewport, axis, self.direction);
            let thumb_hitbox = axis_thumb(track, axis, thumb_start, thumb_length, TRACK_WIDTH);
            let active_width = if state.0.get().dragged == Some(axis) {
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
                || inner.dragged.is_some()
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
        register_wheel_listener(viewport, self.handle.clone(), state.clone(), view, window);
        for axis in &prepaint.axes {
            if !axis.visible {
                continue;
            }
            let hovered = state.0.get().hovered || state.0.get().dragged == Some(axis.axis);
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
            register_axis_listeners(axis, &self.handle, state.clone(), view, window);
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
    handle: ScrollHandle,
    state: ScrollbarState,
    view: gpui::EntityId,
    window: &mut Window,
) {
    window.on_mouse_event(move |event: &ScrollWheelEvent, phase, _, cx| {
        if phase == DispatchPhase::Bubble && viewport.contains(&event.position) {
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
    handle: &ScrollHandle,
    state: ScrollbarState,
    view: gpui::EntityId,
    window: &mut Window,
) {
    let axis = axis.clone();
    let handle = handle.clone();
    let down_handle = handle.clone();
    let down_state = state.clone();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, _, cx| {
        if phase != DispatchPhase::Bubble || !axis.track.contains(&event.position) {
            return;
        }
        cx.stop_propagation();
        let mut inner = down_state.0.get();
        inner.last_activity = Some(Instant::now());
        if axis.thumb_hitbox.contains(&event.position) {
            inner.dragged = Some(axis.axis);
            inner.drag_position = event.position - axis.thumb_hitbox.origin;
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

    let move_handle = handle.clone();
    let move_state = state.clone();
    window.on_mouse_event(move |event: &MouseMoveEvent, _, _, cx| {
        let inner = move_state.0.get();
        if inner.dragged != Some(axis.axis) || !event.dragging() {
            return;
        }
        cx.stop_propagation();
        let position = match axis.axis {
            Axis::Horizontal => event.position.x - inner.drag_position.x - axis.track.origin.x,
            Axis::Vertical => event.position.y - inner.drag_position.y - axis.track.origin.y,
        };
        let travel = match axis.axis {
            Axis::Horizontal => axis.track.size.width - axis.thumb_length,
            Axis::Vertical => axis.track.size.height - axis.thumb_length,
        };
        let percentage = (position / travel.max(px(1.0))).clamp(0.0, 1.0);
        let offset = move_handle.offset();
        match axis.axis {
            Axis::Horizontal => {
                move_handle.set_offset(point(-axis.max_offset * percentage, offset.y));
            }
            Axis::Vertical => {
                move_handle.set_offset(point(offset.x, -axis.max_offset * percentage));
            }
        }
        let mut inner = move_state.0.get();
        inner.last_activity = Some(Instant::now());
        move_state.0.set(inner);
        cx.notify(view);
    });

    let up_state = state;
    window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
        if phase == DispatchPhase::Bubble && up_state.0.get().dragged == Some(axis.axis) {
            let mut inner = up_state.0.get();
            inner.dragged = None;
            inner.last_activity = Some(Instant::now());
            up_state.0.set(inner);
            cx.notify(view);
        }
    });
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
}
