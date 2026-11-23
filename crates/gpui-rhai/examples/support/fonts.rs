//! Fonts are application resources, not an implicit process-global SDK side
//! effect. All embedded examples register the same licensed UI faces.

pub fn sources() -> Vec<gpui_rhai::FontSource> {
    gpui_rhai_registry::BUNDLED_FONT_SOURCES
        .iter()
        .map(|&(name, bytes)| {
            gpui_rhai::FontSource::new(name, bytes.to_vec()).expect("bundled UI font is valid")
        })
        .collect()
}
