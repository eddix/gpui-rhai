//! The Gallery: the acceptance application of the design system.
//!
//! The Gallery is a Rhai application assembled from `patterns/app_shell` and the
//! L2 layouts (`registry/gallery/`). Its pages are the component catalog and the
//! acceptance test of `docs/design/composition.md`: every page must pass the
//! productivity composition audit in both densities, and its scenes must be
//! completable by keyboard alone. This module assembles and runs it; the Host
//! adds only what Rhai cannot do: key bindings, page source documents and the
//! live audit count shown in the status bar.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use gpui_rhai::gpui::prelude::*;
use gpui_rhai::gpui::{App, Bounds, Context, Window, WindowBounds, WindowOptions, px, size};
use gpui_rhai::{
    ActionId, AssetData, AutomationCommand, EmbeddedScriptSource, EmbeddedScriptView,
    KeyBindingSpec, ModuleId, MotionPreference, NativeTextDocument, PreparedScriptView,
    ScriptViewConfig, ScriptViewExtension, ScriptViewHandle, ScriptViewHost, UiRuntimeState,
    UiValue, install,
};
use gpui_rhai_registry::{
    AR_LOCALE, BUNDLED_ASSET_SOURCES, BUNDLED_CHART_SOURCES_BY_ID, BUNDLED_COMPONENT_SOURCES_BY_ID,
    BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_MOTION_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID,
    BUNDLED_PROFILES, BUNDLED_THEME_SOURCES, EN_LOCALE, GALLERY_SOURCES_BY_ID, TOKEN_BASE_SOURCE,
    ZH_CN_LOCALE,
};

/// The page shown when no page is selected.
pub const DEFAULT_PAGE: &str = "button";

/// The launch-time selection of the Gallery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceLaunch {
    /// A page ID from [`page_ids`].
    pub page: String,
    /// `comfortable` or `compact`.
    pub density: String,
    /// `default-dark` or `default-light`.
    pub theme: String,
    /// `en`, `zh-CN` or `ar`.
    pub locale: String,
    pub motion: MotionPreference,
}

impl Default for AcceptanceLaunch {
    fn default() -> Self {
        Self {
            page: DEFAULT_PAGE.to_owned(),
            density: "comfortable".to_owned(),
            theme: "default-dark".to_owned(),
            locale: "en".to_owned(),
            motion: MotionPreference::Normal,
        }
    }
}

/// The launch map in `registry/gallery/main.rhai` that the Host replaces.
const LAUNCH_DEFAULT: &str =
    r#"#{ page: "button", density: "comfortable", mode: "dark", locale: "en" }"#;

/// Every Gallery page ID, in navigation order, read from the page modules'
/// `pages()` lists.
#[must_use]
pub fn page_ids() -> Vec<String> {
    let mut ids = Vec::new();
    for (id, source) in GALLERY_SOURCES_BY_ID {
        if !id.starts_with("gallery/pages/") {
            continue;
        }
        let Some(start) = source.find("fn pages()") else {
            continue;
        };
        let body = &source[start..];
        let end = body.find("\n}").unwrap_or(body.len());
        let mut rest = &body[..end];
        while let Some(index) = rest.find("id: \"") {
            let after = &rest[index + 5..];
            let Some(close) = after.find('"') else {
                break;
            };
            ids.push(after[..close].to_owned());
            rest = &after[close..];
        }
    }
    ids
}

/// The document name under which a Gallery module's source is shown.
#[must_use]
pub fn source_document_name(module: &str) -> String {
    format!("gallery.source.{}", module.replace('/', "."))
}

#[derive(Clone)]
struct GallerySources {
    documents: Vec<(String, NativeTextDocument)>,
}

impl ScriptViewExtension for GallerySources {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        for (name, document) in &self.documents {
            runtime
                .native_documents
                .register(name, document.clone())
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

fn validate(launch: &AcceptanceLaunch) -> Result<(), String> {
    if !page_ids().contains(&launch.page) {
        return Err(format!("unknown Gallery page `{}`", launch.page));
    }
    if !matches!(launch.density.as_str(), "comfortable" | "compact") {
        return Err(format!("unknown Gallery density `{}`", launch.density));
    }
    if !matches!(launch.theme.as_str(), "default-dark" | "default-light") {
        return Err(format!(
            "unknown Gallery theme `{}` (expected default-dark or default-light)",
            launch.theme
        ));
    }
    if !matches!(launch.locale.as_str(), "en" | "zh-CN" | "ar") {
        return Err(format!("unknown Gallery locale `{}`", launch.locale));
    }
    Ok(())
}

fn launch_map(launch: &AcceptanceLaunch) -> Result<String, String> {
    let quoted = |value: &str| serde_json::to_string(value).map_err(|error| error.to_string());
    let mode = if launch.theme == "default-light" {
        "light"
    } else {
        "dark"
    };
    Ok(format!(
        "#{{ page: {}, density: {}, mode: {}, locale: {} }}",
        quoted(&launch.page)?,
        quoted(&launch.density)?,
        quoted(mode)?,
        quoted(&launch.locale)?,
    ))
}

fn modules(launch: &AcceptanceLaunch) -> Result<BTreeMap<ModuleId, String>, String> {
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID
        .iter()
        .chain(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .chain(BUNDLED_PATTERN_SOURCES_BY_ID)
        .chain(BUNDLED_MOTION_SOURCES_BY_ID)
        .chain(BUNDLED_CHART_SOURCES_BY_ID)
    {
        modules.insert(
            ModuleId::parse(*id).map_err(|error| error.to_string())?,
            (*source).to_owned(),
        );
    }
    for (id, source) in GALLERY_SOURCES_BY_ID {
        if *id == "gallery/main" {
            if !source.contains(LAUNCH_DEFAULT) {
                return Err("the Gallery entry no longer declares its launch map".to_owned());
            }
            let main = source.replacen(LAUNCH_DEFAULT, &launch_map(launch)?, 1);
            modules.insert(
                ModuleId::parse("main").map_err(|error| error.to_string())?,
                main,
            );
        } else {
            modules.insert(
                ModuleId::parse(*id).map_err(|error| error.to_string())?,
                (*source).to_owned(),
            );
        }
    }
    Ok(modules)
}

fn source_documents() -> Result<GallerySources, String> {
    let mut documents = Vec::new();
    for (id, source) in GALLERY_SOURCES_BY_ID {
        let name = source_document_name(id);
        let document = NativeTextDocument::new(name.clone(), 1, Arc::<str>::from(*source))
            .map_err(|error| error.to_string())?;
        documents.push((name, document));
    }
    Ok(GallerySources { documents })
}

fn key_bindings() -> Result<Vec<KeyBindingSpec>, String> {
    let palette = ActionId::parse("gallery.palette").map_err(|error| error.to_string())?;
    ["cmd-k", "ctrl-k"]
        .into_iter()
        .map(|keys| {
            KeyBindingSpec::new(keys, palette.clone(), None).map_err(|error| error.to_string())
        })
        .collect()
}

/// Assemble the Gallery without opening a window.
///
/// # Errors
///
/// Returns a launch selection or source assembly error.
pub fn view(launch: &AcceptanceLaunch) -> Result<EmbeddedScriptView, String> {
    validate(launch)?;
    let primary = if launch.theme == "default-light" {
        "default_light.rhai"
    } else {
        "default_dark.rhai"
    };
    let (_, primary_source) = BUNDLED_THEME_SOURCES
        .iter()
        .copied()
        .find(|(name, _)| *name == primary)
        .ok_or_else(|| format!("bundled theme `{primary}` is missing"))?;
    let profile = BUNDLED_PROFILES
        .iter()
        .find(|(name, _)| *name == "productivity")
        .map(|(_, source)| *source)
        .ok_or_else(|| "the productivity profile is not bundled".to_owned())?;
    let entry = ModuleId::parse("main").map_err(|error| error.to_string())?;
    let mut view = EmbeddedScriptView::new(
        entry,
        EmbeddedScriptSource::new(modules(launch)?),
        primary_source,
    )
    .token_base(TOKEN_BASE_SOURCE)
    .profile_source(profile)
    .theme_sources(
        BUNDLED_THEME_SOURCES
            .iter()
            .filter(|(name, _)| *name != primary)
            .map(|(name, source)| ((*name).to_owned(), (*source).to_owned())),
    )
    .locale_sources([
        ("en.rhai".to_owned(), EN_LOCALE.to_owned()),
        ("zh_cn.rhai".to_owned(), ZH_CN_LOCALE.to_owned()),
        ("ar.rhai".to_owned(), AR_LOCALE.to_owned()),
    ])
    .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
        (
            path.strip_suffix(".svg").unwrap_or(path).to_owned(),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: source.as_bytes().to_vec(),
            },
        )
    }))
    .motion_preference(launch.motion)
    .extension(source_documents()?);
    for binding in key_bindings()? {
        view = view.key_binding(binding);
    }
    Ok(view)
}

/// Assemble and prepare the Gallery without opening a window.
///
/// # Errors
///
/// Returns a launch selection, assembly or preparation error.
pub fn prepare(launch: &AcceptanceLaunch) -> Result<PreparedScriptView, String> {
    view(launch)?.prepare().map_err(|error| error.to_string())
}

/// Report the live audit finding count to the Gallery's status bar, once per
/// change.
///
/// # Errors
///
/// Returns an audit or automation error from the mounted view.
pub fn report_audit(
    view: &ScriptViewHandle,
    last: &mut Option<usize>,
    window: &mut Window,
    cx: &mut App,
) -> Result<(), String> {
    let count = view
        .composition_audit(cx)
        .map_err(|error| error.to_string())?
        .len();
    if *last == Some(count) {
        return Ok(());
    }
    *last = Some(count);
    let payload = UiValue::Integer(i64::try_from(count).unwrap_or(i64::MAX));
    view.automate(
        AutomationCommand::Action {
            id: "gallery.audit".to_owned(),
            payload: Some(payload),
        },
        window,
        cx,
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

struct GalleryRoot {
    host: ScriptViewHost,
    view: Option<ScriptViewHandle>,
    error: Option<String>,
    audit: Option<usize>,
}

impl Render for GalleryRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(view) = self.view.clone() else {
            let message = self
                .error
                .clone()
                .unwrap_or_else(|| "Mounting the Gallery".to_owned());
            return self.host.container(gpui_rhai::gpui::div().child(message));
        };
        // The audit reads committed geometry, so it runs after this frame paints.
        let entity = cx.entity();
        window.on_next_frame(move |window, cx| {
            entity.update(cx, |root, cx| {
                if let Some(view) = root.view.clone()
                    && let Err(error) = report_audit(&view, &mut root.audit, window, cx)
                {
                    eprintln!("gpui-rhai gallery: audit failed: {error}");
                }
            });
        });
        match view.element() {
            Ok(element) => self.host.container(element),
            Err(error) => self
                .host
                .container(gpui_rhai::gpui::div().child(error.to_string())),
        }
    }
}

/// Run the Gallery in a GPUI window.
///
/// # Errors
///
/// Returns a launch selection, preparation or platform error.
pub fn run(launch: &AcceptanceLaunch) -> Result<(), String> {
    // Platform callbacks cannot unwind safely; prepare before entering GPUI.
    let prepared = prepare(launch)?;
    let reported = std::rc::Rc::new(std::cell::RefCell::new(None::<String>));
    let error = std::rc::Rc::clone(&reported);
    gpui_rhai::gpui_platform::application().run(move |cx: &mut App| {
        install(cx);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let host = match ScriptViewHost::new("gallery", cx) {
            Ok(host) => host,
            Err(host_error) => {
                eprintln!("gpui-rhai gallery: {host_error}");
                *error.borrow_mut() = Some(host_error.to_string());
                cx.quit();
                return;
            }
        };
        if let Err(bind_error) = host.bind_keys(prepared.key_bindings().iter().cloned(), cx) {
            eprintln!("gpui-rhai gallery: {bind_error}");
        }
        let bounds = Bounds::centered(None, size(px(1280.0), px(860.0)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..WindowOptions::default()
            },
            move |window, cx| {
                let root = cx.new(|_| GalleryRoot {
                    host: host.clone(),
                    view: None,
                    error: None,
                    audit: None,
                });
                let weak = root.downgrade();
                window.defer(cx, move |window, cx| {
                    let mounted = prepared.mount(
                        ScriptViewConfig::new("gallery").paint_background(true),
                        host,
                        window,
                        cx,
                    );
                    let _ = weak.update(cx, |root, cx| {
                        match mounted {
                            Ok(view) => {
                                let _ = view.focus(window, cx);
                                root.view = Some(view);
                            }
                            Err(mount_error) => {
                                eprintln!("gpui-rhai gallery: {mount_error}");
                                root.error = Some(mount_error.to_string());
                            }
                        }
                        cx.notify();
                    });
                });
                root
            },
        );
        if let Err(open_error) = opened {
            *error.borrow_mut() = Some(open_error.to_string());
            cx.quit();
        }
        if let Some(milliseconds) = std::env::var("GPUI_RHAI_GALLERY_AUTO_QUIT_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
        {
            let timer = cx
                .background_executor()
                .timer(Duration::from_millis(milliseconds));
            cx.spawn(async move |cx| {
                timer.await;
                cx.update(|app| app.quit());
            })
            .detach();
        }
        cx.activate(true);
    });
    match reported.borrow_mut().take() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_ids_cover_every_bundled_component() {
        let ids = page_ids();
        assert!(ids.iter().any(|id| id == DEFAULT_PAGE));
        for (component, _) in BUNDLED_COMPONENT_SOURCES_BY_ID {
            let name = component.trim_start_matches("components/");
            assert!(
                ids.iter().any(|id| id == name),
                "component {component} has no Gallery page"
            );
        }
        let unique = ids.iter().collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), ids.len(), "page IDs are unique");
    }

    #[test]
    fn launch_selection_is_validated_before_a_window_opens() {
        let launch = AcceptanceLaunch {
            page: "no-such-page".to_owned(),
            ..AcceptanceLaunch::default()
        };
        assert!(view(&launch).is_err());
        let launch = AcceptanceLaunch {
            density: "dense".to_owned(),
            ..AcceptanceLaunch::default()
        };
        assert!(view(&launch).is_err());
    }
}
