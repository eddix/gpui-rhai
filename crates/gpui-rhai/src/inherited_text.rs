//! Read the actual parent text color before a node's own native refinements.

#[cfg(feature = "vendor-gpui")]
use gpui::IntoElement;
use gpui::{
    AnyElement, App, Bounds, Div, Element, ElementId, GlobalElementId, Hsla, InspectorElementId,
    LayoutId, ParentElement, Pixels, Stateful, StyleRefinement, Styled, Window,
};

type Refine = Box<dyn FnOnce(Stateful<Div>, Hsla) -> AnyElement>;

pub(crate) struct InheritedTextElement {
    element: Option<Stateful<Div>>,
    refine: Option<Refine>,
}

impl InheritedTextElement {
    pub(crate) fn new(
        element: Stateful<Div>,
        refine: impl FnOnce(Stateful<Div>, Hsla) -> AnyElement + 'static,
    ) -> Self {
        Self {
            element: Some(element),
            refine: Some(Box::new(refine)),
        }
    }
}

impl ParentElement for InheritedTextElement {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.element
            .as_mut()
            .expect("before layout")
            .extend(elements);
    }
}

impl Styled for InheritedTextElement {
    fn style(&mut self) -> &mut StyleRefinement {
        self.element.as_mut().expect("before layout").style()
    }
}

impl Element for InheritedTextElement {
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
        let element = self.element.take().expect("inherited text renders once");
        let refine = self.refine.take().expect("inherited text refines once");
        // GPUI invokes hover/focus builder closures immediately. Build those
        // refinements here, after the parent's native style is actually active.
        let mut child = refine(element, window.text_style().color);
        (child.request_layout(window, cx), child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
    }
}

#[cfg(feature = "vendor-gpui")]
impl IntoElement for InheritedTextElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

#[cfg(not(feature = "vendor-gpui"))]
pub(crate) mod public_paint {
    use crate::{
        NodeId,
        paint_motion::PaintTransition,
        presentation_backend::{ComputedPaint, PaintInk},
        renderer::{PaintMotionFn, PaintTextSampleFn},
    };
    use gpui::{
        App, Bounds, Div, Element, ElementId, GlobalElementId, Hitbox, InspectorElementId,
        InteractiveElement, LayoutId, Pixels, Stateful, Styled, Window,
    };
    use std::{rc::Rc, time::Instant};

    pub(crate) struct Binding {
        pub path: String,
        pub owner: NodeId,
        pub transition: PaintTransition,
        pub driver: Rc<PaintMotionFn>,
        pub text_sample: Rc<PaintTextSampleFn>,
        pub now: Instant,
    }

    /// The probe has only the authored style refinements, no children, hitbox,
    /// listeners or ownership. It asks GPUI to resolve them against the live
    /// Div's own element state and hitbox. Only that Div participates in layout,
    /// input dispatch, native paint and accessibility.
    pub(crate) struct ElementWithPaint {
        pub element: Stateful<Div>,
        pub probe: Stateful<Div>,
        pub binding: Binding,
    }

    impl Element for ElementWithPaint {
        type RequestLayoutState = <Stateful<Div> as Element>::RequestLayoutState;
        type PrepaintState = <Stateful<Div> as Element>::PrepaintState;

        fn id(&self) -> Option<ElementId> {
            Element::id(&self.element)
        }

        fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
            self.element.source_location()
        }

        fn a11y_role(&self) -> Option<gpui::Role> {
            self.element.a11y_role()
        }

        fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
            self.element.write_a11y_info(node);
        }

        fn a11y_synthetic_children(
            &mut self,
            state: &mut Self::PrepaintState,
            builder: &mut gpui::A11ySubtreeBuilder<'_>,
        ) {
            self.element.a11y_synthetic_children(state, builder);
        }

        fn request_layout(
            &mut self,
            id: Option<&GlobalElementId>,
            inspector: Option<&InspectorElementId>,
            window: &mut Window,
            cx: &mut App,
        ) -> (LayoutId, Self::RequestLayoutState) {
            let base = self.element.style().text.color;
            let binding = &self.binding;
            let sample = (binding.transition.duration_ms != 0)
                .then(|| {
                    (binding.text_sample)(&binding.path, window.text_style().color, binding.now)
                })
                .flatten();
            let text_color = sample.map_or_else(
                || {
                    self.probe.style().text.color = base;
                    self.probe
                        .interactivity()
                        .compute_style(id, None, window, cx)
                        .text
                        .color
                },
                |sample| sample.0,
            );
            self.element.style().text.color = text_color;
            let layout = self.element.request_layout(id, inspector, window, cx);
            self.element.style().text.color = base;
            layout
        }

        fn prepaint(
            &mut self,
            id: Option<&GlobalElementId>,
            inspector: Option<&InspectorElementId>,
            bounds: Bounds<Pixels>,
            request: &mut Self::RequestLayoutState,
            window: &mut Window,
            cx: &mut App,
        ) -> Self::PrepaintState {
            self.element
                .prepaint(id, inspector, bounds, request, window, cx)
        }

        fn paint(
            &mut self,
            id: Option<&GlobalElementId>,
            inspector: Option<&InspectorElementId>,
            bounds: Bounds<Pixels>,
            request: &mut Self::RequestLayoutState,
            hitbox: &mut Option<Hitbox>,
            window: &mut Window,
            cx: &mut App,
        ) {
            let style = self.element.style();
            let base = (
                style.background.clone(),
                style.border_color,
                style.text.color,
            );
            let probe = self.probe.style();
            (probe.background, probe.border_color, probe.text.color) = base.clone();
            let target = self
                .probe
                .interactivity()
                .compute_style(id, hitbox.as_ref(), window, cx);
            let mut paint = ComputedPaint {
                background: target.background,
                border_color: target.border_color.map(PaintInk::NativeHsla),
                text_color: target.text.color,
                ..ComputedPaint::default()
            };
            let binding = &self.binding;
            (binding.driver)(
                &binding.path,
                binding.owner,
                &mut paint,
                binding.transition,
                binding.now,
                window,
            );

            let parent = window.text_style().color;
            let style = self.element.style();
            style.background = paint.background;
            style.border_color = paint
                .border_color
                .map(|ink| ink.resolve_hsla(paint.text_color.unwrap_or(parent)));
            style.text.color = paint.text_color;
            self.element
                .paint(id, inspector, bounds, request, hitbox, window, cx);
            let style = self.element.style();
            (style.background, style.border_color, style.text.color) = base;
        }
    }
}
