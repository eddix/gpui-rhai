use std::collections::BTreeSet;

use rhai::{
    CustomType, Engine, EvalAltResult, FLOAT, FuncRegistration, INT, ImmutableString, Map,
    Position, TypeBuilder,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "unit", content = "value", rename_all = "snake_case")]
pub enum Length {
    Pixels(f64),
    Rems(f64),
    Relative(f64),
    ThemeSpacing(SpacingToken),
    ThemeRadius(RadiusToken),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpacingToken {
    Xs,
    Sm,
    Md,
    Lg,
}

impl SpacingToken {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Xs => "xs",
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
        }
    }

    fn parse(value: &str) -> Result<Self, LengthError> {
        match value {
            "xs" => Ok(Self::Xs),
            "sm" => Ok(Self::Sm),
            "md" => Ok(Self::Md),
            "lg" => Ok(Self::Lg),
            _ => Err(LengthError::UnknownSpacingToken(value.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadiusToken {
    Sm,
    Md,
    Lg,
}

impl RadiusToken {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sm => "sm",
            Self::Md => "md",
            Self::Lg => "lg",
        }
    }

    fn parse(value: &str) -> Result<Self, LengthError> {
        match value {
            "sm" => Ok(Self::Sm),
            "md" => Ok(Self::Md),
            "lg" => Ok(Self::Lg),
            _ => Err(LengthError::UnknownRadiusToken(value.to_owned())),
        }
    }
}

impl Length {
    /// Create a finite, non-negative pixel length.
    ///
    /// # Errors
    ///
    /// Returns [`LengthError`] for negative or non-finite values.
    pub fn pixels(value: f64) -> Result<Self, LengthError> {
        validate_non_negative(value).map(|()| Self::Pixels(value))
    }

    /// Create a finite, non-negative rem length.
    ///
    /// # Errors
    ///
    /// Returns [`LengthError`] for negative or non-finite values.
    pub fn rems(value: f64) -> Result<Self, LengthError> {
        validate_non_negative(value).map(|()| Self::Rems(value))
    }

    /// Create a finite relative fraction.
    ///
    /// # Errors
    ///
    /// Returns [`LengthError`] unless `value` is between zero and one.
    pub fn relative(value: f64) -> Result<Self, LengthError> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self::Relative(value))
        } else {
            Err(LengthError::InvalidRelative(value))
        }
    }

    /// Create one standard semantic spacing reference.
    ///
    /// # Errors
    ///
    /// Returns [`LengthError`] for an unknown standard token.
    pub fn theme_spacing(value: &str) -> Result<Self, LengthError> {
        SpacingToken::parse(value).map(Self::ThemeSpacing)
    }

    /// Create one standard semantic radius reference.
    ///
    /// # Errors
    ///
    /// Returns [`LengthError`] for an unknown standard token.
    pub fn theme_radius(value: &str) -> Result<Self, LengthError> {
        RadiusToken::parse(value).map(Self::ThemeRadius)
    }

    #[must_use]
    pub const fn is_theme_token(self) -> bool {
        matches!(self, Self::ThemeSpacing(_) | Self::ThemeRadius(_))
    }

    /// Revalidate a deserialized length.
    ///
    /// # Errors
    ///
    /// Returns [`LengthError`] when the stored value is outside its unit's
    /// allowed range.
    pub fn validate(self) -> Result<(), LengthError> {
        match self {
            Self::Pixels(value) | Self::Rems(value) => validate_non_negative(value),
            Self::Relative(value) => Self::relative(value).map(|_| ()),
            Self::ThemeSpacing(_) | Self::ThemeRadius(_) => Ok(()),
        }
    }
}

impl CustomType for Length {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("Length");
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum LengthError {
    #[error("length must be finite and non-negative, got {0}")]
    InvalidLength(f64),
    #[error("relative length must be finite and between 0 and 1, got {0}")]
    InvalidRelative(f64),
    #[error("spacing token `{0}` is unknown; expected xs, sm, md, or lg")]
    UnknownSpacingToken(String),
    #[error("radius token `{0}` is unknown; expected sm, md, or lg")]
    UnknownRadiusToken(String),
}

fn validate_non_negative(value: f64) -> Result<(), LengthError> {
    if value.is_finite() && value >= 0.0 && value <= f64::from(f32::MAX) {
        Ok(())
    } else {
        Err(LengthError::InvalidLength(value))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Rgba8(u32);

impl Rgba8 {
    #[must_use]
    pub const fn from_rgba_hex(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn from_rgb_hex(value: u32) -> Self {
        Self((value << 8) | 0xff)
    }

    #[must_use]
    pub const fn as_rgba_hex(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", content = "value", rename_all = "snake_case")]
pub enum ColorValue {
    Literal(Rgba8),
    Token(String),
}

impl CustomType for ColorValue {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("ColorValue");
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlexDirection {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverflowMode {
    Visible,
    Hidden,
    Scroll,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Justify {
    Start,
    Center,
    End,
    Between,
    Around,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EdgeLengths {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<Length>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right: Option<Length>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom: Option<Length>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<Length>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<Length>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<Length>,
}

impl EdgeLengths {
    #[must_use]
    pub fn all(value: Length) -> Self {
        Self {
            top: Some(value),
            right: Some(value),
            bottom: Some(value),
            left: Some(value),
            start: None,
            end: None,
        }
    }

    #[must_use]
    pub fn horizontal(value: Length) -> Self {
        Self {
            right: Some(value),
            left: Some(value),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn vertical(value: Length) -> Self {
        Self {
            top: Some(value),
            bottom: Some(value),
            ..Self::default()
        }
    }

    fn merge(&mut self, overlay: &Self) {
        merge_option(&mut self.top, overlay.top);
        merge_option(&mut self.right, overlay.right);
        merge_option(&mut self.bottom, overlay.bottom);
        merge_option(&mut self.left, overlay.left);
        merge_option(&mut self.start, overlay.start);
        merge_option(&mut self.end, overlay.end);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StyleProperties {
    pub direction: Option<FlexDirection>,
    pub align: Option<Align>,
    pub justify: Option<Justify>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub min_width: Option<Length>,
    pub max_width: Option<Length>,
    pub min_height: Option<Length>,
    pub max_height: Option<Length>,
    pub gap: Option<Length>,
    #[serde(default)]
    pub padding: EdgeLengths,
    #[serde(default)]
    pub margin: EdgeLengths,
    pub background: Option<ColorValue>,
    pub text_color: Option<ColorValue>,
    pub border_color: Option<ColorValue>,
    pub border_width: Option<Length>,
    pub radius: Option<Length>,
    pub font_size: Option<Length>,
    pub flex_grow: Option<bool>,
    pub clip: Option<bool>,
    pub overflow_x: Option<OverflowMode>,
    pub overflow_y: Option<OverflowMode>,
}

impl StyleProperties {
    fn merge(&mut self, overlay: &Self) {
        merge_option(&mut self.direction, overlay.direction);
        merge_option(&mut self.align, overlay.align);
        merge_option(&mut self.justify, overlay.justify);
        merge_option(&mut self.width, overlay.width);
        merge_option(&mut self.height, overlay.height);
        merge_option(&mut self.min_width, overlay.min_width);
        merge_option(&mut self.max_width, overlay.max_width);
        merge_option(&mut self.min_height, overlay.min_height);
        merge_option(&mut self.max_height, overlay.max_height);
        merge_option(&mut self.gap, overlay.gap);
        self.padding.merge(&overlay.padding);
        self.margin.merge(&overlay.margin);
        merge_option(&mut self.background, overlay.background.clone());
        merge_option(&mut self.text_color, overlay.text_color.clone());
        merge_option(&mut self.border_color, overlay.border_color.clone());
        merge_option(&mut self.border_width, overlay.border_width);
        merge_option(&mut self.radius, overlay.radius);
        merge_option(&mut self.font_size, overlay.font_size);
        merge_option(&mut self.flex_grow, overlay.flex_grow);
        merge_option(&mut self.clip, overlay.clip);
        merge_option(&mut self.overflow_x, overlay.overflow_x);
        merge_option(&mut self.overflow_y, overlay.overflow_y);
    }
}

fn merge_option<T>(base: &mut Option<T>, overlay: Option<T>) {
    if overlay.is_some() {
        *base = overlay;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Style {
    #[serde(default)]
    pub base: StyleProperties,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover: Option<StyleProperties>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<StyleProperties>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<StyleProperties>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<StyleProperties>,
}

impl Style {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn merged(mut self, overlay: &Self) -> Self {
        self.base.merge(&overlay.base);
        merge_pseudo(&mut self.hover, overlay.hover.as_ref());
        merge_pseudo(&mut self.active, overlay.active.as_ref());
        merge_pseudo(&mut self.focus, overlay.focus.as_ref());
        merge_pseudo(&mut self.disabled, overlay.disabled.as_ref());
        self
    }

    #[must_use]
    pub fn resolve(&self, state: &InteractionState) -> StyleProperties {
        let mut resolved = self.base.clone();
        if state.contains(PseudoState::Hovered)
            && let Some(hover) = &self.hover
        {
            resolved.merge(hover);
        }
        if state.contains(PseudoState::Active)
            && let Some(active) = &self.active
        {
            resolved.merge(active);
        }
        if state.contains(PseudoState::Focused)
            && let Some(focus) = &self.focus
        {
            resolved.merge(focus);
        }
        if state.contains(PseudoState::Disabled)
            && let Some(disabled) = &self.disabled
        {
            resolved.merge(disabled);
        }
        resolved
    }

    #[must_use]
    pub fn width(mut self, value: Length) -> Self {
        self.base.width = Some(value);
        self
    }

    #[must_use]
    pub fn height(mut self, value: Length) -> Self {
        self.base.height = Some(value);
        self
    }

    #[must_use]
    pub fn max_width(mut self, value: Length) -> Self {
        self.base.max_width = Some(value);
        self
    }

    #[must_use]
    pub fn gap(mut self, value: Length) -> Self {
        self.base.gap = Some(value);
        self
    }

    #[must_use]
    pub fn padding(mut self, value: Length) -> Self {
        self.base.padding = EdgeLengths::all(value);
        self
    }

    #[must_use]
    pub fn padding_x(mut self, value: Length) -> Self {
        self.base.padding.merge(&EdgeLengths::horizontal(value));
        self
    }

    #[must_use]
    pub fn padding_y(mut self, value: Length) -> Self {
        self.base.padding.merge(&EdgeLengths::vertical(value));
        self
    }

    #[must_use]
    pub fn padding_top(mut self, value: Length) -> Self {
        self.base.padding.top = Some(value);
        self
    }

    #[must_use]
    pub fn padding_start(mut self, value: Length) -> Self {
        self.base.padding.start = Some(value);
        self
    }

    #[must_use]
    pub fn padding_end(mut self, value: Length) -> Self {
        self.base.padding.end = Some(value);
        self
    }

    #[must_use]
    pub fn margin_start(mut self, value: Length) -> Self {
        self.base.margin.start = Some(value);
        self
    }

    #[must_use]
    pub fn margin_end(mut self, value: Length) -> Self {
        self.base.margin.end = Some(value);
        self
    }

    #[must_use]
    pub fn background(mut self, value: ColorValue) -> Self {
        self.base.background = Some(value);
        self
    }

    #[must_use]
    pub fn text_color(mut self, value: ColorValue) -> Self {
        self.base.text_color = Some(value);
        self
    }

    #[must_use]
    pub fn radius(mut self, value: Length) -> Self {
        self.base.radius = Some(value);
        self
    }

    #[must_use]
    pub fn border(mut self, value: Length) -> Self {
        self.base.border_width = Some(value);
        self
    }

    #[must_use]
    pub fn border_color(mut self, value: ColorValue) -> Self {
        self.base.border_color = Some(value);
        self
    }

    #[must_use]
    pub fn font_size(mut self, value: Length) -> Self {
        self.base.font_size = Some(value);
        self
    }

    #[must_use]
    pub fn flex_row(mut self) -> Self {
        self.base.direction = Some(FlexDirection::Row);
        self
    }

    #[must_use]
    pub fn flex_col(mut self) -> Self {
        self.base.direction = Some(FlexDirection::Column);
        self
    }

    #[must_use]
    pub fn items_center(mut self) -> Self {
        self.base.align = Some(Align::Center);
        self
    }

    #[must_use]
    pub fn items_start(mut self) -> Self {
        self.base.align = Some(Align::Start);
        self
    }

    #[must_use]
    pub fn items_end(mut self) -> Self {
        self.base.align = Some(Align::End);
        self
    }

    #[must_use]
    pub fn justify_center(mut self) -> Self {
        self.base.justify = Some(Justify::Center);
        self
    }

    #[must_use]
    pub fn justify_start(mut self) -> Self {
        self.base.justify = Some(Justify::Start);
        self
    }

    #[must_use]
    pub fn justify_end(mut self) -> Self {
        self.base.justify = Some(Justify::End);
        self
    }

    #[must_use]
    pub fn justify_between(mut self) -> Self {
        self.base.justify = Some(Justify::Between);
        self
    }

    #[must_use]
    pub fn clip(mut self) -> Self {
        self.base.clip = Some(true);
        self
    }

    #[must_use]
    pub fn overflow_x_scroll(mut self) -> Self {
        self.base.overflow_x = Some(OverflowMode::Scroll);
        self
    }

    #[must_use]
    pub fn overflow_y_scroll(mut self) -> Self {
        self.base.overflow_y = Some(OverflowMode::Scroll);
        self
    }

    #[must_use]
    pub fn overflow_scroll(mut self) -> Self {
        self.base.overflow_x = Some(OverflowMode::Scroll);
        self.base.overflow_y = Some(OverflowMode::Scroll);
        self
    }

    #[must_use]
    pub fn overflow_hidden(mut self) -> Self {
        self.base.overflow_x = Some(OverflowMode::Hidden);
        self.base.overflow_y = Some(OverflowMode::Hidden);
        self
    }

    #[must_use]
    pub fn hover(mut self, style: &Self) -> Self {
        merge_pseudo(&mut self.hover, Some(&style.base));
        self
    }

    #[must_use]
    pub fn active(mut self, style: &Self) -> Self {
        merge_pseudo(&mut self.active, Some(&style.base));
        self
    }

    #[must_use]
    pub fn focus(mut self, style: &Self) -> Self {
        merge_pseudo(&mut self.focus, Some(&style.base));
        self
    }

    #[must_use]
    pub fn disabled(mut self, style: &Self) -> Self {
        merge_pseudo(&mut self.disabled, Some(&style.base));
        self
    }
}

fn merge_pseudo(base: &mut Option<StyleProperties>, overlay: Option<&StyleProperties>) {
    if let Some(overlay) = overlay {
        base.get_or_insert_with(StyleProperties::default)
            .merge(overlay);
    }
}

impl CustomType for Style {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("Style")
            .with_fn("width", |style: &mut Self, value: Length| {
                style.clone().width(value)
            })
            .with_fn("height", |style: &mut Self, value: Length| {
                style.clone().height(value)
            })
            .with_fn("max_width", |style: &mut Self, value: Length| {
                style.clone().max_width(value)
            })
            .with_fn("gap", |style: &mut Self, value: Length| {
                style.clone().gap(value)
            })
            .with_fn("padding", |style: &mut Self, value: Length| {
                style.clone().padding(value)
            })
            .with_fn("padding_x", |style: &mut Self, value: Length| {
                style.clone().padding_x(value)
            })
            .with_fn("padding_y", |style: &mut Self, value: Length| {
                style.clone().padding_y(value)
            })
            .with_fn("padding_top", |style: &mut Self, value: Length| {
                style.clone().padding_top(value)
            })
            .with_fn("padding_start", |style: &mut Self, value: Length| {
                style.clone().padding_start(value)
            })
            .with_fn("padding_end", |style: &mut Self, value: Length| {
                style.clone().padding_end(value)
            })
            .with_fn("margin_start", |style: &mut Self, value: Length| {
                style.clone().margin_start(value)
            })
            .with_fn("margin_end", |style: &mut Self, value: Length| {
                style.clone().margin_end(value)
            })
            .with_fn("background", |style: &mut Self, value: ColorValue| {
                style.clone().background(value)
            })
            .with_fn("text_color", |style: &mut Self, value: ColorValue| {
                style.clone().text_color(value)
            })
            .with_fn("radius", |style: &mut Self, value: Length| {
                style.clone().radius(value)
            })
            .with_fn("border", |style: &mut Self, value: Length| {
                style.clone().border(value)
            })
            .with_fn("border_color", |style: &mut Self, value: ColorValue| {
                style.clone().border_color(value)
            })
            .with_fn("font_size", |style: &mut Self, value: Length| {
                style.clone().font_size(value)
            })
            .with_fn("flex_row", |style: &mut Self| style.clone().flex_row())
            .with_fn("flex_col", |style: &mut Self| style.clone().flex_col())
            .with_fn("items_center", |style: &mut Self| {
                style.clone().items_center()
            })
            .with_fn("items_start", |style: &mut Self| {
                style.clone().items_start()
            })
            .with_fn("items_end", |style: &mut Self| style.clone().items_end())
            .with_fn("justify_center", |style: &mut Self| {
                style.clone().justify_center()
            })
            .with_fn("justify_start", |style: &mut Self| {
                style.clone().justify_start()
            })
            .with_fn("justify_end", |style: &mut Self| {
                style.clone().justify_end()
            })
            .with_fn("justify_between", |style: &mut Self| {
                style.clone().justify_between()
            })
            .with_fn("clip", |style: &mut Self| style.clone().clip());
        register_overflow_methods(&mut builder);
        builder
            .with_fn("hover", |style: &mut Self, state: Self| {
                style.clone().hover(&state)
            })
            .with_fn("active", |style: &mut Self, state: Self| {
                style.clone().active(&state)
            })
            .with_fn("focus", |style: &mut Self, state: Self| {
                style.clone().focus(&state)
            })
            .with_fn("disabled", |style: &mut Self, state: Self| {
                style.clone().disabled(&state)
            })
            .with_fn("merge", |style: &mut Self, overlay: Self| {
                style.clone().merged(&overlay)
            });
    }
}

fn register_overflow_methods(builder: &mut TypeBuilder<Style>) {
    builder
        .with_fn("overflow_x_scroll", |style: &mut Style| {
            style.clone().overflow_x_scroll()
        })
        .with_fn("overflow_y_scroll", |style: &mut Style| {
            style.clone().overflow_y_scroll()
        })
        .with_fn("overflow_scroll", |style: &mut Style| {
            style.clone().overflow_scroll()
        })
        .with_fn("overflow_hidden", |style: &mut Style| {
            style.clone().overflow_hidden()
        });
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PseudoState {
    Hovered,
    Active,
    Focused,
    Disabled,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InteractionState {
    states: BTreeSet<PseudoState>,
}

impl InteractionState {
    #[must_use]
    pub fn with(mut self, state: PseudoState) -> Self {
        self.states.insert(state);
        self
    }

    #[must_use]
    pub fn contains(&self, state: PseudoState) -> bool {
        self.states.contains(&state)
    }
}

pub(crate) fn register_style_api(engine: &mut Engine) {
    engine.build_type::<Length>();
    engine.build_type::<ColorValue>();
    engine.build_type::<Style>();

    FuncRegistration::new("style")
        .in_global_namespace()
        .register_into_engine(engine, Style::new);
    register_length_constructor(engine, "px", Length::pixels);
    register_length_constructor(engine, "rem", Length::rems);
    register_length_constructor(engine, "relative", Length::relative);
    FuncRegistration::new("theme_spacing")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |token: ImmutableString| -> Result<Length, Box<EvalAltResult>> {
                Length::theme_spacing(token.as_str())
                    .map_err(|error| Box::new(style_runtime_error(error.to_string())))
            },
        );
    FuncRegistration::new("theme_radius")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |token: ImmutableString| -> Result<Length, Box<EvalAltResult>> {
                Length::theme_radius(token.as_str())
                    .map_err(|error| Box::new(style_runtime_error(error.to_string())))
            },
        );
    FuncRegistration::new("rgb")
        .in_global_namespace()
        .register_into_engine(engine, rgb_color);
    FuncRegistration::new("rgba")
        .in_global_namespace()
        .register_into_engine(engine, rgba_color);
    FuncRegistration::new("theme_color")
        .in_global_namespace()
        .register_into_engine(engine, |token: String| ColorValue::Token(token));
    FuncRegistration::new("component_style")
        .in_global_namespace()
        .register_into_engine(engine, component_style);
}

fn component_style(
    mut props: Map,
    part: ImmutableString,
    mut base: Style,
) -> Result<Style, Box<EvalAltResult>> {
    let part: String = part.into();
    if part == "root"
        && let Some(style) = props.remove("style")
    {
        if !style.is::<Style>() {
            return Err(Box::new(style_runtime_error(
                "component style must be a Style".to_owned(),
            )));
        }
        base = base.merged(&style.cast::<Style>());
    }
    if let Some(part_styles) = props.remove("part_styles") {
        if !part_styles.is::<Map>() {
            return Err(Box::new(style_runtime_error(
                "component part_styles must be a map of Style values".to_owned(),
            )));
        }
        let mut part_styles = part_styles.cast::<Map>();
        if let Some(style) = part_styles.remove(part.as_str()) {
            if !style.is::<Style>() {
                return Err(Box::new(style_runtime_error(format!(
                    "component part style `{part}` must be a Style"
                ))));
            }
            base = base.merged(&style.cast::<Style>());
        }
    }
    Ok(base)
}

fn register_length_constructor(
    engine: &mut Engine,
    name: &str,
    constructor: fn(f64) -> Result<Length, LengthError>,
) {
    FuncRegistration::new(name)
        .in_global_namespace()
        .register_into_engine(
            engine,
            move |value: FLOAT| -> Result<Length, Box<EvalAltResult>> {
                constructor(value).map_err(|error| Box::new(style_runtime_error(error.to_string())))
            },
        );
    FuncRegistration::new(name)
        .in_global_namespace()
        .register_into_engine(
            engine,
            move |value: INT| -> Result<Length, Box<EvalAltResult>> {
                constructor(numeric_to_f64(&value)?)
                    .map_err(|error| Box::new(style_runtime_error(error.to_string())))
            },
        );
}

fn numeric_to_f64(value: &impl ToString) -> Result<f64, Box<EvalAltResult>> {
    value.to_string().parse::<f64>().map_err(|_| {
        Box::new(style_runtime_error(
            "numeric length is outside the supported range".to_owned(),
        ))
    })
}

fn rgb_color(value: INT) -> Result<ColorValue, Box<EvalAltResult>> {
    let value = u32::try_from(value)
        .ok()
        .filter(|value| *value <= 0x00ff_ffff)
        .ok_or_else(|| {
            Box::new(style_runtime_error(
                "rgb value must be between 0 and 0xffffff".to_owned(),
            ))
        })?;
    Ok(ColorValue::Literal(Rgba8::from_rgb_hex(value)))
}

fn rgba_color(value: INT) -> Result<ColorValue, Box<EvalAltResult>> {
    let value = u32::try_from(value).map_err(|_| {
        Box::new(style_runtime_error(
            "rgba value must be between 0 and 0xffffffff".to_owned(),
        ))
    })?;
    Ok(ColorValue::Literal(Rgba8::from_rgba_hex(value)))
}

fn style_runtime_error(error: String) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(error.into(), Position::NONE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caller_overlay_wins_deterministically() {
        let base = Style::new()
            .width(Length::pixels(100.0).unwrap())
            .background(ColorValue::Literal(Rgba8::from_rgb_hex(0x0011_1111)));
        let caller = Style::new()
            .width(Length::pixels(240.0).unwrap())
            .text_color(ColorValue::Literal(Rgba8::from_rgb_hex(0x00ff_ffff)));
        let merged = base.merged(&caller);

        assert_eq!(merged.base.width, Some(Length::Pixels(240.0)));
        assert_eq!(
            merged.base.background,
            Some(ColorValue::Literal(Rgba8::from_rgb_hex(0x0011_1111)))
        );
        assert_eq!(
            merged.base.text_color,
            Some(ColorValue::Literal(Rgba8::from_rgb_hex(0x00ff_ffff)))
        );
    }

    #[test]
    fn disabled_pseudo_state_has_final_precedence() {
        let style = Style::new()
            .background(ColorValue::Token("button".to_owned()))
            .hover(&Style::new().background(ColorValue::Token("button_hover".to_owned())))
            .disabled(&Style::new().background(ColorValue::Token("button_disabled".to_owned())));
        let resolved = style.resolve(
            &InteractionState::default()
                .with(PseudoState::Hovered)
                .with(PseudoState::Disabled),
        );
        assert_eq!(
            resolved.background,
            Some(ColorValue::Token("button_disabled".to_owned()))
        );
    }

    #[test]
    fn script_builds_typed_styles() {
        let mut engine = Engine::new();
        register_style_api(&mut engine);
        let style: Style = engine
            .eval(
                r#"
                    style()
                        .flex_col()
                        .gap(theme_spacing("sm"))
                        .padding_x(rem(1.5))
                        .radius(theme_radius("md"))
                        .background(rgb(0x112233))
                        .hover(style().background(theme_color("surface_hover")))
                "#,
            )
            .unwrap();
        assert_eq!(style.base.direction, Some(FlexDirection::Column));
        assert_eq!(style.base.gap, Some(Length::ThemeSpacing(SpacingToken::Sm)));
        assert_eq!(
            style.base.radius,
            Some(Length::ThemeRadius(RadiusToken::Md))
        );
        assert!(style.hover.is_some());
    }

    #[test]
    fn invalid_lengths_are_rejected() {
        assert_eq!(
            Length::relative(1.5),
            Err(LengthError::InvalidRelative(1.5))
        );
        assert!(Length::pixels(f64::NAN).is_err());
        assert!(Length::theme_spacing("xxl").is_err());
    }
}
