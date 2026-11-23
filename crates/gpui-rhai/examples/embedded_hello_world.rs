#[path = "support/fonts.rs"]
mod shared_fonts;
#[path = "support/themes.rs"]
mod shared_themes;

use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, ModuleId, ScriptApplication, ScriptView};

const MAIN: &str = include_str!("../../../examples/hello_world/ui/main.rhai");
const BUTTON: &str = include_str!("../../../examples/hello_world/ui/components/button.rhai");
const LABEL: &str = include_str!("../../../examples/hello_world/ui/components/label.rhai");
const INPUT: &str = include_str!("../../../examples/hello_world/ui/components/input.rhai");
const SPINNER: &str = include_str!("../../../examples/hello_world/ui/components/spinner.rhai");
const THEME: &str = include_str!("../../../examples/hello_world/ui/theme.rhai");
const EN: &str = include_str!("../../../examples/hello_world/ui/locales/en.rhai");
const ZH_CN: &str = include_str!("../../../examples/hello_world/ui/locales/zh_cn.rhai");

/// Build the same embedded tutorial that the standalone program runs.
///
/// # Panics
/// Panics if a static module identifier is invalid.
#[must_use]
pub fn view() -> ScriptView {
    let entry = ModuleId::parse("main").expect("static entry ID");
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        (entry.clone(), MAIN.to_owned()),
        (
            ModuleId::parse("components/button").expect("static Button ID"),
            BUTTON.to_owned(),
        ),
        (
            ModuleId::parse("components/label").expect("static Label ID"),
            LABEL.to_owned(),
        ),
        (
            ModuleId::parse("components/input").expect("static Input ID"),
            INPUT.to_owned(),
        ),
        (
            ModuleId::parse("components/spinner").expect("static Spinner ID"),
            SPINNER.to_owned(),
        ),
    ]));
    ScriptView::sources(entry, scripts, THEME)
        .token_base(include_str!("../../../registry/tokens.rhai"))
        .font_sources(shared_fonts::sources())
        .theme_sources(shared_themes::additional())
        .locale_sources([
            ("locales/en.rhai".to_owned(), EN.to_owned()),
            ("locales/zh_cn.rhai".to_owned(), ZH_CN.to_owned()),
        ])
}

fn main() {
    view()
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(560.0, 320.0)
                .run()
        })
        .expect("embedded hello_world failed");
}
