//! The last frame's painted quads in logical pixels. GPUI's scene is in device pixels (the
//! test window draws at scale 2); everything here is divided by the window's scale factor.

use gpui::{Background, Hsla, VisualTestContext, rgba};

/// `(x, y, width, height)` in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rect {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl Rect {
    pub(crate) fn right(&self) -> f32 {
        self.x + self.width
    }

    pub(crate) fn bottom(&self) -> f32 {
        self.y + self.height
    }
}

/// One painted quad, in logical pixels.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PaintedQuad {
    pub(crate) bounds: Rect,
    /// The content mask the quad was painted under.
    pub(crate) mask: Rect,
    pub(crate) background: Background,
    pub(crate) border_color: Hsla,
    /// Top, right, bottom, left.
    pub(crate) border_widths: [f32; 4],
    /// Top left, top right, bottom right, bottom left.
    pub(crate) corner_radii: [f32; 4],
}

impl PaintedQuad {
    /// The part of the quad inside its content mask; empty sizes are clamped to zero.
    pub(crate) fn visible(&self) -> Rect {
        let x = self.bounds.x.max(self.mask.x);
        let y = self.bounds.y.max(self.mask.y);
        Rect {
            x,
            y,
            width: (self.bounds.right().min(self.mask.right()) - x).max(0.0),
            height: (self.bounds.bottom().min(self.mask.bottom()) - y).max(0.0),
        }
    }

    /// Whether any edge has a border.
    pub(crate) fn has_border(&self) -> bool {
        self.border_widths.iter().any(|width| *width > 0.0)
    }
}

/// Every quad of the last frame, in paint order.
pub(crate) fn painted_quads(visual: &mut VisualTestContext) -> Vec<PaintedQuad> {
    visual.update(|window, _| {
        let scale = window.scale_factor();
        let rect = |bounds: gpui::Bounds<gpui::ScaledPixels>| Rect {
            x: bounds.origin.x.0 / scale,
            y: bounds.origin.y.0 / scale,
            width: bounds.size.width.0 / scale,
            height: bounds.size.height.0 / scale,
        };
        window
            .painted_quads()
            .iter()
            .map(|quad| PaintedQuad {
                bounds: rect(quad.bounds),
                mask: rect(quad.content_mask.bounds),
                background: quad.background,
                border_color: quad.border_color,
                border_widths: [
                    quad.border_widths.top.0 / scale,
                    quad.border_widths.right.0 / scale,
                    quad.border_widths.bottom.0 / scale,
                    quad.border_widths.left.0 / scale,
                ],
                corner_radii: [
                    quad.corner_radii.top_left.0 / scale,
                    quad.corner_radii.top_right.0 / scale,
                    quad.corner_radii.bottom_right.0 / scale,
                    quad.corner_radii.bottom_left.0 / scale,
                ],
            })
            .collect()
    })
}

/// The quads filled with the solid colour `color` (`0xRRGGBBAA`).
pub(crate) fn filled(visual: &mut VisualTestContext, color: u32) -> Vec<PaintedQuad> {
    let fill: Background = rgba(color).into();
    painted_quads(visual)
        .into_iter()
        .filter(|quad| quad.background == fill)
        .collect()
}
