//! Shared non-layout focus decoration. No screenshots, background guesses or
//! extra hitboxes: both backends expose the same native border-quad operation.

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Window, point, px, rgba, size,
};

use crate::{
    ColorResolver, CornerLengths, FocusRingSpec, LayoutLength, Length, Rgba8, SignedLength,
    TextDirection,
};

pub(crate) struct FocusRingElement {
    child: Option<AnyElement>,
    width: Length,
    offset: LayoutLength,
    corners: [Option<Length>; 4],
    color: Rgba8,
    opacity: f32,
}

impl FocusRingElement {
    pub(crate) fn wrap(
        child: AnyElement,
        spec: &FocusRingSpec,
        radii: &CornerLengths,
        resolver: &impl ColorResolver,
        direction: TextDirection,
        opacity: f32,
    ) -> AnyElement {
        let offset = match spec.offset {
            LayoutLength::Definite(length) => {
                resolver.resolve_length(length).map(LayoutLength::Definite)
            }
            LayoutLength::Signed(_) => Some(spec.offset),
            LayoutLength::Auto => None,
        };
        let Some((width, offset, color)) = resolver
            .resolve_length(spec.width)
            .zip(offset)
            .zip(resolver.resolve(&spec.color))
            .map(|((width, offset), color)| (width, offset, color))
        else {
            return child;
        };
        let (start, end) = match direction {
            TextDirection::LeftToRight => (radii.start, radii.end),
            TextDirection::RightToLeft => (radii.end, radii.start),
        };
        let corners = [
            start.or(radii.top_left),
            end.or(radii.top_right),
            end.or(radii.bottom_right),
            start.or(radii.bottom_left),
        ]
        .map(|value| value.and_then(|value| resolver.resolve_length(value)));
        Self {
            child: Some(child),
            width,
            offset,
            corners,
            color,
            opacity,
        }
        .into_any_element()
    }

    fn paint_ring(&self, bounds: Bounds<Pixels>, window: &mut Window) {
        let rem = f64::from(f32::from(window.rem_size()));
        let resolve = |length: Length| match length {
            Length::Pixels(value) => Some(value),
            Length::Rems(value) => Some(value * rem),
            Length::Relative(_) | Length::Token(_) => None,
        };
        let offset = match self.offset {
            LayoutLength::Definite(length) => resolve(length),
            LayoutLength::Signed(SignedLength::Pixels(value)) => Some(value),
            LayoutLength::Signed(SignedLength::Rems(value)) => Some(value * rem),
            LayoutLength::Auto | LayoutLength::Signed(SignedLength::Relative(_)) => None,
        };
        let Some((width, offset)) = resolve(self.width).zip(offset) else {
            return;
        };
        let outset = width + offset;
        let x = f64::from(f32::from(bounds.origin.x)) - outset;
        let y = f64::from(f32::from(bounds.origin.y)) - outset;
        let w = f64::from(f32::from(bounds.size.width));
        let h = f64::from(f32::from(bounds.size.height));
        let outer_w = w + outset * 2.0;
        let outer_h = h + outset * 2.0;
        if width <= 0.0
            || w <= 0.0
            || h <= 0.0
            || outer_w <= 0.0
            || outer_h <= 0.0
            || [x, y, outer_w, outer_h, width, x + outer_w, y + outer_h]
                .into_iter()
                .any(|value| !value.is_finite() || value.abs() > f64::from(f32::MAX))
        {
            return;
        }
        let corner = self.corners.map(|value| {
            let radius = value
                .and_then(resolve)
                .unwrap_or(0.0)
                .max(0.0)
                .min(w.min(h) * 0.5);
            px(native(if radius > 0.0 {
                (radius + outset).max(0.0)
            } else {
                0.0
            }))
        });
        let corners = gpui::Corners {
            top_left: corner[0],
            top_right: corner[1],
            bottom_right: corner[2],
            bottom_left: corner[3],
        };
        let color: gpui::Hsla = rgba(self.color.as_rgba_hex()).into();
        window.paint_quad(gpui::quad(
            Bounds::new(
                point(px(native(x)), px(native(y))),
                size(px(native(outer_w)), px(native(outer_h))),
            ),
            corners,
            gpui::transparent_black(),
            gpui::Edges::all(px(native(width))),
            color.opacity(self.opacity),
            gpui::BorderStyle::default(),
        ));
    }
}

#[allow(clippy::cast_possible_truncation)]
fn native(value: f64) -> f32 {
    value as f32
}

impl Element for FocusRingElement {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
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
        let mut child = self.child.take().expect("focus ring renders once");
        (child.request_layout(window, cx), child)
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut AnyElement,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
        self.paint_ring(bounds, window);
    }
}

impl IntoElement for FocusRingElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
