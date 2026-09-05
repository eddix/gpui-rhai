use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_rhai::{
    AssetData, ColorValue, ComponentInstancePath, EmbeddedScriptSource, EmbeddedScriptView,
    EventResponse, Length, ModuleId, NativeEvent, NativeHandlerDescriptor, NativeHandlerId, Rgba8,
    RuntimeEngine, ScriptApplication, ScriptViewExtension, ThemeMode, ThemeTokenValue,
    ThemeVariant, UiRuntimeState, UiValue, ValueSchema, load_theme_source,
};

use super::{
    ACCORDION_SOURCE, ALERT_DIALOG_SOURCE, ALERT_SOURCE, AR_LOCALE, AVATAR_SOURCE, BADGE_SOURCE,
    BUNDLED_THEME_SOURCES, BUTTON_GROUP_SOURCE, BUTTON_SOURCE, CARD_SOURCE, CHECK_SVG,
    CHECKBOX_SOURCE, CLOSE_SVG, COLLAPSIBLE_SOURCE, COMBOBOX_SOURCE, COMMAND_DIALOG_SOURCE,
    COMMAND_SOURCE, CONTEXT_MENU_SOURCE, DATE_NEXT_SVG, DATE_PICKER_SOURCE, DATE_PREVIOUS_SVG,
    DEFAULT_THEME, DIALOG_SOURCE, DIVIDER_SOURCE, EMPTY_SOURCE, EN_LOCALE, FORM_FIELD_SOURCE,
    GROUP_BOX_SOURCE, ICON_SOURCE, INPUT_GROUP_SOURCE, INPUT_SOURCE, KBD_SOURCE, LABEL_SOURCE,
    MENU_SOURCE, PAGINATION_SOURCE, POPOVER_SOURCE, PROGRESS_SOURCE, RADIO_GROUP_SOURCE,
    RADIO_SOURCE, SCROLL_AREA_SOURCE, SELECT_SOURCE, SHEET_SOURCE, SKELETON_SOURCE, SLIDER_SOURCE,
    SPINNER_SOURCE, SWITCH_SOURCE, TABLE_SOURCE, TABS_SOURCE, TAG_SOURCE, TEXTAREA_SOURCE,
    TOAST_SOURCE, TOGGLE_GROUP_SOURCE, TOGGLE_SOURCE, TOOLTIP_SOURCE, ZH_CN_LOCALE,
};

const STUDIO_SOURCE: &str = include_str!("../../../registry/studio/theme_studio.rhai");
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
    fn preview(&self) -> ThemeVariant {
        ThemeVariant {
            family: self.preview_family.clone(),
            name: self.preview_variant.clone(),
            mode: self.document.mode,
            tokens: self.document.tokens.clone(),
        }
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
                    session.document.tokens.colors.insert(token.clone(), color);
                    session.preview()
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
            session.preview()
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
                canonical_source(&borrowed.document, &borrowed.attribution),
            )
            .map_err(|error| format!("failed to write {}: {error}", resolved.display()))?;
            set_status(runtime, format!("Saved {}", resolved.display()))?;
        }
        "derive" => {
            let preview = {
                let mut borrowed = session.borrow_mut();
                derive_semantic_colors(&mut borrowed.document);
                borrowed.preview()
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
        .replace_theme_variant_from_host(session.preview())
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
        ColorValue::Token(_) => Err("theme token references are not colors".to_owned()),
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
    let mut warnings = Vec::new();
    for (foreground, background) in [
        ("text_primary", "surface"),
        ("text_muted", "surface"),
        ("on_accent", "accent"),
        ("on_danger", "danger"),
        ("on_warning", "warning"),
        ("on_success", "success"),
    ] {
        let ratio = contrast(
            theme.tokens.colors[foreground],
            theme.tokens.colors[background],
        );
        if ratio < 4.5 {
            warnings.push(format!("{foreground}/{background} {ratio:.2}:1"));
        }
    }
    let focus = contrast(
        theme.tokens.colors["focus_ring"],
        theme.tokens.colors["surface"],
    );
    if focus < 3.0 {
        warnings.push(format!("focus_ring/surface {focus:.2}:1"));
    }
    if warnings.is_empty() {
        "Valid · required contrast pairs pass".to_owned()
    } else {
        format!("Valid with contrast warnings: {}", warnings.join(" · "))
    }
}

fn contrast(first: Rgba8, second: Rgba8) -> f64 {
    let first = luminance(first);
    let second = luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn luminance(color: Rgba8) -> f64 {
    let [red, green, blue, _] = color.as_rgba_hex().to_be_bytes();
    0.2126 * linear_channel(red) + 0.7152 * linear_channel(green) + 0.0722 * linear_channel(blue)
}

fn linear_channel(channel: u8) -> f64 {
    let channel = f64::from(channel) / 255.0;
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
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
    let surface = theme.tokens.colors["surface"];
    let text = theme.tokens.colors["text_primary"];
    let accent = theme.tokens.colors["accent"];
    theme
        .tokens
        .colors
        .insert("surface_raised".to_owned(), mix(surface, text, 6));
    theme
        .tokens
        .colors
        .insert("surface_hover".to_owned(), mix(surface, text, 12));
    theme
        .tokens
        .colors
        .insert("text_muted".to_owned(), mix(surface, text, 68));
    theme
        .tokens
        .colors
        .insert("disabled".to_owned(), mix(surface, text, 48));
    theme
        .tokens
        .colors
        .insert("border".to_owned(), mix(surface, text, 24));
    theme.tokens.colors.insert(
        "accent_hover".to_owned(),
        mix(accent, readable_on(accent), 14),
    );
    theme
        .tokens
        .colors
        .insert("on_accent".to_owned(), readable_on(accent));
    theme.tokens.colors.insert("focus_ring".to_owned(), accent);
    for (fill, foreground) in [
        ("danger", "on_danger"),
        ("warning", "on_warning"),
        ("success", "on_success"),
    ] {
        theme.tokens.colors.insert(
            foreground.to_owned(),
            readable_on(theme.tokens.colors[fill]),
        );
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

fn canonical_source(theme: &ThemeVariant, attribution: &[String]) -> String {
    let mut output = String::new();
    for line in attribution {
        let _ = writeln!(output, "{line}");
    }
    if !attribution.is_empty() {
        output.push('\n');
    }
    output.push_str("fn theme() {\n    #{\n");
    let _ = writeln!(output, "        family: {},", json_string(&theme.family));
    let _ = writeln!(output, "        name: {},", json_string(&theme.name));
    let _ = writeln!(
        output,
        "        mode: \"{}\",",
        match theme.mode {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    );
    output.push_str("        tokens: #{\n            colors: #{\n");
    for &token in COLOR_TOKENS {
        let _ = writeln!(
            output,
            "                {token}: 0x{:08x},",
            theme.tokens.colors[token].as_rgba_hex()
        );
    }
    output.push_str("            },\n");
    write_length_map(&mut output, "spacing", &theme.tokens.spacing);
    write_length_map(&mut output, "radii", &theme.tokens.radii);
    if !theme.tokens.namespaces.is_empty() {
        output.push_str("            namespaces: #{\n");
        for (namespace, tokens) in &theme.tokens.namespaces {
            let _ = writeln!(output, "                {namespace}: #{{");
            for (name, value) in tokens {
                let encoded = match value {
                    ThemeTokenValue::Color(color) => {
                        format!(
                            "#{{ type: \"color\", value: 0x{:08x} }}",
                            color.as_rgba_hex()
                        )
                    }
                    ThemeTokenValue::Length(length) => format!(
                        "#{{ type: \"length\", value: {} }}",
                        encoded_length(*length)
                    ),
                    ThemeTokenValue::Number(number) => {
                        format!("#{{ type: \"number\", value: {number:?} }}")
                    }
                    ThemeTokenValue::String(value) => {
                        format!("#{{ type: \"string\", value: {} }}", json_string(value))
                    }
                };
                let _ = writeln!(output, "                    {name}: {encoded},");
            }
            output.push_str("                },\n");
        }
        output.push_str("            },\n");
    }
    output.push_str("        },\n    }\n}\n");
    output
}

fn write_length_map(output: &mut String, name: &str, values: &BTreeMap<String, Length>) {
    let _ = writeln!(output, "            {name}: #{{");
    for (token, value) in values {
        let _ = writeln!(
            output,
            "                {token}: {},",
            encoded_length(*value)
        );
    }
    output.push_str("            },\n");
}

fn encoded_length(length: Length) -> String {
    match length {
        Length::Pixels(value) => {
            format!("#{{ unit: \"pixels\", value: {value:?} }}")
        }
        Length::Rems(value) => format!("#{{ unit: \"rems\", value: {value:?} }}"),
        Length::Relative(value) => {
            format!("#{{ unit: \"relative\", value: {value:?} }}")
        }
        Length::ThemeSpacing(_) | Length::ThemeRadius(_) => {
            unreachable!("validated theme documents cannot nest length tokens")
        }
    }
}

fn default_draft() -> Result<ThemeVariant, String> {
    let engine = RuntimeEngine::new();
    load_theme_source(engine.engine(), "default_dark.rhai", DEFAULT_THEME)
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
    EmbeddedScriptSource::new(BTreeMap::from([
        module("main", main),
        module("components/accordion", ACCORDION_SOURCE),
        module("components/avatar", AVATAR_SOURCE),
        module("components/button", BUTTON_SOURCE),
        module("components/checkbox", CHECKBOX_SOURCE),
        module("components/collapsible", COLLAPSIBLE_SOURCE),
        module("components/date_picker", DATE_PICKER_SOURCE),
        module("components/dialog", DIALOG_SOURCE),
        module("components/divider", DIVIDER_SOURCE),
        module("components/combobox", COMBOBOX_SOURCE),
        module("components/form_field", FORM_FIELD_SOURCE),
        module("components/icon", ICON_SOURCE),
        module("components/input", INPUT_SOURCE),
        module("components/label", LABEL_SOURCE),
        module("components/menu", MENU_SOURCE),
        module("components/pagination", PAGINATION_SOURCE),
        module("components/popover", POPOVER_SOURCE),
        module("components/progress", PROGRESS_SOURCE),
        module("components/radio", RADIO_SOURCE),
        module("components/radio_group", RADIO_GROUP_SOURCE),
        module("components/select", SELECT_SOURCE),
        module("components/skeleton", SKELETON_SOURCE),
        module("components/switch", SWITCH_SOURCE),
        module("components/table", TABLE_SOURCE),
        module("components/tabs", TABS_SOURCE),
        module("components/tag", TAG_SOURCE),
        module("components/textarea", TEXTAREA_SOURCE),
        module("components/toast", TOAST_SOURCE),
        module("components/tooltip", TOOLTIP_SOURCE),
        module("components/alert", ALERT_SOURCE),
        module("components/alert_dialog", ALERT_DIALOG_SOURCE),
        module("components/badge", BADGE_SOURCE),
        module("components/button_group", BUTTON_GROUP_SOURCE),
        module("components/card", CARD_SOURCE),
        module("components/command", COMMAND_SOURCE),
        module("components/command_dialog", COMMAND_DIALOG_SOURCE),
        module("components/context_menu", CONTEXT_MENU_SOURCE),
        module("components/empty", EMPTY_SOURCE),
        module("components/group_box", GROUP_BOX_SOURCE),
        module("components/input_group", INPUT_GROUP_SOURCE),
        module("components/kbd", KBD_SOURCE),
        module("components/sheet", SHEET_SOURCE),
        module("components/slider", SLIDER_SOURCE),
        module("components/spinner", SPINNER_SOURCE),
        module("components/toggle", TOGGLE_SOURCE),
        module("components/toggle_group", TOGGLE_GROUP_SOURCE),
        module("components/scroll_area", SCROLL_AREA_SOURCE),
    ]))
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
            "New theme from Default Dark".to_owned(),
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
    let primary = canonical_source(&borrowed.preview(), &borrowed.attribution);
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
    EmbeddedScriptView::new(entry, scripts, primary)
        .theme_sources(additional_themes)
        .locale_sources([
            ("en.rhai".to_owned(), EN_LOCALE.to_owned()),
            ("zh_cn.rhai".to_owned(), ZH_CN_LOCALE.to_owned()),
            ("ar.rhai".to_owned(), AR_LOCALE.to_owned()),
        ])
        .asset_sources([
            ("icons/check".to_owned(), asset(CHECK_SVG.as_bytes())),
            ("icons/close".to_owned(), asset(CLOSE_SVG.as_bytes())),
            (
                "icons/chevron_left".to_owned(),
                asset(super::CHEVRON_LEFT_SVG.as_bytes()),
            ),
            (
                "icons/chevron_right".to_owned(),
                asset(super::CHEVRON_RIGHT_SVG.as_bytes()),
            ),
            (
                "icons/calendar".to_owned(),
                asset(super::CALENDAR_SVG.as_bytes()),
            ),
            (
                "icons/date_previous".to_owned(),
                asset(DATE_PREVIOUS_SVG.as_bytes()),
            ),
            (
                "icons/date_next".to_owned(),
                asset(DATE_NEXT_SVG.as_bytes()),
            ),
        ])
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
    use gpui_rhai::{RestrictedModuleResolver, ScriptLifecycle};

    #[test]
    fn canonical_theme_round_trips_and_preserves_attribution() {
        let mut theme = default_draft().unwrap();
        theme
            .tokens
            .spacing
            .insert("xs".to_owned(), Length::Rems(0.25));
        theme.tokens.namespaces.insert(
            "charts".to_owned(),
            BTreeMap::from([
                (
                    "series_a".to_owned(),
                    ThemeTokenValue::Color(Rgba8::from_rgb_hex(0x0033_66ff)),
                ),
                (
                    "stroke".to_owned(),
                    ThemeTokenValue::Length(Length::Pixels(2.0)),
                ),
                ("alpha".to_owned(), ThemeTokenValue::Number(0.6)),
                (
                    "label".to_owned(),
                    ThemeTokenValue::String("Primary".to_owned()),
                ),
            ]),
        );
        let source = canonical_source(&theme, &["// Attribution".to_owned()]);
        assert!(source.starts_with("// Attribution\n\n"));
        let engine = RuntimeEngine::new();
        assert_eq!(
            load_theme_source(engine.engine(), "roundtrip.rhai", &source).unwrap(),
            theme
        );
    }

    #[test]
    fn studio_source_contains_every_official_component() {
        for component in [
            "accordion",
            "avatar",
            "button",
            "checkbox",
            "collapsible",
            "date_picker",
            "dialog",
            "divider",
            "combobox",
            "form_field",
            "icon",
            "input",
            "label",
            "menu",
            "pagination",
            "popover",
            "progress",
            "radio_group",
            "radio",
            "select",
            "skeleton",
            "switch_component",
            "table",
            "tabs",
            "tag",
            "textarea",
            "toast",
            "tooltip",
            "alert",
            "alert_dialog",
            "badge",
            "button_group",
            "card",
            "command",
            "command_dialog",
            "context_menu",
            "empty",
            "group_box",
            "input_group",
            "kbd",
            "sheet",
            "slider",
            "spinner",
            "toggle",
            "toggle_group",
            "scroll_area",
        ] {
            assert!(
                STUDIO_SOURCE.contains(&format!("{component}::")),
                "{component}"
            );
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
        assert!(lifecycle.root().is_some());

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
