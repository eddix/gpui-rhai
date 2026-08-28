use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const THEME: &str = include_str!("../../../registry/themes/default_dark.rhai");
const MAIN: &str = r#"
fn state_schema() {
    #{ fields: #{ focused: #{ schema: #{ type: "string" },
        "default": #{ type: "string", value: "item-0" } } } }
}
fn focused(ctx, key) { ctx.set_state("focused", key); }
fn view(ctx) {
    let items = [];
    for index in 0..5000 {
        let key = `item-${index}`;
        items.push(#{ key: key, node: text(`Row ${index}`).with_key(key) });
    }
    column([
        text(`Focused: ${ctx.get_state("focused")}`),
        virtual_list(#{
            key: "five-thousand", label: "Five thousand rows",
            items: items, row_height: 28, height: 420, overscan: 2
        }).on_change(Fn("focused"))
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
