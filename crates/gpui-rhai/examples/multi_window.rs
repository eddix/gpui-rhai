#[path = "support/fonts.rs"]
mod shared_fonts;
#[path = "support/themes.rs"]
mod shared_themes;

use std::collections::BTreeMap;

use gpui_rhai::{
    ComponentStateSchema, EmbeddedScriptSource, ModuleId, ScriptApplication, ScriptView,
    ScriptViewExtension, StateField, StoreId, UiRuntimeState, UiValue, ValueSchema,
};

const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const DIALOG: &str = include_str!("../../../registry/components/dialog.rhai");
const EN: &str = include_str!("../../../registry/locales/en.rhai");
const AR: &str = include_str!("../../../registry/locales/ar.rhai");

const MAIN: &str = r#"
import "components/button" as button;
import "components/dialog" as dialog;

fn state_schema() {
    #{ fields: #{
        close_pending: #{ schema: #{ type: "bool" },
            "default": #{ type: "bool", value: false } },
    } }
}

fn request_close(ctx, payload) { ctx.set_state("close_pending", true); }
fn set_close_pending(ctx, open) { ctx.set_state("close_pending", open); }
fn cancel_close(ctx, payload) { ctx.set_state("close_pending", false); }
fn confirm_close(ctx, payload) { ctx.close_window(ctx.window_id()); }

fn init(ctx) {
    ctx.set_close_handler(Fn("request_close"));
    if ctx.window_id() == "main" {
        ctx.open_window("settings", "GPUI Rhai Settings", 560, 440, true);
    }
}

fn open_settings(ctx, payload) {
    ctx.open_window("settings", "GPUI Rhai Settings", 560, 440, true);
}
fn focus_settings(ctx, payload) { ctx.focus_window("settings"); }
fn increment_shared(ctx, payload) {
    ctx.set_app_store("shared", "count", ctx.get_app_store("shared", "count") + 1);
}
fn edit_window_draft(ctx, payload) {
    ctx.set_window_store("session", "draft", `edited in ${ctx.window_id()}`);
}
fn use_light(ctx, payload) { ctx.set_window_theme(ctx.theme_variant().family, "Light"); }
fn use_dark(ctx, payload) { ctx.set_window_theme(ctx.theme_variant().family, "Dark"); }
fn use_english(ctx, payload) { ctx.set_window_locale("en"); }
fn use_arabic(ctx, payload) { ctx.set_window_locale("ar"); }

fn view(ctx) {
    let id = ctx.window_id();
    let actions = [
        button::Button(#{ text: "Increment shared counter", on_click: Fn("increment_shared") }),
        button::Button(#{ text: "Edit this window's draft", variant: "secondary", on_click: Fn("edit_window_draft") }),
        button::Button(#{ text: "Light", variant: "ghost", on_click: Fn("use_light") }),
        button::Button(#{ text: "Dark", variant: "ghost", on_click: Fn("use_dark") }),
        button::Button(#{ text: "English", variant: "ghost", on_click: Fn("use_english") }),
        button::Button(#{ text: "العربية", variant: "ghost", on_click: Fn("use_arabic") }),
    ];
    if id == "main" {
        actions.push(button::Button(#{ text: "Open settings window", on_click: Fn("open_settings") }));
        actions.push(button::Button(#{ text: "Focus settings window", variant: "secondary", on_click: Fn("focus_settings") }));
    }
    column([
        text(`Window: ${id}`).with_style(style().typography("heading")),
        text(`Shared count: ${ctx.get_app_store("shared", "count")}`),
        text(`Window draft: ${ctx.get_window_store("session", "draft")}`),
        row(actions).with_style(style().flex_wrap().gap(theme_spacing("sm"))),
        dialog::Dialog(#{
            key: `close-${id}`,
            open: ctx.get_state("close_pending"),
            title: "Close this window?",
            content: text("Window-owned state and async work will be released."),
            actions: [
                button::Button(#{ text: "Cancel", variant: "secondary", on_click: Fn("cancel_close") }),
                button::Button(#{ text: "Close", variant: "danger", on_click: Fn("confirm_close") })
            ],
            on_open_change: Fn("set_close_pending")
        })
    ]).with_style(
        style().width(relative(1.0)).height(relative(1.0))
            .padding(theme_spacing("lg")).gap(theme_spacing("md"))
            .typography("body").text_color(theme_color("text_primary"))
            .background(theme_color("surface"))
    )
}
"#;

struct MultiWindowStores;

impl ScriptViewExtension for MultiWindowStores {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .stores
            .declare(
                StoreId::app("shared"),
                ComponentStateSchema::new(BTreeMap::from([(
                    "count".to_owned(),
                    StateField::new(ValueSchema::integer(), UiValue::Integer(0)),
                )]))
                .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())
    }

    fn configure_window(
        &self,
        window_id: &str,
        runtime: &mut UiRuntimeState,
    ) -> Result<(), String> {
        runtime
            .stores
            .declare(
                StoreId::window(window_id, "session"),
                ComponentStateSchema::new(BTreeMap::from([(
                    "draft".to_owned(),
                    StateField::new(
                        ValueSchema::string(),
                        UiValue::String("unchanged".to_owned()),
                    ),
                )]))
                .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())
    }
}

fn module(id: &str, source: &str) -> (ModuleId, String) {
    (
        ModuleId::parse(id).expect("static module ID"),
        source.to_owned(),
    )
}

/// Build the multi-window tutorial, including its shared and per-window stores.
///
/// # Panics
/// Panics if a static module identifier is invalid.
#[must_use]
pub fn view() -> ScriptView {
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        module("main", MAIN),
        module("components/button", BUTTON),
        module("components/spinner", gpui_rhai_registry::SPINNER_SOURCE),
        module("components/dialog", DIALOG),
    ]));
    ScriptView::sources(
        ModuleId::parse("main").unwrap(),
        scripts,
        shared_themes::PRIMARY,
    )
    .token_base(include_str!("../../../registry/tokens.rhai"))
    .font_sources(shared_fonts::sources())
    .extension(MultiWindowStores)
    .theme_sources(shared_themes::additional())
    .locale_sources([
        ("locales/en.rhai".to_owned(), EN.to_owned()),
        ("locales/ar.rhai".to_owned(), AR.to_owned()),
    ])
}

fn main() {
    view()
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(820.0, 460.0)
                .run()
        })
        .expect("multi_window failed");
}
