use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const THEME: &str = include_str!("../../../registry/themes/default_dark.rhai");
const COMPONENT: &str = include_str!("../../../registry/components/diff_viewer.rhai");

const MAIN: &str = r#"
import "components/diff_viewer" as diff_viewer;

fn state_schema() { #{ fields: #{
    mode: #{ schema: #{ type: "string", allowed: ["unified", "split"] },
        "default": #{ type: "string", value: "split" } }
} } }
fn activated(ctx, location) { () }
fn unified(ctx, payload) { ctx.set_state("mode", "unified"); }
fn split(ctx, payload) { ctx.set_state("mode", "split"); }
fn mode_button(ctx, label, mode, handler) {
    let selected = ctx.get_state("mode") == mode;
    let background = if selected { theme_color("selection") } else { theme_color("surface_raised") };
    text(label).with_style(style().padding_x(px(8)).padding_y(px(4)).background(background))
        .on_click(handler)
}

fn view(ctx) {
    column([
        row([
            text("SERVER CONFIG DIFF").with_style(style().typography("heading").font_weight(700)),
            row([
                mode_button(ctx, "Unified", "unified", Fn("unified")),
                mode_button(ctx, "Split", "split", Fn("split"))
            ]).with_style(style().gap(px(1)))
        ]).with_style(style().items_center().justify_between()),
        diff_viewer::DiffViewer(#{
            key: "server-config",
            mode: ctx.get_state("mode"),
            left: #{
                label: "Server A",
                file_name: "service.toml",
                language: "toml",
                source: "[server]\nhost = \"api-a.internal\"\nport = 8080\nworkers = 4\n\n[features]\ncache = true\nmetrics = false\n"
            },
            right: #{
                label: "Server B",
                file_name: "service.toml",
                language: "toml",
                source: "[server]\nhost = \"api-b.internal\"\nport = 8443\nworkers = 8\n\n[features]\ncache = true\nmetrics = true\ntracing = true\n"
            },
            on_location_activate: Fn("activated")
        })
    ]).with_style(style().width(relative(1)).height(relative(1))
        .min_width(px(0)).min_height(px(0)).padding(px(18)).gap(px(10))
        .background(theme_color("surface")))
}
"#;

fn prepared() -> gpui_rhai::PreparedScriptView {
    EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (ModuleId::parse("main").unwrap(), MAIN.to_owned()),
            (
                ModuleId::parse("components/diff_viewer").unwrap(),
                COMPONENT.to_owned(),
            ),
        ])),
        THEME,
    )
    .prepare()
    .expect("DiffViewer prepares")
}

fn main() {
    ScriptApplication::new(prepared())
        .window_size(1_100.0, 680.0)
        .run()
        .expect("DiffViewer runs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_viewer_example_prepares() {
        let _ = prepared();
    }
}
