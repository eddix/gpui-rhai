use std::collections::BTreeMap;

use gpui_rhai::{
    AssetData, CalendarClock, EmbeddedScriptSource, EmbeddedScriptView, GregorianDate, ModuleId,
    ScriptApplication,
};

const INPUT: &str = include_str!("../../../registry/components/input.rhai");
const TEXTAREA: &str = include_str!("../../../registry/components/textarea.rhai");
const SELECT: &str = include_str!("../../../registry/components/select.rhai");
const COMBOBOX: &str = include_str!("../../../registry/components/combobox.rhai");
const DATE_PICKER: &str = include_str!("../../../registry/components/date_picker.rhai");
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
import "components/textarea" as textarea;
import "components/select" as select;
import "components/date_picker" as date_picker;
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
        country: #{ schema: #{ type: "optional", value: #{ type: "string" } },
            "default": __VISUAL_COUNTRY__ },
        country_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        country_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        appointment: #{ schema: #{ type: "optional", value: #{ type: "string" } },
            "default": __VISUAL_APPOINTMENT__ },
        notes: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "__VISUAL_NOTES__" } },
        fixed_note: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "Fixed\nrows" } },
        limit_note: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "0123456789" } },
        dialog_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_DIALOG__ } },
        toast_visible: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_TOAST__ } },
    } }
}

fn set_name(ctx, value) { ctx.set_state("name", value); }
fn set_accepted(ctx, value) { ctx.set_state("accepted", value.checked); }
fn set_plan(ctx, value) { ctx.set_state("plan", value); }
fn set_updates(ctx, value) { ctx.set_state("updates", value); }
fn set_country(ctx, value) { ctx.set_state("country", value); }
fn set_country_open(ctx, value) { ctx.set_state("country_open", value); }
fn set_country_query(ctx, value) { ctx.set_state("country_query", value); }
fn set_appointment(ctx, value) { ctx.set_state("appointment", value); }
fn set_notes(ctx, value) { ctx.set_state("notes", value); }
fn set_limit_note(ctx, value) { ctx.set_state("limit_note", value); }
fn use_english(ctx, payload) { ctx.set_locale("en"); }
fn use_chinese(ctx, payload) { ctx.set_locale("zh-CN"); }
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

fn profile_fields(ctx, name, error) {
    column([
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
        })
    ]).with_style(style().width(px(300)).gap(theme_spacing("sm")))
}

fn advanced_fields(ctx) {
    column([
        row([
            button::Button(#{ text: "English", size: "xs", variant: "ghost", on_click: Fn("use_english") }),
            button::Button(#{ text: "简体中文", size: "xs", variant: "ghost", on_click: Fn("use_chinese") })
        ]).with_style(style().gap(theme_spacing("xs"))),
        text("Country or region"),
        select::Select(#{
            key: "country", value: ctx.get_state("country"),
            open: ctx.get_state("country_open"), query: ctx.get_state("country_query"), searchable: true,
            clearable: true, placeholder: "Choose a country",
            options: [
                #{ value: "cn", label: "China", group: "Asia", keywords: ["zhongguo"] },
                #{ value: "jp", label: "Japan", group: "Asia" },
                #{ value: "fr", label: "France", group: "Europe" },
                #{ value: "de", label: "Germany", group: "Europe" }
            ], on_change: Fn("set_country"), on_open_change: Fn("set_country_open"),
            on_query_change: Fn("set_country_query")
        }),
        text("Appointment date"),
        date_picker::DatePicker(#{
            key: "appointment", value: ctx.get_state("appointment"),
            min_date: "2026-08-30", max_date: "2026-12-31", clearable: true,
            placeholder: "Choose a date",
            presets: [
                #{ label: "Launch", value: "2026-09-01" },
                #{ label: "Review", value: "2026-10-15" }
            ], on_change: Fn("set_appointment")
        }),
        text("Notes"),
        textarea::Textarea(#{
            key: "notes", value: ctx.get_state("notes"),
            placeholder: "Add feedback or context", min_rows: 3, max_rows: 6,
            max_length: 240, show_count: true,
            error: ctx.get_state("notes") == "", on_change: Fn("set_notes")
        }),
        row([
            textarea::Textarea(#{
                key: "fixed-note", value: ctx.get_state("fixed_note"),
                rows: 2, read_only: true
            }).with_style(style().width(px(145))),
            textarea::Textarea(#{
                key: "limit-note", value: ctx.get_state("limit_note"),
                min_rows: 1, max_rows: 2, max_length: 10, show_count: true,
                on_change: Fn("set_limit_note")
            }).with_style(style().width(px(145)))
        ]).with_style(style().gap(theme_spacing("xs")))
    ]).with_style(style().width(px(300)).gap(theme_spacing("xs")))
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
        row([
            profile_fields(ctx, name, error),
            advanced_fields(ctx)
        ]).with_style(style().gap(theme_spacing("lg")).items_start()),
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
        style().width(px(640)).padding(px(24)).gap(px(16))
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
        .filter(|state| {
            matches!(
                state.as_str(),
                "default" | "dialog" | "toast" | "date-picker" | "textarea"
            )
        })
        .unwrap_or_else(|| "default".to_owned());
    let main_source = visual_source(&visual_theme, &visual_locale, &visual_state);
    let date_picker_source = visual_date_picker_source(&visual_state);
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        module("main", &main_source),
        module("components/input", INPUT),
        module("components/textarea", TEXTAREA),
        module("components/select", SELECT),
        module("components/combobox", COMBOBOX),
        module("components/date_picker", &date_picker_source),
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
    EmbeddedScriptView::new(ModuleId::parse("main").unwrap(), scripts, DEFAULT_LIGHT)
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
        .asset_sources([
            (
                "icons/calendar".to_owned(),
                svg(include_bytes!(
                    "../../../registry/assets/icons/calendar.svg"
                )),
            ),
            (
                "icons/date_previous".to_owned(),
                svg(include_bytes!(
                    "../../../registry/assets/icons/date_previous.svg"
                )),
            ),
            (
                "icons/date_next".to_owned(),
                svg(include_bytes!(
                    "../../../registry/assets/icons/date_next.svg"
                )),
            ),
            (
                "icons/close".to_owned(),
                svg(include_bytes!("../../../registry/assets/icons/close.svg")),
            ),
            (
                "icons/check".to_owned(),
                svg(include_bytes!("../../../registry/assets/icons/check.svg")),
            ),
        ])
        .calendar_clock(CalendarClock::fixed(
            GregorianDate::parse_iso("2026-08-30").expect("fixed visual date"),
        ))
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(760.0, 720.0)
                .run()
        })
        .expect("form_showcase failed");
}

fn visual_date_picker_source(state: &str) -> String {
    if state == "date-picker" {
        DATE_PICKER.replace(
            "open: #{ schema: #{ type: \"bool\" }, \"default\": #{ type: \"bool\", value: false } },",
            "open: #{ schema: #{ type: \"bool\" }, \"default\": #{ type: \"bool\", value: true } },",
        )
    } else {
        DATE_PICKER.to_owned()
    }
}

fn visual_source(theme: &str, locale: &str, state: &str) -> String {
    let populated = state != "default";
    MAIN.replace("__VISUAL_THEME__", theme)
        .replace("__VISUAL_LOCALE__", locale)
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
            if state == "dialog" { "true" } else { "false" },
        )
        .replace(
            "__VISUAL_TOAST__",
            if state == "toast" { "true" } else { "false" },
        )
        .replace(
            "__TOAST_DURATION__",
            if state == "toast" { "60000" } else { "2200" },
        )
        .replace(
            "__VISUAL_COUNTRY__",
            if state == "date-picker" {
                "#{ type: \"string\", value: \"cn\" }"
            } else {
                "#{ type: \"null\" }"
            },
        )
        .replace(
            "__VISUAL_APPOINTMENT__",
            if state == "date-picker" {
                "#{ type: \"string\", value: \"2026-09-01\" }"
            } else {
                "#{ type: \"null\" }"
            },
        )
        .replace(
            "__VISUAL_NOTES__",
            if state == "textarea" {
                "A multiline note used for deterministic Textarea visual coverage."
            } else {
                ""
            },
        )
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

fn svg(bytes: &[u8]) -> AssetData {
    AssetData {
        mime_type: "image/svg+xml".to_owned(),
        bytes: bytes.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_picker_visual_state_opens_the_real_component_panel() {
        let source = visual_date_picker_source("date-picker");
        assert!(source.contains(
            "open: #{ schema: #{ type: \"bool\" }, \"default\": #{ type: \"bool\", value: true } },"
        ));
        assert_eq!(
            source.matches("value: true } },").count(),
            DATE_PICKER.matches("value: true } },").count() + 1
        );
    }
}
