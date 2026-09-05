use std::collections::BTreeMap;

use gpui_rhai::{
    AssetData, EmbeddedScriptSource, EmbeddedScriptView, ModuleId, RuntimeEngine,
    ScriptApplication, ThemeVariant, load_theme_source,
};

const STUDIO: &str = include_str!("../../../registry/studio/theme_studio.rhai");
const DEFAULT_DARK: &str = include_str!("../../../registry/themes/default_dark.rhai");
const EN: &str = include_str!("../../../registry/locales/en.rhai");

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

fn gallery_source(category: &str) -> String {
    let engine = RuntimeEngine::new();
    let theme = load_theme_source(engine.engine(), "default_dark.rhai", DEFAULT_DARK)
        .expect("bundled default theme is valid");
    let mut source = STUDIO.to_owned();
    for (placeholder, value) in [
        ("__PATH__", json("")),
        ("__FAMILY__", json(&theme.family)),
        ("__VARIANT__", json(&theme.name)),
        ("__MODE__", json("dark")),
        ("__STATUS__", json("Component Gallery")),
        ("__PREVIEW_FAMILY__", json(&theme.family)),
        ("__PREVIEW_VARIANT__", json(&theme.name)),
        ("__GALLERY_THEME__", json("default_dark")),
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
        .replace("__VISUAL_MENU__", "false")
        .replace("__VISUAL_TOAST__", "false")
        .replace("__VISUAL_LOCALE__", &json("en"))
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
    let main = gallery_source(category);
    EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        scripts(&main),
        DEFAULT_DARK,
    )
    .theme_sources(
        THEMES
            .iter()
            .skip(1)
            .map(|(name, source)| ((*name).to_owned(), (*source).to_owned())),
    )
    .locale_sources([("en.rhai".to_owned(), EN.to_owned())])
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

fn main() {
    let category = std::env::var("GPUI_RHAI_GALLERY_CATEGORY")
        .ok()
        .filter(|category| {
            matches!(
                category.as_str(),
                "all" | "foundations" | "forms" | "navigation" | "overlays"
            )
        })
        .unwrap_or_else(|| "all".to_owned());
    ScriptApplication::new(prepared(&category).expect("component gallery prepares"))
        .window_size(1280.0, 860.0)
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
        assert_eq!(
            gpui_rhai::ScriptSource::module_ids(&scripts(&gallery_source("all"))).len(),
            47
        );
    }
}
