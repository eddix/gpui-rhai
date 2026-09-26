use std::collections::BTreeMap;

use gpui_rhai::{
    AssetData, ChartDataLimits, ChartDataset, EmbeddedScriptSource, EmbeddedScriptView, ModuleId,
    NativeChartData, NativeCollection, PreparedScriptView, RuntimeEngine, ScriptViewExtension,
    ThemeTokenOverrides, UiRuntimeState, UiValue, load_theme_source,
};
use gpui_rhai_registry::{
    AR_LOCALE, BUNDLED_ASSET_SOURCES, BUNDLED_CHART_SOURCES_BY_ID, BUNDLED_COMPONENT_SOURCES_BY_ID,
    BUNDLED_MOTION_SOURCES_BY_ID, BUNDLED_STORIES, BUNDLED_THEME_SOURCES, EN_LOCALE,
    StoryDefinition, ZH_CN_LOCALE,
};

pub const DEFAULT_STORY: &str = "components/button";

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

pub(crate) fn story_source(story: &StoryDefinition, case: &str) -> Result<String, String> {
    if story.id == "apps/operations" {
        let page = match case {
            "config-diff" => "configurations",
            "theme-overrides" => "settings",
            _ => "dashboard",
        };
        Ok(story.source.replace(
            "__OPERATIONS_PAGE__",
            &serde_json::to_string(page).map_err(|error| error.to_string())?,
        ))
    } else {
        Ok(story.source.to_owned())
    }
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

#[derive(Clone, Copy, Debug)]
struct OperationsFixture;

impl ScriptViewExtension for OperationsFixture {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .native_collections
            .register("operations_hosts", operations_hosts()?)
            .map_err(|error| error.to_string())?;
        runtime
            .native_chart_data
            .insert("operations_metrics".to_owned(), operations_metrics()?);
        Ok(())
    }
}

fn operations_hosts() -> Result<NativeCollection, String> {
    let rows = [
        ("edge-01", "Tokyo", "1.8.4", "Healthy", "success"),
        ("edge-02", "Berlin", "1.8.3", "Deploying", "accent"),
        ("edge-03", "Virginia", "1.7.9", "Drift", "warning"),
    ]
    .into_iter()
    .map(|(host, region, version, status, variant)| {
        BTreeMap::from([
            ("id".to_owned(), UiValue::String(host.to_owned())),
            ("host".to_owned(), UiValue::String(host.to_owned())),
            ("region".to_owned(), UiValue::String(region.to_owned())),
            ("version".to_owned(), UiValue::String(version.to_owned())),
            ("status".to_owned(), UiValue::String(status.to_owned())),
            (
                "status_variant".to_owned(),
                UiValue::String(variant.to_owned()),
            ),
        ])
    });
    NativeCollection::new("id", rows).map_err(|error| error.to_string())
}

fn operations_metrics() -> Result<NativeChartData, String> {
    let rows = [
        ("m1", "09:00", 42_i64),
        ("m2", "09:05", 51_i64),
        ("m3", "09:10", 38_i64),
        ("m4", "09:15", 44_i64),
        ("m5", "09:20", 36_i64),
    ]
    .into_iter()
    .map(|(id, minute, latency)| {
        BTreeMap::from([
            ("id".to_owned(), UiValue::String(id.to_owned())),
            ("minute".to_owned(), UiValue::String(minute.to_owned())),
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

/// Prepare one exact bundled story without opening a window.
///
/// # Errors
///
/// Returns a catalog, theme, locale, source, or runtime preparation error.
pub fn prepare(launch: &GalleryLaunch) -> Result<PreparedScriptView, String> {
    let story = resolve_story(&launch.story, &launch.case)?;
    let (theme_name, primary_theme) = theme_source(&launch.theme)?;
    let locale_sources = locale_sources(&launch.locale)?;
    let engine = RuntimeEngine::new();
    let selected = load_theme_source(engine.engine(), theme_name, primary_theme)
        .map_err(|error| error.to_string())?;
    let story_source = story_source(story, &launch.case)?;
    let source = format!(
        "{}\nfn init(ctx) {{ ctx.set_theme({}, {}); ctx.set_locale({}); }}\n",
        story_source,
        serde_json::to_string(&selected.family).map_err(|error| error.to_string())?,
        serde_json::to_string(&selected.name).map_err(|error| error.to_string())?,
        serde_json::to_string(&launch.locale).map_err(|error| error.to_string())?,
    );
    let mut view = EmbeddedScriptView::new(
        ModuleId::parse(story.source_module).map_err(|error| error.to_string())?,
        story_scripts(story, source)?,
        primary_theme,
    )
    .theme_sources(
        BUNDLED_THEME_SOURCES
            .iter()
            .filter(|(name, _)| *name != theme_name)
            .map(|(name, source)| ((*name).to_owned(), (*source).to_owned())),
    )
    .locale_sources(locale_sources.map(|(name, source)| (name.to_owned(), source.to_owned())))
    .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
        (
            path.strip_suffix(".svg").unwrap_or(path).to_owned(),
            asset(source.as_bytes()),
        )
    }));
    if story.fixture == Some("operations") {
        view = view.extension(OperationsFixture);
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
    view.prepare().map_err(|error| error.to_string())
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
        assert!(list_text().starts_with("components/button\tactions\tbasic\t"));
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
