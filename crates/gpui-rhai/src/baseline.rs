//! Baseline alignment for the text children of an `items_baseline` row.
//!
//! GPUI lays text out as boxes and gives Taffy no baselines, so a row cannot align
//! words of different sizes on one line. A text child of such a row is wrapped in
//! [`BaselineText`]: while it requests layout, the row's text style is the window's
//! current one, so the child's top margin can be set to the distance between the
//! row's baseline and its own, both measured from the top of their line boxes the
//! way GPUI paints a line (half the leading above the ascent).

use gpui::{
    A11ySubtreeBuilder, AbsoluteLength, App, Bounds, DefiniteLength, Element, ElementId,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, Length, Pixels, Refineable, Styled,
    TextStyle, TextStyleRefinement, Window, px,
};

/// A row's text child moved down (or up) so its first baseline sits on the row's.
pub(crate) struct BaselineText<E> {
    element: E,
    text: TextStyleRefinement,
}

impl<E> BaselineText<E> {
    /// `text` is the child's own text style refinement, applied over the row's.
    pub(crate) fn new(element: E, text: TextStyleRefinement) -> Self {
        Self { element, text }
    }
}

/// The distance from the top of a line box to its baseline, as GPUI paints the line.
fn first_baseline(style: &TextStyle, window: &Window) -> Pixels {
    let rem_size = window.rem_size();
    let text_system = window.text_system();
    let font = text_system.resolve_font(&style.font());
    text_system.baseline_offset(
        font,
        style.font_size.to_pixels(rem_size),
        style.line_height_in_pixels(rem_size),
    )
}

impl<E: Element + Styled> IntoElement for BaselineText<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<E: Element + Styled> Element for BaselineText<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.element.id()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.element.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let row = window.text_style();
        let mut own = row.clone();
        own.refine(&self.text);
        let shift = first_baseline(&row, window) - first_baseline(&own, window);
        let margin = &mut self.element.style().margin.top;
        // A pixel margin of the child's own adds to the shift; any other margin stays.
        match margin {
            None => *margin = Some(Length::Definite(px(f32::from(shift)).into())),
            Some(Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(value)))) => {
                *value += shift;
            }
            Some(_) => {}
        }
        self.element.request_layout(id, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.element
            .prepaint(id, inspector_id, bounds, request_layout, window, cx)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.paint(
            id,
            inspector_id,
            bounds,
            request_layout,
            prepaint,
            window,
            cx,
        );
    }

    fn a11y_role(&self) -> Option<gpui::accesskit::Role> {
        self.element.a11y_role()
    }

    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        self.element.write_a11y_info(node);
    }

    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        self.element.a11y_synthetic_children(prepaint, builder);
    }
}
