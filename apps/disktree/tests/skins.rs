//! Prepare every shipped skin through gpui-rhai's real loaders. Runs the same
//! validation a launch does — module resolution, component schemas,
//! runtime_api, token and asset coverage — without opening a window, so a
//! registry or runtime change that breaks a skin fails `cargo test`.

use std::path::{Path, PathBuf};

use disktree_rhai::{capability::DisktreeExtension, embedded};
use gpui_rhai::{EmbeddedScriptView, FileScriptView};

/// The skin's deep view functions exceed the runtime's 32-deep function
/// default; this mirrors src/main.rs.
const EXPRESSION_DEPTH_LIMITS: (usize, usize) = (64, 128);

fn extension() -> DisktreeExtension {
    DisktreeExtension::new().with_args(PathBuf::from("/tmp"), false, false, 3, "size", "dark")
}

fn prepare_file_skin(entry: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(entry);
    if let Err(error) = FileScriptView::new(path)
        .extension(extension())
        .expression_depth_limits(EXPRESSION_DEPTH_LIMITS.0, EXPRESSION_DEPTH_LIMITS.1)
        .prepare()
    {
        panic!("{entry}: {error}");
    }
}

/// The default skin as files (what --dev and --skin load).
#[test]
fn the_default_skin_prepares_from_files() {
    prepare_file_skin("ui/main.rhai");
}

/// The minimal example skin as files.
#[test]
fn the_minimal_skin_prepares_from_files() {
    prepare_file_skin("skins/minimal/main.rhai");
}

/// The embedded packaging path: frozen sources + token base + assets, the
/// exact assembly src/main.rs ships.
#[test]
fn the_embedded_skin_prepares() {
    if let Err(error) =
        EmbeddedScriptView::new(embedded::entry(), embedded::sources(), embedded::THEME)
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .asset_sources(embedded::assets())
            .extension(extension())
            .expression_depth_limits(EXPRESSION_DEPTH_LIMITS.0, EXPRESSION_DEPTH_LIMITS.1)
            .prepare()
    {
        panic!("embedded: {error}");
    }
}
