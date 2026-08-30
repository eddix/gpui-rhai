use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const THEME: &str = include_str!("../../../registry/themes/default_dark.rhai");

const TIMELINE: &str = r#"
define_component(#{
    metadata: #{
        id: "components/mini_timeline", "export": "MiniTimeline", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{}
    },
    schema: #{
        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"]
    },
    render: Fn("render_MiniTimeline")
});

fn pointer_x(event) {
    let x = event.window.x - 40.0;
    if x < 0.0 { 0.0 } else if x > 640.0 { 640.0 } else { x }
}

fn begin_drag(ctx, event) {
    ctx.set_signal("dragging", true);
    ctx.set_signal("playhead_x", pointer_x(event));
    event_response().prevent_default().capture_pointer()
}

fn drag(ctx, event) {
    if ctx.get_signal("dragging") {
        ctx.set_signal("playhead_x", pointer_x(event));
        event_response().prevent_default().stop()
    } else { propagate() }
}

fn end_drag(ctx, event) {
    ctx.set_signal("dragging", false);
    event_response().prevent_default().release_pointer()
}

fn MiniTimeline(props) { render_component("components/mini_timeline", props) }
fn render_MiniTimeline(ctx, props) {
    let playhead_x = signal("playhead_x", 180.0);
    let dragging = signal("dragging", false);
    let commands = [
        canvas_rect("track_a", 0.0, 34.0, 640.0, 54.0, rgba(0x334155ff)),
        canvas_rect("clip_a", 36.0, 44.0, 190.0, 34.0, rgba(0x7aa2f7ff)),
        canvas_rect("clip_b", 246.0, 44.0, 132.0, 34.0, rgba(0xbb9af7ff)),
        canvas_rect("track_b", 0.0, 104.0, 640.0, 54.0, rgba(0x1f2937ff)),
        canvas_rect("clip_c", 112.0, 114.0, 286.0, 34.0, rgba(0x7dcfffff)),
        canvas_line("baseline", 0.0, 184.0, 640.0, 184.0, 1.0, theme_color("border"))
    ];
    for index in 0..11 {
        let x = index * 64;
        commands.push(canvas_line(
            `tick_${index}`, x * 1.0, 176.0, x * 1.0, 192.0, 1.0,
            theme_color("text_muted")
        ));
    }
    let surface = canvas(canvas_scene(commands))
        .with_key("timeline_scene")
        .with_style(style().width(px(640)).height(px(210)));
    let playhead = box([])
        .with_key("playhead")
        .bind_signal("translate_x", playhead_x)
        .with_style(style().absolute().left(px(0)).top(px(0))
            .width(px(2)).height(px(210)).background(theme_color("danger")));
    stack([surface, playhead])
        .with_key("timeline_surface")
        .with_style(component_style(props, "root",
            style().width(px(640)).height(px(210)).clip()
                .background(theme_color("surface_raised"))))
        .on("pointer_down", Fn("begin_drag"))
        .on("pointer_move", Fn("drag"))
        .on("pointer_up", Fn("end_drag"))
        .accessibility_role("slider")
        .accessibility_label("Timeline playhead")
}
"#;

const MAIN: &str = r#"
import "components/mini_timeline" as timeline;
fn view(ctx) {
    box([
        text([span("MINI ").bold(), span("TIMELINE").color(theme_color("accent"))])
            .with_key("title").with_style(style().font_size(rem(1.4))),
        text("Drag anywhere on the tracks; pointer capture keeps the playhead attached.")
            .with_key("help").with_style(style().text_color(theme_color("text_muted"))),
        timeline::MiniTimeline(#{ key: "editor" })
    ]).with_key("app").with_style(style().flex_col().gap(px(16)).padding(px(40))
        .width(relative(1.0)).height(relative(1.0)).background(theme_color("surface")))
}
"#;

fn timeline_view() -> EmbeddedScriptView {
    EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(BTreeMap::from([
            (ModuleId::parse("main").unwrap(), MAIN.to_owned()),
            (
                ModuleId::parse("components/mini_timeline").unwrap(),
                TIMELINE.to_owned(),
            ),
        ])),
        THEME,
    )
}

fn main() {
    let view = timeline_view().prepare().expect("timeline prepares");
    ScriptApplication::new(view)
        .window_size(760.0, 380.0)
        .run()
        .expect("timeline runs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_rhai_mini_timeline_prepares() {
        timeline_view().prepare().unwrap();
    }
}
