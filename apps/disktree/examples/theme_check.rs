//! Load a skin's token base and full primary theme together, as ScriptView
//! does. Usage: theme_check [skin-directory], relative to this application.

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let skin = root.join(std::env::args().nth(1).unwrap_or_else(|| "ui".into()));
    let theme_path = skin.join("theme.rhai");
    let tokens_path = skin.join("tokens.rhai");
    let source = std::fs::read_to_string(&theme_path).expect("read skin/theme.rhai");
    let tokens = std::fs::read_to_string(&tokens_path).expect("read skin/tokens.rhai");
    let engine = gpui_rhai::RuntimeEngine::new();
    let base = gpui_rhai::load_token_base(engine.engine(), &tokens_path.to_string_lossy(), &tokens)
        .expect("load skin token base");
    match gpui_rhai::load_theme_with_layers(
        engine.engine(),
        Some(&base),
        &theme_path.to_string_lossy(),
        &source,
        &gpui_rhai::ThemeTokenOverrides::default(),
    ) {
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
