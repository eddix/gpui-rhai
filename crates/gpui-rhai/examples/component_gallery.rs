use std::collections::BTreeMap;

use gpui_rhai::{
    AssetData, EmbeddedScriptSource, EmbeddedScriptView, ModuleId, RuntimeEngine,
    ScriptApplication, ThemeVariant, load_theme_source,
};

const STUDIO: &str = include_str!("../../../registry/studio/theme_studio.rhai");
const EN: &str = include_str!("../../../registry/locales/en.rhai");
const AR: &str = include_str!("../../../registry/locales/ar.rhai");
const ZH_CN: &str = include_str!("../../../registry/locales/zh_cn.rhai");

const THEMES: &[(&str, &str)] = &[
    (
        "default_dark.rhai",
        include_str!("../../../registry/themes/default_dark.rhai"),
    ),
    (
        "default_light.rhai",
        include_str!("../../../registry/themes/default_light.rhai"),
    ),
    (
        "tokyo_night.rhai",
        include_str!("../../../registry/themes/tokyo_night.rhai"),
    ),
    (
        "tokyo_storm.rhai",
        include_str!("../../../registry/themes/tokyo_storm.rhai"),
    ),
    (
        "catppuccin_latte.rhai",
        include_str!("../../../registry/themes/catppuccin_latte.rhai"),
    ),
    (
        "catppuccin_mocha.rhai",
        include_str!("../../../registry/themes/catppuccin_mocha.rhai"),
    ),
    (
        "ethereal.rhai",
        include_str!("../../../registry/themes/ethereal.rhai"),
    ),
    (
        "everforest.rhai",
        include_str!("../../../registry/themes/everforest.rhai"),
    ),
    (
        "gruvbox.rhai",
        include_str!("../../../registry/themes/gruvbox.rhai"),
    ),
    (
        "hackerman.rhai",
        include_str!("../../../registry/themes/hackerman.rhai"),
    ),
    (
        "nord.rhai",
        include_str!("../../../registry/themes/nord.rhai"),
    ),
    (
        "retro_82.rhai",
        include_str!("../../../registry/themes/retro_82.rhai"),
    ),
    (
        "hermarchy.rhai",
        include_str!("../../../registry/themes/hermarchy.rhai"),
    ),
    (
        "futurism.rhai",
        include_str!("../../../registry/themes/futurism.rhai"),
    ),
    (
        "aetheria.rhai",
        include_str!("../../../registry/themes/aetheria.rhai"),
    ),
];

const COLOR_TOKENS: &[&str] = &[
    "surface",
    "surface_raised",
    "surface_hover",
    "text_primary",
    "text_muted",
    "accent",
    "accent_hover",
    "on_accent",
    "danger",
    "on_danger",
    "warning",
    "on_warning",
    "success",
    "on_success",
    "border",
    "focus_ring",
    "selection",
    "disabled",
];

macro_rules! component {
    ($id:literal, $file:literal) => {
        (
            ModuleId::parse(concat!("components/", $id)).expect("static component module ID"),
            include_str!(concat!("../../../registry/components/", $file, ".rhai")).to_owned(),
        )
    };
}

fn scripts(main: &str) -> EmbeddedScriptSource {
    EmbeddedScriptSource::new(BTreeMap::from([
        (ModuleId::parse("main").unwrap(), main.to_owned()),
        component!("accordion", "accordion"),
        component!("alert", "alert"),
        component!("alert_dialog", "alert_dialog"),
        component!("avatar", "avatar"),
        component!("badge", "badge"),
        component!("button", "button"),
        component!("button_group", "button_group"),
        component!("card", "card"),
        component!("checkbox", "checkbox"),
        component!("collapsible", "collapsible"),
        component!("combobox", "combobox"),
        component!("command", "command"),
        component!("command_dialog", "command_dialog"),
        component!("context_menu", "context_menu"),
        component!("date_picker", "date_picker"),
        component!("dialog", "dialog"),
        component!("divider", "divider"),
        component!("empty", "empty"),
        component!("form_field", "form_field"),
        component!("group_box", "group_box"),
        component!("icon", "icon"),
        component!("input", "input"),
        component!("input_group", "input_group"),
        component!("kbd", "kbd"),
        component!("label", "label"),
        component!("menu", "menu"),
        component!("pagination", "pagination"),
        component!("popover", "popover"),
        component!("progress", "progress"),
        component!("radio", "radio"),
        component!("radio_group", "radio_group"),
        component!("scroll_area", "scroll_area"),
        component!("select", "select"),
        component!("sheet", "sheet"),
        component!("skeleton", "skeleton"),
        component!("slider", "slider"),
        component!("spinner", "spinner"),
        component!("switch", "switch"),
        component!("table", "table"),
        component!("tabs", "tabs"),
        component!("tag", "tag"),
        component!("textarea", "textarea"),
        component!("toast", "toast"),
        component!("toggle", "toggle"),
        component!("toggle_group", "toggle_group"),
        component!("tooltip", "tooltip"),
    ]))
}

fn json(value: &str) -> String {
    serde_json::to_string(value).expect("static gallery strings serialize")
}

fn color_source(theme: &ThemeVariant, token: &str) -> String {
    json(&format!(
        "#{:08x}",
        theme.tokens.colors[token].as_rgba_hex()
    ))
}

fn theme_entry(slug: &str) -> (&'static str, &'static str) {
    let normalized = slug.replace('-', "_");
    THEMES
        .iter()
        .copied()
        .find(|(name, _)| name.strip_suffix(".rhai") == Some(normalized.as_str()))
        .unwrap_or(THEMES[0])
}

fn gallery_source(category: &str, theme_slug: &str, locale: &str, visual_state: &str) -> String {
    let engine = RuntimeEngine::new();
    let (theme_file, theme_source) = theme_entry(theme_slug);
    let theme = load_theme_source(engine.engine(), theme_file, theme_source)
        .expect("bundled gallery theme is valid");
    let selected = theme_file.strip_suffix(".rhai").unwrap_or("default_dark");
    let locale = if matches!(locale, "en" | "ar" | "zh-CN") {
        locale
    } else {
        "en"
    };
    let mut source = STUDIO.to_owned();
    for (placeholder, value) in [
        ("__PATH__", json("")),
        ("__FAMILY__", json(&theme.family)),
        ("__VARIANT__", json(&theme.name)),
        ("__MODE__", json("dark")),
        ("__STATUS__", json("Component Gallery")),
        ("__PREVIEW_FAMILY__", json(&theme.family)),
        ("__PREVIEW_VARIANT__", json(&theme.name)),
        ("__GALLERY_THEME__", json(selected)),
        ("__GALLERY_CATEGORY__", json(category)),
    ] {
        source = source.replace(placeholder, &value);
    }
    for token in COLOR_TOKENS {
        source = source.replace(
            &format!("__COLOR_{}__", token.to_ascii_uppercase()),
            &color_source(&theme, token),
        );
    }
    source
        .replace("__BUILTIN_OPTIONS__", "[]")
        .replace(
            "__VISUAL_DIALOG__",
            if visual_state == "dialog" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__VISUAL_POPOVER__",
            if visual_state == "popover" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__VISUAL_COMMAND__",
            if visual_state == "command-dialog" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__VISUAL_SHEET__",
            if visual_state == "sheet" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__VISUAL_ALERT__",
            if visual_state == "alert-dialog" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__VISUAL_MENU__",
            if visual_state == "menu" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__VISUAL_TOAST__",
            if visual_state == "toast" {
                "true"
            } else {
                "false"
            },
        )
        .replace("__VISUAL_LOCALE__", &json(locale))
        .replace("__GALLERY_ONLY__", "true")
}

fn svg(bytes: &'static [u8]) -> AssetData {
    AssetData {
        mime_type: "image/svg+xml".to_owned(),
        bytes: bytes.to_vec(),
    }
}

pub(crate) fn prepared(
    category: &str,
) -> Result<gpui_rhai::PreparedScriptView, gpui_rhai::ScriptViewError> {
    prepared_with_environment(category, "default_dark", "en", "default")
}

fn prepared_with_environment(
    category: &str,
    theme_slug: &str,
    locale: &str,
    visual_state: &str,
) -> Result<gpui_rhai::PreparedScriptView, gpui_rhai::ScriptViewError> {
    let (theme_file, theme_source) = theme_entry(theme_slug);
    let main = gallery_source(category, theme_slug, locale, visual_state);
    EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        scripts(&main),
        theme_source,
    )
    .theme_sources(
        THEMES
            .iter()
            .filter(|(name, _)| *name != theme_file)
            .map(|(name, source)| ((*name).to_owned(), (*source).to_owned())),
    )
    .locale_sources([
        ("en.rhai".to_owned(), EN.to_owned()),
        ("ar.rhai".to_owned(), AR.to_owned()),
        ("zh_cn.rhai".to_owned(), ZH_CN.to_owned()),
    ])
    .asset_sources([
        (
            "icons/check".to_owned(),
            svg(include_bytes!("../../../registry/assets/icons/check.svg")),
        ),
        (
            "icons/close".to_owned(),
            svg(include_bytes!("../../../registry/assets/icons/close.svg")),
        ),
        (
            "icons/chevron_left".to_owned(),
            svg(include_bytes!(
                "../../../registry/assets/icons/chevron_left.svg"
            )),
        ),
        (
            "icons/chevron_right".to_owned(),
            svg(include_bytes!(
                "../../../registry/assets/icons/chevron_right.svg"
            )),
        ),
        (
            "icons/calendar".to_owned(),
            svg(include_bytes!(
                "../../../registry/assets/icons/calendar.svg"
            )),
        ),
        (
            "icons/date_previous".to_owned(),
            svg(include_bytes!(
                "../../../registry/assets/icons/date_previous.svg"
            )),
        ),
        (
            "icons/date_next".to_owned(),
            svg(include_bytes!(
                "../../../registry/assets/icons/date_next.svg"
            )),
        ),
    ])
    .prepare()
}

fn gallery_category(explicit: Option<String>, visual_state: &str) -> String {
    explicit
        .filter(|category| {
            matches!(
                category.as_str(),
                "all" | "foundations" | "forms" | "navigation" | "overlays"
            )
        })
        .or_else(|| {
            matches!(
                visual_state,
                "all" | "foundations" | "forms" | "navigation" | "overlays"
            )
            .then(|| visual_state.to_owned())
        })
        .unwrap_or_else(|| "all".to_owned())
}

fn main() {
    let visual_state =
        std::env::var("GPUI_RHAI_VISUAL_STATE").unwrap_or_else(|_| "default".to_owned());
    let category = gallery_category(
        std::env::var("GPUI_RHAI_GALLERY_CATEGORY").ok(),
        &visual_state,
    );
    let theme =
        std::env::var("GPUI_RHAI_VISUAL_THEME").unwrap_or_else(|_| "default-dark".to_owned());
    let locale = std::env::var("GPUI_RHAI_VISUAL_LOCALE").unwrap_or_else(|_| "en".to_owned());
    let prepared = if theme == "default-dark" && locale == "en" && visual_state == "default" {
        prepared(&category)
    } else {
        prepared_with_environment(&category, &theme, &locale, &visual_state)
    };
    let (width, height) = match visual_state.as_str() {
        "compact" => (560.0, 760.0),
        "regular" => (900.0, 800.0),
        _ => (1280.0, 860.0),
    };
    ScriptApplication::new(prepared.expect("component gallery prepares"))
        .window_size(width, height)
        .run()
        .expect("component gallery runs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_gallery_category_prepares_with_all_official_sources() {
        for category in ["all", "foundations", "forms", "navigation", "overlays"] {
            prepared(category).unwrap_or_else(|error| panic!("{category}: {error}"));
        }
        for theme in ["default-light", "catppuccin-mocha", "nord"] {
            prepared_with_environment("foundations", theme, "en", "default")
                .unwrap_or_else(|error| panic!("{theme}: {error}"));
        }
        prepared_with_environment("forms", "catppuccin-mocha", "ar", "default").unwrap();
        prepared_with_environment("overlays", "tokyo-night", "en", "sheet").unwrap();
        assert_eq!(
            gpui_rhai::ScriptSource::module_ids(&scripts(&gallery_source(
                "all",
                "default_dark",
                "en",
                "default"
            )))
            .len(),
            47
        );
        assert_eq!(gallery_category(None, "forms"), "forms");
        assert_eq!(
            gallery_category(Some("navigation".to_owned()), "compact"),
            "navigation"
        );
        assert_eq!(
            gallery_category(Some("unknown".to_owned()), "default"),
            "all"
        );
    }
}
