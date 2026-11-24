//! Local generic layered-sRGB-background candidate, independent of SDK skins.
#![allow(missing_docs)]
use crate::{
    Bounds, ContentMask, Corners, DrawOrder, Edges, Fill, LinearColorStop, Pixels, Point, Rgba,
    ScaledPixels, Size, Window, point, size,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const BACKGROUND_LAYER_CAP: usize = 8;
pub const BACKGROUND_STOP_CAP: usize = 16;
pub const BACKGROUND_REPEAT_PERIOD_LIMITS: std::ops::RangeInclusive<f32> = 0.5..=65536.;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum BackgroundBox {
    #[default]
    Border,
    Padding,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BackgroundStop<C = Rgba> {
    pub offset: f32,
    pub color: C,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BackgroundLayer<C = Rgba> {
    pub angle: f32,
    pub stops: Vec<BackgroundStop<C>>,
    pub origin: BackgroundBox,
    pub clip: BackgroundBox,
    pub image_size: [f32; 2],
    pub image_position: [f32; 2],
    /// Logical pixels along the gradient line; absent means a single span.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_period: Option<f32>,
}

impl From<LayeredFill> for crate::Primitive {
    fn from(value: LayeredFill) -> Self {
        Self::LayeredFill(value)
    }
}
impl crate::Scene {
    pub fn has_layered_fills(&self) -> bool {
        if !self.layered_fills.is_empty() {
            return true;
        }
        false
    }
}

impl<C> BackgroundLayer<C> {
    pub fn new(angle: f32, stops: Vec<BackgroundStop<C>>) -> Self {
        Self {
            angle,
            stops,
            origin: BackgroundBox::Border,
            clip: BackgroundBox::Border,
            image_size: [1., 1.],
            image_position: [0., 0.],
            repeat_period: None,
        }
    }
    pub fn boxes(mut self, origin: BackgroundBox, clip: BackgroundBox) -> Self {
        self.origin = origin;
        self.clip = clip;
        self
    }
    pub fn image(mut self, size: [f32; 2], position: [f32; 2]) -> Self {
        self.image_size = size;
        self.image_position = position;
        self
    }
    pub fn repeating(mut self, period: Option<f32>) -> Self {
        self.repeat_period = period;
        self
    }
    pub fn map_colors<D>(&self, mut map: impl FnMut(&C) -> D) -> BackgroundLayer<D> {
        BackgroundLayer {
            angle: self.angle,
            stops: self.stops.iter().map(|stop| BackgroundStop {
                offset: stop.offset,
                color: map(&stop.color),
            }).collect(),
            origin: self.origin,
            clip: self.clip,
            image_size: self.image_size,
            image_position: self.image_position,
            repeat_period: self.repeat_period,
        }
    }
    fn validate_geometry(&self) -> Result<(), String> {
        if self.repeat_period.is_some_and(|period| !BACKGROUND_REPEAT_PERIOD_LIMITS.contains(&period)) {
            return Err("invalid repeating gradient period".into());
        }
        if !self.angle.is_finite()
            || self.image_size.iter().any(|v| !v.is_finite() || *v <= 0.)
            || self.image_position.iter().any(|v| !v.is_finite())
        {
            return Err("invalid gradient image geometry".into());
        }
        if !(2..=BACKGROUND_STOP_CAP).contains(&self.stops.len()) {
            return Err("unsupported gradient stop count".into());
        }
        let mut prior = None;
        for stop in &self.stops {
            let t = stop.offset;
            if !t.is_finite() || !(0. ..=1.).contains(&t) || (t != 0. && !t.is_normal()) {
                return Err("invalid/unrepresentable gradient stop".into());
            }
            if let Some(p) = prior {
                if t < p || (t != p && !(t - p).is_normal()) {
                    return Err("non-monotonic/unrepresentable gradient interval".into());
                }
            }
            prior = Some(t);
        }
        Ok(())
    }
}

/// A paint declaration whose foreground is resolved after native refinement.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum PaintInk {
    Literal(Rgba),
    NativeHsla(crate::Hsla),
    CurrentText { alpha: f32 },
}

impl PaintInk {
    pub fn resolve_hsla(self, current: crate::Hsla) -> crate::Hsla {
        match self {
            Self::NativeHsla(ink) => ink,
            Self::CurrentText { alpha } => current.opacity(alpha),
            Self::Literal(ink) => ink.into(),
        }
    }

    pub fn resolve(self, current: crate::Hsla) -> Rgba {
        match self {
            Self::Literal(ink) => ink,
            Self::NativeHsla(ink) => ink.into(),
            Self::CurrentText { alpha } => {
                let mut ink: Rgba = current.into();
                ink.a *= alpha;
                ink
            }
        }
    }

    fn validate(&self) -> bool {
        match self {
            Self::Literal(c) => valid_rgba(*c),
            Self::NativeHsla(c) => valid_rgba((*c).into()),
            Self::CurrentText { alpha } => (0. ..=1.).contains(alpha),
        }
    }
}

impl From<crate::Hsla> for PaintInk {
    fn from(value: crate::Hsla) -> Self {
        Self::NativeHsla(value)
    }
}

fn valid_rgba(c: Rgba) -> bool {
    [c.r, c.g, c.b, c.a].iter().all(|v| (0. ..=1.).contains(v))
}

/// Contextual stack; only the winning declaration is resolved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "Vec<BackgroundLayer<PaintInk>>", into = "Vec<BackgroundLayer<PaintInk>>")]
pub struct ContextualBackground(Arc<[BackgroundLayer<PaintInk>]>);

impl TryFrom<Vec<BackgroundLayer<PaintInk>>> for ContextualBackground {
    type Error = String;
    fn try_from(layers: Vec<BackgroundLayer<PaintInk>>) -> Result<Self, Self::Error> {
        check_layers(&layers, PaintInk::validate)?;
        Ok(Self(layers.into()))
    }
}

impl From<ContextualBackground> for Vec<BackgroundLayer<PaintInk>> {
    fn from(value: ContextualBackground) -> Self {
        value.0.to_vec()
    }
}

impl ContextualBackground {
    pub fn resolve(&self, current: crate::Hsla) -> LayeredBackground {
        LayeredBackground(self.0.iter().map(|layer| layer.map_colors(|ink| ink.resolve(current))).collect())
    }
}

fn check_layers<C>(layers: &[BackgroundLayer<C>], valid_ink: impl Fn(&C) -> bool) -> Result<(), String> {
    if layers.len() > BACKGROUND_LAYER_CAP {
        return Err("unsupported background layer count".into());
    }
    for layer in layers {
        layer.validate_geometry()?;
        if layer.stops.iter().any(|stop| !valid_ink(&stop.color)) {
            return Err("invalid sRGB stop color".into());
        }
    }
    Ok(())
}
impl From<LinearColorStop> for BackgroundStop {
    fn from(stop: LinearColorStop) -> Self {
        Self {
            offset: stop.percentage,
            color: stop.color.into(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "Vec<BackgroundLayer>", into = "Vec<BackgroundLayer>")]
pub struct LayeredBackground(Arc<[BackgroundLayer]>);
impl LayeredBackground {
    pub fn new(layers: Vec<BackgroundLayer>) -> Result<Self, String> {
        check_layers(&layers, |ink| valid_rgba(*ink))?;
        Ok(Self(layers.into()))
    }
    pub fn layers(&self) -> &[BackgroundLayer] {
        &self.0
    }
}
impl TryFrom<Vec<BackgroundLayer>> for LayeredBackground {
    type Error = String;
    fn try_from(layers: Vec<BackgroundLayer>) -> Result<Self, String> {
        Self::new(layers)
    }
}
impl From<LayeredBackground> for Vec<BackgroundLayer> {
    fn from(value: LayeredBackground) -> Self {
        value.0.to_vec()
    }
}
impl From<LayeredBackground> for Fill {
    fn from(value: LayeredBackground) -> Self {
        Self::Layers(value)
    }
}
#[derive(Clone, Debug)]
pub struct ResolvedBackgroundLayer {
    /// Relative to the primitive's bounds, for inexpensive cached translation.
    pub origin: Bounds<ScaledPixels>,
    pub clip: Bounds<ScaledPixels>,
    pub radii_x: Corners<ScaledPixels>,
    pub radii_y: Corners<ScaledPixels>,
    pub direction: Point<f32>,
    /// In the same projected device units as dot(point, direction).
    pub repeat_period: Option<ScaledPixels>,
    pub stops: Arc<[BackgroundStop]>,
}
#[derive(Clone, Debug)]
pub struct LayeredFill {
    pub order: DrawOrder,
    pub bounds: Bounds<ScaledPixels>,
    pub content_mask: ContentMask<ScaledPixels>,
    pub opacity: f32,
    pub layers: Arc<[ResolvedBackgroundLayer]>,
}
impl LayeredFill {
    pub fn translated(&self, delta: Point<ScaledPixels>) -> Self {
        let mut result = self.clone();
        result.bounds.origin = result.bounds.origin + delta;
        result.content_mask.bounds.origin = result.content_mask.bounds.origin + delta;
        result
    }
}
fn direction(angle: f32) -> Point<f32> {
    let a = angle.rem_euclid(360.);
    let (x, y) = match a {
        0. => (0., -1.),
        45. => (1., -1.),
        90. => (1., 0.),
        135. => (1., 1.),
        180. => (0., 1.),
        225. => (-1., 1.),
        270. => (-1., 0.),
        315. => (-1., -1.),
        _ => {
            let r = a.to_radians();
            (r.sin(), -r.cos())
        }
    };
    point(x, y)
}
fn inset_radii(
    r: Corners<ScaledPixels>,
    b: Edges<ScaledPixels>,
) -> (Corners<ScaledPixels>, Corners<ScaledPixels>) {
    let minus = |a: ScaledPixels, b: ScaledPixels| ScaledPixels((a.0 - b.0).max(0.));
    (
        Corners {
            top_left: minus(r.top_left, b.left),
            top_right: minus(r.top_right, b.right),
            bottom_right: minus(r.bottom_right, b.right),
            bottom_left: minus(r.bottom_left, b.left),
        },
        Corners {
            top_left: minus(r.top_left, b.top),
            top_right: minus(r.top_right, b.top),
            bottom_right: minus(r.bottom_right, b.bottom),
            bottom_left: minus(r.bottom_left, b.bottom),
        },
    )
}
impl LayeredBackground {
    pub fn resolve(
        &self,
        extent: Size<ScaledPixels>,
        corners: Corners<ScaledPixels>,
        borders: Edges<ScaledPixels>,
    ) -> Result<Arc<[ResolvedBackgroundLayer]>, String> {
        self.resolve_at_scale(extent, corners, borders, 1.)
    }

    pub fn resolve_at_scale(
        &self,
        extent: Size<ScaledPixels>,
        corners: Corners<ScaledPixels>,
        borders: Edges<ScaledPixels>,
        device_scale: f32,
    ) -> Result<Arc<[ResolvedBackgroundLayer]>, String> {
        if [
            extent.width.0,
            extent.height.0,
            corners.top_left.0,
            corners.top_right.0,
            corners.bottom_right.0,
            corners.bottom_left.0,
            borders.top.0,
            borders.bottom.0,
            borders.left.0,
            borders.right.0,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.)
        {
            return Err("invalid background box geometry".into());
        }
        let border = Bounds::new(Point::default(), extent);
        let padding = Bounds::new(
            point(borders.left, borders.top),
            size(
                ScaledPixels((extent.width.0 - borders.left.0 - borders.right.0).max(0.)),
                ScaledPixels((extent.height.0 - borders.top.0 - borders.bottom.0).max(0.)),
            ),
        );
        let (inner_x, inner_y) = inset_radii(corners, borders);
        let mut resolved = Vec::with_capacity(self.0.len());
        for layer in &*self.0 {
            let base = match layer.origin {
                BackgroundBox::Border => border,
                BackgroundBox::Padding => padding,
            };
            let (clip, rx, ry) = match layer.clip {
                BackgroundBox::Border => (border, corners, corners),
                BackgroundBox::Padding => (padding, inner_x, inner_y),
            };
            if base.is_empty() || clip.is_empty() {
                continue;
            }
            let w = base.size.width.0 * layer.image_size[0];
            let h = base.size.height.0 * layer.image_size[1];
            let x = base.origin.x.0 + (base.size.width.0 - w) * layer.image_position[0];
            let y = base.origin.y.0 + (base.size.height.0 - h) * layer.image_position[1];
            let d = direction(layer.angle);
            let span = d.x.abs() * w + d.y.abs() * h;
            // Keep the exact octant vectors used for legacy fractional
            // offsets, but convert a physical period to their dot units.
            let repeat_period = layer.repeat_period.map(|period| period * device_scale * d.x.hypot(d.y));
            if repeat_period.is_some_and(|period| period <= 0. || !period.is_normal()) {
                return Err("repeating gradient period is outside finite native geometry".into());
            }
            if [w, h, x, y, x + w, y + h, span]
                .iter()
                .any(|v| !v.is_finite())
                || w <= 0.
                || h <= 0.
                || span <= 0.
                || !span.is_normal()
            {
                return Err("background projection is outside finite native geometry".into());
            }
            resolved.push(ResolvedBackgroundLayer {
                origin: Bounds::new(
                    point(ScaledPixels(x), ScaledPixels(y)),
                    size(ScaledPixels(w), ScaledPixels(h)),
                ),
                clip,
                radii_x: rx,
                radii_y: ry,
                direction: d,
                repeat_period: repeat_period.map(ScaledPixels),
                stops: layer.stops.clone().into(),
            });
        }
        Ok(resolved.into())
    }
}
impl Window {
    /// Paint one generic layered background before content. Layers are topmost-first;
    /// this records no input target, timeline, image asset or readback.
    pub fn paint_layered_background(
        &mut self,
        bounds: Bounds<Pixels>,
        corners: Corners<Pixels>,
        borders: Edges<Pixels>,
        background: &LayeredBackground,
    ) -> Result<(), String> {
        self.invalidator.debug_assert_paint();
        if bounds.is_empty() || background.layers().is_empty() {
            return Ok(());
        }
        let sf = self.scale_factor();
        let scaled = self.snap_bounds(bounds);
        let layers = background.resolve_at_scale(scaled.size, corners.scale(sf), borders.scale(sf), sf)?;
        if layers.is_empty() {
            return Ok(());
        }
        let fill = LayeredFill {
            order: 0,
            bounds: scaled,
            content_mask: self.snapped_content_mask(),
            opacity: self.element_opacity(),
            layers,
        };
        self.next_frame.scene.insert_primitive(fill);
        Ok(())
    }
}
