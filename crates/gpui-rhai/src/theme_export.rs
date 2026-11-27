//! Export effective themes through the ordinary, validated Rhai authoring API.
//!
//! This is intentionally not a second theme decoder: generated styles use the
//! same constructors as handwritten themes. Unsupported declarations fail
//! explicitly, and a round trip checks the entire effective token set.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::{ColorValue, Style, ThemeError, ThemeTokenValue, ThemeVariant};

type Result<T> = std::result::Result<T, ThemeError>;

fn error(message: impl Into<String>) -> ThemeError {
    ThemeError::Script(format!("theme export: {}", message.into()))
}

fn json(value: &impl Serialize) -> Result<Value> {
    serde_json::to_value(value).map_err(|e| error(e.to_string()))
}

fn quoted(value: &str) -> String {
    // JSON string escapes are also accepted by Rhai string literals.
    serde_json::to_string(value).expect("serializing a string cannot fail")
}

/// Export a complete effective theme as a self-contained `theme() -> map` source.
///
/// Preserves environment tables, typography aliases, motion, namespaced tokens,
/// component recipes and all pseudo states. Color expressions in styles remain
/// symbolic; already evaluated palette colors are exported as literal values.
/// Source comments, helper functions and original formatting are not retained.
///
/// # Errors
///
/// Returns an error for an invalid theme or a declaration that cannot be
/// represented losslessly by the public Rhai constructors. Never silently
/// downgrades a complete preset to a palette.
pub fn export_theme_source(theme: &ThemeVariant) -> Result<String> {
    theme.validate()?;
    let tokens = &theme.tokens;
    let mut fields = BTreeMap::new();
    fields.insert(
        "colors".to_owned(),
        map_expr(
            tokens
                .colors
                .iter()
                .map(|(k, c)| Ok((k.clone(), format!("0x{:08x}", c.as_rgba_hex())))),
        )?,
    );
    fields.insert("spacing".to_owned(), variable_map(&tokens.spacing)?);
    fields.insert("radii".to_owned(), variable_map(&tokens.radii)?);
    fields.insert("typography".to_owned(), expr(&json(&tokens.typography)?)?);
    fields.insert("motion".to_owned(), expr(&json(&tokens.motion)?)?);
    fields.insert("environment".to_owned(), expr(&json(&tokens.environment)?)?);
    fields.insert(
        "namespaces".to_owned(),
        map_expr(tokens.namespaces.iter().map(|(namespace, values)| {
            Ok((
                namespace.clone(),
                map_expr(values.iter().map(|(name, value)| {
                    let value = match value {
                        ThemeTokenValue::Color(c) => format!("0x{:08x}", c.as_rgba_hex()),
                        ThemeTokenValue::Length(v) => expr(&json(v)?)?,
                        ThemeTokenValue::Number(v) => match v {
                            crate::token::Variable::Fixed(n) => {
                                format!("#{{ type: \"number\", value: {n:?} }}")
                            }
                            crate::token::Variable::ByEnv(_) => {
                                format!("#{{ type: \"number\", value: {} }}", expr(&json(v)?)?)
                            }
                        },
                        ThemeTokenValue::String(v) => expr(&json(v)?)?,
                        ThemeTokenValue::Style(v) => variable_style_expr(v)?,
                    };
                    Ok((name.clone(), value))
                }))?,
            ))
        }))?,
    );
    if tokens.component_styles_replace {
        fields.insert("component_styles_replace".to_owned(), "true".to_owned());
    }
    fields.insert(
        "component_styles".to_owned(),
        map_expr(tokens.component_styles.iter().map(|(id, parts)| {
            Ok((
                id.to_string(),
                map_expr(
                    parts
                        .iter()
                        .map(|(part, style)| Ok((part.clone(), variable_style_expr(style)?))),
                )?,
            ))
        }))?,
    );
    let tokens = render_map(fields);
    let source = format!(
        "fn theme() {{\n    #{{\n        family: {},\n        name: {},\n        mode: {},\n        tokens: {tokens},\n    }}\n}}\n",
        quoted(&theme.family),
        quoted(&theme.name),
        quoted(match theme.mode {
            crate::ThemeMode::Light => "light",
            crate::ThemeMode::Dark => "dark",
        })
    );
    let engine = crate::RuntimeEngine::new();
    let decoded = crate::load_theme_source(engine.engine(), "<exported-theme>", &source)?;
    if decoded != *theme {
        return Err(error(
            "public constructors cannot represent the complete theme losslessly",
        ));
    }
    Ok(source)
}

fn variable_map(values: &BTreeMap<String, crate::ThemeLength>) -> Result<String> {
    map_expr(
        values
            .iter()
            .map(|(k, v)| Ok((k.clone(), expr(&json(v)?)?))),
    )
}

fn object(value: &Value) -> Result<&Map<String, Value>> {
    value.as_object().ok_or_else(|| error("expected an object"))
}

fn render_map(values: BTreeMap<String, String>) -> String {
    format!(
        "#{{\n{}\n}}",
        values
            .into_iter()
            .map(|(k, v)| format!("{}: {v},", quoted(&k)))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn map_expr(values: impl Iterator<Item = Result<(String, String)>>) -> Result<String> {
    Ok(render_map(values.collect::<Result<BTreeMap<_, _>>>()?))
}

fn expr(value: &Value) -> Result<String> {
    match value {
        Value::Null => Ok("()".to_owned()),
        Value::Bool(_) | Value::Number(_) => Ok(value.to_string()),
        Value::String(v) => Ok(quoted(v)),
        Value::Array(v) => Ok(format!(
            "[{}]",
            v.iter().map(expr).collect::<Result<Vec<_>>>()?.join(", ")
        )),
        Value::Object(v) if v.contains_key("unit") => length(value),
        Value::Object(v) if v.contains_key("keys") && v.contains_key("entries") => {
            let mut root = Map::new();
            for (path, leaf) in object(&v["entries"])? {
                let mut branch = &mut root;
                let mut parts = path.split('.').peekable();
                while let Some(part) = parts.next() {
                    if parts.peek().is_none() {
                        branch.insert(part.to_owned(), leaf.clone());
                        break;
                    }
                    branch = branch
                        .entry(part.to_owned())
                        .or_insert_with(|| Value::Object(Map::new()))
                        .as_object_mut()
                        .ok_or_else(|| error("environment path collision"))?;
                }
            }
            Ok(format!(
                "by_env({}, {})",
                expr(&v["keys"])?,
                expr(&Value::Object(root))?
            ))
        }
        Value::Object(v) => map_expr(v.iter().map(|(k, v)| Ok((k.clone(), expr(v)?)))),
    }
}

fn length(value: &Value) -> Result<String> {
    let v = object(value)?;
    let inner = &v["value"];
    Ok(match v["unit"].as_str() {
        Some("pixels") => format!("px({inner})"),
        Some("rems") => format!("rem({inner})"),
        Some("relative") => format!("relative({inner})"),
        Some("token") => format!(
            "theme_length({}).times({}){}",
            expr(&inner["path"])?,
            inner["scale"],
            if inner["reading_scale"] == Value::Bool(false) {
                ".without_reading_scale()"
            } else {
                ""
            }
        ),
        _ => return Err(error("unknown length unit")),
    })
}

fn layout_length(value: &Value) -> Result<String> {
    match value["kind"].as_str() {
        Some("auto") => Ok("auto()".to_owned()),
        Some("definite") => length(&value["value"]),
        Some("signed") => {
            let inner = &value["value"];
            let constructor = match inner["unit"].as_str() {
                Some("pixels") => "offset_px",
                Some("rems") => "offset_rem",
                Some("relative") => "offset_relative",
                _ => return Err(error("unknown signed length unit")),
            };
            Ok(format!("{constructor}({})", inner["value"]))
        }
        _ => Err(error("unknown layout length")),
    }
}

fn color_expr(value: &Value) -> Result<String> {
    fn render(color: &ColorValue) -> String {
        match color {
            ColorValue::Literal(c) => format!("rgba(0x{:08x})", c.as_rgba_hex()),
            ColorValue::Token(v) => format!("theme_color({})", quoted(v)),
            ColorValue::Mix {
                base,
                other,
                weight,
            } => format!(
                "{}.mix({}, {:?})",
                render(base),
                render(other),
                f64::from(*weight) / 1000.0
            ),
            ColorValue::Alpha { color, alpha } => {
                format!("{}.alpha({:?})", render(color), f64::from(*alpha) / 1000.0)
            }
            ColorValue::PremultipliedMix {
                base,
                other,
                weight,
            } => format!(
                "{}.mix_premultiplied({}, {:?})",
                render(base),
                render(other),
                f64::from(*weight) / 1000.0
            ),
            ColorValue::Over {
                foreground,
                background,
            } => {
                format!("{}.over({})", render(foreground), render(background))
            }
            ColorValue::Readable {
                color,
                toward,
                background,
                ratio,
            } => format!(
                "{}.readable({}, {}, {:?})",
                render(color),
                render(toward),
                render(background),
                f64::from(*ratio) / 100.0
            ),
            ColorValue::ByMode { light, dark } => {
                format!("by_mode({}, {})", render(light), render(dark))
            }
        }
    }
    let color: ColorValue =
        serde_json::from_value(value.clone()).map_err(|e| error(e.to_string()))?;
    Ok(render(&color))
}

fn variable_style_expr(value: &crate::token::Variable<Style>) -> Result<String> {
    match value {
        crate::token::Variable::Fixed(style) => style_expr(style),
        crate::token::Variable::ByEnv(table) => {
            fn render_branch(
                depth: usize,
                keys: &[crate::token::Symbol],
                entries: &[(Vec<String>, String)],
            ) -> Result<String> {
                if depth == keys.len() {
                    return entries
                        .first()
                        .map(|(_, value)| value.clone())
                        .ok_or_else(|| error("empty environment style branch"));
                }
                let mut grouped: BTreeMap<String, Vec<(Vec<String>, String)>> = BTreeMap::new();
                for (path, rendered) in entries {
                    grouped
                        .entry(path[depth].clone())
                        .or_default()
                        .push((path.clone(), rendered.clone()));
                }
                let mut fields = BTreeMap::new();
                for (branch_key, branch_entries) in grouped {
                    fields.insert(branch_key, render_branch(depth + 1, keys, &branch_entries)?);
                }
                Ok(render_map(fields))
            }

            let rendered_entries = table
                .entries()
                .map(|(path, style)| {
                    Ok((
                        path.iter().map(|s| s.as_str().to_string()).collect(),
                        style_expr(style)?,
                    ))
                })
                .collect::<Result<Vec<(Vec<String>, String)>>>()?;
            let keys = table
                .keys()
                .iter()
                .map(|k| quoted(&k.as_str()))
                .collect::<Vec<_>>()
                .join(", ");
            Ok(format!(
                "by_env([{}], {})",
                keys,
                render_branch(0, table.keys(), &rendered_entries)?
            ))
        }
    }
}

fn style_expr(style: &Style) -> Result<String> {
    let mut output = properties_expr(&json(&style.base)?)?;
    for (name, state) in [
        ("hover", &style.hover),
        ("active", &style.active),
        ("focus", &style.focus),
        ("group_focus", &style.group_focus),
        ("group_hover", &style.group_hover),
        ("focus_within", &style.focus_within),
        ("disabled", &style.disabled),
    ] {
        if let Some(state) = state {
            let _ = write!(output, ".{name}({})", properties_expr(&json(state)?)?);
        }
    }
    Ok(output)
}

fn paint_expr(ink: &crate::PaintColor) -> Result<String> {
    use crate::canvas::PaintColorSource;
    match ink.source() {
        PaintColorSource::Value(value) => color_expr(&json(value)?),
        PaintColorSource::CurrentText => Ok("current_color()".to_owned()),
        PaintColorSource::Alpha(value, alpha) => Ok(format!(
            "alpha({}, {:?})",
            paint_expr(value)?,
            f64::from(*alpha) / 1000.
        )),
    }
}

fn background_expr(value: &Value) -> Result<String> {
    match serde_json::from_value::<crate::BackgroundFill>(value.clone())
        .map_err(|e| error(e.to_string()))?
    {
        crate::BackgroundFill::Solid(ink) => Ok(format!("background({})", paint_expr(&ink)?)),
        crate::BackgroundFill::LegacyLinear(gradient) => Ok(format!(
            "linear_gradient(linear_gradient(#{{angle: {:?}, from: {}, to: {}}}))",
            gradient.angle_degrees,
            color_expr(&json(&gradient.from)?)?,
            color_expr(&json(&gradient.to)?)?
        )),
        crate::BackgroundFill::Layers(stack) => {
            let mut layers = Vec::new();
            for layer in stack.layers() {
                let stops = layer
                    .stops
                    .iter()
                    .map(|stop| {
                        Ok(format!(
                            "#{{offset: {:?}, color: {}}}",
                            stop.offset,
                            paint_expr(&stop.color)?
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;
                layers.push(format!(
                    "#{{angle: {:?}, stops: [{}], origin: {}, clip: {}, size: {:?}, position: {:?}}}",
                    layer.angle, stops.join(", "), json(&layer.origin)?, json(&layer.clip)?,
                    layer.size, layer.position
                ));
            }
            Ok(format!(
                "layered_gradient(layered_gradient([{}]))",
                layers.join(", ")
            ))
        }
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "Keep the exhaustive Style field-to-constructor mapping together; unsupported fields fail closed"
)]
fn properties_expr(value: &Value) -> Result<String> {
    let mut output = "style()".to_owned();
    let fields = object(value)?;
    if fields
        .get("layout_constraints")
        .and_then(|layout| layout.get("overflow_row"))
        == Some(&Value::Bool(true))
    {
        output.push_str(".overflow_row()");
    }
    // Display helpers also set direction; establish it before applying the
    // explicitly authored direction, not in alphabetic JSON-key order.
    let ordered = fields
        .get_key_value("display")
        .into_iter()
        .chain(fields.iter().filter(|(key, _)| key.as_str() != "display"));
    for (key, value) in ordered {
        if value.is_null() {
            continue;
        }
        let call = match key.as_str() {
            "padding" | "margin" | "border_widths" | "radii" => {
                let prefix = match key.as_str() {
                    "border_widths" => "border",
                    "radii" => "radius",
                    v => v,
                };
                for (edge, value) in object(value)? {
                    if value.is_null() {
                        continue;
                    }
                    let v = if key == "margin" {
                        layout_length(value)?
                    } else {
                        length(value)?
                    };
                    let _ = write!(output, ".{prefix}_{edge}({v})");
                }
                continue;
            }
            "display" => match value.as_str() {
                Some("block") => "block()",
                Some("grid") => "grid()",
                Some("none") => "hidden()",
                Some("flex") => "flex_row()",
                _ => return Err(error("unknown display")),
            }
            .to_owned(),
            "direction" => match value.as_str() {
                Some("row") => "flex_row()",
                Some("column") => "flex_col()",
                _ => return Err(error("unknown flex direction")),
            }
            .to_owned(),
            "align" | "align_self" | "justify" => {
                let prefix = match key.as_str() {
                    "align" => "items",
                    "align_self" => "self",
                    _ => "justify",
                };
                format!(
                    "{prefix}_{}()",
                    value
                        .as_str()
                        .ok_or_else(|| error("alignment must be a string"))?
                )
            }
            "flex_wrap" => match value.as_str() {
                Some("no_wrap") => "flex_nowrap()",
                Some("wrap") => "flex_wrap()",
                Some("wrap_reverse") => "flex_wrap_reverse()",
                _ => return Err(error("unknown wrapping")),
            }
            .to_owned(),
            "background" => background_expr(value)?,
            "border_color" => {
                let ink =
                    serde_json::from_value(value.clone()).map_err(|e| error(e.to_string()))?;
                format!("border_color({})", paint_expr(&ink)?)
            }
            "text_color" => {
                format!("{key}({})", color_expr(value)?)
            }
            "inherit_text_color" if value == true => "inherit_text_color()".to_owned(),
            "inherit_text_color" => continue,
            "width" | "height" | "min_width" | "max_width" | "min_height" | "max_height"
            | "flex_basis" | "top" | "right" | "bottom" | "left" | "inset_start" | "inset_end" => {
                format!("{key}({})", layout_length(value)?)
            }
            "gap" | "font_size" | "line_height" => format!("{key}({})", length(value)?),
            "letter_spacing" => {
                let unit = match value.get("unit").and_then(Value::as_str) {
                    Some("pixels") => "px",
                    Some("em") => "em",
                    _ => return Err(error("unknown letter spacing unit")),
                };
                let amount = value
                    .get("value")
                    .ok_or_else(|| error("missing letter spacing value"))?;
                format!("letter_spacing_{unit}({amount})")
            }
            "opacity"
            | "isolated_opacity"
            | "foreground_blur"
            | "foreground_grayscale"
            | "backdrop_blur"
            | "translate_x"
            | "translate_y" => {
                format!("{key}({value})")
            }
            "glyph_sweep" => match value.get("kind").and_then(Value::as_str) {
                Some("off") => "without_glyph_sweep()".to_owned(),
                Some("overlay") => format!(
                    "glyph_sweep({}, {}, {})",
                    color_expr(
                        value
                            .get("ink")
                            .ok_or_else(|| error("missing glyph highlight ink"))?
                    )?,
                    value
                        .get("half_width")
                        .ok_or_else(|| error("missing glyph highlight width"))?,
                    value
                        .get("phase")
                        .ok_or_else(|| error("missing glyph highlight phase"))?
                ),
                Some("replace") => format!(
                    "glyph_ink_sweep({}, {}, {}, {})",
                    color_expr(
                        value
                            .get("ink")
                            .ok_or_else(|| error("missing glyph highlight ink"))?
                    )?,
                    value
                        .get("half_width")
                        .ok_or_else(|| error("missing glyph highlight width"))?,
                    value
                        .get("travel")
                        .ok_or_else(|| error("missing glyph highlight travel"))?,
                    value
                        .get("phase")
                        .ok_or_else(|| error("missing glyph highlight phase"))?,
                ),
                _ => return Err(error("unknown glyph sweep")),
            },
            "paint_reveal_x" => match value.get("kind").and_then(Value::as_str) {
                Some("off") => "without_paint_reveal_x()".to_owned(),
                Some("from_left") => format!(
                    "paint_reveal_x({}, {})",
                    value
                        .get("radius")
                        .ok_or_else(|| error("reveal radius is missing"))?,
                    value
                        .get("progress")
                        .ok_or_else(|| error("reveal progress is missing"))?,
                ),
                _ => return Err(error("unknown paint reveal")),
            },
            "bottom_fade" => match value.get("kind").and_then(Value::as_str) {
                Some("off") => "without_bottom_fade()".to_owned(),
                Some("linear") => format!(
                    "bottom_fade({}, {})",
                    value
                        .get("start")
                        .ok_or_else(|| error("fade start is missing"))?,
                    value
                        .get("end")
                        .ok_or_else(|| error("fade end is missing"))?,
                ),
                Some(kind @ ("edges" | "scroll_edges")) => format!(
                    "{}(\"{}\", {}, {})",
                    if kind == "edges" {
                        "edge_fade"
                    } else {
                        "scroll_edge_fade"
                    },
                    if value
                        .get("horizontal")
                        .and_then(Value::as_bool)
                        .ok_or_else(|| error("fade axis is missing"))?
                    {
                        "x"
                    } else {
                        "y"
                    },
                    value
                        .get("leading")
                        .ok_or_else(|| error("fade leading distance is missing"))?,
                    value
                        .get("trailing")
                        .ok_or_else(|| error("fade trailing distance is missing"))?,
                ),
                _ => return Err(error("unknown alpha fade")),
            },
            "transform" | "typography" | "font_family" | "font_fallbacks" | "flex_shrink"
            | "font_weight" | "row_span" | "line_clamp" => {
                format!("{key}({})", expr(value)?)
            }
            "grid_rows" => {
                if fields
                    .get("layout_constraints")
                    .and_then(|layout| layout.get("grid_intrinsic_rows"))
                    == Some(&Value::Bool(true))
                {
                    format!("grid_rows_intrinsic({value})")
                } else {
                    format!("grid_rows({value})")
                }
            }
            "column_span" => format!("col_span({value})"),
            "grid_columns" => {
                if fields
                    .get("layout_constraints")
                    .and_then(|layout| layout.get("grid_max_content"))
                    == Some(&Value::Bool(true))
                {
                    format!("grid_cols_max_content({value})")
                } else {
                    format!("grid_cols({value})")
                }
            }
            "flex_grow_weight" => format!("flex_grow({value})"),
            "flex_grow" if value == true => "flex_grow()".to_owned(),
            "visible" => if value == true {
                "visible()"
            } else {
                "invisible()"
            }
            .to_owned(),
            "clip" if value == true => "clip()".to_owned(),
            "text_ellipsis" if value == true => "text_ellipsis()".to_owned(),
            "border_style" => format!(
                "border_{}()",
                value
                    .as_str()
                    .ok_or_else(|| error("invalid border style"))?
            ),
            "position" => format!(
                "{}()",
                value.as_str().ok_or_else(|| error("invalid position"))?
            ),
            "font_slant" => if value == "italic" {
                "italic()"
            } else {
                "not_italic()"
            }
            .to_owned(),
            "text_align" => format!(
                "text_{}()",
                value
                    .as_str()
                    .ok_or_else(|| error("invalid text alignment"))?
            ),
            "white_space" => format!(
                "whitespace_{}()",
                if value == "no_wrap" {
                    "nowrap"
                } else {
                    "normal"
                }
            ),
            "cursor" => format!(
                "cursor_{}()",
                match value.as_str() {
                    Some("resize_horizontal") => "resize_x",
                    Some("resize_vertical") => "resize_y",
                    Some(v) => v,
                    _ => return Err(error("invalid cursor")),
                }
            ),
            "hit_test" => if value == "block" {
                "occlude()"
            } else {
                "occlude_except_scroll()"
            }
            .to_owned(),
            "font_features" => {
                for (tag, v) in object(value)? {
                    let _ = write!(output, ".font_feature({}, {v})", quoted(tag));
                }
                continue;
            }
            "shadows" => {
                let mut shadows = Vec::new();
                for shadow in value
                    .as_array()
                    .ok_or_else(|| error("shadows must be an array"))?
                {
                    shadows.push(format!(
                        "shadow(#{{ x: {}, y: {}, blur: {}, spread: {}, color: {}, inset: {} }})",
                        shadow["x"],
                        shadow["y"],
                        shadow["blur"],
                        shadow["spread"],
                        paint_expr(
                            &serde_json::from_value(shadow["color"].clone())
                                .map_err(|e| error(e.to_string()))?
                        )?,
                        shadow["inset"]
                    ));
                }
                format!("shadows([{}])", shadows.join(", "))
            }
            "focus_ring" => format!(
                "focus_ring({}, {}, {})",
                length(&value["width"])?,
                layout_length(&value["offset"])?,
                color_expr(&value["color"])?
            ),
            "layout_constraints" => {
                let constraints = object(value)?;
                if constraints.get("overflow_row") == Some(&Value::Bool(false)) {
                    output.push_str(".overflow_row(false)");
                }
                if let Some(priority) = constraints
                    .get("overflow_priority")
                    .filter(|value| !value.is_null())
                {
                    if priority == "required" {
                        output.push_str(".overflow_required()");
                    } else if let Some(priority) = priority.get("optional") {
                        let _ = write!(output, ".overflow_priority({priority})");
                    } else {
                        return Err(error("unknown overflow priority"));
                    }
                }
                for axis in ["gap_x", "gap_y"] {
                    if let Some(gap) = constraints.get(axis).filter(|value| !value.is_null()) {
                        let _ = write!(output, ".{axis}({})", length(gap)?);
                    }
                }
                if let Some(weight) = constraints
                    .get("flex_shrink_weight")
                    .filter(|v| !v.is_null())
                {
                    let _ = write!(output, ".flex_shrink_weight({weight})");
                }
                if let Some(cell) = constraints
                    .get("grid_cell")
                    .filter(|value| !value.is_null())
                {
                    let cell = cell
                        .as_array()
                        .filter(|cell| cell.len() == 2)
                        .ok_or_else(|| error("grid_cell needs a column/row pair"))?;
                    let _ = write!(output, ".grid_cell({},{})", cell[0], cell[1]);
                }
                if let Some(ratio) = constraints.get("aspect_ratio").filter(|v| !v.is_null()) {
                    let _ = write!(output, ".aspect_ratio({ratio})");
                }
                if let Some(grid) = constraints
                    .get("auto_grid_columns")
                    .filter(|v| !v.is_null())
                {
                    let _ = write!(
                        output,
                        ".auto_grid_columns({}, {}, {})",
                        expr(&grid["minimum"])?,
                        expr(&grid["preferred"])?,
                        expr(&grid["maximum"])?
                    );
                }
                continue;
            }
            // Public builders cannot express all native-only clearing states.
            // Refuse them instead of silently losing a declaration on save.
            _ => {
                return Err(error(format!(
                    "unsupported style declaration {key}: {value}"
                )));
            }
        };
        output.push('.');
        output.push_str(&call);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intrinsic_row_sizing_is_preserved_by_theme_export() {
        let style = Style::new()
            .grid_columns_max_content(2)
            .unwrap()
            .grid_rows_intrinsic(4)
            .unwrap();
        let source = style_expr(&style).unwrap();
        assert!(source.contains(".grid_rows_intrinsic(4)"), "{source}");
        assert!(source.contains(".grid_cols_max_content(2)"), "{source}");
        let mut engine = rhai::Engine::new();
        crate::style::register_style_api(&mut engine);
        let decoded: Style = engine.eval(&source).unwrap();
        assert_eq!(decoded, style);
    }

    #[test]
    fn premultiplied_tints_round_trip_in_theme_source() {
        let mut engine = rhai::Engine::new();
        crate::style::register_style_api(&mut engine);
        let input = r#"style().text_color(mix_premultiplied(theme_color("agent"), rgba(0x17171780), 0.15))"#;
        let style = engine.eval::<Style>(input).unwrap();
        let exported = style_expr(&style).unwrap();
        assert!(exported.contains("mix_premultiplied"));
        assert_eq!(engine.eval::<Style>(&exported).unwrap(), style);
    }

    #[test]
    fn flex_shrink_weights_round_trip_and_validate() {
        let mut engine = rhai::Engine::new();
        crate::style::register_style_api(&mut engine);
        let style = engine
            .eval::<Style>("style().flex_shrink_weight(1000).gap_x(px(4)).hover(style().flex_shrink_weight(0.25))")
            .unwrap();
        let exported = style_expr(&style).unwrap();
        assert_eq!(engine.eval::<Style>(&exported).unwrap(), style);
        let reset = style.clone().flex_shrink(false);
        assert_eq!(reset.base.effective_flex_shrink(), Some(0.0));
        assert!(reset.base.flex_shrink_weight().is_none());
        assert_eq!(reset.base.gap_x(), style.base.gap_x());
        assert_eq!(
            style.clone().merged(&Style::new().flex_shrink(false)).base,
            reset.base
        );
        let reversed = Style::new().flex_shrink(false).merged(&style);
        assert_eq!(reversed.base.effective_flex_shrink(), Some(1000.0));
        assert!(reversed.base.flex_shrink.is_none());
        for source in ["-1", "1001", "-0.5"] {
            assert!(
                engine
                    .eval::<Style>(&format!("style().flex_shrink_weight({source})"))
                    .is_err()
            );
        }
        assert!(Style::new().flex_shrink_weight(f64::NAN).is_err());
        assert!(
            serde_json::from_value::<crate::style::LayoutConstraints>(
                serde_json::json!({"flex_shrink_weight": 1001.0})
            )
            .is_err()
        );
    }

    #[test]
    fn group_grayscale_round_trips_and_validates_its_pseudo_refinements() {
        let mut engine = rhai::Engine::new();
        crate::style::register_style_api(&mut engine);
        let original: Style = engine
            .eval("style().foreground_grayscale(0.65).hover(style().foreground_grayscale(0))")
            .unwrap();
        let source = style_expr(&original).unwrap();
        assert!(source.contains("foreground_grayscale(0.65)"), "{source}");
        assert_eq!(engine.eval::<Style>(&source).unwrap(), original);
        let hover =
            original.resolve(&crate::InteractionState::default().with(crate::PseudoState::Hovered));
        assert_eq!(hover.foreground_grayscale, Some(0.0));
        assert!(
            engine
                .eval::<Style>("style().foreground_grayscale(2)")
                .is_err()
        );
        assert!(
            serde_json::from_str::<crate::StyleProperties>(r#"{"foreground_grayscale":-0.1}"#)
                .is_err()
        );
    }

    #[test]
    fn edge_fade_round_trips_through_theme_source_and_pseudo_state() {
        let mut engine = rhai::Engine::new();
        crate::style::register_style_api(&mut engine);
        let original: Style = engine
            .eval(
                r#"style().edge_fade("x", 9.5, 0).hover(style().scroll_edge_fade("y", 17, 6))
                       .disabled(style().without_bottom_fade())"#,
            )
            .unwrap();
        let source = style_expr(&original).unwrap();
        assert!(source.contains(r#"edge_fade("x", 9.5, 0.0)"#), "{source}");
        assert!(
            source.contains(r#"scroll_edge_fade("y", 17.0, 6.0)"#),
            "{source}"
        );
        assert_eq!(engine.eval::<Style>(&source).unwrap(), original);
    }

    #[test]
    fn glyph_sweep_round_trips_through_a_theme_and_can_be_cleared() {
        let mut engine = rhai::Engine::new();
        crate::style::register_style_api(&mut engine);
        let original: Style = engine
            .eval(
                r#"style().glyph_sweep(theme_color("sheen"),0.26,0.625)
                       .disabled(style().without_glyph_sweep())"#,
            )
            .unwrap();
        let source = style_expr(&original).unwrap();
        assert!(
            source.contains(r#"glyph_sweep(theme_color("sheen"), 0.26, 0.625)"#),
            "{source}"
        );
        assert_eq!(engine.eval::<Style>(&source).unwrap(), original);
        let disabled = original
            .resolve(&crate::InteractionState::default().with(crate::PseudoState::Disabled));
        assert_eq!(disabled.glyph_sweep, Some(crate::GlyphSweep::Off));
    }

    #[test]
    fn glyph_ink_replacement_round_trips_and_checks_motion_extent() {
        let mut engine = rhai::Engine::new();
        crate::style::register_style_api(&mut engine);
        let original: Style = engine
            .eval(
                r#"style().glyph_ink_sweep(theme_color("sheen"),0.25,1.5,0.625)
                           .disabled(style().without_glyph_sweep())"#,
            )
            .unwrap();
        assert_eq!(
            engine
                .eval::<Style>(&style_expr(&original).unwrap())
                .unwrap(),
            original
        );
        for invalid in [
            "style().glyph_ink_sweep(rgb(0xffffff),0.25,1.49,0.5)",
            "style().glyph_ink_sweep(rgb(0xffffff),0.25,3.01,0.5)",
        ] {
            assert!(engine.eval::<Style>(invalid).is_err());
        }
    }

    #[test]
    fn reveal_round_trips_through_theme_style_and_can_be_cleared() {
        let mut engine = rhai::Engine::new();
        crate::style::register_style_api(&mut engine);
        let original: Style = engine
            .eval("style().paint_reveal_x(4, 0.25).hover(style().without_paint_reveal_x())")
            .unwrap();
        let source = style_expr(&original).unwrap();
        assert_eq!(engine.eval::<Style>(&source).unwrap(), original);
        let hovered =
            original.resolve(&crate::InteractionState::default().with(crate::PseudoState::Hovered));
        assert_eq!(hovered.paint_reveal_x, Some(crate::PaintRevealX::Off));
        for invalid in [
            "style().paint_reveal_x(-1, 0.25)",
            "style().paint_reveal_x(513, 0.25)",
            "style().paint_reveal_x(4, -0.01)",
            "style().paint_reveal_x(4, 1.01)",
        ] {
            assert!(engine.eval::<Style>(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn complete_recipes_replace_only_the_sheet_and_keep_host_overrides() {
        let runtime = crate::RuntimeEngine::new();
        let engine = runtime.engine();
        let base = crate::load_token_base(
            engine,
            "base",
            r#"fn tokens() { #{
                colors: #{ accent: 0x003344ff },
                component_styles: #{ "components/probe": #{
                    root: style().opacity(0.4), spare: style().width(px(7))
                } }
            } }"#,
        )
        .unwrap();
        let host = crate::load_token_base(
            engine,
            "host",
            r#"fn tokens() { #{
                component_styles: #{ "components/probe": #{ root: style().height(px(12)) } }
            } }"#,
        )
        .unwrap();
        let overrides = crate::ThemeTokenOverrides {
            component_styles: host.component_styles,
            ..Default::default()
        };
        let palette_source = r#"fn theme() { #{
            family: "Probe", name: "Light", mode: "light",
            tokens: #{ component_styles: #{ "components/probe": #{ root: style().width(px(20)) } } }
        } }"#;
        let complete_source =
            palette_source.replace("tokens: #{", "tokens: #{ component_styles_replace: true,");
        let load = |name, source| {
            crate::load_theme_with_layers(engine, Some(&base), name, source, &overrides).unwrap()
        };
        let id = crate::ModuleId::parse("components/probe").unwrap();
        let palette = load("palette", palette_source);
        let palette_parts = palette.tokens.component_styles.component(&id).unwrap();
        assert_eq!(palette_parts["root"].base.opacity, Some(0.4));
        assert!(palette_parts.contains_key("spare"));
        let complete = load("complete", &complete_source);
        let complete_parts = complete.tokens.component_styles.component(&id).unwrap();
        assert_eq!(complete.tokens.colors, palette.tokens.colors);
        assert_eq!(
            complete_parts["root"],
            engine
                .eval::<Style>("style().width(px(20)).height(px(12))")
                .unwrap()
        );
        assert!(!complete_parts.contains_key("spare"));
        let exported = export_theme_source(&complete).unwrap();
        assert_eq!(load("exported", &exported).tokens, complete.tokens);
    }
}
