#[path = "support/fonts.rs"]
mod shared_fonts;
#[path = "support/themes.rs"]
mod shared_themes;

use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, ModuleId, ScriptApplication, ScriptView};

const THEME: &str = shared_themes::PRIMARY;
const COMPONENT: &str = include_str!("../../../registry/components/diff_viewer.rhai");

const MAIN: &str = r#"
import "components/diff_viewer" as diff_viewer;
import "components/toggle_group" as toggle_group;

fn state_schema() { #{ fields: #{
    mode: #{ schema: #{ type: "string", allowed: ["unified", "split"] },
        "default": #{ type: "string", value: "split" } }
} } }
fn activated(ctx, location) { () }
fn mode_changed(ctx, values) { ctx.set_state("mode", values[0]); }

fn view(ctx) {
    column([
        row([
            text("Server config diff").with_style(style().typography("heading")),
            toggle_group::ToggleGroup(#{key:"diff-mode",label:"Diff layout",size:"sm",
                items:[#{value:"unified",label:"Unified"},#{value:"split",label:"Split"}],
                values:[ctx.get_state("mode")],allow_empty:false,on_change:Fn("mode_changed")})
        ]).with_style(style().items_center().justify_between().flex_wrap().gap(theme_spacing("sm"))),
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
        .min_width(px(0)).min_height(px(0)).padding(theme_spacing("lg")).gap(theme_spacing("md"))
        .typography("body").text_color(theme_color("text_primary"))
        .background(theme_color("surface")))
}
"#;

/// Build the neutral file-comparison tutorial with the bundled full designs.
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
                ModuleId::parse("components/diff_viewer").unwrap(),
                COMPONENT.to_owned(),
            ),
            (
                ModuleId::parse("components/toggle_group").unwrap(),
                gpui_rhai_registry::TOGGLE_GROUP_SOURCE.to_owned(),
            ),
        ])),
        THEME,
    )
    .token_base(include_str!("../../../registry/tokens.rhai"))
    .font_sources(shared_fonts::sources())
    .theme_sources(shared_themes::additional())
}

fn main() {
    ScriptApplication::new(view().prepare().expect("DiffViewer prepares"))
        .window_size(1_100.0, 680.0)
        .run()
        .expect("DiffViewer runs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_viewer_example_prepares() {
        view().prepare().unwrap();
    }
}
