use std::collections::{BTreeMap, BTreeSet};

use rhai::{
    Array, CustomType, Dynamic, Engine, EvalAltResult, FLOAT, FuncRegistration, INT,
    ImmutableString, Map, Position, TypeBuilder,
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

impl ColorValue {
    /// Parse a strict, bounded color literal shared by Style and Canvas.
    ///
    /// Supported forms are `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, the CSS
    /// basic named colors plus `transparent`, comma-separated `rgb/rgba`, and
    /// `hsl/hsla` with percentage saturation/lightness.
    ///
    /// # Errors
    ///
    /// Returns [`ColorParseError`] for malformed syntax or out-of-range channels.
    pub fn parse(value: &str) -> Result<Self, ColorParseError> {
        let value = value.trim().to_ascii_lowercase();
        if value.is_empty() || value.len() > 256 {
            return Err(ColorParseError::Invalid(value));
        }
        if let Some(color) = parse_named_color(&value) {
            return Ok(Self::Literal(color));
        }
        if let Some(hex) = value.strip_prefix('#') {
            return parse_hex_color(hex).map(Self::Literal);
        }
        if value.starts_with("rgb(") || value.starts_with("rgba(") {
            return parse_rgb_function(&value).map(Self::Literal);
        }
        if value.starts_with("hsl(") || value.starts_with("hsla(") {
            return parse_hsl_function(&value).map(Self::Literal);
        }
        Err(ColorParseError::Invalid(value))
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ColorParseError {
    #[error("invalid or unsupported color literal `{0}`")]
    Invalid(String),
    #[error("color channel `{0}` is outside its allowed range")]
    Channel(String),
}

fn parse_named_color(value: &str) -> Option<Rgba8> {
    let rgba = match value {
        "transparent" => 0x0000_0000,
        "black" => 0x0000_00ff,
        "silver" => 0xc0c0_c0ff,
        "gray" | "grey" => 0x8080_80ff,
        "white" => 0xffff_ffff,
        "maroon" => 0x8000_00ff,
        "red" => 0xff00_00ff,
        "purple" => 0x8000_80ff,
        "fuchsia" | "magenta" => 0xff00_ffff,
        "green" => 0x0080_00ff,
        "lime" => 0x00ff_00ff,
        "olive" => 0x8080_00ff,
        "yellow" => 0xffff_00ff,
        "navy" => 0x0000_80ff,
        "blue" => 0x0000_ffff,
        "teal" => 0x0080_80ff,
        "aqua" | "cyan" => 0x00ff_ffff,
        _ => return None,
    };
    Some(Rgba8::from_rgba_hex(rgba))
}

fn parse_hex_color(value: &str) -> Result<Rgba8, ColorParseError> {
    let expanded = match value.len() {
        3 | 4 => value
            .chars()
            .flat_map(|character| [character, character])
            .collect::<String>(),
        6 | 8 => value.to_owned(),
        _ => return Err(ColorParseError::Invalid(format!("#{value}"))),
    };
    let parsed = u32::from_str_radix(&expanded, 16)
        .map_err(|_| ColorParseError::Invalid(format!("#{value}")))?;
    Ok(if expanded.len() == 6 {
        Rgba8::from_rgb_hex(parsed)
    } else {
        Rgba8::from_rgba_hex(parsed)
    })
}

fn function_parts<'a>(value: &'a str, name: &str) -> Result<Vec<&'a str>, ColorParseError> {
    let body = value
        .strip_prefix(name)
        .and_then(|value| value.strip_prefix('('))
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| ColorParseError::Invalid(value.to_owned()))?;
    let parts = body.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.iter().any(|part| part.is_empty()) {
        Err(ColorParseError::Invalid(value.to_owned()))
    } else {
        Ok(parts)
    }
}

fn parse_rgb_function(value: &str) -> Result<Rgba8, ColorParseError> {
    let (name, expected) = if value.starts_with("rgba(") {
        ("rgba", 4)
    } else {
        ("rgb", 3)
    };
    let parts = function_parts(value, name)?;
    if parts.len() != expected {
        return Err(ColorParseError::Invalid(value.to_owned()));
    }
    let red = parse_byte(parts[0])?;
    let green = parse_byte(parts[1])?;
    let blue = parse_byte(parts[2])?;
    let alpha = if expected == 4 {
        parse_alpha(parts[3])?
    } else {
        u8::MAX
    };
    Ok(rgba_channels(red, green, blue, alpha))
}

fn parse_hsl_function(value: &str) -> Result<Rgba8, ColorParseError> {
    let (name, expected) = if value.starts_with("hsla(") {
        ("hsla", 4)
    } else {
        ("hsl", 3)
    };
    let parts = function_parts(value, name)?;
    if parts.len() != expected {
        return Err(ColorParseError::Invalid(value.to_owned()));
    }
    let hue = parse_finite(parts[0])?.rem_euclid(360.0) / 360.0;
    let saturation = parse_percent(parts[1])?;
    let lightness = parse_percent(parts[2])?;
    let alpha = if expected == 4 {
        parse_alpha(parts[3])?
    } else {
        u8::MAX
    };
    let (red, green, blue) = hsl_to_rgb(hue, saturation, lightness);
    Ok(rgba_channels(red, green, blue, alpha))
}

fn parse_byte(value: &str) -> Result<u8, ColorParseError> {
    value
        .parse::<u16>()
        .ok()
        .and_then(|value| u8::try_from(value).ok())
        .ok_or_else(|| ColorParseError::Channel(value.to_owned()))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn parse_alpha(value: &str) -> Result<u8, ColorParseError> {
    let alpha = if value.ends_with('%') {
        parse_percent(value)?
    } else {
        let value = parse_finite(value)?;
        if !(0.0..=1.0).contains(&value) {
            return Err(ColorParseError::Channel(value.to_string()));
        }
        value
    };
    Ok((alpha * 255.0).round() as u8)
}

fn parse_percent(value: &str) -> Result<f64, ColorParseError> {
    let number = value
        .strip_suffix('%')
        .ok_or_else(|| ColorParseError::Channel(value.to_owned()))?;
    let number = parse_finite(number)?;
    if (0.0..=100.0).contains(&number) {
        Ok(number / 100.0)
    } else {
        Err(ColorParseError::Channel(value.to_owned()))
    }
}

fn parse_finite(value: &str) -> Result<f64, ColorParseError> {
    value
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(|| ColorParseError::Channel(value.to_owned()))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn hsl_to_rgb(hue: f64, saturation: f64, lightness: f64) -> (u8, u8, u8) {
    let channel = |offset: f64| {
        let k = (offset + hue * 12.0).rem_euclid(12.0);
        let a = saturation * lightness.min(1.0 - lightness);
        let value = lightness - a * (-1.0_f64).max((k - 3.0).min(9.0 - k).min(1.0));
        (value * 255.0).round() as u8
    };
    (channel(0.0), channel(8.0), channel(4.0))
}

fn rgba_channels(red: u8, green: u8, blue: u8, alpha: u8) -> Rgba8 {
    Rgba8::from_rgba_hex(
        u32::from(red) << 24 | u32::from(green) << 16 | u32::from(blue) << 8 | u32::from(alpha),
    )
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
pub enum PositionMode {
    Relative,
    Absolute,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayMode {
    Block,
    Flex,
    Grid,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlexWrapMode {
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlignMode {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhiteSpaceMode {
    Normal,
    NoWrap,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontSlant {
    Normal,
    Italic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorKind {
    Default,
    Pointer,
    Text,
    Move,
    Crosshair,
    NotAllowed,
    ResizeHorizontal,
    ResizeVertical,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShadowSpec {
    pub x: f64,
    pub y: f64,
    pub blur: f64,
    pub spread: f64,
    pub color: ColorValue,
}

impl ShadowSpec {
    /// Create one validated box shadow.
    ///
    /// # Errors
    ///
    /// Returns [`StyleValueError`] for non-finite offsets or negative radii.
    pub fn new(
        x: f64,
        y: f64,
        blur: f64,
        spread: f64,
        color: ColorValue,
    ) -> Result<Self, StyleValueError> {
        if !x.is_finite()
            || !y.is_finite()
            || x.abs() > f64::from(f32::MAX)
            || y.abs() > f64::from(f32::MAX)
            || !blur.is_finite()
            || blur < 0.0
            || blur > f64::from(f32::MAX)
            || !spread.is_finite()
            || spread < 0.0
            || spread > f64::from(f32::MAX)
        {
            return Err(StyleValueError::InvalidShadow);
        }
        Ok(Self {
            x,
            y,
            blur,
            spread,
            color,
        })
    }
}

impl CustomType for ShadowSpec {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("ShadowSpec");
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinearGradientSpec {
    pub angle_degrees: f64,
    pub from: ColorValue,
    pub to: ColorValue,
}

impl LinearGradientSpec {
    /// Create a two-stop gradient with a normalized finite angle.
    ///
    /// # Errors
    ///
    /// Returns [`StyleValueError`] for a non-finite angle.
    pub fn new(
        angle_degrees: f64,
        from: ColorValue,
        to: ColorValue,
    ) -> Result<Self, StyleValueError> {
        if !angle_degrees.is_finite() {
            return Err(StyleValueError::InvalidGradientAngle);
        }
        Ok(Self {
            angle_degrees: angle_degrees.rem_euclid(360.0),
            from,
            to,
        })
    }
}

impl CustomType for LinearGradientSpec {
    fn build(mut builder: TypeBuilder<Self>) {
        builder.with_name("LinearGradientSpec");
    }
}

#[derive(Clone, Debug, Error, PartialEq)]
pub enum StyleValueError {
    #[error("shadow offsets must be finite and blur/spread must be finite and non-negative")]
    InvalidShadow,
    #[error("gradient angle must be finite")]
    InvalidGradientAngle,
    #[error("opacity must be finite and between zero and one, got {0}")]
    InvalidOpacity(f64),
    #[error("translation must be finite, got {0}")]
    InvalidTranslation(f64),
    #[error("font weight must be between 1 and 1000, got {0}")]
    InvalidFontWeight(i64),
    #[error("font family must be a non-empty string no longer than 256 bytes")]
    InvalidFontFamily,
    #[error("font fallback list must contain 1-16 unique non-empty family names")]
    InvalidFontFallbacks,
    #[error("OpenType feature tag must be four ASCII alphanumeric characters")]
    InvalidFontFeatureTag,
    #[error("OpenType feature value must be between 0 and 65535, got {0}")]
    InvalidFontFeatureValue(i64),
    #[error("line clamp must be between 1 and 10000, got {0}")]
    InvalidLineClamp(i64),
    #[error("grid count/span must be between 1 and 1024, got {0}")]
    InvalidGridValue(i64),
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
    pub display: Option<DisplayMode>,
    pub direction: Option<FlexDirection>,
    pub flex_wrap: Option<FlexWrapMode>,
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
    pub flex_shrink: Option<bool>,
    pub flex_basis: Option<Length>,
    pub grid_columns: Option<u16>,
    pub grid_rows: Option<u16>,
    pub column_span: Option<u16>,
    pub row_span: Option<u16>,
    pub clip: Option<bool>,
    pub overflow_x: Option<OverflowMode>,
    pub overflow_y: Option<OverflowMode>,
    pub position: Option<PositionMode>,
    pub top: Option<Length>,
    pub right: Option<Length>,
    pub bottom: Option<Length>,
    pub left: Option<Length>,
    pub opacity: Option<f64>,
    pub visible: Option<bool>,
    pub cursor: Option<CursorKind>,
    pub font_family: Option<String>,
    pub font_fallbacks: Option<Vec<String>>,
    pub font_features: Option<BTreeMap<String, u32>>,
    pub font_weight: Option<u16>,
    pub font_slant: Option<FontSlant>,
    pub line_height: Option<Length>,
    pub text_align: Option<TextAlignMode>,
    pub white_space: Option<WhiteSpaceMode>,
    pub text_ellipsis: Option<bool>,
    pub line_clamp: Option<usize>,
    pub shadows: Option<Vec<ShadowSpec>>,
    pub gradient: Option<LinearGradientSpec>,
    pub translate_x: Option<f64>,
    pub translate_y: Option<f64>,
}

impl StyleProperties {
    fn merge(&mut self, overlay: &Self) {
        merge_option(&mut self.display, overlay.display);
        merge_option(&mut self.direction, overlay.direction);
        merge_option(&mut self.flex_wrap, overlay.flex_wrap);
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
        merge_option(&mut self.flex_shrink, overlay.flex_shrink);
        merge_option(&mut self.flex_basis, overlay.flex_basis);
        merge_option(&mut self.grid_columns, overlay.grid_columns);
        merge_option(&mut self.grid_rows, overlay.grid_rows);
        merge_option(&mut self.column_span, overlay.column_span);
        merge_option(&mut self.row_span, overlay.row_span);
        merge_option(&mut self.clip, overlay.clip);
        merge_option(&mut self.overflow_x, overlay.overflow_x);
        merge_option(&mut self.overflow_y, overlay.overflow_y);
        merge_option(&mut self.position, overlay.position);
        merge_option(&mut self.top, overlay.top);
        merge_option(&mut self.right, overlay.right);
        merge_option(&mut self.bottom, overlay.bottom);
        merge_option(&mut self.left, overlay.left);
        merge_option(&mut self.opacity, overlay.opacity);
        merge_option(&mut self.visible, overlay.visible);
        merge_option(&mut self.cursor, overlay.cursor);
        merge_option(&mut self.font_family, overlay.font_family.clone());
        merge_option(&mut self.font_fallbacks, overlay.font_fallbacks.clone());
        merge_option(&mut self.font_features, overlay.font_features.clone());
        merge_option(&mut self.font_weight, overlay.font_weight);
        merge_option(&mut self.font_slant, overlay.font_slant);
        merge_option(&mut self.line_height, overlay.line_height);
        merge_option(&mut self.text_align, overlay.text_align);
        merge_option(&mut self.white_space, overlay.white_space);
        merge_option(&mut self.text_ellipsis, overlay.text_ellipsis);
        merge_option(&mut self.line_clamp, overlay.line_clamp);
        merge_option(&mut self.shadows, overlay.shadows.clone());
        merge_option(&mut self.gradient, overlay.gradient.clone());
        merge_option(&mut self.translate_x, overlay.translate_x);
        merge_option(&mut self.translate_y, overlay.translate_y);
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
    pub fn min_width(mut self, value: Length) -> Self {
        self.base.min_width = Some(value);
        self
    }

    #[must_use]
    pub fn min_height(mut self, value: Length) -> Self {
        self.base.min_height = Some(value);
        self
    }

    #[must_use]
    pub fn max_height(mut self, value: Length) -> Self {
        self.base.max_height = Some(value);
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
    pub fn padding_right(mut self, value: Length) -> Self {
        self.base.padding.right = Some(value);
        self
    }

    #[must_use]
    pub fn padding_bottom(mut self, value: Length) -> Self {
        self.base.padding.bottom = Some(value);
        self
    }

    #[must_use]
    pub fn padding_left(mut self, value: Length) -> Self {
        self.base.padding.left = Some(value);
        self
    }

    #[must_use]
    pub fn margin(mut self, value: Length) -> Self {
        self.base.margin = EdgeLengths::all(value);
        self
    }

    #[must_use]
    pub fn margin_x(mut self, value: Length) -> Self {
        self.base.margin.merge(&EdgeLengths::horizontal(value));
        self
    }

    #[must_use]
    pub fn margin_y(mut self, value: Length) -> Self {
        self.base.margin.merge(&EdgeLengths::vertical(value));
        self
    }

    #[must_use]
    pub fn margin_top(mut self, value: Length) -> Self {
        self.base.margin.top = Some(value);
        self
    }

    #[must_use]
    pub fn margin_right(mut self, value: Length) -> Self {
        self.base.margin.right = Some(value);
        self
    }

    #[must_use]
    pub fn margin_bottom(mut self, value: Length) -> Self {
        self.base.margin.bottom = Some(value);
        self
    }

    #[must_use]
    pub fn margin_left(mut self, value: Length) -> Self {
        self.base.margin.left = Some(value);
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
        self.base.gradient = None;
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
    pub fn block(mut self) -> Self {
        self.base.display = Some(DisplayMode::Block);
        self
    }

    #[must_use]
    pub fn grid(mut self) -> Self {
        self.base.display = Some(DisplayMode::Grid);
        self
    }

    #[must_use]
    pub fn hidden(mut self) -> Self {
        self.base.display = Some(DisplayMode::None);
        self
    }

    #[must_use]
    pub fn flex_row(mut self) -> Self {
        self.base.display = Some(DisplayMode::Flex);
        self.base.direction = Some(FlexDirection::Row);
        self
    }

    #[must_use]
    pub fn flex_col(mut self) -> Self {
        self.base.display = Some(DisplayMode::Flex);
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
    pub fn justify_around(mut self) -> Self {
        self.base.justify = Some(Justify::Around);
        self
    }

    #[must_use]
    pub fn flex_grow(mut self) -> Self {
        self.base.flex_grow = Some(true);
        self
    }

    #[must_use]
    pub fn flex_shrink(mut self, shrink: bool) -> Self {
        self.base.flex_shrink = Some(shrink);
        self
    }

    #[must_use]
    pub fn flex_basis(mut self, basis: Length) -> Self {
        self.base.flex_basis = Some(basis);
        self
    }

    #[must_use]
    pub fn flex_wrap(mut self, mode: FlexWrapMode) -> Self {
        self.base.flex_wrap = Some(mode);
        self
    }

    /// Set a bounded explicit grid column count.
    ///
    /// # Errors
    ///
    /// Returns [`StyleValueError`] outside 1..=1024.
    pub fn grid_columns(mut self, columns: i64) -> Result<Self, StyleValueError> {
        self.base.grid_columns = Some(grid_value(columns)?);
        self.base.display = Some(DisplayMode::Grid);
        Ok(self)
    }

    /// Set a bounded explicit grid row count.
    ///
    /// # Errors
    ///
    /// Returns [`StyleValueError`] outside 1..=1024.
    pub fn grid_rows(mut self, rows: i64) -> Result<Self, StyleValueError> {
        self.base.grid_rows = Some(grid_value(rows)?);
        self.base.display = Some(DisplayMode::Grid);
        Ok(self)
    }

    /// Set a bounded grid column span.
    ///
    /// # Errors
    ///
    /// Returns [`StyleValueError`] outside 1..=1024.
    pub fn column_span(mut self, span: i64) -> Result<Self, StyleValueError> {
        self.base.column_span = Some(grid_value(span)?);
        Ok(self)
    }

    /// Set a bounded grid row span.
    ///
    /// # Errors
    ///
    /// Returns [`StyleValueError`] outside 1..=1024.
    pub fn row_span(mut self, span: i64) -> Result<Self, StyleValueError> {
        self.base.row_span = Some(grid_value(span)?);
        Ok(self)
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
    pub fn relative(mut self) -> Self {
        self.base.position = Some(PositionMode::Relative);
        self
    }

    #[must_use]
    pub fn absolute(mut self) -> Self {
        self.base.position = Some(PositionMode::Absolute);
        self
    }

    #[must_use]
    pub fn top(mut self, value: Length) -> Self {
        self.base.top = Some(value);
        self
    }

    #[must_use]
    pub fn right(mut self, value: Length) -> Self {
        self.base.right = Some(value);
        self
    }

    #[must_use]
    pub fn bottom(mut self, value: Length) -> Self {
        self.base.bottom = Some(value);
        self
    }

    #[must_use]
    pub fn left(mut self, value: Length) -> Self {
        self.base.left = Some(value);
        self
    }

    /// Set finite unit opacity.
    ///
    /// # Errors
    ///
    /// Returns [`StyleValueError`] outside zero through one.
    pub fn opacity(mut self, value: f64) -> Result<Self, StyleValueError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(StyleValueError::InvalidOpacity(value));
        }
        self.base.opacity = Some(value);
        Ok(self)
    }

    #[must_use]
    pub fn visible(mut self, visible: bool) -> Self {
        self.base.visible = Some(visible);
        self
    }

    #[must_use]
    pub fn cursor(mut self, cursor: CursorKind) -> Self {
        self.base.cursor = Some(cursor);
        self
    }

    /// Set one validated platform font family name.
    ///
    /// # Errors
    ///
    /// Returns [`StyleValueError`] for an empty or oversized name.
    pub fn font_family(mut self, family: impl Into<String>) -> Result<Self, StyleValueError> {
        let family = family.into();
        if family.trim().is_empty() || family.len() > 256 {
            return Err(StyleValueError::InvalidFontFamily);
        }
        self.base.font_family = Some(family);
        Ok(self)
    }

    /// Set an ordered font fallback stack.
    ///
    /// # Errors
    ///
    /// Returns [`StyleValueError::InvalidFontFallbacks`] for empty, duplicate,
    /// oversized, or invalid family names.
    pub fn font_fallbacks(mut self, families: Vec<String>) -> Result<Self, StyleValueError> {
        let unique = families.iter().collect::<BTreeSet<_>>();
        if families.is_empty()
            || families.len() > 16
            || unique.len() != families.len()
            || families
                .iter()
                .any(|family| family.trim().is_empty() || family.len() > 256)
        {
            return Err(StyleValueError::InvalidFontFallbacks);
        }
        self.base.font_fallbacks = Some(families);
        Ok(self)
    }

    /// Set one OpenType feature tag value.
    ///
    /// # Errors
    ///
    /// Returns for invalid four-character tags or values outside 0..=65535.
    pub fn font_feature(
        mut self,
        tag: impl Into<String>,
        value: i64,
    ) -> Result<Self, StyleValueError> {
        let tag = tag.into();
        if tag.len() != 4
            || !tag
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        {
            return Err(StyleValueError::InvalidFontFeatureTag);
        }
        let value = u32::try_from(value)
            .ok()
            .filter(|value| *value <= 65_535)
            .ok_or(StyleValueError::InvalidFontFeatureValue(value))?;
        self.base
            .font_features
            .get_or_insert_with(BTreeMap::new)
            .insert(tag, value);
        Ok(self)
    }

    /// Set a numeric font weight.
    ///
    /// # Errors
    ///
    /// Returns [`StyleValueError`] outside 1..=1000.
    pub fn font_weight(mut self, weight: i64) -> Result<Self, StyleValueError> {
        let weight = u16::try_from(weight)
            .ok()
            .filter(|weight| (1..=1_000).contains(weight))
            .ok_or(StyleValueError::InvalidFontWeight(weight))?;
        self.base.font_weight = Some(weight);
        Ok(self)
    }

    #[must_use]
    pub fn font_slant(mut self, slant: FontSlant) -> Self {
        self.base.font_slant = Some(slant);
        self
    }

    #[must_use]
    pub fn line_height(mut self, value: Length) -> Self {
        self.base.line_height = Some(value);
        self
    }

    #[must_use]
    pub fn text_align(mut self, align: TextAlignMode) -> Self {
        self.base.text_align = Some(align);
        self
    }

    #[must_use]
    pub fn white_space(mut self, white_space: WhiteSpaceMode) -> Self {
        self.base.white_space = Some(white_space);
        self
    }

    #[must_use]
    pub fn text_ellipsis(mut self) -> Self {
        self.base.text_ellipsis = Some(true);
        self
    }

    /// Clamp visible text to a bounded number of lines.
    ///
    /// # Errors
    ///
    /// Returns [`StyleValueError`] outside 1..=10000.
    pub fn line_clamp(mut self, lines: i64) -> Result<Self, StyleValueError> {
        let lines = usize::try_from(lines)
            .ok()
            .filter(|lines| (1..=10_000).contains(lines))
            .ok_or(StyleValueError::InvalidLineClamp(lines))?;
        self.base.line_clamp = Some(lines);
        Ok(self)
    }

    #[must_use]
    pub fn shadow(mut self, shadow: ShadowSpec) -> Self {
        self.base.shadows = Some(vec![shadow]);
        self
    }

    #[must_use]
    pub fn linear_gradient(mut self, gradient: LinearGradientSpec) -> Self {
        self.base.gradient = Some(gradient);
        self.base.background = None;
        self
    }

    /// Apply a finite paint translation along x.
    ///
    /// # Errors
    ///
    /// Returns [`StyleValueError`] for non-finite or unrepresentable values.
    pub fn translate_x(mut self, value: f64) -> Result<Self, StyleValueError> {
        validate_translation(value)?;
        self.base.translate_x = Some(value);
        Ok(self)
    }

    /// Apply a finite paint translation along y.
    ///
    /// # Errors
    ///
    /// Returns [`StyleValueError`] for non-finite or unrepresentable values.
    pub fn translate_y(mut self, value: f64) -> Result<Self, StyleValueError> {
        validate_translation(value)?;
        self.base.translate_y = Some(value);
        Ok(self)
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

fn grid_value(value: i64) -> Result<u16, StyleValueError> {
    u16::try_from(value)
        .ok()
        .filter(|value| (1..=1_024).contains(value))
        .ok_or(StyleValueError::InvalidGridValue(value))
}

fn validate_translation(value: f64) -> Result<(), StyleValueError> {
    if value.is_finite() && value.abs() <= f64::from(f32::MAX) {
        Ok(())
    } else {
        Err(StyleValueError::InvalidTranslation(value))
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
        register_position_methods(&mut builder);
        register_extended_layout_methods(&mut builder);
        register_visual_methods(&mut builder);
        register_text_methods(&mut builder);
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

fn register_position_methods(builder: &mut TypeBuilder<Style>) {
    builder
        .with_fn("relative", |style: &mut Style| style.clone().relative())
        .with_fn("absolute", |style: &mut Style| style.clone().absolute())
        .with_fn("top", |style: &mut Style, value: Length| {
            style.clone().top(value)
        })
        .with_fn("right", |style: &mut Style, value: Length| {
            style.clone().right(value)
        })
        .with_fn("bottom", |style: &mut Style, value: Length| {
            style.clone().bottom(value)
        })
        .with_fn("left", |style: &mut Style, value: Length| {
            style.clone().left(value)
        });
}

fn register_extended_layout_methods(builder: &mut TypeBuilder<Style>) {
    builder
        .with_fn("min_width", |style: &mut Style, value: Length| {
            style.clone().min_width(value)
        })
        .with_fn("min_height", |style: &mut Style, value: Length| {
            style.clone().min_height(value)
        })
        .with_fn("max_height", |style: &mut Style, value: Length| {
            style.clone().max_height(value)
        })
        .with_fn("padding_right", |style: &mut Style, value: Length| {
            style.clone().padding_right(value)
        })
        .with_fn("padding_bottom", |style: &mut Style, value: Length| {
            style.clone().padding_bottom(value)
        })
        .with_fn("padding_left", |style: &mut Style, value: Length| {
            style.clone().padding_left(value)
        })
        .with_fn("margin", |style: &mut Style, value: Length| {
            style.clone().margin(value)
        })
        .with_fn("margin_x", |style: &mut Style, value: Length| {
            style.clone().margin_x(value)
        })
        .with_fn("margin_y", |style: &mut Style, value: Length| {
            style.clone().margin_y(value)
        })
        .with_fn("margin_top", |style: &mut Style, value: Length| {
            style.clone().margin_top(value)
        })
        .with_fn("margin_right", |style: &mut Style, value: Length| {
            style.clone().margin_right(value)
        })
        .with_fn("margin_bottom", |style: &mut Style, value: Length| {
            style.clone().margin_bottom(value)
        })
        .with_fn("margin_left", |style: &mut Style, value: Length| {
            style.clone().margin_left(value)
        })
        .with_fn("block", |style: &mut Style| style.clone().block())
        .with_fn("grid", |style: &mut Style| style.clone().grid())
        .with_fn("hidden", |style: &mut Style| style.clone().hidden())
        .with_fn("justify_around", |style: &mut Style| {
            style.clone().justify_around()
        })
        .with_fn("flex_grow", |style: &mut Style| style.clone().flex_grow())
        .with_fn("flex_shrink", |style: &mut Style, value: bool| {
            style.clone().flex_shrink(value)
        })
        .with_fn("flex_basis", |style: &mut Style, value: Length| {
            style.clone().flex_basis(value)
        })
        .with_fn("flex_wrap", |style: &mut Style| {
            style.clone().flex_wrap(FlexWrapMode::Wrap)
        })
        .with_fn("flex_wrap_reverse", |style: &mut Style| {
            style.clone().flex_wrap(FlexWrapMode::WrapReverse)
        })
        .with_fn("flex_nowrap", |style: &mut Style| {
            style.clone().flex_wrap(FlexWrapMode::NoWrap)
        })
        .with_fn("grid_cols", |style: &mut Style, value: INT| {
            style
                .clone()
                .grid_columns(value)
                .map_err(|error| Box::new(style_runtime_error(error.to_string())))
        })
        .with_fn("grid_rows", |style: &mut Style, value: INT| {
            style
                .clone()
                .grid_rows(value)
                .map_err(|error| Box::new(style_runtime_error(error.to_string())))
        })
        .with_fn("col_span", |style: &mut Style, value: INT| {
            style
                .clone()
                .column_span(value)
                .map_err(|error| Box::new(style_runtime_error(error.to_string())))
        })
        .with_fn("row_span", |style: &mut Style, value: INT| {
            style
                .clone()
                .row_span(value)
                .map_err(|error| Box::new(style_runtime_error(error.to_string())))
        });
}

fn register_visual_methods(builder: &mut TypeBuilder<Style>) {
    builder
        .with_fn("opacity", |style: &mut Style, value: FLOAT| {
            style
                .clone()
                .opacity(value)
                .map_err(|error| Box::new(style_runtime_error(error.to_string())))
        })
        .with_fn("opacity", |style: &mut Style, value: INT| {
            style
                .clone()
                .opacity(numeric_to_f64(&value)?)
                .map_err(|error| Box::new(style_runtime_error(error.to_string())))
        })
        .with_fn("visible", |style: &mut Style| style.clone().visible(true))
        .with_fn("invisible", |style: &mut Style| {
            style.clone().visible(false)
        })
        .with_fn("cursor_default", |style: &mut Style| {
            style.clone().cursor(CursorKind::Default)
        })
        .with_fn("cursor_pointer", |style: &mut Style| {
            style.clone().cursor(CursorKind::Pointer)
        })
        .with_fn("cursor_text", |style: &mut Style| {
            style.clone().cursor(CursorKind::Text)
        })
        .with_fn("cursor_move", |style: &mut Style| {
            style.clone().cursor(CursorKind::Move)
        })
        .with_fn("cursor_crosshair", |style: &mut Style| {
            style.clone().cursor(CursorKind::Crosshair)
        })
        .with_fn("cursor_not_allowed", |style: &mut Style| {
            style.clone().cursor(CursorKind::NotAllowed)
        })
        .with_fn("cursor_resize_x", |style: &mut Style| {
            style.clone().cursor(CursorKind::ResizeHorizontal)
        })
        .with_fn("cursor_resize_y", |style: &mut Style| {
            style.clone().cursor(CursorKind::ResizeVertical)
        })
        .with_fn("shadow", |style: &mut Style, shadow: ShadowSpec| {
            style.clone().shadow(shadow)
        })
        .with_fn(
            "linear_gradient",
            |style: &mut Style, gradient: LinearGradientSpec| {
                style.clone().linear_gradient(gradient)
            },
        )
        .with_fn("translate_x", |style: &mut Style, value: FLOAT| {
            style
                .clone()
                .translate_x(value)
                .map_err(|error| Box::new(style_runtime_error(error.to_string())))
        })
        .with_fn("translate_x", |style: &mut Style, value: INT| {
            style
                .clone()
                .translate_x(numeric_to_f64(&value)?)
                .map_err(|error| Box::new(style_runtime_error(error.to_string())))
        })
        .with_fn("translate_y", |style: &mut Style, value: FLOAT| {
            style
                .clone()
                .translate_y(value)
                .map_err(|error| Box::new(style_runtime_error(error.to_string())))
        })
        .with_fn("translate_y", |style: &mut Style, value: INT| {
            style
                .clone()
                .translate_y(numeric_to_f64(&value)?)
                .map_err(|error| Box::new(style_runtime_error(error.to_string())))
        });
}

fn register_text_methods(builder: &mut TypeBuilder<Style>) {
    builder
        .with_fn(
            "font_family",
            |style: &mut Style, family: ImmutableString| {
                style
                    .clone()
                    .font_family(family.to_string())
                    .map_err(|error| Box::new(style_runtime_error(error.to_string())))
            },
        )
        .with_fn(
            "font_fallbacks",
            |style: &mut Style, families: Array| -> Result<Style, Box<EvalAltResult>> {
                let families = families
                    .into_iter()
                    .map(|family| {
                        family
                            .try_cast::<ImmutableString>()
                            .map(|family| family.to_string())
                    })
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        Box::new(style_runtime_error(
                            "font_fallbacks values must be strings".to_owned(),
                        ))
                    })?;
                style
                    .clone()
                    .font_fallbacks(families)
                    .map_err(|error| Box::new(style_runtime_error(error.to_string())))
            },
        )
        .with_fn(
            "font_feature",
            |style: &mut Style,
             tag: ImmutableString,
             value: INT|
             -> Result<Style, Box<EvalAltResult>> {
                style
                    .clone()
                    .font_feature(tag.to_string(), value)
                    .map_err(|error| Box::new(style_runtime_error(error.to_string())))
            },
        )
        .with_fn("font_weight", |style: &mut Style, weight: INT| {
            style
                .clone()
                .font_weight(weight)
                .map_err(|error| Box::new(style_runtime_error(error.to_string())))
        })
        .with_fn("italic", |style: &mut Style| {
            style.clone().font_slant(FontSlant::Italic)
        })
        .with_fn("not_italic", |style: &mut Style| {
            style.clone().font_slant(FontSlant::Normal)
        })
        .with_fn("line_height", |style: &mut Style, value: Length| {
            style.clone().line_height(value)
        })
        .with_fn("text_left", |style: &mut Style| {
            style.clone().text_align(TextAlignMode::Start)
        })
        .with_fn("text_center", |style: &mut Style| {
            style.clone().text_align(TextAlignMode::Center)
        })
        .with_fn("text_right", |style: &mut Style| {
            style.clone().text_align(TextAlignMode::End)
        })
        .with_fn("whitespace_normal", |style: &mut Style| {
            style.clone().white_space(WhiteSpaceMode::Normal)
        })
        .with_fn("whitespace_nowrap", |style: &mut Style| {
            style.clone().white_space(WhiteSpaceMode::NoWrap)
        })
        .with_fn("text_ellipsis", |style: &mut Style| {
            style.clone().text_ellipsis()
        })
        .with_fn("line_clamp", |style: &mut Style, lines: INT| {
            style
                .clone()
                .line_clamp(lines)
                .map_err(|error| Box::new(style_runtime_error(error.to_string())))
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
    engine.build_type::<ShadowSpec>();
    engine.build_type::<LinearGradientSpec>();
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
    FuncRegistration::new("shadow")
        .in_global_namespace()
        .register_into_engine(engine, shadow_from_map);
    FuncRegistration::new("linear_gradient")
        .in_global_namespace()
        .register_into_engine(engine, linear_gradient_from_map);
    FuncRegistration::new("theme_color")
        .in_global_namespace()
        .register_into_engine(engine, |token: String| ColorValue::Token(token));
    FuncRegistration::new("color")
        .in_global_namespace()
        .register_into_engine(
            engine,
            |value: ImmutableString| -> Result<ColorValue, Box<EvalAltResult>> {
                ColorValue::parse(value.as_str())
                    .map_err(|error| Box::new(style_runtime_error(error.to_string())))
            },
        );
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

fn shadow_from_map(mut value: Map) -> Result<ShadowSpec, Box<EvalAltResult>> {
    let x = optional_map_number(&mut value, "x")?.unwrap_or(0.0);
    let y = optional_map_number(&mut value, "y")?.unwrap_or(0.0);
    let blur = optional_map_number(&mut value, "blur")?.unwrap_or(0.0);
    let spread = optional_map_number(&mut value, "spread")?.unwrap_or(0.0);
    let color = value
        .remove("color")
        .and_then(Dynamic::try_cast::<ColorValue>)
        .ok_or_else(|| {
            Box::new(style_runtime_error(
                "shadow color must be a ColorValue".to_owned(),
            ))
        })?;
    reject_style_map_unknown(value, "shadow")?;
    ShadowSpec::new(x, y, blur, spread, color)
        .map_err(|error| Box::new(style_runtime_error(error.to_string())))
}

fn linear_gradient_from_map(mut value: Map) -> Result<LinearGradientSpec, Box<EvalAltResult>> {
    let angle = optional_map_number(&mut value, "angle")?.unwrap_or(180.0);
    let from = value
        .remove("from")
        .and_then(Dynamic::try_cast::<ColorValue>)
        .ok_or_else(|| {
            Box::new(style_runtime_error(
                "linear gradient from must be a ColorValue".to_owned(),
            ))
        })?;
    let to = value
        .remove("to")
        .and_then(Dynamic::try_cast::<ColorValue>)
        .ok_or_else(|| {
            Box::new(style_runtime_error(
                "linear gradient to must be a ColorValue".to_owned(),
            ))
        })?;
    reject_style_map_unknown(value, "linear gradient")?;
    LinearGradientSpec::new(angle, from, to)
        .map_err(|error| Box::new(style_runtime_error(error.to_string())))
}

fn optional_map_number(value: &mut Map, name: &str) -> Result<Option<f64>, Box<EvalAltResult>> {
    value
        .remove(name)
        .map(|value| {
            if value.is::<FLOAT>() {
                Ok(value.cast::<FLOAT>())
            } else if value.is::<INT>() {
                numeric_to_f64(&value.cast::<INT>())
            } else {
                Err(Box::new(style_runtime_error(format!(
                    "style field `{name}` must be a number"
                ))))
            }
        })
        .transpose()
}

fn reject_style_map_unknown(value: Map, context: &str) -> Result<(), Box<EvalAltResult>> {
    if let Some((name, _)) = value.into_iter().next() {
        Err(Box::new(style_runtime_error(format!(
            "unknown {context} field `{name}`"
        ))))
    } else {
        Ok(())
    }
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
    fn script_builds_extended_layout_paint_and_typography() {
        let mut engine = Engine::new();
        register_style_api(&mut engine);
        let style: Style = engine
            .eval(
                r#"
                    style()
                        .grid_cols(3).grid_rows(2).col_span(2)
                        .min_width(px(120)).max_height(px(480))
                        .margin_x(px(8)).padding_bottom(px(6))
                        .linear_gradient(linear_gradient(#{
                            angle: 135, from: rgb(0x112233), to: rgba(0x445566cc)
                        }))
                        .shadow(shadow(#{ x: 0, y: 8, blur: 24, spread: 2,
                            color: rgba(0x00000066) }))
                        .opacity(0.85).translate_x(-12)
                        .cursor_pointer().font_family("Avenir Next")
                        .font_fallbacks(["PingFang SC", "Noto Sans"])
                        .font_feature("liga", 0).font_feature("ss01", 1)
                        .font_weight(650).italic().line_height(px(24))
                        .text_center().whitespace_nowrap().text_ellipsis().line_clamp(2)
                "#,
            )
            .unwrap();
        assert_eq!(style.base.display, Some(DisplayMode::Grid));
        assert_eq!(style.base.grid_columns, Some(3));
        assert_eq!(style.base.column_span, Some(2));
        assert_eq!(style.base.opacity, Some(0.85));
        assert_eq!(style.base.translate_x, Some(-12.0));
        assert_eq!(style.base.cursor, Some(CursorKind::Pointer));
        assert_eq!(style.base.font_family.as_deref(), Some("Avenir Next"));
        assert_eq!(
            style.base.font_fallbacks.as_deref(),
            Some(["PingFang SC".to_owned(), "Noto Sans".to_owned()].as_slice())
        );
        assert_eq!(
            style.base.font_features,
            Some(BTreeMap::from([
                ("liga".to_owned(), 0),
                ("ss01".to_owned(), 1)
            ]))
        );
        assert_eq!(style.base.font_weight, Some(650));
        assert_eq!(style.base.line_clamp, Some(2));
        assert!(style.base.gradient.is_some());
        assert_eq!(style.base.shadows.as_ref().map(Vec::len), Some(1));
        assert!(
            Style::new()
                .font_fallbacks(vec!["Duplicate".to_owned(), "Duplicate".to_owned()])
                .is_err()
        );
        assert!(Style::new().font_feature("bad", 1).is_err());
    }

    #[test]
    fn strict_color_literals_share_one_typed_value_path() {
        assert_eq!(
            ColorValue::parse("#abc").unwrap(),
            ColorValue::Literal(Rgba8::from_rgba_hex(0xaabb_ccff))
        );
        assert_eq!(
            ColorValue::parse("rgba(255, 0, 0, 50%)").unwrap(),
            ColorValue::Literal(Rgba8::from_rgba_hex(0xff00_0080))
        );
        assert_eq!(
            ColorValue::parse("hsl(120, 100%, 50%)").unwrap(),
            ColorValue::Literal(Rgba8::from_rgba_hex(0x00ff_00ff))
        );
        assert_eq!(
            ColorValue::parse("transparent").unwrap(),
            ColorValue::Literal(Rgba8::from_rgba_hex(0))
        );
        assert!(ColorValue::parse("rgb(256, 0, 0)").is_err());
        assert!(ColorValue::parse("not-a-color").is_err());

        let mut engine = Engine::new();
        register_style_api(&mut engine);
        assert_eq!(
            engine
                .eval::<ColorValue>(r##"color("#336699cc")"##)
                .unwrap(),
            ColorValue::Literal(Rgba8::from_rgba_hex(0x3366_99cc))
        );
    }

    #[test]
    fn invalid_lengths_are_rejected() {
        assert_eq!(
            Length::relative(1.5),
            Err(LengthError::InvalidRelative(1.5))
        );
        assert!(Length::pixels(f64::NAN).is_err());
        assert!(Length::theme_spacing("xxl").is_err());
        assert!(Style::new().opacity(1.1).is_err());
        assert!(Style::new().translate_y(f64::NAN).is_err());
        assert!(Style::new().grid_columns(0).is_err());
        assert!(ShadowSpec::new(0.0, 1.0, -1.0, 0.0, ColorValue::Token("x".into())).is_err());
    }
}
