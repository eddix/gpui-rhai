//! Compile a .rhai file (or ui/*.rhai when run from the repo root) with a
//! plain rhai engine and report errors — a quick script sanity tool.

use rhai::Engine;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: rhai_check <file.rhai>");
    let source = std::fs::read_to_string(&path).expect("read");
    let mut engine = Engine::new();
    engine.set_max_expr_depths(1_000, 1_000);
    match engine.compile(&source) {
        Ok(_) => println!("{}: ok", path),
        Err(error) => {
            eprintln!("{}: {error}", path);
            std::process::exit(1);
        }
    }
}
