//! `gpui-rhai bundle` and `gpui-rhai dev --bundle`: a macOS application
//! bundle around the application's binaries.
//!
//! The configuration lives in the application's Cargo.toml:
//!
//! ```toml
//! [package.metadata.gpui-rhai.bundle]
//! name = "Disk Tree"
//! identifier = "com.example.disktree"
//! icon = "bundle/AppIcon.png"
//! binaries = ["disktree", "disktree-helper"]
//! resources = [{ from = "licenses", to = "licenses" }]
//! ```
//!
//! Command-line options take precedence. The first binary is the bundle's
//! executable; `licenses/` at the project root goes into the bundle's
//! resources unless the configuration lists resources of its own.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::ProjectError;

/// The oldest macOS GPUI runs on.
const MINIMUM_SYSTEM_VERSION: &str = "10.15.7";

/// `[package.metadata.gpui-rhai.bundle]` as written in Cargo.toml.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleMetadata {
    name: Option<String>,
    identifier: Option<String>,
    icon: Option<PathBuf>,
    #[serde(default)]
    binaries: Vec<String>,
    resources: Option<Vec<BundleResource>>,
    minimum_system_version: Option<String>,
}

/// A file or directory copied into `Contents/Resources`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BundleResource {
    /// Relative to the project root.
    pub from: PathBuf,
    /// Relative to `Contents/Resources`.
    pub to: PathBuf,
}

/// Command-line overrides of the bundle configuration.
#[derive(Clone, Debug, Default)]
pub struct BundleOptions {
    pub name: Option<String>,
    pub identifier: Option<String>,
    pub icon: Option<PathBuf>,
    /// A code signing identity; ad-hoc (`-`) when absent.
    pub sign: Option<String>,
}

/// The resolved bundle: what the configuration, the overrides and the
/// package's Cargo metadata say.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundlePlan {
    pub package: String,
    pub name: String,
    pub identifier: String,
    pub version: String,
    pub icon: Option<PathBuf>,
    pub binaries: Vec<String>,
    pub resources: Vec<BundleResource>,
    pub minimum_system_version: String,
    pub sign: String,
    pub target_directory: PathBuf,
}

fn bundle_error(message: impl Into<String>) -> ProjectError {
    ProjectError::Bundle(message.into())
}

impl BundlePlan {
    /// Resolve the bundle of the package at `root` from `cargo metadata`.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectError::Bundle`] when Cargo cannot describe the package
    /// or the configuration lacks an identifier or names no binary.
    pub fn resolve(root: &Path, options: &BundleOptions) -> Result<Self, ProjectError> {
        let output = Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--no-deps"])
            .current_dir(root)
            .output()
            .map_err(ProjectError::Spawn)?;
        if !output.status.success() {
            return Err(bundle_error(format!(
                "cargo metadata failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| bundle_error(format!("cargo metadata: {error}")))?;
        let manifest = fs_canonical(&root.join("Cargo.toml"))?;
        let package = metadata["packages"]
            .as_array()
            .and_then(|packages| {
                packages.iter().find(|package| {
                    package["manifest_path"]
                        .as_str()
                        .is_some_and(|path| Path::new(path) == manifest)
                })
            })
            .ok_or_else(|| {
                bundle_error(format!(
                    "{} is not a package manifest (a workspace root bundles one of its members: run bundle in the member's directory)",
                    manifest.display()
                ))
            })?;
        let target_directory = PathBuf::from(
            metadata["target_directory"]
                .as_str()
                .ok_or_else(|| bundle_error("cargo metadata has no target directory"))?,
        );
        Self::from_package(root, package, target_directory, options)
    }

    fn from_package(
        root: &Path,
        package: &serde_json::Value,
        target_directory: PathBuf,
        options: &BundleOptions,
    ) -> Result<Self, ProjectError> {
        let name = package["name"].as_str().unwrap_or_default().to_owned();
        let version = package["version"].as_str().unwrap_or_default().to_owned();
        let config: BundleMetadata = match package["metadata"]["gpui-rhai"].get("bundle") {
            Some(value) => serde_json::from_value(value.clone()).map_err(|error| {
                bundle_error(format!("[package.metadata.gpui-rhai.bundle]: {error}"))
            })?,
            None => BundleMetadata::default(),
        };
        let identifier = options
            .identifier
            .clone()
            .or(config.identifier)
            .ok_or_else(|| {
                bundle_error(
                    "set `identifier` in [package.metadata.gpui-rhai.bundle] or pass --identifier, such as com.example.app",
                )
            })?;
        validate_identifier(&identifier)?;
        let binaries = if config.binaries.is_empty() {
            let bins = package["targets"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|target| {
                    target["kind"]
                        .as_array()
                        .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
                })
                .filter_map(|target| target["name"].as_str().map(ToOwned::to_owned))
                .collect::<Vec<_>>();
            match bins.as_slice() {
                [_] => bins,
                [] => return Err(bundle_error(format!("package `{name}` has no binary"))),
                _ => {
                    return Err(bundle_error(format!(
                        "package `{name}` has {} binaries ({}); list them in `binaries`, the bundle's executable first",
                        bins.len(),
                        bins.join(", ")
                    )));
                }
            }
        } else {
            config.binaries
        };
        let resources = config.resources.unwrap_or_else(|| {
            root.join("licenses")
                .is_dir()
                .then(|| BundleResource {
                    from: PathBuf::from("licenses"),
                    to: PathBuf::from("licenses"),
                })
                .into_iter()
                .collect()
        });
        Ok(Self {
            name: options
                .name
                .clone()
                .or(config.name)
                .unwrap_or_else(|| name.clone()),
            package: name,
            identifier,
            version,
            icon: options.icon.clone().or(config.icon),
            binaries,
            resources,
            minimum_system_version: config
                .minimum_system_version
                .unwrap_or_else(|| MINIMUM_SYSTEM_VERSION.to_owned()),
            sign: options.sign.clone().unwrap_or_else(|| "-".to_owned()),
            target_directory,
        })
    }

    /// The bundle's `Info.plist`.
    #[must_use]
    pub fn info_plist(&self) -> String {
        let mut entries = vec![
            ("CFBundleDevelopmentRegion", "en".to_owned()),
            ("CFBundleDisplayName", self.name.clone()),
            ("CFBundleExecutable", self.binaries[0].clone()),
            ("CFBundleIdentifier", self.identifier.clone()),
            ("CFBundleInfoDictionaryVersion", "6.0".to_owned()),
            ("CFBundleName", self.name.clone()),
            ("CFBundlePackageType", "APPL".to_owned()),
            ("CFBundleShortVersionString", self.version.clone()),
            ("CFBundleVersion", self.version.clone()),
            (
                "LSMinimumSystemVersion",
                self.minimum_system_version.clone(),
            ),
        ];
        if self.icon.is_some() {
            entries.push(("CFBundleIconFile", "AppIcon".to_owned()));
        }
        entries.sort_by_key(|(key, _)| *key);
        let mut plist = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n",
        );
        for (key, value) in entries {
            let _ = writeln!(
                plist,
                "    <key>{key}</key>\n    <string>{}</string>",
                xml_escape(&value)
            );
        }
        plist.push_str("    <key>NSHighResolutionCapable</key>\n    <true/>\n</dict>\n</plist>\n");
        plist
    }

    /// Build the binaries with Cargo, release or debug, and assemble and sign
    /// `<target>/<profile>/bundle/<name>.app`. Returns the bundle's path.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectError`] when a build, a copy, the icon conversion or
    /// signing fails.
    pub fn build(&self, root: &Path, release: bool) -> Result<PathBuf, ProjectError> {
        let profile = if release { "release" } else { "debug" };
        let mut cargo = Command::new("cargo");
        cargo
            .arg("build")
            .current_dir(root)
            .args(["-p", &self.package]);
        if release {
            cargo.arg("--release");
        }
        for binary in &self.binaries {
            cargo.args(["--bin", binary]);
        }
        run(&mut cargo, "cargo build")?;

        let app = self
            .target_directory
            .join(profile)
            .join("bundle")
            .join(format!("{}.app", self.name));
        if app.exists() {
            std::fs::remove_dir_all(&app).map_err(|source| io_error(&app, source))?;
        }
        let contents = app.join("Contents");
        let executables = contents.join("MacOS");
        let resources = contents.join("Resources");
        for directory in [&executables, &resources] {
            std::fs::create_dir_all(directory).map_err(|source| io_error(directory, source))?;
        }
        for binary in &self.binaries {
            let from = self.target_directory.join(profile).join(binary);
            copy(&from, &executables.join(binary))?;
        }
        write(&contents.join("Info.plist"), &self.info_plist())?;
        if let Some(icon) = &self.icon {
            icns(&root.join(icon), &resources.join("AppIcon.icns"), &app)?;
        }
        for resource in &self.resources {
            copy_tree(&root.join(&resource.from), &resources.join(&resource.to))?;
        }
        run(
            Command::new("codesign")
                .args(["--force", "--deep", "--sign", &self.sign])
                .arg(&app),
            "codesign",
        )?;
        run(
            Command::new("codesign")
                .args(["--verify", "--deep", "--strict"])
                .arg(&app),
            "codesign --verify",
        )?;
        Ok(app)
    }
}

fn fs_canonical(path: &Path) -> Result<PathBuf, ProjectError> {
    std::fs::canonicalize(path).map_err(|source| io_error(path, source))
}

fn io_error(path: &Path, source: std::io::Error) -> ProjectError {
    ProjectError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn validate_identifier(identifier: &str) -> Result<(), ProjectError> {
    let valid = identifier.split('.').count() >= 2
        && identifier.split('.').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
        });
    if valid {
        Ok(())
    } else {
        Err(bundle_error(format!(
            "bundle identifier `{identifier}` must be reverse-DNS: letters, digits and `-` in dot-separated parts, such as com.example.app"
        )))
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn run(command: &mut Command, what: &str) -> Result<(), ProjectError> {
    let status = command.status().map_err(ProjectError::Spawn)?;
    if status.success() {
        Ok(())
    } else {
        Err(bundle_error(format!("{what} failed ({status})")))
    }
}

fn copy(from: &Path, to: &Path) -> Result<(), ProjectError> {
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|source| io_error(from, source))
}

fn write(path: &Path, contents: &str) -> Result<(), ProjectError> {
    std::fs::write(path, contents).map_err(|source| io_error(path, source))
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), ProjectError> {
    if from.is_dir() {
        std::fs::create_dir_all(to).map_err(|source| io_error(to, source))?;
        for entry in std::fs::read_dir(from).map_err(|source| io_error(from, source))? {
            let entry = entry.map_err(|source| io_error(from, source))?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
        }
        copy(from, to)
    }
}

/// Convert a square PNG, ideally 1024px, into `.icns` with macOS's `sips` and
/// `iconutil`.
fn icns(png: &Path, output: &Path, app: &Path) -> Result<(), ProjectError> {
    let iconset = app.with_extension("iconset");
    std::fs::create_dir_all(&iconset).map_err(|source| io_error(&iconset, source))?;
    for size in [16, 32, 128, 256, 512] {
        for (scale, suffix) in [(1, ""), (2, "@2x")] {
            let pixels = (size * scale).to_string();
            run(
                Command::new("sips")
                    .args(["-z", &pixels, &pixels])
                    .arg(png)
                    .arg("--out")
                    .arg(iconset.join(format!("icon_{size}x{size}{suffix}.png")))
                    .stdout(std::process::Stdio::null()),
                "sips",
            )?;
        }
    }
    run(
        Command::new("iconutil")
            .args(["-c", "icns"])
            .arg(&iconset)
            .arg("-o")
            .arg(output),
        "iconutil",
    )?;
    std::fs::remove_dir_all(&iconset).map_err(|source| io_error(&iconset, source))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(metadata: &serde_json::Value, bins: &[&str]) -> serde_json::Value {
        serde_json::json!({
            "name": "demo",
            "version": "1.2.3",
            "metadata": { "gpui-rhai": { "bundle": metadata } },
            "targets": bins
                .iter()
                .map(|name| serde_json::json!({ "name": name, "kind": ["bin"] }))
                .collect::<Vec<_>>(),
        })
    }

    #[test]
    fn the_configuration_and_the_flags_resolve_one_bundle() {
        let root = Path::new("/nonexistent");
        let plan = BundlePlan::from_package(
            root,
            &package(
                &serde_json::json!({ "name": "Demo & Co", "identifier": "com.example.demo" }),
                &["demo"],
            ),
            PathBuf::from("/target"),
            &BundleOptions::default(),
        )
        .unwrap();
        assert_eq!(plan.name, "Demo & Co");
        assert_eq!(plan.binaries, ["demo"]);
        assert_eq!(plan.sign, "-");
        assert_eq!(plan.resources, []);
        let plist = plan.info_plist();
        assert!(plist.contains("<string>Demo &amp; Co</string>"), "{plist}");
        assert!(plist.contains("<key>CFBundleExecutable</key>\n    <string>demo</string>"));
        assert!(
            plist.contains("<key>CFBundleShortVersionString</key>\n    <string>1.2.3</string>")
        );
        assert!(!plist.contains("CFBundleIconFile"));

        let flags = BundleOptions {
            identifier: Some("org.example.flag".to_owned()),
            icon: Some(PathBuf::from("icon.png")),
            sign: Some("Developer ID Application".to_owned()),
            ..BundleOptions::default()
        };
        let plan = BundlePlan::from_package(
            root,
            &package(
                &serde_json::json!({ "identifier": "com.example.demo" }),
                &["demo"],
            ),
            PathBuf::from("/target"),
            &flags,
        )
        .unwrap();
        assert_eq!(plan.identifier, "org.example.flag");
        assert_eq!(plan.name, "demo");
        assert!(plan.info_plist().contains("<string>AppIcon</string>"));
    }

    #[test]
    fn a_bundle_needs_an_identifier_and_one_named_executable() {
        let root = Path::new("/nonexistent");
        let resolve = |metadata, bins: &[&str]| {
            BundlePlan::from_package(
                root,
                &package(&metadata, bins),
                PathBuf::from("/target"),
                &BundleOptions::default(),
            )
            .map(|plan| plan.binaries)
            .map_err(|error| error.to_string())
        };
        assert!(
            resolve(serde_json::json!({}), &["demo"])
                .unwrap_err()
                .contains("identifier")
        );
        assert!(
            resolve(serde_json::json!({ "identifier": "demo" }), &["demo"])
                .unwrap_err()
                .contains("reverse-DNS")
        );
        assert!(
            resolve(
                serde_json::json!({ "identifier": "com.example.demo" }),
                &["a", "b"]
            )
            .unwrap_err()
            .contains("list them in `binaries`")
        );
        assert_eq!(
            resolve(
                serde_json::json!({ "identifier": "com.example.demo", "binaries": ["b", "a"] }),
                &["a", "b"]
            )
            .unwrap(),
            ["b", "a"]
        );
        assert!(
            resolve(
                serde_json::json!({ "identifier": "com.example.demo", "icn": "x" }),
                &["a"]
            )
            .unwrap_err()
            .contains("unknown field")
        );
    }
}
