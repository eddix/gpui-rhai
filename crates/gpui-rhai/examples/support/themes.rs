//! Complete designs shared by the reference examples. A design is not a palette
//! or a density alias; examples start on Modern/Dark and may switch atomically.

pub const PRIMARY: &str = gpui_rhai_registry::MODERN_DARK_THEME;

pub fn additional() -> impl Iterator<Item = (String, String)> {
    gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES
        .iter()
        .filter(|(_, source)| *source != PRIMARY)
        .map(|&(name, source)| (name.to_owned(), source.to_owned()))
}
