//! A `ui/` directory compiled into the binary.

use std::collections::BTreeMap;
use std::path::Path;

use crate::app::{PROFILE_FILE, TOKEN_BASE_FILE, UiSources};
use crate::{AppManifest, AssetData, EmbeddedScriptSource, FontSource, ModuleId, ScriptViewError};

/// A `ui/` directory compiled into the binary by `gpui-rhai-build`: each
/// file's path relative to `ui/`, with `/` separators, and its bytes.
/// [`ScriptView::embedded`](crate::ScriptView::embedded) reads it as it would
/// read the directory it was built from.
#[derive(Clone, Copy, Debug)]
pub struct EmbeddedUi {
    files: &'static [(&'static str, &'static [u8])],
}

/// The parts of an embedded tree: the in-memory sources, and the token base
/// and fonts, which the builder holds.
pub(crate) struct EmbeddedTree {
    pub(crate) sources: UiSources,
    pub(crate) token_base: Option<String>,
    pub(crate) fonts: Vec<FontSource>,
}

impl EmbeddedUi {
    #[must_use]
    pub const fn new(files: &'static [(&'static str, &'static [u8])]) -> Self {
        Self { files }
    }

    /// Each file's path relative to `ui/` and its bytes.
    pub fn files(&self) -> impl Iterator<Item = (&'static str, &'static [u8])> + '_ {
        self.files.iter().copied()
    }

    fn text(path: &str, bytes: &[u8]) -> Result<String, ScriptViewError> {
        String::from_utf8(bytes.to_vec()).map_err(|error| ScriptViewError::Io {
            path: Path::new("ui").join(path),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, error),
        })
    }

    fn file(&self, path: &str) -> Option<&'static [u8]> {
        self.files
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, bytes)| *bytes)
    }

    fn required_text(&self, path: &str) -> Result<String, ScriptViewError> {
        let bytes = self.file(path).ok_or_else(|| ScriptViewError::Io {
            path: Path::new("ui").join(path),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not in the embedded tree"),
        })?;
        Self::text(path, bytes)
    }

    fn optional_text(&self, path: &str) -> Result<Option<String>, ScriptViewError> {
        self.file(path)
            .map(|bytes| Self::text(path, bytes))
            .transpose()
    }

    /// Read the tree as `ScriptView::files` reads a directory: `app.toml`
    /// names the entry, `theme.rhai` is the primary theme, `tokens.rhai`,
    /// `styles.rhai` and `profile.rhai` are optional, `themes/` and
    /// `locales/` hold the other themes and locales, `assets/` the `app`
    /// images, `fonts/` the fonts, and every other `.rhai` file is a module.
    pub(crate) fn tree(&self) -> Result<EmbeddedTree, ScriptViewError> {
        let manifest: AppManifest = toml::from_str(&self.required_text("app.toml")?)
            .map_err(|error| ScriptViewError::Manifest(error.to_string()))?;
        let theme_source = self.required_text("theme.rhai")?;
        let mut modules = BTreeMap::new();
        let mut themes = Vec::new();
        let mut locales = Vec::new();
        let mut assets = BTreeMap::new();
        let mut fonts = Vec::new();
        for (path, bytes) in self.files() {
            let name = path.rsplit('/').next().unwrap_or(path);
            if name.starts_with('.') {
                continue;
            }
            if let Some(asset) = path.strip_prefix("assets/") {
                // Other files beside the images (a license) are no assets.
                let Some(mime_type) = crate::asset::mime_for_path(Path::new(asset)) else {
                    continue;
                };
                let logical = asset.rsplit_once('.').map_or(asset, |(stem, _)| stem);
                assets.insert(
                    logical.to_owned(),
                    AssetData {
                        mime_type: mime_type.to_owned(),
                        bytes: bytes.to_vec(),
                    },
                );
            } else if path.starts_with("fonts/") {
                let extension = name.rsplit('.').next().unwrap_or_default();
                if matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "ttf" | "otf" | "ttc"
                ) {
                    fonts.push(FontSource::new(name, bytes.to_vec())?);
                }
            } else if let Some(stem) = path.strip_suffix(".rhai") {
                let source = Self::text(path, bytes)?;
                let label = format!("ui/{path}");
                if path.starts_with("themes/") {
                    themes.push((label, source));
                } else if path.starts_with("locales/") {
                    locales.push((label, source));
                } else if !matches!(
                    path,
                    "theme.rhai" | "styles.rhai" | TOKEN_BASE_FILE | PROFILE_FILE
                ) {
                    modules.insert(ModuleId::parse(stem)?, source);
                }
            }
        }
        let mut sources = UiSources::new(
            manifest.entry.clone(),
            EmbeddedScriptSource::new(modules),
            theme_source,
        );
        sources.manifest = manifest;
        sources.themes = themes;
        sources.locales = locales;
        sources.assets = assets;
        sources.component_style_source = self.optional_text("styles.rhai")?;
        sources.profile_source = self.optional_text(PROFILE_FILE)?;
        Ok(EmbeddedTree {
            sources,
            token_base: self.optional_text(TOKEN_BASE_FILE)?,
            fonts,
        })
    }
}
