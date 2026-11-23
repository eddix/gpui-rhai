//! Paint endpoints for a scalar progress track owned by `MotionRuntime`.
//! Render phases neither advance a second clock nor run script.
use gpui::{
    Background, BoxShadow, ComputedPaint, Fill, Hsla, LayeredBackground, PaintInk, Rgba,
    ShadowStack,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PaintTransition {
    pub duration_ms: u64,
    pub easing: crate::MotionEasing,
    pub delay_ms: u64,
    pub group_effects: bool,
}

pub(crate) const DURATION_MAX_MS: u64 = 60_000;

pub(crate) fn rhai_transition(
    node: &crate::UiNode,
    duration_ms: rhai::INT,
    easing: &str,
    options: &rhai::Map,
) -> Result<crate::UiNode, Box<rhai::EvalAltResult>> {
    let error = |message: &str| {
        Box::new(rhai::EvalAltResult::ErrorRuntime(
            message.into(),
            rhai::Position::NONE,
        ))
    };
    let bounded = |ms| u64::try_from(ms).ok().filter(|ms| *ms <= DURATION_MAX_MS);
    let duration_ms = bounded(duration_ms)
        .ok_or_else(|| error("paint transition duration must be 0..=60000 ms"))?;
    let easing = crate::MotionEasing::parse(easing).map_err(|e| error(&e.to_string()))?;
    let mut delay_ms = 0;
    let mut group_effects = false;
    for (name, value) in options {
        match name.as_str() {
            "delay_ms" => {
                delay_ms = value
                    .as_int()
                    .ok()
                    .and_then(bounded)
                    .ok_or_else(|| error("paint transition delay must be 0..=60000 ms"))?;
            }
            "group_effects" => {
                group_effects = value
                    .as_bool()
                    .map_err(|_| error("paint transition group_effects must be a boolean"))?;
            }
            _ => return Err(error(&format!("unknown paint transition option `{name}`"))),
        }
    }
    Ok(node
        .clone()
        .with_paint_transition_options(duration_ms, easing, delay_ms, group_effects))
}

pub(crate) fn transition(node: &crate::UiNode) -> Option<PaintTransition> {
    let mut spec = read_spec(
        node,
        "paint_transition_duration_ms",
        "paint_transition_easing",
    )?;
    if let Some(crate::UiValue::Integer(delay)) = node.attributes().get("paint_transition_delay_ms")
    {
        spec.delay_ms = u64::try_from(*delay)
            .ok()
            .filter(|ms| *ms <= DURATION_MAX_MS)?;
    }
    spec.group_effects = node.attributes().get("paint_transition_group_effects")
        == Some(&crate::UiValue::Bool(true));
    Some(spec)
}

/// The measured border-box dimension animated by a layout-size transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutSizeAxis {
    Width,
    Height,
}

impl LayoutSizeAxis {
    pub(crate) fn parse(axis: &str) -> Option<Self> {
        match axis {
            "width" => Some(Self::Width),
            "height" => Some(Self::Height),
            _ => None,
        }
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Width => "width",
            Self::Height => "height",
        }
    }

    pub(crate) const fn extent(self, bounds: crate::GeometryBounds) -> f64 {
        match self {
            Self::Width => bounds.width,
            Self::Height => bounds.height,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LayoutSizeTransition<'a> {
    pub key: &'a str,
    pub explicit_axis: Option<LayoutSizeAxis>,
    pub timing: PaintTransition,
}

impl LayoutSizeTransition<'_> {
    pub(crate) const fn axis(self) -> LayoutSizeAxis {
        match self.explicit_axis {
            Some(axis) => axis,
            None => LayoutSizeAxis::Width,
        }
    }
}

pub(crate) fn layout_size(node: &crate::UiNode) -> Option<LayoutSizeTransition<'_>> {
    let crate::UiValue::String(key) = node.attributes().get("intrinsic_width_motion_key")? else {
        return None;
    };
    let explicit_axis = match node.attributes().get("layout_size_motion_axis") {
        None | Some(crate::UiValue::Null) => None,
        Some(crate::UiValue::String(axis)) => Some(LayoutSizeAxis::parse(axis)?),
        Some(_) => return None,
    };
    Some(LayoutSizeTransition {
        key,
        explicit_axis,
        timing: read_spec(
            node,
            "intrinsic_width_motion_duration_ms",
            "intrinsic_width_motion_easing",
        )?,
    })
}

fn read_spec(
    node: &crate::UiNode,
    duration_attr: &str,
    easing_attr: &str,
) -> Option<PaintTransition> {
    let crate::UiValue::Integer(duration) = node.attributes().get(duration_attr)? else {
        return None;
    };
    let duration_ms = u64::try_from(*duration)
        .ok()
        .filter(|ms| *ms <= DURATION_MAX_MS)?;
    let crate::UiValue::String(easing) = node.attributes().get(easing_attr)? else {
        return None;
    };
    Some(PaintTransition {
        duration_ms,
        easing: crate::MotionEasing::parse(easing).ok()?,
        delay_ms: 0,
        group_effects: false,
    })
}

#[derive(Clone, Debug)]
pub(crate) struct PaintEndpoints {
    pub from: ComputedPaint,
    pub to: ComputedPaint,
    pub group_effects: bool,
}

impl PaintEndpoints {
    pub fn sample(&self, progress: f64, parent: Hsla) -> ComputedPaint {
        let t = crate::renderer::f64_to_f32(progress.clamp(0., 1.));
        if t >= 1. {
            return self.to.clone();
        }
        let from = &self.from;
        let to = &self.to;
        let foreground_from = from.text_color.unwrap_or(parent);
        let foreground_to = to.text_color.unwrap_or(parent);
        ComputedPaint {
            paint_reveal_x: to.paint_reveal_x,
            text_color: if from.text_color.is_none() && to.text_color.is_none() {
                None
            } else {
                Some(color(foreground_from.into(), foreground_to.into(), t).into())
            },
            border_color: blend_ink(
                from.border_color,
                to.border_color,
                foreground_from,
                foreground_to,
                t,
            ),
            background: blend_background(
                from.background.as_ref(),
                to.background.as_ref(),
                foreground_from,
                foreground_to,
                t,
            ),
            shadows: blend_shadows(
                &from.shadows,
                &to.shadows,
                foreground_from,
                foreground_to,
                t,
            ),
            // Pair the endpoints as a composed group. Once settled, preserve
            // the target's exact optional opacity and explicit isolation.
            opacity: if t == 0. {
                from.opacity
            } else if from.opacity == to.opacity {
                to.opacity
            } else {
                Some(mix(from.opacity.unwrap_or(1.), to.opacity.unwrap_or(1.), t))
            },
            // A delayed transition must not establish a new backdrop
            // boundary before its first nonzero progress sample.
            isolated_opacity: from.isolated_opacity || (t > 0. && to.isolated_opacity),
            foreground_grayscale: mix(from.foreground_grayscale, to.foreground_grayscale, t),
        }
    }
}

fn color(from: Rgba, to: Rgba, t: f32) -> Rgba {
    let a = mix(from.a, to.a, t);
    if a <= 0. {
        return Rgba::default();
    }
    let channel = |x, y| (mix(x * from.a, y * to.a, t) / a).clamp(0., 1.);
    Rgba {
        r: channel(from.r, to.r),
        g: channel(from.g, to.g),
        b: channel(from.b, to.b),
        a,
    }
}

fn mix(from: f32, to: f32, t: f32) -> f32 {
    from * (1. - t) + to * t
}

fn blend_ink(
    from: Option<PaintInk>,
    to: Option<PaintInk>,
    a_fg: Hsla,
    b_fg: Hsla,
    t: f32,
) -> Option<PaintInk> {
    if from == to {
        return to;
    }
    if let (Some(PaintInk::CurrentText { alpha: a }), Some(PaintInk::CurrentText { alpha: b })) =
        (from, to)
    {
        return Some(PaintInk::CurrentText {
            alpha: mix(a, b, t),
        });
    }
    let a = from.map_or_else(Rgba::default, |ink| ink.resolve(a_fg));
    let b = to.map_or_else(Rgba::default, |ink| ink.resolve(b_fg));
    Some(PaintInk::Literal(color(a, b, t)))
}

fn layers(fill: Option<&Fill>, foreground: Hsla) -> Option<LayeredBackground> {
    match fill {
        Some(Fill::Layers(stack)) => Some(stack.clone()),
        Some(Fill::ContextualLayers(stack)) => Some(stack.resolve(foreground)),
        _ => None,
    }
}

fn blend_background(
    from: Option<&Fill>,
    to: Option<&Fill>,
    a_fg: Hsla,
    b_fg: Hsla,
    t: f32,
) -> Option<Fill> {
    if from == to {
        return to.cloned();
    }
    if let (Some(a), Some(b)) = (layers(from, a_fg), layers(to, b_fg))
        && a.layers().len() == b.layers().len()
    {
        let combined = a
            .layers()
            .iter()
            .zip(b.layers())
            .map(|(a, b)| {
                if a.angle.to_bits() != b.angle.to_bits()
                    || a.origin != b.origin
                    || a.clip != b.clip
                    || a.image_size.map(f32::to_bits) != b.image_size.map(f32::to_bits)
                    || a.image_position.map(f32::to_bits) != b.image_position.map(f32::to_bits)
                    || a.repeat_period.map(f32::to_bits) != b.repeat_period.map(f32::to_bits)
                    || a.stops.len() != b.stops.len()
                {
                    return None;
                }
                let mut layer = b.clone();
                for (stop, a) in layer.stops.iter_mut().zip(&a.stops) {
                    if stop.offset.to_bits() != a.offset.to_bits() {
                        return None;
                    }
                    stop.color = color(a.color, stop.color, t);
                }
                Some(layer)
            })
            .collect::<Option<Vec<_>>>();
        if let Some(result) = combined.and_then(|layers| LayeredBackground::new(layers).ok()) {
            return Some(Fill::Layers(result));
        }
    }
    let solid = |fill: Option<&Fill>, fg| match fill {
        None => Some(Rgba::default()),
        Some(Fill::Color(background)) => background.as_solid().map(Into::into),
        Some(Fill::Contextual(ink)) => Some(ink.resolve(fg)),
        _ => None,
    };
    match (solid(from, a_fg), solid(to, b_fg)) {
        (Some(a), Some(b)) => Some(Fill::Color(Background::from(color(a, b, t)))),
        _ => to.cloned(), // incompatible fill topology is a discrete declaration
    }
}

fn blend_shadows(
    from: &ShadowStack,
    to: &ShadowStack,
    a_fg: Hsla,
    b_fg: Hsla,
    t: f32,
) -> ShadowStack {
    if from == to {
        return to.clone();
    }
    let a = from.resolve(|| a_fg);
    let b = to.resolve(|| b_fg);
    let mut result = Vec::with_capacity(a.len().max(b.len()));
    for i in 0..a.len().max(b.len()) {
        let left = a.get(i);
        let right = b.get(i);
        let Some(target) = right.or(left) else {
            continue;
        };
        let same_geometry = |shadow: &BoxShadow| {
            shadow.offset == target.offset
                && shadow.blur_radius == target.blur_radius
                && shadow.spread_radius == target.spread_radius
                && shadow.inset == target.inset
        };
        if !left.is_none_or(same_geometry) {
            return to.clone();
        }
        let a = left.map_or_else(Rgba::default, |s| s.color.into());
        let b = right.map_or_else(Rgba::default, |s| s.color.into());
        result.push(BoxShadow {
            color: color(a, b, t).into(),
            ..target.clone()
        });
    }
    ShadowStack::Literal(result)
}
