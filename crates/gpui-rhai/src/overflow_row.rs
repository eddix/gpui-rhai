//! Intrinsic, nonwrapping cells. Layout chooses presentation, never script state.

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, ParentElement, Pixels, SharedString, Size, Style,
    Styled, Window, div, px, size,
};
use std::collections::BTreeMap;

pub(crate) const SUMMARY_FOR: &str = "overflow_summary_for";

pub(crate) struct Cell {
    pub element: AnyElement,
    pub priority: Option<u16>,
    pub summary_for: Option<u16>,
}

pub(crate) struct OverflowRowElement {
    pub id: SharedString,
    pub cells: Vec<Cell>,
    /// Only row placement/alignment/gap are copied; the SDK node owns its paint/insets.
    pub row: Style,
}

pub(crate) struct Frame {
    cells: Vec<MeasuredCell>,
    natural: Size<Pixels>,
    visible: Vec<bool>,
}

struct MeasuredCell {
    element: AnyElement,
    size: Size<Pixels>,
    priority: Option<u16>,
    summary_for: Option<u16>,
}

fn count(value: usize) -> f64 {
    f64::from(u32::try_from(value).unwrap_or(u32::MAX))
}

/// Lower priorities collapse first. Equal priorities are one atomic cohort.
fn visible_cells(
    widths: &[f64],
    priorities: &[Option<u16>],
    summaries: &[Option<u16>],
    width: f64,
    gap: f64,
) -> Vec<bool> {
    let mut variants = BTreeMap::new();
    let mut visible = Vec::with_capacity(widths.len());
    let mut live_count = 0_usize;
    let mut used = 0.0;
    for (index, (&width, &summary)) in widths.iter().zip(summaries).enumerate() {
        if let Some(count) = summary {
            variants.entry(usize::from(count)).or_insert(index);
            visible.push(false);
        } else {
            live_count += 1;
            used += width;
            visible.push(true);
        }
    }
    used += gap * count(live_count.saturating_sub(1));
    let mut optional = priorities
        .iter()
        .enumerate()
        .filter(|(index, _)| visible[*index])
        .filter_map(|(index, priority)| priority.map(|priority| (priority, index)))
        .collect::<Vec<_>>();
    optional.sort_unstable();
    let mut omitted = 0;
    let required = live_count - optional.len();
    let summary_width = |omitted, live_count| {
        variants.get(&omitted).map_or(0.0, |index| {
            widths[*index] + if live_count == 0 { 0.0 } else { gap }
        })
    };
    for cohort in optional.chunk_by(|left, right| left.0 == right.0) {
        if used + summary_width(omitted, live_count) <= width {
            break;
        }
        let previous = live_count;
        for &(_, index) in cohort {
            visible[index] = false;
            used -= widths[index];
            live_count -= 1;
            omitted += 1;
        }
        used -= gap * count(previous.saturating_sub(1) - live_count.saturating_sub(1));
    }
    // Required cells are never displaced by an optional summary. A summary
    // wider than the last available room is simply not presented.
    debug_assert!(live_count >= required);
    if let Some(&summary) = variants.get(&omitted)
        && used + summary_width(omitted, live_count) <= width
    {
        visible[summary] = true;
    }
    visible
}

fn gap_pixels(row: &Style, width: f64, window: &Window) -> f64 {
    let pixels = row.gap.width.to_pixels(
        gpui::px(crate::renderer::f64_to_f32(width)).into(),
        window.rem_size(),
    );
    f64::from(match row.gap.width {
        gpui::DefiniteLength::Absolute(_) => window.pixel_snap(pixels),
        gpui::DefiniteLength::Fraction(_) => pixels,
    })
}

fn available(known: Option<Pixels>, space: AvailableSpace, natural: Pixels) -> Pixels {
    known.unwrap_or_else(|| match space {
        AvailableSpace::Definite(limit) => natural.min(limit),
        AvailableSpace::MinContent | AvailableSpace::MaxContent => natural,
    })
}

impl IntoElement for OverflowRowElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for OverflowRowElement {
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
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Frame) {
        // Each actual subtree is measured once and stays its own layout root.
        // Do not reparent its LayoutId after measuring: both forks cache absolute
        // layout bounds. Independent layout cells below only choose positions.
        let cells = std::mem::take(&mut self.cells)
            .into_iter()
            .map(|cell| {
                let mut element = div()
                    .flex()
                    .flex_none()
                    .child(cell.element)
                    .into_any_element();
                let measured = element.layout_as_root(
                    size(AvailableSpace::MaxContent, AvailableSpace::MaxContent),
                    window,
                    cx,
                );
                MeasuredCell {
                    element,
                    size: measured,
                    priority: cell.priority,
                    summary_for: cell.summary_for,
                }
            })
            .collect::<Vec<_>>();
        // A zero-omission variant belongs to the full row's preferred
        // width, although it is still a replaceable summary during fitting.
        let (width, regular_count) = cells
            .iter()
            .filter(|cell| cell.summary_for.is_none())
            .chain(cells.iter().find(|cell| cell.summary_for == Some(0)))
            .fold((0.0, 0_usize), |(width, count), cell| {
                (width + f64::from(cell.size.width), count + 1)
            });
        // A percentage gap has no definite basis during intrinsic measurement;
        // its actual basis is the allocated row width in prepaint, just as in GPUI.
        let gap = gap_pixels(&self.row, 0.0, window);
        let natural = size(
            px(crate::renderer::f64_to_f32(
                width + gap * count(regular_count.saturating_sub(1)),
            )),
            cells
                .iter()
                .map(|cell| cell.size.height)
                .max()
                .unwrap_or_default(),
        );
        let style = Style {
            flex_grow: 1.0,
            min_size: size(px(0.0).into(), gpui::auto()),
            ..Style::default()
        };
        let layout = window.request_measured_layout(style, move |known, space, _, _| {
            size(
                available(known.width, space.width, natural.width),
                available(known.height, space.height, natural.height),
            )
        });
        (
            layout,
            Frame {
                visible: vec![true; cells.len()],
                cells,
                natural,
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
        let widths = frame
            .cells
            .iter()
            .map(|cell| f64::from(cell.size.width))
            .collect::<Vec<_>>();
        let priorities = frame
            .cells
            .iter()
            .map(|cell| cell.priority)
            .collect::<Vec<_>>();
        let gap = gap_pixels(&self.row, f64::from(bounds.size.width), window);
        let summaries = frame
            .cells
            .iter()
            .map(|cell| cell.summary_for)
            .collect::<Vec<_>>();
        frame.visible = visible_cells(
            &widths,
            &priorities,
            &summaries,
            f64::from(bounds.size.width),
            gap,
        );
        let mut row = self.row.clone();
        row.size = size(bounds.size.width.into(), bounds.size.height.into());
        let layouts = frame
            .cells
            .iter()
            .zip(&frame.visible)
            .filter(|(_, visible)| **visible)
            .map(|(cell, _)| {
                window.request_layout(
                    Style {
                        size: size(cell.size.width.into(), cell.size.height.into()),
                        flex_shrink: 0.0,
                        ..Style::default()
                    },
                    [],
                    cx,
                )
            })
            .collect::<Vec<_>>();
        let layout = window.request_layout(row, layouts.iter().copied(), cx);
        window.compute_layout(
            layout,
            size(
                AvailableSpace::Definite(bounds.size.width),
                AvailableSpace::Definite(frame.natural.height),
            ),
            cx,
        );
        let mut layouts = layouts.into_iter();
        for (cell, visible) in frame.cells.iter_mut().zip(&frame.visible) {
            if *visible {
                let origin = window
                    .layout_bounds(layouts.next().expect("one layout per visible cell"))
                    .origin
                    - window.element_offset();
                cell.element.prepaint_at(bounds.origin + origin, window, cx);
            }
        }
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
        for (cell, visible) in frame.cells.iter_mut().zip(&frame.visible) {
            if *visible {
                cell.element.paint(window, cx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::visible_cells;
    #[test]
    fn optional_cells_collapse_by_priority_and_required_cells_are_never_removed() {
        assert_eq!(
            visible_cells(
                &[32.0, 180.0, 32.0],
                &[None, Some(0), None],
                &[None; 3],
                200.0,
                4.0
            ),
            [true, false, true]
        );
        assert_eq!(
            visible_cells(
                &[32.0, 180.0, 32.0],
                &[None, Some(0), None],
                &[None; 3],
                252.0,
                4.0
            ),
            [true, true, true]
        );
        assert_eq!(
            visible_cells(
                &[32.0, 180.0, 32.0],
                &[None, Some(0), None],
                &[None; 3],
                0.0,
                4.0
            ),
            [true, false, true]
        );
        assert_eq!(
            visible_cells(
                &[40.0; 4],
                &[None, Some(0), Some(0), Some(1)],
                &[None; 4],
                100.0,
                4.0
            ),
            [true, false, false, true]
        );
    }

    #[test]
    fn counted_summary_reserves_its_measured_width_and_only_one_variant_is_visible() {
        let widths = [40.0, 40.0, 40.0, 33.0, 29.0, 32.0];
        let priorities = [Some(2), Some(1), Some(0), None, None, None];
        let summaries = [None, None, None, Some(1), Some(2), Some(3)];
        let fit = |width| visible_cells(&widths, &priorities, &summaries, width, 4.0);
        assert_eq!(fit(128.0), [true, true, true, false, false, false]);
        assert_eq!(fit(121.0), [true, true, false, true, false, false]);
        assert_eq!(fit(120.0), [true, false, false, false, true, false]);
        assert_eq!(fit(32.0), [false, false, false, false, false, true]);
        assert_eq!(fit(31.0), [false; 6]);
    }

    #[test]
    fn zero_omission_variant_reserves_width_and_switches_without_dropping_required_cells() {
        let widths = [30., 20., 18., 17.];
        let priorities = [None, Some(0), None, None];
        let variants = [None, None, Some(0), Some(1)];
        let fit = |width| visible_cells(&widths, &priorities, &variants, width, 4.);
        assert_eq!(fit(76.), [true, true, true, false]);
        assert_eq!(fit(75.), [true, false, false, true]);
        assert_eq!(fit(40.), [true, false, false, false]);
    }
}
