//! Fallbacks for presentation operations not exposed by the published backends.
//! The visual profile in Gallery explains which compositing contracts are omitted.

use gpui::{Bounds, BoxShadow, Fill, Hsla, Pixels, Point, Rgba, TransformationMatrix, Window};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PaintInk {
    Literal(Rgba),
    // The alternate native Ink type also exposes this precise color form.
    #[allow(dead_code)]
    NativeHsla(Hsla),
    CurrentText {
        alpha: f32,
    },
}

impl PaintInk {
    pub(crate) fn resolve_hsla(self, foreground: Hsla) -> Hsla {
        match self {
            Self::Literal(color) => color.into(),
            Self::NativeHsla(color) => color,
            Self::CurrentText { alpha } => foreground.opacity(alpha),
        }
    }

    pub(crate) fn resolve(self, foreground: Hsla) -> Rgba {
        self.resolve_hsla(foreground).into()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComputedPaint {
    pub(crate) paint_reveal_x: crate::PaintRevealX,
    pub(crate) text_color: Option<Hsla>,
    pub(crate) border_color: Option<PaintInk>,
    pub(crate) background: Option<Fill>,
    pub(crate) shadows: Vec<BoxShadow>,
    pub(crate) opacity: Option<f32>,
    pub(crate) isolated_opacity: bool,
    pub(crate) foreground_grayscale: f32,
}

impl Default for ComputedPaint {
    fn default() -> Self {
        Self {
            paint_reveal_x: crate::PaintRevealX::Off,
            text_color: None,
            border_color: None,
            background: None,
            shadows: Vec::new(),
            opacity: None,
            isolated_opacity: false,
            foreground_grayscale: 0.,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum TextExclusionSide {
    Left,
    Right,
}

#[derive(Clone, Copy)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct TextExclusion {
    pub(crate) side: TextExclusionSide,
    pub(crate) width: Pixels,
    pub(crate) height: Pixels,
}

/// Only the untransformed logical plane is presented in the stock preview.
#[derive(Clone, Default)]
pub(crate) struct VisualCoordinates(Clip);

#[derive(Clone, Default)]
enum Clip {
    #[default]
    Unbounded,
    Bounds(Bounds<Pixels>),
    Empty,
}

// GPUI's Bounds::intersect clamps disjoint bounds to a degenerate rectangle.
// Here that would create a phantom caret at the edge of an unrelated clip.
fn intersect(a: Bounds<Pixels>, b: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
    let left = a.left().max(b.left());
    let top = a.top().max(b.top());
    let right = a.right().min(b.right());
    let bottom = a.bottom().min(b.bottom());
    (left <= right && top <= bottom)
        .then(|| Bounds::from_corners(gpui::point(left, top), gpui::point(right, bottom)))
}

// Keep the same call shape as the vendored coordinate adapter. The public
// profile deliberately presents only an untransformed coordinate plane.
#[allow(clippy::unused_self, clippy::unnecessary_wraps)]
impl VisualCoordinates {
    pub(crate) fn is_identity(&self) -> bool {
        true
    }
    pub(crate) fn coefficients(&self) -> [f64; 6] {
        [1., 0., 0., 1., 0., 0.]
    }
    pub(crate) fn visual_point(&self, point: Point<Pixels>) -> Option<Point<Pixels>> {
        (f32::from(point.x).is_finite() && f32::from(point.y).is_finite()).then_some(point)
    }
    pub(crate) fn layout_point(&self, point: Point<Pixels>) -> Option<Point<Pixels>> {
        self.visual_point(point)
    }
    pub(crate) fn accepts_visual_point(&self, point: Point<Pixels>) -> bool {
        self.visual_point(point).is_some()
            && match self.0 {
                Clip::Unbounded => true,
                Clip::Bounds(clip) => clip.contains(&point),
                Clip::Empty => false,
            }
    }
    pub(crate) fn visual_bounds(&self, bounds: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
        Some(bounds)
    }
    pub(crate) fn clipped_visual_bounds(&self, bounds: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
        match self.0 {
            Clip::Unbounded => Some(bounds),
            Clip::Bounds(clip) => intersect(bounds, clip),
            Clip::Empty => None,
        }
    }
    pub(crate) fn clipped_to(&self, bounds: Bounds<Pixels>) -> Self {
        Self(match self.0 {
            Clip::Unbounded => Clip::Bounds(bounds),
            Clip::Bounds(clip) => intersect(clip, bounds).map_or(Clip::Empty, Clip::Bounds),
            Clip::Empty => Clip::Empty,
        })
    }
    #[cfg(all(
        target_os = "macos",
        any(feature = "offscreen-pre", feature = "offscreen-fast")
    ))]
    pub(crate) fn clipped_to_visual(&self, bounds: Bounds<Pixels>) -> Self {
        self.clipped_to(bounds)
    }
    pub(crate) fn transformed(self, _: TransformationMatrix) -> Result<Self, &'static str> {
        Ok(self)
    }
}

pub(crate) trait PreviewWindowExt {
    fn visual_coordinates(&self) -> VisualCoordinates;
    fn event_position_in_layout(&self, point: Point<Pixels>) -> Option<Point<Pixels>>;
    fn with_visual_coordinates<R>(
        &mut self,
        matrix: TransformationMatrix,
        f: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, &'static str>;
    fn with_composited_transform<R>(
        &mut self,
        matrix: TransformationMatrix,
        f: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, &'static str>;
    fn paint_effect_group(&mut self, opacity: f32, blur: Pixels, f: impl FnOnce(&mut Self));
}

impl PreviewWindowExt for Window {
    fn visual_coordinates(&self) -> VisualCoordinates {
        // Input dispatch runs outside every draw phase. The stock profile
        // has no transform stack; paint callers that need a clip already
        // pass their mask or viewport to `clipped_to`.
        VisualCoordinates::default()
    }
    fn event_position_in_layout(&self, point: Point<Pixels>) -> Option<Point<Pixels>> {
        self.visual_coordinates().layout_point(point)
    }
    fn with_visual_coordinates<R>(
        &mut self,
        _: TransformationMatrix,
        f: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, &'static str> {
        Ok(f(self))
    }
    fn with_composited_transform<R>(
        &mut self,
        _: TransformationMatrix,
        f: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, &'static str> {
        Ok(f(self))
    }
    fn paint_effect_group(&mut self, opacity: f32, _: Pixels, f: impl FnOnce(&mut Self)) {
        if opacity > 0. {
            f(self);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VisualCoordinates;
    use gpui::{Bounds, point, px, size};

    #[test]
    fn caret_rectangles_survive_a_mask_but_do_not_reappear_through_disjoint_masks() {
        let rect =
            |x, y, width, height| Bounds::new(point(px(x), px(y)), size(px(width), px(height)));
        let coordinates = VisualCoordinates::default().clipped_to(rect(2., 2., 10., 10.));
        assert_eq!(
            coordinates.clipped_visual_bounds(rect(4., 0., 0., 6.)),
            Some(rect(4., 2., 0., 4.))
        );
        assert_eq!(
            coordinates.clipped_visual_bounds(rect(0., 0., 0., 6.)),
            None
        );
        let empty = coordinates.clipped_to(rect(20., 2., 10., 10.));
        assert_eq!(empty.clipped_visual_bounds(rect(20., 4., 0., 6.)), None);
        assert!(!empty.accepts_visual_point(point(px(20.), px(4.))));
    }
}
