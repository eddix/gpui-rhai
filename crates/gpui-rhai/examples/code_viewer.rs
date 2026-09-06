use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const THEME: &str = include_str!("../../../registry/themes/default_dark.rhai");
const COMPONENT: &str = include_str!("../../../registry/components/code_viewer.rhai");

const MAIN: &str = r#"
import "components/code_viewer" as code_viewer;

fn activated(ctx, location) { () }

fn view(ctx) {
    column([
        text("CODE VIEWER").with_style(style().typography("heading").font_weight(700)),
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
        .min_width(px(0)).min_height(px(0)).padding(px(18)).gap(px(8))
        .background(theme_color("surface")))
}
"#;

fn prepared() -> gpui_rhai::PreparedScriptView {
    EmbeddedScriptView::new(
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
    .prepare()
    .expect("CodeViewer prepares")
}

fn main() {
    ScriptApplication::new(prepared())
        .window_size(900.0, 620.0)
        .run()
        .expect("CodeViewer runs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_viewer_example_prepares() {
        let _ = prepared();
    }
}
