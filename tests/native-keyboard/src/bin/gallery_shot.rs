//! Capture real Gallery interaction states using the SDK's public shot runner.
//!
//! gallery_shot <page> <theme> [shot options]
//! Example: gallery_shot button modern-light --size 1280x860
//!   --click "Run query" --out /tmp/loading.png --wait 225 --out /tmp/rotated.png
//! The existing shot protocol also supports theme/locale changes and JSONL
//! automation steps; no second interaction or rendering implementation lives here.

use gpui_rhai::MotionPreference;
use gpui_rhai::offscreen::{ShotPlan, run_plan};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let page = args
        .next()
        .ok_or("usage: gallery_shot <page> <theme> [shot options]")?;
    let theme = args
        .next()
        .ok_or("gallery_shot requires a bundled theme key")?;
    let plan = ShotPlan::parse(args).map_err(|error| error.to_string())?;
    let launch = AcceptanceLaunch {
        page,
        theme,
        motion: MotionPreference::Normal,
        ..Default::default()
    };
    let view = acceptance::view(&launch)?;
    run_plan(&plan, move || view.prepare()).map_err(|error| error.to_string())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("gallery_shot: {error}");
        std::process::exit(1);
    }
}
