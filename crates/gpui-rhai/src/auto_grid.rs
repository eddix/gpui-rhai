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
    minimum: GridWidth,
    preferred: GridWidth,
    maximum: GridWidth,
}

/// One track bound: logical pixels, or a Length (rems or a theme token) the
/// renderer resolves in the node's environment.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum GridWidth {
    Pixels(f64),
    Length(crate::Length),
}

#[derive(Deserialize)]
struct Widths {
    minimum: GridWidth,
    preferred: GridWidth,
    maximum: GridWidth,
}

impl TryFrom<Widths> for AutoGridColumns {
    type Error = crate::StyleValueError;
    fn try_from(value: Widths) -> Result<Self, Self::Error> {
        let length = |width: GridWidth| match width {
            GridWidth::Pixels(value) => crate::Length::Pixels(value),
            GridWidth::Length(length) => length,
        };
        Self::from_lengths(
            length(value.minimum),
            length(value.preferred),
            length(value.maximum),
        )
    }
}

/// Track bounds in logical pixels, every one of them resolved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Tracks {
    minimum: f64,
    preferred: f64,
    maximum: f64,
}

/// The bounds of an [`AutoGridColumns`] after token resolution: pixels or rems.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedColumns {
    minimum: crate::Length,
    preferred: crate::Length,
    maximum: crate::Length,
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
                minimum: GridWidth::Pixels(minimum),
                preferred: GridWidth::Pixels(preferred),
                maximum: GridWidth::Pixels(maximum),
            })
        } else {
            Err(crate::StyleValueError::InvalidAutoGrid)
        }
    }

    /// Track bounds as Lengths: logical pixels, rems or theme tokens, so the
    /// grid follows the same tokens as its cards. Pixel bounds alone are
    /// checked as [`Self::new`] checks them; bounds in rems or tokens are
    /// checked once the renderer resolves them, which lays a grid whose
    /// bounds come out of order at the nearest valid ones and reports a
    /// presentation diagnostic.
    ///
    /// # Errors
    ///
    /// Returns [`crate::StyleValueError::InvalidAutoGrid`] for a relative
    /// bound, or pixel bounds [`Self::new`] rejects.
    pub fn from_lengths(
        minimum: crate::Length,
        preferred: crate::Length,
        maximum: crate::Length,
    ) -> Result<Self, crate::StyleValueError> {
        let pixels = |length: crate::Length| match length {
            crate::Length::Pixels(value) => Some(value),
            _ => None,
        };
        if let (Some(minimum), Some(preferred), Some(maximum)) =
            (pixels(minimum), pixels(preferred), pixels(maximum))
        {
            return Self::new(minimum, preferred, maximum);
        }
        let width = |length: crate::Length| match length {
            crate::Length::Pixels(value)
                if value.is_finite() && (1.0..=1_000_000.0).contains(&value) =>
            {
                Ok(GridWidth::Pixels(value))
            }
            crate::Length::Rems(value) if value.is_finite() && value > 0.0 => {
                Ok(GridWidth::Length(length))
            }
            crate::Length::Token(_) => Ok(GridWidth::Length(length)),
            _ => Err(crate::StyleValueError::InvalidAutoGrid),
        };
        Ok(Self {
            minimum: width(minimum)?,
            preferred: width(preferred)?,
            maximum: width(maximum)?,
        })
    }

    /// The bounds with their tokens resolved by `resolve`; a token it cannot
    /// resolve stays one, which [`ResolvedColumns::tracks`] reports.
    pub(crate) fn resolved(
        self,
        resolve: impl Fn(crate::Length) -> Option<crate::Length>,
    ) -> ResolvedColumns {
        let length = |width: GridWidth| match width {
            GridWidth::Pixels(value) => crate::Length::Pixels(value),
            GridWidth::Length(length) => resolve(length).unwrap_or(length),
        };
        ResolvedColumns {
            minimum: length(self.minimum),
            preferred: length(self.preferred),
            maximum: length(self.maximum),
        }
    }
}

impl ResolvedColumns {
    /// The bounds in logical pixels at `rem` pixels per rem, made valid: the
    /// minimum at least 1, the maximum at least the minimum and at most a
    /// million, the preferred width between them. The error describes what
    /// was adjusted.
    pub(crate) fn tracks(self, rem: f64) -> (Tracks, Option<String>) {
        let pixels = |length: crate::Length| {
            match length {
                crate::Length::Pixels(value) => Some(value),
                crate::Length::Rems(value) => Some(value * rem),
                crate::Length::Relative(_) | crate::Length::Token(_) => None,
            }
            .filter(|value| value.is_finite())
        };
        let (Some(minimum), Some(preferred), Some(maximum)) = (
            pixels(self.minimum),
            pixels(self.preferred),
            pixels(self.maximum),
        ) else {
            let tracks = Tracks {
                minimum: 1.0,
                preferred: 1.0,
                maximum: 1_000_000.0,
            };
            return (
                tracks,
                Some(format!(
                    "auto_grid_columns has a bound that resolved to no width ({:?}, {:?}, {:?}); laid out with a minimum of 1px",
                    self.minimum, self.preferred, self.maximum
                )),
            );
        };
        let valid_minimum = minimum.clamp(1.0, 1_000_000.0);
        let valid_maximum = maximum.clamp(valid_minimum, 1_000_000.0);
        let tracks = Tracks {
            minimum: valid_minimum,
            preferred: preferred.clamp(valid_minimum, valid_maximum),
            maximum: valid_maximum,
        };
        let adjusted =
            (tracks.minimum, tracks.preferred, tracks.maximum) != (minimum, preferred, maximum);
        let error = adjusted.then(|| {
            format!(
                "auto_grid_columns resolved to minimum {minimum}, preferred {preferred}, maximum {maximum}; \
                 expected 1 <= minimum <= preferred <= maximum <= 1000000, laid out with {}, {}, {}",
                tracks.minimum, tracks.preferred, tracks.maximum
            )
        });
        (tracks, error)
    }
}

impl Tracks {
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
    pub columns: ResolvedColumns,
    /// Where a grid laid out at adjusted bounds reports it: the presentation
    /// sink, the node's path and its retained id.
    pub report: Option<(
        crate::presentation_diagnostic::PresentationSink,
        String,
        Option<crate::NodeId>,
    )>,
    pub gap: crate::Length,
    pub rtl: bool,
    pub children: Vec<AnyElement>,
}

pub(crate) struct Frame {
    element: AnyElement,
    observed_width: Rc<Cell<Option<f64>>>,
    plan: Plan,
    tracks: Tracks,
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
                let (tracks, error) = self.columns.tracks(f64::from(window.rem_size()));
                if let Some((sink, path, node)) = &self.report {
                    sink.style_result(path, "auto_grid_columns", *node, error);
                }
                let width = observed_width.get().unwrap_or_else(|| {
                    let viewport = f64::from(window.viewport_size().width);
                    let gap = self.gap_pixels(viewport, window);
                    let count = f64::from(u16::try_from(count.min(1024)).expect("bounded tracks"));
                    let natural = count * tracks.preferred + (count - 1.0).max(0.0) * gap;
                    natural.min(viewport).max(tracks.minimum)
                });
                let gap = self.gap_pixels(width, window);
                let plan = tracks.plan(width, gap, count);
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
                            tracks,
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
        if !frame.plan.matches(frame.tracks.plan(
            width,
            self.gap_pixels(width, window),
            frame.count,
        )) {
            // One width-refinement frame after mount/resize. All child sizing,
            // intrinsic row heights and aspect ratios remain GPUI's. Cells
            // occupy one track each; spanning belongs to explicit grids.
            // A matching plan schedules nothing: the layout must become idle.
            window.request_animation_frame();
            crate::island::mark_unsettled();
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
        let (columns, error) = AutoGridColumns::new(128.0, 160.0, 224.0)
            .unwrap()
            .resolved(Some)
            .tracks(16.0);
        assert_eq!(error, None);
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
    fn length_bounds_resolve_late_and_out_of_order_bounds_are_adjusted() {
        let token = |path| crate::Length::token(path).unwrap();
        let columns = AutoGridColumns::from_lengths(
            token("metrics.cover_min"),
            crate::Length::Rems(10.0),
            crate::Length::Pixels(224.0),
        )
        .unwrap();
        let resolved = columns.resolved(|length| match length {
            crate::Length::Token(_) => Some(crate::Length::Pixels(128.0)),
            length => Some(length),
        });
        let (tracks, error) = resolved.tracks(16.0);
        assert_eq!(error, None);
        assert_eq!(
            (tracks.minimum, tracks.preferred, tracks.maximum),
            (128.0, 160.0, 224.0)
        );
        // A token that resolves above the maximum lays the grid out at it.
        let (tracks, error) = columns
            .resolved(|length| match length {
                crate::Length::Token(_) => Some(crate::Length::Pixels(300.0)),
                length => Some(length),
            })
            .tracks(16.0);
        assert!(error.is_some_and(|error| error.contains("minimum 300")));
        assert_eq!(
            (tracks.minimum, tracks.preferred, tracks.maximum),
            (300.0, 300.0, 300.0)
        );
        // One that does not resolve keeps the grid laid out.
        let (_, error) = columns.resolved(|_| None).tracks(16.0);
        assert!(error.is_some());
        // Pixel bounds alone keep the eager check; relative ones are no track widths.
        assert!(
            AutoGridColumns::from_lengths(
                crate::Length::Pixels(200.0),
                crate::Length::Pixels(160.0),
                token("metrics.cover_min"),
            )
            .is_ok()
        );
        assert!(
            AutoGridColumns::from_lengths(
                crate::Length::Pixels(200.0),
                crate::Length::Pixels(160.0),
                crate::Length::Pixels(224.0),
            )
            .is_err()
        );
        assert!(
            AutoGridColumns::from_lengths(
                crate::Length::Relative(0.5),
                crate::Length::Pixels(160.0),
                crate::Length::Pixels(224.0),
            )
            .is_err()
        );
        // Numbers serialize as numbers; tokens as Lengths, and both read back.
        let json = serde_json::to_string(&columns).unwrap();
        assert!(json.contains(r#""maximum":224.0"#), "{json}");
        assert_eq!(
            serde_json::from_str::<AutoGridColumns>(&json).unwrap(),
            columns
        );
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
