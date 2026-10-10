use std::collections::BTreeMap;

use gpui_rhai::{
    CalendarClock, EmbeddedScriptSource, EmbeddedScriptView, GregorianDate, ModuleId,
    ScriptApplication,
};

const REGION: &str = include_str!("../../../registry/layouts/region.rhai");
const STACK: &str = include_str!("../../../registry/layouts/stack.rhai");
const FORM_LAYOUT: &str = include_str!("../../../registry/patterns/form_layout.rhai");
const DESCRIPTION_LIST: &str = include_str!("../../../registry/patterns/description_list.rhai");
const TOGGLE_GROUP: &str = include_str!("../../../registry/components/toggle_group.rhai");
const INPUT: &str = include_str!("../../../registry/components/input.rhai");
const TEXTAREA: &str = include_str!("../../../registry/components/textarea.rhai");
const SELECT: &str = include_str!("../../../registry/components/select.rhai");
const COMBOBOX: &str = include_str!("../../../registry/components/combobox.rhai");
const DATE_PICKER: &str = include_str!("../../../registry/components/date_picker.rhai");
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
import "layouts/region" as region;
import "layouts/stack" as stack;
import "patterns/form_layout" as form_layout;
import "patterns/description_list" as description_list;
import "components/input" as input;
import "components/textarea" as textarea;
import "components/select" as select;
import "components/date_picker" as date_picker;
import "components/checkbox" as checkbox;
import "components/radio_group" as radio_group;
import "components/switch" as switch_component;
import "components/toggle_group" as toggle_group;
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
        locale: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "__VISUAL_LOCALE__" } },
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
fn set_language(ctx, values) {
    if values.len == 0 { return; }
    ctx.set_locale(values[0]);
    ctx.set_state("locale", values[0]);
}
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

fn heading() {
    column([
        text("Profile").with_style(style().typography("title").text_color(theme_color("text_primary")))
            .accessibility_role("heading").accessibility_level(1),
        text("Fields validate as you type; Review confirms the profile.").with_style(style()
            .typography("caption").text_color(theme_color("text_muted"))),
    ]).with_style(style().flex_col().gap(theme_spacing("xxs")).min_width(px(0)))
}

fn language(ctx) {
    toggle_group::ToggleGroup(#{
        key: "language", label: "Language", size: "sm", values: [ctx.get_state("locale")],
        items: [#{ value: "en", label: "English" }, #{ value: "zh-CN", label: "简体中文" }],
        allow_empty: false, on_change: Fn("set_language")
    })
}

fn account(ctx, name, error) {
    #{ title: "Account", fields: [
        #{ label: "Name", required: true, description: "Shown to collaborators.", error: error,
            control: input::Input(#{
                key: "name", label: "Name", value: name, placeholder: "Ada Lovelace",
                error: error != (), on_change: Fn("set_name")
            }).with_style(style().width(px(280))) },
        #{ label: "Plan", control: radio_group::RadioGroup(#{
            value: ctx.get_state("plan"), label: "Plan", orientation: "horizontal",
            options: [#{ value: "personal", label: "Personal" }, #{ value: "team", label: "Team" }],
            on_change: Fn("set_plan")
        }) },
        #{ label: "Terms", control: checkbox::Checkbox(#{
            checked: ctx.get_state("accepted"), label: "I accept the project terms",
            on_change: Fn("set_accepted")
        }) },
        #{ label: "Updates", control: switch_component::Switch(#{
            checked: ctx.get_state("updates"), label: "Product updates by email",
            on_change: Fn("set_updates")
        }) },
    ] }
}

fn details(ctx) {
    #{ title: "Details", fields: [
        #{ label: "Country or region", control: select::Select(#{
            key: "country", label: "Country or region", value: ctx.get_state("country"),
            open: ctx.get_state("country_open"), query: ctx.get_state("country_query"),
            searchable: true, clearable: true, placeholder: "Choose a country", width: px(280),
            options: [
                #{ value: "cn", label: "China", group: "Asia", keywords: ["zhongguo"] },
                #{ value: "jp", label: "Japan", group: "Asia" },
                #{ value: "fr", label: "France", group: "Europe" },
                #{ value: "de", label: "Germany", group: "Europe" }
            ], on_change: Fn("set_country"), on_open_change: Fn("set_country_open"),
            on_query_change: Fn("set_country_query")
        }) },
        #{ label: "Appointment", control: date_picker::DatePicker(#{
            key: "appointment", label: "Appointment date", value: ctx.get_state("appointment"),
            min_date: "2026-08-30", max_date: "2026-12-31", clearable: true,
            placeholder: "Choose a date", width: px(280),
            presets: [#{ label: "Launch", value: "2026-09-01" }, #{ label: "Review", value: "2026-10-15" }],
            on_change: Fn("set_appointment")
        }) },
        #{ label: "Notes", error: if ctx.get_state("notes") == "" { "Add a note for the reviewer." } else { () },
            control: textarea::Textarea(#{
                key: "notes", label: "Notes", value: ctx.get_state("notes"),
                placeholder: "Add feedback or context", min_rows: 3, max_rows: 6,
                max_length: 240, show_count: true,
                error: ctx.get_state("notes") == "", on_change: Fn("set_notes")
            }).with_style(style().width(px(360))) },
        #{ label: "Fixed note", control: textarea::Textarea(#{
            key: "fixed-note", label: "Fixed note", value: ctx.get_state("fixed_note"),
            rows: 2, read_only: true
        }).with_style(style().width(px(360))) },
        #{ label: "Limited note", control: textarea::Textarea(#{
            key: "limit-note", label: "Limited note", value: ctx.get_state("limit_note"),
            min_rows: 1, max_rows: 2, max_length: 10, show_count: true,
            on_change: Fn("set_limit_note")
        }).with_style(style().width(px(360))) },
    ] }
}

fn review(ctx, name) {
    dialog::Dialog(#{
        key: "review", open: ctx.get_state("dialog_open"), title: "Review profile",
        content: description_list::DescriptionList(#{ label: "Profile", items: [
            #{ label: "Name", value: name }, #{ label: "Plan", value: ctx.get_state("plan") },
        ] }),
        actions: [
            button::Button(#{ key: "review-cancel", text: "Cancel", variant: "ghost", on_click: Fn("close_dialog") }),
            button::Button(#{ key: "review-confirm", text: "Confirm", variant: "primary", on_click: Fn("confirm") })
        ],
        on_open_change: Fn("set_dialog")
    })
}

fn view(ctx) {
    let name = ctx.get_state("name");
    let error = if name == "" { "Enter a name." } else { () };
    let toasts = [];
    if ctx.get_state("toast_visible") {
        toasts.push(#{
            id: "saved", title: "Profile saved",
            message: `Saved ${name} on the ${ctx.get_state("plan")} plan.`,
            variant: "success", region: "bottom_right", duration_ms: __TOAST_DURATION__,
        });
    }
    let form = form_layout::FormLayout(#{
        key: "profile-form", label: "Profile", groups: [account(ctx, name, error), details(ctx)],
        submit: [button::Button(#{ key: "review", text: "Review", variant: "primary", on_click: Fn("open_dialog") })]
    });
    let body = column([form, review(ctx, name),
        toast::Toast(#{ key: "form-toasts", items: toasts, on_dismiss: Fn("dismiss_toast") })])
        .with_style(style().flex_col().flex_grow().min_height(px(0)).overflow_y_scroll());
    column([region::Region(#{ label: "Profile", title: heading(), actions: [language(ctx)], body: body })])
        .with_style(style().flex_col().width(relative(1.0)).height(relative(1.0)))
}
"#;

fn module(id: &str, source: &str) -> (ModuleId, String) {
    (
        ModuleId::parse(id).expect("static module ID"),
        source.to_owned(),
    )
}

/// Logical window size of the example.
pub const WINDOW: (f32, f32) = (760.0, 720.0);

/// Assemble the form for one theme slug, locale and visual state
/// (`default`, `dialog`, `toast`, `date-picker` or `textarea`).
///
/// # Panics
///
/// Panics only if a static module ID or the fixed visual date is invalid.
pub fn view(theme: &str, locale: &str, state: &str) -> EmbeddedScriptView {
    let theme = if matches!(
        theme,
        "default-light"
            | "default-dark"
            | "tokyo-night"
            | "tokyo-storm"
            | "catppuccin-latte"
            | "catppuccin-mocha"
    ) {
        theme
    } else {
        "default-light"
    };
    let locale = if matches!(locale, "en" | "zh-CN" | "ar") {
        locale
    } else {
        "en"
    };
    let state = if matches!(
        state,
        "default" | "dialog" | "toast" | "date-picker" | "textarea"
    ) {
        state
    } else {
        "default"
    };
    let main_source = visual_source(theme, locale, state);
    let date_picker_source = visual_date_picker_source(state);
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        module("main", &main_source),
        module("layouts/region", REGION),
        module("layouts/stack", STACK),
        module("patterns/form_layout", FORM_LAYOUT),
        module("patterns/description_list", DESCRIPTION_LIST),
        module("components/input", INPUT),
        module("components/textarea", TEXTAREA),
        module("components/select", SELECT),
        module("components/combobox", COMBOBOX),
        module("components/date_picker", &date_picker_source),
        module("components/checkbox", CHECKBOX),
        module("components/radio", RADIO),
        module("components/radio_group", RADIO_GROUP),
        module("components/switch", SWITCH),
        module("components/toggle_group", TOGGLE_GROUP),
        module("components/button", BUTTON),
        module("components/dialog", DIALOG),
        module("components/toast", TOAST),
    ]));
    EmbeddedScriptView::new(ModuleId::parse("main").unwrap(), scripts, DEFAULT_LIGHT)
        .token_base(include_str!("../../../registry/tokens.rhai"))
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
        .asset_sources(registry_icons())
        .calendar_clock(CalendarClock::fixed(
            GregorianDate::parse_iso("2026-08-30").expect("fixed visual date"),
        ))
}

#[allow(dead_code)]
fn main() {
    let theme = std::env::var("GPUI_RHAI_VISUAL_THEME").unwrap_or_default();
    let locale = std::env::var("GPUI_RHAI_VISUAL_LOCALE").unwrap_or_default();
    let state = std::env::var("GPUI_RHAI_VISUAL_STATE").unwrap_or_default();
    view(&theme, &locale, &state)
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(WINDOW.0, WINDOW.1)
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

include!("support/icons.rs");

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
