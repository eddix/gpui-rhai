//! Bring your own design: a treemap app with its own theme vocabulary, no token
//! base and no official components. See `examples/byod_treemap/README.md`.

use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const MAIN: &str = include_str!("../../../examples/byod_treemap/ui/main.rhai");
const THEME: &str = include_str!("../../../examples/byod_treemap/ui/theme.rhai");

fn main() {
    let entry = ModuleId::parse("main").expect("static entry ID");
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([(entry.clone(), MAIN.to_owned())]));
    EmbeddedScriptView::new(entry, scripts, THEME)
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(780.0, 520.0)
                .run()
        })
        .expect("byod_treemap failed");
}
