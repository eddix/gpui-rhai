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
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use gpui_rhai::gpui::prelude::*;
use gpui_rhai::gpui::{
    App, Bounds, Context, TitlebarOptions, Window, WindowBounds, WindowOptions, point, px, size,
};
use gpui_rhai::{
    ActionId, AssetData, AutomationCommand, EmbeddedScriptSource, EmbeddedScriptView,
    KeyBindingSpec, ModuleId, MotionPreference, NativeCollection, NativeTextDocument,
    PreparedScriptView, ScriptViewConfig, ScriptViewExtension, ScriptViewHandle, ScriptViewHost,
    UiRuntimeState, UiValue, install,
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
    /// A bundled theme slug: the file name with dashes (`default-dark`,
    /// `catppuccin-latte`, `tokyo-storm`, ...).
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
    r#"#{ page: "button", density: "comfortable", locale: "en", titlebar_inset: 0 }"#;

/// On macOS the Gallery window has no platform title bar: its own `TitleBar`
/// moves the window and leaves this much room for the window buttons, which
/// sit at [`MACOS_TRAFFIC_LIGHTS`] until [`titlebar_traffic_lights`] centres
/// them on the bar.
const MACOS_TITLEBAR_INSET: u32 = 72;
const MACOS_TRAFFIC_LIGHTS: (f32, f32) = (12.0, 11.0);
/// The window buttons go this far above the middle of the bar: the comfortable
/// bar, 36pt with its bottom hairline, has them at 11.
const MACOS_TRAFFIC_LIGHTS_ABOVE_MIDDLE: f32 = 7.0;

/// Every Gallery page ID, in navigation order, read from the page modules'
/// `pages()` lists.
#[must_use]
pub fn page_ids() -> Vec<String> {
    let mut ids = Vec::new();
    for (id, source) in GALLERY_SOURCES_BY_ID {
        if !id.starts_with("gallery/pages/") {
            continue;
        }
        ids.extend(
            literal_page_ids(source)
                .unwrap_or_else(|error| panic!("invalid bundled Gallery catalog {id}: {error}")),
        );
    }
    ids
}

// Evaluate just the literal `pages()` body in a bounded raw engine: imports,
// definitions and views never execute. Formatting is not part of the catalog ABI.
fn literal_page_ids(source: &str) -> Result<Vec<String>, String> {
    let Some(start) = source.find("fn pages") else {
        return Ok(Vec::new());
    };
    let open = source[start..]
        .find('{')
        .map(|offset| start + offset)
        .ok_or_else(|| "pages has no body".to_owned())?;
    let bytes = source.as_bytes();
    let (mut depth, mut quoted, mut escaped, mut line_comment, mut block_comment) =
        (0, false, false, false, false);
    let mut index = open;
    let mut close = None;
    while index < bytes.len() {
        let byte = bytes[index];
        let next = bytes.get(index + 1).copied();
        if line_comment {
            if byte == b'\n' {
                line_comment = false;
            }
        } else if block_comment {
            if byte == b'*' && next == Some(b'/') {
                block_comment = false;
                index += 1;
            }
        } else if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else {
            match (byte, next) {
                (b'/', Some(b'/')) => {
                    line_comment = true;
                    index += 1;
                }
                (b'/', Some(b'*')) => {
                    block_comment = true;
                    index += 1;
                }
                (b'"', _) => quoted = true,
                (b'{', _) => depth += 1,
                (b'}', _) => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(index);
                        break;
                    }
                }
                _ => {}
            }
        }
        index += 1;
    }
    let close = close.ok_or_else(|| "pages body is unterminated".to_owned())?;
    let mut engine = gpui_rhai::rhai::Engine::new_raw();
    engine.set_max_operations(10_000);
    engine.set_max_array_size(256);
    engine.set_max_map_size(2_048);
    engine.set_max_string_size(65_536);
    let pages = engine
        .eval::<gpui_rhai::rhai::Array>(&source[open..=close])
        .map_err(|error| error.to_string())?;
    pages
        .into_iter()
        .map(|page| {
            let map = page
                .try_cast::<gpui_rhai::rhai::Map>()
                .ok_or_else(|| "a page must be a map".to_owned())?;
            map.get("id")
                .cloned()
                .and_then(gpui_rhai::rhai::Dynamic::try_cast::<gpui_rhai::rhai::ImmutableString>)
                .map(|id| id.to_string())
                .ok_or_else(|| "a page needs a string id".to_owned())
        })
        .collect()
}

/// The document name under which a Gallery module's source is shown.
#[must_use]
pub fn source_document_name(module: &str) -> String {
    format!("gallery.source.{}", module.replace('/', "."))
}

#[derive(Clone)]
struct GalleryFixtures {
    documents: Vec<(String, NativeTextDocument)>,
}

impl ScriptViewExtension for GalleryFixtures {
    fn configure_rhai_engine(&self, engine: &mut gpui_rhai::rhai::Engine) -> Result<(), String> {
        gpui_rhai::rhai::FuncRegistration::new("gallery_reply_document")
            .in_global_namespace()
            .register_into_engine(engine, reply_document);
        Ok(())
    }

    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        for (name, document) in &self.documents {
            runtime
                .native_documents
                .register(name, document.clone())
                .map_err(|error| error.to_string())?;
        }
        runtime
            .native_collections
            .register("gallery_formatted_rows", formatted_rows()?)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

const REPLY_STEPS: i64 = 24;
const REPLY_MARKDOWN: &str = "## Investigation\n\nThe response is a sequence of **Rust-owned document snapshots**. Rhai stores only the message id and progress counter.\n\n1. Inspect the failed health check.\n2. Compare the active configuration.\n3. Retry only the affected host.\n\n```rhai\nfn healthy(ctx) {\n    text(\"Ready · 中文 · العربية\")\n}\n```\n\n| Host | Result |\n|:--|--:|\n| edge-01 | Ready |\n| edge-03 | Retry |\n\n[Inspect the runbook](https://example.com/runbook)\n\n- [x] No network request was made\n- [x] Stop leaves the partial reply visible\n- [x] You can draft the next message while busy\n";

fn reply_document(
    message: gpui_rhai::rhai::INT,
    progress: gpui_rhai::rhai::INT,
) -> Result<NativeTextDocument, Box<gpui_rhai::rhai::EvalAltResult>> {
    let error = |message: String| {
        Box::new(gpui_rhai::rhai::EvalAltResult::ErrorRuntime(
            message.into(),
            gpui_rhai::rhai::Position::NONE,
        ))
    };
    if message < 0 || !(0..=REPLY_STEPS).contains(&progress) {
        return Err(error(
            "invalid Gallery reply identity or progress".to_owned(),
        ));
    }
    let progress = usize::try_from(progress).map_err(|err| error(err.to_string()))?;
    let count = REPLY_MARKDOWN.chars().count() * progress
        / usize::try_from(REPLY_STEPS).expect("24 fits usize");
    let text = REPLY_MARKDOWN.chars().take(count).collect::<String>();
    NativeTextDocument::new(
        format!("gallery.reply.{message}"),
        u64::try_from(progress).expect("progress below 25 fits u64") + 1,
        Arc::<str>::from(text),
    )
    .map_err(|err| error(err.to_string()))
}

fn validate(launch: &AcceptanceLaunch) -> Result<(), String> {
    if !page_ids().contains(&launch.page) {
        return Err(format!("unknown Gallery page `{}`", launch.page));
    }
    if !matches!(launch.density.as_str(), "comfortable" | "compact") {
        return Err(format!("unknown Gallery density `{}`", launch.density));
    }
    theme_file(&launch.theme)?;
    if !matches!(launch.locale.as_str(), "en" | "zh-CN" | "ar") {
        return Err(format!("unknown Gallery locale `{}`", launch.locale));
    }
    Ok(())
}

/// The bundled theme file for a slug (`default-dark` is `default_dark.rhai`).
fn theme_file(slug: &str) -> Result<(&'static str, &'static str), String> {
    let file = format!("{}.rhai", slug.replace('-', "_"));
    BUNDLED_THEME_SOURCES
        .iter()
        .copied()
        .find(|(name, _)| *name == file)
        .ok_or_else(|| {
            let known = BUNDLED_THEME_SOURCES
                .iter()
                .map(|(name, _)| name.trim_end_matches(".rhai").replace('_', "-"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("unknown Gallery theme `{slug}` (expected one of {known})")
        })
}

fn launch_map(launch: &AcceptanceLaunch, titlebar_inset: u32) -> Result<String, String> {
    let quoted = |value: &str| serde_json::to_string(value).map_err(|error| error.to_string());
    Ok(format!(
        "#{{ page: {}, density: {}, locale: {}, titlebar_inset: {titlebar_inset} }}",
        quoted(&launch.page)?,
        quoted(&launch.density)?,
        quoted(&launch.locale)?,
    ))
}

fn modules(
    launch: &AcceptanceLaunch,
    titlebar_inset: u32,
) -> Result<BTreeMap<ModuleId, String>, String> {
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
            let main = source.replacen(LAUNCH_DEFAULT, &launch_map(launch, titlebar_inset)?, 1);
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

// Immutable source data is shared across Gallery mounts; query/selection state remains
// view-local. Thirty thousand display/rank pairs never become a Rhai Array.
fn formatted_rows() -> Result<NativeCollection, String> {
    static ROWS: OnceLock<Result<NativeCollection, String>> = OnceLock::new();
    ROWS.get_or_init(|| {
        let rows = (0..30_000).map(|index| {
            let rank = index % 3;
            let seconds = index % 7_200;
            let missing = index % 17 == 0;
            BTreeMap::from([
                (
                    "id".to_owned(),
                    UiValue::String(format!("instance-{index:05}")),
                ),
                (
                    "name".to_owned(),
                    UiValue::String(format!("api-{index:05}")),
                ),
                (
                    "status".to_owned(),
                    UiValue::String(["critical", "warning", "healthy"][rank].to_owned()),
                ),
                (
                    "status_rank".to_owned(),
                    UiValue::Integer(i64::try_from(rank).expect("three ranks fit i64")),
                ),
                (
                    "duration".to_owned(),
                    UiValue::String(if missing {
                        "—".to_owned()
                    } else if seconds < 60 {
                        "<1m".to_owned()
                    } else {
                        format!("{}h {:02}m", seconds / 3_600, seconds / 60 % 60)
                    }),
                ),
                (
                    "duration_seconds".to_owned(),
                    if missing {
                        UiValue::Null
                    } else {
                        UiValue::Integer(
                            i64::try_from(seconds).expect("seconds below 7200 fit i64"),
                        )
                    },
                ),
            ])
        });
        NativeCollection::new("id", rows).map_err(|error| error.to_string())
    })
    .clone()
}

fn source_documents() -> Result<GalleryFixtures, String> {
    let mut documents = Vec::new();
    for (id, source) in GALLERY_SOURCES_BY_ID {
        let name = source_document_name(id);
        let document = NativeTextDocument::new(name.clone(), 1, Arc::<str>::from(*source))
            .map_err(|error| error.to_string())?;
        documents.push((name, document));
    }
    let markdown = "# Native Markdown\n\nThis document stays in Rust. **Select across blocks**, copy code, or inspect a link without opening a browser.\n\n- Native document revisions\n- Theme-aware typography\n- Keyboard and selection\n\n```rhai\nfn view(ctx) { text(\"Hello from Rhai\") }\n```\n\n| Region | State |\n|:--|--:|\n| Singapore | Ready |\n| Frankfurt | Waiting |\n\n[Inspect a link](https://example.com/docs)\n\n- [x] Native source\n- [ ] No automatic network requests\n";
    for (name, text) in [
        ("gallery.markdown.reference", markdown.to_owned()),
        ("gallery.markdown.large", format!("# Large native document\n\n{}", "A streamed reply can be longer than the Rhai string budget. 中文、العربية and code remain native.\n\n".repeat(14_000))),
        ("gallery.markdown.stream", "# Streaming reply\n\nReady to stream.\n".to_owned()),
    ] {
        documents.push((name.to_owned(), NativeTextDocument::new(name, 1, Arc::<str>::from(text))
            .map_err(|error| error.to_string())?));
    }
    Ok(GalleryFixtures { documents })
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
    view_with_titlebar(launch, 0)
}

/// The Gallery for a window whose own `TitleBar` replaces the platform title
/// bar, leaving `titlebar_inset` points at its start for window buttons.
fn view_with_titlebar(
    launch: &AcceptanceLaunch,
    titlebar_inset: u32,
) -> Result<EmbeddedScriptView, String> {
    validate(launch)?;
    let (primary, primary_source) = theme_file(&launch.theme)?;
    let profile = BUNDLED_PROFILES
        .iter()
        .find(|(name, _)| *name == "productivity")
        .map(|(_, source)| *source)
        .ok_or_else(|| "the productivity profile is not bundled".to_owned())?;
    let entry = ModuleId::parse("main").map_err(|error| error.to_string())?;
    let mut view = EmbeddedScriptView::new(
        entry,
        EmbeddedScriptSource::new(modules(launch, titlebar_inset)?),
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

/// Where the macOS window buttons go so they sit in the middle of the
/// Gallery's `TitleBar`, from its height as last laid out: the bar is the
/// top-most toolbar. Density changes the bar's height (36pt comfortable, 32pt
/// compact, hairline included), so the Host moves the buttons after each
/// committed render.
/// `None` before the bar has been laid out.
///
/// # Errors
///
/// Returns an error from the mounted view's semantics snapshot.
pub fn titlebar_traffic_lights(
    view: &ScriptViewHandle,
    cx: &App,
) -> Result<Option<gpui_rhai::gpui::Point<gpui_rhai::gpui::Pixels>>, String> {
    let snapshot = view
        .accessibility_snapshot(cx)
        .map_err(|error| error.to_string())?;
    let bar = snapshot
        .nodes()
        .filter(|node| node.role == "toolbar")
        .filter_map(|node| node.geometry.map(|geometry| geometry.visual))
        .filter(|bounds| bounds.height > 0.0)
        .min_by(|a, b| a.y.total_cmp(&b.y));
    #[allow(clippy::cast_possible_truncation)]
    Ok(bar.map(|bar| {
        point(
            px(MACOS_TRAFFIC_LIGHTS.0),
            px(bar.height as f32 / 2.0 - MACOS_TRAFFIC_LIGHTS_ABOVE_MIDDLE),
        )
    }))
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
    /// The committed revision last audited: frames that only repaint (hover,
    /// scrolling, a caret) are not audited again.
    audited_revision: Option<u64>,
    /// Whether the Gallery draws its own title bar, and where the window
    /// buttons were last put on it.
    own_titlebar: bool,
    traffic_lights: Option<gpui_rhai::gpui::Point<gpui_rhai::gpui::Pixels>>,
}

impl GalleryRoot {
    /// Keep the macOS window buttons in the middle of the title bar, whose
    /// height follows the density. Only macOS draws its own bar.
    #[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
    fn place_traffic_lights(&mut self, view: &ScriptViewHandle, window: &mut Window, cx: &App) {
        if !self.own_titlebar {
            return;
        }
        match titlebar_traffic_lights(view, cx) {
            Ok(Some(position)) if self.traffic_lights != Some(position) => {
                #[cfg(target_os = "macos")]
                window.set_traffic_light_position(position);
                self.traffic_lights = Some(position);
            }
            Ok(_) => {}
            Err(error) => eprintln!("gpui-rhai gallery: window buttons: {error}"),
        }
    }
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
        // The audit and the window buttons read committed geometry, so they run
        // after a frame that follows a new render, and only then.
        let revision = view.committed_revision(cx).ok();
        if revision != self.audited_revision {
            self.audited_revision = revision;
            let entity = cx.entity();
            window.on_next_frame(move |window, cx| {
                entity.update(cx, |root, cx| {
                    if let Some(view) = root.view.clone() {
                        root.place_traffic_lights(&view, window, cx);
                        if let Err(error) = report_audit(&view, &mut root.audit, window, cx) {
                            eprintln!("gpui-rhai gallery: audit failed: {error}");
                        }
                    }
                });
            });
        }
        match view.element() {
            Ok(element) => self.host.container(element),
            Err(error) => self
                .host
                .container(gpui_rhai::gpui::div().child(error.to_string())),
        }
    }
}

fn window_options(own_titlebar: bool, cx: &mut App) -> WindowOptions {
    let bounds = Bounds::centered(None, size(px(1280.0), px(860.0)), cx);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some("gpui-rhai Gallery".into()),
            appears_transparent: own_titlebar,
            traffic_light_position: own_titlebar
                .then(|| point(px(MACOS_TRAFFIC_LIGHTS.0), px(MACOS_TRAFFIC_LIGHTS.1))),
        }),
        // The Gallery's TitleBar moves the window itself; AppKit must not claim
        // clicks in the title strip.
        app_owns_titlebar_drag: own_titlebar,
        ..WindowOptions::default()
    }
}

/// Run the Gallery in a GPUI window.
///
/// # Errors
///
/// Returns a launch selection, preparation or platform error.
pub fn run(launch: &AcceptanceLaunch) -> Result<(), String> {
    // Platform callbacks cannot unwind safely; prepare before entering GPUI.
    let own_titlebar = cfg!(target_os = "macos");
    let titlebar_inset = if own_titlebar {
        MACOS_TITLEBAR_INSET
    } else {
        0
    };
    let prepared = view_with_titlebar(launch, titlebar_inset)?
        .prepare()
        .map_err(|error| error.to_string())?;
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
        let options = window_options(own_titlebar, cx);
        let opened = cx.open_window(options, move |window, cx| {
            let root = cx.new(|_| GalleryRoot {
                host: host.clone(),
                view: None,
                error: None,
                audit: None,
                audited_revision: None,
                own_titlebar,
                traffic_lights: None,
            });
            let weak = root.downgrade();
            window.defer(cx, move |window, cx| {
                let mounted = prepared.mount(
                    ScriptViewConfig::new("gallery")
                        .paint_background(true)
                        .window_drag_areas(own_titlebar),
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
        });
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
    fn bundled_entry_scripts_do_not_shadow_builtins() {
        // The Theme Studio and story entries are covered next to the code
        // that fills their placeholders.
        let mut entries = vec![("gallery/main".to_owned(), gallery_entry_source())];
        for example in ["hello_world", "byod_treemap"] {
            let path = format!(
                "{}/../../examples/{example}/ui/main.rhai",
                env!("CARGO_MANIFEST_DIR")
            );
            entries.push((example.to_owned(), std::fs::read_to_string(path).unwrap()));
        }
        let mut runtime = gpui_rhai::RuntimeEngine::new();
        let mut found = Vec::new();
        for (name, source) in &entries {
            for diagnostic in runtime.lint_shadowed_builtins(name, source).unwrap() {
                found.push(diagnostic.to_string());
            }
        }
        assert!(found.is_empty(), "{}", found.join("\n"));
    }

    #[test]
    fn gallery_native_fixtures_are_bounded_and_revisioned() {
        assert_eq!(formatted_rows().unwrap().len(), 30_000);
        let partial = reply_document(7, 12).unwrap();
        let complete = reply_document(7, REPLY_STEPS).unwrap();
        assert_eq!(partial.identity(), complete.identity());
        assert!(partial.revision() < complete.revision());
        assert_eq!(complete.text(), REPLY_MARKDOWN);
        assert!(complete.text().starts_with(partial.text()));
        assert!(reply_document(-1, 1).is_err());
        assert!(reply_document(7, REPLY_STEPS + 1).is_err());
        let fixtures = source_documents().unwrap();
        let mut runtime = UiRuntimeState::new();
        fixtures.configure_runtime(&mut runtime).unwrap();
        assert!(
            runtime
                .native_documents
                .get("gallery.markdown.large")
                .unwrap()
                .text()
                .len()
                > 1_048_576
        );
    }

    fn gallery_entry_source() -> String {
        GALLERY_SOURCES_BY_ID
            .iter()
            .find(|(id, _)| *id == "gallery/main")
            .map(|(_, source)| (*source).to_owned())
            .unwrap()
    }

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
    fn gallery_catalog_is_independent_of_whitespace_and_later_demo_data() {
        for source in [
            "fn pages(){[#{id:\"one\",title:\"{quoted}\"},#{id:\"two\"}]}\nfn view(){#{id:\"not-a-page\"}}",
            "fn pages() { /* } */ [#{ id: \"one\" }, #{ id: \"two\" }] // }\n}\nfn view(){}",
        ] {
            assert_eq!(literal_page_ids(source).unwrap(), ["one", "two"]);
        }
        assert!(literal_page_ids("fn pages(){loop {}}").is_err());
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
