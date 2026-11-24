//! The last frame's painted quads in logical pixels. GPUI's scene is in device pixels (the
//! test window draws at scale 2); everything here is divided by the window's scale factor.

use gpui::{Background, Hsla, VisualTestContext, rgba};

/// The same scene evidence from the published backend's bench-support API.
#[cfg(not(feature = "vendor-gpui"))]
pub(crate) trait PublicSceneWitness {
    fn visual_probe_with_painted_scene<T>(&self, visit: impl FnOnce(&gpui::Scene) -> T) -> T;
}

#[cfg(not(feature = "vendor-gpui"))]
impl PublicSceneWitness for gpui::Window {
    fn visual_probe_with_painted_scene<T>(&self, visit: impl FnOnce(&gpui::Scene) -> T) -> T {
        visit(self.rendered_scene())
    }
}

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

/// Walk the live paint stream's scenes with their device-space placement.
/// Isolated surfaces are local; Fast's non-tiled rounded clips are already
/// in the containing scene's coordinates. Tiled overscan is not live paint.
pub(crate) fn walk_live_paint(
    scene: &gpui::Scene,
    placement: gpui::TransformationMatrix,
    visit: &mut impl FnMut(&gpui::Scene, gpui::TransformationMatrix),
) {
    visit(scene, placement);
    #[cfg(feature = "vendor-gpui")]
    for group in &scene.isolated_groups {
        if group.is_renderable() {
            walk_live_paint(group.source(), placement.compose(group.transform()), visit);
        }
    }
    #[cfg(feature = "gpui-fast")]
    for child in scene
        .layers
        .frames
        .iter()
        .filter(|layer| layer.tile_size == 0)
        .flat_map(|layer| layer.content.scenes())
    {
        walk_live_paint(child, placement, visit);
    }
}

pub(crate) fn logical_paint_rect(
    bounds: gpui::Bounds<gpui::ScaledPixels>,
    placement: gpui::TransformationMatrix,
    scale: f32,
) -> Rect {
    let [[a, b], [c, d]] = placement.rotation_scale;
    let [tx, ty] = placement.translation;
    let mut left = f32::INFINITY;
    let mut top = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    let mut bottom = f32::NEG_INFINITY;
    for x in [bounds.left().0, bounds.right().0] {
        for y in [bounds.top().0, bounds.bottom().0] {
            let px = a * x + b * y + tx;
            let py = c * x + d * y + ty;
            left = left.min(px);
            right = right.max(px);
            top = top.min(py);
            bottom = bottom.max(py);
        }
    }
    Rect {
        x: left / scale,
        y: top / scale,
        width: (right - left) / scale,
        height: (bottom - top) / scale,
    }
}

/// Visible solid fills of this color across the live root, clips and groups.
/// Unordered rectangles; group alpha and the final blend are not sampled.
pub(crate) fn live_fills(visual: &mut VisualTestContext, color: u32) -> Vec<Rect> {
    let fill: Background = rgba(color).into();
    visual.update(|window, _| {
        let scale = window.scale_factor();
        window.visual_probe_with_painted_scene(|scene| {
            let mut result = Vec::new();
            walk_live_paint(
                scene,
                gpui::TransformationMatrix::default(),
                &mut |scene, pose| {
                    for quad in scene.quads.iter().filter(|quad| quad.background == fill) {
                        let rect = logical_paint_rect(quad.bounds, pose, scale);
                        let clip = logical_paint_rect(quad.content_mask.bounds, pose, scale);
                        let x = rect.x.max(clip.x);
                        let y = rect.y.max(clip.y);
                        let width = rect.right().min(clip.right()) - x;
                        let height = rect.bottom().min(clip.bottom()) - y;
                        if width > 0. && height > 0. {
                            result.push(Rect {
                                x,
                                y,
                                width,
                                height,
                            });
                        }
                    }
                },
            );
            result
        })
    })
}

/// Painted monochrome glyphs of `ink` whose centers belong to a measured
/// control, in both the live root and its isolated or clipped descendants.
/// A style snapshot cannot prove that an inheriting leaf actually used it.
pub(crate) fn glyphs_with_ink(
    visual: &mut VisualTestContext,
    region: gpui_rhai::GeometryBounds,
    ink: u32,
) -> usize {
    glyph_rects_with_ink(visual, region, ink).len()
}

/// Painted glyph destinations, so a test can compare ink rather than the line boxes.
pub(crate) fn glyph_rects_with_ink(
    visual: &mut VisualTestContext,
    region: gpui_rhai::GeometryBounds,
    ink: u32,
) -> Vec<Rect> {
    #[allow(clippy::cast_possible_truncation)]
    let rect = Rect {
        x: region.x as f32,
        y: region.y as f32,
        width: region.width as f32,
        height: region.height as f32,
    };
    let target = rgba(ink);
    visual.update(|window, _| {
        let scale = window.scale_factor();
        window.visual_probe_with_painted_scene(|scene| {
            let mut visible = Vec::new();
            walk_live_paint(
                scene,
                gpui::TransformationMatrix::default(),
                &mut |scene, pose| {
                    for glyph in &scene.monochrome_sprites {
                        let color = glyph.color.to_rgb();
                        if [
                            color.r - target.r,
                            color.g - target.g,
                            color.b - target.b,
                            color.a - target.a,
                        ]
                        .iter()
                        .any(|diff| diff.abs() > 1. / 255.)
                        {
                            continue;
                        }
                        let glyph_rect = logical_paint_rect(
                            glyph.bounds,
                            pose.compose(glyph.transformation),
                            scale,
                        );
                        let clip = logical_paint_rect(glyph.content_mask.bounds, pose, scale);
                        let (x, y) = (
                            glyph_rect.x + glyph_rect.width / 2.,
                            glyph_rect.y + glyph_rect.height / 2.,
                        );
                        if x >= rect.x
                            && x < rect.right()
                            && y >= rect.y
                            && y < rect.bottom()
                            && x >= clip.x
                            && x < clip.right()
                            && y >= clip.y
                            && y < clip.bottom()
                        {
                            visible.push(glyph_rect);
                        }
                    }
                },
            );
            visible
        })
    })
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

/// The last frame's root-scene quads. Fast moves rounded-clip descendants
/// into non-tiled layer scenes; use `filled_including_rounded_clips` when the
/// tested fill belongs to one of those descendants.
pub(crate) fn painted_quads(visual: &mut VisualTestContext) -> Vec<PaintedQuad> {
    quad_snapshot(visual, false)
}

/// Include Fast's non-tiled rounded-clip descendants, in window coordinates.
pub(crate) fn painted_quads_including_rounded_clips(
    visual: &mut VisualTestContext,
) -> Vec<PaintedQuad> {
    quad_snapshot(visual, true)
}

fn quad_snapshot(visual: &mut VisualTestContext, include_rounded_clips: bool) -> Vec<PaintedQuad> {
    visual.update(|window, _| {
        let scale = window.scale_factor();
        let rect = |bounds: gpui::Bounds<gpui::ScaledPixels>| Rect {
            x: bounds.origin.x.0 / scale,
            y: bounds.origin.y.0 / scale,
            width: bounds.size.width.0 / scale,
            height: bounds.size.height.0 / scale,
        };
        window_quads(window, include_rounded_clips)
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

#[cfg(not(feature = "gpui-fast"))]
fn window_quads(window: &gpui::Window, _: bool) -> Vec<gpui::Quad> {
    window.painted_quads()
}

#[cfg(feature = "gpui-fast")]
fn window_quads(window: &gpui::Window, include_rounded_clips: bool) -> Vec<gpui::Quad> {
    fn append(scene: &gpui::Scene, quads: &mut Vec<gpui::Quad>) {
        quads.extend_from_slice(&scene.quads);
        // Verified against fast0.1.13 rounded_clip_scene::compile: rounded
        // scopes are non-tiled child scenes, in unchanged window coordinates.
        // Their empty geometry-metadata scenes contribute no drawable quads.
        // Tiled scroll content is a separate coordinate/visibility domain.
        for layer in scene
            .layers
            .frames
            .iter()
            .filter(|layer| layer.tile_size == 0)
        {
            if let Some(child) = layer.content.scene() {
                append(child, quads);
            }
        }
    }
    if include_rounded_clips {
        let mut quads = Vec::new();
        #[cfg(feature = "vendor-gpui")]
        let scene = window.painted_scene();
        #[cfg(not(feature = "vendor-gpui"))]
        let scene = window.rendered_scene();
        append(scene, &mut quads);
        quads
    } else {
        window.painted_quads()
    }
}

/// The quads filled with the solid colour `color` (`0xRRGGBBAA`).
pub(crate) fn filled(visual: &mut VisualTestContext, color: u32) -> Vec<PaintedQuad> {
    let fill: Background = rgba(color).into();
    painted_quads(visual)
        .into_iter()
        .filter(|quad| quad.background == fill)
        .collect()
}

/// Matching fills in the root and non-tiled rounded-clip scenes, unordered.
/// Bounds/masks describe primitive geometry, not final rounded-edge pixels.
pub(crate) fn filled_including_rounded_clips(
    visual: &mut VisualTestContext,
    color: u32,
) -> Vec<PaintedQuad> {
    let fill: Background = rgba(color).into();
    painted_quads_including_rounded_clips(visual)
        .into_iter()
        .filter(|quad| quad.background == fill)
        .collect()
}
