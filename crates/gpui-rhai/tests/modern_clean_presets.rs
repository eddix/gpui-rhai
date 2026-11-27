//! Exact design contracts, independent from alternate-palette readability policy.
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use gpui_rhai::{
    BackgroundFill, ColorResolver, ColorValue, ComponentInstancePath, Environment, ExecutionPhase,
    Length, ModuleId, PaintColor, Rgba8, RuntimeEngine, Style, StyleProperties, ThemeManager,
    ThemeSelection, ThemeTokenOverrides, ThemeVariant, UiContext, UiRuntimeState,
    load_theme_with_layers, load_token_base,
};

// Preserve contextual leaves and exact alpha operations. Resolve only the
// named/literal color leaves in PaintColor's serializable recipe.
fn resolve_paint_ink(ink: &mut PaintColor, theme: &ThemeVariant) {
    fn resolve(value: &mut serde_json::Value, theme: &ThemeVariant) {
        match value["source"].as_str().unwrap() {
            "value" => {
                let color: ColorValue = serde_json::from_value(value["value"].take()).unwrap();
                let resolved = theme
                    .resolve(&color)
                    .unwrap_or_else(|| panic!("unresolved recipe color {color:?}"));
                value["value"] = serde_json::to_value(ColorValue::Literal(resolved)).unwrap();
            }
            "current_text" => {}
            "alpha" => resolve(&mut value["value"][0], theme),
            source => panic!("unknown paint source {source}"),
        }
    }
    let mut value = serde_json::to_value(&*ink).unwrap();
    resolve(&mut value, theme);
    *ink = serde_json::from_value(value).unwrap();
}

fn resolve_recipe_paint(properties: &mut StyleProperties, theme: &ThemeVariant) {
    let color = |value: &mut ColorValue| {
        *value = ColorValue::Literal(
            theme
                .resolve(value)
                .unwrap_or_else(|| panic!("unresolved recipe color {value:?}")),
        );
    };
    if let Some(value) = &mut properties.text_color {
        color(value);
    }
    if let Some(value) = &mut properties.border_color {
        resolve_paint_ink(value, theme);
    }
    match properties.background.as_deref_mut() {
        Some(BackgroundFill::Solid(ink)) => resolve_paint_ink(ink, theme),
        Some(BackgroundFill::LegacyLinear(gradient)) => {
            color(&mut gradient.from);
            color(&mut gradient.to);
        }
        Some(BackgroundFill::Layers(stack)) => {
            let mut layers = stack.layers().to_vec();
            for layer in &mut layers {
                for stop in &mut layer.stops {
                    resolve_paint_ink(&mut stop.color, theme);
                }
            }
            *stack = layers.try_into().unwrap();
        }
        None => {}
    }
    if let Some(shadows) = &mut properties.shadows {
        for shadow in shadows {
            resolve_paint_ink(&mut shadow.color, theme);
        }
    }
    if let Some(ring) = &mut properties.focus_ring {
        color(&mut ring.color);
        ring.width = theme
            .tokens
            .resolve_length(ring.width, &Environment::EMPTY)
            .unwrap();
        if let gpui_rhai::LayoutLength::Definite(length) = &mut ring.offset {
            *length = theme
                .tokens
                .resolve_length(*length, &Environment::EMPTY)
                .unwrap();
        }
    }
}

fn resolve_recipe_lengths(properties: &mut StyleProperties, theme: &ThemeVariant) {
    let length = |value: &mut Option<Length>| {
        if let Some(current) = value {
            *current = theme
                .tokens
                .resolve_length(*current, &Environment::EMPTY)
                .expect("recipe length resolves");
        }
    };
    let layout = |value: &mut Option<gpui_rhai::LayoutLength>| {
        if let Some(gpui_rhai::LayoutLength::Definite(current)) = value {
            *current = theme
                .tokens
                .resolve_length(*current, &Environment::EMPTY)
                .expect("recipe layout length resolves");
        }
    };
    for value in [
        &mut properties.gap,
        &mut properties.font_size,
        &mut properties.line_height,
    ] {
        length(value);
    }
    for edge in [&mut properties.padding, &mut properties.border_widths] {
        for value in [
            &mut edge.top,
            &mut edge.right,
            &mut edge.bottom,
            &mut edge.left,
            &mut edge.start,
            &mut edge.end,
        ] {
            length(value);
        }
    }
    for value in [
        &mut properties.radii.top_left,
        &mut properties.radii.top_right,
        &mut properties.radii.bottom_right,
        &mut properties.radii.bottom_left,
        &mut properties.radii.start,
        &mut properties.radii.end,
    ] {
        length(value);
    }
    for value in [
        &mut properties.width,
        &mut properties.height,
        &mut properties.min_width,
        &mut properties.min_height,
        &mut properties.max_width,
        &mut properties.max_height,
        &mut properties.flex_basis,
        &mut properties.top,
        &mut properties.right,
        &mut properties.bottom,
        &mut properties.left,
        &mut properties.inset_start,
        &mut properties.inset_end,
        &mut properties.margin.top,
        &mut properties.margin.right,
        &mut properties.margin.bottom,
        &mut properties.margin.left,
        &mut properties.margin.start,
        &mut properties.margin.end,
    ] {
        layout(value);
    }
}

fn resolved_recipe(mut style: Style, theme: &ThemeVariant) -> Style {
    for state in [
        Some(&mut style.base),
        style.hover.as_mut(),
        style.active.as_mut(),
        style.focus.as_mut(),
        style.group_focus.as_mut(),
        style.group_hover.as_mut(),
        style.focus_within.as_mut(),
        style.disabled.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        resolve_recipe_paint(state, theme);
        resolve_recipe_lengths(state, theme);
    }
    style
}

#[test]
fn every_preset_recipe_resolves_its_entire_paint_table() {
    for &(name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let theme = preset(source);
        let mut styles = BTreeMap::new();
        for (id, parts) in theme.tokens.component_styles.iter() {
            for (part, style) in parts {
                let resolved = style
                    .resolve(&Environment::EMPTY, &theme.tokens.environment)
                    .expect("preset part style resolves with default environment");
                styles.insert(
                    format!("{id}.{part}"),
                    resolved_recipe(resolved.clone(), &theme),
                );
            }
        }
        for (namespace, tokens) in &theme.tokens.namespaces {
            for (token, value) in tokens {
                if let Some(style) =
                    value.resolve_style(&Environment::EMPTY, &theme.tokens.environment)
                {
                    styles.insert(
                        format!("token:{namespace}.{token}"),
                        resolved_recipe(style.clone(), &theme),
                    );
                }
            }
        }
        let named = theme
            .tokens
            .namespaces
            .values()
            .flat_map(|values| values.values())
            .filter(|value| matches!(value, gpui_rhai::ThemeTokenValue::Style(_)))
            .count();
        assert_eq!(
            styles.len(),
            theme.tokens.component_styles.len() + named,
            "{name}: every part and named recipe must be included"
        );
        assert!(named > 0 && theme.tokens.component_styles.len() > 103);
        // Opt-in diagnostic snapshots compare refactors outside the repository;
        // the ordinary test above always resolves every part and pseudo state.
        if let Some(directory) = std::env::var_os("GPUI_RHAI_PRESET_SNAPSHOT_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let snapshot =
                serde_json::json!({"colors":theme.tokens.color_snapshot(),"styles":styles});
            std::fs::write(
                directory.join(format!("{name}.json")),
                serde_json::to_vec_pretty(&snapshot).unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn grid_and_inset_focus_recipes_survive_full_theme_export() {
    let mut theme = preset(gpui_rhai_registry::MODERN_DARK_THEME);
    std::sync::Arc::make_mut(&mut theme.tokens)
        .namespaces
        .entry("custom".to_owned())
        .or_default()
        .insert(
            "overlay".to_owned(),
            gpui_rhai::ThemeTokenValue::Style(Box::new(gpui_rhai::Variable::Fixed(
                Style::new()
                    .background(
                        ColorValue::token("tag.chip")
                            .unwrap()
                            .over(ColorValue::token("surface_raised").unwrap()),
                    )
                    .grid_cell(1, 1)
                    .unwrap()
                    .focus_ring(
                        Length::Pixels(2.0),
                        gpui_rhai::SignedLength::Pixels(-2.0),
                        ColorValue::token("focus_ring").unwrap(),
                    )
                    .unwrap(),
            ))),
        );
    let custom = std::sync::Arc::make_mut(&mut theme.tokens)
        .namespaces
        .get_mut("custom")
        .unwrap();
    for (name, style) in [
        (
            "axis_gaps",
            Box::new(
                Style::new()
                    .gap(Length::Pixels(10.0))
                    .gap_x(Length::theme_spacing("sm").unwrap())
                    .gap_y(Length::Relative(0.1)),
            ),
        ),
        (
            "overflow",
            Box::new(
                Style::new()
                    .overflow_row(true)
                    .items_end()
                    .overflow_priority(0)
                    .unwrap()
                    .disabled(&Style::new().overflow_required().overflow_row(false)),
            ),
        ),
        (
            "equal_grid",
            Box::new(Style::new().grid_columns(3).unwrap()),
        ),
        (
            "content_grid",
            Box::new(Style::new().grid_columns_max_content(3).unwrap()),
        ),
        (
            "inherit_ink",
            Box::new(
                Style::new()
                    .inherit_text_color()
                    .hover(&Style::new().inherit_text_color()),
            ),
        ),
    ] {
        custom.insert(
            name.to_owned(),
            gpui_rhai::ThemeTokenValue::Style(Box::new(gpui_rhai::Variable::Fixed(*style))),
        );
    }
    let source = gpui_rhai::export_theme_source(&theme).unwrap();
    assert_eq!(
        gpui_rhai::load_theme_source(RuntimeEngine::new().engine(), "exported.rhai", &source)
            .unwrap(),
        theme
    );
}
use gpui_rhai_registry::{
    BUNDLED_CHART_SOURCES_BY_ID, BUNDLED_COMPONENT_SOURCES_BY_ID, BUNDLED_LAYOUT_SOURCES_BY_ID,
    BUNDLED_MOTION_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID, CLEAN_DARK_THEME,
    CLEAN_LIGHT_THEME, MODERN_DARK_THEME, MODERN_LIGHT_THEME, TOKEN_BASE_SOURCE,
};

fn preset(source: &str) -> ThemeVariant {
    preset_with_overrides(source, &ThemeTokenOverrides::default())
}

fn preset_with_overrides(source: &str, overrides: &ThemeTokenOverrides) -> ThemeVariant {
    let engine = RuntimeEngine::new();
    let base = load_token_base(engine.engine(), "tokens.rhai", TOKEN_BASE_SOURCE).unwrap();
    load_theme_with_layers(
        engine.engine(),
        Some(&base),
        "preset.rhai",
        source,
        overrides,
    )
    .unwrap()
}

#[test]
fn adjacent_month_dates_use_subtle_text_not_empty_field_placeholder_ink() {
    for &(name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let theme = preset(source);
        for component in ["components/calendar", "components/date_picker"] {
            let parts = theme
                .tokens
                .resolve_component_styles(&ModuleId::parse(component).unwrap(), &Environment::EMPTY)
                .unwrap();
            let part = &parts["day_outside"];
            let color = theme
                .resolve(part.base.text_color.as_ref().unwrap())
                .unwrap();
            assert_eq!(
                color,
                theme.tokens.color("text.subtle").unwrap(),
                "{name}/{component}"
            );
            assert_ne!(
                color,
                theme.tokens.color("field.placeholder").unwrap(),
                "{name}/{component}"
            );
        }
    }
}

#[test]
fn full_preset_parts_follow_common_and_namespaced_color_overrides() {
    let probe = gpui_rhai::Rgba8::from_rgba_hex(0x23bf_a5ff);
    for &(name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for (path, module, part, property) in [
            ("text_muted", "components/form_field", "description", "text"),
            (
                "field.placeholder",
                "components/input",
                "placeholder",
                "text",
            ),
            ("text.subtle", "components/menu", "shortcut", "text"),
            (
                "surface_raised",
                "components/popover",
                "content",
                "background",
            ),
            (
                "surface_hover",
                "components/progress",
                "track",
                "background",
            ),
            ("border", "components/column_browser", "root", "border"),
            ("table.grid", "components/table", "header_cell", "border"),
            ("table.surface", "components/table", "header", "background"),
            ("tag.chip", "components/tag", "root", "background"),
        ] {
            let theme = preset_with_overrides(
                source,
                &ThemeTokenOverrides {
                    colors: [(path.to_owned(), probe)].into(),
                    ..Default::default()
                },
            );
            let rules = theme
                .tokens
                .resolve_component_styles(&ModuleId::parse(module).unwrap(), &Environment::EMPTY)
                .unwrap();
            let properties = &rules[part].base;
            let resolve_ink = |value: &PaintColor| {
                value.resolve_with(Rgba8::from_rgba_hex(0xfe0d_44ff), &mut |color| {
                    theme.resolve(color)
                })
            };
            let resolved = match property {
                "text" => properties
                    .text_color
                    .as_ref()
                    .and_then(|value| theme.resolve(value)),
                "background" => match properties.background.as_deref() {
                    Some(BackgroundFill::Solid(ink)) => resolve_ink(ink),
                    _ => None,
                },
                _ => properties.border_color.as_ref().and_then(resolve_ink),
            };
            assert_eq!(
                resolved,
                Some(probe),
                "{name}: {path} must reach {module}.{part}"
            );
        }
    }
}

#[test]
fn derived_chip_contrast_is_materialized_in_theme_roles_not_per_node_searches() {
    for &(name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let theme = preset(source);
        for part in [
            "normal",
            "facet_neutral",
            "text_accent",
            "text_success",
            "text_warning",
            "text_danger",
            "facet_accent",
            "facet_success",
            "facet_warning",
            "facet_danger",
        ] {
            let Some(style) = theme.tokens.namespaces["tag"][part]
                .resolve_style(&Environment::EMPTY, &theme.tokens.environment)
            else {
                panic!("{name}/{part}: a color role replaced the style recipe");
            };
            let fill = style.base.background.as_deref().map(|fill| {
                fill.as_solid_value()
                    .expect("chip recipe background must be a named solid")
            });
            for color in [style.base.text_color.as_ref(), fill].into_iter().flatten() {
                assert!(
                    matches!(color, ColorValue::Token(_)),
                    "{name}/{part}: render-time contrast search {color:?}"
                );
                assert!(
                    theme.resolve(color).is_some(),
                    "{name}/{part}: missing named color"
                );
            }
        }
    }
}

#[test]
fn strong_badge_pairs_are_opaque_and_readable_in_presets_and_alternate_palettes() {
    for &(name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES
        .iter()
        .chain(gpui_rhai_registry::BUNDLED_PALETTE_SOURCES)
    {
        let theme = preset(source);
        for tone in ["neutral", "info", "success", "warning", "danger", "purple"] {
            let color = |suffix: &str| {
                theme
                    .tokens
                    .color(&format!("badge.{tone}_{suffix}"))
                    .unwrap()
            };
            let ink = color("strong_fg");
            let fill = color("strong_bg");
            assert_eq!(
                fill.as_rgba_hex() & 255,
                255,
                "{name}/{tone}: solid means opaque"
            );
            assert_eq!(
                ink.as_rgba_hex() & 255,
                255,
                "{name}/{tone}: solid label alpha"
            );
            assert!(
                ink.contrast_ratio(fill) >= 4.5,
                "{name}/{tone}: {}",
                ink.contrast_ratio(fill)
            );
        }
    }
    // Original subtle variants are not a side effect of derived strong calibration.
    for (source, fg, bg) in [
        (MODERN_LIGHT_THEME, 0x1e8a_4fff, 0xe6f6_edff),
        (CLEAN_LIGHT_THEME, 0x007d_2ee0, 0x00a4_3319),
        (MODERN_DARK_THEME, 0x5fc8_8aff, 0x1230_20ff),
        (CLEAN_DARK_THEME, 0x5fc8_8aff, 0x00f0_4a1f),
    ] {
        let theme = preset(source);
        assert_eq!(
            theme
                .tokens
                .color("badge.success_fg")
                .unwrap()
                .as_rgba_hex(),
            fg
        );
        assert_eq!(
            theme
                .tokens
                .color("badge.success_bg")
                .unwrap()
                .as_rgba_hex(),
            bg
        );
    }
}

#[test]
fn derived_collection_metadata_is_readable_on_normal_hover_and_selected_surfaces() {
    for &(name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES
        .iter()
        .chain(gpui_rhai_registry::BUNDLED_PALETTE_SOURCES)
    {
        let theme = preset(source);
        let color = |name| theme.tokens.color(name).unwrap();
        let ink = color("collection.metadata");
        assert_eq!(
            ink.as_rgba_hex() & 255,
            255,
            "{name}: explicit derived ink is opaque"
        );
        for surface in [
            "surface",
            "surface_raised",
            "surface_hover",
            "selection",
            "table.selection",
        ] {
            let backing = color(surface).over(color("surface_raised"));
            assert!(
                ink.contrast_ratio(backing) >= 4.5,
                "{name}/{surface}: {}",
                ink.contrast_ratio(backing)
            );
        }
    }
}

#[test]
fn derived_semantic_text_uses_composited_translucent_wells() {
    for &(name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES
        .iter()
        .chain(gpui_rhai_registry::BUNDLED_PALETTE_SOURCES)
    {
        let theme = preset(source);
        let color = |name| theme.tokens.color(name).unwrap();
        let wells = [
            color("surface_raised"),
            color("surface_hover").over(color("surface_raised")),
        ];
        for role in [
            "text.accent",
            "text.success",
            "text.warning",
            "text.danger",
            "syntax.string",
            "syntax.number",
            "syntax.keyword",
            "syntax.function",
            "syntax.type",
            "syntax.constant",
            "syntax.operator",
            "syntax.tag",
            "syntax.attribute",
        ] {
            let ink = color(role);
            assert_eq!(
                ink.as_rgba_hex() & 255,
                255,
                "{name}/{role}: derived ink is opaque"
            );
            for well in wells {
                assert!(
                    ink.contrast_ratio(well) >= 4.5,
                    "{name}/{role}: {}",
                    ink.contrast_ratio(well)
                );
            }
        }
    }
}

fn sized(name: &str) -> Environment {
    Environment::EMPTY
        .with(
            gpui_rhai::Symbol::intern("size"),
            gpui_rhai::Symbol::intern(name),
        )
        .unwrap()
}

#[test]
fn all_four_full_presets_load_with_exact_control_metrics() {
    for (source, clean) in [
        (MODERN_LIGHT_THEME, false),
        (MODERN_DARK_THEME, false),
        (CLEAN_LIGHT_THEME, true),
        (CLEAN_DARK_THEME, true),
    ] {
        let theme = preset(source);
        assert_eq!(theme.tokens.typography.family.as_deref(), Some("Inter"));
        for (index, size) in ["xs", "sm", "md", "lg"].iter().enumerate() {
            let env = sized(size);
            let button = if clean {
                [20.0, 24.0, 28.0, 32.0]
            } else {
                [28.0, 32.0, 40.0, 48.0]
            };
            let field = if clean {
                [24.0, 28.0, 32.0, 36.0]
            } else {
                [28.0, 32.0, 36.0, 40.0]
            };
            assert_eq!(
                theme
                    .tokens
                    .resolve_length(Length::token("metrics.button_h").unwrap(), &env),
                Some(Length::Pixels(button[index]))
            );
            assert_eq!(
                theme
                    .tokens
                    .resolve_length(Length::token("metrics.field_h").unwrap(), &env),
                Some(Length::Pixels(field[index]))
            );
            assert_eq!(
                theme
                    .tokens
                    .resolve_typography("button", &env)
                    .unwrap()
                    .weight,
                500
            );
        }
    }
}

#[test]
fn appearance_switch_keeps_geometry_and_type_but_design_switch_does_not() {
    for (light, dark) in [
        (MODERN_LIGHT_THEME, MODERN_DARK_THEME),
        (CLEAN_LIGHT_THEME, CLEAN_DARK_THEME),
    ] {
        let a = preset(light);
        let b = preset(dark);
        for size in ["xs", "sm", "md", "lg"] {
            assert_eq!(
                a.tokens.length_snapshot(&sized(size)),
                b.tokens.length_snapshot(&sized(size))
            );
            assert_eq!(
                a.tokens.typography_snapshot(&sized(size)),
                b.tokens.typography_snapshot(&sized(size))
            );
        }
        assert_ne!(a.tokens.color_snapshot(), b.tokens.color_snapshot());
    }
    assert_ne!(
        preset(MODERN_LIGHT_THEME)
            .tokens
            .length_snapshot(&sized("md")),
        preset(CLEAN_LIGHT_THEME)
            .tokens
            .length_snapshot(&sized("md"))
    );
}

#[test]
fn command_presets_keep_local_chrome_roles_and_original_soft_inks() {
    for &(name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let theme = preset(source);
        let clean = name.starts_with("clean");
        let dark = name.contains("dark");
        let parts = theme
            .tokens
            .resolve_component_styles(&ModuleId::parse("components/command").unwrap(), &Environment::EMPTY)
            .unwrap();
        let rule = |part: &str| resolved_recipe(parts[part].clone(), &theme);
        assert_eq!(
            rule("root").base.radii.top_left,
            Some(Length::Pixels(if clean { 12.0 } else { 20.0 }))
        );
        assert_eq!(
            rule("root").base.padding.top,
            Some(Length::Pixels(if clean { 0.0 } else { 4.0 }))
        );
        assert_eq!(
            rule("card").base.radii.top_left,
            Some(Length::Pixels(if clean { 0.0 } else { 16.0 }))
        );
        assert_eq!(
            rule("search").base.height,
            Some(Length::Pixels(if clean { 48.0 } else { 52.0 }).into())
        );
        assert_eq!(
            rule("search").base.border_widths.bottom,
            Some(Length::Pixels(if clean { 0.5 } else { 1.0 }))
        );
        for part in ["root_embedded", "card_embedded"] {
            assert_eq!(
                rule(part)
                    .base
                    .background
                    .as_deref()
                    .and_then(BackgroundFill::as_solid_value),
                Some(&ColorValue::Literal(Rgba8::from_rgba_hex(0)))
            );
            assert_eq!(rule(part).base.shadows, Some(vec![]));
        }
        let rgba = |key: &str| theme.tokens.color(key).unwrap().as_rgba_hex();
        assert_eq!(
            rgba("command.surface"),
            if dark {
                if clean { 0x1c1c_1eff } else { 0x1919_1bff }
            } else {
                0xffff_ffff
            }
        );
        assert_eq!(
            rgba("command.selection"),
            if clean {
                if dark { 0xffff_ff14 } else { 0x0000_000f }
            } else if dark {
                0x2626_2aff
            } else {
                0xf0f0_f1ff
            }
        );
        assert_eq!(
            rgba("command.placeholder"),
            if clean {
                if dark { 0xffff_ff4d } else { 0x0000_0045 }
            } else if dark {
                0x5c5c_63ff
            } else {
                0xb5b5_b5ff
            }
        );
        assert_eq!(
            rule("item")
                .hover
                .unwrap()
                .background
                .as_deref()
                .and_then(BackgroundFill::as_solid_value),
            Some(&ColorValue::Literal(Rgba8::from_rgba_hex(0))),
            "rejected controlled roving must not paint a second independent highlight"
        );
        let source = theme
            .tokens
            .token("command.search_asset")
            .and_then(|value| value.resolve_string(&Environment::EMPTY, &theme.tokens.environment))
            .unwrap();
        assert_eq!(
            source,
            format!(
                "app/icons/command_search_{}",
                if clean { "clean" } else { "modern" }
            )
        );
        let query = theme
            .tokens
            .resolve_typography("command_search", &Environment::EMPTY)
            .unwrap();
        assert_eq!(query.size, Length::Pixels(if clean { 13.0 } else { 15.0 }));
    }
}

#[test]
fn command_presets_keep_glyph_and_logo_geometry_with_original_hint_ink() {
    for &(name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let theme = preset(source);
        let clean = name.starts_with("clean");
        let dark = name.contains("dark");
        let parts = theme
            .tokens
            .resolve_component_styles(&ModuleId::parse("components/command").unwrap(), &Environment::EMPTY)
            .unwrap();
        let rule = |part: &str| resolved_recipe(parts[part].clone(), &theme);
        let rgba = |key: &str| theme.tokens.color(key).unwrap().as_rgba_hex();
        assert_eq!(
            rule("icon_slot").base.width,
            Some(Length::Pixels(if clean { 20.0 } else { 26.0 }).into())
        );
        assert_eq!(rule("icon_slot").base.clip, Some(true));
        assert_eq!(
            rule("icon").base.width,
            Some(Length::Pixels(if clean { 16.0 } else { 17.0 }).into())
        );
        assert_eq!(
            rule("logo").base.width,
            Some(Length::Pixels(if clean { 12.0 } else { 14.0 }).into())
        );
        assert_eq!(
            rule("logo_slot").base.radii.top_left,
            Some(Length::Pixels(if clean { 5.0 } else { 7.0 }))
        );
        assert_eq!(
            rgba("command.logo_surface"),
            if dark {
                if clean { 0x2a2a_2dff } else { 0x2626_2aff }
            } else {
                0xffff_ffff
            }
        );
        assert_eq!(
            rgba("command.hint"),
            if clean {
                if dark { 0xffff_ff75 } else { 0x0000_0073 }
            } else if dark {
                0x8b8b_92ff
            } else {
                0x8f8f_8fff
            }
        );
    }
}

#[test]
fn table_presets_keep_their_own_surfaces_grid_and_header_action() {
    for &(name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let theme = preset(source);
        let clean = name.starts_with("clean");
        let dark = name.contains("dark");
        let parts = theme
            .tokens
            .resolve_component_styles(&ModuleId::parse("components/table").unwrap(), &Environment::EMPTY)
            .unwrap();
        let rule = |part: &str| resolved_recipe(parts[part].clone(), &theme);
        assert_eq!(
            rule("root").base.radii.top_left,
            Some(Length::Pixels(if clean { 10.0 } else { 16.0 }))
        );
        assert_eq!(
            rule("root").base.border_widths.top,
            Some(Length::Pixels(if clean { 0.0 } else { 1.0 }))
        );
        assert_eq!(
            rule("header_action").base.height,
            Some(Length::Pixels(if clean { 28.0 } else { 32.0 }).into())
        );
        assert!(
            rule("header_action").base.text_color.is_none(),
            "{name}: inherit the th's type/ink instead of overriding Host header-cell styling"
        );
        assert_eq!(
            rule("cell").base.border_widths.top,
            Some(Length::Pixels(if clean { 0.5 } else { 1.0 }))
        );
        assert_eq!(
            rule("cell").base.border_widths.start,
            Some(Length::Pixels(if clean { 0.0 } else { 1.0 }))
        );
        let header = theme
            .tokens
            .resolve_typography("table_header", &Environment::EMPTY)
            .unwrap();
        assert_eq!(header.size, Length::Pixels(if clean { 12.0 } else { 14.0 }));
        assert_eq!(header.weight, if clean { 500 } else { 450 });
        let rgba = |key: &str| theme.tokens.color(key).unwrap().as_rgba_hex();
        assert_eq!(
            rgba("table.surface"),
            if dark {
                if clean { 0x1c1c_1eff } else { 0x2323_26ff }
            } else {
                0xffff_ffff
            }
        );
        if !clean {
            assert_eq!(rgba("table.header_ink"), 0x8f8f_8fff);
            assert_eq!(
                rgba("table.text"),
                if dark { 0xc4c4_c4ff } else { 0x4040_40ff }
            );
        }
        if name == "modern_light.rhai" {
            assert_eq!(rgba("table.selection"), 0xf5f8_ffff);
            assert_ne!(rgba("table.selection"), rgba("collection.selection"));
            assert!(
                theme
                    .tokens
                    .color("table.header_ink")
                    .unwrap()
                    .contrast_ratio(theme.tokens.color("table.surface").unwrap())
                    < 4.5,
                "the original header-color risk must not be silently recolored"
            );
        }
    }
}

#[test]
fn compact_is_an_orthogonal_geometry_preference_not_a_smaller_type_or_design_alias() {
    for &(name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let theme = preset(source);
        for size in ["xs", "sm", "md", "lg"] {
            let comfortable = sized(size);
            let compact = comfortable
                .with(
                    gpui_rhai::Symbol::intern("density"),
                    gpui_rhai::Symbol::intern("compact"),
                )
                .unwrap();
            assert_eq!(
                theme.tokens.typography_snapshot(&comfortable),
                theme.tokens.typography_snapshot(&compact),
                "{name}/{size}: density must not change type"
            );
            let read = |path: &str, env: &Environment| match theme
                .tokens
                .resolve_length(Length::token(path).unwrap(), env)
                .unwrap()
            {
                Length::Pixels(value) => value,
                _ => panic!("pixel metric"),
            };
            assert_eq!(
                read("metrics.mark", &comfortable),
                read("metrics.mark", &compact)
            );
            assert_eq!(
                read("metrics.icon", &comfortable),
                read("metrics.icon", &compact)
            );
            assert!(read("metrics.row", &compact) < read("metrics.row", &comfortable));
            assert!(read("metrics.table_row", &compact) < read("metrics.table_row", &comfortable));
            assert!(read("metrics.field_h", &compact) >= 24.0);
            assert!(read("metrics.button_h", &compact) <= read("metrics.button_h", &comfortable));
            assert_eq!(
                read("metrics.tabs_track_total_h", &compact),
                read("metrics.tabs_track_h", &compact)
                    + 2.0 * read("metrics.segment_pad", &compact)
            );
            assert_eq!(
                read("metrics.segment_radius", &compact),
                read("metrics.segment_thumb_radius", &compact)
                    + read("metrics.segment_pad", &compact)
            );
        }
    }
}

#[test]
fn every_public_module_has_an_explicit_preset_recipe() {
    let modules = BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .chain(BUNDLED_PATTERN_SOURCES_BY_ID)
        .chain(BUNDLED_MOTION_SOURCES_BY_ID)
        .chain(BUNDLED_CHART_SOURCES_BY_ID)
        .map(|(id, _)| ModuleId::parse(*id).unwrap())
        .collect::<BTreeSet<_>>();
    for source in [
        MODERN_LIGHT_THEME,
        MODERN_DARK_THEME,
        CLEAN_LIGHT_THEME,
        CLEAN_DARK_THEME,
    ] {
        let theme = preset(source);
        let covered = theme
            .tokens
            .component_styles
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(covered, modules, "{}/{}", theme.family, theme.name);
    }
}

#[test]
fn scoped_recipe_reads_follow_theme_switches_and_preserve_symbolic_lengths() {
    let modern = preset(MODERN_LIGHT_THEME);
    let clean = preset(CLEAN_LIGHT_THEME);
    let mut state = UiRuntimeState::new();
    state.theme = Some(
        ThemeManager::from_variants([modern, clean], ThemeSelection::new("Modern", "Light"))
            .unwrap(),
    );
    let runtime = Rc::new(RefCell::new(state));
    let context = UiContext::new(
        runtime.clone(),
        ComponentInstancePath::root("App", "preset"),
        None,
        ExecutionPhase::Event,
        BTreeMap::new(),
    );
    let modern_recipe = context.theme_style("button.primary").unwrap();
    assert!(matches!(modern_recipe.base.background.as_deref(),
        Some(BackgroundFill::Layers(stack)) if stack.layers().len() == 2));
    let length = context
        .theme_token("metrics.button_h")
        .unwrap()
        .cast::<Length>();
    assert!(length.is_theme_token());
    context.set_theme("Clean", "Light").unwrap();
    let clean_recipe = context.theme_style("button.primary").unwrap();
    assert!(matches!(
        clean_recipe.base.background.as_deref(),
        Some(BackgroundFill::Solid(_))
    ));
    assert_ne!(modern_recipe, clean_recipe);
    assert!(context.theme_style("button.missing").is_err());
}

#[test]
fn complete_presets_export_without_losing_geometry_type_or_component_states() {
    let engine = RuntimeEngine::new();
    for (name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let original = preset(source);
        let exported = gpui_rhai::export_theme_source(&original)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let reloaded = gpui_rhai::load_theme_source(engine.engine(), name, &exported).unwrap();
        assert_eq!(reloaded, original, "{name}");
        assert_eq!(gpui_rhai::export_theme_source(&reloaded).unwrap(), exported);
    }
}

#[test]
fn design_axis_is_orthogonal_across_all_color_palettes() {
    let neutral_dark = preset(gpui_rhai_registry::NEUTRAL_DARK_THEME);
    let mocha = preset(gpui_rhai_registry::CATPPUCCIN_MOCHA_THEME);
    let default_light = preset(gpui_rhai_registry::DEFAULT_LIGHT_THEME);
    let mut state = UiRuntimeState::new();
    state.theme = Some(
        ThemeManager::from_variants(
            [neutral_dark, mocha, default_light],
            ThemeSelection::new("Catppuccin", "Mocha"),
        )
        .unwrap(),
    );
    let runtime = Rc::new(RefCell::new(state));
    let context = UiContext::new(
        runtime,
        ComponentInstancePath::root("App", "orthogonal"),
        None,
        ExecutionPhase::Event,
        BTreeMap::new(),
    );
    assert_eq!(context.environment_value("design").as_deref(), Some("modern"));
    assert_eq!(
        context.theme_token("design.name").unwrap().into_string().unwrap(),
        "modern"
    );
    let modern_btn = context.theme_style("button.primary").unwrap();
    context.set_environment("design", "clean").unwrap();
    assert_eq!(context.environment_value("design").as_deref(), Some("clean"));
    assert_eq!(
        context.theme_token("design.name").unwrap().into_string().unwrap(),
        "clean"
    );
    let clean_btn = context.theme_style("button.primary").unwrap();
    assert_ne!(modern_btn, clean_btn);
    // Switching palette preserves the orthogonal `design = clean` choice.
    context.set_theme("Default", "Light").unwrap();
    assert_eq!(context.environment_value("design").as_deref(), Some("clean"));
    assert_eq!(
        context.theme_token("design.name").unwrap().into_string().unwrap(),
        "clean"
    );
}
