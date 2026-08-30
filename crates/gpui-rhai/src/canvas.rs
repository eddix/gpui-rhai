use std::collections::BTreeSet;

use rhai::{
    Array, CustomType, Engine, EvalAltResult, FLOAT, FuncRegistration, ImmutableString, Position,
    TypeBuilder,
};
use thiserror::Error;

use crate::ColorValue;

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
}

impl CanvasCommand {
    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::Rect { key, .. } | Self::Circle { key, .. } | Self::Line { key, .. } => key,
        }
    }
}

impl CustomType for CanvasCommand {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("CanvasCommand");
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
}
