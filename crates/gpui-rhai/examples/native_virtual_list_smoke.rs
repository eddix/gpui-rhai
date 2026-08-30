use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const THEME: &str = include_str!("../../../registry/themes/default_dark.rhai");
const MAIN: &str = r#"
fn row(ctx, payload) { text(payload.item.label).with_key(payload.key) }
fn view(ctx) {
    let data = [];
    for index in 0..5000 {
        let key = `item-${index}`;
        data.push(#{ key: key, label: `Row ${index}` });
    }
    column([
        text("Data-backed virtual collection"),
        virtual_collection(#{
            key: "five-thousand", label: "Five thousand rows",
            data: data, estimated_height: 28, height: 420, overdraw_pixels: 112,
            alignment: "top", follow_tail: false
        }, Fn("row"))
    ]).with_style(style().padding(px(16)).gap(px(8)).background(theme_color("surface")))
}
"#;

fn main() {
    let entry = ModuleId::parse("main").expect("static module ID");
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([(entry.clone(), MAIN.to_owned())]));
    EmbeddedScriptView::new(entry, scripts, THEME)
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(620.0, 500.0)
                .run()
        })
        .expect("native virtual list smoke failed");
}
