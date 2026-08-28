use std::path::PathBuf;

use gpui_rhai::ScriptApp;

fn main() {
    let entry =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello_world/ui/main.rhai");
    ScriptApp::new(entry)
        .window_size(560.0, 320.0)
        .run()
        .expect("hello_world failed");
}
