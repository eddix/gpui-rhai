//! Bring your own design: a treemap app with its own theme vocabulary, no token
//! base and no official components. See `examples/byod_treemap/README.md`.

#[path = "support/fonts.rs"]
mod shared_fonts;

use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, ModuleId, ScriptApplication, ScriptView};

const MAIN: &str = include_str!("../../../examples/byod_treemap/ui/main.rhai");
const THEME: &str = include_str!("../../../examples/byod_treemap/ui/theme.rhai");

/// Build the intentional bring-your-own-design control, without preset themes.
///
/// # Panics
/// Panics if the static module identifier is invalid.
#[must_use]
pub fn view() -> ScriptView {
    let entry = ModuleId::parse("main").expect("static entry ID");
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([(entry.clone(), MAIN.to_owned())]));
    ScriptView::sources(entry, scripts, THEME).font_sources(shared_fonts::sources())
}

fn main() {
    view()
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(780.0, 520.0)
                .run()
        })
        .expect("byod_treemap failed");
}
