use std::collections::BTreeMap;

use gpui_rhai::{
    ComponentStateSchema, EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication,
    ScriptViewExtension, StateField, StoreId, UiRuntimeState, UiValue, ValueSchema,
};

const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const DIALOG: &str = include_str!("../../../registry/components/dialog.rhai");
const DEFAULT_DARK: &str = include_str!("../../../registry/themes/default_dark.rhai");
const DEFAULT_LIGHT: &str = include_str!("../../../registry/themes/default_light.rhai");
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
fn use_light(ctx, payload) { ctx.set_window_theme("Default", "Light"); }
fn use_dark(ctx, payload) { ctx.set_window_theme("Default", "Dark"); }
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
        text(`Window: ${id}`).with_style(style().font_size(rem(1.25))),
        text(`Shared count: ${ctx.get_app_store("shared", "count")}`),
        text(`Window draft: ${ctx.get_window_store("session", "draft")}`),
        row(actions).with_style(style().gap(px(8))),
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
        style().padding(px(24)).gap(px(16)).background(theme_color("surface"))
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
                    StateField::new(ValueSchema::Integer, UiValue::Integer(0)),
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

fn main() {
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        module("main", MAIN),
        module("components/button", BUTTON),
        module("components/dialog", DIALOG),
    ]));
    EmbeddedScriptView::new(ModuleId::parse("main").unwrap(), scripts, DEFAULT_DARK)
        .extension(MultiWindowStores)
        .theme_sources([(
            "themes/default_light.rhai".to_owned(),
            DEFAULT_LIGHT.to_owned(),
        )])
        .locale_sources([
            ("locales/en.rhai".to_owned(), EN.to_owned()),
            ("locales/ar.rhai".to_owned(), AR.to_owned()),
        ])
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(820.0, 460.0)
                .run()
        })
        .expect("multi_window failed");
}
