//! One viewport-based column plan for the Table header, virtual body and extent.
//!
//! Percentage widths are relative to the viewport, never the scroll extent.
//! Flex columns share the positive viewport space left by fixed/percentage
//! columns. A native width signal makes that column fixed for the current frame.

use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, ScrollHandle, SharedString, Window, point, px,
};
use rhai::{Array, EvalAltResult, FLOAT, INT, Map, Position};

use crate::{
    LayoutLength, Length, NativeSignal, SignalRegistry, SignalValue, StyleProperties, TextDirection,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TableLayout {
    Columns(Vec<TableColumn>),
    Resolved { extent: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TableColumn {
    width: ColumnWidth,
    signal: Option<NativeSignal>,
    pub minimum: f64,
    maximum: Option<f64>,
    minimum_inputs: Vec<ColumnMinimum>,
}

#[derive(Clone, Debug, PartialEq)]
struct ColumnMinimum {
    insets: [Option<Length>; 4],
    width: Option<LayoutLength>,
    maximum: Option<LayoutLength>,
}

impl TableColumn {
    pub(crate) fn include_minimum_style(
        &mut self,
        style: &StyleProperties,
        direction: TextDirection,
    ) {
        let logical = |edges: &crate::EdgeLengths| {
            let (start, end) = if direction == TextDirection::LeftToRight {
                (edges.left, edges.right)
            } else {
                (edges.right, edges.left)
            };
            [edges.start.or(start), edges.end.or(end)]
        };
        let padding = logical(&style.padding);
        let border = logical(&style.border_widths);
        let minimum = ColumnMinimum {
            insets: [padding[0], padding[1], border[0], border[1]],
            width: style.min_width,
            maximum: style.max_width,
        };
        if !self.minimum_inputs.contains(&minimum) {
            self.minimum_inputs.push(minimum);
        }
    }

    fn measure_minimum(&mut self, viewport: f64, rem: f64) {
        self.minimum = self
            .minimum_inputs
            .iter()
            .map(|minimum| {
                let insets = minimum
                    .insets
                    .iter()
                    .map(|value| length_pixels(*value, viewport, rem))
                    .sum::<f64>();
                let declared = match minimum.width {
                    Some(LayoutLength::Definite(value)) => {
                        length_pixels(Some(value), viewport, rem)
                    }
                    _ => 0.0,
                };
                insets.max(declared)
            })
            .reduce(f64::max)
            .unwrap_or(0.0);
        self.maximum = self
            .minimum_inputs
            .iter()
            .filter_map(|minimum| match minimum.maximum {
                Some(LayoutLength::Definite(value)) => {
                    Some(length_pixels(Some(value), viewport, rem).max(self.minimum))
                }
                _ => None,
            })
            .reduce(f64::min);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ColumnWidth {
    Fixed(f64),
    Percent(f64),
    Flex(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ResolvedColumns {
    pub widths: Vec<f64>,
    pub extent: f64,
}

#[allow(
    clippy::unnecessary_box_returns,
    reason = "Rhai native-function error signatures require Box<EvalAltResult>"
)]
pub(crate) fn runtime_error(message: &str) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(message.into(), Position::NONE))
}

pub(crate) fn decode_columns(
    columns: &Array,
    signals: &Array,
) -> Result<Vec<TableColumn>, Box<EvalAltResult>> {
    if columns.is_empty() || columns.len() > 256 {
        return Err(runtime_error("Table requires between 1 and 256 columns"));
    }
    if !signals.is_empty() && signals.len() != columns.len() {
        return Err(runtime_error("Table width signals must match its columns"));
    }
    let mut keys = BTreeSet::new();
    columns
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let column = value
                .clone()
                .try_cast::<Map>()
                .ok_or_else(|| runtime_error("Table column must be a map"))?;
            let key = column
                .get("key")
                .and_then(|value| value.clone().try_cast::<rhai::ImmutableString>())
                .ok_or_else(|| runtime_error("Table column key must be a string"))?;
            if !keys.insert(key) {
                return Err(runtime_error("Table column keys must be unique"));
            }
            let spec = column
                .get("width")
                .and_then(|value| value.clone().try_cast::<Map>())
                .ok_or_else(|| runtime_error("Table column width must be a map"))?;
            let value = spec
                .get("value")
                .and_then(|value| {
                    value.clone().try_cast::<FLOAT>().or_else(|| {
                        value
                            .clone()
                            .try_cast::<INT>()
                            .and_then(|value| value.to_string().parse::<f64>().ok())
                    })
                })
                .filter(|value| value.is_finite() && *value > 0.0)
                .ok_or_else(|| runtime_error("Table column width must be positive and finite"))?;
            let kind = spec
                .get("kind")
                .and_then(|value| value.clone().try_cast::<rhai::ImmutableString>())
                .ok_or_else(|| runtime_error("Table column width kind must be a string"))?;
            let width = match kind.as_str() {
                "fixed" if value <= 16_384.0 => ColumnWidth::Fixed(value),
                "percent" if value <= 100.0 => ColumnWidth::Percent(value),
                "flex" => ColumnWidth::Flex(value),
                _ => {
                    return Err(runtime_error(
                        "Table width is invalid: fixed <= 16384, percent <= 100, or flex",
                    ));
                }
            };
            let signal = signals
                .get(index)
                .filter(|value| !value.is_unit())
                .map(|value| {
                    value.clone().try_cast::<NativeSignal>().ok_or_else(|| {
                        runtime_error("Table width override must be a native signal")
                    })
                })
                .transpose()?;
            if signal
                .as_ref()
                .is_some_and(|signal| signal.id().kind() != crate::SignalKind::OptionalFloat)
            {
                return Err(runtime_error(
                    "Table width override must be an optional-float signal",
                ));
            }
            Ok(TableColumn {
                width,
                signal,
                minimum: 0.0,
                maximum: None,
                minimum_inputs: Vec::new(),
            })
        })
        .collect()
}

pub(crate) fn resolve(
    columns: &[TableColumn],
    viewport: f64,
    signals: &SignalRegistry,
) -> ResolvedColumns {
    let viewport = viewport.max(0.0);
    let mut widths = vec![0.0; columns.len()];
    let mut flexible = Vec::new();
    let mut fixed = 0.0;
    for (index, column) in columns.iter().enumerate() {
        let override_width = column
            .signal
            .as_ref()
            .and_then(|signal| signals.read(signal).ok())
            .and_then(|value| match value {
                SignalValue::OptionalFloat(value) => value,
                _ => None,
            });
        if let Some(width) = override_width {
            widths[index] = width.clamp(0.0, 16_384.0);
        } else {
            match column.width {
                ColumnWidth::Fixed(width) => widths[index] = width,
                ColumnWidth::Percent(percent) => widths[index] = viewport * percent / 100.0,
                ColumnWidth::Flex(weight) => {
                    flexible.push((
                        index,
                        weight,
                        column.minimum,
                        column.maximum.unwrap_or(f64::MAX),
                    ));
                    continue;
                }
            }
        }
        widths[index] = widths[index].clamp(column.minimum, column.maximum.unwrap_or(f64::MAX));
        fixed += widths[index];
    }
    let mut available = (viewport - fixed).max(0.0);
    // Use the sign of the total min/max violation when freezing constraints.
    // Freezing both kinds at once is wrong (e.g. min=34 and max=1 among three
    // equally weighted columns). Re-normalize weights after each frozen set,
    // so even extreme legal weights never require an infinite multiplier.
    while !flexible.is_empty() {
        let maximum_weight = flexible
            .iter()
            .map(|(_, weight, _, _)| *weight)
            .reduce(f64::max)
            .unwrap_or(1.0);
        let weight_sum: f64 = flexible
            .iter()
            .map(|(_, weight, _, _)| weight / maximum_weight)
            .sum();
        let violation: f64 = flexible
            .iter()
            .map(|(_, weight, minimum, maximum)| {
                let requested = available * (weight / maximum_weight) / weight_sum;
                requested.clamp(*minimum, *maximum) - requested
            })
            .sum();
        if violation == 0.0 {
            for (index, weight, minimum, maximum) in flexible {
                widths[index] =
                    (available * (weight / maximum_weight) / weight_sum).clamp(minimum, maximum);
            }
            break;
        }
        flexible.retain(|(index, weight, minimum, maximum)| {
            let requested = available * (weight / maximum_weight) / weight_sum;
            let freeze = if violation > 0.0 {
                requested < *minimum
            } else {
                requested > *maximum
            };
            if freeze {
                widths[*index] = requested.clamp(*minimum, *maximum);
                fixed += widths[*index];
            }
            !freeze
        });
        available = (viewport - fixed).max(0.0);
    }
    ResolvedColumns {
        extent: widths.iter().sum::<f64>().max(viewport),
        widths,
    }
}

#[derive(Default)]
struct ViewportState {
    viewport: f64,
    border: f64,
    maximum: f64,
    direction: Option<TextDirection>,
}

pub(crate) struct TableViewportElement {
    id: ElementId,
    columns: Vec<TableColumn>,
    signals: SignalRegistry,
    direction: TextDirection,
    style: StyleProperties,
    scroll: Option<ScrollHandle>,
    render: Option<Box<dyn FnOnce(ResolvedColumns, f64) -> AnyElement>>,
}

pub(crate) struct TableViewportFrame {
    element: AnyElement,
    state: Rc<RefCell<ViewportState>>,
    extent: f64,
    border: f64,
}

impl TableViewportElement {
    pub(crate) fn new(
        path: &str,
        columns: Vec<TableColumn>,
        signals: SignalRegistry,
        direction: TextDirection,
        style: StyleProperties,
        scroll: Option<ScrollHandle>,
        render: impl FnOnce(ResolvedColumns, f64) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: SharedString::from(format!("{path}/table-viewport")).into(),
            columns,
            signals,
            direction,
            style,
            scroll,
            render: Some(Box::new(render)),
        }
    }
}

impl IntoElement for TableViewportElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for TableViewportElement {
    type RequestLayoutState = TableViewportFrame;
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        window.with_element_state(
            id.expect("Table viewport is keyed"),
            |state: Option<Rc<RefCell<ViewportState>>>, window| {
                let state = state.unwrap_or_default();
                let viewport = state.borrow().viewport;
                for column in &mut self.columns {
                    column.measure_minimum(viewport, f64::from(window.rem_size()));
                }
                let plan = resolve(&self.columns, state.borrow().viewport, &self.signals);
                let extent = plan.extent;
                let border = state.borrow().border;
                let mut element =
                    self.render.take().expect("Table renders once per frame")(plan, border);
                let layout = element.request_layout(window, cx);
                (
                    (
                        layout,
                        TableViewportFrame {
                            element,
                            state: state.clone(),
                            extent,
                            border,
                        },
                    ),
                    state,
                )
            },
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        frame: &mut TableViewportFrame,
        window: &mut Window,
        cx: &mut App,
    ) {
        let viewport = (f64::from(bounds.size.width)
            - horizontal_insets(
                &self.style,
                self.direction,
                f64::from(bounds.size.width),
                f64::from(window.rem_size()),
            ))
        .max(0.0);
        let border = horizontal_edges(
            &self.style.border_widths,
            self.direction,
            f64::from(bounds.size.width),
            f64::from(window.rem_size()),
        );
        // Preserve distance from the logical start against the range actually
        // drawn in this frame, including the first measurement frame. GPUI's
        // scroll maximum includes padding but not its own border widths.
        let maximum = (frame.extent - viewport + frame.border - border).max(0.0);
        let mut state = frame.state.borrow_mut();
        if let Some(scroll) = &self.scroll {
            let old = f64::from(scroll.offset().x);
            let logical = match state.direction {
                None => 0.0,
                Some(TextDirection::LeftToRight) => -old,
                Some(TextDirection::RightToLeft) => state.maximum + old,
            }
            .clamp(0.0, maximum);
            let offset = match self.direction {
                TextDirection::LeftToRight => -logical,
                TextDirection::RightToLeft => logical - maximum,
            };
            scroll.set_offset(point(
                px(crate::renderer::f64_to_f32(offset)),
                scroll.offset().y,
            ));
        }
        let changed =
            (state.viewport - viewport).abs() > 0.01 || (state.border - border).abs() > 0.01;
        state.viewport = viewport;
        state.border = border;
        state.maximum = maximum;
        state.direction = Some(self.direction);
        drop(state);
        frame.element.prepaint(window, cx);
        // The bounded native follow-up frame uses the measured viewport. No
        // component execution or geometry subscription is needed for resizing.
        if changed {
            // refresh() is deliberately a no-op during GPUI prepaint. Demand
            // the next frame for the current view instead of relying on some
            // unrelated input or a test driver's extra refresh.
            window.request_animation_frame();
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        frame: &mut TableViewportFrame,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.element.paint(window, cx);
    }
}

pub(crate) fn horizontal_insets(
    style: &StyleProperties,
    direction: TextDirection,
    width: f64,
    rem: f64,
) -> f64 {
    horizontal_edges(&style.padding, direction, width, rem)
        + horizontal_edges(&style.border_widths, direction, width, rem)
}

fn horizontal_edges(
    edges: &crate::style::EdgeLengths,
    direction: TextDirection,
    width: f64,
    rem: f64,
) -> f64 {
    let (start, end) = if direction == TextDirection::LeftToRight {
        (edges.left, edges.right)
    } else {
        (edges.right, edges.left)
    };
    length_pixels(edges.start.or(start), width, rem) + length_pixels(edges.end.or(end), width, rem)
}

fn length_pixels(value: Option<Length>, width: f64, rem: f64) -> f64 {
    value.map_or(0.0, |value| match value {
        Length::Pixels(value) => value,
        Length::Rems(value) => value * rem,
        Length::Relative(value) => value * width,
        Length::Token(_) => 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn column(width: ColumnWidth) -> TableColumn {
        TableColumn {
            width,
            signal: None,
            minimum: 0.0,
            maximum: None,
            minimum_inputs: Vec::new(),
        }
    }
    #[test]
    fn viewport_is_the_only_percentage_and_flex_basis() {
        let columns = [
            column(ColumnWidth::Fixed(160.0)),
            column(ColumnWidth::Percent(50.0)),
            column(ColumnWidth::Flex(1.0)),
            column(ColumnWidth::Flex(3.0)),
        ];
        let registry = SignalRegistry::default();
        assert_eq!(
            resolve(&columns, 300.0, &registry).widths,
            [160.0, 150.0, 0.0, 0.0]
        );
        let large = resolve(&columns, 800.0, &registry);
        assert_eq!(large.widths, [160.0, 400.0, 60.0, 180.0]);
        assert_eq!(large.extent.to_bits(), 800.0_f64.to_bits());
        assert_eq!(
            resolve(
                &[
                    column(ColumnWidth::Fixed(120.0)),
                    column(ColumnWidth::Fixed(140.0)),
                    column(ColumnWidth::Fixed(160.0))
                ],
                300.0,
                &registry
            )
            .extent
            .to_bits(),
            420.0_f64.to_bits()
        );
    }
    #[test]
    fn unused_viewport_and_extreme_weights_remain_finite() {
        let registry = SignalRegistry::default();
        let small = resolve(&[column(ColumnWidth::Fixed(100.0))], 300.0, &registry);
        assert_eq!(small.widths, [100.0]);
        assert_eq!(small.extent.to_bits(), 300.0_f64.to_bits());
        let large = resolve(
            &[
                column(ColumnWidth::Flex(f64::MAX)),
                column(ColumnWidth::Flex(f64::MAX)),
            ],
            300.0,
            &registry,
        );
        assert_eq!(large.widths, [150.0, 150.0]);
    }

    #[test]
    fn flexible_minima_are_frozen_without_losing_weighted_allocation() {
        let mut first = column(ColumnWidth::Flex(1.0));
        first.minimum = 100.0;
        let other = column(ColumnWidth::Flex(3.0));
        assert_eq!(
            resolve(&[first, other], 300.0, &SignalRegistry::default()).widths,
            [100.0, 200.0]
        );
        let mut narrow = column(ColumnWidth::Flex(1.0));
        narrow.minimum = 48.0;
        let plan = resolve(
            &[
                column(ColumnWidth::Fixed(350.0)),
                column(ColumnWidth::Percent(50.0)),
                narrow,
            ],
            298.0,
            &SignalRegistry::default(),
        );
        assert_eq!(plan.widths, [350.0, 149.0, 48.0]);
        assert_eq!(plan.extent.to_bits(), 547.0_f64.to_bits());
    }

    #[test]
    fn mixed_minimum_and_maximum_constraints_do_not_freeze_the_wrong_column() {
        let mut first = column(ColumnWidth::Flex(1.0));
        first.minimum = 34.0;
        let mut second = column(ColumnWidth::Flex(1.0));
        second.maximum = Some(1.0);
        assert_eq!(
            resolve(
                &[first, second, column(ColumnWidth::Flex(1.0))],
                100.0,
                &SignalRegistry::default()
            )
            .widths,
            [49.5, 1.0, 49.5]
        );
        let mut first = column(ColumnWidth::Flex(1.0));
        first.minimum = 90.0;
        let mut second = column(ColumnWidth::Flex(100.0));
        second.maximum = Some(10.0);
        let plan = resolve(
            &[first, second, column(ColumnWidth::Flex(1.0))],
            100.0,
            &SignalRegistry::default(),
        );
        assert_eq!(plan.widths[0].to_bits(), 90.0_f64.to_bits());
        assert!((plan.widths[1] - 9.900_990_099).abs() < 1e-8);
        assert!((plan.widths[2] - 0.099_009_901).abs() < 1e-8);
    }
}
