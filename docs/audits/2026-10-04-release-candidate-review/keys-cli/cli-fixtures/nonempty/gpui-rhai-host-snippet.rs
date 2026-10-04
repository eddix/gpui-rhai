fn main() {
    let view = gpui_rhai::FileScriptView::new("ui/main.rhai")
        .prepare()
        .expect("GPUI Rhai view preparation failed");
    gpui_rhai::ScriptApplication::new(view)
        .run()
        .expect("GPUI Rhai application failed");
}
