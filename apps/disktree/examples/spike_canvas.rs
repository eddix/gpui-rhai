//! Spike 2 (M0): does a viewport-sized retained canvas scene with thousands of
//! rects plus absolutely-positioned text overlays render without blowing the
//! command budget (100k) or the script op budget (1M)?
//!
//! Run: `cargo run --example spike_canvas` — a window with a 3000-rect grid
//! and 150 labels appears. Watch for error banners; close to exit.

use std::collections::BTreeMap;

use gpui_rhai::{
    AppManifest, EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication,
};

const THEME: &str = include_str!("../ui/theme.rhai");

const MAIN: &str = r#"
fn view(ctx) {
    let rects = 3000;
    let cols = 60;
    let rows = (rects / cols) + 1;
    let commands = [];
    let overlays = [];
    let i = 0;
    while i < rects {
        let col = i % cols;
        let row = i / cols;
        let x = col.to_float() * 20.0 + 4.0;
        let y = row.to_float() * 16.0 + 4.0;
        // A long-tailed size distribution, painted as alpha variation.
        let mass = 1000.0 / (i + 1).to_float();
        let alpha = if mass > 500.0 { 255 } else if mass > 100.0 { 190 } else { 110 };
        commands.push(canvas_rect(`r${i}`, x, y, 18.0, 14.0, rgba(0x7aa2f700 + alpha)));
        if i % 20 == 0 {
            overlays.push(
                text(`#${i}`)
                    .with_key(`label_${i}`)
                    .with_style(
                        style()
                            .absolute()
                            .left(px(x + 21.0))
                            .top(px(y + 1.0))
                            .font_size(px(8.0))
                            .text_color(theme_color("text_muted"))
                    )
            );
        }
        i += 1;
    }

    let stage = box(
        [
            canvas(canvas_scene(commands))
                .with_key("grid")
                .with_style(style().width(px(1208.0)).height(px(rows.to_float() * 16.0 + 8.0))),
        ] + overlays
    )
        .with_key("stage")
        .with_style(
            style()
                .relative()
                .width(px(1208.0))
                .height(px(rows.to_float() * 16.0 + 8.0))
        );

    column([
        text([
            span("canvas spike").color(theme_color("text_primary")).bold(),
            span(` — ${rects} rects, ${rows} rows, ${overlays.len()} labels`)
                .color(theme_color("text_muted")),
        ]).with_style(style().font_size(rem(1.25))),
        stage,
    ])
        .with_key("root")
        .with_style(
            style()
                .flex_col()
                .padding(px(24.0))
                .gap(px(12.0))
                .background(theme_color("surface"))
        )
}
"#;

fn main() {
    let entry = ModuleId::parse("main").expect("static module ID");
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([(entry.clone(), MAIN.to_owned())]));
    let manifest = AppManifest::new(entry.clone());
    EmbeddedScriptView::new(entry, scripts, THEME)
        .manifest(manifest)
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(1280.0, 900.0)
                .window_options(|mut options, _| {
                    options.titlebar = Some(gpui::TitlebarOptions {
                        title: Some("spike_canvas".into()),
                        ..gpui::TitlebarOptions::default()
                    });
                    options
                })
                .run()
        })
        .expect("spike_canvas failed");
}
