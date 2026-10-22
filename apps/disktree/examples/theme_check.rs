//! Load ui/theme.rhai through gpui-rhai's theme loader and print the
//! decoded variant, or the exact error. Mirrors what EmbeddedScriptView and
//! FileScriptView do at prepare() time.

fn main() {
    let source = std::fs::read_to_string("ui/theme.rhai").expect("read ui/theme.rhai");
    let engine = gpui_rhai::RuntimeEngine::new();
    match gpui_rhai::load_theme_source(engine.engine(), "ui/theme.rhai", &source) {
        Ok(theme) => {
            println!(
                "theme ok: family={} name={} mode={:?} colors={}",
                theme.family,
                theme.name,
                theme.mode,
                theme.tokens.colors.len()
            );
            for (name, color) in theme.tokens.colors.iter().take(6) {
                println!("  {name} = #{:08x}", color.as_rgba_hex());
            }
        }
        Err(error) => {
            println!("theme FAILED: {error}");
            std::process::exit(1);
        }
    }
}
