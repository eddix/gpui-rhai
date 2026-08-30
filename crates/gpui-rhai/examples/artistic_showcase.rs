use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const THEME: &str = include_str!("../../../registry/themes/tokyo_night.rhai");

const PANEL: &str = r##"
define_component(#{
    metadata: #{
        id: "components/art_panel", "export": "ArtPanel", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{}
    },
    schema: #{
        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"]
    },
    render: Fn("render_ArtPanel")
});

fn pulse(ctx, event) {
    let current = ctx.get_signal("panel_alpha");
    ctx.set_signal("panel_alpha", if current > 0.8 { 0.58 } else { 0.96 });
    event_response().prevent_default()
}

fn ArtPanel(props) { render_component("components/art_panel", props) }
fn render_ArtPanel(ctx, props) {
    let alpha = signal("panel_alpha", 0.92);
    let scene = canvas(canvas_scene([
        canvas_rect("horizon", 0.0, 132.0, 560.0, 2.0, theme_color("accent")),
        canvas_circle("sun", 430.0, 72.0, 42.0, rgba(0xffe0afcc)),
        canvas_circle("moon", 128.0, 92.0, 22.0, rgba(0x7aa2f7cc)),
        canvas_line("route_a", 36.0, 184.0, 290.0, 42.0, 3.0, rgba(0xbb9af7ff)),
        canvas_line("route_b", 290.0, 42.0, 520.0, 196.0, 3.0, rgba(0x7dcfffff)),
        canvas_fill_path("aurora", [
            path_move(24.0, 156.0),
            path_cubic(188.0, 112.0, 72.0, 88.0, 138.0, 196.0),
            path_cubic(352.0, 148.0, 238.0, 52.0, 310.0, 224.0),
            path_line(528.0, 212.0),
            path_line(528.0, 220.0),
            path_line(24.0, 220.0),
            path_close()
        ], linear_gradient(#{ angle: 90,
            from: color("#7aa2f744"), to: color("hsla(280, 65%, 65%, 15%)") }))
            .clip_rect(0.0, 0.0, 560.0, 220.0)
    ])).with_key("sky").with_style(style().width(px(560)).height(px(220)));

    box([
        text([
            span("NIGHT ").color(theme_color("accent")).bold(),
            span("SIGNALS").color(rgba(0x7dcfffff)).italic()
        ]).with_key("title").with_style(style().font_size(rem(1.8))),
        text("A pure Rhai retained scene — click the card to pulse its native opacity.")
            .with_key("subtitle")
            .with_style(style().text_color(theme_color("text_muted"))),
        scene
    ])
        .with_key("panel")
        .bind_signal("opacity", alpha)
        .on("pointer_down", Fn("pulse"))
        .with_style(component_style(props, "root",
            style().width(px(608)).padding(px(24)).gap(px(16)).flex_col()
                .radius(px(18)).border(px(1)).border_color(theme_color("border"))
                .linear_gradient(linear_gradient(#{ angle: 145,
                    from: rgba(0x24283be8), to: rgba(0x16161ee8) }))
                .shadow(shadow(#{ x: 0, y: 18, blur: 48, spread: 2,
                    color: rgba(0x00000070) }))
                .font_family("Avenir Next").cursor_pointer()))
        .accessibility_role("group")
        .accessibility_label("Night Signals artistic showcase")
}
"##;

const MAIN: &str = r#"
import "components/art_panel" as art;

fn view(ctx) {
    box([
        fragment([
            art::ArtPanel(#{ key: "hero" }),
            text("Box · Fragment · Span · Canvas · Gradient · Shadow · NativeSignal")
                .with_key("capabilities")
                .with_style(style().text_color(theme_color("text_muted")))
        ])
    ])
        .with_key("showcase")
        .with_style(style().width(relative(1.0)).height(relative(1.0))
            .padding(px(36)).gap(px(18)).flex_col()
            .linear_gradient(linear_gradient(#{ angle: 180,
                from: theme_color("surface"), to: rgba(0x10121bcc) })))
}
"#;

fn showcase_view() -> EmbeddedScriptView {
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        (ModuleId::parse("main").unwrap(), MAIN.to_owned()),
        (
            ModuleId::parse("components/art_panel").unwrap(),
            PANEL.to_owned(),
        ),
    ]));
    EmbeddedScriptView::new(ModuleId::parse("main").unwrap(), scripts, THEME)
}

fn main() {
    let view = showcase_view()
        .prepare()
        .expect("artistic showcase prepares");
    ScriptApplication::new(view)
        .window_size(760.0, 520.0)
        .run()
        .expect("artistic showcase runs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_rhai_artistic_showcase_prepares() {
        showcase_view().prepare().unwrap();
    }
}
