//! Grapheme-based anchors and range rectangles from the very layout GPUI paints.
//!
//! Shaping, wrapping and alignment are inputs, never reconstructed with an
//! independent font or a guessed advance. Geometry lives in the layout
//! coordinate system; any view transform and clip remain the caller's job.

use std::{collections::BTreeMap, ops::Range};

use gpui::{Bounds, Pixels, Point, TextAlign, TextLayout, WrappedLineLayout, point, px, size};
use unicode_bidi::BidiInfo;
use unicode_segmentation::UnicodeSegmentation as _;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaretAffinity {
    /// The trailing edge of the preceding logical cluster.
    Upstream,
    /// The leading edge of the following logical cluster.
    Downstream,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextCaret {
    pub position: Point<Pixels>,
    pub line_height: Pixels,
    /// The corresponding byte offset. A shaper may merge multiple graphemes
    /// into one cluster; in that case this returns the real cluster boundary.
    pub byte_offset: usize,
}

#[derive(Debug)]
struct Cluster {
    bytes: Range<usize>,
    left: Pixels,
    right: Pixels,
    rtl: bool,
}

impl Cluster {
    fn start(&self) -> Pixels {
        if self.rtl { self.right } else { self.left }
    }

    fn end(&self) -> Pixels {
        if self.rtl { self.left } else { self.right }
    }
}

#[derive(Debug)]
struct Row {
    y: Pixels,
    left: Pixels,
    right: Pixels,
    bytes: Range<usize>,
    clusters: Vec<Cluster>,
}

/// Detached geometry for one shape/layout frame. Construct it after the
/// text's prepaint, with the alignment from its actual native text style.
/// No window reference, run handles or text bytes are retained.
#[derive(Debug)]
pub struct TextGeometry {
    rows: Vec<Row>,
    graphemes: Vec<usize>,
    line_height: Pixels,
}

impl TextGeometry {
    /// Project the exact text being painted by a prepainted GPUI layout.
    ///
    /// Uses the text as presented, including any native truncation. Returns
    /// `None` when the native layout cannot be matched to its byte stream.
    ///
    /// # Panics
    ///
    /// The GPUI layout must already have undergone measurement and prepaint.
    #[must_use]
    pub fn from_layout(layout: &TextLayout, align: TextAlign) -> Option<Self> {
        Self::from_painted(layout, align, &layout.text())
    }

    pub(crate) fn from_painted(
        layout: &TextLayout,
        align: TextAlign,
        painted_text: &str,
    ) -> Option<Self> {
        Self::project(
            layout.line_layouts().iter().map(std::sync::Arc::as_ref),
            layout.bounds(),
            layout.line_height(),
            align,
            painted_text,
        )
    }

    fn project<'a>(
        lines: impl IntoIterator<Item = &'a WrappedLineLayout>,
        bounds: Bounds<Pixels>,
        height: Pixels,
        align: TextAlign,
        text: &str,
    ) -> Option<Self> {
        let graphemes = text
            .grapheme_indices(true)
            .map(|(byte, _)| byte)
            .chain([text.len()])
            .collect::<Vec<_>>();
        let mut rows = Vec::new();
        let mut byte = 0_usize;
        let mut y = bounds.top();
        for line in lines {
            let end = byte.checked_add(line.len())?;
            let paragraph = text.get(byte..end)?;
            append_line(
                &mut rows,
                line,
                paragraph,
                byte,
                &graphemes,
                point(bounds.left(), y),
                bounds.size.width,
                height,
                align,
            )?;
            y += line.size(height).height;
            byte = end;
            if text.as_bytes().get(byte) == Some(&b'\n') {
                byte += 1;
            }
        }
        if byte != text.len() {
            return None;
        }
        if rows.is_empty() {
            let x = bounds.left() + align_offset(align, bounds.size.width, px(0.0));
            rows.push(Row {
                y: bounds.top(),
                left: x,
                right: x,
                bytes: 0..0,
                clusters: Vec::new(),
            });
        }
        Some(Self {
            rows,
            graphemes,
            line_height: height,
        })
    }

    /// Convert a grapheme boundary (including the end) to the painted
    /// string's UTF-8 byte offset.
    #[must_use]
    pub fn byte_at_grapheme(&self, index: usize) -> Option<usize> {
        self.graphemes.get(index).copied()
    }

    /// Number of extended grapheme clusters in the presented source.
    #[must_use]
    pub fn grapheme_count(&self) -> usize {
        self.graphemes.len().saturating_sub(1)
    }

    /// Resolve a logical grapheme boundary to the painted cluster edge.
    /// Affinity distinguishes the two valid visual positions at a wrap or
    /// a bidi run boundary. The returned byte may differ inside a ligature.
    #[must_use]
    pub fn caret(&self, grapheme: usize, affinity: CaretAffinity) -> Option<TextCaret> {
        let byte = self.byte_at_grapheme(grapheme)?;
        let mut before = None;
        let mut after = None;
        for row in &self.rows {
            if byte < row.bytes.start || byte > row.bytes.end {
                continue;
            }
            for cluster in &row.clusters {
                if cluster.bytes.start <= byte && byte < cluster.bytes.end {
                    after = Some(self.at(cluster.start(), row.y, cluster.bytes.start));
                }
                if cluster.bytes.start < byte && byte <= cluster.bytes.end {
                    before = Some(self.at(cluster.end(), row.y, cluster.bytes.end));
                }
            }
        }
        let (first, second) = match affinity {
            CaretAffinity::Upstream => (before, after),
            CaretAffinity::Downstream => (after, before),
        };
        let empty = self
            .rows
            .iter()
            .find(|row| row.clusters.is_empty() && row.bytes.start == byte);
        first
            .or_else(|| empty.map(|row| self.at(row.left, row.y, byte)))
            .or(second)
    }

    fn at(&self, x: Pixels, y: Pixels, byte_offset: usize) -> TextCaret {
        TextCaret {
            position: point(x, y),
            line_height: self.line_height,
            byte_offset,
        }
    }

    /// Visual rectangles for the grapheme range. A single logical range may
    /// have several disjoint rectangles even on one visual row.
    #[must_use]
    pub fn range(&self, graphemes: Range<usize>) -> Vec<Bounds<Pixels>> {
        let (Some(start), Some(end)) = (
            self.byte_at_grapheme(graphemes.start),
            self.byte_at_grapheme(graphemes.end),
        ) else {
            return Vec::new();
        };
        self.byte_range(start..end)
    }

    pub(crate) fn byte_range(&self, bytes: Range<usize>) -> Vec<Bounds<Pixels>> {
        if bytes.is_empty() || bytes.end > self.graphemes.last().copied().unwrap_or(0) {
            return Vec::new();
        }
        let mut result: Vec<Bounds<Pixels>> = Vec::new();
        for row in &self.rows {
            if row.bytes.end <= bytes.start || row.bytes.start >= bytes.end {
                continue;
            }
            for cluster in &row.clusters {
                if cluster.bytes.end <= bytes.start
                    || cluster.bytes.start >= bytes.end
                    || cluster.right <= cluster.left
                {
                    continue;
                }
                if let Some(last) = result.last_mut()
                    && last.origin.y == row.y
                    && cluster.left <= last.right() + px(0.01)
                {
                    last.size.width = last.right().max(cluster.right) - last.origin.x;
                } else {
                    result.push(Bounds::new(
                        point(cluster.left, row.y),
                        size(cluster.right - cluster.left, self.line_height),
                    ));
                }
            }
        }
        result
    }

    /// Choose the nearest painted cluster edge to a layout-space point.
    #[must_use]
    pub fn hit(&self, target: Point<Pixels>) -> usize {
        if target.y < self.rows[0].y {
            return 0;
        }
        let last = &self.rows[self.rows.len() - 1];
        if target.y >= last.y + self.line_height {
            return self.graphemes.last().copied().unwrap_or(0);
        }
        let row = &self.rows[self
            .rows
            .partition_point(|row| row.y <= target.y)
            .saturating_sub(1)];
        if row.clusters.is_empty() {
            return row.bytes.start;
        }
        let mut best = (Pixels::MAX, row.bytes.start);
        for cluster in &row.clusters {
            for (x, offset) in [
                (cluster.start(), cluster.bytes.start),
                (cluster.end(), cluster.bytes.end),
            ] {
                let distance = (target.x - x).abs();
                if distance < best.0 {
                    best = (distance, offset);
                }
            }
        }
        best.1
    }

    /// Visual row extents, in paint order; useful for scrolling to anchors.
    pub fn rows(&self) -> impl ExactSizeIterator<Item = Bounds<Pixels>> + '_ {
        self.rows.iter().map(|row| {
            Bounds::new(
                point(row.left, row.y),
                size(row.right - row.left, self.line_height),
            )
        })
    }
}

fn align_offset(align: TextAlign, available: Pixels, actual: Pixels) -> Pixels {
    match align {
        TextAlign::Left => px(0.0),
        TextAlign::Center => (available - actual) / 2.0,
        TextAlign::Right => available - actual,
    }
}

#[allow(clippy::too_many_arguments)]
fn append_line(
    rows: &mut Vec<Row>,
    line: &WrappedLineLayout,
    paragraph: &str,
    start: usize,
    graphemes: &[usize],
    origin: Point<Pixels>,
    width: Pixels,
    height: Pixels,
    align: TextAlign,
) -> Option<()> {
    // Use the same boundary glyphs as both native paint_line implementations:
    // their positions are on the unwrapped line. Subtract the row start
    // and apply per-row alignment, not an offset for the whole paragraph.
    let layout = &*line.unwrapped_layout;
    let mut source_rows = vec![(px(0.0), BTreeMap::<usize, Pixels>::new())];
    let mut wraps = line.wrap_boundaries().iter().peekable();
    let mut logical = BTreeMap::<usize, usize>::new();
    for (run_ix, run) in layout.runs.iter().enumerate() {
        for (glyph_ix, glyph) in run.glyphs.iter().enumerate() {
            if wraps
                .peek()
                .is_some_and(|boundary| boundary.run_ix == run_ix && boundary.glyph_ix == glyph_ix)
            {
                wraps.next();
                source_rows.push((glyph.position.x, BTreeMap::new()));
            }
            if glyph.index >= paragraph.len() {
                continue;
            }
            let byte = start + glyph.index;
            let cluster =
                graphemes[graphemes.partition_point(|&anchor| anchor <= byte) - 1].max(start);
            logical.entry(cluster).or_default();
            let (_, glyphs) = source_rows.last_mut()?;
            glyphs
                .entry(cluster)
                .and_modify(|x| *x = (*x).min(glyph.position.x))
                .or_insert(glyph.position.x);
        }
    }
    let mut next = start + paragraph.len();
    for (&byte, end) in logical.iter_mut().rev() {
        *end = next;
        next = byte;
    }
    let bidi = (!paragraph.is_ascii()).then(|| BidiInfo::new(paragraph, None));
    let mut row_end = source_rows
        .iter()
        .map(|(x, _)| *x)
        .skip(1)
        .chain([layout.width])
        .collect::<Vec<_>>()
        .into_iter();
    let mut y = origin.y;
    for (row_index, (row_start, glyphs)) in source_rows.into_iter().enumerate() {
        let end_x = row_end.next()?;
        let row_width = end_x - row_start;
        let (offset, available) = line.row_interval(row_index, width);
        let left = origin.x + offset + align_offset(align, available, row_width);
        let mut visual = glyphs.into_iter().collect::<Vec<_>>();
        visual.sort_by(|(ab, ax), (bb, bx)| ax.partial_cmp(bx).unwrap_or(ab.cmp(bb)));
        let bytes = visual.iter().map(|(byte, _)| *byte).min().unwrap_or(start)
            ..visual
                .iter()
                .map(|(byte, _)| logical[byte])
                .max()
                .unwrap_or(start + paragraph.len());
        let clusters = visual
            .iter()
            .enumerate()
            .map(|(i, &(byte, x))| Cluster {
                bytes: byte..logical[&byte],
                left: left + x - row_start,
                right: left + visual.get(i + 1).map_or(end_x, |(_, x)| *x) - row_start,
                rtl: bidi
                    .as_ref()
                    .is_some_and(|bidi| bidi.levels[byte - start].is_rtl()),
            })
            .collect();
        rows.push(Row {
            y,
            left,
            right: left + row_width,
            bytes,
            clusters,
        });
        y += height;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{GlyphId, LineLayout, ShapedGlyph, ShapedRun, WrapBoundary};
    use std::sync::Arc;

    fn line(
        text: &str,
        stamps: &[(usize, f32)],
        width: f32,
        wrap: Option<usize>,
    ) -> WrappedLineLayout {
        let glyphs = stamps
            .iter()
            .map(|&(index, x)| ShapedGlyph {
                id: GlyphId(0),
                position: point(px(x), px(0.0)),
                index,
                is_emoji: false,
            })
            .collect();
        WrappedLineLayout {
            row_bands: None,
            unwrapped_layout: Arc::new(LineLayout {
                width: px(width),
                len: text.len(),
                runs: vec![ShapedRun {
                    font_id: gpui::FontId(0),
                    glyphs,
                }],
                ..LineLayout::default()
            }),
            wrap_boundaries: wrap
                .map(|glyph_ix| WrapBoundary {
                    run_ix: 0,
                    glyph_ix,
                })
                .into_iter()
                .collect(),
            wrap_width: None,
        }
    }

    #[test]
    fn wrapped_rows_share_paint_origin_and_keep_both_caret_affinities() {
        let text = "ab cd";
        let line = line(
            text,
            &[(0, 0.0), (1, 6.0), (2, 12.0), (3, 16.0), (4, 22.0)],
            28.0,
            Some(3),
        );
        let bounds = Bounds::new(point(px(10.0), px(20.0)), size(px(45.0), px(36.0)));
        let map = TextGeometry::project([&line], bounds, px(18.0), TextAlign::Right, text).unwrap();
        let before = map.caret(3, CaretAffinity::Upstream).unwrap();
        let after = map.caret(3, CaretAffinity::Downstream).unwrap();
        assert_eq!(before.position, point(px(55.0), px(20.0)));
        assert_eq!(after.position, point(px(43.0), px(38.0)));
        assert_eq!(
            map.range(1..5),
            vec![
                Bounds::new(point(px(45.0), px(20.0)), size(px(10.0), px(18.0))),
                Bounds::new(point(px(43.0), px(38.0)), size(px(12.0), px(18.0))),
            ]
        );
        assert_eq!(map.hit(point(px(44.0), px(41.0))), 3);
    }

    #[test]
    fn bidi_range_does_not_fill_an_unselected_visual_island() {
        let text = "aאבz";
        let line = line(
            text,
            &[(0, 0.0), (3, 6.0), (1, 12.0), (5, 18.0)],
            24.0,
            None,
        );
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(30.0), px(12.0)));
        let map =
            TextGeometry::project([&line], bounds, px(12.0), TextAlign::Center, text).unwrap();
        assert_eq!(
            map.caret(1, CaretAffinity::Upstream).unwrap().position.x,
            px(9.0)
        );
        assert_eq!(
            map.caret(1, CaretAffinity::Downstream).unwrap().position.x,
            px(21.0)
        );
        assert_eq!(
            map.range(0..2),
            vec![
                Bounds::new(point(px(3.0), px(0.0)), size(px(6.0), px(12.0))),
                Bounds::new(point(px(15.0), px(0.0)), size(px(6.0), px(12.0))),
            ]
        );
        assert_eq!(map.hit(point(px(20.0), px(2.0))), 1);
    }

    #[test]
    fn ligature_edges_and_extended_graphemes_are_never_split() {
        let text = "fi🙂e\u{0301}";
        let line = line(
            text,
            &[(0, 0.0), (2, 10.0), (6, 20.0), (7, 23.0)],
            29.0,
            None,
        );
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(30.0), px(12.0)));
        let map = TextGeometry::project([&line], bounds, px(12.0), TextAlign::Left, text).unwrap();
        assert_eq!(map.byte_at_grapheme(1), Some(1));
        assert_eq!(map.byte_at_grapheme(2), Some(2));
        assert_eq!(map.byte_at_grapheme(3), Some(6));
        assert_eq!(map.byte_at_grapheme(4), Some(text.len()));
        assert_eq!(
            map.caret(1, CaretAffinity::Downstream).unwrap().byte_offset,
            0
        );
        assert_eq!(
            map.caret(1, CaretAffinity::Upstream).unwrap().byte_offset,
            2
        );
        assert_eq!(map.range(1..2)[0].size.width, px(10.0));
        assert_eq!(map.range(3..4)[0].size.width, px(9.0));
    }

    #[test]
    fn empty_lines_have_carets_and_do_not_paint_highlight_area() {
        let a = line("A", &[(0, 0.0)], 7.0, None);
        let blank = line("", &[], 0.0, None);
        let b = line("B", &[(0, 0.0)], 6.0, None);
        let bounds = Bounds::new(point(px(3.0), px(5.0)), size(px(50.0), px(36.0)));
        let map = TextGeometry::project(
            [&a, &blank, &b],
            bounds,
            px(12.0),
            TextAlign::Right,
            "A\n\nB",
        )
        .unwrap();
        let empty = map.caret(2, CaretAffinity::Downstream).unwrap();
        assert_eq!(empty.position, point(px(53.0), px(17.0)));
        assert_eq!(empty.byte_offset, 2);
        assert_eq!(map.hit(point(px(40.0), px(19.0))), 2);
        assert_eq!(map.range(1..3), [] as [Bounds<Pixels>; 0]);
    }
}
