use std::path::PathBuf;

use gpui_rhai::{ScriptApplication, ScriptView};

/// Build the file-backed tutorial without starting a separate application.
#[must_use]
pub fn view() -> ScriptView {
    let entry =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello_world/ui/main.rhai");
    ScriptView::files(entry)
}

fn main() {
    view()
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(560.0, 320.0)
                .run()
        })
        .expect("hello_world failed");
}
