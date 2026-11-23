use std::collections::BTreeSet;
use std::sync::Arc;

use rhai::{
    Array, CustomType, Dynamic, Engine, EvalAltResult, FLOAT, FuncRegistration, INT,
    ImmutableString, Position, TypeBuilder,
};
use thiserror::Error;

use crate::{ColorValue, LinearGradientSpec, Rgba8, StyleValueError};

/// Paint evaluated with its native owner's text-color context.
///
/// Ordinary theme/literal colors convert with `Into`. Contextual colors do not
/// belong to a context-free theme snapshot.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct PaintColor(PaintColorSource);

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "source", content = "value", rename_all = "snake_case")]
pub(crate) enum PaintColorSource {
    Value(ColorValue),
    CurrentText,
    Alpha(
        Box<PaintColor>,
        #[serde(deserialize_with = "paint_alpha")] u16,
    ),
}

fn paint_alpha<'de, D: serde::Deserializer<'de>>(input: D) -> Result<u16, D::Error> {
    let alpha = <u16 as serde::Deserialize>::deserialize(input)?;
    if alpha <= 1000 {
        Ok(alpha)
    } else {
        Err(serde::de::Error::custom("paint alpha exceeds 1000"))
    }
}

impl From<ColorValue> for PaintColor {
    fn from(value: ColorValue) -> Self {
        Self(PaintColorSource::Value(value))
    }
}

impl PaintColor {
    pub(crate) fn source(&self) -> &PaintColorSource {
        &self.0
    }

    pub(crate) fn as_value(&self) -> Option<&ColorValue> {
        match self.source() {
            PaintColorSource::Value(value) => Some(value),
            _ => None,
        }
    }

    /// Keep contextual alpha independent of the 8-bit palette representation.
    pub(crate) fn native_ink(
        &self,
        resolve: &mut impl FnMut(&ColorValue) -> Option<Rgba8>,
    ) -> Option<gpui::PaintInk> {
        use gpui::PaintInk;
        match self.source() {
            PaintColorSource::Value(value) => {
                resolve(value).map(|v| PaintInk::Literal(gpui::rgba(v.as_rgba_hex())))
            }
            PaintColorSource::CurrentText => Some(PaintInk::CurrentText { alpha: 1. }),
            PaintColorSource::Alpha(color, alpha) => {
                let factor = f32::from(*alpha) / 1000.;
                Some(match color.native_ink(resolve)? {
                    PaintInk::Literal(mut v) => {
                        v.a *= factor;
                        PaintInk::Literal(v)
                    }
                    PaintInk::NativeHsla(v) => PaintInk::NativeHsla(v.opacity(factor)),
                    PaintInk::CurrentText { alpha } => PaintInk::CurrentText {
                        alpha: alpha * factor,
                    },
                })
            }
        }
    }

    /// Read the actual owning element's inherited/refined native foreground.
    #[must_use]
    pub const fn current_text() -> Self {
        Self(PaintColorSource::CurrentText)
    }

    /// Multiply paint alpha, preserving the deferred native context.
    ///
    /// # Errors
    /// Returns [`StyleValueError::InvalidColorFactor`] outside zero to one.
    pub fn alpha(self, alpha: f64) -> Result<Self, StyleValueError> {
        Ok(Self(PaintColorSource::Alpha(
            Box::new(self),
            crate::style::thousandths(alpha)?,
        )))
    }

    /// Resolve against a native foreground and the caller's ordinary color
    /// resolver. Resolution allocates nothing and does not retain native state.
    pub fn resolve_with(
        &self,
        current_text: Rgba8,
        resolve_color: &mut dyn FnMut(&ColorValue) -> Option<Rgba8>,
    ) -> Option<Rgba8> {
        match &self.0 {
            PaintColorSource::Value(value) => resolve_color(value),
            PaintColorSource::CurrentText => Some(current_text),
            PaintColorSource::Alpha(color, alpha) => color
                .resolve_with(current_text, resolve_color)
                .map(|value| value.with_alpha_factor(f64::from(*alpha) / 1000.0)),
        }
    }
}

impl CustomType for PaintColor {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("PaintColor");
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CanvasPathSegment {
    Move {
        x: f64,
        y: f64,
    },
    Line {
        x: f64,
        y: f64,
    },
    Quadratic {
        x: f64,
        y: f64,
        control_x: f64,
        control_y: f64,
    },
    Cubic {
        x: f64,
        y: f64,
        control_a_x: f64,
        control_a_y: f64,
        control_b_x: f64,
        control_b_y: f64,
    },
    Close,
}

impl CustomType for CanvasPathSegment {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("CanvasPathSegment");
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CanvasFill {
    Solid(PaintColor),
    LinearGradient(LinearGradientSpec),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasTransform {
    pub translate_x: f64,
    pub translate_y: f64,
    pub scale: f64,
    pub rotate_degrees: f64,
}

impl Default for CanvasTransform {
    fn default() -> Self {
        Self {
            translate_x: 0.0,
            translate_y: 0.0,
            scale: 1.0,
            rotate_degrees: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasClipRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// End shape of an open Canvas stroke. Closed paths have no end caps.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CanvasLineCap {
    #[default]
    Butt,
    Round,
    Square,
}

/// Connection between consecutive Canvas stroke segments.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CanvasLineJoin {
    #[default]
    Miter,
    MiterClip,
    Round,
    Bevel,
}

/// Stroke geometry shared by native paint and hit testing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CanvasStrokeStyle {
    pub cap: CanvasLineCap,
    pub join: CanvasLineJoin,
    pub dash: Option<CanvasStrokeDash>,
}

/// A repeating pair in the same Canvas units as stroke width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasStrokeDash {
    on: f64,
    off: f64,
}

impl CanvasStrokeDash {
    /// Both lengths must be between 0.5 and 4096 Canvas units.
    ///
    /// # Errors
    /// Rejects zero, non-finite and out-of-range intervals.
    pub fn new(on: f64, off: f64) -> Result<Self, CanvasError> {
        if [on, off].iter().all(|v| (0.5..=4096.).contains(v)) {
            Ok(Self { on, off })
        } else {
            Err(CanvasError::InvalidStrokeOption {
                field: "dash",
                value: format!("{on}, {off}"),
            })
        }
    }

    fn apply(
        self,
        builder: gpui::PathBuilder,
        scale: f64,
        length_upper: f64,
    ) -> Option<gpui::PathBuilder> {
        let [on, off] = [self.on, self.off].map(|n| n * scale);
        // Bound the native splitter and guarantee that its f32 cursor advances.
        if !length_upper.is_finite()
            || length_upper > 2048. * (on + off)
            || length_upper > 1.0e7
            || on.min(off) <= f64::from(f32::EPSILON) * length_upper.max(1.) * 2.
            || on.max(off) > f64::from(f32::MAX)
        {
            return None;
        }
        Some(builder.dash_array(&[on, off].map(|n| gpui::px(crate::renderer::f64_to_f32(n)))))
    }
}

impl CanvasStrokeStyle {
    fn native(self, width: f64) -> Option<gpui::StrokeOptions> {
        if !width.is_finite()
            || !(f64::from(f32::MIN_POSITIVE)..=f64::from(f32::MAX)).contains(&width)
        {
            return None;
        }
        let cap = match self.cap {
            CanvasLineCap::Butt => lyon::path::LineCap::Butt,
            CanvasLineCap::Round => lyon::path::LineCap::Round,
            CanvasLineCap::Square => lyon::path::LineCap::Square,
        };
        let join = match self.join {
            CanvasLineJoin::Miter => lyon::path::LineJoin::Miter,
            CanvasLineJoin::MiterClip => lyon::path::LineJoin::MiterClip,
            CanvasLineJoin::Round => lyon::path::LineJoin::Round,
            CanvasLineJoin::Bevel => lyon::path::LineJoin::Bevel,
        };
        Some(
            gpui::StrokeOptions::default()
                .with_line_width(crate::renderer::f64_to_f32(width))
                .with_line_cap(cap)
                .with_line_join(join),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CanvasCommand {
    Rect {
        key: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: PaintColor,
    },
    Circle {
        key: String,
        center_x: f64,
        center_y: f64,
        radius: f64,
        fill: PaintColor,
    },
    Line {
        key: String,
        from_x: f64,
        from_y: f64,
        to_x: f64,
        to_y: f64,
        width: f64,
        color: PaintColor,
        stroke_style: CanvasStrokeStyle,
    },
    Path {
        key: String,
        segments: Vec<CanvasPathSegment>,
        fill: Option<CanvasFill>,
        stroke: Option<(PaintColor, f64)>,
        stroke_style: CanvasStrokeStyle,
        transform: CanvasTransform,
        clip: Option<CanvasClipRect>,
    },
    MorphPath {
        key: String,
        from: Vec<CanvasPathSegment>,
        to: Vec<CanvasPathSegment>,
        stroke: (PaintColor, f64),
        stroke_style: CanvasStrokeStyle,
        transform: CanvasTransform,
        clip: Option<CanvasClipRect>,
    },
    MorphFillPath {
        key: String,
        from: Vec<CanvasPathSegment>,
        to: Vec<CanvasPathSegment>,
        fill: CanvasFill,
        transform: CanvasTransform,
        clip: Option<CanvasClipRect>,
    },
}

/// A retained, bounded set of polyline vertices in Canvas layout coordinates.
/// The peak is the earliest highest vertex at the sampled progress.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasPeakRoute {
    from: Arc<[[f64; 2]]>,
    to: Arc<[[f64; 2]]>,
    transform: CanvasTransform,
    project_y: Option<[f64; 4]>,
}

impl CanvasPeakRoute {
    /// Sample the highest vertex in the path's authored coordinate space,
    /// after the command transform. Apply an optional vertical display
    /// projection only to the winner, not to the ranking of samples.
    #[must_use]
    pub fn point(&self, progress: f64) -> [f64; 2] {
        self.project_point(self.source_point(progress))
    }

    /// Map an authored ordinate range to the display range, saturating at
    /// its two ends. Order and over-limit paint still use authored values.
    ///
    /// # Errors
    /// Requires ordered ranges and finite, native-sized endpoints.
    pub fn with_y_projection(
        mut self,
        source_top: f64,
        source_bottom: f64,
        display_top: f64,
        display_bottom: f64,
    ) -> Result<Self, CanvasError> {
        let map = [source_top, source_bottom, display_top, display_bottom];
        if !map.iter().all(|v| v.is_finite() && v.abs() <= 1.0e7)
            || source_top >= source_bottom
            || display_top > display_bottom
        {
            return Err(CanvasError::InvalidPeakProjection);
        }
        self.project_y = Some(map);
        Ok(self)
    }

    fn project_point(&self, [x, y]: [f64; 2]) -> [f64; 2] {
        let Some([top, bottom, display_top, display_bottom]) = self.project_y else {
            return [x, y];
        };
        let fraction = (y.clamp(top, bottom) - top) / (bottom - top);
        [x, display_top + (display_bottom - display_top) * fraction]
    }

    fn source_point(&self, progress: f64) -> [f64; 2] {
        let p = if progress.is_finite() {
            progress.clamp(0., 1.)
        } else {
            1.
        };
        let mut best = [0., f64::INFINITY];
        for ([ax, ay], [bx, by]) in self.from.iter().copied().zip(self.to.iter().copied()) {
            // As with threshold bands, keep committed endpoint values
            // exact: a large change can round away from an equal limit.
            let blend = |a, b| if p >= 1. { b } else { a + (b - a) * p };
            let (x, y) = canvas_transform_point(self.transform, blend(ax, bx), blend(ay, by));
            if y < best[1] {
                best = [x, y];
            }
        }
        best
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CanvasPeakBinding {
    route: CanvasPeakRoute,
    offset: [f64; 2],
    bounds: Option<[f64; 4]>,
    paint_above: Option<CanvasPeakPaint>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CanvasPeakPaint {
    limit_y: f64,
    pub(crate) foreground: ColorValue,
    pub(crate) background: Option<ColorValue>,
}

impl CanvasPeakBinding {
    pub(crate) fn new(
        route: CanvasPeakRoute,
        offset: [f64; 2],
        bounds: Option<[f64; 4]>,
    ) -> Result<Self, CanvasError> {
        if !offset.iter().all(|v| v.is_finite() && v.abs() <= 1.0e7)
            || bounds.is_some_and(|[lx, ly, rx, ry]| {
                ![lx, ly, rx, ry]
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= 1.0e7)
                    || lx > rx
                    || ly > ry
            })
        {
            return Err(CanvasError::InvalidPeakBounds);
        }
        Ok(Self {
            route,
            offset,
            bounds,
            paint_above: None,
        })
    }

    pub(crate) fn with_paint_above(
        mut self,
        limit_y: f64,
        foreground: ColorValue,
        background: Option<ColorValue>,
    ) -> Result<Self, CanvasError> {
        if !limit_y.is_finite() || limit_y.abs() > 1.0e7 {
            return Err(CanvasError::InvalidPeakBounds);
        }
        self.paint_above = Some(CanvasPeakPaint {
            limit_y,
            foreground,
            background,
        });
        Ok(self)
    }

    pub(crate) fn sample(&self, progress: f64) -> ([f64; 2], Option<&CanvasPeakPaint>) {
        let raw = self.route.source_point(progress);
        let paint = self
            .paint_above
            .as_ref()
            .filter(|paint| raw[1] < paint.limit_y);
        let [x, y] = self.route.project_point(raw);
        let (x, y) = (x + self.offset[0], y + self.offset[1]);
        let offset = self.bounds.map_or([x, y], |[lx, ly, rx, ry]| {
            [x.clamp(lx, rx), y.clamp(ly, ry)]
        });
        (offset, paint)
    }

    pub(crate) fn complexity(&self) -> usize {
        self.route.from.len().saturating_add(self.route.to.len())
    }
}

fn projected_distance(a: gpui::Point<gpui::Pixels>, b: gpui::Point<gpui::Pixels>) -> f64 {
    (f64::from(a.x) - f64::from(b.x)).hypot(f64::from(a.y) - f64::from(b.y))
}

// The control polygon bounds a Bézier arc. Compute it from the same
// projected points the native tessellator will receive, not from its AABB.
fn projected_length_upper(
    segments: &[CanvasPathSegment],
    point: &impl Fn(f64, f64) -> Option<gpui::Point<gpui::Pixels>>,
) -> Option<f64> {
    let mut length = 0.;
    let mut last = None;
    let mut start = None;
    let mut add = |last: &mut Option<_>, next| {
        if let Some(previous) = last.replace(next) {
            length += projected_distance(previous, next);
        }
    };
    for segment in segments {
        match *segment {
            CanvasPathSegment::Move { x, y } => {
                start = Some(point(x, y)?);
                last = start;
            }
            CanvasPathSegment::Line { x, y } => add(&mut last, point(x, y)?),
            CanvasPathSegment::Quadratic {
                x,
                y,
                control_x,
                control_y,
            } => {
                add(&mut last, point(control_x, control_y)?);
                add(&mut last, point(x, y)?);
            }
            CanvasPathSegment::Cubic {
                x,
                y,
                control_a_x,
                control_a_y,
                control_b_x,
                control_b_y,
            } => {
                add(&mut last, point(control_a_x, control_a_y)?);
                add(&mut last, point(control_b_x, control_b_y)?);
                add(&mut last, point(x, y)?);
            }
            CanvasPathSegment::Close => add(&mut last, start?),
        }
    }
    Some(length)
}

impl CanvasCommand {
    /// Get a motion-ready peak route for a single open polyline.
    ///
    /// # Errors
    /// Rejects non-polylines, unmatched endpoints or more than 1024 vertices.
    pub fn peak_route(&self) -> Result<CanvasPeakRoute, CanvasError> {
        let (from, to, transform) = match self {
            Self::Path {
                segments,
                transform,
                ..
            } => (segments, segments, *transform),
            Self::MorphPath {
                from,
                to,
                transform,
                ..
            } => (from, to, *transform),
            _ => return Err(CanvasError::InvalidPeakPath),
        };
        if from.len() < 2 || from.len() > 1024 || !compatible_path_topology(from, to) {
            return Err(CanvasError::InvalidPeakPath);
        }
        let vertices = |segments: &[CanvasPathSegment]| -> Result<Arc<[[f64; 2]]>, CanvasError> {
            segments
                .iter()
                .enumerate()
                .map(|(i, segment)| {
                    let (x, y) = match *segment {
                        CanvasPathSegment::Move { x, y } if i == 0 => (x, y),
                        CanvasPathSegment::Line { x, y } if i > 0 => (x, y),
                        _ => return Err(CanvasError::InvalidPeakPath),
                    };
                    let (tx, ty) = canvas_transform_point(transform, x, y);
                    if ![x, y, tx, ty]
                        .iter()
                        .all(|v| v.is_finite() && v.abs() <= 1.0e7)
                    {
                        return Err(CanvasError::InvalidPeakPath);
                    }
                    Ok([x, y])
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Arc::from)
        };
        let a = vertices(from)?;
        let b = if std::ptr::eq(from, to) {
            a.clone()
        } else {
            vertices(to)?
        };
        Ok(CanvasPeakRoute {
            from: a,
            to: b,
            transform,
            project_y: None,
        })
    }
    /// One native tessellation path for paint and geometric stroke hits. The
    /// caller projects source coordinates into the presented drawable; stroke
    /// width remains in paint space, independent from command/node transforms.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn native_path(
        &self,
        progress: Option<f64>,
        stroke_scale: f64,
        project: impl Fn(f64, f64) -> gpui::Point<gpui::Pixels>,
    ) -> Option<gpui::Path<gpui::Pixels>> {
        let projected = |x: f64, y: f64| {
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let point = project(x, y);
            (f32::from(point.x).is_finite() && f32::from(point.y).is_finite()).then_some(point)
        };
        let interpolated;
        let (segments, stroke, style, transform, morph) = match self {
            Self::Line {
                from_x,
                from_y,
                to_x,
                to_y,
                width,
                stroke_style,
                ..
            } => {
                let mut builder = gpui::PathBuilder::default().with_style(gpui::PathStyle::Stroke(
                    stroke_style.native(width * stroke_scale)?,
                ));
                let start = projected(*from_x, *from_y)?;
                let end = projected(*to_x, *to_y)?;
                if let Some(dash) = stroke_style.dash {
                    builder = dash.apply(builder, stroke_scale, projected_distance(start, end))?;
                }
                builder.move_to(start);
                builder.line_to(end);
                return builder.build().ok();
            }
            Self::Path {
                segments,
                stroke,
                stroke_style,
                transform,
                ..
            } => (
                segments.as_slice(),
                stroke.as_ref(),
                *stroke_style,
                *transform,
                false,
            ),
            Self::MorphPath {
                from,
                to,
                stroke,
                stroke_style,
                transform,
                ..
            } => {
                interpolated = interpolate_path(from, to, progress.unwrap_or(0.0))?;
                (
                    interpolated.as_slice(),
                    Some(stroke),
                    *stroke_style,
                    *transform,
                    true,
                )
            }
            Self::MorphFillPath {
                from,
                to,
                transform,
                ..
            } => {
                interpolated = interpolate_path(from, to, progress.unwrap_or(0.0))?;
                (
                    interpolated.as_slice(),
                    None,
                    CanvasStrokeStyle::default(),
                    *transform,
                    true,
                )
            }
            _ => return None,
        };
        let point = |x, y| {
            let (x, y) = canvas_transform_point(transform, x, y);
            projected(x, y)
        };
        let mut builder = gpui::PathBuilder::default();
        if let Some((_, width)) = stroke {
            builder =
                builder.with_style(gpui::PathStyle::Stroke(style.native(width * stroke_scale)?));
            if let Some(dash) = style.dash {
                builder = dash.apply(
                    builder,
                    stroke_scale,
                    projected_length_upper(segments, &point)?,
                )?;
            }
        }
        if !morph && stroke.is_some() && progress.is_some_and(|value| value < 1.0) {
            for path in trimmed_canvas_paths(
                flatten_path(segments, transform),
                progress.unwrap_or(1.0).clamp(0.0, 1.0),
            ) {
                if let Some((first, rest)) = path.split_first() {
                    builder.move_to(projected(first.0, first.1)?);
                    for &(x, y) in rest {
                        builder.line_to(projected(x, y)?);
                    }
                }
            }
        } else {
            for segment in segments {
                match *segment {
                    CanvasPathSegment::Move { x, y } => builder.move_to(point(x, y)?),
                    CanvasPathSegment::Line { x, y } => builder.line_to(point(x, y)?),
                    CanvasPathSegment::Quadratic {
                        x,
                        y,
                        control_x,
                        control_y,
                    } => builder.curve_to(point(x, y)?, point(control_x, control_y)?),
                    CanvasPathSegment::Cubic {
                        x,
                        y,
                        control_a_x,
                        control_a_y,
                        control_b_x,
                        control_b_y,
                    } => builder.cubic_bezier_to(
                        point(x, y)?,
                        point(control_a_x, control_a_y)?,
                        point(control_b_x, control_b_y)?,
                    ),
                    CanvasPathSegment::Close => builder.close(),
                }
            }
        }
        builder.build().ok()
    }

    fn tessellated_hit(
        &self,
        x: f64,
        y: f64,
        progress: Option<f64>,
        stroke_scale: f64,
        project: impl Fn(f64, f64) -> gpui::Point<gpui::Pixels>,
    ) -> bool {
        self.native_path(progress, stroke_scale, project)
            .is_some_and(|path| {
                path.vertices.as_chunks::<3>().0.iter().any(|triangle| {
                    let [first, second, third] = triangle.each_ref().map(|vertex| {
                        let point = vertex.xy_position;
                        (f64::from(point.x), f64::from(point.y))
                    });
                    let cross = |start: (f64, f64), end: (f64, f64), point: (f64, f64)| {
                        (end.0 - start.0)
                            .mul_add(point.1 - start.1, -(end.1 - start.1) * (point.0 - start.0))
                    };
                    if cross(first, second, third).abs() <= f64::EPSILON {
                        return false;
                    }
                    let sides = [
                        cross(first, second, (x, y)),
                        cross(second, third, (x, y)),
                        cross(third, first, (x, y)),
                    ];
                    sides.iter().all(|v| *v >= -1e-9) || sides.iter().all(|v| *v <= 1e-9)
                })
            })
    }

    fn path_clip(&self) -> Option<CanvasClipRect> {
        match self {
            Self::Path { clip, .. }
            | Self::MorphPath { clip, .. }
            | Self::MorphFillPath { clip, .. } => *clip,
            _ => None,
        }
    }
    /// Replace cap/join geometry on a line or stroked path.
    ///
    /// # Errors
    /// Returns an error for commands that have no stroke.
    pub fn with_stroke_style(mut self, style: CanvasStrokeStyle) -> Result<Self, CanvasError> {
        match &mut self {
            Self::Line { stroke_style, .. }
            | Self::MorphPath { stroke_style, .. }
            | Self::Path {
                stroke: Some(_),
                stroke_style,
                ..
            } => *stroke_style = style,
            _ => return Err(CanvasError::StrokeDecorationOnly),
        }
        Ok(self)
    }

    /// Apply a repeating dash to the projected stroke, including morph frames.
    ///
    /// # Errors
    /// The command must have a stroke.
    pub fn with_stroke_dash(self, dash: CanvasStrokeDash) -> Result<Self, CanvasError> {
        let mut style = self.stroke_style()?;
        style.dash = Some(dash);
        self.with_stroke_style(style)
    }

    fn stroke_style(&self) -> Result<CanvasStrokeStyle, CanvasError> {
        match self {
            Self::Line { stroke_style, .. }
            | Self::MorphPath { stroke_style, .. }
            | Self::Path {
                stroke: Some(_),
                stroke_style,
                ..
            } => Ok(*stroke_style),
            _ => Err(CanvasError::StrokeDecorationOnly),
        }
    }

    fn stroke_cap(self, cap: &str) -> Result<Self, CanvasError> {
        let mut style = self.stroke_style()?;
        style.cap = match cap {
            "butt" => CanvasLineCap::Butt,
            "round" => CanvasLineCap::Round,
            "square" => CanvasLineCap::Square,
            _ => {
                return Err(CanvasError::InvalidStrokeOption {
                    field: "cap",
                    value: cap.to_owned(),
                });
            }
        };
        self.with_stroke_style(style)
    }

    fn stroke_join(self, join: &str) -> Result<Self, CanvasError> {
        let mut style = self.stroke_style()?;
        style.join = match join {
            "miter" => CanvasLineJoin::Miter,
            "miter_clip" => CanvasLineJoin::MiterClip,
            "round" => CanvasLineJoin::Round,
            "bevel" => CanvasLineJoin::Bevel,
            _ => {
                return Err(CanvasError::InvalidStrokeOption {
                    field: "join",
                    value: join.to_owned(),
                });
            }
        };
        self.with_stroke_style(style)
    }
    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::Rect { key, .. }
            | Self::Circle { key, .. }
            | Self::Line { key, .. }
            | Self::Path { key, .. }
            | Self::MorphPath { key, .. }
            | Self::MorphFillPath { key, .. } => key,
        }
    }

    #[must_use]
    pub fn complexity(&self) -> usize {
        match self {
            Self::Path { segments, .. } => 1usize.saturating_add(segments.len()),
            Self::MorphPath { from, to, .. } | Self::MorphFillPath { from, to, .. } => {
                1usize.saturating_add(from.len()).saturating_add(to.len())
            }
            Self::Rect { .. } | Self::Circle { .. } | Self::Line { .. } => 1,
        }
    }

    fn transform_mut(&mut self) -> Result<&mut CanvasTransform, CanvasError> {
        match self {
            Self::Path { transform, .. }
            | Self::MorphPath { transform, .. }
            | Self::MorphFillPath { transform, .. } => Ok(transform),
            _ => Err(CanvasError::PathDecorationOnly),
        }
    }

    fn translate(mut self, x: f64, y: f64) -> Result<Self, CanvasError> {
        let transform = self.transform_mut()?;
        transform.translate_x = finite_value("path.translate_x", x)?;
        transform.translate_y = finite_value("path.translate_y", y)?;
        Ok(self)
    }

    fn scale(mut self, value: f64) -> Result<Self, CanvasError> {
        self.transform_mut()?.scale = positive_value("path.scale", value)?;
        Ok(self)
    }

    fn rotate(mut self, degrees: f64) -> Result<Self, CanvasError> {
        self.transform_mut()?.rotate_degrees = finite_value("path.rotate", degrees)?;
        Ok(self)
    }

    fn clip_rect(mut self, x: f64, y: f64, width: f64, height: f64) -> Result<Self, CanvasError> {
        let clip = CanvasClipRect {
            x: finite_value("path.clip.x", x)?,
            y: finite_value("path.clip.y", y)?,
            width: positive_value("path.clip.width", width)?,
            height: positive_value("path.clip.height", height)?,
        };
        match &mut self {
            Self::Path { clip: target, .. }
            | Self::MorphPath { clip: target, .. }
            | Self::MorphFillPath { clip: target, .. } => {
                *target = Some(clip);
            }
            _ => return Err(CanvasError::PathDecorationOnly),
        }
        Ok(self)
    }
}

impl CustomType for CanvasCommand {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("CanvasCommand")
            .with_fn("peak_route", |command: &mut Self| {
                command
                    .peak_route()
                    .map_err(|e| Box::new(canvas_runtime_error(&e)))
            })
            .with_fn(
                "stroke_dash",
                |command: &mut Self, on: Dynamic, off: Dynamic| {
                    let dash =
                        CanvasStrokeDash::new(number("dash.on", &on)?, number("dash.off", &off)?)
                            .map_err(|e| Box::new(canvas_runtime_error(&e)))?;
                    command
                        .clone()
                        .with_stroke_dash(dash)
                        .map_err(|e| Box::new(canvas_runtime_error(&e)))
                },
            )
            .with_fn("stroke_cap", |command: &mut Self, cap: ImmutableString| {
                command
                    .clone()
                    .stroke_cap(&cap)
                    .map_err(|e| Box::new(canvas_runtime_error(&e)))
            })
            .with_fn(
                "stroke_join",
                |command: &mut Self, join: ImmutableString| {
                    command
                        .clone()
                        .stroke_join(&join)
                        .map_err(|e| Box::new(canvas_runtime_error(&e)))
                },
            );
        register_canvas_command_transforms(&mut builder);
    }
}

fn register_canvas_command_transforms(builder: &mut TypeBuilder<CanvasCommand>) {
    builder
        .with_fn(
            "translate",
            |command: &mut CanvasCommand, x: FLOAT, y: FLOAT| {
                command
                    .clone()
                    .translate(x, y)
                    .map_err(|error| Box::new(canvas_runtime_error(&error)))
            },
        )
        .with_fn(
            "translate",
            |command: &mut CanvasCommand,
             x: Dynamic,
             y: Dynamic|
             -> Result<CanvasCommand, Box<EvalAltResult>> {
                command
                    .clone()
                    .translate(
                        number("path.translate_x", &x)?,
                        number("path.translate_y", &y)?,
                    )
                    .map_err(|error| Box::new(canvas_runtime_error(&error)))
            },
        )
        .with_fn("scale", |command: &mut CanvasCommand, value: FLOAT| {
            command
                .clone()
                .scale(value)
                .map_err(|error| Box::new(canvas_runtime_error(&error)))
        })
        .with_fn(
            "scale",
            |command: &mut CanvasCommand,
             value: INT|
             -> Result<CanvasCommand, Box<EvalAltResult>> {
                command
                    .clone()
                    .scale(number("path.scale", &Dynamic::from_int(value))?)
                    .map_err(|error| Box::new(canvas_runtime_error(&error)))
            },
        )
        .with_fn("rotate", |command: &mut CanvasCommand, degrees: FLOAT| {
            command
                .clone()
                .rotate(degrees)
                .map_err(|error| Box::new(canvas_runtime_error(&error)))
        })
        .with_fn(
            "rotate",
            |command: &mut CanvasCommand,
             degrees: INT|
             -> Result<CanvasCommand, Box<EvalAltResult>> {
                command
                    .clone()
                    .rotate(number("path.rotate", &Dynamic::from_int(degrees))?)
                    .map_err(|error| Box::new(canvas_runtime_error(&error)))
            },
        )
        .with_fn(
            "clip_rect",
            |command: &mut CanvasCommand, x: FLOAT, y: FLOAT, width: FLOAT, height: FLOAT| {
                command
                    .clone()
                    .clip_rect(x, y, width, height)
                    .map_err(|error| Box::new(canvas_runtime_error(&error)))
            },
        )
        .with_fn(
            "clip_rect",
            |command: &mut CanvasCommand,
             x: Dynamic,
             y: Dynamic,
             width: Dynamic,
             height: Dynamic|
             -> Result<CanvasCommand, Box<EvalAltResult>> {
                command
                    .clone()
                    .clip_rect(
                        number("path.clip.x", &x)?,
                        number("path.clip.y", &y)?,
                        number("path.clip.width", &width)?,
                        number("path.clip.height", &height)?,
                    )
                    .map_err(|error| Box::new(canvas_runtime_error(&error)))
            },
        );
}

/// One numeric argument of a Canvas function, an integer or a float.
fn number(field: &'static str, value: &Dynamic) -> Result<FLOAT, Box<EvalAltResult>> {
    crate::value::script_number(value).ok_or_else(|| {
        Box::new(canvas_runtime_error(&format!(
            "{field} must be a number, got {}",
            value.type_name()
        )))
    })
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanvasScene {
    commands: Vec<CanvasCommand>,
    view_box: Option<CanvasClipRect>,
    path_pulse: Option<Arc<CanvasPathPulse>>,
    threshold_bands: Option<Arc<CanvasThresholdBands>>,
}

/// Stationary x samples and their two ordinates. The fill is anchored to
/// the whole canvas viewport, regardless of which cells expose it.
#[derive(Debug, PartialEq)]
pub(crate) struct CanvasThresholdBands {
    samples: Vec<[f64; 3]>,
    limit: f64,
    pub(crate) fill: crate::BackgroundFill,
}

impl CanvasThresholdBands {
    pub(crate) fn for_each_span(&self, progress: f64, mut emit: impl FnMut(f64, f64)) {
        let progress = if progress.is_finite() {
            progress.clamp(0., 1.)
        } else {
            1.
        };
        let n = self.samples.len();
        let edge = |i: usize| match i {
            0 => self.samples[0][0],
            i if i == n => self.samples[n - 1][0],
            i => f64::midpoint(self.samples[i - 1][0], self.samples[i][0]),
        };
        let mut start = None;
        for (i, &[_, a, b]) in self.samples.iter().enumerate() {
            let y = if progress >= 1. {
                b
            } else {
                a + (b - a) * progress
            };
            if y < self.limit {
                start.get_or_insert(i);
            } else if let Some(start) = start.take() {
                emit(edge(start), edge(i));
            }
        }
        if let Some(start) = start {
            emit(edge(start), edge(n));
        }
    }

    fn complexity(&self) -> usize {
        let paint = match &self.fill {
            crate::BackgroundFill::Layers(layers) => {
                1 + layers
                    .layers()
                    .iter()
                    .map(|layer| layer.stops.len())
                    .sum::<usize>()
            }
            _ => 1,
        };
        self.samples.len() + self.samples.len().div_ceil(2) * paint
    }
}

/// Paint-only decoration sampled from the owner's `PathProgress` clock.
/// Its route can be replaced independently of the motion declaration.
#[derive(Debug, PartialEq)]
pub(crate) struct CanvasPathPulse {
    paths: Vec<Vec<(f64, f64)>>,
    pub radius: f64,
    pub paint: PaintColor,
    pub hollow: bool,
    reverse: bool,
}

impl CanvasPathPulse {
    pub(crate) fn sample(&self, progress: f64, lag: f64) -> Option<CanvasPathSample> {
        if !progress.is_finite() || progress < lag {
            return None;
        }
        let t = (progress - lag).clamp(0.0, 1.0);
        sample_flattened_paths(&self.paths, if self.reverse { 1.0 - t } else { t })
    }
}

/// One uniform, centered source-to-drawable projection. Paint and presented
/// hit testing share this value; it is not another retained animation clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CanvasViewport {
    pub scale: f64,
    source_x: f64,
    source_y: f64,
    offset_x: f64,
    offset_y: f64,
}

impl Default for CanvasViewport {
    fn default() -> Self {
        Self {
            scale: 1.0,
            source_x: 0.0,
            source_y: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

impl CanvasViewport {
    pub(crate) fn point(self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.source_x).mul_add(self.scale, self.offset_x),
            (y - self.source_y).mul_add(self.scale, self.offset_y),
        )
    }

    pub(crate) fn source_point(self, point: (f64, f64)) -> Option<(f64, f64)> {
        let x = (point.0 - self.offset_x) / self.scale + self.source_x;
        let y = (point.1 - self.offset_y) / self.scale + self.source_y;
        (x.is_finite() && y.is_finite()).then_some((x, y))
    }

    pub(crate) fn rect(self, rect: CanvasClipRect) -> CanvasClipRect {
        let (x, y) = self.point(rect.x, rect.y);
        CanvasClipRect {
            x,
            y,
            width: rect.width * self.scale,
            height: rect.height * self.scale,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasPathSample {
    pub x: f64,
    pub y: f64,
    pub tangent_degrees: f64,
}

impl CanvasScene {
    /// Construct a keyed retained vector scene.
    ///
    /// # Errors
    ///
    /// Returns [`CanvasError::DuplicateKey`] when sibling commands collide.
    pub fn new(commands: Vec<CanvasCommand>) -> Result<Self, CanvasError> {
        let mut keys = BTreeSet::new();
        if let Some(key) = commands
            .iter()
            .map(CanvasCommand::key)
            .find(|key| !keys.insert((*key).to_owned()))
        {
            return Err(CanvasError::DuplicateKey(key.to_owned()));
        }
        Ok(Self {
            commands,
            view_box: None,
            path_pulse: None,
            threshold_bands: None,
        })
    }

    /// Fit source coordinates into the actual native drawable, uniformly and
    /// centered (the equivalent of `xMidYMid meet`). The node's layout size is
    /// still controlled by its Style, including symbolic theme lengths.
    ///
    /// # Errors
    /// Returns for non-finite/unrepresentable bounds. Dimensions must be
    /// between `f32::MIN_POSITIVE` and `f32::MAX`, origins must fit f32, and
    /// each origin plus its dimension must remain distinct in f64.
    pub fn with_view_box(
        mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<Self, CanvasError> {
        let view_box = CanvasClipRect {
            x: finite_value("view_box.x", x)?,
            y: finite_value("view_box.y", y)?,
            width: positive_value("view_box.width", width)?,
            height: positive_value("view_box.height", height)?,
        };
        let limit = f64::from(f32::MAX);
        let minimum = f64::from(f32::MIN_POSITIVE);
        if x.abs() > limit
            || y.abs() > limit
            || width < minimum
            || height < minimum
            || width > limit
            || height > limit
            || x + width <= x
            || y + height <= y
        {
            return Err(CanvasError::InvalidViewBox);
        }
        self.view_box = Some(view_box);
        Ok(self)
    }

    #[must_use]
    pub const fn view_box(&self) -> Option<CanvasClipRect> {
        self.view_box
    }

    pub(crate) fn viewport(&self, width: f64, height: f64) -> Option<CanvasViewport> {
        let Some(source) = self.view_box else {
            return Some(CanvasViewport::default());
        };
        let scale = (width / source.width).min(height / source.height);
        let offset_x = (width - source.width * scale) / 2.0;
        let offset_y = (height - source.height * scale) / 2.0;
        (scale.is_finite() && scale > 0.0 && offset_x.is_finite() && offset_y.is_finite())
            .then_some(CanvasViewport {
                scale,
                source_x: source.x,
                source_y: source.y,
                offset_x,
                offset_y,
            })
    }

    #[must_use]
    pub fn commands(&self) -> &[CanvasCommand] {
        &self.commands
    }

    #[must_use]
    pub fn complexity(&self) -> usize {
        self.commands.iter().fold(
            (if self.path_pulse.is_some() { 4 } else { 0 })
                + self
                    .threshold_bands
                    .as_ref()
                    .map_or(0, |bands| bands.complexity()),
            |total: usize, command| total.saturating_add(command.complexity()),
        )
    }

    /// Paint a background before the commands, visible only in cells whose
    /// interpolated y is strictly less than the limit. Cells use midpoint
    /// edges in source space; adjacent active cells become one span.
    ///
    /// This decoration consumes the node's `path_progress` (default one)
    /// but remains fixed in the Canvas viewport like a path clip. It creates
    /// no hit region. Each call replaces the previous decoration.
    ///
    /// # Errors
    /// Returns an error for non-finite limits, invalid open polylines, or
    /// transformed x coordinates that move or are not strictly increasing.
    pub fn with_threshold_bands(
        mut self,
        command: &CanvasCommand,
        limit: f64,
        fill: crate::BackgroundFill,
    ) -> Result<Self, CanvasError> {
        let limit = finite_value("threshold_bands.limit", limit)?;
        let route = command.peak_route()?;
        let mut samples = Vec::<[f64; 3]>::with_capacity(route.from.len());
        for (&[ax, ay], &[bx, by]) in route.from.iter().zip(route.to.iter()) {
            let (left, a) = canvas_transform_point(route.transform, ax, ay);
            let (right, b) = canvas_transform_point(route.transform, bx, by);
            if left.partial_cmp(&right) != Some(std::cmp::Ordering::Equal)
                || samples.last().is_some_and(|p| p[0] >= left)
            {
                return Err(CanvasError::MovingBandSamples);
            }
            samples.push([left, a, b]);
        }
        self.threshold_bands = Some(Arc::new(CanvasThresholdBands {
            samples,
            limit,
            fill,
        }));
        Ok(self)
    }

    pub(crate) fn threshold_bands(&self) -> Option<&CanvasThresholdBands> {
        self.threshold_bands.as_deref()
    }

    /// Decorate a static path with a dot and a short trail. The dot consumes
    /// the Canvas node's `path_progress`; no second clock or hit owner is
    /// created. A reverse pulse reads the same route from end to start.
    ///
    /// # Errors
    ///
    /// Returns an error for unknown paths, morphs, empty routes or invalid radii.
    pub fn with_path_pulse(
        mut self,
        key: &str,
        radius: f64,
        paint: impl Into<PaintColor>,
        hollow: bool,
        reverse: bool,
    ) -> Result<Self, CanvasError> {
        let radius = positive_value("path_pulse.radius", radius)?;
        let command = self
            .commands
            .iter()
            .find(|command| command.key() == key)
            .ok_or_else(|| CanvasError::UnknownPath(key.to_owned()))?;
        let CanvasCommand::Path {
            segments,
            transform,
            ..
        } = command
        else {
            return Err(CanvasError::NotAPath(key.to_owned()));
        };
        let paths = flatten_path(segments, *transform);
        if sample_flattened_paths(&paths, 0.0)
            .is_none_or(|point| !point.x.is_finite() || !point.y.is_finite())
        {
            return Err(CanvasError::EmptyPath(key.to_owned()));
        }
        self.path_pulse = Some(Arc::new(CanvasPathPulse {
            paths,
            radius,
            paint: paint.into(),
            hollow,
            reverse,
        }));
        Ok(self)
    }

    pub(crate) fn path_pulse(&self) -> Option<&CanvasPathPulse> {
        self.path_pulse.as_deref()
    }

    /// Sample position and tangent along one retained path by arc length.
    ///
    /// # Errors
    ///
    /// Returns an explicit error for unknown keys, non-path commands, or an
    /// empty path.
    pub fn sample_path(&self, key: &str, progress: f64) -> Result<CanvasPathSample, CanvasError> {
        if !progress.is_finite() {
            return Err(CanvasError::NonFinite {
                field: "path.progress",
                value: progress,
            });
        }
        let command = self
            .commands
            .iter()
            .find(|command| command.key() == key)
            .ok_or_else(|| CanvasError::UnknownPath(key.to_owned()))?;
        let paths = match command {
            CanvasCommand::Path {
                segments,
                transform,
                ..
            } => flatten_path(segments, *transform),
            CanvasCommand::MorphPath {
                from, transform, ..
            }
            | CanvasCommand::MorphFillPath {
                from, transform, ..
            } => flatten_path(from, *transform),
            _ => return Err(CanvasError::NotAPath(key.to_owned())),
        };
        sample_flattened_paths(&paths, progress.clamp(0.0, 1.0))
            .ok_or_else(|| CanvasError::EmptyPath(key.to_owned()))
    }

    #[must_use]
    pub fn hit_test(&self, x: f64, y: f64) -> Option<&str> {
        self.commands
            .iter()
            .rev()
            .find(|command| command.hit_test(x, y))
            .map(CanvasCommand::key)
    }

    pub(crate) fn hit_test_presented(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        presentation: crate::geometry::CanvasMotionTransform,
    ) -> Option<&str> {
        let viewport = self.viewport(width, height)?;
        self.commands
            .iter()
            .rev()
            .find(|command| command.hit_test_presented(x, y, width, height, presentation, viewport))
            .map(CanvasCommand::key)
    }
}

impl CustomType for CanvasScene {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("CanvasScene")
            .with_fn(
                "view_box",
                |scene: &mut Self, x: Dynamic, y: Dynamic, width: Dynamic, height: Dynamic| {
                    scene
                        .clone()
                        .with_view_box(
                            number("view_box.x", &x)?,
                            number("view_box.y", &y)?,
                            number("view_box.width", &width)?,
                            number("view_box.height", &height)?,
                        )
                        .map_err(|error| Box::new(canvas_runtime_error(&error)))
                },
            )
            .with_fn(
                "threshold_bands",
                |scene: &mut Self, route: CanvasCommand, limit: Dynamic, paint: crate::Style| {
                    let mut scene = scene.clone();
                    let limit = number("threshold_bands.limit", &limit)?;
                    if let Some(fill) = paint.base.background {
                        scene
                            .with_threshold_bands(&route, limit, *fill)
                            .map_err(|error| Box::new(canvas_runtime_error(&error)))
                    } else {
                        scene.threshold_bands = None;
                        Ok(scene)
                    }
                },
            )
            .with_fn(
                "path_pulse",
                |scene: &mut Self,
                 key: ImmutableString,
                 radius: Dynamic,
                 color: Dynamic,
                 hollow: bool,
                 reverse: bool| {
                    scene
                        .clone()
                        .with_path_pulse(
                            key.as_str(),
                            number("path_pulse.radius", &radius)?,
                            paint_color(color)?,
                            hollow,
                            reverse,
                        )
                        .map_err(|error| Box::new(canvas_runtime_error(&error)))
                },
            );
    }
}

impl CanvasCommand {
    #[allow(clippy::too_many_lines)]
    fn hit_test_presented(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        presentation: crate::geometry::CanvasMotionTransform,
        viewport: CanvasViewport,
    ) -> bool {
        let transform_point = |point: (f64, f64)| {
            let point = viewport.point(point.0, point.1);
            canvas_motion_point(width, height, point.0, point.1, presentation)
        };
        let outside_clip = |clip: CanvasClipRect| {
            let clip = viewport.rect(clip);
            x < clip.x || x > clip.x + clip.width || y < clip.y || y > clip.y + clip.height
        };
        if self.stroke_style().is_ok() || matches!(self, Self::MorphFillPath { .. }) {
            if self.path_clip().is_some_and(outside_clip) {
                return false;
            }
            return self.tessellated_hit(
                x,
                y,
                presentation.path_progress,
                viewport.scale,
                |x, y| {
                    let (x, y) = transform_point((x, y));
                    gpui::point(
                        gpui::px(crate::renderer::f64_to_f32(x)),
                        gpui::px(crate::renderer::f64_to_f32(y)),
                    )
                },
            );
        }
        let transform_paths = |paths: Vec<Vec<(f64, f64)>>| {
            paths
                .into_iter()
                .map(|path| path.into_iter().map(transform_point).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        };
        match self {
            Self::Rect {
                x: left,
                y: top,
                width,
                height,
                ..
            } => point_in_polygon(
                x,
                y,
                &[
                    transform_point((*left, *top)),
                    transform_point((left + width, *top)),
                    transform_point((left + width, top + height)),
                    transform_point((*left, top + height)),
                ],
            ),
            Self::Circle {
                center_x,
                center_y,
                radius,
                ..
            } => {
                let polygon = (0..48)
                    .map(|index| {
                        let angle = std::f64::consts::TAU * f64::from(index) / 48.0;
                        transform_point((
                            center_x + radius * angle.cos(),
                            center_y + radius * angle.sin(),
                        ))
                    })
                    .collect::<Vec<_>>();
                point_in_polygon(x, y, &polygon)
            }
            Self::Line { .. } | Self::MorphPath { .. } | Self::MorphFillPath { .. } => false, // handled above
            Self::Path {
                segments,
                fill,
                transform,
                clip,
                ..
            } => {
                if clip.is_some_and(outside_clip) {
                    return false;
                }
                let paths = transform_paths(flatten_path(segments, *transform));
                fill.is_some()
                    && paths
                        .iter()
                        .fold(false, |inside, path| inside ^ point_in_polygon(x, y, path))
            }
        }
    }

    fn hit_test(&self, x: f64, y: f64) -> bool {
        if self.stroke_style().is_ok() || matches!(self, Self::MorphFillPath { .. }) {
            return self.hit_test_presented(
                x,
                y,
                0.0,
                0.0,
                crate::geometry::CanvasMotionTransform::default(),
                CanvasViewport::default(),
            );
        }
        match self {
            Self::Rect {
                x: left,
                y: top,
                width,
                height,
                ..
            } => x >= *left && x <= left + width && y >= *top && y <= top + height,
            Self::Circle {
                center_x,
                center_y,
                radius,
                ..
            } => {
                (x - center_x).mul_add(x - center_x, (y - center_y) * (y - center_y))
                    <= radius * radius
            }
            Self::Line { .. } | Self::MorphPath { .. } | Self::MorphFillPath { .. } => false, // handled above
            Self::Path {
                segments,
                fill,
                transform,
                clip,
                ..
            } => {
                if clip.is_some_and(|clip| {
                    x < clip.x || x > clip.x + clip.width || y < clip.y || y > clip.y + clip.height
                }) {
                    return false;
                }
                let paths = flatten_path(segments, *transform);
                fill.is_some()
                    && paths
                        .iter()
                        .fold(false, |inside, path| inside ^ point_in_polygon(x, y, path))
            }
        }
    }
}

pub(crate) fn canvas_motion_point(
    width: f64,
    height: f64,
    x: f64,
    y: f64,
    motion: crate::geometry::CanvasMotionTransform,
) -> (f64, f64) {
    canvas_motion_affine(width, height, motion).map_point((x, y))
}

pub(crate) fn canvas_motion_affine(
    width: f64,
    height: f64,
    motion: crate::geometry::CanvasMotionTransform,
) -> crate::Affine2D {
    crate::Affine2D::scale(motion.scale_x, motion.scale_y)
        .and_then(|transform| {
            transform.then(crate::Affine2D::skew_degrees(motion.skew_x, motion.skew_y)?)
        })
        .and_then(|transform| transform.then(crate::Affine2D::rotation_degrees(motion.rotate)?))
        .and_then(|transform| transform.around((width / 2.0, height / 2.0)))
        .unwrap_or(crate::Affine2D::IDENTITY)
}

pub(crate) fn trimmed_canvas_paths(
    paths: Vec<Vec<(f64, f64)>>,
    progress: f64,
) -> Vec<Vec<(f64, f64)>> {
    let total = paths
        .iter()
        .flat_map(|path| path.windows(2))
        .map(|pair| (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1))
        .sum::<f64>();
    let mut remaining = total * progress.clamp(0.0, 1.0);
    let mut output = Vec::new();
    for path in paths {
        let Some(first) = path.first().copied() else {
            continue;
        };
        let mut trimmed = vec![first];
        for pair in path.windows(2) {
            if remaining <= 0.0 {
                break;
            }
            let length = (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1);
            if length <= remaining {
                trimmed.push(pair[1]);
                remaining -= length;
            } else if length > f64::EPSILON {
                let ratio = remaining / length;
                trimmed.push((
                    pair[0].0 + (pair[1].0 - pair[0].0) * ratio,
                    pair[0].1 + (pair[1].1 - pair[0].1) * ratio,
                ));
                remaining = 0.0;
            }
        }
        if trimmed.len() > 1 {
            output.push(trimmed);
        }
        if remaining <= 0.0 {
            break;
        }
    }
    output
}

pub(crate) fn flatten_path(
    segments: &[CanvasPathSegment],
    transform: CanvasTransform,
) -> Vec<Vec<(f64, f64)>> {
    let mut paths = Vec::<Vec<(f64, f64)>>::new();
    let mut current = (0.0, 0.0);
    let mut start = (0.0, 0.0);
    for segment in segments {
        match *segment {
            CanvasPathSegment::Move { x, y } => {
                current = canvas_transform_point(transform, x, y);
                start = current;
                paths.push(vec![current]);
            }
            CanvasPathSegment::Line { x, y } => {
                current = canvas_transform_point(transform, x, y);
                if let Some(path) = paths.last_mut() {
                    path.push(current);
                }
            }
            CanvasPathSegment::Quadratic {
                x,
                y,
                control_x,
                control_y,
            } => {
                let control = canvas_transform_point(transform, control_x, control_y);
                let end = canvas_transform_point(transform, x, y);
                append_quadratic(paths.last_mut(), current, control, end);
                current = end;
            }
            CanvasPathSegment::Cubic {
                x,
                y,
                control_a_x,
                control_a_y,
                control_b_x,
                control_b_y,
            } => {
                let control_a = canvas_transform_point(transform, control_a_x, control_a_y);
                let control_b = canvas_transform_point(transform, control_b_x, control_b_y);
                let end = canvas_transform_point(transform, x, y);
                append_cubic(paths.last_mut(), current, control_a, control_b, end);
                current = end;
            }
            CanvasPathSegment::Close => {
                if let Some(path) = paths.last_mut()
                    && path.last().is_none_or(|last| {
                        (last.0 - start.0).abs() > f64::EPSILON
                            || (last.1 - start.1).abs() > f64::EPSILON
                    })
                {
                    path.push(start);
                }
                current = start;
            }
        }
    }
    paths
}

pub(crate) fn canvas_transform_point(transform: CanvasTransform, x: f64, y: f64) -> (f64, f64) {
    crate::Affine2D::scale(transform.scale, transform.scale)
        .and_then(|affine| {
            affine.then(crate::Affine2D::rotation_degrees(transform.rotate_degrees)?)
        })
        .and_then(|affine| {
            affine.then(crate::Affine2D::translation(
                transform.translate_x,
                transform.translate_y,
            )?)
        })
        .expect("validated canvas path transform remains finite")
        .map_point((x, y))
}

fn append_quadratic(
    path: Option<&mut Vec<(f64, f64)>>,
    start: (f64, f64),
    control: (f64, f64),
    end: (f64, f64),
) {
    if let Some(path) = path {
        for step in 1..=16 {
            let t = f64::from(step) / 16.0;
            let inverse = 1.0 - t;
            path.push((
                inverse * inverse * start.0 + 2.0 * inverse * t * control.0 + t * t * end.0,
                inverse * inverse * start.1 + 2.0 * inverse * t * control.1 + t * t * end.1,
            ));
        }
    }
}

fn append_cubic(
    path: Option<&mut Vec<(f64, f64)>>,
    start: (f64, f64),
    control_a: (f64, f64),
    control_b: (f64, f64),
    end: (f64, f64),
) {
    if let Some(path) = path {
        for step in 1..=24 {
            let t = f64::from(step) / 24.0;
            let inverse = 1.0 - t;
            path.push((
                inverse.powi(3) * start.0
                    + 3.0 * inverse * inverse * t * control_a.0
                    + 3.0 * inverse * t * t * control_b.0
                    + t.powi(3) * end.0,
                inverse.powi(3) * start.1
                    + 3.0 * inverse * inverse * t * control_a.1
                    + 3.0 * inverse * t * t * control_b.1
                    + t.powi(3) * end.1,
            ));
        }
    }
}

fn point_in_polygon(x: f64, y: f64, polygon: &[(f64, f64)]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &current in polygon {
        if (current.1 > y) != (previous.1 > y)
            && x < (previous.0 - current.0) * (y - current.1) / (previous.1 - current.1) + current.0
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum CanvasError {
    #[error("Canvas peak projection requires finite, ordered source and display ranges")]
    InvalidPeakProjection,
    #[error("Canvas threshold bands require increasing, stationary sample x coordinates")]
    MovingBandSamples,
    #[error("conditional peak paint requires a Canvas peak binding")]
    MissingPeakBinding,
    #[error("conditional peak background must be a color or ()")]
    InvalidPeakPaint,
    #[error(
        "Canvas peak routes require one open polyline of 2–1024 matching native-sized vertices"
    )]
    InvalidPeakPath,
    #[error("Canvas peak placement needs finite coordinates and ordered bounds")]
    InvalidPeakBounds,
    #[error("stroke decoration requires a line or stroked path")]
    StrokeDecorationOnly,
    #[error("invalid Canvas stroke {field}: `{value}`")]
    InvalidStrokeOption { field: &'static str, value: String },
    #[error(
        "view box bounds must fit native float range, dimensions must be at least f32::MIN_POSITIVE, and ends must be distinct from origins"
    )]
    InvalidViewBox,
    #[error("canvas command key `{0}` must be 1-128 safe ASCII characters")]
    InvalidKey(String),
    #[error("canvas command key `{0}` occurs more than once")]
    DuplicateKey(String),
    #[error("canvas {field} must be finite, got {value}")]
    NonFinite { field: &'static str, value: f64 },
    #[error("canvas {field} must be positive, got {value}")]
    NonPositive { field: &'static str, value: f64 },
    #[error("canvas scene item {index} must be CanvasCommand, got {actual}")]
    InvalidCommand { index: usize, actual: String },
    #[error("canvas path item {index} must be CanvasPathSegment, got {actual}")]
    InvalidPathSegment { index: usize, actual: String },
    #[error("canvas path must begin with a move segment and contain at least two segments")]
    InvalidPath,
    #[error("Catmull–Rom interpolation requires one open subpath of move/line segments")]
    SplineRequiresPolyline,
    #[error("canvas path cannot contain more than 10000 segments")]
    TooManyPathSegments,
    #[error("canvas transform and clip refinements apply only to path commands")]
    PathDecorationOnly,
    #[error("canvas morph paths must have identical segment topology")]
    IncompatibleMorphTopology,
    #[error("canvas path key `{0}` does not exist")]
    UnknownPath(String),
    #[error("canvas command `{0}` is not a path")]
    NotAPath(String),
    #[error("canvas path `{0}` has no measurable segments")]
    EmptyPath(String),
    #[error("canvas paths can use a solid or two-stop style background, not layered paint")]
    UnsupportedStyleFill,
}

fn sample_flattened_paths(paths: &[Vec<(f64, f64)>], progress: f64) -> Option<CanvasPathSample> {
    let total = paths
        .iter()
        .flat_map(|path| path.windows(2))
        .map(|pair| (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1))
        .sum::<f64>();
    if total <= f64::EPSILON {
        return None;
    }
    let mut target = total * progress;
    let mut last = None;
    for pair in paths.iter().flat_map(|path| path.windows(2)) {
        let dx = pair[1].0 - pair[0].0;
        let dy = pair[1].1 - pair[0].1;
        let length = dx.hypot(dy);
        if length <= f64::EPSILON {
            continue;
        }
        last = Some((pair, dx, dy, length));
        if target <= length {
            let ratio = target / length;
            return Some(CanvasPathSample {
                x: pair[0].0 + dx * ratio,
                y: pair[0].1 + dy * ratio,
                tangent_degrees: dy.atan2(dx).to_degrees(),
            });
        }
        target -= length;
    }
    last.map(|(pair, dx, dy, _)| CanvasPathSample {
        x: pair[1].0,
        y: pair[1].1,
        tangent_degrees: dy.atan2(dx).to_degrees(),
    })
}

fn finite_value(field: &'static str, value: f64) -> Result<f64, CanvasError> {
    value
        .is_finite()
        .then_some(value)
        .ok_or(CanvasError::NonFinite { field, value })
}

fn positive_value(field: &'static str, value: f64) -> Result<f64, CanvasError> {
    let value = finite_value(field, value)?;
    (value > 0.0)
        .then_some(value)
        .ok_or(CanvasError::NonPositive { field, value })
}

fn validate_key(key: ImmutableString) -> Result<String, Box<EvalAltResult>> {
    let key: String = key.into();
    let valid = (1..=128).contains(&key.len())
        && key.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':')
        });
    valid
        .then_some(key.clone())
        .ok_or_else(|| Box::new(canvas_runtime_error(&CanvasError::InvalidKey(key))))
}

fn finite(field: &'static str, value: FLOAT) -> Result<f64, Box<EvalAltResult>> {
    value.is_finite().then_some(value).ok_or_else(|| {
        Box::new(canvas_runtime_error(&CanvasError::NonFinite {
            field,
            value,
        }))
    })
}

fn positive(field: &'static str, value: FLOAT) -> Result<f64, Box<EvalAltResult>> {
    let value = finite(field, value)?;
    (value > 0.0).then_some(value).ok_or_else(|| {
        Box::new(canvas_runtime_error(&CanvasError::NonPositive {
            field,
            value,
        }))
    })
}

fn canvas_rect<C: Into<PaintColor>>(
    key: ImmutableString,
    x: FLOAT,
    y: FLOAT,
    width: FLOAT,
    height: FLOAT,
    fill: C,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    Ok(CanvasCommand::Rect {
        key: validate_key(key)?,
        x: finite("rect.x", x)?,
        y: finite("rect.y", y)?,
        width: positive("rect.width", width)?,
        height: positive("rect.height", height)?,
        fill: fill.into(),
    })
}

fn canvas_circle<C: Into<PaintColor>>(
    key: ImmutableString,
    center_x: FLOAT,
    center_y: FLOAT,
    radius: FLOAT,
    fill: C,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    Ok(CanvasCommand::Circle {
        key: validate_key(key)?,
        center_x: finite("circle.center_x", center_x)?,
        center_y: finite("circle.center_y", center_y)?,
        radius: positive("circle.radius", radius)?,
        fill: fill.into(),
    })
}

#[allow(clippy::too_many_arguments)]
fn canvas_line<C: Into<PaintColor>>(
    key: ImmutableString,
    from_x: FLOAT,
    from_y: FLOAT,
    to_x: FLOAT,
    to_y: FLOAT,
    width: FLOAT,
    color: C,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    Ok(CanvasCommand::Line {
        key: validate_key(key)?,
        from_x: finite("line.from_x", from_x)?,
        from_y: finite("line.from_y", from_y)?,
        to_x: finite("line.to_x", to_x)?,
        to_y: finite("line.to_y", to_y)?,
        width: positive("line.width", width)?,
        color: color.into(),
        stroke_style: CanvasStrokeStyle::default(),
    })
}

fn path_move(x: FLOAT, y: FLOAT) -> Result<CanvasPathSegment, Box<EvalAltResult>> {
    Ok(CanvasPathSegment::Move {
        x: finite("path.move.x", x)?,
        y: finite("path.move.y", y)?,
    })
}

fn path_line(x: FLOAT, y: FLOAT) -> Result<CanvasPathSegment, Box<EvalAltResult>> {
    Ok(CanvasPathSegment::Line {
        x: finite("path.line.x", x)?,
        y: finite("path.line.y", y)?,
    })
}

fn path_quadratic(
    x: FLOAT,
    y: FLOAT,
    control_x: FLOAT,
    control_y: FLOAT,
) -> Result<CanvasPathSegment, Box<EvalAltResult>> {
    Ok(CanvasPathSegment::Quadratic {
        x: finite("path.quadratic.x", x)?,
        y: finite("path.quadratic.y", y)?,
        control_x: finite("path.quadratic.control_x", control_x)?,
        control_y: finite("path.quadratic.control_y", control_y)?,
    })
}

#[allow(clippy::too_many_arguments, clippy::similar_names)]
fn path_cubic(
    x: FLOAT,
    y: FLOAT,
    control_a_x: FLOAT,
    control_a_y: FLOAT,
    control_b_x: FLOAT,
    control_b_y: FLOAT,
) -> Result<CanvasPathSegment, Box<EvalAltResult>> {
    Ok(CanvasPathSegment::Cubic {
        x: finite("path.cubic.x", x)?,
        y: finite("path.cubic.y", y)?,
        control_a_x: finite("path.cubic.control_a_x", control_a_x)?,
        control_a_y: finite("path.cubic.control_a_y", control_a_y)?,
        control_b_x: finite("path.cubic.control_b_x", control_b_x)?,
        control_b_y: finite("path.cubic.control_b_y", control_b_y)?,
    })
}

const fn path_close() -> CanvasPathSegment {
    CanvasPathSegment::Close
}

fn path_catmull_rom(values: Array) -> Result<Array, Box<EvalAltResult>> {
    let segments = parse_path_segments(values)?;
    let vertices: Vec<_> = segments
        .iter()
        .enumerate()
        .map(|(i, segment)| match *segment {
            CanvasPathSegment::Move { x, y } if i == 0 => Ok([x, y]),
            CanvasPathSegment::Line { x, y } if i > 0 => Ok([x, y]),
            _ => Err(Box::new(canvas_runtime_error(
                &CanvasError::SplineRequiresPolyline,
            ))),
        })
        .collect::<Result<_, _>>()?;
    catmull_rom_segments(&vertices).map(|path| path.into_iter().map(Dynamic::from).collect())
}

pub(crate) fn catmull_rom_segments(
    vertices: &[[f64; 2]],
) -> Result<Vec<CanvasPathSegment>, Box<EvalAltResult>> {
    if vertices.len() < 2 {
        return Err(Box::new(canvas_runtime_error(&CanvasError::InvalidPath)));
    }
    let mut path = Vec::with_capacity(vertices.len());
    path.push(path_move(vertices[0][0], vertices[0][1])?);
    for i in 0..vertices.len() - 1 {
        let [x0, y0] = vertices[i.saturating_sub(1)];
        let [x1, y1] = vertices[i];
        let [x2, y2] = vertices[i + 1];
        let [x3, y3] = vertices[(i + 2).min(vertices.len() - 1)];
        path.push(path_cubic(
            x2,
            y2,
            x1 + (x2 - x0) / 6.,
            y1 + (y2 - y0) / 6.,
            x2 - (x3 - x1) / 6.,
            y2 - (y3 - y1) / 6.,
        )?);
    }
    Ok(path)
}

fn parse_path_segments(values: Array) -> Result<Vec<CanvasPathSegment>, Box<EvalAltResult>> {
    if values.len() < 2 {
        return Err(Box::new(canvas_runtime_error(&CanvasError::InvalidPath)));
    }
    if values.len() > 10_000 {
        return Err(Box::new(canvas_runtime_error(
            &CanvasError::TooManyPathSegments,
        )));
    }
    let segments = values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let actual = value.type_name().to_owned();
            value.try_cast::<CanvasPathSegment>().ok_or_else(|| {
                Box::new(canvas_runtime_error(&CanvasError::InvalidPathSegment {
                    index,
                    actual,
                }))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !matches!(segments.first(), Some(CanvasPathSegment::Move { .. })) {
        return Err(Box::new(canvas_runtime_error(&CanvasError::InvalidPath)));
    }
    Ok(segments)
}

fn canvas_fill_path<C: Into<PaintColor>>(
    key: ImmutableString,
    segments: Array,
    fill: C,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    canvas_path(key, segments, Some(CanvasFill::Solid(fill.into())), None)
}

fn canvas_gradient_path(
    key: ImmutableString,
    segments: Array,
    fill: LinearGradientSpec,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    canvas_path(key, segments, Some(CanvasFill::LinearGradient(fill)), None)
}

fn canvas_style_fill_path(
    key: ImmutableString,
    segments: Array,
    style: crate::Style,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    canvas_path(key, segments, Some(fill_from_style(style)?), None)
}

fn fill_from_style(style: crate::Style) -> Result<CanvasFill, Box<EvalAltResult>> {
    Ok(match style.base.background.map(|fill| *fill) {
        Some(crate::BackgroundFill::Solid(paint)) => CanvasFill::Solid(paint),
        Some(crate::BackgroundFill::LegacyLinear(paint)) => CanvasFill::LinearGradient(paint),
        Some(crate::BackgroundFill::Layers(_)) => {
            return Err(Box::new(canvas_runtime_error(
                &CanvasError::UnsupportedStyleFill,
            )));
        }
        None => CanvasFill::Solid(ColorValue::Literal(Rgba8::from_rgba_hex(0)).into()),
    })
}

fn canvas_stroke_path<C: Into<PaintColor>>(
    key: ImmutableString,
    segments: Array,
    width: FLOAT,
    color: C,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    let width = positive("path.stroke_width", width)?;
    canvas_path(key, segments, None, Some((color.into(), width)))
}

fn canvas_morph_stroke_path<C: Into<PaintColor>>(
    key: ImmutableString,
    from: Array,
    to: Array,
    width: FLOAT,
    color: C,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    let from = parse_path_segments(from)?;
    let to = parse_path_segments(to)?;
    if !compatible_path_topology(&from, &to) {
        return Err(Box::new(canvas_runtime_error(
            &CanvasError::IncompatibleMorphTopology,
        )));
    }
    Ok(CanvasCommand::MorphPath {
        key: validate_key(key)?,
        from,
        to,
        stroke: (color.into(), positive("path.stroke_width", width)?),
        stroke_style: CanvasStrokeStyle::default(),
        transform: CanvasTransform::default(),
        clip: None,
    })
}

fn canvas_morph_fill_path(
    key: ImmutableString,
    from: Array,
    to: Array,
    paint: Dynamic,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    let from = parse_path_segments(from)?;
    let to = parse_path_segments(to)?;
    if !compatible_path_topology(&from, &to) {
        return Err(Box::new(canvas_runtime_error(
            &CanvasError::IncompatibleMorphTopology,
        )));
    }
    let fill = if paint.is::<crate::Style>() {
        fill_from_style(paint.cast::<crate::Style>())?
    } else if paint.is::<LinearGradientSpec>() {
        CanvasFill::LinearGradient(paint.cast::<LinearGradientSpec>())
    } else {
        CanvasFill::Solid(paint_color(paint)?)
    };
    Ok(CanvasCommand::MorphFillPath {
        key: validate_key(key)?,
        from,
        to,
        fill,
        transform: CanvasTransform::default(),
        clip: None,
    })
}

pub(crate) fn compatible_path_topology(
    from: &[CanvasPathSegment],
    to: &[CanvasPathSegment],
) -> bool {
    from.len() == to.len()
        && from.iter().zip(to).all(|(from, to)| {
            matches!(
                (from, to),
                (
                    CanvasPathSegment::Move { .. },
                    CanvasPathSegment::Move { .. }
                ) | (
                    CanvasPathSegment::Line { .. },
                    CanvasPathSegment::Line { .. }
                ) | (
                    CanvasPathSegment::Quadratic { .. },
                    CanvasPathSegment::Quadratic { .. }
                ) | (
                    CanvasPathSegment::Cubic { .. },
                    CanvasPathSegment::Cubic { .. }
                ) | (CanvasPathSegment::Close, CanvasPathSegment::Close)
            )
        })
}

pub(crate) fn interpolate_path(
    from: &[CanvasPathSegment],
    to: &[CanvasPathSegment],
    progress: f64,
) -> Option<Vec<CanvasPathSegment>> {
    compatible_path_topology(from, to).then(|| {
        let progress = progress.clamp(0.0, 1.0);
        if progress == 0.0 {
            return from.to_vec();
        }
        if progress >= 1.0 {
            return to.to_vec();
        }
        let value = |from: f64, to: f64| from + (to - from) * progress;
        from.iter()
            .zip(to)
            .map(|(from, to)| match (from, to) {
                (
                    CanvasPathSegment::Move { x: ax, y: ay },
                    CanvasPathSegment::Move { x: bx, y: by },
                ) => CanvasPathSegment::Move {
                    x: value(*ax, *bx),
                    y: value(*ay, *by),
                },
                (
                    CanvasPathSegment::Line { x: ax, y: ay },
                    CanvasPathSegment::Line { x: bx, y: by },
                ) => CanvasPathSegment::Line {
                    x: value(*ax, *bx),
                    y: value(*ay, *by),
                },
                (
                    CanvasPathSegment::Quadratic {
                        x: ax,
                        y: ay,
                        control_x: acx,
                        control_y: acy,
                    },
                    CanvasPathSegment::Quadratic {
                        x: bx,
                        y: by,
                        control_x: bcx,
                        control_y: bcy,
                    },
                ) => CanvasPathSegment::Quadratic {
                    x: value(*ax, *bx),
                    y: value(*ay, *by),
                    control_x: value(*acx, *bcx),
                    control_y: value(*acy, *bcy),
                },
                (
                    CanvasPathSegment::Cubic {
                        x: ax,
                        y: ay,
                        control_a_x: aax,
                        control_a_y: aay,
                        control_b_x: abx,
                        control_b_y: aby,
                    },
                    CanvasPathSegment::Cubic {
                        x: bx,
                        y: by,
                        control_a_x: bax,
                        control_a_y: bay,
                        control_b_x: bbx,
                        control_b_y: bby,
                    },
                ) => CanvasPathSegment::Cubic {
                    x: value(*ax, *bx),
                    y: value(*ay, *by),
                    control_a_x: value(*aax, *bax),
                    control_a_y: value(*aay, *bay),
                    control_b_x: value(*abx, *bbx),
                    control_b_y: value(*aby, *bby),
                },
                (CanvasPathSegment::Close, CanvasPathSegment::Close) => CanvasPathSegment::Close,
                _ => unreachable!("topology was validated"),
            })
            .collect()
    })
}

fn canvas_path(
    key: ImmutableString,
    segments: Array,
    fill: Option<CanvasFill>,
    stroke: Option<(PaintColor, f64)>,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    Ok(CanvasCommand::Path {
        key: validate_key(key)?,
        segments: parse_path_segments(segments)?,
        fill,
        stroke,
        stroke_style: CanvasStrokeStyle::default(),
        transform: CanvasTransform::default(),
        clip: None,
    })
}

fn canvas_scene(values: Array) -> Result<CanvasScene, Box<EvalAltResult>> {
    let commands = values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let actual = value.type_name().to_owned();
            value.try_cast::<CanvasCommand>().ok_or_else(|| {
                Box::new(canvas_runtime_error(&CanvasError::InvalidCommand {
                    index,
                    actual,
                }))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    CanvasScene::new(commands).map_err(|error| Box::new(canvas_runtime_error(&error)))
}

fn canvas_runtime_error(error: &dyn std::fmt::Display) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(error.to_string().into(), Position::NONE)
}

fn register_canvas_peak_api(engine: &mut Engine) {
    engine.register_type_with_name::<CanvasPeakRoute>("CanvasPeakRoute");
    FuncRegistration::new("project_y")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |route: &mut CanvasPeakRoute,
             top: Dynamic,
             bottom: Dynamic,
             y0: Dynamic,
             y1: Dynamic| {
                route
                    .clone()
                    .with_y_projection(
                        number("peak.source_top", &top)?,
                        number("peak.source_bottom", &bottom)?,
                        number("peak.display_top", &y0)?,
                        number("peak.display_bottom", &y1)?,
                    )
                    .map_err(|e| Box::new(canvas_runtime_error(&e)))
            },
        );
    FuncRegistration::new("canvas_peak_paint_above")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |node: &mut crate::UiNode,
             limit: Dynamic,
             foreground: ColorValue,
             background: Dynamic|
             -> Result<crate::UiNode, Box<EvalAltResult>> {
                let background = if background.is_unit() {
                    None
                } else {
                    Some(background.try_cast::<ColorValue>().ok_or_else(|| {
                        Box::new(canvas_runtime_error(&CanvasError::InvalidPeakPaint))
                    })?)
                };
                node.clone()
                    .with_canvas_peak_paint_above(
                        number("peak.limit_y", &limit)?,
                        foreground,
                        background,
                    )
                    .map_err(|e| Box::new(canvas_runtime_error(&e)))
            },
        );
    FuncRegistration::new("follow_canvas_peak")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |node: &mut crate::UiNode,
             route: CanvasPeakRoute,
             x: Dynamic,
             y: Dynamic,
             bounds: Array|
             -> Result<crate::UiNode, Box<EvalAltResult>> {
                let bounds = if bounds.is_empty() {
                    None
                } else {
                    if bounds.len() != 4 {
                        return Err(Box::new(canvas_runtime_error(
                            &CanvasError::InvalidPeakBounds,
                        )));
                    }
                    Some([
                        number("peak.min_x", &bounds[0])?,
                        number("peak.min_y", &bounds[1])?,
                        number("peak.max_x", &bounds[2])?,
                        number("peak.max_y", &bounds[3])?,
                    ])
                };
                node.clone()
                    .with_canvas_peak(
                        route,
                        [number("peak.offset_x", &x)?, number("peak.offset_y", &y)?],
                        bounds,
                    )
                    .map_err(|e| Box::new(canvas_runtime_error(&e)))
            },
        );
}

pub(crate) fn register_canvas_api(engine: &mut Engine) {
    register_canvas_peak_api(engine);
    engine.build_type::<CanvasPathSegment>();
    engine.build_type::<CanvasCommand>();
    engine.build_type::<CanvasScene>();
    FuncRegistration::new("canvas_rect")
        .in_global_namespace()
        .register_into_engine(engine, canvas_rect::<ColorValue>);
    FuncRegistration::new("canvas_circle")
        .in_global_namespace()
        .register_into_engine(engine, canvas_circle::<ColorValue>);
    FuncRegistration::new("canvas_line")
        .in_global_namespace()
        .register_into_engine(engine, canvas_line::<ColorValue>);
    FuncRegistration::new("path_move")
        .in_global_namespace()
        .register_into_engine(engine, path_move);
    FuncRegistration::new("path_line")
        .in_global_namespace()
        .register_into_engine(engine, path_line);
    FuncRegistration::new("path_quadratic")
        .in_global_namespace()
        .register_into_engine(engine, path_quadratic);
    FuncRegistration::new("path_cubic")
        .in_global_namespace()
        .register_into_engine(engine, path_cubic);
    FuncRegistration::new("path_close")
        .in_global_namespace()
        .register_into_engine(engine, path_close);
    FuncRegistration::new("path_catmull_rom")
        .in_global_namespace()
        .register_into_engine(engine, path_catmull_rom);
    FuncRegistration::new("canvas_fill_path")
        .in_global_namespace()
        .register_into_engine(engine, canvas_fill_path::<ColorValue>);
    FuncRegistration::new("canvas_fill_path")
        .in_global_namespace()
        .register_into_engine(engine, canvas_fill_path::<PaintColor>);
    FuncRegistration::new("current_color")
        .in_global_namespace()
        .register_into_engine(engine, PaintColor::current_text);
    FuncRegistration::new("alpha")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |color: PaintColor, factor: Dynamic| -> Result<PaintColor, Box<EvalAltResult>> {
                color
                    .alpha(number("paint.alpha", &factor)?)
                    .map_err(|error| Box::new(canvas_runtime_error(&error)))
            },
        );
    FuncRegistration::new("canvas_fill_path")
        .in_global_namespace()
        .register_into_engine(engine, canvas_gradient_path);
    FuncRegistration::new("canvas_fill_path")
        .in_global_namespace()
        .register_into_engine(engine, canvas_style_fill_path);
    FuncRegistration::new("canvas_stroke_path")
        .in_global_namespace()
        .register_into_engine(engine, canvas_stroke_path::<ColorValue>);
    FuncRegistration::new("canvas_morph_stroke_path")
        .in_global_namespace()
        .register_into_engine(engine, canvas_morph_stroke_path::<ColorValue>);
    FuncRegistration::new("canvas_morph_fill_path")
        .in_global_namespace()
        .register_into_engine(engine, canvas_morph_fill_path);
    FuncRegistration::new("canvas_scene")
        .in_global_namespace()
        .register_into_engine(engine, canvas_scene);
    register_mixed_number_canvas_api(engine);
}

/// Overloads that take each coordinate, size and width as an integer or a
/// float, so `canvas_rect("r", 0, 0, 10, 10, fill)` works; an all-float call
/// still resolves to the typed function.
#[allow(clippy::too_many_lines)]
fn register_mixed_number_canvas_api(engine: &mut Engine) {
    FuncRegistration::new("canvas_rect")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |key: ImmutableString,
             x: Dynamic,
             y: Dynamic,
             width: Dynamic,
             height: Dynamic,
             fill: Dynamic|
             -> Result<CanvasCommand, Box<EvalAltResult>> {
                canvas_rect(
                    key,
                    number("rect.x", &x)?,
                    number("rect.y", &y)?,
                    number("rect.width", &width)?,
                    number("rect.height", &height)?,
                    paint_color(fill)?,
                )
            },
        );
    FuncRegistration::new("canvas_circle")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |key: ImmutableString,
             center_x: Dynamic,
             center_y: Dynamic,
             radius: Dynamic,
             fill: Dynamic|
             -> Result<CanvasCommand, Box<EvalAltResult>> {
                canvas_circle(
                    key,
                    number("circle.center_x", &center_x)?,
                    number("circle.center_y", &center_y)?,
                    number("circle.radius", &radius)?,
                    paint_color(fill)?,
                )
            },
        );
    FuncRegistration::new("canvas_line")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |key: ImmutableString,
             from_x: Dynamic,
             from_y: Dynamic,
             to_x: Dynamic,
             to_y: Dynamic,
             width: Dynamic,
             color: Dynamic|
             -> Result<CanvasCommand, Box<EvalAltResult>> {
                canvas_line(
                    key,
                    number("line.from_x", &from_x)?,
                    number("line.from_y", &from_y)?,
                    number("line.to_x", &to_x)?,
                    number("line.to_y", &to_y)?,
                    number("line.width", &width)?,
                    paint_color(color)?,
                )
            },
        );
    FuncRegistration::new("path_move")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |x: Dynamic, y: Dynamic| -> Result<CanvasPathSegment, Box<EvalAltResult>> {
                path_move(number("path.move.x", &x)?, number("path.move.y", &y)?)
            },
        );
    FuncRegistration::new("path_line")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |x: Dynamic, y: Dynamic| -> Result<CanvasPathSegment, Box<EvalAltResult>> {
                path_line(number("path.line.x", &x)?, number("path.line.y", &y)?)
            },
        );
    FuncRegistration::new("path_quadratic")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |x: Dynamic,
             y: Dynamic,
             control_x: Dynamic,
             control_y: Dynamic|
             -> Result<CanvasPathSegment, Box<EvalAltResult>> {
                path_quadratic(
                    number("path.quadratic.x", &x)?,
                    number("path.quadratic.y", &y)?,
                    number("path.quadratic.control_x", &control_x)?,
                    number("path.quadratic.control_y", &control_y)?,
                )
            },
        );
    FuncRegistration::new("path_cubic")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |x: Dynamic,
             y: Dynamic,
             control_a_x: Dynamic,
             control_a_y: Dynamic,
             control_b_x: Dynamic,
             control_b_y: Dynamic|
             -> Result<CanvasPathSegment, Box<EvalAltResult>> {
                path_cubic(
                    number("path.cubic.x", &x)?,
                    number("path.cubic.y", &y)?,
                    number("path.cubic.control_a_x", &control_a_x)?,
                    number("path.cubic.control_a_y", &control_a_y)?,
                    number("path.cubic.control_b_x", &control_b_x)?,
                    number("path.cubic.control_b_y", &control_b_y)?,
                )
            },
        );
    FuncRegistration::new("canvas_stroke_path")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |key: ImmutableString,
             segments: Array,
             width: Dynamic,
             color: Dynamic|
             -> Result<CanvasCommand, Box<EvalAltResult>> {
                canvas_stroke_path(
                    key,
                    segments,
                    number("path.stroke_width", &width)?,
                    paint_color(color)?,
                )
            },
        );
    FuncRegistration::new("canvas_morph_stroke_path")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |key: ImmutableString,
             from: Array,
             to: Array,
             width: Dynamic,
             color: Dynamic|
             -> Result<CanvasCommand, Box<EvalAltResult>> {
                canvas_morph_stroke_path(
                    key,
                    from,
                    to,
                    number("path.stroke_width", &width)?,
                    paint_color(color)?,
                )
            },
        );
}

fn paint_color(value: Dynamic) -> Result<PaintColor, Box<EvalAltResult>> {
    if value.is::<PaintColor>() {
        return Ok(value.cast());
    }
    if value.is::<ColorValue>() {
        return Ok(value.cast::<ColorValue>().into());
    }
    Err(Box::new(canvas_runtime_error(&format!(
        "Canvas paint requires ColorValue or PaintColor, got {}",
        value.type_name()
    ))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catmull_rom_polylines_preserve_vertices_and_share_morph_controls() {
        let mut engine = Engine::new();
        register_canvas_api(&mut engine);
        let path = |y1, y2| {
            parse_path_segments(
                engine
                    .eval::<Array>(&format!(
                        "path_catmull_rom([path_move(0,0),path_line(6,{y1}),path_line(12,{y2})])"
                    ))
                    .unwrap(),
            )
            .unwrap()
        };
        let from = path(12, 0);
        let to = path(0, 12);
        assert_eq!(
            from[1],
            CanvasPathSegment::Cubic {
                x: 6.,
                y: 12.,
                control_a_x: 1.,
                control_a_y: 2.,
                control_b_x: 4.,
                control_b_y: 12.,
            }
        );
        assert_eq!(interpolate_path(&from, &to, 0.5).unwrap(), path(6, 6));
        assert_eq!(interpolate_path(&from, &to, 1.).unwrap(), to);
        assert!(
            engine
                .eval::<Array>("path_catmull_rom([path_move(0,0),path_line(1,1),path_close()])")
                .is_err()
        );
    }

    #[test]
    fn themed_path_uses_only_style_background() {
        let mut engine = Engine::new();
        crate::style::register_style_api(&mut engine);
        register_canvas_api(&mut engine);
        let command: CanvasCommand = engine
            .eval(
                r#"canvas_fill_path("fill", [path_move(0,0), path_line(8,1)],
                    style().padding(px(44)).background(rgb(0xffaa33))
                        .hover(style().background(rgb(0x00ff00))))"#,
            )
            .unwrap();
        let CanvasCommand::Path { fill, segments, .. } = command else {
            panic!("expected a filled path")
        };
        assert_eq!(segments.len(), 2);
        assert_eq!(
            fill,
            Some(CanvasFill::Solid(
                ColorValue::Literal(Rgba8::from_rgba_hex(0xffaa_33ff)).into()
            ))
        );
        let gradient: CanvasCommand = engine
            .eval(
                r#"canvas_fill_path("fill", [path_move(0,0), path_line(8,1)],
                    style().linear_gradient(linear_gradient(#{angle:180,
                        from:theme_color("accent"), to:theme_color("surface")})))"#,
            )
            .unwrap();
        assert!(matches!(gradient, CanvasCommand::Path {
            fill: Some(CanvasFill::LinearGradient(LinearGradientSpec {
                from: ColorValue::Token(ref token), ..
            })), ..
        } if token == "accent"));
    }

    #[test]
    fn path_pulse_reuses_progress_on_new_routes_and_never_owns_a_hit() {
        let scene = |end, reverse| {
            CanvasScene::new(vec![CanvasCommand::Path {
                key: "route".into(),
                segments: vec![
                    CanvasPathSegment::Move { x: 10.0, y: 10.0 },
                    CanvasPathSegment::Line { x: end, y: 10.0 },
                ],
                fill: None,
                stroke: None,
                stroke_style: CanvasStrokeStyle::default(),
                transform: CanvasTransform::default(),
                clip: None,
            }])
            .unwrap()
            .with_path_pulse(
                "route",
                4.0,
                ColorValue::Literal(Rgba8::from_rgba_hex(0x3377_ccff)),
                reverse,
                reverse,
            )
            .unwrap()
        };
        let forward = scene(110.0, false);
        let reverse = scene(110.0, true);
        let resized = scene(210.0, true);
        let [out, back, new] = [&forward, &reverse, &resized]
            .map(|scene| scene.path_pulse().unwrap().sample(0.25, 0.0).unwrap());
        assert!((out.x - 35.0).abs() < 1e-9);
        assert!((back.x - 85.0).abs() < 1e-9);
        assert!((new.x - 160.0).abs() < 1e-9);
        assert!(reverse.path_pulse().unwrap().hollow);
        assert!(reverse.path_pulse().unwrap().sample(0.01, 0.05).is_none());
        assert!(reverse.hit_test(back.x, back.y + 2.0).is_none());
    }

    fn stroke_scene(cap: &str, join: &str) -> CanvasScene {
        let mut engine = Engine::new();
        crate::style::register_style_api(&mut engine);
        register_canvas_api(&mut engine);
        engine.eval(&format!(r#"canvas_scene([
            canvas_stroke_path("stroke",[path_move(10,20),path_line(30,20),path_line(30,0)],10,rgb(0))
                .stroke_cap("{cap}").stroke_join("{join}")])"#)).unwrap()
    }

    #[test]
    fn cap_and_join_hits_use_native_stroke_triangles_not_round_distance_bounds() {
        for cap in ["butt", "round", "square"] {
            for join in ["miter", "miter_clip", "round", "bevel"] {
                assert_eq!(stroke_scene(cap, join).hit_test(20.0, 20.0), Some("stroke"));
            }
        }
        let butt = stroke_scene("butt", "miter");
        let round = stroke_scene("round", "round");
        let square = stroke_scene("square", "bevel");
        for scene in [&butt, &round, &square] {
            assert_eq!(scene.hit_test(20.0, 20.0), Some("stroke"));
            assert_eq!(scene.hit_test(3.0, 20.0), None);
        }
        assert_eq!(butt.hit_test(7.0, 20.0), None, "butt has no end extension");
        assert_eq!(round.hit_test(7.0, 20.0), Some("stroke"));
        assert_eq!(square.hit_test(6.0, 24.0), Some("stroke"));
        assert_eq!(
            round.hit_test(6.0, 24.0),
            None,
            "outside round cap, inside its box"
        );
        assert_eq!(
            butt.hit_test(34.0, 24.0),
            Some("stroke"),
            "miter outer corner"
        );
        assert_eq!(round.hit_test(34.0, 24.0), None);
        assert_eq!(round.hit_test(34.0, 22.0), Some("stroke"));
        assert_eq!(
            square.hit_test(34.0, 22.0),
            None,
            "bevel cuts the outer corner"
        );
    }

    #[test]
    fn trimmed_round_strokes_recap_the_presented_endpoint_and_obey_projection_and_clip() {
        let scene = stroke_scene("round", "round");
        let half = crate::geometry::CanvasMotionTransform {
            path_progress: Some(0.5),
            ..Default::default()
        };
        assert_eq!(
            scene.hit_test_presented(30.0, 23.0, 40.0, 40.0, half),
            Some("stroke")
        );
        assert_eq!(scene.hit_test_presented(30.0, 10.0, 40.0, 40.0, half), None);
        let empty = crate::geometry::CanvasMotionTransform {
            path_progress: Some(0.0),
            ..Default::default()
        };
        assert_eq!(
            scene.hit_test_presented(10.0, 20.0, 40.0, 40.0, empty),
            None
        );
        let scene = scene.with_view_box(0.0, 0.0, 40.0, 40.0).unwrap();
        assert_eq!(
            scene.hit_test_presented(60.0, 46.0, 80.0, 80.0, half),
            Some("stroke")
        );
        assert_eq!(scene.hit_test_presented(60.0, 20.0, 80.0, 80.0, half), None);
        let clipped = scene.commands()[0]
            .clone()
            .clip_rect(10.0, 0.0, 30.0, 40.0)
            .unwrap();
        assert_eq!(
            CanvasScene::new(vec![clipped]).unwrap().hit_test(7.0, 20.0),
            None
        );
    }

    #[test]
    fn unrepresentable_native_strokes_fail_closed_before_tessellation() {
        for width in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MAX, 1e-300] {
            let line = CanvasCommand::Line {
                key: "invalid".to_owned(),
                from_x: 0.0,
                from_y: 0.0,
                to_x: 20.0,
                to_y: 20.0,
                width,
                color: PaintColor::current_text(),
                stroke_style: CanvasStrokeStyle {
                    cap: CanvasLineCap::Round,
                    join: CanvasLineJoin::Round,
                    dash: None,
                },
            };
            assert_eq!(
                CanvasScene::new(vec![line]).unwrap().hit_test(10.0, 10.0),
                None
            );
        }
        let scene = stroke_scene("round", "round");
        assert!(
            scene.commands()[0]
                .native_path(None, 1.0, |_, _| gpui::point(
                    gpui::px(f32::INFINITY),
                    gpui::px(0.0)
                ))
                .is_none()
        );
    }

    #[test]
    fn threshold_bands_merge_adjacent_active_cells_and_own_no_hits() {
        let mut engine = Engine::new();
        register_canvas_api(&mut engine);
        crate::style::register_style_api(&mut engine);
        let scene = engine
            .eval::<CanvasScene>(
                r#"
            let a = [path_move(0,-2), path_line(10,0), path_line(20,-2),
                path_line(30,0), path_line(40,0)];
            let b = [path_move(0,-2), path_line(10,-2), path_line(20,-2),
                path_line(30,0), path_line(40,-2)];
            let route = canvas_morph_stroke_path("z",a,b,1,rgb(0));
            canvas_scene([]).threshold_bands(route,0,style().background(rgb(0)))
        "#,
            )
            .unwrap();
        let bands = scene.threshold_bands().unwrap();
        let sample = |progress| {
            let mut spans = Vec::new();
            bands.for_each_span(progress, |left, right| spans.push((left, right)));
            spans
        };
        assert_eq!(sample(0.), vec![(0., 5.), (15., 25.)]);
        assert_eq!(sample(0.5), vec![(0., 25.), (35., 40.)]);
        assert_eq!(sample(f64::NAN), sample(1.));
        assert_eq!(scene.hit_test(3., 2.), None);
        assert_eq!(scene.complexity(), 8);
        assert!(engine.eval::<CanvasScene>(r#"
            let a = [path_move(0,0), path_line(10,0)];
            let b = [path_move(0,0), path_line(11,0)];
            canvas_scene([]).threshold_bands(canvas_morph_stroke_path("z",a,b,1,rgb(0)),0,style().background(rgb(0)))
        "#).is_err());
    }

    #[test]
    fn canvas_peak_samples_the_winning_vertex_not_a_line_between_peaks() {
        let mut engine = Engine::new();
        crate::style::register_style_api(&mut engine);
        register_canvas_api(&mut engine);
        let route = engine
            .eval::<CanvasPeakRoute>(
                r#"
            let a=[path_move(0,4),path_line(20,12),path_line(40,30)];
            let b=[path_move(0,30),path_line(20,12),path_line(40,4)];
            canvas_morph_stroke_path("p",a,b,2,rgb(0)).peak_route()
        "#,
            )
            .unwrap();
        assert_eq!(route.point(0.), [0., 4.]);
        assert_eq!(route.point(0.2), [0., 9.2]);
        assert_eq!(route.point(0.5), [20., 12.]);
        assert_eq!(route.point(1.), [40., 4.]);
        let bounded = CanvasPeakBinding::new(route, [5., -21.], Some([0., 0., 32., 99.])).unwrap();
        assert_eq!(bounded.sample(1.).0, [32., 0.]);
        assert_eq!(bounded.complexity(), 6);
        let foreground = ColorValue::Literal(Rgba8::from_rgba_hex(0xf044_38ff));
        let background = ColorValue::Literal(Rgba8::from_rgba_hex(0xfdea_e9ff));
        let painted = bounded
            .clone()
            .with_paint_above(12., foreground.clone(), Some(background.clone()))
            .unwrap();
        assert_eq!(painted.sample(0.5).0[1], 0.);
        assert!(
            painted.sample(0.5).1.is_none(),
            "a peak at the limit is neutral even when the label is clamped above"
        );
        let warning = painted.sample(1.).1.unwrap();
        assert_eq!(warning.foreground, foreground);
        assert_eq!(warning.background, Some(background));
        assert!(
            bounded
                .with_paint_above(f64::NAN, foreground, None)
                .is_err()
        );
        assert!(engine.eval::<CanvasPeakRoute>(r#"
            canvas_stroke_path("curve",[path_move(0,0),path_quadratic(9,4,5,1)],1,rgb(0)).peak_route()
        "#).is_err());
    }

    #[test]
    fn canvas_peak_projection_clamps_after_ranking_and_warning() {
        let mut engine = Engine::new();
        crate::style::register_style_api(&mut engine);
        register_canvas_api(&mut engine);
        let route = engine
            .eval::<CanvasPeakRoute>(
                r#"
            let a=[path_move(0,8),path_line(20,12),path_line(40,30)];
            let b=[path_move(0,30),path_line(20,12),path_line(40,4)];
            canvas_morph_stroke_path("p",a,b,2,rgb(0)).peak_route().project_y(15,35,0,20)
        "#,
            )
            .unwrap();
        assert_eq!(route.point(0.5), [20., 0.]);
        assert_eq!(route.point(1.), [40., 0.]);
        let paint = ColorValue::Literal(Rgba8::from_rgba_hex(0xf044_38ff));
        let bound = CanvasPeakBinding::new(route, [0., 0.], None)
            .unwrap()
            .with_paint_above(8., paint, None)
            .unwrap();
        assert!(bound.sample(0.).1.is_none(), "exactly the authored limit");
        assert!(
            bound.sample(0.5).1.is_none(),
            "display clipping does not trigger warnings"
        );
        assert!(bound.sample(1.).1.is_some());

        let precise = engine
            .eval::<CanvasPeakRoute>(
                r#"
            let from=[path_move(0,-20),path_line(20,0)];
            let to=[path_move(0,-0.000000001),path_line(20,0)];
            canvas_morph_stroke_path("end",from,to,2,rgb(0)).peak_route()
        "#,
            )
            .unwrap();
        assert_eq!(precise.point(1.), [0., -1e-9]);
    }

    #[test]
    fn morph_fill_hits_share_native_interpolation_projection_and_clip() {
        let mut engine = Engine::new();
        crate::style::register_style_api(&mut engine);
        register_canvas_api(&mut engine);
        let scene=engine.eval::<CanvasScene>(r#"
            let from=[path_move(0,16),path_line(40,16),path_line(40,20),path_line(0,20),path_close()];
            let to=[path_move(0,4),path_line(40,4),path_line(40,20),path_line(0,20),path_close()];
            let paint=style().linear_gradient(linear_gradient(#{angle:180,from:rgb(0x404040),to:rgba(0x40404000)}));
            canvas_scene([canvas_morph_fill_path("area",from,to,paint).clip_rect(0,0,30,20)])
                .view_box(0,0,40,20)
        "#).unwrap();
        assert!(matches!(
            scene.commands()[0],
            CanvasCommand::MorphFillPath {
                fill: CanvasFill::LinearGradient(_),
                ..
            }
        ));
        assert_eq!(scene.commands()[0].complexity(), 11);
        assert_eq!(scene.hit_test(10., 12.), None);
        assert_eq!(scene.hit_test(10., 18.), Some("area"));
        let motion = crate::geometry::CanvasMotionTransform {
            path_progress: Some(0.5),
            ..Default::default()
        };
        assert_eq!(
            scene.hit_test_presented(30., 24., 80., 40., motion),
            Some("area")
        );
        assert_eq!(scene.hit_test_presented(30., 18., 80., 40., motion), None);
        assert_eq!(scene.hit_test_presented(70., 24., 80., 40., motion), None);
        assert!(
            engine
                .eval::<CanvasCommand>(
                    r#"
            canvas_morph_fill_path("bad",
              [path_move(0,0),path_line(10,0),path_close()],
              [path_move(0,0),path_line(10,0),path_line(0,10)],rgb(0))
        "#
                )
                .is_err()
        );
    }

    #[test]
    fn stroke_dash_keeps_holes_after_morph_and_projection() {
        let mut engine = Engine::new();
        crate::style::register_style_api(&mut engine);
        register_canvas_api(&mut engine);
        let scene = engine
            .eval::<CanvasScene>(
                r#"
            let a=[path_move(0,5),path_line(48,5)];
            let b=[path_move(0,15),path_line(48,15)];
            canvas_scene([canvas_morph_stroke_path("dash",a,b,2,rgb(0)).stroke_dash(4,4)])
                .view_box(0,0,48,20)
        "#,
            )
            .unwrap();
        let motion = crate::geometry::CanvasMotionTransform {
            path_progress: Some(0.5),
            ..Default::default()
        };
        assert_eq!(
            scene.hit_test_presented(4., 20., 96., 40., motion),
            Some("dash")
        );
        assert_eq!(scene.hit_test_presented(12., 20., 96., 40., motion), None);
        assert_eq!(
            scene.hit_test_presented(20., 20., 96., 40., motion),
            Some("dash")
        );
        assert_eq!(scene.hit_test_presented(4., 10., 96., 40., motion), None);
        for length in [0.0, -1.0, f64::NAN, f64::INFINITY, 4097.] {
            assert!(CanvasStrokeDash::new(length, 4.).is_err());
        }
        let huge = engine
            .eval::<CanvasScene>(
                r#"
            canvas_scene([canvas_stroke_path("huge",[path_move(0,0),path_line(50000,0)],2,rgb(0))
                .stroke_dash(4,4)])
        "#,
            )
            .unwrap();
        assert!(
            huge.hit_test(2., 0.).is_none(),
            "over-budget dashes are not hit"
        );
    }

    #[test]
    fn stroke_options_are_strict_and_do_not_mutate_other_clones_or_fill_commands() {
        let mut engine = Engine::new();
        crate::style::register_style_api(&mut engine);
        register_canvas_api(&mut engine);
        for expression in [
            r#"canvas_line("l",0,0,10,10,1,rgb(0)).stroke_cap("Round")"#,
            r#"canvas_stroke_path("p",[path_move(0,0),path_line(10,10)],1,rgb(0)).stroke_join("sharp")"#,
            r#"canvas_fill_path("p",[path_move(0,0),path_line(10,10)],rgb(0)).stroke_cap("round")"#,
            r#"canvas_rect("r",0,0,10,10,rgb(0)).stroke_join("round")"#,
        ] {
            assert!(
                engine.eval::<CanvasCommand>(expression).is_err(),
                "{expression}"
            );
        }
        let original = stroke_scene("butt", "miter").commands()[0].clone();
        let changed = original
            .clone()
            .stroke_cap("round")
            .unwrap()
            .stroke_join("bevel")
            .unwrap();
        assert_eq!(
            original.stroke_style().unwrap(),
            CanvasStrokeStyle::default()
        );
        assert_eq!(
            changed.stroke_style().unwrap(),
            CanvasStrokeStyle {
                cap: CanvasLineCap::Round,
                join: CanvasLineJoin::Bevel,
                dash: None,
            }
        );
    }

    #[test]
    fn native_paint_context_preserves_rgba_and_alpha_expression_semantics() {
        for raw in [0xff00_00ff, 0x00ff_0080, 0x0000_ff00, 0x9364_c721] {
            let current = Rgba8::from_rgba_hex(raw);
            assert_eq!(
                PaintColor::current_text().resolve_with(current, &mut |_| None),
                Some(current)
            );
            let paint = PaintColor::current_text()
                .alpha(0.333)
                .unwrap()
                .alpha(0.2)
                .unwrap();
            let expected = ColorValue::Literal(current)
                .alpha(0.333)
                .unwrap()
                .alpha(0.2)
                .unwrap()
                .resolve_with(&mut |_| None);
            assert_eq!(paint.resolve_with(current, &mut |_| None), expected);
        }
        for invalid in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
            assert!(PaintColor::current_text().alpha(invalid).is_err());
        }
        let literal = ColorValue::Token("brand.icon".to_owned());
        let paint = PaintColor::from(literal.clone());
        let custom = Rgba8::from_rgb_hex(0x0012_3456);
        let mut calls = 0;
        assert_eq!(
            paint.resolve_with(Rgba8::from_rgba_hex(0), &mut |value| {
                calls += 1;
                assert_eq!(value, &literal);
                Some(custom)
            }),
            Some(custom)
        );
        assert_eq!(
            calls, 1,
            "the complete ColorResolver expression remains authoritative"
        );
    }

    #[test]
    fn contextual_paint_is_shared_but_context_free_color_inputs_reject_it() {
        let mut engine = Engine::new();
        crate::style::register_style_api(&mut engine);
        register_canvas_api(&mut engine);
        let scene = engine
            .eval::<CanvasScene>(
                r#"
            let ink=current_color();
            let path=[path_move(0,0),path_line(10,10),path_line(0,10),path_close()];
            canvas_scene([
                canvas_rect("r",0,0.0,10,10.0,ink),
                canvas_circle("c",20.0,20,3.0,ink.alpha(0.5)),
                canvas_line("l",0.0,0,10,10.0,2,ink.alpha(1)),
                canvas_fill_path("p",path,ink),
                canvas_stroke_path("s",path,2.0,ink),
                canvas_morph_stroke_path("m",path,path,2,ink),
                canvas_rect("fixed",30.0,0.0,10.0,10.0,rgb(0xff0000))
            ])
        "#,
            )
            .unwrap();
        assert_eq!(scene.commands().len(), 7);
        let CanvasCommand::Circle { fill, .. } = &scene.commands()[1] else {
            panic!("circle")
        };
        assert_eq!(
            fill.resolve_with(Rgba8::from_rgba_hex(0xff00_0080), &mut |_| None),
            Some(Rgba8::from_rgba_hex(0xff00_0040))
        );
        let style = engine
            .eval::<crate::Style>("style().background(current_color())")
            .unwrap();
        assert_eq!(
            style.base.background.as_deref(),
            Some(&crate::BackgroundFill::Solid(PaintColor::current_text()))
        );
        for expression in [
            "style().text_color(current_color())",
            "linear_gradient(#{angle:90,from:current_color(),to:rgb(0)})",
            "current_color().alpha(2)",
            "canvas_rect(\"bad\",0,0,2,2,42)",
        ] {
            assert!(
                engine.eval::<Dynamic>(expression).is_err(),
                "must reject unsupported paint use: {expression}"
            );
        }
    }

    #[test]
    fn view_box_rejects_unprojectable_bounds_and_preserves_large_origin_precision() {
        let scene = CanvasScene::default();
        for (x, y, width, height) in [
            (0.0, 0.0, 1e-300, 24.0),
            (f64::MAX, 0.0, 24.0, 24.0),
            (1e20, 0.0, 1.0, 24.0),
            (0.0, 0.0, f64::INFINITY, 24.0),
        ] {
            assert!(scene.clone().with_view_box(x, y, width, height).is_err());
        }
        let large = scene.clone().with_view_box(1e20, -1e20, 1e8, 1e8).unwrap();
        assert_eq!(
            large.viewport(100.0, 100.0).unwrap().point(1e20, -1e20),
            (0.0, 0.0),
            "subtract the source origin before scaling; do not cancel huge products"
        );
        let minimum = f64::from(f32::MIN_POSITIVE);
        let small = scene.with_view_box(0.0, 0.0, minimum, minimum).unwrap();
        let mapped = small
            .viewport(240.0, 240.0)
            .unwrap()
            .point(minimum, minimum);
        assert!((mapped.0 - 240.0).abs() < 1e-10 && (mapped.1 - 240.0).abs() < 1e-10);
    }

    #[test]
    fn view_box_projects_paint_and_hits_in_the_same_coordinate_domain() {
        let mut engine = Engine::new();
        crate::style::register_style_api(&mut engine);
        register_canvas_api(&mut engine);
        let scene = engine
            .eval::<CanvasScene>(
                r#"
            canvas_scene([canvas_rect("box",10,20,10,5,rgb(0))]).view_box(10,20,20,10)
        "#,
            )
            .unwrap();
        let viewport = scene.viewport(200.0, 200.0).unwrap();
        assert_eq!(viewport.point(10.0, 20.0), (0.0, 50.0));
        assert_eq!(viewport.point(20.0, 25.0), (100.0, 100.0));
        assert_eq!(
            scene.hit_test_presented(
                50.0,
                75.0,
                200.0,
                200.0,
                crate::geometry::CanvasMotionTransform::default()
            ),
            Some("box")
        );
        assert_eq!(
            scene.hit_test_presented(
                50.0,
                25.0,
                200.0,
                200.0,
                crate::geometry::CanvasMotionTransform::default()
            ),
            None
        );
        assert_eq!(
            scene.hit_test(15.0, 22.0),
            Some("box"),
            "raw queries remain in the source domain"
        );
        let rotation = crate::geometry::CanvasMotionTransform {
            rotate: 90.0,
            ..Default::default()
        };
        assert_eq!(
            scene.hit_test_presented(125.0, 50.0, 200.0, 200.0, rotation),
            Some("box")
        );
        assert_eq!(
            scene.hit_test_presented(50.0, 75.0, 200.0, 200.0, rotation),
            None
        );
        assert!(scene.viewport(0.0, 100.0).is_none());
        assert!(scene.clone().with_view_box(0.0, 0.0, 0.0, 24.0).is_err());
        assert!(scene.with_view_box(f64::NAN, 0.0, 24.0, 24.0).is_err());
    }

    #[test]
    fn view_box_scales_strokes_and_clips_without_mutating_source_commands() {
        let mut engine = Engine::new();
        crate::style::register_style_api(&mut engine);
        register_canvas_api(&mut engine);
        let scene = engine
            .eval::<CanvasScene>(
                r#"
            canvas_scene([canvas_stroke_path("line",[path_move(0,12),path_line(24,12)],2,rgb(0))
                .clip_rect(4,0,16,24)]).view_box(0,0,24,24)
        "#,
            )
            .unwrap();
        let identity = crate::geometry::CanvasMotionTransform::default();
        assert_eq!(
            scene.hit_test_presented(24.0, 25.9, 48.0, 48.0, identity),
            Some("line")
        );
        assert_eq!(
            scene.hit_test_presented(24.0, 26.1, 48.0, 48.0, identity),
            None
        );
        assert_eq!(
            scene.hit_test_presented(4.0, 24.0, 48.0, 48.0, identity),
            None
        );
        let mapped = scene.viewport(48.0, 48.0).unwrap().rect(CanvasClipRect {
            x: 4.0,
            y: 0.0,
            width: 16.0,
            height: 24.0,
        });
        assert_eq!(
            mapped,
            CanvasClipRect {
                x: 8.0,
                y: 0.0,
                width: 32.0,
                height: 48.0
            }
        );
    }

    #[test]
    fn scene_rejects_duplicate_keys() {
        let command = CanvasCommand::Rect {
            key: "same".to_owned(),
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            fill: ColorValue::Literal(crate::Rgba8::from_rgb_hex(0x00ff_0000)).into(),
        };
        assert!(matches!(
            CanvasScene::new(vec![command.clone(), command]),
            Err(CanvasError::DuplicateKey(_))
        ));
    }

    #[test]
    fn script_paths_are_typed_transformed_clipped_and_budgeted_by_segments() {
        let mut engine = Engine::new();
        crate::style::register_style_api(&mut engine);
        register_canvas_api(&mut engine);
        let scene = engine
            .eval::<CanvasScene>(
                r##"
                    canvas_scene([
                        canvas_fill_path("wave", [
                            path_move(0.0, 40.0),
                            path_cubic(80.0, 0.0, 20.0, 0.0, 60.0, 80.0),
                            path_line(120.0, 40.0),
                            path_line(120.0, 100.0),
                            path_close()
                        ], linear_gradient(#{ angle: 90,
                            from: color("#7aa2f7"), to: color("hsl(280, 60%, 60%)") }))
                            .translate(12.0, 8.0).scale(1.25).rotate(5.0)
                            .clip_rect(0.0, 0.0, 160.0, 120.0)
                    ])
                "##,
            )
            .unwrap();
        assert_eq!(scene.complexity(), 6);
        let CanvasCommand::Path {
            fill,
            transform,
            clip,
            ..
        } = &scene.commands()[0]
        else {
            unreachable!()
        };
        assert!(matches!(fill, Some(CanvasFill::LinearGradient(_))));
        assert!((transform.translate_x - 12.0).abs() < f64::EPSILON);
        assert!((transform.scale - 1.25).abs() < f64::EPSILON);
        assert!(
            clip.as_ref()
                .is_some_and(|clip| (clip.width - 160.0).abs() < f64::EPSILON)
        );
        assert!(
            engine
                .eval::<CanvasCommand>(
                    r#"canvas_fill_path("bad", [path_line(0.0, 0.0), path_close()], color("red"))"#
                )
                .is_err()
        );
    }

    #[test]
    fn numeric_arguments_take_integers_and_floats_in_any_mix() {
        let mut engine = Engine::new();
        crate::style::register_style_api(&mut engine);
        register_canvas_api(&mut engine);
        let scene = engine
            .eval::<CanvasScene>(
                r#"
                    canvas_scene([
                        canvas_rect("rect", 0, 0, 10, 10.5, color("red")),
                        canvas_circle("circle", 4, 4.0, 2, color("red")),
                        canvas_line("line", 0, 0, 10, 10, 1, color("red")),
                        canvas_stroke_path("path", [
                            path_move(0, 0), path_line(10, 0.5),
                            path_quadratic(20, 0, 15, 5), path_cubic(30, 0, 22, 4, 28, 4)
                        ], 2, color("red"))
                            .translate(1, 2.5).scale(2).rotate(90).clip_rect(0, 0, 40, 40),
                        canvas_morph_stroke_path("morph", [path_move(0, 0), path_line(4, 4)],
                            [path_move(0, 0), path_line(8, 0)], 1, color("red")),
                    ])
                "#,
            )
            .unwrap();
        let [
            CanvasCommand::Rect { height, .. },
            CanvasCommand::Circle { radius, .. },
            CanvasCommand::Line { width, .. },
            CanvasCommand::Path {
                segments,
                transform,
                clip,
                ..
            },
            CanvasCommand::MorphPath { .. },
        ] = scene.commands()
        else {
            panic!("unexpected commands: {:?}", scene.commands());
        };
        assert!((height - 10.5).abs() < f64::EPSILON);
        assert!((radius - 2.0).abs() < f64::EPSILON);
        assert!((width - 1.0).abs() < f64::EPSILON);
        assert_eq!(segments.len(), 4);
        assert!((transform.translate_y - 2.5).abs() < f64::EPSILON);
        assert!((transform.scale - 2.0).abs() < f64::EPSILON);
        assert!((transform.rotate_degrees - 90.0).abs() < f64::EPSILON);
        assert!(clip.is_some());
        let error = engine
            .eval::<CanvasCommand>(r#"canvas_rect("bad", "0", 0, 10, 10, color("red"))"#)
            .unwrap_err();
        assert!(
            error.to_string().contains("rect.x must be a number"),
            "{error}"
        );
    }

    #[test]
    fn scene_hit_testing_uses_reverse_paint_order_and_path_geometry() {
        let under = CanvasCommand::Rect {
            key: "under".to_owned(),
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
            fill: ColorValue::Token("surface".to_owned()).into(),
        };
        let path = CanvasCommand::Path {
            key: "triangle".to_owned(),
            segments: vec![
                CanvasPathSegment::Move { x: 0.0, y: 0.0 },
                CanvasPathSegment::Line { x: 100.0, y: 0.0 },
                CanvasPathSegment::Line { x: 0.0, y: 100.0 },
                CanvasPathSegment::Close,
            ],
            fill: Some(CanvasFill::Solid(
                ColorValue::Token("accent".to_owned()).into(),
            )),
            stroke: None,
            stroke_style: CanvasStrokeStyle::default(),
            transform: CanvasTransform {
                translate_x: 10.0,
                translate_y: 20.0,
                ..CanvasTransform::default()
            },
            clip: Some(CanvasClipRect {
                x: 0.0,
                y: 0.0,
                width: 120.0,
                height: 120.0,
            }),
        };
        let scene = CanvasScene::new(vec![under, path]).unwrap();
        assert_eq!(scene.hit_test(20.0, 30.0), Some("triangle"));
        assert_eq!(scene.hit_test(150.0, 150.0), Some("under"));
        assert_eq!(scene.hit_test(250.0, 250.0), None);
    }

    #[test]
    fn path_sampling_uses_arc_length_and_morph_requires_matching_topology() {
        let from = vec![
            CanvasPathSegment::Move { x: 0.0, y: 0.0 },
            CanvasPathSegment::Line { x: 100.0, y: 0.0 },
            CanvasPathSegment::Line { x: 100.0, y: 100.0 },
        ];
        let to = vec![
            CanvasPathSegment::Move { x: 0.0, y: 0.0 },
            CanvasPathSegment::Line { x: 50.0, y: 50.0 },
            CanvasPathSegment::Line { x: 0.0, y: 100.0 },
        ];
        let scene = CanvasScene::new(vec![CanvasCommand::MorphPath {
            key: "route".to_owned(),
            from: from.clone(),
            to: to.clone(),
            stroke: (ColorValue::Token("accent".to_owned()).into(), 2.0),
            stroke_style: CanvasStrokeStyle::default(),
            transform: CanvasTransform::default(),
            clip: None,
        }])
        .unwrap();
        let midpoint = scene.sample_path("route", 0.5).unwrap();
        assert!((midpoint.x - 100.0).abs() < 0.01);
        assert!(midpoint.y.abs() < 0.01);
        assert!(interpolate_path(&from, &to, 0.5).is_some());
        assert!(!compatible_path_topology(
            &from,
            &[CanvasPathSegment::Move { x: 0.0, y: 0.0 }]
        ));
    }

    #[test]
    fn presented_hit_testing_tracks_morph_trim_and_non_scaling_stroke() {
        let morph = CanvasScene::new(vec![CanvasCommand::MorphPath {
            key: "morph".to_owned(),
            from: vec![
                CanvasPathSegment::Move { x: 20.0, y: 20.0 },
                CanvasPathSegment::Line { x: 180.0, y: 20.0 },
            ],
            to: vec![
                CanvasPathSegment::Move { x: 20.0, y: 80.0 },
                CanvasPathSegment::Line { x: 180.0, y: 80.0 },
            ],
            stroke: (ColorValue::Token("accent".to_owned()).into(), 10.0),
            stroke_style: CanvasStrokeStyle::default(),
            transform: CanvasTransform::default(),
            clip: None,
        }])
        .unwrap();
        let end = crate::geometry::CanvasMotionTransform {
            path_progress: Some(1.0),
            ..crate::geometry::CanvasMotionTransform::default()
        };
        assert_eq!(
            morph.hit_test_presented(100.0, 80.0, 200.0, 100.0, end),
            Some("morph")
        );
        assert_eq!(
            morph.hit_test_presented(100.0, 20.0, 200.0, 100.0, end),
            None
        );

        let trimmed = CanvasScene::new(vec![CanvasCommand::Path {
            key: "trimmed".to_owned(),
            segments: vec![
                CanvasPathSegment::Move { x: 20.0, y: 50.0 },
                CanvasPathSegment::Line { x: 180.0, y: 50.0 },
            ],
            fill: None,
            stroke: Some((ColorValue::Token("accent".to_owned()).into(), 10.0)),
            stroke_style: CanvasStrokeStyle::default(),
            transform: CanvasTransform::default(),
            clip: None,
        }])
        .unwrap();
        let half = crate::geometry::CanvasMotionTransform {
            scale_x: 1.5,
            scale_y: 0.5,
            path_progress: Some(0.5),
            ..crate::geometry::CanvasMotionTransform::default()
        };
        assert_eq!(
            trimmed.hit_test_presented(60.0, 50.0, 200.0, 100.0, half),
            Some("trimmed")
        );
        assert_eq!(
            trimmed.hit_test_presented(180.0, 50.0, 200.0, 100.0, half),
            None
        );
        assert_eq!(
            trimmed.hit_test_presented(60.0, 56.0, 200.0, 100.0, half),
            None,
            "stroke width remains in paint-space pixels under non-uniform scale"
        );
    }
}
