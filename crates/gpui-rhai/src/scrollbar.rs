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
    ElementId, GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, IntoElement, IsZero,
    LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, PaintQuad, Pixels, Point, Position,
    Rgba, ScrollHandle, ScrollWheelEvent, Style, Window, fill, point, px, relative, rgba, size,
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
    /// Optional identity for retained wrappers; a changed identity retires a
    /// gesture that captured an older native source under the same element id.
    fn identity(&self) -> Option<usize> {
        None
    }
}

/// Retain this adapter alongside a Host-native scroll state when that state may
/// be replaced under a stable scrollbar id. Clones share a source lease;
/// constructing a replacement adapter retires an old captured thumb drag.
#[derive(Clone)]
pub struct ScrollbarSourceHandle(Rc<dyn ScrollbarSource>);

impl ScrollbarSourceHandle {
    #[must_use]
    pub fn new(source: impl ScrollbarSource) -> Self {
        Self(Rc::new(source))
    }
}

impl ScrollbarSource for ScrollbarSourceHandle {
    fn viewport(&self) -> Bounds<Pixels> {
        self.0.viewport()
    }
    fn offset(&self) -> Point<Pixels> {
        self.0.offset()
    }
    fn max_offset(&self) -> Point<Pixels> {
        self.0.max_offset()
    }
    fn set_offset(&self, value: Point<Pixels>) {
        self.0.set_offset(value);
    }
    fn drag_started(&self) {
        self.0.drag_started();
    }
    fn drag_ended(&self) {
        self.0.drag_ended();
    }
    fn identity(&self) -> Option<usize> {
        Some(Rc::as_ptr(&self.0).cast::<()>() as usize)
    }
}

#[derive(Clone)]
pub(crate) struct ScrollWheelSource {
    source: Rc<dyn ScrollbarSource>,
    axes: (bool, bool),
    translate: bool,
}

impl ScrollWheelSource {
    pub(crate) fn new(source: impl ScrollbarSource, axes: (bool, bool), translate: bool) -> Self {
        Self {
            source: Rc::new(source),
            axes,
            translate,
        }
    }

    fn apply(&self, position: Point<Pixels>, mut remaining: Point<Pixels>) -> Point<Pixels> {
        if !self.source.viewport().contains(&position) {
            return remaining;
        }
        let max = self.source.max_offset();
        let clamp = |value: Point<Pixels>| {
            point(
                value.x.clamp(-max.x.max(px(0.0)), px(0.0)),
                value.y.clamp(-max.y.max(px(0.0)), px(0.0)),
            )
        };
        let before = clamp(self.source.offset());
        let translate_y = self.translate && self.axes == (true, false) && remaining.x.is_zero();
        let translate_x = self.translate && self.axes == (false, true) && remaining.y.is_zero();
        let delta = point(
            if self.axes.0 {
                if translate_y {
                    remaining.y
                } else {
                    remaining.x
                }
            } else {
                px(0.0)
            },
            if self.axes.1 {
                if translate_x {
                    remaining.x
                } else {
                    remaining.y
                }
            } else {
                px(0.0)
            },
        );
        let target = clamp(before + delta);
        if target == before {
            return remaining;
        }
        self.source.set_offset(target);
        // Native tail anchors can report past max until their next layout;
        // normalize the native report, never invent a parallel position cache.
        let applied = clamp(self.source.offset()) - before;
        if translate_y {
            remaining.y -= applied.x;
        } else {
            remaining.x -= applied.x;
        }
        if translate_x {
            remaining.x -= applied.y;
        } else {
            remaining.y -= applied.y;
        }
        remaining
    }
}

/// End the native drag even if its window/coordinator is dropped before it can
/// deliver a cancellation callback. Both normal paths and this finalizer are
/// idempotent and run on the foreground thread.
struct ScrollbarDrag {
    source: Rc<dyn ScrollbarSource>,
    active: Cell<bool>,
}

impl ScrollbarDrag {
    fn end(&self) {
        if self.active.replace(false) {
            self.source.drag_ended();
        }
    }
}

impl Drop for ScrollbarDrag {
    fn drop(&mut self) {
        self.end();
    }
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
    wheel_parents: Vec<ScrollWheelSource>,
    wheel_axes: (bool, bool),
    translate_wheel: bool,
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
            wheel_parents: Vec::new(),
            wheel_axes: (true, true),
            translate_wheel: false,
        }
    }

    /// Restrict wheel ownership independently of visual bar visibility.
    /// `None` permits both axes; unsupported axes remain available to parents.
    #[must_use]
    pub fn with_scroll_axis(mut self, axis: Option<Axis>) -> Self {
        self.wheel_axes = (axis != Some(Axis::Vertical), axis != Some(Axis::Horizontal));
        self
    }

    /// Add the source's nearest native parent, ahead of inherited SDK ancestors.
    /// Later calls add a nearer parent: `.with_scroll_parent(document, axis)`
    /// followed by `.with_scroll_parent(table, axis)` routes residual input to
    /// table, document, then the automatically inherited Rhai/SDK chain.
    #[must_use]
    pub fn with_scroll_parent(mut self, source: impl ScrollbarSource, axis: Option<Axis>) -> Self {
        self.wheel_parents.insert(
            0,
            ScrollWheelSource::new(
                source,
                (axis != Some(Axis::Vertical), axis != Some(Axis::Horizontal)),
                false,
            ),
        );
        self
    }

    pub(crate) fn with_wheel_parents(mut self, parents: Vec<ScrollWheelSource>) -> Self {
        self.wheel_parents = parents;
        self
    }

    pub(crate) fn with_wheel_policy(mut self, axes: (bool, bool), translate: bool) -> Self {
        self.wheel_axes = axes;
        self.translate_wheel = translate;
        self
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
        self.build_element(id.into(), source, spec, false)
    }

    /// Opt this native viewport's body into the shared residual wheel policy.
    /// Capture-phase routing consumes actual movement once, before GPUI's
    /// automatic list/div scrolling. Completely unconsumed input propagates.
    ///
    /// Retain `source` with the native viewport. Declare every enclosing native
    /// viewport with `with_scroll_parent` using that parent's same retained
    /// handle. The innermost hit, clipped, unoccluded region owns the event even
    /// when a parent scrollbar overlay paints later. Unrelated overlapping
    /// regions retain front-to-back paint priority. This does not claim clicks,
    /// selection, focus or keys; ordinary `element` remains track-only.
    #[must_use]
    pub fn element_with_wheel_routing(
        &self,
        id: impl Into<ElementId>,
        source: ScrollbarSourceHandle,
        spec: ScrollbarSpec,
    ) -> gpui::AnyElement {
        self.build_element(id.into(), source, spec, true)
    }

    fn build_element(
        &self,
        id: ElementId,
        source: impl ScrollbarSource,
        spec: ScrollbarSpec,
        wheel_body: bool,
    ) -> gpui::AnyElement {
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
        scrollbar.wheel_parents.clone_from(&self.wheel_parents);
        scrollbar.wheel_axes = self.wheel_axes;
        scrollbar.translate_wheel = self.translate_wheel;
        scrollbar.wheel_body = wheel_body;
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
    source_identity: Option<usize>,
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

impl AxisPrepaint {
    fn coordinate(&self, point: Point<Pixels>) -> Pixels {
        match self.axis {
            Axis::Horizontal => point.x,
            Axis::Vertical => point.y,
        }
    }

    fn percentage(&self, position: Point<Pixels>, grab: Pixels) -> f32 {
        let container = match self.axis {
            Axis::Horizontal => self.track.size.width,
            Axis::Vertical => self.track.size.height,
        };
        ((self.coordinate(position) - self.coordinate(self.track.origin) - grab)
            / (container - self.thumb_length).max(px(1.0)))
        .clamp(0.0, 1.0)
    }

    fn set_percentage(&self, source: &dyn ScrollbarSource, percentage: f32) {
        let offset = source.offset();
        source.set_offset(match self.axis {
            Axis::Horizontal => point(-self.max_offset * percentage, offset.y),
            Axis::Vertical => point(offset.x, -self.max_offset * percentage),
        });
    }
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
    wheel_parents: Vec<ScrollWheelSource>,
    wheel_axes: (bool, bool),
    translate_wheel: bool,
    wheel_body: bool,
}

impl ThemedScrollbar {
    fn body_hitbox(&self, viewport: Bounds<Pixels>, window: &mut Window) -> Option<Hitbox> {
        (self.wheel_body && !self.disabled).then(|| {
            window.with_content_mask(Some(ContentMask { bounds: viewport }), |window| {
                window.insert_hitbox(viewport, HitboxBehavior::Normal)
            })
        })
    }

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
            wheel_parents: Vec::new(),
            wheel_axes: (true, true),
            translate_wheel: false,
            wheel_body: false,
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
    body: Option<Hitbox>,
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
        _bounds: Bounds<Pixels>,
        (): &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let viewport = self.handle.viewport();
        let state = window
            .use_state(cx, |_, _| ScrollbarState::default())
            .read(cx)
            .clone();
        let now = Instant::now();
        let offset = self.handle.offset();
        let mut inner = state.0.get();
        let identity = self.handle.identity();
        if inner.source_identity != identity {
            for axis in [Axis::Horizontal, Axis::Vertical] {
                self.interactions.cancel_owner(
                    &axis_interaction_owner(&self.interaction_owner, axis),
                    window,
                    cx,
                );
            }
            inner.source_identity = identity;
        }
        if inner.last_offset != offset {
            inner.last_offset = offset;
            inner.last_activity = Some(now);
        }
        state.0.set(inner);
        schedule_idle_refresh(&state, now, window, cx);

        let max = self.handle.max_offset();
        let body = self.body_hitbox(viewport, window);
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
            if !self.disabled {
                self.interactions.present(owner.clone());
            }
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
            body,
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
        let mut wheel_chain = vec![ScrollWheelSource {
            source: self.handle.clone(),
            axes: self.wheel_axes,
            translate: self.translate_wheel,
        }];
        wheel_chain.extend(self.wheel_parents.clone());
        register_wheel_listener(
            viewport,
            wheel_chain,
            state.clone(),
            view,
            prepaint.body.as_ref().map_or_else(
                || {
                    WheelHitRegion::Tracks(
                        prepaint
                            .axes
                            .iter()
                            .map(|axis| axis.bar_hitbox.clone())
                            .collect(),
                    )
                },
                |_| WheelHitRegion::Body {
                    coordinator: self.interactions.clone(),
                    owner: self.interaction_owner.clone(),
                    source: self
                        .handle
                        .identity()
                        .expect("body routing requires a retained source handle"),
                    ancestors: self
                        .wheel_parents
                        .iter()
                        .filter_map(|parent| parent.source.identity())
                        .collect(),
                    hitbox: prepaint.body.clone().expect("body hitbox was prepared"),
                },
            ),
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
            if !self.disabled {
                window.set_cursor_style(CursorStyle::PointingHand, &axis.bar_hitbox);
            }
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

enum WheelHitRegion {
    Tracks(Vec<Hitbox>),
    Body {
        coordinator: crate::interaction::WindowInteractionCoordinator,
        owner: crate::interaction::InteractionOwner,
        source: usize,
        ancestors: Vec<usize>,
        hitbox: Hitbox,
    },
}

fn register_wheel_listener(
    viewport: Bounds<Pixels>,
    chain: Vec<ScrollWheelSource>,
    state: ScrollbarState,
    view: gpui::EntityId,
    region: WheelHitRegion,
    disabled: bool,
    window: &mut Window,
) {
    let apply: Rc<crate::interaction::NativeWheelHandler> = Rc::new(move |event, window, cx| {
        let delta = event.delta.pixel_delta(window.line_height());
        let remaining = chain.iter().fold(delta, |remaining, source| {
            source.apply(event.position, remaining)
        });
        if remaining != delta {
            cx.stop_propagation();
            window.refresh();
        }
        let mut inner = state.0.get();
        if let Some(source) = chain.first() {
            inner.last_offset = source.source.offset();
        }
        inner.last_activity = Some(Instant::now());
        state.0.set(inner);
        cx.notify(view);
    });
    window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
        if !disabled && viewport.contains(&event.position) {
            match (&region, phase) {
                (
                    WheelHitRegion::Body {
                        coordinator,
                        owner,
                        source,
                        ancestors,
                        hitbox,
                    },
                    DispatchPhase::Capture,
                ) => {
                    if hitbox.should_handle_scroll(window) {
                        coordinator.register_wheel_region(
                            owner.clone(),
                            *source,
                            ancestors.clone(),
                            hitbox.clone(),
                            Rc::clone(&apply),
                        );
                    }
                }
                (WheelHitRegion::Tracks(tracks), DispatchPhase::Bubble)
                    if tracks
                        .iter()
                        .any(|track| track.should_handle_scroll(window)) =>
                {
                    apply(event, window, cx)
                }
                _ => {}
            }
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
            let drag = Rc::new(ScrollbarDrag {
                source: down_handle.clone(),
                active: Cell::new(false),
            });
            let drag_position = event.position - axis.thumb_hitbox.origin;
            let update_axis = axis.clone();
            let update_handle = down_handle.clone();
            let update_state = down_state.clone();
            let update =
                move |gesture: crate::interaction::GestureUpdate, _: &mut Window, _: &mut App| {
                    let percentage = update_axis.percentage(
                        gesture.current(),
                        update_axis.coordinate(drag_position) + THUMB_INSET,
                    );
                    update_axis.set_percentage(update_handle.as_ref(), percentage);
                    let mut inner = update_state.0.get();
                    inner.last_activity = Some(Instant::now());
                    update_state.0.set(inner);
                    crate::interaction::InteractionFlow::Continue
                };
            let finish_state = down_state.clone();
            let finish_drag = Rc::clone(&drag);
            let finish =
                move |_: crate::interaction::GestureUpdate, _: &mut Window, _: &mut App| {
                    finish_drag.end();
                    let mut inner = finish_state.0.get();
                    inner.last_activity = Some(Instant::now());
                    finish_state.0.set(inner);
                };
            let cancel_state = down_state.clone();
            let cancel_drag = Rc::clone(&drag);
            let cancel = move |_: &mut Window, _: &mut App| {
                cancel_drag.end();
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
            drag.active.set(true);
        } else {
            axis.set_percentage(
                down_handle.as_ref(),
                axis.percentage(event.position, axis.thumb_length / 2.0),
            );
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
    use std::cell::RefCell;

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

    #[derive(Clone)]
    struct OrderedParent {
        name: &'static str,
        offset: Rc<Cell<Point<Pixels>>>,
        max: Pixels,
        writes: Rc<RefCell<Vec<&'static str>>>,
    }

    impl ScrollbarSource for OrderedParent {
        fn viewport(&self) -> Bounds<Pixels> {
            Bounds::new(point(px(0.0), px(0.0)), size(px(100.0), px(100.0)))
        }
        fn offset(&self) -> Point<Pixels> {
            self.offset.get()
        }
        fn max_offset(&self) -> Point<Pixels> {
            point(px(0.0), self.max)
        }
        fn set_offset(&self, offset: Point<Pixels>) {
            self.offset.set(offset);
            self.writes.borrow_mut().push(self.name);
        }
    }

    #[test]
    fn explicit_native_parents_precede_inherited_ancestors_nearest_last_added() {
        let writes = Rc::new(RefCell::new(Vec::new()));
        let parent = |name, max| OrderedParent {
            name,
            offset: Rc::new(Cell::new(point(px(0.0), px(0.0)))),
            max: px(max),
            writes: Rc::clone(&writes),
        };
        let table = parent("table", 20.0);
        let document = parent("document", 30.0);
        let inherited = parent("rhai", 100.0);
        let theme = crate::PrimitiveTheme::from_colors(
            &crate::LiteralColorResolver,
            TextDirection::LeftToRight,
        );
        let bars = ScrollbarContext::new(
            crate::interaction::InteractionOwner::new("test", "parent-order"),
            crate::interaction::WindowInteractionCoordinator::default(),
            &theme,
        )
        .with_wheel_parents(vec![ScrollWheelSource::new(
            inherited.clone(),
            (false, true),
            false,
        )])
        .with_scroll_parent(document.clone(), Some(Axis::Vertical))
        .with_scroll_parent(table.clone(), Some(Axis::Vertical));
        let residual = bars
            .wheel_parents
            .iter()
            .fold(point(px(0.0), px(-80.0)), |remaining, source| {
                source.apply(point(px(10.0), px(10.0)), remaining)
            });
        assert_eq!(*writes.borrow(), ["table", "document", "rhai"]);
        assert_eq!(table.offset().y, px(-20.0));
        assert_eq!(document.offset().y, px(-30.0));
        assert_eq!(inherited.offset().y, px(-30.0));
        assert_eq!(residual, point(px(0.0), px(0.0)));
    }
}
