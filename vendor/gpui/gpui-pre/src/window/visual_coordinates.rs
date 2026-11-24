//! Immutable layout/visual coordinates shared by input and platform geometry.
//! This is the coordinate half of an isolated paint scope, not a paint transform.
use crate::{Bounds, Pixels, Point, TransformationMatrix, point, px};
use std::rc::Rc;

type Affine = [f64; 6];
const IDENTITY: Affine = [1., 0., 0., 1., 0., 0.];

#[derive(Clone, Debug, PartialEq)]
struct VisualClip {
    inverse: Option<Affine>,
    bounds: Bounds<Pixels>,
}

/// An immutable coordinate snapshot. Matrices map layout-window pixels to
/// visual-window pixels; device scale is applied only at the platform boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct VisualCoordinates {
    forward: Affine,
    inverse: Option<Affine>,
    clips: Rc<[VisualClip]>,
}

impl Default for VisualCoordinates {
    fn default() -> Self {
        Self {
            forward: IDENTITY,
            inverse: Some(IDENTITY),
            clips: Rc::from([]),
        }
    }
}

fn apply(m: Affine, [x, y]: [f64; 2]) -> [f64; 2] {
    [m[0] * x + m[1] * y + m[4], m[2] * x + m[3] * y + m[5]]
}

fn pixels([x, y]: [f64; 2]) -> Option<Point<Pixels>> {
    let [x, y] = [x as f32, y as f32];
    (x.is_finite() && y.is_finite()).then_some(point(px(x), px(y)))
}

fn inverse(m: Affine) -> Option<Affine> {
    let det = m[0] * m[3] - m[1] * m[2];
    if det == 0. || !det.is_finite() {
        return None;
    }
    let [a, b, c, d] = [m[3] / det, -m[1] / det, -m[2] / det, m[0] / det];
    let result = [a, b, c, d, -(a * m[4] + b * m[5]), -(c * m[4] + d * m[5])];
    result.iter().all(|x| x.is_finite()).then_some(result)
}

fn corners(bounds: Bounds<Pixels>) -> Option<[[f64; 2]; 4]> {
    let x = f64::from(bounds.origin.x.0);
    let y = f64::from(bounds.origin.y.0);
    let w = f64::from(bounds.size.width.0);
    let h = f64::from(bounds.size.height.0);
    if ![x, y, w, h].iter().all(|v| v.is_finite()) || w < 0. || h < 0. {
        return None;
    }
    Some([[x, y], [x + w, y], [x + w, y + h], [x, y + h]])
}

fn enclosing(points: &[[f64; 2]]) -> Option<Bounds<Pixels>> {
    if points.is_empty() || !points.iter().flatten().all(|v| v.is_finite()) {
        return None;
    }
    let min = [0, 1].map(|axis| points.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min));
    let max = [0, 1].map(|axis| {
        points
            .iter()
            .map(|p| p[axis])
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let origin = pixels(min)?;
    let end = pixels(max)?;
    let bounds = Bounds::from_corners(origin, end);
    (bounds.size.width.0.is_finite() && bounds.size.height.0.is_finite()).then_some(bounds)
}

impl VisualCoordinates {
    /// Layout-window to visual-window coefficients in row-major order:
    /// `[xx, xy, yx, yy, tx, ty]`.
    pub fn coefficients(&self) -> [f64; 6] {
        self.forward
    }

    /// Whether no transform or transformed clip needs an additional input path.
    pub fn is_identity(&self) -> bool {
        self.forward == IDENTITY && self.clips.is_empty()
    }
    /// Compose a relative layout-window transform. Finite singular transforms
    /// are accepted but have no inverse, hits or platform input rectangles.
    pub fn transformed(&self, transform: TransformationMatrix) -> Result<Self, &'static str> {
        let [[a, b], [c, d]] = transform.rotation_scale;
        let [x, y] = transform.translation;
        if ![a, b, c, d, x, y].iter().all(|v| v.is_finite()) {
            return Err("non-finite visual transform");
        }
        let [a, b, c, d, x, y] = [a, b, c, d, x, y].map(f64::from);
        let m = self.forward;
        let forward = [
            m[0] * a + m[1] * c,
            m[0] * b + m[1] * d,
            m[2] * a + m[3] * c,
            m[2] * b + m[3] * d,
            m[0] * x + m[1] * y + m[4],
            m[2] * x + m[3] * y + m[5],
        ];
        if !forward.iter().all(|v| (*v as f32).is_finite()) {
            return Err("visual transform overflow");
        }
        let inverse = inverse(forward);
        if inverse.is_some_and(|m| !m.iter().all(|v| (*v as f32).is_finite())) {
            return Err("visual inverse exceeds finite render coordinates");
        }
        Ok(Self {
            forward,
            inverse,
            clips: self.clips.clone(),
        })
    }

    /// Capture a rectangular clip in the current layout coordinate domain.
    pub fn clipped_to(&self, bounds: Bounds<Pixels>) -> Self {
        if self
            .clips
            .last()
            .is_some_and(|clip| clip.inverse == self.inverse && clip.bounds == bounds)
        {
            return self.clone();
        }
        let mut clips = self.clips.to_vec();
        clips.push(VisualClip {
            inverse: self.inverse,
            bounds,
        });
        Self {
            clips: clips.into(),
            ..self.clone()
        }
    }

    /// Add a fixed viewport clip expressed in visual-window pixels.
    pub fn clipped_to_visual(&self, bounds: Bounds<Pixels>) -> Self {
        let mut clips = self.clips.to_vec();
        clips.push(VisualClip {
            inverse: Some(IDENTITY),
            bounds,
        });
        Self {
            clips: clips.into(),
            ..self.clone()
        }
    }

    /// Map one global visual-window point back into this layout domain. This
    /// deliberately does not reject clips: text services may ask for the
    /// nearest character outside the visible text rectangle.
    pub fn layout_point(&self, visual: Point<Pixels>) -> Option<Point<Pixels>> {
        if self.is_identity() {
            return Some(visual);
        }
        pixels(apply(
            self.inverse?,
            [f64::from(visual.x.0), f64::from(visual.y.0)],
        ))
    }

    /// Inverse-map a movement vector without applying translation.
    pub fn layout_vector(&self, visual: Point<Pixels>) -> Option<Point<Pixels>> {
        let mut inverse = self.inverse?;
        inverse[4] = 0.;
        inverse[5] = 0.;
        pixels(apply(
            inverse,
            [f64::from(visual.x.0), f64::from(visual.y.0)],
        ))
    }

    /// Map one layout-window point into visual-window coordinates.
    pub fn visual_point(&self, layout: Point<Pixels>) -> Option<Point<Pixels>> {
        pixels(apply(
            self.forward,
            [f64::from(layout.x.0), f64::from(layout.y.0)],
        ))
    }

    /// Test every exact transformed ancestor rectangle, not their bounding boxes.
    pub fn accepts_visual_point(&self, visual: Point<Pixels>) -> bool {
        self.inverse.is_some()
            && self.clips.iter().all(|clip| {
                clip.inverse
                    .and_then(|m| pixels(apply(m, [f64::from(visual.x.0), f64::from(visual.y.0)])))
                    .is_some_and(|p| clip.bounds.contains(&p))
            })
    }

    /// Axis-aligned platform bounds without clipping (for accessibility, which
    /// must keep offscreen nodes available). Singular transforms have no bounds.
    pub fn visual_bounds(&self, bounds: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
        self.inverse?;
        if self.is_identity() {
            corners(bounds)?;
            return Some(bounds);
        }
        enclosing(&corners(bounds)?.map(|p| apply(self.forward, p)))
    }

    /// Clip the transformed polygon exactly before taking its platform AABB.
    /// Degenerate caret rectangles/lines remain valid.
    pub fn clipped_visual_bounds(&self, bounds: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
        self.inverse?;
        if self.is_identity() {
            corners(bounds)?;
            return Some(bounds);
        }
        let mut polygon = corners(bounds)?.map(|p| apply(self.forward, p)).to_vec();
        for clip in self.clips.iter() {
            let m = clip.inverse?;
            let cs = corners(clip.bounds)?;
            for (axis, edge, greater) in [
                (0, cs[0][0], true),
                (0, cs[2][0], false),
                (1, cs[0][1], true),
                (1, cs[2][1], false),
            ] {
                if polygon.is_empty() {
                    return None;
                }
                let input = std::mem::take(&mut polygon);
                let mut previous = *input.last()?;
                let mut prev_value = apply(m, previous)[axis];
                let inside = |value: f64| {
                    if greater {
                        value >= edge
                    } else {
                        value <= edge
                    }
                };
                for current in input {
                    let value = apply(m, current)[axis];
                    if inside(value) != inside(prev_value) {
                        let t = (edge - prev_value) / (value - prev_value);
                        polygon.push([
                            previous[0] + t * (current[0] - previous[0]),
                            previous[1] + t * (current[1] - previous[1]),
                        ]);
                    }
                    if inside(value) {
                        polygon.push(current);
                    }
                    previous = current;
                    prev_value = value;
                }
            }
        }
        enclosing(&polygon)
    }

    pub(super) fn layout_cull_bounds(
        &self,
        parent: &Self,
        bounds: Bounds<Pixels>,
    ) -> Option<Bounds<Pixels>> {
        let inverse = self.inverse?;
        enclosing(&corners(bounds)?.map(|p| apply(inverse, apply(parent.forward, p))))
    }

    pub(crate) fn project_accessibility_node(&self, node: &mut accesskit::Node, scale: f32) {
        let Some(rect) = node.bounds() else {
            return;
        };
        let bounds = Bounds::from_corners(
            point(px(rect.x0 as f32 / scale), px(rect.y0 as f32 / scale)),
            point(px(rect.x1 as f32 / scale), px(rect.y1 as f32 / scale)),
        );
        if let Some(bounds) = self.visual_bounds(bounds) {
            node.set_bounds(accesskit::Rect {
                x0: f64::from(bounds.left().0 * scale),
                y0: f64::from(bounds.top().0 * scale),
                x1: f64::from(bounds.right().0 * scale),
                y1: f64::from(bounds.bottom().0 * scale),
            });
        } else {
            node.clear_bounds();
        }
    }
}
