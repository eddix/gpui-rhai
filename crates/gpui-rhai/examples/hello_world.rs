use std::path::PathBuf;

use gpui_rhai::{FileScriptView, ScriptApplication};

fn main() {
    let entry =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello_world/ui/main.rhai");
    FileScriptView::new(entry)
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(560.0, 320.0)
                .run()
        })
        .expect("hello_world failed");
}
