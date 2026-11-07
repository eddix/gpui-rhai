//! `gpui_rhai::offscreen`, which an application's `examples/shot.rs` runs: the
//! steps drive the real macOS renderer and save PNGs. No test harness: the
//! macOS platform runs on the main thread.

use std::collections::BTreeMap;
use std::path::Path;

use gpui_rhai::offscreen::{ShotError, ShotPlan, run_plan};
use gpui_rhai::{EmbeddedScriptSource, ModuleId, PreparedScriptView, ScriptView, ScriptViewError};
use gpui_rhai_registry::{DEFAULT_THEME, TOKEN_BASE_SOURCE};

const COUNTER: &str = r#"
    fn state_schema() { #{ fields: #{ count: #{ schema: #{ type: "integer" },
        "default": #{ type: "integer", value: 0 } } } } }
    fn increment(ctx, payload) { ctx.set_state("count", ctx.get_state("count") + 1); }
    fn view(ctx) {
        column([
            text(`Count: ${ctx.get_state("count")}`),
            text("Increment").on_click(Fn("increment")),
        ])
    }
"#;

fn counter() -> Result<PreparedScriptView, ScriptViewError> {
    let main = ModuleId::parse("main").unwrap();
    ScriptView::sources(
        main.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(main, COUNTER.to_owned())])),
        DEFAULT_THEME,
    )
    .token_base(TOKEN_BASE_SOURCE)
    .prepare()
}

fn plan(line: &str) -> ShotPlan {
    ShotPlan::parse(line.split_whitespace().map(ToOwned::to_owned)).unwrap()
}

fn main() {
    let directory = std::env::temp_dir().join(format!("gpui-rhai-shot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let path = |name: &str| directory.join(name).display().to_string();

    if cfg!(not(target_os = "macos")) {
        assert!(matches!(
            run_plan(&plan(&format!("--out {}", path("a.png"))), counter),
            Err(ShotError::Unsupported)
        ));
        println!("offscreen_shot: not macOS, run_plan reports Unsupported");
        return;
    }

    // A click by accessible name reaches the script handler: the steps file's
    // query for the new text fails the run otherwise.
    let steps = path("steps.jsonl");
    std::fs::write(
        &steps,
        format!(
            concat!(
                r#"{{"id":"q","command":"query","locator":{{"kind":"text","text":"Count: 2"}}}}"#,
                "\n",
                r#"{{"command":"capture","path":"{}"}}"#,
                "\n",
            ),
            path("after.png"),
        ),
    )
    .unwrap();
    run_plan(
        &plan(&format!(
            "--size 320x200 --out {} --click Increment --click Increment --steps {steps}",
            path("before.png")
        )),
        counter,
    )
    .unwrap();
    for name in ["before.png", "after.png"] {
        let (width, height) = image::image_dimensions(directory.join(name)).unwrap();
        // Device pixels: 1x or 2x the window's points.
        assert!(
            [(320, 200), (640, 400)].contains(&(width, height)),
            "{name}: {width}x{height}"
        );
    }
    let before = image::open(directory.join("before.png")).unwrap();
    let after = image::open(directory.join("after.png")).unwrap();
    assert_ne!(before.as_bytes(), after.as_bytes(), "the count changed");

    // The first failing line stops the run and names itself.
    std::fs::write(
        &steps,
        r#"{"command":"query","locator":{"kind":"text","text":"Count: 9"}}"#,
    )
    .unwrap();
    let error = run_plan(&plan(&format!("--steps {steps}")), counter)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains(&format!("{}:1:", Path::new(&steps).display()))
            && error.contains("no retained"),
        "{error}"
    );
    let error = run_plan(
        &plan(&format!("--click Missing --out {}", path("x.png"))),
        counter,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("no node named `Missing`"), "{error}");

    std::fs::remove_dir_all(&directory).unwrap();
    println!("offscreen_shot: ok");
}
