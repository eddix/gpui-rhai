use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::Command;

use gpui_rhai::{
    AppManifest, ComponentDefinition, ComponentMetadata, ComponentRegistry, EmbeddedScriptSource,
    LocaleManager, ModuleId, RUNTIME_API_VERSION, RestrictedModuleResolver, RuntimeEngine,
    ScriptAsset, ThemeManager, ThemeSelection, load_locale_source, load_theme_source,
    parse_component_header,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};

const BUTTON_SOURCE: &str = include_str!("../../../registry/components/button.rhai");
const LABEL_SOURCE: &str = include_str!("../../../registry/components/label.rhai");
const ICON_SOURCE: &str = include_str!("../../../registry/components/icon.rhai");
const INPUT_SOURCE: &str = include_str!("../../../registry/components/input.rhai");
const TEXTAREA_SOURCE: &str = include_str!("../../../registry/components/textarea.rhai");
const DIVIDER_SOURCE: &str = include_str!("../../../registry/components/divider.rhai");
const POPOVER_SOURCE: &str = include_str!("../../../registry/components/popover.rhai");
const DIALOG_SOURCE: &str = include_str!("../../../registry/components/dialog.rhai");
const DROPDOWN_SOURCE: &str = include_str!("../../../registry/components/dropdown.rhai");
const SELECT_SOURCE: &str = include_str!("../../../registry/components/select.rhai");
const DATE_PICKER_SOURCE: &str = include_str!("../../../registry/components/date_picker.rhai");
const TABLE_SOURCE: &str = include_str!("../../../registry/components/table.rhai");
const PAGINATION_SOURCE: &str = include_str!("../../../registry/components/pagination.rhai");
const CHECKBOX_SOURCE: &str = include_str!("../../../registry/components/checkbox.rhai");
const RADIO_SOURCE: &str = include_str!("../../../registry/components/radio.rhai");
const RADIO_GROUP_SOURCE: &str = include_str!("../../../registry/components/radio_group.rhai");
const SWITCH_SOURCE: &str = include_str!("../../../registry/components/switch.rhai");
const TAG_SOURCE: &str = include_str!("../../../registry/components/tag.rhai");
const AVATAR_SOURCE: &str = include_str!("../../../registry/components/avatar.rhai");
const PROGRESS_SOURCE: &str = include_str!("../../../registry/components/progress.rhai");
const SKELETON_SOURCE: &str = include_str!("../../../registry/components/skeleton.rhai");
const FORM_FIELD_SOURCE: &str = include_str!("../../../registry/components/form_field.rhai");
const COLLAPSIBLE_SOURCE: &str = include_str!("../../../registry/components/collapsible.rhai");
const ACCORDION_SOURCE: &str = include_str!("../../../registry/components/accordion.rhai");
const TABS_SOURCE: &str = include_str!("../../../registry/components/tabs.rhai");
const TOOLTIP_SOURCE: &str = include_str!("../../../registry/components/tooltip.rhai");
const MENU_SOURCE: &str = include_str!("../../../registry/components/menu.rhai");
const TOAST_SOURCE: &str = include_str!("../../../registry/components/toast.rhai");
const CHECK_SVG: &str = include_str!("../../../registry/assets/icons/check.svg");
const CLOSE_SVG: &str = include_str!("../../../registry/assets/icons/close.svg");
const CHEVRON_LEFT_SVG: &str = include_str!("../../../registry/assets/icons/chevron_left.svg");
const CHEVRON_RIGHT_SVG: &str = include_str!("../../../registry/assets/icons/chevron_right.svg");
const CALENDAR_SVG: &str = include_str!("../../../registry/assets/icons/calendar.svg");
const DATE_PREVIOUS_SVG: &str = include_str!("../../../registry/assets/icons/date_previous.svg");
const DATE_NEXT_SVG: &str = include_str!("../../../registry/assets/icons/date_next.svg");
const DEFAULT_THEME: &str = include_str!("../../../registry/themes/default_dark.rhai");
const DEFAULT_LIGHT_THEME: &str = include_str!("../../../registry/themes/default_light.rhai");
const TOKYO_NIGHT_THEME: &str = include_str!("../../../registry/themes/tokyo_night.rhai");
const TOKYO_STORM_THEME: &str = include_str!("../../../registry/themes/tokyo_storm.rhai");
const CATPPUCCIN_LATTE_THEME: &str = include_str!("../../../registry/themes/catppuccin_latte.rhai");
const CATPPUCCIN_MOCHA_THEME: &str = include_str!("../../../registry/themes/catppuccin_mocha.rhai");
const EN_LOCALE: &str = include_str!("../../../registry/locales/en.rhai");
const ZH_CN_LOCALE: &str = include_str!("../../../registry/locales/zh_cn.rhai");
const AR_LOCALE: &str = include_str!("../../../registry/locales/ar.rhai");

#[derive(Clone, Debug)]
struct RegistryEntry {
    metadata: ComponentMetadata,
    source: &'static str,
    assets: &'static [RegistryAsset],
}

#[derive(Clone, Debug)]
struct RegistryAsset {
    path: &'static str,
    source: &'static str,
}

const ICON_ASSETS: &[RegistryAsset] = &[
    RegistryAsset {
        path: "icons/check.svg",
        source: CHECK_SVG,
    },
    RegistryAsset {
        path: "icons/close.svg",
        source: CLOSE_SVG,
    },
];

const PAGINATION_ASSETS: &[RegistryAsset] = &[
    RegistryAsset {
        path: "icons/chevron_left.svg",
        source: CHEVRON_LEFT_SVG,
    },
    RegistryAsset {
        path: "icons/chevron_right.svg",
        source: CHEVRON_RIGHT_SVG,
    },
];

const DATE_PICKER_ASSETS: &[RegistryAsset] = &[
    RegistryAsset {
        path: "icons/calendar.svg",
        source: CALENDAR_SVG,
    },
    RegistryAsset {
        path: "icons/date_previous.svg",
        source: DATE_PREVIOUS_SVG,
    },
    RegistryAsset {
        path: "icons/date_next.svg",
        source: DATE_NEXT_SVG,
    },
    RegistryAsset {
        path: "icons/close.svg",
        source: CLOSE_SVG,
    },
];

const SELECT_ASSETS: &[RegistryAsset] = &[];

const TABLE_ASSETS: &[RegistryAsset] = &[];

#[derive(Clone, Debug, Default)]
pub struct BundledRegistry {
    entries: BTreeMap<ModuleId, RegistryEntry>,
}

impl BundledRegistry {
    /// Load and validate the CLI's built-in registry snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectError`] when a bundled component header is invalid or
    /// duplicates another component ID.
    pub fn load() -> Result<Self, ProjectError> {
        let mut registry = Self::default();
        for (source, assets) in [
            (BUTTON_SOURCE, &[][..]),
            (LABEL_SOURCE, &[][..]),
            (ICON_SOURCE, ICON_ASSETS),
            (INPUT_SOURCE, &[][..]),
            (TEXTAREA_SOURCE, &[][..]),
            (DIVIDER_SOURCE, &[][..]),
            (POPOVER_SOURCE, &[][..]),
            (DIALOG_SOURCE, &[][..]),
            (DROPDOWN_SOURCE, SELECT_ASSETS),
            (SELECT_SOURCE, SELECT_ASSETS),
            (DATE_PICKER_SOURCE, DATE_PICKER_ASSETS),
            (TABLE_SOURCE, TABLE_ASSETS),
            (PAGINATION_SOURCE, PAGINATION_ASSETS),
            (CHECKBOX_SOURCE, &[][..]),
            (RADIO_SOURCE, &[][..]),
            (RADIO_GROUP_SOURCE, &[][..]),
            (SWITCH_SOURCE, &[][..]),
            (TAG_SOURCE, &[][..]),
            (AVATAR_SOURCE, &[][..]),
            (PROGRESS_SOURCE, &[][..]),
            (SKELETON_SOURCE, &[][..]),
            (FORM_FIELD_SOURCE, &[][..]),
            (COLLAPSIBLE_SOURCE, &[][..]),
            (ACCORDION_SOURCE, &[][..]),
            (TABS_SOURCE, &[][..]),
            (TOOLTIP_SOURCE, &[][..]),
            (MENU_SOURCE, &[][..]),
            (TOAST_SOURCE, &[][..]),
        ] {
            let metadata = parse_component_header(source)?;
            let id = metadata.id.clone();
            validate_component_documentation(source, &id)?;
            let declared_assets = metadata
                .assets
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            let bundled_assets = assets
                .iter()
                .map(|asset| asset.path)
                .collect::<BTreeSet<_>>();
            if declared_assets != bundled_assets {
                return Err(ProjectError::RegistryAssetMismatch {
                    component: id,
                    declared: declared_assets.into_iter().map(ToOwned::to_owned).collect(),
                    bundled: bundled_assets.into_iter().map(ToOwned::to_owned).collect(),
                });
            }
            let id = metadata.id.clone();
            if registry
                .entries
                .insert(
                    id.clone(),
                    RegistryEntry {
                        metadata,
                        source,
                        assets,
                    },
                )
                .is_some()
            {
                return Err(ProjectError::DuplicateRegistry(id));
            }
        }
        Ok(registry)
    }

    fn resolve(&self, requested: &[String]) -> Result<Vec<ModuleId>, ProjectError> {
        let mut ordered = Vec::new();
        let mut complete = BTreeSet::new();
        let mut stack = Vec::new();
        for request in requested {
            let normalized = if request.contains('/') {
                request.clone()
            } else {
                format!("components/{request}")
            };
            let id = ModuleId::parse(normalized)?;
            self.visit(&id, &mut stack, &mut complete, &mut ordered)?;
        }
        Ok(ordered)
    }

    fn visit(
        &self,
        id: &ModuleId,
        stack: &mut Vec<ModuleId>,
        complete: &mut BTreeSet<ModuleId>,
        ordered: &mut Vec<ModuleId>,
    ) -> Result<(), ProjectError> {
        if complete.contains(id) {
            return Ok(());
        }
        if let Some(start) = stack.iter().position(|active| active == id) {
            let mut cycle = stack[start..].to_vec();
            cycle.push(id.clone());
            return Err(ProjectError::RegistryCycle(cycle));
        }
        let entry = self
            .entries
            .get(id)
            .ok_or_else(|| ProjectError::UnknownComponent(id.clone()))?;
        stack.push(id.clone());
        for dependency in &entry.metadata.dependencies {
            self.visit(dependency, stack, complete, ordered)?;
        }
        stack.pop();
        complete.insert(id.clone());
        ordered.push(id.clone());
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Project {
    root: PathBuf,
}

impl Project {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Plan a non-destructive project initialization.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectError`] for missing/invalid Cargo metadata or conflicting
    /// generated files.
    pub fn plan_init(&self) -> Result<ProjectPlan, ProjectError> {
        let cargo_path = self.root.join("Cargo.toml");
        let cargo_source = read(&cargo_path)?;
        let mut cargo =
            cargo_source
                .parse::<DocumentMut>()
                .map_err(|source| ProjectError::CargoToml {
                    path: cargo_path.clone(),
                    source,
                })?;
        if cargo
            .get("dependencies")
            .and_then(|dependencies| dependencies.get("gpui-rhai"))
            .is_none()
        {
            let mut features = Array::new();
            features.push("dev-reload");
            let mut dependency = InlineTable::new();
            dependency.insert("version", Value::from("0.1"));
            dependency.insert("features", Value::Array(features));
            cargo["dependencies"]["gpui-rhai"] = Item::Value(Value::InlineTable(dependency));
        }

        let mut plan = ProjectPlan::new(self.root.clone());
        plan.update(cargo_path, cargo.to_string(), cargo_source);
        let main_path = self.root.join("src/main.rs");
        if main_path.exists() {
            plan.create(self.root.join("gpui-rhai-host-snippet.rs"), host_source())?;
        } else {
            plan.create(main_path, host_source())?;
        }
        plan.create(self.root.join("ui/main.rhai"), starter_ui())?;
        plan.create(self.root.join("ui/theme.rhai"), DEFAULT_THEME.to_owned())?;
        for (name, source) in [
            ("default_light.rhai", DEFAULT_LIGHT_THEME),
            ("tokyo_night.rhai", TOKYO_NIGHT_THEME),
            ("tokyo_storm.rhai", TOKYO_STORM_THEME),
            ("catppuccin_latte.rhai", CATPPUCCIN_LATTE_THEME),
            ("catppuccin_mocha.rhai", CATPPUCCIN_MOCHA_THEME),
        ] {
            plan.create(self.root.join("ui/themes").join(name), source.to_owned())?;
        }
        plan.create(self.root.join("ui/locales/en.rhai"), EN_LOCALE.to_owned())?;
        plan.create(
            self.root.join("ui/locales/zh_cn.rhai"),
            ZH_CN_LOCALE.to_owned(),
        )?;
        plan.create(self.root.join("ui/locales/ar.rhai"), AR_LOCALE.to_owned())?;
        plan.create(self.root.join("ui/app.toml"), app_manifest_source()?)?;
        plan.create(
            self.root.join(".gpui-rhai/manifest.toml"),
            toml::to_string_pretty(&LocalManifest::default())?,
        )?;
        Ok(plan)
    }

    /// Plan installation of components and transitive dependencies.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectError`] for missing manifests, unknown components,
    /// compatibility failures, or existing untracked source.
    pub fn plan_add(
        &self,
        registry: &BundledRegistry,
        requested: &[String],
    ) -> Result<ProjectPlan, ProjectError> {
        if requested.is_empty() {
            return Err(ProjectError::NoComponents);
        }
        let manifest_path = self.root.join(".gpui-rhai/manifest.toml");
        let manifest_source = read(&manifest_path)?;
        let mut manifest: LocalManifest = toml::from_str(&manifest_source)?;
        let resolved = registry.resolve(requested)?;
        let mut plan = ProjectPlan::new(self.root.clone());
        for id in resolved {
            if manifest.components.contains_key(id.as_str()) {
                continue;
            }
            let entry = registry
                .entries
                .get(&id)
                .ok_or_else(|| ProjectError::UnknownComponent(id.clone()))?;
            if !entry.metadata.runtime_api.contains(RUNTIME_API_VERSION) {
                return Err(ProjectError::IncompatibleRuntime(id));
            }
            let relative = component_relative_path(&id)?;
            plan.create(
                self.root.join("ui").join(&relative),
                entry.source.to_owned(),
            )?;
            plan.create(
                self.root.join(".gpui-rhai/baselines").join(&relative),
                entry.source.to_owned(),
            )?;
            for asset in entry.assets {
                plan.create(
                    self.root.join("ui/assets").join(asset.path),
                    asset.source.to_owned(),
                )?;
                plan.create(
                    self.root
                        .join(".gpui-rhai/baselines/assets")
                        .join(asset.path),
                    asset.source.to_owned(),
                )?;
            }
            let hash = ScriptAsset::new(id.clone(), entry.source.to_owned()).content_hash;
            manifest.components.insert(
                id.as_str().to_owned(),
                InstalledComponent {
                    version: entry.metadata.version.clone(),
                    content_hash: format!("{hash:016x}"),
                    dependencies: entry
                        .metadata
                        .dependencies
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                },
            );
        }
        plan.update(
            manifest_path,
            toml::to_string_pretty(&manifest)?,
            manifest_source,
        );
        Ok(plan)
    }

    /// Validate installed baselines, headers, themes, app manifest, imports, and
    /// the required `view(ctx)` function.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectError`] for any invalid artifact.
    pub fn check(&self) -> Result<CheckReport, ProjectError> {
        let manifest_path = self.root.join(".gpui-rhai/manifest.toml");
        let manifest: LocalManifest = toml::from_str(&read(&manifest_path)?)?;
        if manifest.runtime_api != RUNTIME_API_VERSION {
            return Err(ProjectError::ManifestRuntime {
                required: manifest.runtime_api,
                actual: RUNTIME_API_VERSION,
            });
        }
        let mut modules = BTreeMap::new();
        let mut headers = BTreeMap::new();
        for (raw_id, installed) in &manifest.components {
            let id = ModuleId::parse(raw_id.clone())?;
            let relative = component_relative_path(&id)?;
            let baseline_source = read(&self.root.join(".gpui-rhai/baselines").join(&relative))?;
            let baseline_hash = ScriptAsset::new(id.clone(), baseline_source).content_hash;
            if format!("{baseline_hash:016x}") != installed.content_hash {
                return Err(ProjectError::BaselineHash(id));
            }
            let source = read(&self.root.join("ui").join(relative))?;
            let header = parse_component_header(&source)?;
            validate_component_documentation(&source, &id)?;
            if header.id != id || header.version != installed.version {
                return Err(ProjectError::InstalledMetadata(id));
            }
            headers.insert(id.clone(), header);
            modules.insert(id, source);
        }

        let components = validate_component_exports(&modules, &headers)?;
        validate_entry(&self.root, &modules)?;
        let app: AppManifest = toml::from_str(&read(&self.root.join("ui/app.toml"))?)?;
        if app.entry.as_str() != "main" {
            return Err(ProjectError::ManifestEntry(app.entry));
        }
        if app.runtime_api != RUNTIME_API_VERSION {
            return Err(ProjectError::ManifestRuntime {
                required: app.runtime_api,
                actual: RUNTIME_API_VERSION,
            });
        }
        app.validate_components(&components)?;
        let theme_source = read(&self.root.join("ui/theme.rhai"))?;
        let theme_runtime = RuntimeEngine::new();
        let primary = load_theme_source(theme_runtime.engine(), "ui/theme.rhai", &theme_source)
            .map_err(|error| ProjectError::Theme(error.to_string()))?;
        let mut variants = vec![primary.clone()];
        for path in collect_paths(&self.root.join("ui/themes"))? {
            let source = read(&path)?;
            variants.push(
                load_theme_source(theme_runtime.engine(), &path.to_string_lossy(), &source)
                    .map_err(|error| ProjectError::Theme(error.to_string()))?,
            );
        }
        ThemeManager::from_variants(variants, ThemeSelection::new(primary.family, primary.name))
            .map_err(|error| ProjectError::Theme(error.to_string()))?;
        validate_locales(&self.root)?;
        validate_embedded_assets(&self.root)?;
        Ok(CheckReport {
            components: manifest.components.len(),
            entry: app.entry,
        })
    }

    /// Generate schema-driven component metadata and basic editor snippets.
    ///
    /// # Errors
    ///
    /// Returns manifest, source, compilation, export, or serialization errors.
    pub fn plan_editor_metadata(&self) -> Result<ProjectPlan, ProjectError> {
        let manifest: LocalManifest =
            toml::from_str(&read(&self.root.join(".gpui-rhai/manifest.toml"))?)?;
        let mut modules = BTreeMap::new();
        let mut headers = BTreeMap::new();
        for raw_id in manifest.components.keys() {
            let id = ModuleId::parse(raw_id.clone())?;
            let source = read(&self.root.join("ui").join(component_relative_path(&id)?))?;
            headers.insert(id.clone(), parse_component_header(&source)?);
            modules.insert(id, source);
        }
        let registry = validate_component_exports(&modules, &headers)?;
        let metadata = EditorMetadata {
            runtime_api: RUNTIME_API_VERSION,
            components: registry
                .iter()
                .map(|(_, component)| component.clone())
                .collect(),
        };
        let snippets = editor_snippets(&registry);
        let mut plan = ProjectPlan::new(self.root.clone());
        plan.replace_or_create(
            self.root.join(".gpui-rhai/editor/components.json"),
            format!("{}\n", serde_json::to_string_pretty(&metadata)?),
        )?;
        plan.replace_or_create(
            self.root.join(".gpui-rhai/editor/snippets.json"),
            format!("{}\n", serde_json::to_string_pretty(&snippets)?),
        )?;
        Ok(plan)
    }

    /// Compare editable files with their committed baselines.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectError`] when project metadata or files cannot be read.
    pub fn diff(&self) -> Result<Vec<String>, ProjectError> {
        let manifest: LocalManifest =
            toml::from_str(&read(&self.root.join(".gpui-rhai/manifest.toml"))?)?;
        manifest
            .components
            .keys()
            .map(|raw_id| {
                let id = ModuleId::parse(raw_id.clone())?;
                let relative = component_relative_path(&id)?;
                let source = read(&self.root.join("ui").join(&relative))?;
                let baseline = read(&self.root.join(".gpui-rhai/baselines").join(relative))?;
                Ok(format!(
                    "{raw_id}: {}",
                    if source == baseline {
                        "unchanged"
                    } else {
                        "modified"
                    }
                ))
            })
            .collect()
    }

    /// Plan safe updates from the bundled registry.
    ///
    /// Locally modified files are three-way merged against their install
    /// baseline. Conflicts leave the editable source and baseline untouched and
    /// create an inspectable artifact under `.gpui-rhai/conflicts/`.
    ///
    /// # Errors
    ///
    /// Returns I/O, manifest, or path errors. Merge conflicts are recorded on
    /// the returned plan and are not silent overwrites.
    pub fn plan_update(&self, registry: &BundledRegistry) -> Result<ProjectPlan, ProjectError> {
        let manifest_path = self.root.join(".gpui-rhai/manifest.toml");
        let manifest_source = read(&manifest_path)?;
        let mut manifest: LocalManifest = toml::from_str(&manifest_source)?;
        let mut plan = ProjectPlan::new(self.root.clone());
        for (raw_id, installed) in &mut manifest.components {
            let id = ModuleId::parse(raw_id.clone())?;
            let Some(upstream) = registry.entries.get(&id) else {
                continue;
            };
            if upstream.metadata.version <= installed.version {
                continue;
            }
            let relative = component_relative_path(&id)?;
            let source_path = self.root.join("ui").join(&relative);
            let baseline_path = self.root.join(".gpui-rhai/baselines").join(&relative);
            let source = read(&source_path)?;
            let baseline = read(&baseline_path)?;
            let Ok(merged) = three_way_merge(&baseline, &source, upstream.source) else {
                let conflict_path = self.root.join(".gpui-rhai/conflicts").join(&relative);
                plan.replace_or_create(
                    conflict_path,
                    conflict_artifact(&source, &baseline, upstream.source),
                )?;
                plan.conflicts.push(id);
                continue;
            };
            plan.update(source_path, merged, source);
            plan.update(baseline_path, upstream.source.to_owned(), baseline);
            installed.version = upstream.metadata.version.clone();
            installed.content_hash = format!(
                "{:016x}",
                ScriptAsset::new(id, upstream.source.to_owned()).content_hash
            );
        }
        plan.update(
            manifest_path,
            toml::to_string_pretty(&manifest)?,
            manifest_source,
        );
        Ok(plan)
    }

    /// Generate a deterministic Rust module for production embedding.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectError`] when manifests or referenced resources cannot
    /// be read.
    pub fn plan_embed(&self) -> Result<ProjectPlan, ProjectError> {
        let manifest: LocalManifest =
            toml::from_str(&read(&self.root.join(".gpui-rhai/manifest.toml"))?)?;
        let app: AppManifest = toml::from_str(&read(&self.root.join("ui/app.toml"))?)?;
        if app.entry.as_str() != "main" {
            return Err(ProjectError::ManifestEntry(app.entry));
        }
        let mut modules = vec![(app.entry.to_string(), "../ui/main.rhai".to_owned())];
        for raw_id in manifest.components.keys() {
            let id = ModuleId::parse(raw_id.clone())?;
            let relative = component_relative_path(&id)?;
            modules.push((id.to_string(), format!("../ui/{}", slash_path(&relative))));
        }
        modules.sort();

        let locales = collect_relative_files(&self.root.join("ui/locales"), "../ui/locales")?;
        let themes = collect_relative_files(&self.root.join("ui/themes"), "../ui/themes")?;
        let assets = collect_relative_files(&self.root.join("ui/assets"), "../ui/assets")?;
        let generated = generated_embed_module(&app, &modules, &locales, &themes, &assets)?;
        let path = self.root.join("src/gpui_rhai_embedded.rs");
        let mut plan = ProjectPlan::new(self.root.clone());
        match fs::read_to_string(&path) {
            Ok(existing) => plan.update(path, generated, existing),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                plan.create(path, generated)?;
            }
            Err(source) => return Err(ProjectError::Io { path, source }),
        }
        Ok(plan)
    }

    /// Run the Cargo application in the project root.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectError`] when Cargo cannot start or exits unsuccessfully.
    pub fn dev(&self) -> Result<(), ProjectError> {
        let status = Command::new("cargo")
            .arg("run")
            .current_dir(&self.root)
            .status()
            .map_err(ProjectError::Spawn)?;
        if status.success() {
            Ok(())
        } else {
            Err(ProjectError::DevFailed(status.code()))
        }
    }
}

fn validate_component_documentation(source: &str, id: &ModuleId) -> Result<(), ProjectError> {
    let lower = source.to_ascii_lowercase();
    for (label, present) in [
        ("props", source.contains("// Props:")),
        (
            "state",
            lower.contains("state:") || lower.contains("state/"),
        ),
        (
            "events",
            lower.contains("event") || lower.contains("emits no events"),
        ),
        ("example", source.contains("// Example:")),
        (
            "render_component wrapper",
            source.contains("render_component("),
        ),
    ] {
        if !present {
            return Err(ProjectError::ComponentDocumentation {
                component: id.clone(),
                missing: label,
            });
        }
    }
    Ok(())
}

fn validate_locales(root: &Path) -> Result<(), ProjectError> {
    let engine = RuntimeEngine::new();
    let mut bundles = Vec::new();
    for path in collect_paths(&root.join("ui/locales"))? {
        let source = read(&path)?;
        bundles.push(
            load_locale_source(engine.engine(), &path.to_string_lossy(), &source)
                .map_err(|error| ProjectError::Locale(error.to_string()))?,
        );
    }
    if bundles.is_empty() {
        return Ok(());
    }
    let fallback = bundles
        .iter()
        .find(|bundle| bundle.locale == "en")
        .unwrap_or(&bundles[0])
        .locale
        .clone();
    LocaleManager::new(bundles, fallback.clone(), fallback)
        .map(|_| ())
        .map_err(|error| ProjectError::Locale(error.to_string()))
}

fn validate_embedded_assets(root: &Path) -> Result<(), ProjectError> {
    let assets = collect_relative_files(&root.join("ui/assets"), "../ui/assets")?;
    let mut logical = BTreeSet::new();
    for (name, _) in assets {
        embedded_asset_mime(&name)?;
        let name = name
            .rsplit_once('.')
            .map_or(name.as_str(), |(stem, _)| stem);
        if !logical.insert(name.to_owned()) {
            return Err(ProjectError::DuplicateAssetLogical(name.to_owned()));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
struct EditorMetadata {
    runtime_api: u32,
    components: Vec<ComponentDefinition>,
}

#[derive(Clone, Debug, Serialize)]
struct EditorSnippet {
    prefix: String,
    body: Vec<String>,
    description: String,
}

fn editor_snippets(registry: &ComponentRegistry) -> BTreeMap<String, EditorSnippet> {
    registry
        .iter()
        .map(|(id, component)| {
            let alias = id.as_str().rsplit('/').next().unwrap_or("component");
            let mut body = vec![
                format!("import \"{id}\" as {alias};"),
                format!("{alias}::{}(#{{", component.metadata.export),
            ];
            let mut placeholder = 1;
            for (name, field) in &component.schema.props {
                if field.required {
                    body.push(format!("    {name}: ${{{placeholder}:{name}}},"));
                    placeholder += 1;
                }
            }
            body.push("})".to_owned());
            (
                component.metadata.export.clone(),
                EditorSnippet {
                    prefix: component.metadata.export.clone(),
                    body,
                    description: format!(
                        "{} from {} {}",
                        component.metadata.export, id, component.metadata.version
                    ),
                },
            )
        })
        .collect()
}

fn validate_component_exports(
    modules: &BTreeMap<ModuleId, String>,
    headers: &BTreeMap<ModuleId, ComponentMetadata>,
) -> Result<ComponentRegistry, ProjectError> {
    if modules.is_empty() {
        return Ok(ComponentRegistry::default());
    }
    let source = EmbeddedScriptSource::new(modules.clone());
    let mut runtime = RuntimeEngine::new();
    runtime.set_module_resolver(RestrictedModuleResolver::from_source(&source)?);
    let imports = modules
        .keys()
        .enumerate()
        .map(|(index, id)| format!("import \"{id}\" as component_{index};"))
        .collect::<Vec<_>>()
        .join("\n");
    runtime.compile_self_contained_named("<component-check>", &imports)?;
    let exported = runtime.component_exports()?;
    for (id, header) in headers {
        exported
            .get(id)
            .ok_or_else(|| ProjectError::MissingExport(id.clone()))?
            .validate_header(header)?;
    }
    Ok(exported)
}

fn validate_entry(root: &Path, modules: &BTreeMap<ModuleId, String>) -> Result<(), ProjectError> {
    let entry_path = root.join("ui/main.rhai");
    let entry = read(&entry_path)?;
    let source = EmbeddedScriptSource::new(modules.clone());
    let mut runtime = RuntimeEngine::new();
    runtime.set_module_resolver(RestrictedModuleResolver::from_source(&source)?);
    let compiled = runtime.compile_self_contained_named("ui/main.rhai", &entry)?;
    if compiled.has_function("view", 1) {
        Ok(())
    } else {
        Err(ProjectError::MissingView)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LineChange {
    base: Range<usize>,
    replacement: Vec<String>,
}

fn three_way_merge(base: &str, local: &str, upstream: &str) -> Result<String, ()> {
    if local == base {
        return Ok(upstream.to_owned());
    }
    if upstream == base || local == upstream {
        return Ok(local.to_owned());
    }
    let base_lines = split_lines(base);
    let local_changes = line_changes(&base_lines, &split_lines(local))?;
    let upstream_changes = line_changes(&base_lines, &split_lines(upstream))?;
    for local in &local_changes {
        for upstream in &upstream_changes {
            if changes_overlap(local, upstream) && local != upstream {
                return Err(());
            }
        }
    }
    let mut changes = local_changes;
    for change in upstream_changes {
        if !changes.contains(&change) {
            changes.push(change);
        }
    }
    changes.sort_by_key(|change| (change.base.start, change.base.end));
    let mut output = String::new();
    let mut cursor = 0;
    for change in changes {
        if change.base.start < cursor {
            return Err(());
        }
        output.extend(
            base_lines[cursor..change.base.start]
                .iter()
                .map(String::as_str),
        );
        output.extend(change.replacement.iter().map(String::as_str));
        cursor = change.base.end;
    }
    output.extend(base_lines[cursor..].iter().map(String::as_str));
    Ok(output)
}

fn split_lines(source: &str) -> Vec<String> {
    source
        .split_inclusive('\n')
        .map(ToOwned::to_owned)
        .collect()
}

fn line_changes(base: &[String], target: &[String]) -> Result<Vec<LineChange>, ()> {
    const MAX_LCS_CELLS: usize = 4_000_000;
    if base.len().saturating_mul(target.len()) > MAX_LCS_CELLS {
        return Err(());
    }
    let mut lcs = vec![vec![0_u32; target.len().saturating_add(1)]; base.len().saturating_add(1)];
    for base_index in (0..base.len()).rev() {
        for target_index in (0..target.len()).rev() {
            lcs[base_index][target_index] = if base[base_index] == target[target_index] {
                lcs[base_index + 1][target_index + 1].saturating_add(1)
            } else {
                lcs[base_index + 1][target_index].max(lcs[base_index][target_index + 1])
            };
        }
    }
    let mut changes = Vec::new();
    let mut current: Option<LineChange> = None;
    let (mut base_index, mut target_index) = (0, 0);
    while base_index < base.len() || target_index < target.len() {
        if base_index < base.len()
            && target_index < target.len()
            && base[base_index] == target[target_index]
        {
            if let Some(change) = current.take() {
                changes.push(change);
            }
            base_index += 1;
            target_index += 1;
        } else if target_index < target.len()
            && (base_index == base.len()
                || lcs[base_index][target_index + 1] > lcs[base_index + 1][target_index])
        {
            current
                .get_or_insert_with(|| LineChange {
                    base: base_index..base_index,
                    replacement: Vec::new(),
                })
                .replacement
                .push(target[target_index].clone());
            target_index += 1;
        } else {
            let change = current.get_or_insert_with(|| LineChange {
                base: base_index..base_index,
                replacement: Vec::new(),
            });
            base_index += 1;
            change.base.end = base_index;
        }
    }
    if let Some(change) = current {
        changes.push(change);
    }
    Ok(changes)
}

fn changes_overlap(left: &LineChange, right: &LineChange) -> bool {
    if left.base.is_empty() && right.base.is_empty() {
        left.base.start == right.base.start
    } else if left.base.is_empty() {
        right.base.start < left.base.start && left.base.start < right.base.end
    } else if right.base.is_empty() {
        left.base.start < right.base.start && right.base.start < left.base.end
    } else {
        left.base.start < right.base.end && right.base.start < left.base.end
    }
}

fn conflict_artifact(local: &str, baseline: &str, upstream: &str) -> String {
    format!(
        "<<<<<<< local\n{}||||||| installed baseline\n{}=======\n{}>>>>>>> bundled registry\n",
        with_final_newline(local),
        with_final_newline(baseline),
        with_final_newline(upstream)
    )
}

fn with_final_newline(source: &str) -> String {
    if source.ends_with('\n') {
        source.to_owned()
    } else {
        format!("{source}\n")
    }
}

fn component_relative_path(id: &ModuleId) -> Result<PathBuf, ProjectError> {
    let path = id.as_str();
    let component = path
        .strip_prefix("components/")
        .ok_or_else(|| ProjectError::InvalidComponentPath(id.clone()))?;
    Ok(PathBuf::from("components")
        .join(component)
        .with_extension("rhai"))
}

fn slash_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn collect_relative_files(
    directory: &Path,
    include_prefix: &str,
) -> Result<Vec<(String, String)>, ProjectError> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut pending = vec![directory.to_path_buf()];
    let mut files = Vec::new();
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current).map_err(|source| ProjectError::Io {
            path: current.clone(),
            source,
        })? {
            let entry = entry.map_err(|source| ProjectError::Io {
                path: current.clone(),
                source,
            })?;
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path
                    .strip_prefix(directory)
                    .map_err(|_| ProjectError::InvalidPath(path.clone()))?;
                let logical = slash_path(relative);
                files.push((logical.clone(), format!("{include_prefix}/{logical}")));
            }
        }
    }
    files.sort();
    Ok(files)
}

fn collect_paths(directory: &Path) -> Result<Vec<PathBuf>, ProjectError> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(directory)
        .map_err(|source| ProjectError::Io {
            path: directory.to_path_buf(),
            source,
        })?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|source| ProjectError::Io {
                    path: directory.to_path_buf(),
                    source,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.extension().and_then(|value| value.to_str()) == Some("rhai"));
    paths.sort();
    Ok(paths)
}

fn generated_embed_module(
    manifest: &AppManifest,
    modules: &[(String, String)],
    locales: &[(String, String)],
    themes: &[(String, String)],
    assets: &[(String, String)],
) -> Result<String, ProjectError> {
    let mut output =
        String::from("// @generated by `gpui-rhai embed`; edit files under `ui/` instead.\n\n");
    let _ = writeln!(
        output,
        "pub fn app_manifest() -> gpui_rhai::AppManifest {{\n    let mut manifest = gpui_rhai::AppManifest::new(gpui_rhai::ModuleId::parse({:?}).expect(\"generated entry id\"));",
        manifest.entry.as_str()
    );
    for (id, requirement) in &manifest.capabilities {
        let _ = writeln!(
            output,
            "    manifest = manifest.with_capability({:?}, {:?}).expect(\"generated capability requirement\");",
            id.as_str(),
            requirement.to_string()
        );
    }
    output.push_str("    manifest\n}\n\n");
    output.push_str("pub fn script_source() -> gpui_rhai::EmbeddedScriptSource {\n");
    output
        .push_str("    gpui_rhai::EmbeddedScriptSource::new(std::collections::BTreeMap::from([\n");
    for (id, path) in modules {
        let _ = writeln!(
            output,
            "        (gpui_rhai::ModuleId::parse({id:?}).expect(\"generated module id\"), include_str!({path:?}).to_owned()),"
        );
    }
    output.push_str("    ]))\n}\n\n");
    output.push_str("pub const THEME_SOURCE: &str = include_str!(\"../ui/theme.rhai\");\n\n");
    output.push_str("pub const LOCALES: &[(&str, &str)] = &[\n");
    for (name, path) in locales {
        let _ = writeln!(output, "    ({name:?}, include_str!({path:?})),");
    }
    output.push_str("];\n\n");
    output.push_str("pub const THEMES: &[(&str, &str)] = &[\n");
    for (name, path) in themes {
        let _ = writeln!(output, "    ({name:?}, include_str!({path:?})),");
    }
    output.push_str("];\n\n");
    output.push_str("pub const ASSETS: &[(&str, &[u8])] = &[\n");
    for (name, path) in assets {
        let _ = writeln!(output, "    ({name:?}, include_bytes!({path:?})),");
    }
    output.push_str("];\n\n");
    output.push_str("pub fn asset_sources() -> Vec<(String, gpui_rhai::AssetData)> {\n    vec![\n");
    let mut logical_assets = BTreeSet::new();
    for (name, path) in assets {
        let mime_type = embedded_asset_mime(name)?;
        let logical_name = name
            .rsplit_once('.')
            .map_or(name.as_str(), |(stem, _)| stem);
        if !logical_assets.insert(logical_name) {
            return Err(ProjectError::DuplicateAssetLogical(logical_name.to_owned()));
        }
        let _ = writeln!(
            output,
            "        ({logical_name:?}.to_owned(), gpui_rhai::AssetData {{ mime_type: {mime_type:?}.to_owned(), bytes: include_bytes!({path:?}).to_vec() }}),"
        );
    }
    output.push_str("    ]\n}\n");
    Ok(output)
}

fn embedded_asset_mime(path: &str) -> Result<&'static str, ProjectError> {
    match path.rsplit('.').next().unwrap_or_default() {
        "svg" => Ok("image/svg+xml"),
        "png" => Ok("image/png"),
        "jpg" | "jpeg" => Ok("image/jpeg"),
        "gif" => Ok("image/gif"),
        "webp" => Ok("image/webp"),
        "bmp" => Ok("image/bmp"),
        "tif" | "tiff" => Ok("image/tiff"),
        _ => Err(ProjectError::UnsupportedAsset(path.to_owned())),
    }
}

fn host_source() -> String {
    r#"fn main() {
    let view = gpui_rhai::FileScriptView::new("ui/main.rhai")
        .prepare()
        .expect("GPUI Rhai view preparation failed");
    gpui_rhai::ScriptApplication::new(view)
        .run()
        .expect("GPUI Rhai application failed");
}
"#
    .to_owned()
}

fn starter_ui() -> String {
    r#"fn view(ctx) {
    column([
        text("GPUI Rhai"),
        text("Run `gpui-rhai add button label` to install source components.")
    ]).with_style(style().gap(px(8)).padding(px(16)))
}
"#
    .to_owned()
}

fn app_manifest_source() -> Result<String, ProjectError> {
    Ok(toml::to_string_pretty(&AppManifest {
        entry: ModuleId::parse("main")?,
        runtime_api: RUNTIME_API_VERSION,
        capabilities: BTreeMap::new(),
    })?)
}

fn read(path: &Path) -> Result<String, ProjectError> {
    fs::read_to_string(path).map_err(|source| ProjectError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[derive(Clone, Debug)]
enum ExpectedFile {
    Missing,
    Exact(String),
}

#[derive(Clone, Debug)]
struct PlannedWrite {
    path: PathBuf,
    content: String,
    expected: ExpectedFile,
}

#[derive(Clone, Debug)]
pub struct ProjectPlan {
    root: PathBuf,
    writes: Vec<PlannedWrite>,
    conflicts: Vec<ModuleId>,
}

impl ProjectPlan {
    fn new(root: PathBuf) -> Self {
        Self {
            root,
            writes: Vec::new(),
            conflicts: Vec::new(),
        }
    }

    fn create(&mut self, path: PathBuf, content: String) -> Result<(), ProjectError> {
        match fs::read_to_string(&path) {
            Ok(existing) if existing == content => Ok(()),
            Ok(_) => Err(ProjectError::WouldOverwrite(path)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.writes.push(PlannedWrite {
                    path,
                    content,
                    expected: ExpectedFile::Missing,
                });
                Ok(())
            }
            Err(source) => Err(ProjectError::Io { path, source }),
        }
    }

    fn update(&mut self, path: PathBuf, content: String, existing: String) {
        if content != existing {
            self.writes.push(PlannedWrite {
                path,
                content,
                expected: ExpectedFile::Exact(existing),
            });
        }
    }

    fn replace_or_create(&mut self, path: PathBuf, content: String) -> Result<(), ProjectError> {
        match fs::read_to_string(&path) {
            Ok(existing) => {
                self.update(path, content, existing);
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.create(path, content)
            }
            Err(source) => Err(ProjectError::Io { path, source }),
        }
    }

    #[must_use]
    pub fn conflicts(&self) -> &[ModuleId] {
        &self.conflicts
    }

    #[must_use]
    pub fn summary(&self) -> String {
        let mut lines = if self.writes.is_empty() {
            vec!["No filesystem changes required.".to_owned()]
        } else {
            vec![format!("{} file change(s):", self.writes.len())]
        };
        lines.extend(self.writes.iter().map(|write| {
            format!(
                "- {} {}",
                match &write.expected {
                    ExpectedFile::Missing => "create",
                    ExpectedFile::Exact(_) => "update",
                },
                write
                    .path
                    .strip_prefix(&self.root)
                    .unwrap_or(&write.path)
                    .display()
            )
        }));
        if !self.conflicts.is_empty() {
            lines.push(format!(
                "{} merge conflict(s): {}",
                self.conflicts.len(),
                self.conflicts
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        lines.join("\n")
    }

    /// Verify all expectations and atomically replace each planned file.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectError::ConcurrentChange`] or an I/O error. All
    /// expectations are checked before the first write.
    pub fn apply(self) -> Result<(), ProjectError> {
        for write in &self.writes {
            match (&write.expected, fs::read_to_string(&write.path)) {
                (ExpectedFile::Missing, Err(error))
                    if error.kind() == std::io::ErrorKind::NotFound => {}
                (ExpectedFile::Exact(expected), Ok(actual)) if expected == &actual => {}
                (_, result) => {
                    return Err(ProjectError::ConcurrentChange {
                        path: write.path.clone(),
                        detail: result.map_or_else(
                            |error| error.to_string(),
                            |_| "content changed".to_owned(),
                        ),
                    });
                }
            }
        }
        for (index, write) in self.writes.iter().enumerate() {
            let parent = write
                .path
                .parent()
                .ok_or_else(|| ProjectError::InvalidPath(write.path.clone()))?;
            fs::create_dir_all(parent).map_err(|source| ProjectError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
            let temporary = parent.join(format!(".gpui-rhai-tmp-{}-{index}", std::process::id()));
            fs::write(&temporary, &write.content).map_err(|source| ProjectError::Io {
                path: temporary.clone(),
                source,
            })?;
            fs::rename(&temporary, &write.path).map_err(|source| ProjectError::Io {
                path: write.path.clone(),
                source,
            })?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LocalManifest {
    runtime_api: u32,
    #[serde(default)]
    components: BTreeMap<String, InstalledComponent>,
}

impl Default for LocalManifest {
    fn default() -> Self {
        Self {
            runtime_api: RUNTIME_API_VERSION,
            components: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct InstalledComponent {
    version: Version,
    content_hash: String,
    #[serde(default)]
    dependencies: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CheckReport {
    pub components: usize,
    pub entry: ModuleId,
}

impl CheckReport {
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Check passed: entry `{}`, {} installed component(s).",
            self.entry, self.components
        )
    }
}

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("I/O failed for `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid Cargo.toml `{path}`: {source}")]
    CargoToml {
        path: PathBuf,
        source: toml_edit::TomlError,
    },
    #[error("initialization would overwrite existing file `{0}`")]
    WouldOverwrite(PathBuf),
    #[error("file `{path}` changed after planning: {detail}")]
    ConcurrentChange { path: PathBuf, detail: String },
    #[error("path `{0}` has no parent directory")]
    InvalidPath(PathBuf),
    #[error("component registry contains duplicate `{0}`")]
    DuplicateRegistry(ModuleId),
    #[error(
        "component `{component}` asset metadata differs from bundled assets: declared {declared:?}, bundled {bundled:?}"
    )]
    RegistryAssetMismatch {
        component: ModuleId,
        declared: BTreeSet<String>,
        bundled: BTreeSet<String>,
    },
    #[error("component `{0}` is not in the bundled registry")]
    UnknownComponent(ModuleId),
    #[error("component registry dependency cycle: {0:?}")]
    RegistryCycle(Vec<ModuleId>),
    #[error("at least one component name is required")]
    NoComponents,
    #[error("component `{0}` is incompatible with this runtime API")]
    IncompatibleRuntime(ModuleId),
    #[error("module `{0}` is not an official component path")]
    InvalidComponentPath(ModuleId),
    #[error("baseline hash for `{0}` does not match the install manifest")]
    BaselineHash(ModuleId),
    #[error("installed metadata for `{0}` does not match its manifest")]
    InstalledMetadata(ModuleId),
    #[error("component `{0}` did not call define_component")]
    MissingExport(ModuleId),
    #[error("component `{component}` source documentation is missing {missing}")]
    ComponentDocumentation {
        component: ModuleId,
        missing: &'static str,
    },
    #[error("application entry must declare `view(ctx)`")]
    MissingView,
    #[error("manifest requires runtime API {required}, current API is {actual}")]
    ManifestRuntime { required: u32, actual: u32 },
    #[error("generated host requires manifest entry `main`, got `{0}`")]
    ManifestEntry(ModuleId),
    #[error("embedded asset `{0}` has an unsupported image extension")]
    UnsupportedAsset(String),
    #[error("embedded assets resolve to duplicate logical ID `app/{0}`")]
    DuplicateAssetLogical(String),
    #[error("theme validation failed: {0}")]
    Theme(String),
    #[error("locale validation failed: {0}")]
    Locale(String),
    #[error("component update conflicts require inspection: {0:?}")]
    UpdateConflictsWritten(Vec<ModuleId>),
    #[error("failed to start Cargo: {0}")]
    Spawn(std::io::Error),
    #[error("Cargo application exited unsuccessfully with code {0:?}")]
    DevFailed(Option<i32>),
    #[error(transparent)]
    ModuleId(#[from] gpui_rhai::ModuleIdError),
    #[error(transparent)]
    Header(#[from] gpui_rhai::ComponentHeaderError),
    #[error(transparent)]
    Component(#[from] gpui_rhai::ComponentError),
    #[error(transparent)]
    ComponentExport(#[from] gpui_rhai::ComponentExportError),
    #[error(transparent)]
    Capability(#[from] gpui_rhai::CapabilityError),
    #[error(transparent)]
    Runtime(#[from] gpui_rhai::RuntimeError),
    #[error(transparent)]
    Source(#[from] gpui_rhai::ScriptSourceError),
    #[error(transparent)]
    TomlSerialize(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDeserialize(#[from] toml::de::Error),
    #[error(transparent)]
    JsonSerialize(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("src")).unwrap();
        fs::write(
            directory.path().join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n",
        )
        .unwrap();
        directory
    }

    #[test]
    fn dry_run_plan_has_zero_writes_until_applied() {
        let directory = fixture();
        let project = Project::new(directory.path());
        let plan = project.plan_init().unwrap();
        assert!(!plan.writes.is_empty());
        assert!(!directory.path().join("ui/main.rhai").exists());
        drop(plan);
        assert!(!directory.path().join("ui/main.rhai").exists());
    }

    #[test]
    fn init_add_and_check_form_a_reproducible_flow() {
        let directory = fixture();
        let project = Project::new(directory.path());
        project.plan_init().unwrap().apply().unwrap();
        project
            .plan_add(
                &BundledRegistry::load().unwrap(),
                &["button".to_owned(), "label".to_owned()],
            )
            .unwrap()
            .apply()
            .unwrap();
        let report = project.check().unwrap();
        assert_eq!(report.components, 2);
        assert!(
            directory
                .path()
                .join(".gpui-rhai/baselines/components/button.rhai")
                .exists()
        );
    }

    #[test]
    fn init_never_overwrites_existing_main() {
        let directory = fixture();
        fs::write(directory.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        let project = Project::new(directory.path());
        project.plan_init().unwrap().apply().unwrap();
        assert_eq!(
            read(&directory.path().join("src/main.rs")).unwrap(),
            "fn main() {}\n"
        );
        assert!(directory.path().join("gpui-rhai-host-snippet.rs").exists());
    }

    #[test]
    fn apply_detects_changes_after_planning() {
        let directory = fixture();
        let project = Project::new(directory.path());
        let plan = project.plan_init().unwrap();
        fs::write(
            directory.path().join("Cargo.toml"),
            "[package]\nname = \"changed\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert!(matches!(
            plan.apply(),
            Err(ProjectError::ConcurrentChange { .. })
        ));
    }

    #[test]
    fn embed_plan_contains_entry_components_theme_locale_and_assets() {
        let directory = fixture();
        let project = Project::new(directory.path());
        project.plan_init().unwrap().apply().unwrap();
        project
            .plan_add(&BundledRegistry::load().unwrap(), &["button".to_owned()])
            .unwrap()
            .apply()
            .unwrap();
        fs::create_dir_all(directory.path().join("ui/locales")).unwrap();
        fs::create_dir_all(directory.path().join("ui/assets")).unwrap();
        fs::write(directory.path().join("ui/locales/en.rhai"), "#{}\n").unwrap();
        fs::write(directory.path().join("ui/assets/check.svg"), "<svg/>\n").unwrap();

        project.plan_embed().unwrap().apply().unwrap();
        let generated = read(&directory.path().join("src/gpui_rhai_embedded.rs")).unwrap();
        assert!(generated.contains("pub fn app_manifest()"));
        assert!(generated.contains("AppManifest::new"));
        assert!(generated.contains("pub fn asset_sources()"));
        assert!(generated.contains("image/svg+xml"));
        assert!(generated.contains("components/button"));
        assert!(generated.contains("THEME_SOURCE"));
        assert!(generated.contains("locales/en.rhai"));
        assert!(generated.contains("assets/check.svg"));
    }

    #[test]
    fn adding_icon_installs_minimal_svg_asset_pack() {
        let directory = fixture();
        let project = Project::new(directory.path());
        project.plan_init().unwrap().apply().unwrap();
        project
            .plan_add(&BundledRegistry::load().unwrap(), &["icon".to_owned()])
            .unwrap()
            .apply()
            .unwrap();
        project.check().unwrap();
        assert!(directory.path().join("ui/assets/icons/check.svg").exists());
        assert!(directory.path().join("ui/assets/icons/close.svg").exists());
        assert!(
            directory
                .path()
                .join(".gpui-rhai/baselines/assets/icons/check.svg")
                .exists()
        );
    }

    #[test]
    fn complex_components_install_transitive_sources_and_assets() {
        let directory = fixture();
        let project = Project::new(directory.path());
        project.plan_init().unwrap().apply().unwrap();
        project
            .plan_add(
                &BundledRegistry::load().unwrap(),
                &[
                    "pagination".to_owned(),
                    "table".to_owned(),
                    "textarea".to_owned(),
                    "date_picker".to_owned(),
                ],
            )
            .unwrap()
            .apply()
            .unwrap();
        let report = project.check().unwrap();
        assert_eq!(report.components, 9);
        for component in [
            "button.rhai",
            "icon.rhai",
            "input.rhai",
            "dropdown.rhai",
            "select.rhai",
            "pagination.rhai",
            "table.rhai",
            "textarea.rhai",
            "date_picker.rhai",
        ] {
            assert!(
                directory
                    .path()
                    .join("ui/components")
                    .join(component)
                    .exists()
            );
        }
        for asset in [
            "icons/check.svg",
            "icons/close.svg",
            "icons/chevron_left.svg",
            "icons/chevron_right.svg",
            "icons/calendar.svg",
            "icons/date_previous.svg",
            "icons/date_next.svg",
        ] {
            assert!(directory.path().join("ui/assets").join(asset).exists());
        }
    }

    #[test]
    fn check_rejects_invalid_locales_and_duplicate_logical_assets() {
        let directory = fixture();
        let project = Project::new(directory.path());
        project.plan_init().unwrap().apply().unwrap();
        fs::write(
            directory.path().join("ui/locales/en.rhai"),
            "fn locale() { #{ locale: \"en\", direction: \"left_to_right\", messages: #{} } }",
        )
        .unwrap();
        assert!(matches!(project.check(), Err(ProjectError::Locale(_))));

        fs::write(directory.path().join("ui/locales/en.rhai"), EN_LOCALE).unwrap();
        fs::create_dir_all(directory.path().join("ui/assets/icons")).unwrap();
        fs::write(directory.path().join("ui/assets/icons/check.svg"), "<svg/>").unwrap();
        fs::write(directory.path().join("ui/assets/icons/check.png"), [0u8]).unwrap();
        assert!(matches!(
            project.check(),
            Err(ProjectError::DuplicateAssetLogical(name)) if name == "icons/check"
        ));
    }

    fn bumped_button_registry(upstream_purpose: &str) -> BundledRegistry {
        let mut registry = BundledRegistry::load().unwrap();
        let id = ModuleId::parse("components/button").unwrap();
        let entry = registry.entries.get_mut(&id).unwrap();
        let source = entry
            .source
            .replace("0.1.0", "0.2.0")
            .replace("// Button presents a desktop action.", upstream_purpose);
        let source: &'static str = Box::leak(source.into_boxed_str());
        entry.metadata = parse_component_header(source).unwrap();
        entry.source = source;
        registry
    }

    #[test]
    fn update_three_way_merges_non_overlapping_local_changes() {
        let directory = fixture();
        let project = Project::new(directory.path());
        project.plan_init().unwrap().apply().unwrap();
        project
            .plan_add(&BundledRegistry::load().unwrap(), &["button".to_owned()])
            .unwrap()
            .apply()
            .unwrap();
        let source_path = directory.path().join("ui/components/button.rhai");
        let local = format!(
            "{}\n// application-owned footer\n",
            read(&source_path).unwrap()
        );
        fs::write(&source_path, &local).unwrap();
        let registry = bumped_button_registry("// Button presents a refined desktop action.");
        let plan = project.plan_update(&registry).unwrap();
        assert!(plan.conflicts().is_empty());
        plan.apply().unwrap();

        let merged = read(&source_path).unwrap();
        assert!(merged.contains("refined desktop action"));
        assert!(merged.contains("application-owned footer"));
        assert!(merged.contains("0.2.0"));
        assert_eq!(
            read(
                &directory
                    .path()
                    .join(".gpui-rhai/baselines/components/button.rhai")
            )
            .unwrap(),
            registry.entries[&ModuleId::parse("components/button").unwrap()].source
        );
    }

    #[test]
    fn update_conflict_preserves_sources_and_writes_artifact() {
        let directory = fixture();
        let project = Project::new(directory.path());
        project.plan_init().unwrap().apply().unwrap();
        project
            .plan_add(&BundledRegistry::load().unwrap(), &["button".to_owned()])
            .unwrap()
            .apply()
            .unwrap();
        let source_path = directory.path().join("ui/components/button.rhai");
        let baseline_path = directory
            .path()
            .join(".gpui-rhai/baselines/components/button.rhai");
        let baseline = read(&baseline_path).unwrap();
        let local = baseline.replace(
            "// Button presents a desktop action.",
            "// Button presents the application's custom action.",
        );
        fs::write(&source_path, &local).unwrap();
        let registry = bumped_button_registry("// Button presents the registry action.");
        let plan = project.plan_update(&registry).unwrap();
        assert_eq!(
            plan.conflicts(),
            &[ModuleId::parse("components/button").unwrap()]
        );
        plan.apply().unwrap();

        assert_eq!(read(&source_path).unwrap(), local);
        assert_eq!(read(&baseline_path).unwrap(), baseline);
        let artifact = read(
            &directory
                .path()
                .join(".gpui-rhai/conflicts/components/button.rhai"),
        )
        .unwrap();
        assert!(artifact.contains("<<<<<<< local"));
        assert!(artifact.contains("||||||| installed baseline"));
        assert!(artifact.contains(">>>>>>> bundled registry"));
    }

    #[test]
    fn line_merge_combines_independent_insertions() {
        let merged = three_way_merge(
            "alpha\nbeta\n",
            "local\nalpha\nbeta\n",
            "alpha\nbeta\nupstream\n",
        )
        .unwrap();
        assert_eq!(merged, "local\nalpha\nbeta\nupstream\n");
    }

    #[test]
    fn editor_metadata_comes_from_installed_component_schemas() {
        let directory = fixture();
        let project = Project::new(directory.path());
        project.plan_init().unwrap().apply().unwrap();
        project
            .plan_add(
                &BundledRegistry::load().unwrap(),
                &["dropdown".to_owned(), "button".to_owned()],
            )
            .unwrap()
            .apply()
            .unwrap();
        let plan = project.plan_editor_metadata().unwrap();
        assert_eq!(plan.writes.len(), 2);
        plan.apply().unwrap();

        let metadata: serde_json::Value = serde_json::from_str(
            &read(&directory.path().join(".gpui-rhai/editor/components.json")).unwrap(),
        )
        .unwrap();
        let components = metadata["components"].as_array().unwrap();
        let installed = components
            .iter()
            .map(|component| component["metadata"]["id"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            installed,
            BTreeSet::from([
                "components/button",
                "components/dropdown",
                "components/input",
            ])
        );
        assert!(components.iter().any(|component| {
            component["metadata"]["id"] == "components/dropdown"
                && component["schema"]["parts"]
                    .as_array()
                    .is_some_and(|parts| parts.iter().any(|part| part == "option"))
        }));
        let snippets = read(&directory.path().join(".gpui-rhai/editor/snippets.json")).unwrap();
        assert!(snippets.contains("import \\\"components/dropdown\\\" as dropdown;"));
        assert!(snippets.contains("${1:key}"));
        assert!(snippets.contains("${2:options}"));
    }
}
