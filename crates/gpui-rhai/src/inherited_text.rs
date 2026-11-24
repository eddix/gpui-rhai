//! Read the actual parent text color before a node's own native refinements.

use gpui::{
    AnyElement, App, Bounds, Div, Element, ElementId, GlobalElementId, Hsla, InspectorElementId,
    IntoElement, LayoutId, ParentElement, Pixels, Stateful, StyleRefinement, Styled, Window,
};

type Refine = Box<dyn FnOnce(Stateful<Div>, Hsla) -> Stateful<Div>>;

pub(crate) struct InheritedTextElement {
    element: Option<Stateful<Div>>,
    refine: Option<Refine>,
}

impl InheritedTextElement {
    pub(crate) fn new(
        element: Stateful<Div>,
        refine: impl FnOnce(Stateful<Div>, Hsla) -> Stateful<Div> + 'static,
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
        let mut child = refine(element, window.text_style().color).into_any_element();
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
