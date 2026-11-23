//! Lay out one real rich-text paragraph, then place ordinary retained
//! semantic controls at its text ranges without measuring or painting again.

use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, ComputedPaint, DispatchPhase, Element, ElementId,
    FocusHandle, GlobalElementId, Hitbox, HitboxBehavior, Hsla, InspectorElementId, IntoElement,
    LayoutId, MouseMoveEvent, ParentElement as _, Pixels, Point, SharedString, Styled as _,
    StyledText, Window, div, size,
};

use crate::inline_link::{InlineLinkFrame, paint_outline};

#[derive(Clone)]
pub(crate) struct InlineProjection {
    ranges: Rc<[Range<usize>]>,
    target_paint: Rc<[InlineTargetPaint]>,
    frame: Rc<RefCell<InlineLinkFrame>>,
}

pub(crate) type InlinePaintDriver =
    dyn Fn(&mut ComputedPaint, crate::paint_motion::PaintTransition, &mut Window);

pub(crate) struct InlineTargetPaint {
    pub(crate) enabled: bool,
    pub(crate) animate: Option<Rc<InlinePaintDriver>>,
}

pub(crate) struct HoverUnderline {
    slot: usize,
    idle: Option<Hsla>,
    glyph: Option<Hsla>,
    transition: crate::paint_motion::PaintTransition,
}

impl InlineProjection {
    pub(crate) fn new(ranges: Vec<Range<usize>>, target_paint: Vec<InlineTargetPaint>) -> Self {
        Self {
            ranges: ranges.into(),
            target_paint: target_paint.into(),
            frame: Rc::default(),
        }
    }

    pub(crate) fn hover_underlines(
        &self,
        spans: &[crate::Span],
        colors: &impl crate::ColorResolver,
    ) -> Vec<HoverUnderline> {
        let resolve = |value: &crate::ColorValue| {
            colors
                .resolve(value)
                .map(|color| gpui::rgba(color.as_rgba_hex()).into())
        };
        crate::node::spans_with_ranges(spans)
            .filter_map(|(range, span)| {
                let underline = span.underline_paint()?;
                let transition = underline.hover?;
                let slot = self.ranges.iter().position(|target| target == &range)?;
                self.target_paint
                    .get(slot)?
                    .enabled
                    .then(|| HoverUnderline {
                        slot,
                        idle: resolve(&underline.color),
                        glyph: span.color_value().and_then(resolve),
                        transition,
                    })
            })
            .collect()
    }

    pub(crate) fn allows(
        &self,
        slot: Option<usize>,
        position: Point<Pixels>,
        window: &Window,
    ) -> bool {
        window
            .event_position_in_layout(position)
            .is_some_and(|p| self.frame.borrow().hit(p) == slot)
    }
}

pub(crate) struct InlineSource {
    pub(crate) text: StyledText,
    pub(crate) source: SharedString,
    pub(crate) projection: InlineProjection,
    pub(crate) hover: Vec<HoverUnderline>,
}

fn hovered_slot(hitboxes: &[(usize, Hitbox)], window: &Window) -> Option<usize> {
    hitboxes
        .iter()
        .rev()
        .find_map(|(slot, hitbox)| hitbox.is_hovered(window).then_some(*slot))
}

impl IntoElement for InlineSource {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for InlineSource {
    type RequestLayoutState = ();
    type PrepaintState = Vec<(usize, Hitbox)>;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        self.text.request_layout(id, inspector, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.text.prepaint(id, inspector, bounds, state, window, cx);
        self.projection
            .frame
            .replace(InlineLinkFrame::project_ranges(
                self.text.layout(),
                self.source.as_ref(),
                self.projection.ranges.iter(),
                window,
            ));
        let frame = self.projection.frame.borrow();
        self.hover
            .iter()
            .flat_map(|underline| {
                let slot = underline.slot;
                frame
                    .0
                    .get(slot)
                    .into_iter()
                    .flatten()
                    .map(move |bounds| (slot, *bounds))
            })
            .map(|(slot, bounds)| (slot, window.insert_hitbox(bounds, HitboxBehavior::Normal)))
            .collect()
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut (),
        hitboxes: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.hover.is_empty() {
            self.text
                .paint(id, inspector, bounds, state, &mut (), window, cx);
            return;
        }
        let hovered = hovered_slot(hitboxes, window);
        let fallback = window.text_style().color;
        let mut strokes = [None; 32];
        for underline in &self.hover {
            let glyph = underline.glyph.unwrap_or(fallback);
            let mut ink = if hovered == Some(underline.slot) {
                glyph
            } else {
                underline.idle.unwrap_or(glyph)
            };
            if let Some(animate) = self
                .projection
                .target_paint
                .get(underline.slot)
                .and_then(|target| target.animate.as_ref())
            {
                let mut paint = ComputedPaint {
                    text_color: Some(ink),
                    ..Default::default()
                };
                animate(&mut paint, underline.transition, window);
                ink = paint.text_color.unwrap_or(ink);
            }
            strokes[underline.slot] = Some(ink);
        }
        let _ =
            self.text.paint_with_underline_handler(
                window,
                cx,
                |bytes, origin, width, style, window| {
                    let ink = self.projection.ranges.iter().zip(strokes.iter()).find_map(
                        |(range, ink)| {
                            if bytes.start >= range.start && bytes.end <= range.end {
                                *ink
                            } else {
                                None
                            }
                        },
                    );
                    let stroke = gpui::UnderlineStyle {
                        color: ink.or(style.color),
                        ..*style
                    };
                    window.paint_underline(origin, width, &stroke);
                },
            );
        if !hitboxes.is_empty() {
            let hitboxes = std::mem::take(hitboxes);
            let mut previous = hovered;
            window.on_mouse_event(move |_: &MouseMoveEvent, phase, window, _| {
                if phase == DispatchPhase::Capture {
                    let next = hovered_slot(&hitboxes, window);
                    if next != previous {
                        previous = next;
                        window.refresh();
                    }
                }
            });
        }
    }
}

pub(crate) struct InlineTarget {
    pub(crate) element: Option<AnyElement>,
    pub(crate) focus: Option<FocusHandle>,
    #[cfg(all(
        target_os = "macos",
        any(feature = "offscreen-pre", feature = "offscreen-fast")
    ))]
    pub(crate) owner: Option<crate::NodeId>,
}

pub(crate) struct InlineProse {
    pub(crate) body: AnyElement,
    pub(crate) targets: Vec<InlineTarget>,
    pub(crate) projection: InlineProjection,
    #[cfg(all(
        target_os = "macos",
        any(feature = "offscreen-pre", feature = "offscreen-fast")
    ))]
    pub(crate) geometry: crate::GeometryRegistry,
    pub(crate) focus_color: Option<Hsla>,
}

impl IntoElement for InlineProse {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for InlineProse {
    type RequestLayoutState = ();
    type PrepaintState = Vec<AnyElement>;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        Element::request_layout(&mut self.body, id, inspector, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<AnyElement> {
        self.projection.frame.replace(InlineLinkFrame::default());
        Element::prepaint(&mut self.body, id, inspector, bounds, state, window, cx);
        let frame = self.projection.frame.borrow();
        self.targets
            .iter_mut()
            .zip(&frame.0)
            .filter_map(|(target, fragments)| {
                let union = fragments
                    .iter()
                    .skip(1)
                    .fold(*fragments.first()?, |a, b| a.union(b));
                let mut region = div()
                    .w(union.size.width)
                    .h(union.size.height)
                    .child(target.element.take()?)
                    .into_any_element();
                region.prepaint_as_root(
                    union.origin,
                    size(
                        AvailableSpace::Definite(union.size.width),
                        AvailableSpace::Definite(union.size.height),
                    ),
                    window,
                    cx,
                );
                #[cfg(all(
                    target_os = "macos",
                    any(feature = "offscreen-pre", feature = "offscreen-fast")
                ))]
                if let Some(owner) = target.owner {
                    self.geometry
                        .record_pointer_aim(owner, pointer_aim(fragments, window));
                }
                Some(region)
            })
            .collect()
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut (),
        targets: &mut Vec<AnyElement>,
        window: &mut Window,
        cx: &mut App,
    ) {
        Element::paint(
            &mut self.body,
            id,
            inspector,
            bounds,
            state,
            &mut (),
            window,
            cx,
        );
        for target in targets {
            target.paint(window, cx);
        }
        if let Some(ink) = self.focus_color
            && window.last_input_was_keyboard()
        {
            for (target, rects) in self.targets.iter().zip(&self.projection.frame.borrow().0) {
                if target
                    .focus
                    .as_ref()
                    .is_some_and(|focus| focus.is_focused(window))
                {
                    for rect in rects {
                        paint_outline(*rect, ink, window);
                    }
                }
            }
        }
    }
}

#[cfg(all(
    target_os = "macos",
    any(feature = "offscreen-pre", feature = "offscreen-fast")
))]
fn pointer_aim(fragments: &[Bounds<Pixels>], window: &Window) -> crate::offscreen::PointerAim {
    use crate::offscreen::PointerAim;

    let mask = window.content_mask().bounds;
    let viewport = Bounds::new(Point::default(), window.viewport_size());
    let coordinates = window
        .visual_coordinates()
        .clipped_to(mask)
        .clipped_to_visual(viewport);
    fragments
        .iter()
        .find_map(|fragment| {
            let rect = fragment.intersect(&mask);
            if rect.size.width <= Pixels::ZERO || rect.size.height <= Pixels::ZERO {
                return None;
            }
            // In the ordinary untransformed path, intersect viewport and mask in
            // the same plane. For an affine, verify every candidate in both
            // planes: an AABB center need not actually hit a clipped polygon.
            let middle = if coordinates.is_identity() {
                Some(rect.intersect(&viewport).center())
            } else {
                coordinates.visual_point(rect.center())
            };
            let clipped_middle = || {
                coordinates
                    .clipped_visual_bounds(rect)
                    .map(|bounds| bounds.center())
            };
            let valid = |point: &Point<Pixels>| {
                point.x >= viewport.left()
                    && point.x < viewport.right()
                    && point.y >= viewport.top()
                    && point.y < viewport.bottom()
                    && coordinates.accepts_visual_point(*point)
                    && coordinates.layout_point(*point).is_some_and(|p| {
                        rect.left() <= p.x
                            && p.x < rect.right()
                            && rect.top() <= p.y
                            && p.y < rect.bottom()
                    })
            };
            middle
                .filter(valid)
                .or_else(|| clipped_middle().filter(valid))
        })
        .map_or(PointerAim::Hidden, PointerAim::Point)
}
