//! Embedded sources for the built-in default skin.
//!
//! The `ui/` directory is the editable source of truth; this module freezes it
//! into the binary so a stock `disktree-rhai` run needs no files on disk.
//! `--export-skin` writes these same sources out for user modification.
//!
//! Since the 0.2 migration the skin rides the official component set: the
//! token base (`tokens.rhai`) sits under the palettes, and the chrome mounts
//! the copied `components/` + `layouts/` + `patterns/` sources. Assets are the
//! SVGs official components declare (checkbox marks).

use std::collections::BTreeMap;

use gpui_rhai::{AppManifest, AssetData, EmbeddedScriptSource, ModuleId};

pub const THEME: &str = include_str!("../ui/theme.rhai");
pub const FLEXOKI_LIGHT: &str = include_str!("../ui/themes/flexoki_light.rhai");
pub const TOKENS: &str = include_str!("../ui/tokens.rhai");
pub const EN_LOCALE: &str = include_str!("../ui/locales/en.rhai");
pub const MAIN: &str = include_str!("../ui/main.rhai");
pub const CORE: &str = include_str!("../ui/core.rhai");
pub const APP_MANIFEST_TOML: &str = include_str!("../ui/app.toml");

pub const COMPONENT_BUTTON: &str = include_str!("../ui/components/button.rhai");
pub const COMPONENT_BADGE: &str = include_str!("../ui/components/badge.rhai");
pub const COMPONENT_CHECKBOX: &str = include_str!("../ui/components/checkbox.rhai");
pub const COMPONENT_DIVIDER: &str = include_str!("../ui/components/divider.rhai");
pub const COMPONENT_KBD: &str = include_str!("../ui/components/kbd.rhai");
pub const COMPONENT_PROGRESS: &str = include_str!("../ui/components/progress.rhai");
pub const COMPONENT_SPINNER: &str = include_str!("../ui/components/spinner.rhai");
pub const COMPONENT_STATUS_BAR: &str = include_str!("../ui/components/status_bar.rhai");
pub const COMPONENT_TOGGLE_GROUP: &str = include_str!("../ui/components/toggle_group.rhai");
pub const COMPONENT_SELECT: &str = include_str!("../ui/components/select.rhai");
pub const COMPONENT_COMBOBOX: &str = include_str!("../ui/components/combobox.rhai");
pub const COMPONENT_INPUT: &str = include_str!("../ui/components/input.rhai");
pub const COMPONENT_SPLIT_PANE: &str = include_str!("../ui/components/split_pane.rhai");
pub const COMPONENT_POPOVER: &str = include_str!("../ui/components/popover.rhai");
pub const COMPONENT_TITLE_BAR: &str = include_str!("../ui/components/title_bar.rhai");
pub const LAYOUT_TOOLBAR: &str = include_str!("../ui/layouts/toolbar.rhai");
pub const LAYOUT_INLINE: &str = include_str!("../ui/layouts/inline.rhai");
pub const PATTERN_DESCRIPTION_LIST: &str = include_str!("../ui/patterns/description_list.rhai");
pub const PATTERN_SECTION: &str = include_str!("../ui/patterns/section.rhai");
pub const PATTERN_STAT: &str = include_str!("../ui/patterns/stat.rhai");

pub const CHECK_SVG: &[u8] = include_bytes!("../ui/assets/icons/check.svg");
pub const MINUS_SVG: &[u8] = include_bytes!("../ui/assets/icons/minus.svg");
pub const CLOSE_SVG: &[u8] = include_bytes!("../ui/assets/icons/close.svg");
pub const CHEVRON_DOWN_SVG: &[u8] = include_bytes!("../ui/assets/icons/chevron_down.svg");
pub const CHEVRON_UP_SVG: &[u8] = include_bytes!("../ui/assets/icons/chevron_up.svg");

/// gpui-rhai's bundled palettes, mounted as extra variants of the skin. The
/// first two skin palettes (theme.rhai, flexoki_light) stay the defaults.
macro_rules! bundled_themes {
    ($(($const_name:ident, $file:literal)),+ $(,)?) => {
        $(pub const $const_name: &str = include_str!(concat!("../ui/themes/", $file));)+
    };
}
bundled_themes!(
    (AETHERIA, "aetheria.rhai"),
    (CATPPUCCIN_LATTE, "catppuccin_latte.rhai"),
    (CATPPUCCIN_MOCHA, "catppuccin_mocha.rhai"),
    (DEFAULT_DARK, "default_dark.rhai"),
    (DEFAULT_LIGHT, "default_light.rhai"),
    (ETHEREAL, "ethereal.rhai"),
    (EVERFOREST, "everforest.rhai"),
    (FUTURISM, "futurism.rhai"),
    (GRUVBOX, "gruvbox.rhai"),
    (HACKERMAN, "hackerman.rhai"),
    (HERMARCHY, "hermarchy.rhai"),
    (NORD, "nord.rhai"),
    (RETRO_82, "retro_82.rhai"),
    (TOKYO_NIGHT, "tokyo_night.rhai"),
    (TOKYO_STORM, "tokyo_storm.rhai"),
);

/// Every bundled palette as `(themes/ file name, source)`, for
/// `EmbeddedScriptView::theme_sources`.
pub fn bundled_palette_sources() -> Vec<(String, &'static str)> {
    vec![
        ("aetheria.rhai", AETHERIA),
        ("catppuccin_latte.rhai", CATPPUCCIN_LATTE),
        ("catppuccin_mocha.rhai", CATPPUCCIN_MOCHA),
        ("default_dark.rhai", DEFAULT_DARK),
        ("default_light.rhai", DEFAULT_LIGHT),
        ("ethereal.rhai", ETHEREAL),
        ("everforest.rhai", EVERFOREST),
        ("futurism.rhai", FUTURISM),
        ("gruvbox.rhai", GRUVBOX),
        ("hackerman.rhai", HACKERMAN),
        ("hermarchy.rhai", HERMARCHY),
        ("nord.rhai", NORD),
        ("retro_82.rhai", RETRO_82),
        ("tokyo_night.rhai", TOKYO_NIGHT),
        ("tokyo_storm.rhai", TOKYO_STORM),
    ]
    .into_iter()
    .map(|(name, source)| (format!("themes/{name}"), source))
    .collect()
}

/// Every file that makes up the default skin, as `(relative path, contents)`.
/// `--export-skin` writes exactly this set.
pub const SKIN_FILES: &[(&str, &[u8])] = &[
    ("app.toml", APP_MANIFEST_TOML.as_bytes()),
    ("tokens.rhai", TOKENS.as_bytes()),
    ("main.rhai", MAIN.as_bytes()),
    ("core.rhai", CORE.as_bytes()),
    ("components/button.rhai", COMPONENT_BUTTON.as_bytes()),
    ("components/badge.rhai", COMPONENT_BADGE.as_bytes()),
    ("components/checkbox.rhai", COMPONENT_CHECKBOX.as_bytes()),
    ("components/divider.rhai", COMPONENT_DIVIDER.as_bytes()),
    ("components/kbd.rhai", COMPONENT_KBD.as_bytes()),
    ("components/progress.rhai", COMPONENT_PROGRESS.as_bytes()),
    ("components/spinner.rhai", COMPONENT_SPINNER.as_bytes()),
    (
        "components/status_bar.rhai",
        COMPONENT_STATUS_BAR.as_bytes(),
    ),
    (
        "components/toggle_group.rhai",
        COMPONENT_TOGGLE_GROUP.as_bytes(),
    ),
    ("components/select.rhai", COMPONENT_SELECT.as_bytes()),
    ("components/combobox.rhai", COMPONENT_COMBOBOX.as_bytes()),
    ("components/input.rhai", COMPONENT_INPUT.as_bytes()),
    (
        "components/split_pane.rhai",
        COMPONENT_SPLIT_PANE.as_bytes(),
    ),
    ("components/popover.rhai", COMPONENT_POPOVER.as_bytes()),
    ("components/title_bar.rhai", COMPONENT_TITLE_BAR.as_bytes()),
    ("layouts/toolbar.rhai", LAYOUT_TOOLBAR.as_bytes()),
    ("layouts/inline.rhai", LAYOUT_INLINE.as_bytes()),
    (
        "patterns/description_list.rhai",
        PATTERN_DESCRIPTION_LIST.as_bytes(),
    ),
    ("patterns/section.rhai", PATTERN_SECTION.as_bytes()),
    ("patterns/stat.rhai", PATTERN_STAT.as_bytes()),
    ("theme.rhai", THEME.as_bytes()),
    ("themes/flexoki_light.rhai", FLEXOKI_LIGHT.as_bytes()),
    ("locales/en.rhai", EN_LOCALE.as_bytes()),
    ("assets/icons/check.svg", CHECK_SVG),
    ("assets/icons/minus.svg", MINUS_SVG),
    ("assets/icons/close.svg", CLOSE_SVG),
    ("assets/icons/chevron_down.svg", CHEVRON_DOWN_SVG),
    ("assets/icons/chevron_up.svg", CHEVRON_UP_SVG),
    ("themes/aetheria.rhai", AETHERIA.as_bytes()),
    ("themes/catppuccin_latte.rhai", CATPPUCCIN_LATTE.as_bytes()),
    ("themes/catppuccin_mocha.rhai", CATPPUCCIN_MOCHA.as_bytes()),
    ("themes/default_dark.rhai", DEFAULT_DARK.as_bytes()),
    ("themes/default_light.rhai", DEFAULT_LIGHT.as_bytes()),
    ("themes/ethereal.rhai", ETHEREAL.as_bytes()),
    ("themes/everforest.rhai", EVERFOREST.as_bytes()),
    ("themes/futurism.rhai", FUTURISM.as_bytes()),
    ("themes/gruvbox.rhai", GRUVBOX.as_bytes()),
    ("themes/hackerman.rhai", HACKERMAN.as_bytes()),
    ("themes/hermarchy.rhai", HERMARCHY.as_bytes()),
    ("themes/nord.rhai", NORD.as_bytes()),
    ("themes/retro_82.rhai", RETRO_82.as_bytes()),
    ("themes/tokyo_night.rhai", TOKYO_NIGHT.as_bytes()),
    ("themes/tokyo_storm.rhai", TOKYO_STORM.as_bytes()),
];

pub fn entry() -> ModuleId {
    ModuleId::parse("main").expect("static module id")
}

fn module(id: &str, source: &str) -> (ModuleId, String) {
    (
        ModuleId::parse(id).expect("static module id"),
        source.to_owned(),
    )
}

pub fn sources() -> EmbeddedScriptSource {
    EmbeddedScriptSource::new(BTreeMap::from([
        module("main", MAIN),
        module("core", CORE),
        module("components/button", COMPONENT_BUTTON),
        module("components/badge", COMPONENT_BADGE),
        module("components/checkbox", COMPONENT_CHECKBOX),
        module("components/divider", COMPONENT_DIVIDER),
        module("components/kbd", COMPONENT_KBD),
        module("components/progress", COMPONENT_PROGRESS),
        module("components/spinner", COMPONENT_SPINNER),
        module("components/status_bar", COMPONENT_STATUS_BAR),
        module("components/toggle_group", COMPONENT_TOGGLE_GROUP),
        module("components/select", COMPONENT_SELECT),
        module("components/combobox", COMPONENT_COMBOBOX),
        module("components/input", COMPONENT_INPUT),
        module("components/split_pane", COMPONENT_SPLIT_PANE),
        module("components/popover", COMPONENT_POPOVER),
        module("components/title_bar", COMPONENT_TITLE_BAR),
        module("layouts/toolbar", LAYOUT_TOOLBAR),
        module("layouts/inline", LAYOUT_INLINE),
        module("patterns/description_list", PATTERN_DESCRIPTION_LIST),
        module("patterns/section", PATTERN_SECTION),
        module("patterns/stat", PATTERN_STAT),
    ]))
}

/// The SVGs the mounted official components declare, keyed by the
/// provider-relative asset name (namespace `app`).
pub fn assets() -> Vec<(String, AssetData)> {
    let svg = |bytes: &[u8]| AssetData {
        mime_type: "image/svg+xml".to_owned(),
        bytes: bytes.to_vec(),
    };
    vec![
        ("icons/check".to_owned(), svg(CHECK_SVG)),
        ("icons/minus".to_owned(), svg(MINUS_SVG)),
        ("icons/close".to_owned(), svg(CLOSE_SVG)),
        ("icons/chevron_down".to_owned(), svg(CHEVRON_DOWN_SVG)),
        ("icons/chevron_up".to_owned(), svg(CHEVRON_UP_SVG)),
    ]
}

pub fn manifest() -> AppManifest {
    AppManifest::new(entry())
        .with_capability("disktree.scan", "*")
        .expect("static capability requirement")
        .with_capability("disktree.tree", "*")
        .expect("static capability requirement")
        .with_capability("disktree.open", "*")
        .expect("static capability requirement")
        .with_capability("disktree.git", "*")
        .expect("static capability requirement")
        .with_capability("disktree.args", "*")
        .expect("static capability requirement")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run one pure function of the embedded core module.
    fn core_call_i64(name: &str, arg: i64) -> String {
        let mut engine = rhai::Engine::new();
        engine.set_max_expr_depths(1_000, 1_000);
        let ast = engine.compile(CORE).expect("core.rhai compiles");
        let mut scope = rhai::Scope::new();
        engine
            .call_fn(&mut scope, &ast, name, (arg,))
            .expect("core function call")
    }

    #[test]
    fn bytes_step_up_at_1023_5_of_a_unit() {
        // Below the boundary the value keeps its unit; at 10 and up no
        // decimal is shown, so 1023.499 KiB prints "1023 KiB".
        assert_eq!(core_call_i64("human_bytes", 1_048_063), "1023 KiB");
        assert_eq!(core_call_i64("human_bytes", 1_047_552), "1023 KiB");
        // From 1023.5 of a unit the next unit takes over: 1024 of a unit
        // would print instead of 1.0 of the next (tobi PR #70).
        assert_eq!(core_call_i64("human_bytes", 1_048_064), "1.0 MiB");
        assert_eq!(core_call_i64("human_bytes", 1_048_575), "1.0 MiB");
        assert_eq!(core_call_i64("human_bytes", 1_048_576), "1.0 MiB");
        // The short form follows the same boundary.
        assert_eq!(core_call_i64("human_bytes_short", 1_048_063), "1023KiB");
        assert_eq!(core_call_i64("human_bytes_short", 1_048_064), "1.0MiB");
        // A value under 10 keeps its one decimal.
        assert_eq!(core_call_i64("human_bytes", 1_281), "1.3 KiB");
        assert_eq!(core_call_i64("human_bytes", 1_229), "1.2 KiB");
    }

    #[test]
    fn counts_step_up_where_the_decimal_would_round() {
        assert_eq!(core_call_i64("human_count", 9_999), "9999");
        assert_eq!(core_call_i64("human_count", 10_000), "10.0k");
        assert_eq!(core_call_i64("human_count", 999_949), "999.9k");
        // 999,999 used to read "1000.0k"; it is 1.0M now.
        assert_eq!(core_call_i64("human_count", 999_950), "1.0M");
        assert_eq!(core_call_i64("human_count", 999_999), "1.0M");
    }
}
