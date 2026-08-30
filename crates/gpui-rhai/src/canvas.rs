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
}

impl CanvasCommand {
    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::Rect { key, .. }
            | Self::Circle { key, .. }
            | Self::Line { key, .. }
            | Self::Path { key, .. } => key,
        }
    }

    #[must_use]
    pub fn complexity(&self) -> usize {
        match self {
            Self::Path { segments, .. } => 1usize.saturating_add(segments.len()),
            Self::Rect { .. } | Self::Circle { .. } | Self::Line { .. } => 1,
        }
    }

    fn transform_mut(&mut self) -> Result<&mut CanvasTransform, CanvasError> {
        match self {
            Self::Path { transform, .. } => Ok(transform),
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
            Self::Path { clip: target, .. } => *target = Some(clip),
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
}

impl CustomType for CanvasScene {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("CanvasScene");
    }
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
}
