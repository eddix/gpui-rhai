use std::collections::BTreeMap;
use std::time::Duration;

use gpui_rhai::{
    AppManifest, AssetData, CapabilityDescriptor, CapabilityId, CapabilityMethod, ChartDataLimits,
    ChartDataset, ChartGeoMap, EmbeddedScriptSource, EmbeddedScriptView, ModuleId, NativeChartData,
    NativeCollection, PreparedScriptView, RuntimeEngine, ScriptViewExtension,
    SubscriptionCapabilityHandler, SubscriptionWork, ThemeMode, ThemeTokenOverrides, ThemeVariant,
    UiRuntimeState, UiValue, ValueSchema, load_theme_source,
};
use gpui_rhai_registry::{
    AR_LOCALE, BUNDLED_ASSET_SOURCES, BUNDLED_CHART_SOURCES_BY_ID, BUNDLED_COMPONENT_SOURCES_BY_ID,
    BUNDLED_MOTION_SOURCES_BY_ID, BUNDLED_STORIES, BUNDLED_THEME_SOURCES, EN_LOCALE,
    StoryDefinition, ZH_CN_LOCALE,
};

pub const DEFAULT_STORY: &str = "components/button";

const COMPONENT_CATALOG_COLOR_TOKENS: &[&str] = &[
    "surface",
    "surface_raised",
    "surface_hover",
    "text_primary",
    "text_muted",
    "accent",
    "accent_hover",
    "on_accent",
    "danger",
    "on_danger",
    "warning",
    "on_warning",
    "success",
    "on_success",
    "border",
    "focus_ring",
    "selection",
    "disabled",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GalleryLaunch {
    pub story: String,
    pub case: String,
    pub theme: String,
    pub locale: String,
}

impl Default for GalleryLaunch {
    fn default() -> Self {
        Self {
            story: DEFAULT_STORY.to_owned(),
            case: "basic".to_owned(),
            theme: "default-dark".to_owned(),
            locale: "en".to_owned(),
        }
    }
}

#[must_use]
pub fn stories() -> &'static [StoryDefinition] {
    BUNDLED_STORIES
}

#[must_use]
pub fn list_text() -> String {
    BUNDLED_STORIES
        .iter()
        .map(|story| {
            let cases = story
                .cases
                .iter()
                .map(|case| case.id)
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{}\t{}\t{}\t{}",
                story.id, story.category, cases, story.title
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn resolve_story(id: &str, case: &str) -> Result<&'static StoryDefinition, String> {
    let story = BUNDLED_STORIES
        .iter()
        .find(|story| story.id == id)
        .ok_or_else(|| format!("unknown Gallery story `{id}`; run `gpui-rhai gallery --list`"))?;
    if !story.cases.iter().any(|candidate| candidate.id == case) {
        return Err(format!(
            "unknown case `{case}` for Gallery story `{id}`; available: {}",
            story
                .cases
                .iter()
                .map(|candidate| candidate.id)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Ok(story)
}

fn operations_fixture_case(case: &str) -> &str {
    match case {
        "loading" | "empty" | "failure" | "streaming" | "large" => case,
        _ => "normal",
    }
}

fn materialize_story_source(
    story: &StoryDefinition,
    launch: &GalleryLaunch,
    selected: &ThemeVariant,
) -> Result<String, String> {
    if story.id == "apps/operations" {
        let page = match launch.case.as_str() {
            "config-diff" | "failure" => "configurations",
            "theme-overrides" => "settings",
            "large" => "hosts",
            _ => "dashboard",
        };
        let fixture_case = operations_fixture_case(&launch.case);
        let source = story.source.replace(
            "__OPERATIONS_PAGE__",
            &serde_json::to_string(page).map_err(|error| error.to_string())?,
        );
        Ok(source.replace(
            "__OPERATIONS_CASE__",
            &serde_json::to_string(fixture_case).map_err(|error| error.to_string())?,
        ))
    } else if story.id == "components/catalog" {
        let json = |value: &str| serde_json::to_string(value).map_err(|error| error.to_string());
        let category = if launch.case == "basic" {
            "foundations"
        } else {
            launch.case.as_str()
        };
        let mode = match selected.mode {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        };
        let mut source = story.source.to_owned();
        for (placeholder, value) in [
            ("__PATH__", json("")?),
            ("__FAMILY__", json(&selected.family)?),
            ("__VARIANT__", json(&selected.name)?),
            ("__MODE__", json(mode)?),
            ("__STATUS__", json("Component Catalog")?),
            ("__PREVIEW_FAMILY__", json(&selected.family)?),
            ("__PREVIEW_VARIANT__", json(&selected.name)?),
            ("__GALLERY_THEME__", json(&launch.theme.replace('-', "_"))?),
            ("__GALLERY_CATEGORY__", json(category)?),
        ] {
            source = source.replace(placeholder, &value);
        }
        for token in COMPONENT_CATALOG_COLOR_TOKENS {
            let value = format!("#{:08x}", selected.tokens.colors[*token].as_rgba_hex());
            source = source.replace(
                &format!("__COLOR_{}__", token.to_ascii_uppercase()),
                &serde_json::to_string(&value).map_err(|error| error.to_string())?,
            );
        }
        Ok(source
            .replace("__BUILTIN_OPTIONS__", "[]")
            .replace("__VISUAL_DIALOG__", "false")
            .replace("__VISUAL_POPOVER__", "false")
            .replace("__VISUAL_COMMAND__", "false")
            .replace("__VISUAL_SHEET__", "false")
            .replace("__VISUAL_ALERT__", "false")
            .replace("__VISUAL_MENU__", "false")
            .replace("__VISUAL_TOAST__", "false")
            .replace(
                "__VISUAL_LOCALE__",
                &serde_json::to_string(&launch.locale).map_err(|error| error.to_string())?,
            )
            .replace("__GALLERY_ONLY__", "true"))
    } else {
        Ok(story.source.to_owned())
    }
}

pub(crate) fn story_source(launch: &GalleryLaunch) -> Result<String, String> {
    let story = resolve_story(&launch.story, &launch.case)?;
    let (theme_name, primary_theme) = theme_source(&launch.theme)?;
    let engine = RuntimeEngine::new();
    let selected = load_theme_source(engine.engine(), theme_name, primary_theme)
        .map_err(|error| error.to_string())?;
    materialize_story_source(story, launch, &selected)
}

fn theme_source(slug: &str) -> Result<(&'static str, &'static str), String> {
    let normalized = slug.replace('-', "_");
    BUNDLED_THEME_SOURCES
        .iter()
        .copied()
        .find(|(name, _)| name.strip_suffix(".rhai") == Some(normalized.as_str()))
        .ok_or_else(|| format!("unknown Gallery theme `{slug}`"))
}

fn locale_sources(locale: &str) -> Result<[(&'static str, &'static str); 3], String> {
    if !matches!(locale, "en" | "zh-CN" | "ar") {
        return Err(format!("unknown Gallery locale `{locale}`"));
    }
    Ok([
        ("en.rhai", EN_LOCALE),
        ("zh_cn.rhai", ZH_CN_LOCALE),
        ("ar.rhai", AR_LOCALE),
    ])
}

fn story_scripts(story: &StoryDefinition, source: String) -> Result<EmbeddedScriptSource, String> {
    let mut modules = BTreeMap::new();
    for &(id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(BUNDLED_MOTION_SOURCES_BY_ID)
        .chain(BUNDLED_CHART_SOURCES_BY_ID)
    {
        modules.insert(
            ModuleId::parse(id).map_err(|error| error.to_string())?,
            source.to_owned(),
        );
    }
    modules.insert(
        ModuleId::parse(story.source_module).map_err(|error| error.to_string())?,
        source,
    );
    Ok(EmbeddedScriptSource::new(modules))
}

fn asset(bytes: &[u8]) -> AssetData {
    AssetData {
        mime_type: "image/svg+xml".to_owned(),
        bytes: bytes.to_vec(),
    }
}

#[derive(Clone, Debug)]
struct OperationsFixture {
    fixture_case: String,
}

impl ScriptViewExtension for OperationsFixture {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .native_collections
            .register("operations_hosts", operations_hosts(&self.fixture_case)?)
            .map_err(|error| error.to_string())?;
        runtime.native_chart_data.insert(
            "operations_metrics".to_owned(),
            operations_metrics(&self.fixture_case)?,
        );
        let capability =
            CapabilityId::parse("gallery.operations").map_err(|error| error.to_string())?;
        runtime
            .capabilities
            .register_subscription(
                CapabilityDescriptor {
                    id: capability,
                    version: semver::Version::new(1, 0, 0),
                    methods: BTreeMap::from([
                        (
                            "deploy".to_owned(),
                            CapabilityMethod {
                                input: ValueSchema::Bool,
                                output: ValueSchema::integer(),
                            },
                        ),
                        (
                            "stream".to_owned(),
                            CapabilityMethod {
                                input: ValueSchema::Null,
                                output: ValueSchema::integer(),
                            },
                        ),
                    ]),
                },
                OperationsSubscription,
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
struct OperationsSubscription;

impl SubscriptionCapabilityHandler for OperationsSubscription {
    fn subscribe(&mut self, method: &str, input: UiValue) -> Result<SubscriptionWork, String> {
        let (name, values) = match (method, input) {
            ("deploy", UiValue::Bool(true)) => {
                ("gpui-rhai-gallery-deploy-failure", vec![15, 48, -1])
            }
            ("deploy", UiValue::Bool(false)) => ("gpui-rhai-gallery-deploy", vec![15, 48, 76, 100]),
            ("stream", UiValue::Null) => ("gpui-rhai-gallery-stream", vec![1, 2, 3, 4]),
            ("deploy", _) => return Err("deploy expects a failure boolean".to_owned()),
            ("stream", _) => return Err("stream expects null input".to_owned()),
            _ => return Err(format!("unknown operations subscription `{method}`")),
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                for value in values {
                    std::thread::sleep(Duration::from_millis(45));
                    if sender.send(UiValue::Integer(value)).is_err() {
                        break;
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(SubscriptionWork::from_receiver(receiver))
    }
}

#[derive(Clone, Debug)]
struct ChartCatalogFixture {
    stream: NativeChartData,
}

impl ScriptViewExtension for ChartCatalogFixture {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        let map = ChartGeoMap::from_geojson(
            "demo_map",
            r#"{"type":"FeatureCollection","features":[
              {"type":"Feature","id":"north","properties":{"name":"north"},"geometry":{"type":"Polygon","coordinates":[[[0,5],[10,5],[10,10],[0,10],[0,5]]]}},
              {"type":"Feature","id":"south","properties":{"name":"south"},"geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,5],[0,5],[0,0]]]}}
            ]}"#,
        )
        .map_err(|error| error.to_string())?;
        engine
            .register_chart_map(map)
            .map_err(|error| error.to_string())
    }

    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .register_native_chart_data("gallery_stream", self.stream.clone())
            .map_err(|error| error.to_string())
    }
}

fn operations_host_row(
    host: impl Into<String>,
    region: impl Into<String>,
    version: impl Into<String>,
    status: impl Into<String>,
    variant: impl Into<String>,
) -> BTreeMap<String, UiValue> {
    let host = host.into();
    BTreeMap::from([
        ("id".to_owned(), UiValue::String(host.clone())),
        ("host".to_owned(), UiValue::String(host)),
        ("region".to_owned(), UiValue::String(region.into())),
        ("version".to_owned(), UiValue::String(version.into())),
        ("status".to_owned(), UiValue::String(status.into())),
        ("status_variant".to_owned(), UiValue::String(variant.into())),
    ])
}

fn operations_hosts(fixture_case: &str) -> Result<NativeCollection, String> {
    let rows = if matches!(fixture_case, "loading" | "empty") {
        Vec::new()
    } else if fixture_case == "large" {
        let regions = ["Tokyo", "Berlin", "Virginia", "Singapore"];
        (1..=1_000)
            .map(|index| {
                let drift = index % 97 == 0;
                operations_host_row(
                    format!("edge-{index:02}"),
                    regions[(index - 1) % regions.len()],
                    if drift { "1.7.9" } else { "1.8.4" },
                    if drift { "Drift" } else { "Healthy" },
                    if drift { "warning" } else { "success" },
                )
            })
            .collect()
    } else {
        let mut rows = vec![
            operations_host_row("edge-01", "Tokyo", "1.8.4", "Healthy", "success"),
            operations_host_row("edge-02", "Berlin", "1.8.3", "Deploying", "accent"),
            operations_host_row("edge-03", "Virginia", "1.7.9", "Drift", "warning"),
        ];
        if fixture_case == "failure" {
            rows.push(operations_host_row(
                "edge-04",
                "Singapore",
                "unknown",
                "Unreachable",
                "danger",
            ));
        }
        rows
    };
    NativeCollection::new("id", rows).map_err(|error| error.to_string())
}

fn operations_metrics(fixture_case: &str) -> Result<NativeChartData, String> {
    let count = match fixture_case {
        "streaming" => 24,
        "large" => 120,
        _ => 5,
    };
    let rows = (0..count)
        .map(|index| {
            let minute = format!("{:02}:{:02}", 9 + index / 12, (index % 12) * 5);
            let latency = 36 + i64::from((index * 7) % 17);
            BTreeMap::from([
                ("id".to_owned(), UiValue::String(format!("m{}", index + 1))),
                ("minute".to_owned(), UiValue::String(minute)),
                ("latency".to_owned(), UiValue::Integer(latency)),
            ])
        })
        .collect::<Vec<_>>();
    let dataset = ChartDataset::from_rows(
        "main",
        &rows,
        Some("id".to_owned()),
        ChartDataLimits::default(),
    )
    .map_err(|error| error.to_string())?;
    NativeChartData::new([dataset], ChartDataLimits::default()).map_err(|error| error.to_string())
}

#[allow(clippy::cast_precision_loss)]
fn chart_catalog_stream(rows: usize) -> Result<NativeChartData, String> {
    let values = (0..rows)
        .map(|index| {
            let x = index as f64;
            BTreeMap::from([
                ("id".to_owned(), UiValue::String(format!("p{index}"))),
                ("x".to_owned(), UiValue::Float(x)),
                (
                    "y".to_owned(),
                    UiValue::Float((x / 270.0).sin() * 20.0 + (x / 67.0).cos() * 4.0),
                ),
            ])
        })
        .collect::<Vec<_>>();
    let dataset = ChartDataset::from_rows(
        "main",
        &values,
        Some("id".to_owned()),
        ChartDataLimits::default(),
    )
    .map_err(|error| error.to_string())?;
    NativeChartData::new([dataset], ChartDataLimits::default()).map_err(|error| error.to_string())
}

fn build_view(
    launch: &GalleryLaunch,
    chart_stream: Option<NativeChartData>,
) -> Result<EmbeddedScriptView, String> {
    let story = resolve_story(&launch.story, &launch.case)?;
    let (theme_name, primary_theme) = theme_source(&launch.theme)?;
    let locale_sources = locale_sources(&launch.locale)?;
    let engine = RuntimeEngine::new();
    let selected = load_theme_source(engine.engine(), theme_name, primary_theme)
        .map_err(|error| error.to_string())?;
    let story_source = materialize_story_source(story, launch, &selected)?;
    let source = if story.fixture == Some("component-catalog") {
        story_source
    } else {
        format!(
            "{}\nfn init(ctx) {{ ctx.set_theme({}, {}); ctx.set_locale({}); }}\n",
            story_source,
            serde_json::to_string(&selected.family).map_err(|error| error.to_string())?,
            serde_json::to_string(&selected.name).map_err(|error| error.to_string())?,
            serde_json::to_string(&launch.locale).map_err(|error| error.to_string())?,
        )
    };
    let entry = ModuleId::parse(story.source_module).map_err(|error| error.to_string())?;
    let mut view =
        EmbeddedScriptView::new(entry.clone(), story_scripts(story, source)?, primary_theme)
            .theme_sources(
                BUNDLED_THEME_SOURCES
                    .iter()
                    .filter(|(name, _)| *name != theme_name)
                    .map(|(name, source)| ((*name).to_owned(), (*source).to_owned())),
            )
            .locale_sources(
                locale_sources.map(|(name, source)| (name.to_owned(), source.to_owned())),
            )
            .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
                (
                    path.strip_suffix(".svg").unwrap_or(path).to_owned(),
                    asset(source.as_bytes()),
                )
            }));
    if story.fixture == Some("operations") {
        let manifest = AppManifest::new(entry)
            .with_capability("gallery.operations", "*")
            .map_err(|error| error.to_string())?;
        view = view.manifest(manifest).extension(OperationsFixture {
            fixture_case: operations_fixture_case(&launch.case).to_owned(),
        });
    } else if story.fixture == Some("chart-catalog") {
        view = view.extension(ChartCatalogFixture {
            stream: match chart_stream {
                Some(stream) => stream,
                None => chart_catalog_stream(4_096)?,
            },
        });
    }
    if story.id == "apps/operations" && launch.case == "theme-overrides" {
        view = view.theme_token_overrides(ThemeTokenOverrides {
            radii: BTreeMap::from([
                ("sm".to_owned(), gpui_rhai::Length::Pixels(4.0)),
                ("md".to_owned(), gpui_rhai::Length::Pixels(7.0)),
                ("lg".to_owned(), gpui_rhai::Length::Pixels(10.0)),
            ]),
            ..ThemeTokenOverrides::default()
        });
    }
    Ok(view)
}

/// Build one exact bundled story without opening a window.
///
/// # Errors
///
/// Returns a catalog, theme, locale, source, or runtime assembly error.
pub fn view(launch: &GalleryLaunch) -> Result<EmbeddedScriptView, String> {
    build_view(launch, None)
}

/// Build the source-identical Chart catalog with benchmark-owned streaming data.
///
/// This is public for the independent performance workspace; applications
/// should use [`view`] or [`prepare`] instead.
///
/// # Errors
///
/// Returns a catalog, theme, locale, source, or runtime assembly error.
#[doc(hidden)]
pub fn chart_catalog_view_with_stream(
    stream: NativeChartData,
) -> Result<EmbeddedScriptView, String> {
    build_view(
        &GalleryLaunch {
            story: "charts/catalog".to_owned(),
            ..GalleryLaunch::default()
        },
        Some(stream),
    )
}

/// Prepare one exact bundled story without opening a window.
///
/// # Errors
///
/// Returns a catalog, theme, locale, source, or runtime preparation error.
pub fn prepare(launch: &GalleryLaunch) -> Result<PreparedScriptView, String> {
    view(launch)?.prepare().map_err(|error| error.to_string())
}

/// Run one exact bundled story in a standalone GPUI window.
///
/// # Errors
///
/// Returns a preparation or platform application error.
pub fn run(launch: &GalleryLaunch) -> Result<(), String> {
    super::gallery_app::run(launch.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_deterministic_and_does_not_prepare_a_window() {
        assert_eq!(list_text().lines().count(), BUNDLED_STORIES.len());
        assert!(list_text().starts_with(
            "components/catalog\tcomponents\tbasic,forms,navigation,documents,overlays\t"
        ));
    }

    #[test]
    fn every_bundled_story_case_prepares() {
        for story in BUNDLED_STORIES {
            for case in story.cases {
                prepare(&GalleryLaunch {
                    story: story.id.to_owned(),
                    case: case.id.to_owned(),
                    ..GalleryLaunch::default()
                })
                .unwrap_or_else(|error| panic!("{} / {}: {error}", story.id, case.id));
            }
        }
    }

    #[test]
    fn invalid_story_case_theme_and_locale_are_explicit() {
        let launch = GalleryLaunch {
            story: "missing".to_owned(),
            ..GalleryLaunch::default()
        };
        let Err(error) = prepare(&launch) else {
            panic!("missing story must fail")
        };
        assert!(error.contains("unknown Gallery story"));
        let launch = GalleryLaunch {
            case: "missing".to_owned(),
            ..GalleryLaunch::default()
        };
        let Err(error) = prepare(&launch) else {
            panic!("missing case must fail")
        };
        assert!(error.contains("unknown case"));
        let launch = GalleryLaunch {
            theme: "missing".to_owned(),
            ..GalleryLaunch::default()
        };
        let Err(error) = prepare(&launch) else {
            panic!("missing theme must fail")
        };
        assert!(error.contains("unknown Gallery theme"));
        let launch = GalleryLaunch {
            locale: "missing".to_owned(),
            ..GalleryLaunch::default()
        };
        let Err(error) = prepare(&launch) else {
            panic!("missing locale must fail")
        };
        assert!(error.contains("unknown Gallery locale"));
    }
}
