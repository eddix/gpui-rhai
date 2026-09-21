use std::collections::BTreeSet;

use rhai::{
    Array, CustomType, Engine, EvalAltResult, FLOAT, FuncRegistration, ImmutableString, Position,
    TypeBuilder,
};
use thiserror::Error;

use crate::{ColorValue, LinearGradientSpec};

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
    Solid(ColorValue),
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

#[derive(Clone, Debug, PartialEq)]
pub enum CanvasCommand {
    Rect {
        key: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: ColorValue,
    },
    Circle {
        key: String,
        center_x: f64,
        center_y: f64,
        radius: f64,
        fill: ColorValue,
    },
    Line {
        key: String,
        from_x: f64,
        from_y: f64,
        to_x: f64,
        to_y: f64,
        width: f64,
        color: ColorValue,
    },
    Path {
        key: String,
        segments: Vec<CanvasPathSegment>,
        fill: Option<CanvasFill>,
        stroke: Option<(ColorValue, f64)>,
        transform: CanvasTransform,
        clip: Option<CanvasClipRect>,
    },
    MorphPath {
        key: String,
        from: Vec<CanvasPathSegment>,
        to: Vec<CanvasPathSegment>,
        stroke: (ColorValue, f64),
        transform: CanvasTransform,
        clip: Option<CanvasClipRect>,
    },
}

impl CanvasCommand {
    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::Rect { key, .. }
            | Self::Circle { key, .. }
            | Self::Line { key, .. }
            | Self::Path { key, .. }
            | Self::MorphPath { key, .. } => key,
        }
    }

    #[must_use]
    pub fn complexity(&self) -> usize {
        match self {
            Self::Path { segments, .. } => 1usize.saturating_add(segments.len()),
            Self::MorphPath { from, to, .. } => {
                1usize.saturating_add(from.len()).saturating_add(to.len())
            }
            Self::Rect { .. } | Self::Circle { .. } | Self::Line { .. } => 1,
        }
    }

    fn transform_mut(&mut self) -> Result<&mut CanvasTransform, CanvasError> {
        match self {
            Self::Path { transform, .. } | Self::MorphPath { transform, .. } => Ok(transform),
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
            Self::Path { clip: target, .. } | Self::MorphPath { clip: target, .. } => {
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
            .with_fn("translate", |command: &mut Self, x: FLOAT, y: FLOAT| {
                command
                    .clone()
                    .translate(x, y)
                    .map_err(|error| Box::new(canvas_runtime_error(&error)))
            })
            .with_fn("scale", |command: &mut Self, value: FLOAT| {
                command
                    .clone()
                    .scale(value)
                    .map_err(|error| Box::new(canvas_runtime_error(&error)))
            })
            .with_fn("rotate", |command: &mut Self, degrees: FLOAT| {
                command
                    .clone()
                    .rotate(degrees)
                    .map_err(|error| Box::new(canvas_runtime_error(&error)))
            })
            .with_fn(
                "clip_rect",
                |command: &mut Self, x: FLOAT, y: FLOAT, width: FLOAT, height: FLOAT| {
                    command
                        .clone()
                        .clip_rect(x, y, width, height)
                        .map_err(|error| Box::new(canvas_runtime_error(&error)))
                },
            );
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanvasScene {
    commands: Vec<CanvasCommand>,
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
        Ok(Self { commands })
    }

    #[must_use]
    pub fn commands(&self) -> &[CanvasCommand] {
        &self.commands
    }

    #[must_use]
    pub fn complexity(&self) -> usize {
        self.commands.iter().fold(0usize, |total, command| {
            total.saturating_add(command.complexity())
        })
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
        self.commands
            .iter()
            .rev()
            .find(|command| command.hit_test_presented(x, y, width, height, presentation))
            .map(CanvasCommand::key)
    }
}

impl CustomType for CanvasScene {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("CanvasScene");
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
    ) -> bool {
        let transform_point =
            |point: (f64, f64)| canvas_motion_point(width, height, point.0, point.1, presentation);
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
            Self::Line {
                from_x,
                from_y,
                to_x,
                to_y,
                width,
                ..
            } => {
                let from = transform_point((*from_x, *from_y));
                let to = transform_point((*to_x, *to_y));
                point_segment_distance(x, y, from.0, from.1, to.0, to.1) <= width / 2.0
            }
            Self::Path {
                segments,
                fill,
                stroke,
                transform,
                clip,
                ..
            } => {
                if clip.is_some_and(|clip| {
                    x < clip.x || x > clip.x + clip.width || y < clip.y || y > clip.y + clip.height
                }) {
                    return false;
                }
                let mut paths = flatten_path(segments, *transform);
                if stroke.is_some()
                    && presentation
                        .path_progress
                        .is_some_and(|progress| progress < 1.0)
                {
                    paths = trimmed_canvas_paths(
                        paths,
                        presentation.path_progress.unwrap_or(1.0).clamp(0.0, 1.0),
                    );
                }
                let paths = transform_paths(paths);
                let fill_hit = stroke.is_none()
                    && fill.is_some()
                    && paths
                        .iter()
                        .fold(false, |inside, path| inside ^ point_in_polygon(x, y, path));
                let stroke_hit = stroke.as_ref().is_some_and(|(_, stroke_width)| {
                    paths.iter().any(|path| {
                        path.windows(2).any(|segment| {
                            point_segment_distance(
                                x,
                                y,
                                segment[0].0,
                                segment[0].1,
                                segment[1].0,
                                segment[1].1,
                            ) <= stroke_width / 2.0
                        })
                    })
                });
                fill_hit || stroke_hit
            }
            Self::MorphPath {
                from,
                to,
                stroke: (_, stroke_width),
                transform,
                clip,
                ..
            } => {
                if clip.is_some_and(|clip| {
                    x < clip.x || x > clip.x + clip.width || y < clip.y || y > clip.y + clip.height
                }) {
                    return false;
                }
                let segments = interpolate_path(
                    from,
                    to,
                    presentation.path_progress.unwrap_or(0.0).clamp(0.0, 1.0),
                )
                .unwrap_or_else(|| from.clone());
                transform_paths(flatten_path(&segments, *transform))
                    .iter()
                    .any(|path| {
                        path.windows(2).any(|segment| {
                            point_segment_distance(
                                x,
                                y,
                                segment[0].0,
                                segment[0].1,
                                segment[1].0,
                                segment[1].1,
                            ) <= stroke_width / 2.0
                        })
                    })
            }
        }
    }

    fn hit_test(&self, x: f64, y: f64) -> bool {
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
            Self::Line {
                from_x,
                from_y,
                to_x,
                to_y,
                width,
                ..
            } => point_segment_distance(x, y, *from_x, *from_y, *to_x, *to_y) <= width / 2.0,
            Self::Path {
                segments,
                fill,
                stroke,
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
                let fill_hit = stroke.is_none()
                    && fill.is_some()
                    && paths
                        .iter()
                        .fold(false, |inside, path| inside ^ point_in_polygon(x, y, path));
                let stroke_hit = stroke.as_ref().is_some_and(|(_, width)| {
                    paths.iter().any(|path| {
                        path.windows(2).any(|segment| {
                            point_segment_distance(
                                x,
                                y,
                                segment[0].0,
                                segment[0].1,
                                segment[1].0,
                                segment[1].1,
                            ) <= width / 2.0
                        })
                    })
                });
                fill_hit || stroke_hit
            }
            Self::MorphPath {
                from,
                stroke: (_, width),
                transform,
                clip,
                ..
            } => {
                if clip.is_some_and(|clip| {
                    x < clip.x || x > clip.x + clip.width || y < clip.y || y > clip.y + clip.height
                }) {
                    return false;
                }
                flatten_path(from, *transform).iter().any(|path| {
                    path.windows(2).any(|segment| {
                        point_segment_distance(
                            x,
                            y,
                            segment[0].0,
                            segment[0].1,
                            segment[1].0,
                            segment[1].1,
                        ) <= width / 2.0
                    })
                })
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
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let rotate = motion.rotate.to_radians();
    let skew_x = motion.skew_x.to_radians().tan();
    let skew_y = motion.skew_y.to_radians().tan();
    let scaled_x = (x - center_x) * motion.scale_x;
    let scaled_y = (y - center_y) * motion.scale_y;
    let local_x = scaled_x + skew_x * scaled_y;
    let local_y = scaled_y + skew_y * scaled_x;
    (
        local_x * rotate.cos() - local_y * rotate.sin() + center_x,
        local_x * rotate.sin() + local_y * rotate.cos() + center_y,
    )
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

fn canvas_transform_point(transform: CanvasTransform, x: f64, y: f64) -> (f64, f64) {
    let radians = transform.rotate_degrees.to_radians();
    let scaled_x = x * transform.scale;
    let scaled_y = y * transform.scale;
    (
        scaled_x * radians.cos() - scaled_y * radians.sin() + transform.translate_x,
        scaled_x * radians.sin() + scaled_y * radians.cos() + transform.translate_y,
    )
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

fn point_segment_distance(x: f64, y: f64, from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> f64 {
    let delta_x = to_x - from_x;
    let delta_y = to_y - from_y;
    let length_squared = delta_x.mul_add(delta_x, delta_y * delta_y);
    if length_squared <= f64::EPSILON {
        return (x - from_x).hypot(y - from_y);
    }
    let projection = ((x - from_x) * delta_x + (y - from_y) * delta_y) / length_squared;
    let projection = projection.clamp(0.0, 1.0);
    (x - (from_x + projection * delta_x)).hypot(y - (from_y + projection * delta_y))
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum CanvasError {
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

fn canvas_rect(
    key: ImmutableString,
    x: FLOAT,
    y: FLOAT,
    width: FLOAT,
    height: FLOAT,
    fill: ColorValue,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    Ok(CanvasCommand::Rect {
        key: validate_key(key)?,
        x: finite("rect.x", x)?,
        y: finite("rect.y", y)?,
        width: positive("rect.width", width)?,
        height: positive("rect.height", height)?,
        fill,
    })
}

fn canvas_circle(
    key: ImmutableString,
    center_x: FLOAT,
    center_y: FLOAT,
    radius: FLOAT,
    fill: ColorValue,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    Ok(CanvasCommand::Circle {
        key: validate_key(key)?,
        center_x: finite("circle.center_x", center_x)?,
        center_y: finite("circle.center_y", center_y)?,
        radius: positive("circle.radius", radius)?,
        fill,
    })
}

#[allow(clippy::too_many_arguments)]
fn canvas_line(
    key: ImmutableString,
    from_x: FLOAT,
    from_y: FLOAT,
    to_x: FLOAT,
    to_y: FLOAT,
    width: FLOAT,
    color: ColorValue,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    Ok(CanvasCommand::Line {
        key: validate_key(key)?,
        from_x: finite("line.from_x", from_x)?,
        from_y: finite("line.from_y", from_y)?,
        to_x: finite("line.to_x", to_x)?,
        to_y: finite("line.to_y", to_y)?,
        width: positive("line.width", width)?,
        color,
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

fn canvas_fill_path(
    key: ImmutableString,
    segments: Array,
    fill: ColorValue,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    canvas_path(key, segments, Some(CanvasFill::Solid(fill)), None)
}

fn canvas_gradient_path(
    key: ImmutableString,
    segments: Array,
    fill: LinearGradientSpec,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    canvas_path(key, segments, Some(CanvasFill::LinearGradient(fill)), None)
}

fn canvas_stroke_path(
    key: ImmutableString,
    segments: Array,
    width: FLOAT,
    color: ColorValue,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    let width = positive("path.stroke_width", width)?;
    canvas_path(key, segments, None, Some((color, width)))
}

fn canvas_morph_stroke_path(
    key: ImmutableString,
    from: Array,
    to: Array,
    width: FLOAT,
    color: ColorValue,
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
        stroke: (color, positive("path.stroke_width", width)?),
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
    stroke: Option<(ColorValue, f64)>,
) -> Result<CanvasCommand, Box<EvalAltResult>> {
    Ok(CanvasCommand::Path {
        key: validate_key(key)?,
        segments: parse_path_segments(segments)?,
        fill,
        stroke,
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

pub(crate) fn register_canvas_api(engine: &mut Engine) {
    engine.build_type::<CanvasPathSegment>();
    engine.build_type::<CanvasCommand>();
    engine.build_type::<CanvasScene>();
    FuncRegistration::new("canvas_rect")
        .in_global_namespace()
        .register_into_engine(engine, canvas_rect);
    FuncRegistration::new("canvas_circle")
        .in_global_namespace()
        .register_into_engine(engine, canvas_circle);
    FuncRegistration::new("canvas_line")
        .in_global_namespace()
        .register_into_engine(engine, canvas_line);
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
    FuncRegistration::new("canvas_fill_path")
        .in_global_namespace()
        .register_into_engine(engine, canvas_fill_path);
    FuncRegistration::new("canvas_fill_path")
        .in_global_namespace()
        .register_into_engine(engine, canvas_gradient_path);
    FuncRegistration::new("canvas_stroke_path")
        .in_global_namespace()
        .register_into_engine(engine, canvas_stroke_path);
    FuncRegistration::new("canvas_morph_stroke_path")
        .in_global_namespace()
        .register_into_engine(engine, canvas_morph_stroke_path);
    FuncRegistration::new("canvas_scene")
        .in_global_namespace()
        .register_into_engine(engine, canvas_scene);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_rejects_duplicate_keys() {
        let command = CanvasCommand::Rect {
            key: "same".to_owned(),
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            fill: ColorValue::Literal(crate::Rgba8::from_rgb_hex(0x00ff_0000)),
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
    fn scene_hit_testing_uses_reverse_paint_order_and_path_geometry() {
        let under = CanvasCommand::Rect {
            key: "under".to_owned(),
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
            fill: ColorValue::Token("surface".to_owned()),
        };
        let path = CanvasCommand::Path {
            key: "triangle".to_owned(),
            segments: vec![
                CanvasPathSegment::Move { x: 0.0, y: 0.0 },
                CanvasPathSegment::Line { x: 100.0, y: 0.0 },
                CanvasPathSegment::Line { x: 0.0, y: 100.0 },
                CanvasPathSegment::Close,
            ],
            fill: Some(CanvasFill::Solid(ColorValue::Token("accent".to_owned()))),
            stroke: None,
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
            stroke: (ColorValue::Token("accent".to_owned()), 2.0),
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
            stroke: (ColorValue::Token("accent".to_owned()), 10.0),
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
            stroke: Some((ColorValue::Token("accent".to_owned()), 10.0)),
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
