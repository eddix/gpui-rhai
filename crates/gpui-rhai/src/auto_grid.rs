//! Responsive equal-width tracks. Width observation is native presentation
//! state, never a Rhai state update or a business data dependency.
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, ParentElement, Pixels, SharedString, Styled, Window, div, px, relative,
};
use serde::{Deserialize, Serialize};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Widths")]
pub struct AutoGridColumns {
    minimum: f64,
    preferred: f64,
    maximum: f64,
}

#[derive(Deserialize)]
struct Widths {
    minimum: f64,
    preferred: f64,
    maximum: f64,
}

impl TryFrom<Widths> for AutoGridColumns {
    type Error = crate::StyleValueError;
    fn try_from(value: Widths) -> Result<Self, Self::Error> {
        Self::new(value.minimum, value.preferred, value.maximum)
    }
}

impl AutoGridColumns {
    /// Logical-pixel track bounds: 1 <= minimum <= preferred <= maximum <= 1e6.
    /// Minimum controls auto-fit breakpoints. Preferred supplies a natural
    /// width hint before measuring the container. Maximum caps track growth.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StyleValueError::InvalidAutoGrid`] for invalid bounds.
    pub fn new(minimum: f64, preferred: f64, maximum: f64) -> Result<Self, crate::StyleValueError> {
        if [minimum, preferred, maximum]
            .iter()
            .all(|value| value.is_finite())
            && minimum >= 1.0
            && minimum <= preferred
            && preferred <= maximum
            && maximum <= 1_000_000.0
        {
            Ok(Self {
                minimum,
                preferred,
                maximum,
            })
        } else {
            Err(crate::StyleValueError::InvalidAutoGrid)
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn plan(self, width: f64, gap: f64, items: usize) -> Plan {
        if items == 0 {
            return Plan {
                columns: 1,
                width: 0.0,
            };
        }
        let capacity = ((width + gap) / (self.minimum + gap)).floor().max(1.0);
        let columns = (capacity.min(1024.0) as usize).min(items).max(1);
        let columns = u16::try_from(columns).expect("bounded above by 1024");
        let gaps = f64::from(columns - 1) * gap;
        let track = ((width - gaps) / f64::from(columns)).clamp(self.minimum, self.maximum);
        Plan {
            columns,
            width: track * f64::from(columns) + gaps,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Plan {
    columns: u16,
    width: f64,
}
impl Plan {
    fn matches(self, other: Self) -> bool {
        self.columns == other.columns && (self.width - other.width).abs() < 0.01
    }
}

pub(crate) struct AutoGridElement {
    pub id: SharedString,
    pub columns: AutoGridColumns,
    pub gap: crate::Length,
    pub rtl: bool,
    pub children: Vec<AnyElement>,
}

pub(crate) struct Frame {
    element: AnyElement,
    observed_width: Rc<Cell<Option<f64>>>,
    plan: Plan,
    count: usize,
}

impl IntoElement for AutoGridElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl AutoGridElement {
    fn gap_pixels(&self, width: f64, window: &Window) -> f64 {
        match self.gap {
            crate::Length::Pixels(value) => value,
            crate::Length::Rems(value) => value * f64::from(window.rem_size()),
            crate::Length::Relative(value) => value * width,
            crate::Length::Token(_) => 0.0,
        }
    }
}

impl Element for AutoGridElement {
    type RequestLayoutState = Frame;
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone().into())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Frame) {
        window.with_element_state(
            id.expect("keyed grid"),
            |state: Option<Rc<Cell<Option<f64>>>>, window| {
                let observed_width = state.unwrap_or_default();
                let count = self.children.len();
                let width = observed_width.get().unwrap_or_else(|| {
                    let viewport = f64::from(window.viewport_size().width);
                    let gap = self.gap_pixels(viewport, window);
                    let tracks = f64::from(u16::try_from(count.min(1024)).expect("bounded tracks"));
                    let natural = tracks * self.columns.preferred + (tracks - 1.0).max(0.0) * gap;
                    natural.min(viewport).max(self.columns.minimum)
                });
                let gap = self.gap_pixels(width, window);
                let plan = self.columns.plan(width, gap, count);
                let children = std::mem::take(&mut self.children)
                    .into_iter()
                    .enumerate()
                    .map(|(index, child)| {
                        let column = index % usize::from(plan.columns);
                        let column = if self.rtl {
                            usize::from(plan.columns) - column
                        } else {
                            column + 1
                        };
                        div()
                            .min_w(px(0.0))
                            .col_start(i16::try_from(column).expect("at most 1024 columns"))
                            .row_start(
                                i16::try_from(index / usize::from(plan.columns) + 1)
                                    .unwrap_or(i16::MAX),
                            )
                            .child(child)
                    });
                let grid = div()
                    .grid()
                    .grid_cols(plan.columns)
                    .w(px(crate::renderer::f64_to_f32(plan.width)))
                    .gap(px(crate::renderer::f64_to_f32(gap)))
                    .children(children);
                let mut root = div().flex().flex_col().w(relative(1.0)).min_w(px(0.0));
                if self.rtl {
                    root = root.items_end();
                }
                let mut element = root.child(grid).into_any_element();
                let layout = element.request_layout(window, cx);
                (
                    (
                        layout,
                        Frame {
                            element,
                            observed_width: observed_width.clone(),
                            plan,
                            count,
                        },
                    ),
                    observed_width,
                )
            },
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        frame: &mut Frame,
        window: &mut Window,
        cx: &mut App,
    ) {
        let width = f64::from(bounds.size.width);
        frame.observed_width.set(Some(width));
        if !frame.plan.matches(self.columns.plan(
            width,
            self.gap_pixels(width, window),
            frame.count,
        )) {
            // One width-refinement frame after mount/resize. All child sizing,
            // intrinsic row heights and aspect ratios remain GPUI's. Cells
            // occupy one track each; spanning belongs to explicit grids.
            // A matching plan schedules nothing: the layout must become idle.
            window.request_animation_frame();
        }
        frame.element.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        frame: &mut Frame,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.element.paint(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tracks_shrink_to_minimum_before_dropping_and_respect_the_cap() {
        let columns = AutoGridColumns::new(128.0, 160.0, 224.0).unwrap();
        for width in 0..2000 {
            let plan = columns.plan(f64::from(width), 16.0, 20);
            let track = (plan.width - 16.0 * f64::from(plan.columns - 1)) / f64::from(plan.columns);
            assert!((128.0..=224.0).contains(&track));
            if width >= 128 {
                assert!(plan.width <= f64::from(width) + 0.001);
            }
        }
        assert_eq!(columns.plan(416.0, 16.0, 20).columns, 3);
        assert_eq!(columns.plan(415.0, 16.0, 20).columns, 2);
        assert!((columns.plan(1000.0, 16.0, 2).width - 464.0).abs() < f64::EPSILON);
        assert!(columns.plan(1000.0, 16.0, 0).width.abs() < f64::EPSILON);
    }
    #[test]
    fn invalid_widths_are_rejected_in_rust_and_serialized_styles() {
        for values in [
            (0.0, 160.0, 224.0),
            (200.0, 160.0, 224.0),
            (128.0, 240.0, 224.0),
            (128.0, 160.0, f64::INFINITY),
        ] {
            assert!(AutoGridColumns::new(values.0, values.1, values.2).is_err());
        }
        assert!(
            serde_json::from_str::<AutoGridColumns>(
                r#"{"minimum":0,"preferred":160,"maximum":224}"#
            )
            .is_err()
        );
    }
}
