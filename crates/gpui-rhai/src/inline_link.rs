//! Inline action geometry and keyboard presentation for native shaped prose.
//! Pointer hits and focus paint use the same line fragments; the native
//! platform receives one accessible action per logical link.

use std::{collections::BTreeMap, ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, Bounds, Hsla, InteractiveElement as _, IntoElement as _, ParentElement as _,
    Pixels, Point, SharedString, StatefulInteractiveElement as _, Styled as _, TextLayout, Window,
    canvas, div, px, size,
};

use crate::{UiValue, text_geometry::TextGeometry};

pub(crate) fn link_payload(destination: String, label: String) -> UiValue {
    UiValue::Map(BTreeMap::from([
        ("href".into(), UiValue::String(destination)),
        ("text".into(), UiValue::String(label)),
    ]))
}

pub(crate) struct InlineLink {
    pub(crate) bytes: Range<usize>,
    pub(crate) label: String,
    pub(crate) destination: String,
    pub(crate) first: bool,
}

/// One projected line range per inline link, in the input's order. Empty
/// entries have no presented range and never route a pointer event.
#[derive(Default)]
pub(crate) struct InlineLinkFrame(pub(crate) Vec<Vec<Bounds<Pixels>>>);

impl InlineLinkFrame {
    pub(crate) fn project(
        layout: &TextLayout,
        source: Option<&str>,
        links: &[InlineLink],
        window: &Window,
    ) -> Self {
        let Some(source) = source.filter(|_| !links.is_empty()) else {
            return Self::default();
        };
        Self::project_ranges(layout, source, links.iter().map(|link| &link.bytes), window)
    }

    pub(crate) fn project_ranges<'a>(
        layout: &TextLayout,
        source: &str,
        ranges: impl Iterator<Item = &'a Range<usize>>,
        window: &Window,
    ) -> Self {
        // An index into the original document isn't valid after a truncating
        // paint path, or if this frame belongs to different source bytes.
        let displayed = layout.text();
        if displayed != source {
            return Self::default();
        }
        let Some(map) = TextGeometry::from_painted(layout, window.text_style().text_align, source)
        else {
            return Self::default();
        };
        Self(ranges.map(|range| map.byte_range(range.clone())).collect())
    }

    pub(crate) fn hit(&self, p: Point<Pixels>) -> Option<usize> {
        self.0.iter().position(|rects| {
            rects.iter().any(|rect| {
                rect.left() <= p.x && p.x < rect.right() && rect.top() <= p.y && p.y < rect.bottom()
            })
        })
    }

    pub(crate) fn focus_target(
        &self,
        index: usize,
        link: &InlineLink,
        focus_color: Hsla,
        activate: impl Fn(&mut Window, &mut App) + 'static,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let rects = self.0.get(index)?;
        let bounds = *rects.first()?;
        let union = rects.iter().skip(1).fold(bounds, |a, b| a.union(b));
        let rects = rects
            .iter()
            .map(|rect| Bounds::new(rect.origin - union.origin, rect.size))
            .collect::<Vec<_>>();
        let activate = Rc::new(activate);
        let via_a11y = activate.clone();
        // This element supplies focus and accessibility but has no pointer
        // handler. The text listener owns pointer routing.
        let mut target = div()
            .id(SharedString::from(format!("link:{}", link.bytes.start)))
            .w(union.size.width)
            .h(union.size.height)
            .role(gpui::Role::Link)
            .aria_label(link.label.clone())
            .aria_description(link.destination.clone())
            .text_color(gpui::transparent_black())
            .tab_index(0)
            .focus_visible(move |style| style.text_color(focus_color))
            .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                via_a11y(window, cx);
            })
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "enter" {
                    activate(window, cx);
                    cx.stop_propagation();
                }
            })
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, (), window, _| {
                        let color = window.text_style().color;
                        if color.a <= 0. {
                            return;
                        }
                        for rect in &rects {
                            paint_outline(
                                Bounds::new(bounds.origin + rect.origin, rect.size),
                                color,
                                window,
                            );
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .into_any_element();
        target.prepaint_as_root(
            union.origin,
            size(
                gpui::AvailableSpace::Definite(union.size.width),
                gpui::AvailableSpace::Definite(union.size.height),
            ),
            window,
            cx,
        );
        Some(target)
    }
}

pub(crate) fn paint_outline(bounds: Bounds<Pixels>, color: Hsla, window: &mut Window) {
    window.paint_quad(gpui::quad(
        bounds,
        gpui::Corners::all(px(2.)),
        gpui::transparent_black(),
        gpui::Edges::all(px(1.)),
        color,
        gpui::BorderStyle::default(),
    ));
}
