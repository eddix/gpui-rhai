use rhai::{Array, CustomType, Dynamic, EvalAltResult, ImmutableString, Map, TypeBuilder};
use serde::{Deserialize, Serialize};

use crate::style::{optional_map_number, reject_style_map_unknown, style_runtime_error};
use crate::{ColorValue, LinearGradientSpec, PaintColor, Rgba8};

/// One background property. Merging a style replaces the entire fill;
/// an empty layered gradient explicitly clears it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum BackgroundFill {
    Solid(PaintColor),
    /// Original native two-stop behavior, also used by Canvas paths.
    LegacyLinear(LinearGradientSpec),
    /// Premultiplied sRGB gradient layers, topmost first.
    Layers(LayeredGradient),
}

impl BackgroundFill {
    pub fn as_solid_value(&self) -> Option<&ColorValue> {
        match self {
            Self::Solid(color) => color.as_value(),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FillBox {
    #[default]
    Border,
    Padding,
}

impl From<FillBox> for gpui::BackgroundBox {
    fn from(value: FillBox) -> Self {
        match value {
            FillBox::Border => Self::Border,
            FillBox::Padding => Self::Padding,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GradientStop {
    pub offset: f64,
    pub color: PaintColor,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GradientLayer {
    pub angle: f64,
    pub stops: Vec<GradientStop>,
    pub origin: FillBox,
    pub clip: FillBox,
    pub size: [f64; 2],
    pub position: [f64; 2],
    /// Absolute logical pixels along the gradient line, independent of size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_period: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<GradientLayer>", into = "Vec<GradientLayer>")]
pub struct LayeredGradient(Vec<GradientLayer>);

impl CustomType for LayeredGradient {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("LayeredGradient");
    }
}

impl From<LayeredGradient> for Vec<GradientLayer> {
    fn from(value: LayeredGradient) -> Self {
        value.0
    }
}

impl TryFrom<Vec<GradientLayer>> for LayeredGradient {
    type Error = GradientError;
    fn try_from(mut layers: Vec<GradientLayer>) -> Result<Self, Self::Error> {
        if layers.len() > gpui::BACKGROUND_LAYER_CAP {
            return Err(GradientError::TooManyLayers);
        }
        for layer in &mut layers {
            if layer.repeat_period.is_some_and(|period| {
                let bounds = gpui::BACKGROUND_REPEAT_PERIOD_LIMITS;
                !(f64::from(*bounds.start())..=f64::from(*bounds.end())).contains(&period)
            }) {
                return Err(GradientError::RepeatPeriod);
            }
            if !layer.angle.is_finite()
                || layer.size.iter().any(|v| !(0.0001..=1024.).contains(v))
                || layer
                    .position
                    .iter()
                    .any(|v| !(-1024. ..=1024.).contains(v))
            {
                return Err(GradientError::Geometry);
            }
            layer.angle = layer.angle.rem_euclid(360.);
            if !(2..=gpui::BACKGROUND_STOP_CAP).contains(&layer.stops.len()) {
                return Err(GradientError::StopCount);
            }
            let mut previous: Option<(f64, f32)> = None;
            for stop in &mut layer.stops {
                if stop.offset == 0. {
                    stop.offset = 0.;
                }
                let current = crate::renderer::f64_to_f32(stop.offset);
                if !(0. ..=1.).contains(&stop.offset)
                    || (stop.offset != 0. && !current.is_normal())
                    || previous.is_some_and(|(offset, rounded)| {
                        stop.offset < offset
                            || (stop.offset > offset && !(current - rounded).is_normal())
                    })
                {
                    return Err(GradientError::Stops);
                }
                previous = Some((stop.offset, current));
            }
        }
        Ok(Self(layers))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum GradientError {
    #[error("layered gradient exceeds the native layer budget")]
    TooManyLayers,
    #[error("repeat_period must be between 0.5 and 65536 logical pixels")]
    RepeatPeriod,
    #[error(
        "gradient image must have a finite angle, size in [0.0001, 1024], and position in [-1024, 1024]"
    )]
    Geometry,
    #[error("each layer requires 2 to 16 gradient stops")]
    StopCount,
    #[error(
        "gradient offsets must be ordered and representable in [0, 1]; equal offsets form a hard stop"
    )]
    Stops,
}

impl LayeredGradient {
    pub fn layers(&self) -> &[GradientLayer] {
        &self.0
    }

    pub(crate) fn native(
        &self,
        mut resolve: impl FnMut(&ColorValue) -> Option<Rgba8>,
    ) -> Option<gpui::Fill> {
        use crate::renderer::f64_to_f32 as f;
        let mut contextual = false;
        let layers = self
            .0
            .iter()
            .map(|layer| {
                let stops = layer
                    .stops
                    .iter()
                    .map(|stop| {
                        let color = stop.color.native_ink(&mut resolve)?;
                        contextual |= matches!(color, gpui::PaintInk::CurrentText { .. });
                        Some(gpui::BackgroundStop {
                            offset: f(stop.offset),
                            color,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some(
                    gpui::BackgroundLayer::new(f(layer.angle), stops)
                        .boxes(layer.origin.into(), layer.clip.into())
                        .image(layer.size.map(f), layer.position.map(f))
                        .repeating(layer.repeat_period.map(f)),
                )
            })
            .collect::<Option<Vec<_>>>()?;
        if contextual {
            gpui::ContextualBackground::try_from(layers)
                .ok()
                .map(gpui::Fill::ContextualLayers)
        } else {
            // No live foreground dependency: keep the normal native layer
            // stack, including the existing retained scroll/tile paths.
            let layers = layers
                .into_iter()
                .map(|layer| layer.map_colors(|ink| ink.resolve(gpui::Hsla::default())))
                .collect();
            gpui::LayeredBackground::new(layers)
                .ok()
                .map(gpui::Fill::Layers)
        }
    }
}

type RhaiResult<T> = Result<T, Box<EvalAltResult>>;

#[expect(
    clippy::unnecessary_box_returns,
    reason = "Rhai's fallible native API requires Box<EvalAltResult>"
)]
fn invalid(message: impl Into<String>) -> Box<EvalAltResult> {
    Box::new(style_runtime_error(message.into()))
}

pub(crate) fn paint_color(value: Dynamic) -> RhaiResult<PaintColor> {
    if value.is::<PaintColor>() {
        value
            .try_cast()
            .ok_or_else(|| invalid("invalid PaintColor"))
    } else {
        value
            .try_cast::<ColorValue>()
            .map(Into::into)
            .ok_or_else(|| invalid("paint requires ColorValue or PaintColor"))
    }
}

pub(crate) fn layered_gradient(values: Array) -> RhaiResult<LayeredGradient> {
    if values.len() > gpui::BACKGROUND_LAYER_CAP {
        return Err(invalid(GradientError::TooManyLayers.to_string()));
    }
    let layers = values
        .into_iter()
        .map(|value| {
            let mut layer = value
                .try_cast::<Map>()
                .ok_or_else(|| invalid("gradient layer must be a map"))?;
            let stops = layer
                .remove("stops")
                .and_then(Dynamic::try_cast::<Array>)
                .ok_or_else(|| invalid("gradient layer requires a stops array"))?;
            if !(2..=gpui::BACKGROUND_STOP_CAP).contains(&stops.len()) {
                return Err(invalid(GradientError::StopCount.to_string()));
            }
            let stops = stops
                .into_iter()
                .map(|value| {
                    let mut stop = value
                        .try_cast::<Map>()
                        .ok_or_else(|| invalid("gradient stop must be a map"))?;
                    let offset = optional_map_number(&mut stop, "offset")?
                        .ok_or_else(|| invalid("gradient stop requires offset"))?;
                    let color = paint_color(
                        stop.remove("color")
                            .ok_or_else(|| invalid("gradient stop requires color"))?,
                    )?;
                    reject_style_map_unknown(stop, "gradient stop")?;
                    Ok(GradientStop { offset, color })
                })
                .collect::<RhaiResult<Vec<_>>>()?;
            let angle = optional_map_number(&mut layer, "angle")?.unwrap_or(180.);
            let origin = paint_box(&mut layer, "origin")?;
            let clip = paint_box(&mut layer, "clip")?;
            let size = read_pair(&mut layer, "size", [1., 1.])?;
            let position = read_pair(&mut layer, "position", [0., 0.])?;
            let repeat_period = optional_map_number(&mut layer, "repeat_period")?;
            reject_style_map_unknown(layer, "gradient layer")?;
            Ok(GradientLayer {
                angle,
                stops,
                origin,
                clip,
                size,
                position,
                repeat_period,
            })
        })
        .collect::<RhaiResult<Vec<_>>>()?;
    LayeredGradient::try_from(layers).map_err(|err| invalid(err.to_string()))
}

fn paint_box(layer: &mut Map, key: &str) -> RhaiResult<FillBox> {
    let Some(value) = layer.remove(key) else {
        return Ok(FillBox::Border);
    };
    match value.try_cast::<ImmutableString>().as_deref() {
        Some("border") => Ok(FillBox::Border),
        Some("padding") => Ok(FillBox::Padding),
        _ => Err(invalid(format!("gradient {key} must be border or padding"))),
    }
}

fn read_pair(layer: &mut Map, key: &str, default: [f64; 2]) -> RhaiResult<[f64; 2]> {
    let Some(value) = layer.remove(key) else {
        return Ok(default);
    };
    let values = value
        .try_cast::<Array>()
        .ok_or_else(|| invalid(format!("gradient {key} must be a pair")))?;
    let [a, b]: [Dynamic; 2] = values
        .try_into()
        .map_err(|_| invalid(format!("gradient {key} must have two values")))?;
    let read = |value: Dynamic| -> RhaiResult<f64> {
        if value.is::<rhai::FLOAT>() {
            Ok(value.cast::<rhai::FLOAT>())
        } else if value.is::<rhai::INT>() {
            crate::style::numeric_to_f64(&value.cast::<rhai::INT>())
        } else {
            Err(invalid(format!("gradient {key} values must be numbers")))
        }
    };
    Ok([read(a)?, read(b)?])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeEngine;

    #[test]
    fn repeating_gradient_keeps_fractional_stops_and_rejects_invalid_periods() {
        let runtime = RuntimeEngine::new();
        let source = |period| {
            format!(
                "layered_gradient([#{{angle:135, {period} stops:[#{{offset:0,color:rgb(0x446677)}},#{{offset:1,color:rgb(0x112233)}}]}}])"
            )
        };
        let stack = runtime
            .engine()
            .eval::<LayeredGradient>(&source("repeat_period:7,"))
            .unwrap();
        assert_eq!(stack.layers()[0].repeat_period, Some(7.0));
        let encoded = serde_json::to_string(&stack).unwrap();
        assert_eq!(
            serde_json::from_str::<LayeredGradient>(&encoded).unwrap(),
            stack
        );
        let layer = runtime
            .engine()
            .eval::<LayeredGradient>(&source(""))
            .unwrap();
        assert_eq!(layer.layers()[0].repeat_period, None);
        assert!(
            !serde_json::to_string(&layer)
                .unwrap()
                .contains("repeat_period")
        );
        for invalid in [
            "repeat_period:0,",
            "repeat_period:0.499,",
            "repeat_period:-7,",
            "repeat_period:65537,",
            "repeat_period:\"7\",",
        ] {
            assert!(
                runtime
                    .engine()
                    .eval::<LayeredGradient>(&source(invalid))
                    .is_err(),
                "{invalid}"
            );
        }
    }
}
