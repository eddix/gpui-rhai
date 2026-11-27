//! Short document flows: one floated figure and one real text subtree per paragraph.

use crate::{
    AnyElement, App, Bounds, Element, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, TextExclusion, TextExclusionSide, Window,
};
use std::{cell::Cell, rc::Rc};

/// A frame-local binding between the flow solver and one native text leaf.
#[derive(Clone, Debug, Default)]
pub struct FloatParagraph(Rc<Paragraph>);

#[derive(Debug, Default)]
struct Paragraph {
    leaf: Cell<Option<LayoutId>>,
    ambiguous: Cell<bool>,
    exclusion: Cell<Option<TextExclusion>>,
}

impl FloatParagraph {
    /// Wrap the actual paragraph. The wrapper adds no layout box or event owner.
    pub fn wrap(&self, child: AnyElement) -> AnyElement {
        FloatElement { child, kind: Kind::Paragraph(self.clone()) }.into_any_element()
    }

    pub(crate) fn bind(&self, leaf: LayoutId) {
        if self.0.leaf.replace(Some(leaf)).is_some() {
            self.0.ambiguous.set(true);
        }
    }
    pub(crate) fn leaf(&self) -> Option<LayoutId> {
        self.0.leaf.get().filter(|_| !self.0.ambiguous.get())
    }
    pub(crate) fn exclusion(&self) -> Option<TextExclusion> {
        self.0.exclusion.get()
    }
    pub(crate) fn set_exclusion(&self, exclusion: Option<TextExclusion>) -> bool {
        self.0.exclusion.replace(exclusion) != exclusion
    }
}

/// Coordinate line wrapping for a full-width column whose first layout child
/// is the figure and whose remaining children are individually bound prose.
#[derive(Clone, Debug)]
pub struct LeadingFloat {
    /// Physical edge on which to place the figure.
    pub side: TextExclusionSide,
    /// Minimum real column width for floating instead of stacking.
    pub min_column_width: Pixels,
    /// Minimum available width next to the laid-out figure.
    pub min_text_width: Pixels,
}

impl LeadingFloat {
    /// Wrap a compact, full-width column (without padding or border). The
    /// figure has an in-flow natural size and paragraphs are full-width prose.
    /// The column's horizontal gap separates text from the figure; the
    /// figure's bottom margin extends its exclusion vertically.
    /// Nested flows or a missing native text leaf fall back to a stacked column.
    pub fn wrap(self, child: AnyElement, paragraphs: Vec<FloatParagraph>) -> AnyElement {
        FloatElement { child, kind: Kind::Group(FlowGroup { options: self, paragraphs }) }.into_any_element()
    }
}

#[derive(Clone)]
pub(crate) struct FlowGroup {
    pub(crate) options: LeadingFloat,
    pub(crate) paragraphs: Vec<FloatParagraph>,
}

enum Kind {
    Paragraph(FloatParagraph),
    Group(FlowGroup),
}

struct FloatElement {
    child: AnyElement,
    kind: Kind,
}

impl IntoElement for FloatElement {
    type Element = Self;
    fn into_element(self) -> Self { self }
}

impl Element for FloatElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<crate::ElementId> { None }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> { None }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let id = match &self.kind {
            Kind::Paragraph(paragraph) => {
                paragraph.0.leaf.set(None);
                paragraph.0.ambiguous.set(false);
                paragraph.0.exclusion.set(None);
                window.with_float_paragraph(paragraph.clone(), |window| self.child.request_layout(window, cx))
            }
            Kind::Group(group) => {
                let id = self.child.request_layout(window, cx);
                window.register_leading_float(id, group.clone());
                id
            }
        };
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) { self.child.prepaint(window, cx); }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) { self.child.paint(window, cx); }
}
