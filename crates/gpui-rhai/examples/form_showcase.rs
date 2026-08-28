use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptApp, EmbeddedScriptSource, ModuleId};

const INPUT: &str = include_str!("../../../registry/components/input.rhai");
const LABEL: &str = include_str!("../../../registry/components/label.rhai");
const FORM_FIELD: &str = include_str!("../../../registry/components/form_field.rhai");
const CHECKBOX: &str = include_str!("../../../registry/components/checkbox.rhai");
const RADIO: &str = include_str!("../../../registry/components/radio.rhai");
const RADIO_GROUP: &str = include_str!("../../../registry/components/radio_group.rhai");
const SWITCH: &str = include_str!("../../../registry/components/switch.rhai");
const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const DIALOG: &str = include_str!("../../../registry/components/dialog.rhai");
const TOAST: &str = include_str!("../../../registry/components/toast.rhai");
const DEFAULT_LIGHT: &str = include_str!("../../../registry/themes/default_light.rhai");
const DEFAULT_DARK: &str = include_str!("../../../registry/themes/default_dark.rhai");
const TOKYO_NIGHT: &str = include_str!("../../../registry/themes/tokyo_night.rhai");
const TOKYO_STORM: &str = include_str!("../../../registry/themes/tokyo_storm.rhai");
const CATPPUCCIN_LATTE: &str = include_str!("../../../registry/themes/catppuccin_latte.rhai");
const CATPPUCCIN_MOCHA: &str = include_str!("../../../registry/themes/catppuccin_mocha.rhai");
const EN: &str = include_str!("../../../registry/locales/en.rhai");
const ZH_CN: &str = include_str!("../../../registry/locales/zh_cn.rhai");
const AR: &str = include_str!("../../../registry/locales/ar.rhai");

const MAIN: &str = r#"
import "components/input" as input;
import "components/form_field" as form_field;
import "components/checkbox" as checkbox;
import "components/radio_group" as radio_group;
import "components/switch" as switch_component;
import "components/button" as button;
import "components/dialog" as dialog;
import "components/toast" as toast;

fn state_schema() {
    #{ fields: #{
        name: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "__VISUAL_NAME__" } },
        accepted: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_ACCEPTED__ } },
        plan: #{ schema: #{ type: "string", allowed: ["personal", "team"] },
            "default": #{ type: "string", value: "personal" } },
        updates: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } },
        dialog_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_DIALOG__ } },
        toast_visible: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_TOAST__ } },
    } }
}

fn set_name(ctx, value) { ctx.set_state("name", value); }
fn set_accepted(ctx, value) { ctx.set_state("accepted", value.checked); }
fn set_plan(ctx, value) { ctx.set_state("plan", value); }
fn set_updates(ctx, value) { ctx.set_state("updates", value); }
fn set_dialog(ctx, open) { ctx.set_state("dialog_open", open); }
fn close_dialog(ctx, payload) { ctx.set_state("dialog_open", false); }
fn open_dialog(ctx, payload) {
    if ctx.get_state("name") != "" { ctx.set_state("dialog_open", true); }
}
fn confirm(ctx, payload) {
    ctx.set_state("dialog_open", false);
    ctx.set_state("toast_visible", true);
}
fn dismiss_toast(ctx, id) { ctx.set_state("toast_visible", false); }

fn init(ctx) {
    let theme = "__VISUAL_THEME__";
    if theme == "default-light" { ctx.set_theme("Default", "Light"); }
    else if theme == "default-dark" { ctx.set_theme("Default", "Dark"); }
    else if theme == "tokyo-night" { ctx.set_theme("Tokyo Night", "Night"); }
    else if theme == "tokyo-storm" { ctx.set_theme("Tokyo Night", "Storm"); }
    else if theme == "catppuccin-latte" { ctx.set_theme("Catppuccin", "Latte"); }
    else if theme == "catppuccin-mocha" { ctx.set_theme("Catppuccin", "Mocha"); }
    ctx.set_locale("__VISUAL_LOCALE__");
}

fn view(ctx) {
    let name = ctx.get_state("name");
    let error = if name == "" { "Name is required" } else { () };
    let toasts = [];
    if ctx.get_state("toast_visible") {
        toasts.push(#{
            id: "saved", title: "Profile saved",
            message: `Saved ${name} on the ${ctx.get_state("plan")} plan.`,
            variant: "success", region: "bottom_right", duration_ms: __TOAST_DURATION__,
        });
    }
    column([
        toast::Toast(#{ key: "form-toasts", items: toasts, on_dismiss: Fn("dismiss_toast") }),
        text("Profile form").with_style(style().font_size(rem(1.25))),
        form_field::FormField(#{
            id: "name-field", label: "Name", required: true,
            description: "Displayed to collaborators", error: error,
            control: input::Input(#{
                key: "name", value: name, placeholder: "Ada Lovelace",
                error: error != (), on_change: Fn("set_name")
            })
        }),
        checkbox::Checkbox(#{
            checked: ctx.get_state("accepted"), label: "I accept the project terms",
            on_change: Fn("set_accepted")
        }),
        radio_group::RadioGroup(#{
            value: ctx.get_state("plan"), label: "Plan", orientation: "horizontal",
            options: [
                #{ value: "personal", label: "Personal" },
                #{ value: "team", label: "Team" }
            ],
            on_change: Fn("set_plan")
        }),
        switch_component::Switch(#{
            checked: ctx.get_state("updates"), label: "Product updates",
            on_change: Fn("set_updates")
        }),
        button::Button(#{ text: "Review", on_click: Fn("open_dialog") }),
        dialog::Dialog(#{
            key: "review", open: ctx.get_state("dialog_open"), title: "Review profile",
            content: column([
                text(`Name: ${name}`),
                text(`Plan: ${ctx.get_state("plan")}`)
            ]).with_style(style().gap(px(6))),
            actions: [
                button::Button(#{ text: "Cancel", variant: "secondary", on_click: Fn("close_dialog") }),
                button::Button(#{ text: "Confirm", on_click: Fn("confirm") })
            ],
            on_open_change: Fn("set_dialog")
        })
    ]).with_style(
        style().width(px(560)).padding(px(24)).gap(px(16))
            .background(theme_color("surface"))
    )
}
"#;

fn module(id: &str, source: &str) -> (ModuleId, String) {
    (
        ModuleId::parse(id).expect("static module ID"),
        source.to_owned(),
    )
}

fn main() {
    let visual_theme = visual_theme("default-light");
    let visual_locale = visual_locale();
    let visual_state = std::env::var("GPUI_RHAI_VISUAL_STATE")
        .ok()
        .filter(|state| matches!(state.as_str(), "default" | "dialog" | "toast"))
        .unwrap_or_else(|| "default".to_owned());
    let populated = visual_state != "default";
    let main_source = MAIN
        .replace("__VISUAL_THEME__", &visual_theme)
        .replace("__VISUAL_LOCALE__", &visual_locale)
        .replace(
            "__VISUAL_NAME__",
            if populated { "Ada Lovelace" } else { "" },
        )
        .replace(
            "__VISUAL_ACCEPTED__",
            if populated { "true" } else { "false" },
        )
        .replace(
            "__VISUAL_DIALOG__",
            if visual_state == "dialog" {
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
        .replace(
            "__TOAST_DURATION__",
            if visual_state == "toast" {
                "60000"
            } else {
                "2200"
            },
        );
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        module("main", &main_source),
        module("components/input", INPUT),
        module("components/label", LABEL),
        module("components/form_field", FORM_FIELD),
        module("components/checkbox", CHECKBOX),
        module("components/radio", RADIO),
        module("components/radio_group", RADIO_GROUP),
        module("components/switch", SWITCH),
        module("components/button", BUTTON),
        module("components/dialog", DIALOG),
        module("components/toast", TOAST),
    ]));
    EmbeddedScriptApp::new(ModuleId::parse("main").unwrap(), scripts, DEFAULT_LIGHT)
        .theme_sources([
            ("default_dark.rhai".to_owned(), DEFAULT_DARK.to_owned()),
            ("tokyo_night.rhai".to_owned(), TOKYO_NIGHT.to_owned()),
            ("tokyo_storm.rhai".to_owned(), TOKYO_STORM.to_owned()),
            (
                "catppuccin_latte.rhai".to_owned(),
                CATPPUCCIN_LATTE.to_owned(),
            ),
            (
                "catppuccin_mocha.rhai".to_owned(),
                CATPPUCCIN_MOCHA.to_owned(),
            ),
        ])
        .locale_sources([
            ("en.rhai".to_owned(), EN.to_owned()),
            ("zh_cn.rhai".to_owned(), ZH_CN.to_owned()),
            ("ar.rhai".to_owned(), AR.to_owned()),
        ])
        .window_size(680.0, 680.0)
        .run()
        .expect("form_showcase failed");
}

fn visual_theme(default: &str) -> String {
    std::env::var("GPUI_RHAI_VISUAL_THEME")
        .ok()
        .filter(|theme| {
            matches!(
                theme.as_str(),
                "default-light"
                    | "default-dark"
                    | "tokyo-night"
                    | "tokyo-storm"
                    | "catppuccin-latte"
                    | "catppuccin-mocha"
            )
        })
        .unwrap_or_else(|| default.to_owned())
}

fn visual_locale() -> String {
    std::env::var("GPUI_RHAI_VISUAL_LOCALE")
        .ok()
        .filter(|locale| matches!(locale.as_str(), "en" | "zh-CN" | "ar"))
        .unwrap_or_else(|| "en".to_owned())
}
