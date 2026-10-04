//! Decoding of Rhai token sources: palette themes (`theme() -> map`) and token
//! bases (`tokens() -> map`).
//!
//! Sources may use the ordinary style helpers: `px(8)`, `rem(1)`,
//! `theme_color("accent").mix(theme_color("surface"), 0.3)` and
//! `by_env("density", #{ comfortable: px(32), compact: px(28) })`. The verbose
//! serialized forms (`#{ unit: "pixels", value: 8.0 }`) remain accepted.

use std::collections::BTreeMap;

use rhai::{Array, Dynamic, Engine, Map, Scope};

use crate::theme::{
    ThemeError, ThemeLength, ThemeMotionOverrides, ThemeMotionSpring, ThemeTokenValue, TokenLayer,
    TypographyRole, TypographyToken,
};
use crate::token::{EnvTable, EnvironmentDeclaration, Symbol, Variable, valid_token_segment};
use crate::{ColorValue, Length, Rgba8, ThemeMode};

/// The raw `by_env(keys, table)` value returned to sources.
#[derive(Clone, Debug)]
pub struct EnvTableSource {
    keys: Vec<String>,
    table: Map,
}

impl rhai::CustomType for EnvTableSource {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder.with_name("EnvTable");
    }
}

impl EnvTableSource {
    /// Build from one environment name or an array of names.
    ///
    /// # Errors
    ///
    /// Returns a message for empty or non-string keys.
    pub fn new(keys: Dynamic, table: Map) -> Result<Self, String> {
        let keys = if keys.is_string() {
            vec![keys.into_string().unwrap_or_default()]
        } else if keys.is_array() {
            keys.into_typed_array::<rhai::ImmutableString>()
                .map_err(|_| "by_env keys must be strings".to_owned())?
                .into_iter()
                .map(|key| key.to_string())
                .collect()
        } else {
            return Err("by_env expects an environment name or an array of names".to_owned());
        };
        if keys.is_empty() || keys.iter().any(|key| !valid_token_segment(key)) {
            return Err("by_env keys must be snake_case environment names".to_owned());
        }
        Ok(Self { keys, table })
    }
}

pub(crate) struct DecodedTheme {
    pub family: String,
    pub name: String,
    pub mode: ThemeMode,
    pub tokens: TokenLayer,
}

fn evaluate(
    engine: &Engine,
    source_name: &str,
    source: &str,
    function: &str,
) -> Result<Dynamic, ThemeError> {
    let mut ast = engine
        .compile(source)
        .map_err(|error| ThemeError::Script(error.to_string()))?;
    crate::engine::validate_assignment_targets(&ast)
        .map_err(|error| ThemeError::Script(error.to_string()))?;
    ast.set_source(source_name);
    engine
        .call_fn(&mut Scope::new(), &ast, function, ())
        .map_err(|error| ThemeError::Script(error.to_string()))
}

pub(crate) fn decode_theme(
    engine: &Engine,
    source_name: &str,
    source: &str,
) -> Result<DecodedTheme, ThemeError> {
    let raw = evaluate(engine, source_name, source, "theme")?;
    let mut map = expect_map(raw, "theme()")?;
    let family = take_string(&mut map, "family", "theme")?;
    let name = take_string(&mut map, "name", "theme")?;
    let mode = match take_string(&mut map, "mode", "theme")?.as_str() {
        "light" => ThemeMode::Light,
        "dark" => ThemeMode::Dark,
        other => {
            return Err(decode_error(
                "theme.mode",
                format!("expected \"light\" or \"dark\", got {other:?}"),
            ));
        }
    };
    let tokens = match map.remove("tokens") {
        Some(tokens) => decode_layer(expect_map(tokens, "theme.tokens")?)?,
        None => TokenLayer::default(),
    };
    reject_unknown(&map, "theme")?;
    Ok(DecodedTheme {
        family,
        name,
        mode,
        tokens,
    })
}

pub(crate) fn decode_token_base(
    engine: &Engine,
    source_name: &str,
    source: &str,
) -> Result<TokenLayer, ThemeError> {
    let raw = evaluate(engine, source_name, source, "tokens")?;
    decode_layer(expect_map(raw, "tokens()")?)
}

fn decode_layer(map: Map) -> Result<TokenLayer, ThemeError> {
    let mut layer = TokenLayer::default();
    for (key, value) in map {
        let key = key.as_str();
        match key {
            "colors" => {
                for (name, value) in expect_map(value, "colors")? {
                    let path = name.to_string();
                    let color = decode_color(&value, &format!("colors.{path}"))?;
                    layer.colors.insert(path, color);
                }
            }
            "spacing" => {
                for (name, value) in expect_map(value, "spacing")? {
                    let length = decode_length(&value, &format!("spacing.{name}"))?;
                    layer.spacing.insert(name.to_string(), length);
                }
            }
            "radii" | "radius" => {
                for (name, value) in expect_map(value, key)? {
                    let length = decode_length(&value, &format!("radius.{name}"))?;
                    layer.radii.insert(name.to_string(), length);
                }
            }
            "typography" => decode_typography(expect_map(value, "typography")?, &mut layer)?,
            "motion" => layer.motion = decode_motion(&value)?,
            "environment" => {
                for (name, value) in expect_map(value, "environment")? {
                    let declaration = decode_declaration(&name, value)?;
                    layer.environment.insert(name.to_string(), declaration);
                }
            }
            "namespaces" => {
                for (namespace, tokens) in expect_map(value, "namespaces")? {
                    decode_namespace(&namespace, tokens, &mut layer)?;
                }
            }
            namespace => decode_namespace(namespace, value, &mut layer)?,
        }
    }
    layer.normalize_colors();
    Ok(layer)
}

fn decode_namespace(
    namespace: &str,
    value: Dynamic,
    layer: &mut TokenLayer,
) -> Result<(), ThemeError> {
    for (name, value) in expect_map(value, namespace)? {
        let path = format!("{namespace}.{name}");
        if let Some(color) = try_color(&value, &path)? {
            layer.colors.insert(path, color);
            continue;
        }
        let token = decode_namespaced_value(value, &path)?;
        layer
            .namespaces
            .entry(namespace.to_owned())
            .or_default()
            .insert(name.to_string(), token);
    }
    Ok(())
}

fn decode_namespaced_value(value: Dynamic, path: &str) -> Result<ThemeTokenValue, ThemeError> {
    if value.is_float() {
        return Ok(ThemeTokenValue::Number(
            value.as_float().unwrap_or_default(),
        ));
    }
    if value.is_string() {
        return Ok(ThemeTokenValue::String(
            value.into_string().unwrap_or_default(),
        ));
    }
    if let Some((map, kind)) = value.read_lock::<Map>().and_then(|map| {
        map.get("type")
            .and_then(|kind| kind.clone().into_string().ok())
            .map(|kind| (map.clone(), kind))
    }) {
        {
            let inner = map.get("value").cloned().unwrap_or(Dynamic::UNIT);
            return match kind.as_str() {
                "color" => Ok(ThemeTokenValue::Color(decode_literal_color(inner, path)?)),
                "length" => Ok(ThemeTokenValue::Length(decode_length(&inner, path)?)),
                "number" => inner
                    .as_float()
                    .or_else(|_| {
                        inner
                            .as_int()
                            .ok()
                            .and_then(|value| i32::try_from(value).ok())
                            .map(f64::from)
                            .ok_or("not a number")
                    })
                    .map(ThemeTokenValue::Number)
                    .map_err(|_| decode_error(path, "expected a number")),
                "string" => inner
                    .into_string()
                    .map(ThemeTokenValue::String)
                    .map_err(|_| decode_error(path, "expected a string")),
                other => Err(decode_error(path, format!("unknown token type {other:?}"))),
            };
        }
    }
    decode_length(&value, path).map(ThemeTokenValue::Length)
}

fn decode_typography(map: Map, layer: &mut TokenLayer) -> Result<(), ThemeError> {
    for (key, value) in map {
        match key.as_str() {
            "family" => {
                layer.typography_family = Some(if value.is_unit() {
                    crate::theme::FamilyChoice::Platform
                } else {
                    crate::theme::FamilyChoice::Named(dynamic_string(value, "typography.family")?)
                });
            }
            "fallbacks" => {
                layer.typography_fallbacks = Some(string_array(value, "typography.fallbacks")?);
            }
            "roles" => {
                for (role, value) in expect_map(value, "typography.roles")? {
                    let decoded = decode_role(value, &format!("typography.{role}"))?;
                    layer.typography_roles.insert(role.to_string(), decoded);
                }
            }
            other => {
                return Err(decode_error(
                    "typography",
                    format!("unknown field {other:?}; expected family, fallbacks or roles"),
                ));
            }
        }
    }
    Ok(())
}

fn decode_role(value: Dynamic, path: &str) -> Result<TypographyRole, ThemeError> {
    if value.is_string() {
        return Ok(TypographyRole::Alias(Variable::Fixed(
            value.into_string().unwrap_or_default(),
        )));
    }
    if let Some(source) = value
        .read_lock::<EnvTableSource>()
        .map(|value| value.clone())
    {
        return decode_env_table(source, path, &mut |leaf, leaf_path| {
            dynamic_string(leaf, leaf_path)
        })
        .map(|table| TypographyRole::Alias(Variable::ByEnv(table)));
    }
    let mut map = expect_map(value, path)?;
    let size = decode_fixed_length(&take(&mut map, "size", path)?, &format!("{path}.size"))?;
    let line_height = decode_fixed_length(
        &take(&mut map, "line_height", path)?,
        &format!("{path}.line_height"),
    )?;
    let weight = take(&mut map, "weight", path)?
        .as_int()
        .ok()
        .and_then(|weight| u16::try_from(weight).ok())
        .ok_or_else(|| decode_error(&format!("{path}.weight"), "expected an integer weight"))?;
    let family = match map.remove("family") {
        None => None,
        Some(value) if value.is_unit() => None,
        Some(value) => Some(dynamic_string(value, &format!("{path}.family"))?),
    };
    let fallbacks = match map.remove("fallbacks") {
        None => Vec::new(),
        Some(value) => string_array(value, &format!("{path}.fallbacks"))?,
    };
    reject_unknown(&map, path)?;
    Ok(TypographyRole::Style(TypographyToken {
        size,
        line_height,
        weight,
        family,
        fallbacks,
    }))
}

fn decode_declaration(name: &str, value: Dynamic) -> Result<EnvironmentDeclaration, ThemeError> {
    let path = format!("environment.{name}");
    let mut map = expect_map(value, &path)?;
    let values = string_array(take(&mut map, "values", &path)?, &format!("{path}.values"))?;
    let default = dynamic_string(
        take(&mut map, "default", &path)?,
        &format!("{path}.default"),
    )?;
    reject_unknown(&map, &path)?;
    EnvironmentDeclaration::new(name, &values, &default)
        .map_err(|error| decode_error(&path, error.to_string()))
}

fn decode_motion(value: &Dynamic) -> Result<ThemeMotionOverrides, ThemeError> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct MotionSource {
        #[serde(default)]
        durations_ms: BTreeMap<String, u64>,
        #[serde(default)]
        easings: BTreeMap<String, crate::MotionEasing>,
        #[serde(default)]
        springs: BTreeMap<String, ThemeMotionSpring>,
        #[serde(default)]
        distances: BTreeMap<String, f64>,
        #[serde(default)]
        staggers_ms: BTreeMap<String, u64>,
    }
    let source = rhai::serde::from_dynamic::<MotionSource>(value)
        .map_err(|error| decode_error("motion", error.to_string()))?;
    Ok(ThemeMotionOverrides {
        durations_ms: source.durations_ms,
        easings: source.easings,
        springs: source.springs,
        distances: source.distances,
        staggers_ms: source.staggers_ms,
    })
}

fn decode_color(value: &Dynamic, path: &str) -> Result<ColorValue, ThemeError> {
    try_color(value, path)?.ok_or_else(|| {
        decode_error(
            path,
            "expected a 0xRRGGBBAA integer, a color string or a theme_color(...) expression",
        )
    })
}

fn try_color(value: &Dynamic, path: &str) -> Result<Option<ColorValue>, ThemeError> {
    if value.is_int() {
        return decode_literal_color(value.clone(), path)
            .map(|color| Some(ColorValue::Literal(color)));
    }
    if let Some(color) = value.read_lock::<ColorValue>() {
        return Ok(Some(color.clone()));
    }
    Ok(None)
}

fn decode_literal_color(value: Dynamic, path: &str) -> Result<Rgba8, ThemeError> {
    if let Ok(value) = value.as_int() {
        return u32::try_from(value)
            .map(Rgba8::from_rgba_hex)
            .map_err(|_| decode_error(path, "color integers must fit 0xRRGGBBAA"));
    }
    if value.is_string() {
        let text = value.into_string().unwrap_or_default();
        return match ColorValue::parse(&text) {
            Ok(ColorValue::Literal(color)) => Ok(color),
            Ok(_) => Err(decode_error(path, "expected a literal color")),
            Err(error) => Err(decode_error(path, error.to_string())),
        };
    }
    Err(decode_error(path, "expected a color"))
}

fn decode_length(value: &Dynamic, path: &str) -> Result<ThemeLength, ThemeError> {
    if let Some(source) = value
        .read_lock::<EnvTableSource>()
        .map(|value| value.clone())
    {
        return decode_env_table(source, path, &mut |leaf, leaf_path| {
            decode_fixed_length(&leaf, leaf_path)
        })
        .map(Variable::ByEnv);
    }
    decode_fixed_length(value, path).map(Variable::Fixed)
}

fn decode_fixed_length(value: &Dynamic, path: &str) -> Result<Length, ThemeError> {
    if let Some(length) = value.read_lock::<Length>() {
        return Ok(*length);
    }
    if value.is_map() {
        return rhai::serde::from_dynamic::<Length>(value)
            .map_err(|error| decode_error(path, error.to_string()));
    }
    Err(decode_error(
        path,
        "expected px(...), rem(...) or a length map",
    ))
}

fn decode_env_table<T>(
    source: EnvTableSource,
    path: &str,
    leaf: &mut dyn FnMut(Dynamic, &str) -> Result<T, ThemeError>,
) -> Result<EnvTable<T>, ThemeError> {
    fn walk<T>(
        value: Dynamic,
        depth: usize,
        keys: usize,
        prefix: &mut Vec<Symbol>,
        path: &str,
        entries: &mut BTreeMap<Vec<Symbol>, T>,
        leaf: &mut dyn FnMut(Dynamic, &str) -> Result<T, ThemeError>,
    ) -> Result<(), ThemeError> {
        if depth == keys {
            let leaf_path = format!(
                "{path}[{}]",
                prefix
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(".")
            );
            let value = leaf(value, &leaf_path)?;
            entries.insert(prefix.clone(), value);
            return Ok(());
        }
        for (name, value) in expect_map(value, path)? {
            if !valid_token_segment(&name) {
                return Err(decode_error(
                    path,
                    format!("environment value {name:?} must be snake_case"),
                ));
            }
            prefix.push(Symbol::intern(&name));
            walk(value, depth + 1, keys, prefix, path, entries, leaf)?;
            prefix.pop();
        }
        Ok(())
    }

    let keys = source
        .keys
        .iter()
        .map(|key| Symbol::intern(key))
        .collect::<Vec<_>>();
    let mut entries = BTreeMap::new();
    walk(
        Dynamic::from_map(source.table),
        0,
        keys.len(),
        &mut Vec::new(),
        path,
        &mut entries,
        leaf,
    )?;
    EnvTable::new(keys, entries).map_err(|error| decode_error(path, error.to_string()))
}

fn expect_map(value: Dynamic, path: &str) -> Result<Map, ThemeError> {
    value
        .try_cast::<Map>()
        .ok_or_else(|| decode_error(path, "expected a map"))
}

fn take(map: &mut Map, field: &str, path: &str) -> Result<Dynamic, ThemeError> {
    map.remove(field)
        .ok_or_else(|| decode_error(path, format!("missing field {field:?}")))
}

fn take_string(map: &mut Map, field: &str, path: &str) -> Result<String, ThemeError> {
    dynamic_string(take(map, field, path)?, &format!("{path}.{field}"))
}

fn dynamic_string(value: Dynamic, path: &str) -> Result<String, ThemeError> {
    value
        .into_string()
        .map_err(|_| decode_error(path, "expected a string"))
}

fn string_array(value: Dynamic, path: &str) -> Result<Vec<String>, ThemeError> {
    value
        .try_cast::<Array>()
        .ok_or_else(|| decode_error(path, "expected an array of strings"))?
        .into_iter()
        .map(|value| dynamic_string(value, path))
        .collect()
}

fn reject_unknown(map: &Map, path: &str) -> Result<(), ThemeError> {
    match map.keys().next() {
        Some(field) => Err(decode_error(path, format!("unknown field {field:?}"))),
        None => Ok(()),
    }
}

fn decode_error(path: &str, reason: impl Into<String>) -> ThemeError {
    ThemeError::InvalidTokenValue {
        token: path.to_owned(),
        reason: reason.into(),
    }
}
