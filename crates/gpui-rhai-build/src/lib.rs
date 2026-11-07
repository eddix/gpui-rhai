//! Embeds a gpui-rhai application's `ui/` directory into its binary.
//!
//! Call [`embed_ui`] from the application's `build.rs`:
//!
//! ```no_run
//! // build.rs
//! fn main() {
//!     gpui_rhai_build::embed_ui("ui").expect("embed ui/");
//! }
//! ```
//!
//! and mount with `gpui_rhai::script_view!()`, which reads the files in a
//! debug build and this embedded copy in a release build. Cargo runs the build
//! script again whenever a file under `ui/` changes, so the copy never goes
//! stale.

// The example is a build script, whose `main` is the point.
#![allow(clippy::needless_doctest_main)]

use std::fmt::{self, Write as _};
use std::path::{Path, PathBuf};
use std::{env, fs, io};

/// The file `gpui_rhai::script_view!()` includes from `OUT_DIR`.
pub const OUTPUT_FILE: &str = "gpui_rhai_ui.rs";

/// Why the `ui/` directory could not be embedded.
#[derive(Debug)]
pub enum EmbedError {
    /// A file or directory could not be read or written.
    Io { path: PathBuf, source: io::Error },
    /// The directory lacks a file every `ui/` tree has.
    Missing(PathBuf),
    /// Cargo did not set an environment variable a build script gets.
    Environment(&'static str),
}

impl fmt::Display for EmbedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::Missing(path) => write!(
                formatter,
                "{} is missing; a ui/ directory needs app.toml and theme.rhai",
                path.display()
            ),
            Self::Environment(name) => write!(
                formatter,
                "{name} is not set; call embed_ui from a build script"
            ),
        }
    }
}

impl std::error::Error for EmbedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Missing(_) | Self::Environment(_) => None,
        }
    }
}

/// Embed the `ui/` directory at `dir`, relative to the crate's manifest
/// directory, into `OUT_DIR/gpui_rhai_ui.rs`, and ask Cargo to run the build
/// script again when anything under it changes. Hidden files (`.DS_Store`)
/// are left out. Returns the path written.
///
/// # Errors
///
/// Returns [`EmbedError`] when the directory cannot be read, lacks `app.toml`
/// or `theme.rhai`, or the output cannot be written.
pub fn embed_ui(dir: impl AsRef<Path>) -> Result<PathBuf, EmbedError> {
    let manifest_dir =
        env::var_os("CARGO_MANIFEST_DIR").ok_or(EmbedError::Environment("CARGO_MANIFEST_DIR"))?;
    let out_dir = env::var_os("OUT_DIR").ok_or(EmbedError::Environment("OUT_DIR"))?;
    let root = Path::new(&manifest_dir).join(dir);
    println!("cargo:rerun-if-changed={}", root.display());
    let source = embedded_source(&root)?;
    let output = Path::new(&out_dir).join(OUTPUT_FILE);
    fs::write(&output, source).map_err(|source| EmbedError::Io {
        path: output.clone(),
        source,
    })?;
    Ok(output)
}

/// The Rust expression for the files under `root`: a slice of each file's
/// path relative to `root`, with `/` separators, and its bytes, sorted by path.
///
/// # Errors
///
/// Returns [`EmbedError`] when the directory cannot be read or lacks
/// `app.toml` or `theme.rhai`.
pub fn embedded_source(root: &Path) -> Result<String, EmbedError> {
    for required in ["app.toml", "theme.rhai"] {
        let path = root.join(required);
        if !path.is_file() {
            return Err(EmbedError::Missing(path));
        }
    }
    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    files.sort();
    let mut source = String::from("&[\n");
    for (name, path) in files {
        let path = path.to_str().ok_or_else(|| EmbedError::Io {
            path: path.clone(),
            source: io::Error::new(io::ErrorKind::InvalidData, "path is not UTF-8"),
        })?;
        let _ = writeln!(source, "    ({name:?}, include_bytes!({path:?})),");
    }
    source.push(']');
    Ok(source)
}

fn collect(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), EmbedError> {
    let read_error = |source| EmbedError::Io {
        path: directory.to_path_buf(),
        source,
    };
    for entry in fs::read_dir(directory).map_err(read_error)? {
        let path = entry.map_err(read_error)?.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_none_or(|name| name.starts_with('.'))
        {
            continue;
        }
        if path.is_dir() {
            collect(root, &path, files)?;
        } else {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            let name = relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let absolute = fs::canonicalize(&path).map_err(|source| EmbedError::Io {
                path: path.clone(),
                source,
            })?;
            files.push((name, absolute));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tree_lists_every_visible_file_by_its_relative_path() {
        let directory =
            std::env::temp_dir().join(format!("gpui-rhai-build-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("assets/icons")).unwrap();
        fs::write(directory.join("app.toml"), "entry = \"main\"\n").unwrap();
        fs::write(directory.join("theme.rhai"), "fn theme() {}\n").unwrap();
        fs::write(directory.join("main.rhai"), "fn view(ctx) {}\n").unwrap();
        fs::write(directory.join("assets/icons/check.svg"), "<svg/>").unwrap();
        fs::write(directory.join(".DS_Store"), [0u8]).unwrap();
        let source = embedded_source(&directory).unwrap();
        let names = source
            .lines()
            .filter_map(|line| line.trim().strip_prefix("(\""))
            .map(|line| line.split('"').next().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "app.toml",
                "assets/icons/check.svg",
                "main.rhai",
                "theme.rhai"
            ]
        );
        assert!(source.starts_with("&[\n") && source.ends_with(']'));

        fs::remove_file(directory.join("theme.rhai")).unwrap();
        assert!(matches!(
            embedded_source(&directory),
            Err(EmbedError::Missing(path)) if path.ends_with("theme.rhai")
        ));
        fs::remove_dir_all(&directory).unwrap();
    }
}
