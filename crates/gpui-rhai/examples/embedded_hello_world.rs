use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const MAIN: &str = include_str!("../../../examples/hello_world/ui/main.rhai");
const BUTTON: &str = include_str!("../../../examples/hello_world/ui/components/button.rhai");
const LABEL: &str = include_str!("../../../examples/hello_world/ui/components/label.rhai");
const INPUT: &str = include_str!("../../../examples/hello_world/ui/components/input.rhai");
const THEME: &str = include_str!("../../../examples/hello_world/ui/theme.rhai");
const EN: &str = include_str!("../../../examples/hello_world/ui/locales/en.rhai");
const ZH_CN: &str = include_str!("../../../examples/hello_world/ui/locales/zh_cn.rhai");

fn main() {
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
    ]));
    EmbeddedScriptView::new(entry, scripts, THEME)
        .locale_sources([
            ("locales/en.rhai".to_owned(), EN.to_owned()),
            ("locales/zh_cn.rhai".to_owned(), ZH_CN.to_owned()),
        ])
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(560.0, 320.0)
                .run()
        })
        .expect("embedded hello_world failed");
}
