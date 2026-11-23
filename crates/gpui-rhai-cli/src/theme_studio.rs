use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_rhai::{
    AssetData, ColorValue, ComponentInstancePath, EmbeddedScriptSource, EventResponse, ModuleId,
    NativeEvent, NativeHandlerDescriptor, NativeHandlerId, Rgba8, RuntimeEngine, ScriptApplication,
    ScriptView, ScriptViewExtension, ThemeMode, ThemeVariant, UiRuntimeState, UiValue, ValueSchema,
    load_theme_source,
};

use super::{
    AR_LOCALE, BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID,
    BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID, BUNDLED_THEME_SOURCES, EN_LOCALE,
    MODERN_DARK_THEME, STUDIO_SOURCE, ZH_CN_LOCALE,
};
const COLOR_TOKENS: &[&str] = &[
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

#[derive(Clone)]
struct StudioSession {
    root: PathBuf,
    path: Option<PathBuf>,
    document: ThemeVariant,
    attribution: Vec<String>,
    preview_family: String,
    preview_variant: String,
}

impl StudioSession {
    /// The complete edited theme over the design-language token base, under the
    /// identity the specimen is previewing.
    fn preview(&self) -> Result<ThemeVariant, String> {
        let mut variant = with_token_base(&self.document)?;
        variant.family.clone_from(&self.preview_family);
        variant.name.clone_from(&self.preview_variant);
        Ok(variant)
    }

    fn path_text(&self) -> String {
        self.path
            .as_ref()
            .map_or_else(String::new, |path| path.to_string_lossy().into_owned())
    }

    fn resolve_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_owned()
        } else {
            self.root.join(path)
        }
    }
}

#[derive(Clone)]
struct ThemeStudioExtension {
    session: Rc<RefCell<StudioSession>>,
    builtins: Rc<BTreeMap<String, String>>,
}

impl ScriptViewExtension for ThemeStudioExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        for field in ["path", "family", "variant"] {
            register_handler(
                engine,
                &format!("theme_studio.set_{field}"),
                "change",
                ValueSchema::string(),
                field_handler(Rc::clone(&self.session), field.to_owned()),
            )?;
        }
        for &token in COLOR_TOKENS {
            register_handler(
                engine,
                &format!("theme_studio.set_{token}"),
                "change",
                ValueSchema::string(),
                color_handler(Rc::clone(&self.session), token.to_owned()),
            )?;
        }
        for mode in ["dark", "light"] {
            register_handler(
                engine,
                &format!("theme_studio.mode_{mode}"),
                "click",
                ValueSchema::UiValue,
                mode_handler(Rc::clone(&self.session), mode.to_owned()),
            )?;
        }
        for action in ["new", "open", "import", "save", "derive"] {
            let session = Rc::clone(&self.session);
            let builtins = Rc::clone(&self.builtins);
            register_handler(
                engine,
                &format!("theme_studio.{action}"),
                "click",
                ValueSchema::UiValue,
                Box::new(move |event, runtime, window, app| {
                    match handle_action(action, &session, &builtins, event, runtime, window, app) {
                        Ok(response) => Ok(response),
                        Err(error) => {
                            set_status(runtime, error)?;
                            Ok(EventResponse::new().stop())
                        }
                    }
                }),
            )?;
        }
        let session = Rc::clone(&self.session);
        let builtins = Rc::clone(&self.builtins);
        register_handler(
            engine,
            "theme_studio.builtin",
            "change",
            ValueSchema::Array {
                items: Box::new(ValueSchema::string()),
                max_items: Some(1),
            },
            Box::new(move |event, runtime, _, _| {
                let key = match event.payload {
                    UiValue::Array(values) => {
                        values.into_iter().next().and_then(|value| match value {
                            UiValue::String(value) => Some(value),
                            _ => None,
                        })
                    }
                    _ => None,
                }
                .ok_or_else(|| "bundled theme selection is empty".to_owned())?;
                let source = builtins
                    .get(&key)
                    .ok_or_else(|| format!("unknown bundled theme `{key}`"))?;
                load_into_session(&session, source, None, true, runtime)?;
                Ok(EventResponse::new().stop())
            }),
        )?;
        Ok(())
    }
}

type Handler = Box<
    dyn FnMut(
        NativeEvent,
        &mut UiRuntimeState,
        &mut gpui_rhai::gpui::Window,
        &mut gpui_rhai::gpui::App,
    ) -> Result<EventResponse, String>,
>;

fn register_handler(
    engine: &RuntimeEngine,
    id: &str,
    event: &str,
    schema: ValueSchema,
    handler: Handler,
) -> Result<(), String> {
    engine
        .register_native_handler(
            NativeHandlerDescriptor::new(
                NativeHandlerId::parse(id).map_err(|error| error.to_string())?,
                BTreeMap::from([(event.to_owned(), schema)]),
            )
            .map_err(|error| error.to_string())?,
            handler,
        )
        .map_err(|error| error.to_string())
}

fn string_payload(event: NativeEvent) -> Result<String, String> {
    match event.payload {
        UiValue::String(value) => Ok(value),
        value => Err(format!("expected string payload, received {value:?}")),
    }
}

fn field_handler(session: Rc<RefCell<StudioSession>>, field: String) -> Handler {
    Box::new(move |event, runtime, _, _| {
        let value = string_payload(event)?;
        {
            let mut session = session.borrow_mut();
            match field.as_str() {
                "path" => {
                    session.path = (!value.trim().is_empty()).then(|| PathBuf::from(value.trim()));
                }
                "family" => session.document.family.clone_from(&value),
                "variant" => session.document.name.clone_from(&value),
                _ => return Err(format!("unknown studio field `{field}`")),
            }
        }
        set_root_field(runtime, &field, UiValue::String(value))?;
        set_status(runtime, validation_status(&session.borrow().document))?;
        Ok(EventResponse::new().stop())
    })
}

fn color_handler(session: Rc<RefCell<StudioSession>>, token: String) -> Handler {
    Box::new(move |event, runtime, _, _| {
        let value = string_payload(event)?;
        set_root_field(
            runtime,
            &format!("color_{token}"),
            UiValue::String(value.clone()),
        )?;
        match parse_color(&value) {
            Ok(color) => {
                let preview = {
                    let mut session = session.borrow_mut();
                    std::sync::Arc::make_mut(&mut session.document.tokens)
                        .colors
                        .insert(token.clone(), color);
                    session.preview()?
                };
                runtime
                    .replace_theme_variant_from_host(preview)
                    .map_err(|error| error.to_string())?;
                set_status(runtime, validation_status(&session.borrow().document))?;
            }
            Err(error) => set_status(runtime, format!("{token}: {error}"))?,
        }
        Ok(EventResponse::new().stop())
    })
}

fn mode_handler(session: Rc<RefCell<StudioSession>>, mode: String) -> Handler {
    Box::new(move |_, runtime, _, _| {
        let preview = {
            let mut session = session.borrow_mut();
            session.document.mode = if mode == "light" {
                ThemeMode::Light
            } else {
                ThemeMode::Dark
            };
            session.preview()?
        };
        set_root_field(runtime, "mode", UiValue::String(mode.clone()))?;
        runtime
            .replace_theme_variant_from_host(preview)
            .map_err(|error| error.to_string())?;
        set_status(runtime, validation_status(&session.borrow().document))?;
        Ok(EventResponse::new().stop())
    })
}

fn handle_action(
    action: &str,
    session: &Rc<RefCell<StudioSession>>,
    _builtins: &BTreeMap<String, String>,
    _event: NativeEvent,
    runtime: &mut UiRuntimeState,
    _window: &mut gpui_rhai::gpui::Window,
    _app: &mut gpui_rhai::gpui::App,
) -> Result<EventResponse, String> {
    match action {
        "new" => {
            let mut draft = default_draft()?;
            "Untitled".clone_into(&mut draft.family);
            {
                let mut session = session.borrow_mut();
                session.document = draft;
                session.path = None;
                session.attribution.clear();
            }
            sync_document(runtime, &session.borrow(), "New unsaved theme")?;
        }
        "open" | "import" => {
            let path = session
                .borrow()
                .path
                .clone()
                .ok_or_else(|| "Enter a .rhai path first".to_owned())?;
            let resolved = session.borrow().resolve_path(&path);
            let source = fs::read_to_string(&resolved)
                .map_err(|error| format!("failed to read {}: {error}", resolved.display()))?;
            load_into_session(session, &source, Some(path), action == "import", runtime)?;
        }
        "save" => {
            let borrowed = session.borrow();
            let path = borrowed
                .path
                .as_ref()
                .ok_or_else(|| "Set a .rhai path before saving".to_owned())?;
            if path.extension().and_then(|extension| extension.to_str()) != Some("rhai") {
                return Err("Theme Studio saves only .rhai files".to_owned());
            }
            borrowed
                .document
                .validate()
                .map_err(|error| error.to_string())?;
            let resolved = borrowed.resolve_path(path);
            if let Some(parent) = resolved.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            }
            fs::write(
                &resolved,
                canonical_source(&borrowed.document, &borrowed.attribution)?,
            )
            .map_err(|error| format!("failed to write {}: {error}", resolved.display()))?;
            set_status(runtime, format!("Saved {}", resolved.display()))?;
        }
        "derive" => {
            let preview = {
                let mut borrowed = session.borrow_mut();
                derive_semantic_colors(&mut borrowed.document);
                borrowed.preview()?
            };
            runtime
                .replace_theme_variant_from_host(preview)
                .map_err(|error| error.to_string())?;
            sync_color_fields(runtime, &session.borrow().document)?;
            set_status(runtime, validation_status(&session.borrow().document))?;
        }
        _ => return Err(format!("unknown Theme Studio action `{action}`")),
    }
    Ok(EventResponse::new().stop())
}

fn load_into_session(
    session: &Rc<RefCell<StudioSession>>,
    source: &str,
    path: Option<PathBuf>,
    imported: bool,
    runtime: &mut UiRuntimeState,
) -> Result<(), String> {
    let engine = RuntimeEngine::new();
    let theme = load_theme_source(engine.engine(), "<theme-studio-import>", source)
        .map_err(|error| error.to_string())?;
    {
        let mut session = session.borrow_mut();
        session.document = theme;
        session.path = if imported { None } else { path };
        session.attribution = leading_attribution(source);
    }
    sync_document(
        runtime,
        &session.borrow(),
        if imported {
            "Imported as an unsaved copy"
        } else {
            "Opened theme"
        },
    )
}

fn sync_document(
    runtime: &mut UiRuntimeState,
    session: &StudioSession,
    status: &str,
) -> Result<(), String> {
    set_root_field(runtime, "path", UiValue::String(session.path_text()))?;
    set_root_field(
        runtime,
        "family",
        UiValue::String(session.document.family.clone()),
    )?;
    set_root_field(
        runtime,
        "variant",
        UiValue::String(session.document.name.clone()),
    )?;
    set_root_field(
        runtime,
        "mode",
        UiValue::String(
            match session.document.mode {
                ThemeMode::Light => "light",
                ThemeMode::Dark => "dark",
            }
            .to_owned(),
        ),
    )?;
    sync_color_fields(runtime, &session.document)?;
    runtime
        .replace_theme_variant_from_host(session.preview()?)
        .map_err(|error| error.to_string())?;
    set_status(
        runtime,
        format!("{status}. {}", validation_status(&session.document)),
    )
}

fn sync_color_fields(runtime: &mut UiRuntimeState, theme: &ThemeVariant) -> Result<(), String> {
    for &token in COLOR_TOKENS {
        set_root_field(
            runtime,
            &format!("color_{token}"),
            UiValue::String(format_color(theme.tokens.colors[token])),
        )?;
    }
    Ok(())
}

fn root_path(runtime: &UiRuntimeState) -> Result<ComponentInstancePath, String> {
    runtime
        .component_state
        .inspect()
        .into_iter()
        .find(|snapshot| snapshot.fields.contains_key("color_surface"))
        .map(|snapshot| snapshot.path)
        .ok_or_else(|| "Theme Studio root state is not mounted".to_owned())
}

fn set_root_field(runtime: &mut UiRuntimeState, field: &str, value: UiValue) -> Result<(), String> {
    let path = root_path(runtime)?;
    runtime
        .set_component_state_from_host(&path, field, value)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn set_status(runtime: &mut UiRuntimeState, status: String) -> Result<(), String> {
    set_root_field(runtime, "status", UiValue::String(status))
}

fn parse_color(value: &str) -> Result<Rgba8, String> {
    match ColorValue::parse(value).map_err(|error| error.to_string())? {
        ColorValue::Literal(color) => Ok(color),
        _ => Err("theme token references are not colors".to_owned()),
    }
}

fn format_color(color: Rgba8) -> String {
    let value = color.as_rgba_hex();
    if value & 0xff == 0xff {
        format!("#{:06x}", value >> 8)
    } else {
        format!("#{value:08x}")
    }
}

fn validation_status(theme: &ThemeVariant) -> String {
    if let Err(error) = theme.validate() {
        return format!("Invalid theme: {error}");
    }
    let resolved = match with_token_base(theme) {
        Ok(theme) => theme,
        Err(error) => return format!("Invalid theme: {error}"),
    };
    let theme = &resolved;
    let mut warnings = Vec::new();
    let mut pairs = Vec::new();
    for foreground in ["text_primary", "text_muted"] {
        for background in ["surface", "surface_raised", "surface_hover", "selection"] {
            pairs.push((foreground, background, 4.5));
        }
    }
    pairs.extend([
        ("on_accent", "accent", 4.5),
        ("on_accent", "accent_hover", 4.5),
        ("on_danger", "danger", 4.5),
        ("on_warning", "warning", 4.5),
        ("on_success", "success", 4.5),
        ("field.placeholder", "surface.field", 4.5),
        ("choice.description", "surface", 4.5),
        ("switch.thumb_off_ink", "switch.thumb", 4.5),
        ("text.subtle", "surface_raised", 4.5),
        ("button.ghost_fg", "surface_raised", 4.5),
        ("text.icon", "surface.field", 3.0),
    ]);
    for foreground in ["focus_ring", "control.border", "scrollbar.thumb"] {
        for background in ["surface", "surface_raised"] {
            pairs.push((foreground, background, 3.0));
        }
    }
    for (foreground, background, minimum) in pairs {
        let (Some(fg), Some(bg)) = (
            theme.tokens.color(foreground),
            theme.tokens.color(background),
        ) else {
            continue;
        };
        if bg.as_rgba_hex() & 0xff != 0xff {
            warnings.push(format!("{foreground}/{background}: translucent backing requires parent-specific contrast evidence"));
            continue;
        }
        let ratio = contrast(fg, bg);
        if ratio < minimum {
            warnings.push(format!(
                "{foreground}/{background} {ratio:.2}:1 (requires {minimum}:1)"
            ));
        }
    }
    if warnings.is_empty() {
        "Valid · required contrast pairs pass".to_owned()
    } else {
        format!("Valid with contrast warnings: {}", warnings.join(" · "))
    }
}

fn contrast(first: Rgba8, second: Rgba8) -> f64 {
    first.over(second).contrast_ratio(second)
}

fn mix(first: Rgba8, second: Rgba8, second_weight: u16) -> Rgba8 {
    let first = first.as_rgba_hex().to_be_bytes();
    let second = second.as_rgba_hex().to_be_bytes();
    let blend = |index: usize| {
        let first = u16::from(first[index]);
        let second = u16::from(second[index]);
        u8::try_from((first * (100 - second_weight) + second * second_weight) / 100)
            .unwrap_or(u8::MAX)
    };
    Rgba8::from_rgba_hex(u32::from_be_bytes([blend(0), blend(1), blend(2), 0xff]))
}

fn readable_on(color: Rgba8) -> Rgba8 {
    let dark = Rgba8::from_rgb_hex(0x0000_0000);
    let light = Rgba8::from_rgb_hex(0x00ff_ffff);
    if contrast(dark, color) >= contrast(light, color) {
        dark
    } else {
        light
    }
}

fn derive_semantic_colors(theme: &mut ThemeVariant) {
    let colors = &mut std::sync::Arc::make_mut(&mut theme.tokens).colors;
    let color = |colors: &BTreeMap<String, Rgba8>, name: &str| {
        colors.get(name).copied().unwrap_or(Rgba8::from_rgba_hex(0))
    };
    let surface = color(colors, "surface");
    let text = color(colors, "text_primary");
    let accent = color(colors, "accent");
    colors.insert("surface_raised".to_owned(), mix(surface, text, 6));
    colors.insert("surface_hover".to_owned(), mix(surface, text, 12));
    colors.insert("text_muted".to_owned(), mix(surface, text, 68));
    colors.insert("disabled".to_owned(), mix(surface, text, 48));
    colors.insert("border".to_owned(), mix(surface, text, 24));
    colors.insert(
        "accent_hover".to_owned(),
        mix(accent, readable_on(accent), 14),
    );
    colors.insert("on_accent".to_owned(), readable_on(accent));
    // Focus is drawn in the ink color so it never collides with accent fills.
    colors.insert("focus_ring".to_owned(), text);
    for (fill, foreground) in [
        ("danger", "on_danger"),
        ("warning", "on_warning"),
        ("success", "on_success"),
    ] {
        let fill = color(colors, fill);
        colors.insert(foreground.to_owned(), readable_on(fill));
    }
}

fn leading_attribution(source: &str) -> Vec<String> {
    source
        .lines()
        .take_while(|line| line.trim().is_empty() || line.trim_start().starts_with("//"))
        .filter(|line| line.trim_start().starts_with("//"))
        .map(ToOwned::to_owned)
        .collect()
}

/// Preserve the complete theme, not only the palette. The public exporter
/// checks a lossless round trip through the ordinary Rhai constructors.
fn canonical_source(theme: &ThemeVariant, attribution: &[String]) -> Result<String, String> {
    let mut output = String::new();
    for line in attribution {
        let _ = writeln!(output, "{line}");
    }
    if !attribution.is_empty() {
        output.push('\n');
    }
    output.push_str(&gpui_rhai::export_theme_source(theme).map_err(|error| error.to_string())?);
    Ok(output)
}

/// Merge the complete document over the bundled base. In particular Clean
/// typography, geometry and component recipes must survive this operation.
fn with_token_base(document: &ThemeVariant) -> Result<ThemeVariant, String> {
    let engine = RuntimeEngine::new();
    let source = canonical_source(document, &[])?;
    gpui_rhai::load_token_base(
        engine.engine(),
        "tokens.rhai",
        gpui_rhai_registry::TOKEN_BASE_SOURCE,
    )
    .and_then(|base| {
        gpui_rhai::load_theme_with_layers(
            engine.engine(),
            Some(&base),
            "<studio-preview>",
            &source,
            &gpui_rhai::ThemeTokenOverrides::default(),
        )
    })
    .map_err(|error| error.to_string())
}

fn default_draft() -> Result<ThemeVariant, String> {
    let engine = RuntimeEngine::new();
    load_theme_source(engine.engine(), "modern_dark.rhai", MODERN_DARK_THEME)
        .map_err(|error| error.to_string())
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}

fn source_with_state(theme: &ThemeVariant, path: &str, status: &str) -> String {
    let mut source = STUDIO_SOURCE.to_owned();
    for (placeholder, value) in [
        ("__PATH__", json_string(path)),
        ("__FAMILY__", json_string(&theme.family)),
        ("__VARIANT__", json_string(&theme.name)),
        (
            "__MODE__",
            json_string(match theme.mode {
                ThemeMode::Light => "light",
                ThemeMode::Dark => "dark",
            }),
        ),
        ("__STATUS__", json_string(status)),
        ("__PREVIEW_FAMILY__", json_string(&theme.family)),
        ("__PREVIEW_VARIANT__", json_string(&theme.name)),
    ] {
        source = source.replace(placeholder, &value);
    }
    for &token in COLOR_TOKENS {
        source = source.replace(
            &format!("__COLOR_{}__", token.to_ascii_uppercase()),
            &json_string(&format_color(theme.tokens.colors[token])),
        );
    }
    let visual_state = std::env::var("GPUI_RHAI_VISUAL_STATE").unwrap_or_default();
    let visual_locale = std::env::var("GPUI_RHAI_VISUAL_LOCALE")
        .ok()
        .filter(|locale| matches!(locale.as_str(), "en" | "zh-CN" | "ar"))
        .unwrap_or_else(|| "en".to_owned());
    source
        .replace("__BUILTIN_OPTIONS__", &builtin_options_source())
        .replace("__GALLERY_ONLY__", "false")
        .replace("__GALLERY_CATEGORY__", &json_string("all"))
        .replace(
            "__VISUAL_DIALOG__",
            if visual_state == "dialog" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__VISUAL_POPOVER__",
            if visual_state == "popover" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__VISUAL_COMMAND__",
            if visual_state == "command-dialog" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__VISUAL_SHEET__",
            if visual_state == "sheet" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__VISUAL_ALERT__",
            if visual_state == "alert-dialog" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__VISUAL_MENU__",
            if visual_state == "menu" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__VISUAL_TOAST__",
            if visual_state == "toast" {
                "true"
            } else {
                "false"
            },
        )
        .replace("__VISUAL_LOCALE__", &json_string(&visual_locale))
}

fn builtin_options_source() -> String {
    let engine = RuntimeEngine::new();
    let mut options = Vec::new();
    for &(file, source) in BUNDLED_THEME_SOURCES {
        if let Ok(theme) = load_theme_source(engine.engine(), file, source) {
            options.push(format!(
                "#{{ value: {}, label: {} }}",
                json_string(file.trim_end_matches(".rhai")),
                json_string(&format!("{} {}", theme.family, theme.name))
            ));
        }
    }
    format!("[{}]", options.join(", "))
}

fn module(id: &str, source: &str) -> (ModuleId, String) {
    (
        ModuleId::parse(id).expect("static Theme Studio module ID"),
        source.to_owned(),
    )
}

fn asset(bytes: &[u8]) -> AssetData {
    AssetData {
        mime_type: "image/svg+xml".to_owned(),
        bytes: bytes.to_vec(),
    }
}

fn studio_scripts(main: &str) -> EmbeddedScriptSource {
    let mut modules = BTreeMap::from([module("main", main)]);
    modules.extend(
        BUNDLED_COMPONENT_SOURCES_BY_ID
            .iter()
            .chain(BUNDLED_LAYOUT_SOURCES_BY_ID)
            .chain(BUNDLED_PATTERN_SOURCES_BY_ID)
            .map(|(id, source)| module(id, source)),
    );
    EmbeddedScriptSource::new(modules)
}

/// Launch the first-party gpui-rhai theme editor.
///
/// # Errors
///
/// Returns file, theme, compilation, or application errors.
pub fn run(root: PathBuf, path: Option<PathBuf>) -> Result<(), String> {
    let visual_theme = std::env::var("GPUI_RHAI_VISUAL_THEME").ok();
    let (document, attribution, path, status) = if let Some(path) = path {
        let resolved = if path.is_absolute() {
            path.clone()
        } else {
            root.join(&path)
        };
        let source = fs::read_to_string(&resolved)
            .map_err(|error| format!("failed to read {}: {error}", resolved.display()))?;
        let engine = RuntimeEngine::new();
        let theme = load_theme_source(engine.engine(), &resolved.to_string_lossy(), &source)
            .map_err(|error| error.to_string())?;
        (
            theme,
            leading_attribution(&source),
            Some(path),
            format!("Opened {}", resolved.display()),
        )
    } else if let Some(key) = visual_theme {
        let normalized_key = key.replace('-', "_");
        let source = BUNDLED_THEME_SOURCES
            .iter()
            .find(|(file, _)| file.trim_end_matches(".rhai") == normalized_key)
            .map(|(_, source)| *source)
            .ok_or_else(|| format!("unknown visual Theme Studio theme `{key}`"))?;
        let engine = RuntimeEngine::new();
        let theme = load_theme_source(engine.engine(), "<visual-theme>", source)
            .map_err(|error| error.to_string())?;
        (
            theme,
            leading_attribution(source),
            None,
            format!("Visual theme {key}"),
        )
    } else {
        (
            default_draft()?,
            Vec::new(),
            None,
            "New theme from Modern Dark".to_owned(),
        )
    };
    let session = Rc::new(RefCell::new(StudioSession {
        root,
        path,
        preview_family: document.family.clone(),
        preview_variant: document.name.clone(),
        document,
        attribution,
    }));
    let builtins = Rc::new(
        BUNDLED_THEME_SOURCES
            .iter()
            .map(|(file, source)| {
                (
                    file.trim_end_matches(".rhai").to_owned(),
                    (*source).to_owned(),
                )
            })
            .collect::<BTreeMap<_, _>>(),
    );
    launch(session, builtins, &status)
}

fn launch(
    session: Rc<RefCell<StudioSession>>,
    builtins: Rc<BTreeMap<String, String>>,
    status: &str,
) -> Result<(), String> {
    let borrowed = session.borrow();
    let main = source_with_state(
        &borrowed.document,
        &borrowed.path_text(),
        &format!("{status}. {}", validation_status(&borrowed.document)),
    );
    let primary = canonical_source(&borrowed.preview()?, &borrowed.attribution)?;
    let preview_identity = (
        borrowed.preview_family.clone(),
        borrowed.preview_variant.clone(),
    );
    drop(borrowed);
    let scripts = studio_scripts(&main);
    let engine = RuntimeEngine::new();
    let additional_themes = BUNDLED_THEME_SOURCES
        .iter()
        .filter_map(|(file, source)| {
            load_theme_source(engine.engine(), file, source)
                .ok()
                .filter(|theme| {
                    (theme.family.as_str(), theme.name.as_str())
                        != (preview_identity.0.as_str(), preview_identity.1.as_str())
                })
                .map(|_| ((*file).to_owned(), (*source).to_owned()))
        })
        .collect::<Vec<_>>();
    let entry = ModuleId::parse("main").map_err(|error| error.to_string())?;
    ScriptView::sources(entry, scripts, primary)
        .font_sources(crate::gallery::gallery_fonts()?)
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
        .theme_sources(additional_themes)
        .locale_sources([
            ("en.rhai".to_owned(), EN_LOCALE.to_owned()),
            ("zh_cn.rhai".to_owned(), ZH_CN_LOCALE.to_owned()),
            ("ar.rhai".to_owned(), AR_LOCALE.to_owned()),
        ])
        .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
            (
                path.strip_suffix(".svg").unwrap_or(path).to_owned(),
                asset(source.as_bytes()),
            )
        }))
        .extension(ThemeStudioExtension { session, builtins })
        .development(true)
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(1280.0, 820.0)
                .run()
        })
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_rhai::{
        RestrictedModuleResolver, ScriptLifecycle, UiEventHandler, UiNode, UiNodeKind,
    };

    fn find_key<'a>(node: &'a UiNode, key: &str) -> Option<&'a UiNode> {
        if node.key().is_some_and(|node_key| node_key.as_str() == key) {
            return Some(node);
        }
        match node.kind() {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                children.iter().find_map(|child| find_key(child, key))
            }
            _ => None,
        }
    }

    #[test]
    fn canonical_theme_round_trips_and_preserves_attribution() {
        let mut theme = load_theme_source(
            RuntimeEngine::new().engine(),
            "palette",
            gpui_rhai_registry::DEFAULT_THEME,
        )
        .unwrap();
        std::sync::Arc::make_mut(&mut theme.tokens)
            .namespaces
            .insert(
                "charts".to_owned(),
                BTreeMap::from([(
                    "series_a".to_owned(),
                    gpui_rhai::ThemeTokenValue::Color(Rgba8::from_rgb_hex(0x0033_66ff)),
                )]),
            );
        let source = canonical_source(&theme, &["// Attribution".to_owned()]).unwrap();
        assert!(source.starts_with("// Attribution\n\n"));
        let engine = RuntimeEngine::new();
        let reloaded = load_theme_source(engine.engine(), "roundtrip.rhai", &source).unwrap();
        assert_eq!(
            reloaded.tokens.color_snapshot(),
            theme.tokens.color_snapshot()
        );
        assert!(reloaded.tokens.spacing.is_empty() && reloaded.tokens.typography.roles.is_empty());
        let preview = with_token_base(&reloaded).unwrap();
        assert!(preview.tokens.color("table.selection").is_some());
        assert_eq!(
            canonical_source(&reloaded, &[]),
            canonical_source(&theme, &[]),
            "derived base colors are never written into the palette"
        );
    }

    #[test]
    fn full_presets_keep_their_recipes_through_studio_preview_and_save() {
        let engine = RuntimeEngine::new();
        for &(name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
            let original = load_theme_source(engine.engine(), name, source).unwrap();
            let preview = with_token_base(&original).unwrap();
            for (component, parts) in original.tokens.component_styles.iter() {
                let actual = preview
                    .tokens
                    .component_styles
                    .component(component)
                    .unwrap();
                for (part, expected) in parts {
                    assert_eq!(
                        actual.get(part),
                        Some(expected),
                        "{name}: preview {component}/{part}"
                    );
                }
            }
            assert!(
                preview.tokens.component_styles == original.tokens.component_styles,
                "{name}: preview changed the style sheet"
            );
            assert_eq!(
                preview.tokens.typography, original.tokens.typography,
                "{name}"
            );
            assert_eq!(preview.tokens.spacing, original.tokens.spacing, "{name}");
            let saved = canonical_source(&preview, &[]).unwrap();
            let reloaded = load_theme_source(engine.engine(), "saved.rhai", &saved).unwrap();
            for (component, parts) in preview.tokens.component_styles.iter() {
                let actual = reloaded
                    .tokens
                    .component_styles
                    .component(component)
                    .unwrap();
                for (part, expected) in parts {
                    assert_eq!(
                        actual.get(part),
                        Some(expected),
                        "{name}: {component}/{part}"
                    );
                }
            }
            assert_eq!(reloaded, preview, "{name}");
        }
    }

    #[test]
    fn validation_checks_hover_foregrounds_and_every_reading_surface() {
        let mut theme = load_theme_source(
            RuntimeEngine::new().engine(),
            "calibrated-palette",
            gpui_rhai_registry::DEFAULT_THEME,
        )
        .unwrap();
        assert!(!validation_status(&theme).contains("warnings"));
        let colors = &mut std::sync::Arc::make_mut(&mut theme.tokens).colors;
        colors.insert("accent_hover".to_owned(), colors["on_accent"]);
        let status = validation_status(&theme);
        assert!(status.contains("on_accent/accent_hover"), "{status}");
        let colors = &mut std::sync::Arc::make_mut(&mut theme.tokens).colors;
        colors.insert("surface_raised".to_owned(), colors["text_muted"]);
        let status = validation_status(&theme);
        assert!(status.contains("text_muted/surface_raised"), "{status}");
    }

    #[test]
    fn every_bundled_palette_passes_the_studio_readability_matrix() {
        let engine = RuntimeEngine::new();
        for &(name, source) in gpui_rhai_registry::BUNDLED_PALETTE_SOURCES {
            let theme = load_theme_source(engine.engine(), name, source).unwrap();
            let status = validation_status(&theme);
            assert!(!status.contains("warnings"), "{name}: {status}");
        }
    }

    #[test]
    fn exact_design_presets_keep_visible_contrast_warnings_instead_of_being_recolored() {
        let engine = RuntimeEngine::new();
        for &(name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
            let theme = load_theme_source(engine.engine(), name, source).unwrap();
            let status = validation_status(&theme);
            assert!(
                status.contains("warnings"),
                "{name}: source soft edges must not be silently calibrated"
            );
            assert!(!theme.tokens.component_styles.is_empty());
        }
    }

    #[test]
    fn studio_reports_placeholder_alpha_and_parent_dependent_backgrounds() {
        let engine = RuntimeEngine::new();
        let mut theme = load_theme_source(
            engine.engine(),
            "modern_light.rhai",
            gpui_rhai_registry::MODERN_LIGHT_THEME,
        )
        .unwrap();
        let original = theme.clone();
        let status = validation_status(&theme);
        assert!(
            status.contains("field.placeholder/surface.field"),
            "{status}"
        );
        assert!(status.contains("choice.description/surface"), "{status}");
        assert_eq!(theme, original, "reporting must not change the palette");
        let colors = &mut std::sync::Arc::make_mut(&mut theme.tokens).colors;
        colors.insert("text_muted".to_owned(), Rgba8::from_rgba_hex(0x0000_0010));
        let status = validation_status(&theme);
        assert!(status.contains("text_muted/surface_raised"), "{status}");
        assert!(
            contrast(
                Rgba8::from_rgba_hex(0x0000_0010),
                Rgba8::from_rgba_hex(0xffff_ffff)
            ) < 1.2
        );
        std::sync::Arc::make_mut(&mut theme.tokens).colors.insert(
            "surface_raised".to_owned(),
            Rgba8::from_rgba_hex(0xffff_ff80),
        );
        assert!(validation_status(&theme).contains("parent-specific contrast evidence"));
    }

    #[test]
    fn studio_source_contains_every_official_component() {
        for (component, _) in BUNDLED_COMPONENT_SOURCES_BY_ID {
            assert!(
                STUDIO_SOURCE.contains(&format!("import \"{component}\"")),
                "{component}"
            );
        }
    }

    #[test]
    fn studio_entry_does_not_shadow_builtins() {
        let document = default_draft().unwrap();
        let main = source_with_state(&document, "", &validation_status(&document));
        let found = RuntimeEngine::new()
            .lint_shadowed_builtins("studio", &main)
            .unwrap();
        assert!(found.is_empty(), "{found:?}");
    }

    fn assert_every_preview_switch(lifecycle: &mut ScriptLifecycle, engine: &mut RuntimeEngine) {
        assert!(find_key(lifecycle.root().unwrap(), "preview-contents-controls").is_some());
        let mut previous = "controls";
        // Cover every ordered pair: the budget counts both teardown and setup.
        for next in [
            "navigation",
            "data",
            "controls",
            "product",
            "navigation",
            "controls",
            "data",
            "navigation",
            "product",
            "data",
            "product",
            "controls",
        ] {
            let button = find_key(lifecycle.root().unwrap(), &format!("preview-switch-{next}"))
                .expect("preview switch is mounted");
            let Some(UiEventHandler::Script(callback)) = button.handler("click") else {
                panic!("preview switch has no script handler");
            };
            let _ = lifecycle
                .invoke_callback_transactional(engine, callback, UiValue::Null)
                .unwrap();
            assert!(
                lifecycle
                    .render_dirty(engine)
                    .unwrap_or_else(|error| panic!("preview {previous} -> {next}: {error}"))
            );
            let root = lifecycle.root().unwrap();
            assert!(find_key(root, &format!("preview-contents-{next}")).is_some());
            assert!(find_key(root, &format!("preview-contents-{previous}")).is_none());
            assert_eq!(
                find_key(root, &format!("preview-switch-{next}"))
                    .unwrap()
                    .attributes()
                    .get("pressed"),
                Some(&UiValue::Bool(true))
            );
            assert_eq!(
                find_key(root, &format!("preview-switch-{previous}"))
                    .unwrap()
                    .attributes()
                    .get("pressed"),
                Some(&UiValue::Bool(false))
            );
            previous = next;
        }
    }

    #[test]
    fn studio_executes_the_real_component_specimen() {
        let document = default_draft().unwrap();
        let session = Rc::new(RefCell::new(StudioSession {
            root: PathBuf::new(),
            path: None,
            preview_family: document.family.clone(),
            preview_variant: document.name.clone(),
            document: document.clone(),
            attribution: Vec::new(),
        }));
        let builtins = Rc::new(
            BUNDLED_THEME_SOURCES
                .iter()
                .map(|(file, source)| {
                    (
                        file.trim_end_matches(".rhai").to_owned(),
                        (*source).to_owned(),
                    )
                })
                .collect(),
        );
        let main = source_with_state(&document, "", &validation_status(&document));
        let scripts = studio_scripts(&main);
        let mut engine = RuntimeEngine::new();
        ThemeStudioExtension {
            session: Rc::clone(&session),
            builtins: Rc::clone(&builtins),
        }
        .configure_engine(&mut engine)
        .unwrap();
        engine.set_module_resolver(RestrictedModuleResolver::from_source(&scripts).unwrap());
        let compiled = engine
            .compile_self_contained_named("studio/main.rhai", &main)
            .unwrap();
        let schema = engine.root_state_schema(&compiled).unwrap();
        let mut runtime_state = UiRuntimeState::new();
        runtime_state.theme = Some(
            gpui_rhai::ThemeManager::from_variants(
                [document.clone()],
                gpui_rhai::ThemeSelection::new(&document.family, &document.name),
            )
            .unwrap(),
        );
        let locale = gpui_rhai::load_locale_source(engine.engine(), "en.rhai", EN_LOCALE).unwrap();
        runtime_state.locale = Some(gpui_rhai::LocaleManager::new([locale], "en", "en").unwrap());
        let runtime = Rc::new(RefCell::new(runtime_state));
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::clone(&runtime),
            ComponentInstancePath::root("ThemeStudio", "root"),
            Some("main".to_owned()),
            BTreeMap::new(),
            &schema,
        )
        .unwrap();
        lifecycle.start(&mut engine).unwrap();
        assert_every_preview_switch(&mut lifecycle, &mut engine);

        load_into_session(
            &session,
            super::super::ETHEREAL_THEME,
            Some(PathBuf::from("source.rhai")),
            true,
            &mut runtime.borrow_mut(),
        )
        .unwrap();
        assert_eq!(session.borrow().document.family, "Ethereal");
        assert!(session.borrow().path.is_none(), "Import must create a copy");
        let root = runtime
            .borrow()
            .component_state
            .inspect()
            .into_iter()
            .find(|snapshot| snapshot.fields.contains_key("color_surface"))
            .unwrap();
        assert_eq!(
            root.fields["family"].value,
            UiValue::String("Ethereal".to_owned())
        );
    }
}
