#[path = "support/fonts.rs"]
mod shared_fonts;
#[path = "support/themes.rs"]
mod shared_themes;

use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, ModuleId, ScriptApplication, ScriptView};

const THEME: &str = shared_themes::PRIMARY;
const COMPONENT: &str = include_str!("../../../registry/components/code_viewer.rhai");

const MAIN: &str = r#"
import "components/code_viewer" as code_viewer;

fn activated(ctx, location) { () }

fn view(ctx) {
    column([
        text("Code viewer").with_style(style().typography("heading")),
        text("Native syntax highlighting, virtual scrolling, selection and Cmd+F search.")
            .with_style(style().typography("body_small").text_color(theme_color("text_muted"))),
        code_viewer::CodeViewer(#{
            key: "rhai-source",
            label: "Rhai component source",
            file_name: "counter.rhai",
            language: "rhai",
            source: "import \"components/button\" as button;\n\nfn state_schema() {\n    #{ fields: #{ count: #{ schema: #{ type: \"integer\" }, default: #{ type: \"integer\", value: 0 } } } }\n}\n\nfn increment(ctx, payload) {\n    ctx.set_state(\"count\", ctx.get_state(\"count\") + 1);\n}\n\nfn view(ctx) {\n    column([\n        text(`Count: ${ctx.get_state(\"count\")}`),\n        button::Button(#{ text: \"Increment\", on_click: Fn(\"increment\") })\n    ])\n}\n",
            on_location_activate: Fn("activated")
        })
    ]).with_style(style().width(relative(1)).height(relative(1))
        .min_width(px(0)).min_height(px(0)).padding(theme_spacing("lg")).gap(theme_spacing("sm"))
        .typography("body").text_color(theme_color("text_primary"))
        .background(theme_color("surface")))
}
"#;

/// Build the native code-viewer tutorial, using the bundled full designs.
///
/// # Panics
/// Panics if a static module identifier is invalid.
#[must_use]
pub fn view() -> ScriptView {
    ScriptView::sources(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (ModuleId::parse("main").unwrap(), MAIN.to_owned()),
            (
                ModuleId::parse("components/code_viewer").unwrap(),
                COMPONENT.to_owned(),
            ),
        ])),
        THEME,
    )
    .token_base(include_str!("../../../registry/tokens.rhai"))
    .font_sources(shared_fonts::sources())
    .theme_sources(shared_themes::additional())
}

fn main() {
    ScriptApplication::new(view().prepare().expect("CodeViewer prepares"))
        .window_size(900.0, 620.0)
        .run()
        .expect("CodeViewer runs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_viewer_example_prepares() {
        view().prepare().unwrap();
    }
}
