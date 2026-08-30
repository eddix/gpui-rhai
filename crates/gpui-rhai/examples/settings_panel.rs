use std::collections::BTreeMap;

use gpui_rhai::{AssetData, EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const LABEL: &str = include_str!("../../../registry/components/label.rhai");
const DIVIDER: &str = include_str!("../../../registry/components/divider.rhai");
const DROPDOWN: &str = include_str!("../../../registry/components/dropdown.rhai");
const POPOVER: &str = include_str!("../../../registry/components/popover.rhai");
const SWITCH: &str = include_str!("../../../registry/components/switch.rhai");
const RADIO: &str = include_str!("../../../registry/components/radio.rhai");
const RADIO_GROUP: &str = include_str!("../../../registry/components/radio_group.rhai");
const ACCORDION: &str = include_str!("../../../registry/components/accordion.rhai");
const DEFAULT_DARK: &str = include_str!("../../../registry/themes/default_dark.rhai");
const DEFAULT_LIGHT: &str = include_str!("../../../registry/themes/default_light.rhai");
const TOKYO_NIGHT: &str = include_str!("../../../registry/themes/tokyo_night.rhai");
const TOKYO_STORM: &str = include_str!("../../../registry/themes/tokyo_storm.rhai");
const CATPPUCCIN_LATTE: &str = include_str!("../../../registry/themes/catppuccin_latte.rhai");
const CATPPUCCIN_MOCHA: &str = include_str!("../../../registry/themes/catppuccin_mocha.rhai");
const EN: &str = include_str!("../../../registry/locales/en.rhai");
const ZH_CN: &str = include_str!("../../../registry/locales/zh_cn.rhai");
const AR: &str = include_str!("../../../registry/locales/ar.rhai");

const MAIN: &str = r#"
import "components/button" as button;
import "components/label" as label;
import "components/divider" as divider;
import "components/dropdown" as dropdown_component;
import "components/popover" as popover;
import "components/switch" as switch_component;
import "components/radio_group" as radio_group;
import "components/accordion" as accordion;

fn state_schema() {
    #{
        fields: #{
            theme: #{
                schema: #{ type: "array", max_items: 1, items: #{ type: "string" } },
                "default": #{
                    type: "array",
                    value: [#{ type: "string", value: "__VISUAL_THEME__" }]
                },
            },
            theme_open: #{
                schema: #{ type: "bool" },
                "default": #{ type: "bool", value: false },
            },
            theme_query: #{
                schema: #{ type: "string" },
                "default": #{ type: "string", value: "" },
            },
            help_open: #{
                schema: #{ type: "bool" },
                "default": #{ type: "bool", value: false },
            },
            locale: #{
                schema: #{ type: "string", allowed: ["en", "zh-CN", "ar"] },
                "default": #{ type: "string", value: "__VISUAL_LOCALE__" },
            },
            notifications: #{
                schema: #{ type: "bool" },
                "default": #{ type: "bool", value: true },
            },
            density: #{
                schema: #{ type: "string", allowed: ["comfortable", "compact"] },
                "default": #{ type: "string", value: "comfortable" },
            },
            expanded: #{
                schema: #{ type: "array", max_items: 8, items: #{ type: "string" } },
                "default": #{ type: "array", value: [] },
            },
        },
    }
}

fn choose_theme(ctx, values) {
    ctx.set_state("theme", values);
    ctx.set_state("theme_open", false);
    if values.len == 0 { return; }
    let choice = values[0];
    if choice == "default-light" { ctx.set_theme("Default", "Light"); }
    else if choice == "default-dark" { ctx.set_theme("Default", "Dark"); }
    else if choice == "tokyo-night" { ctx.set_theme("Tokyo Night", "Night"); }
    else if choice == "tokyo-storm" { ctx.set_theme("Tokyo Night", "Storm"); }
    else if choice == "catppuccin-latte" { ctx.set_theme("Catppuccin", "Latte"); }
    else if choice == "catppuccin-mocha" { ctx.set_theme("Catppuccin", "Mocha"); }
}

fn set_theme_open(ctx, open) { ctx.set_state("theme_open", open); }
fn set_theme_query(ctx, query) { ctx.set_state("theme_query", query); }
fn set_help_open(ctx, open) { ctx.set_state("help_open", open); }
fn reset_theme(ctx, payload) { choose_theme(ctx, ["default-dark"]); }
fn use_english(ctx, payload) {
    ctx.set_locale("en");
    ctx.set_state("locale", "en");
}
fn use_chinese(ctx, payload) {
    ctx.set_locale("zh-CN");
    ctx.set_state("locale", "zh-CN");
}
fn set_notifications(ctx, checked) { ctx.set_state("notifications", checked); }
fn set_density(ctx, value) { ctx.set_state("density", value); }
fn set_expanded(ctx, values) { ctx.set_state("expanded", values); }

fn init(ctx) {
    choose_theme(ctx, ["__VISUAL_THEME__"]);
    ctx.set_locale("__VISUAL_LOCALE__");
}

fn view(ctx) {
    let help_opacity = if ctx.get_state("help_open") { 1.0 } else { 0.65 };
    column([
        row([
            label::Label(#{
                text: "Settings",
                description: "Source-owned components backed by native GPUI mechanisms"
            }),
            popover::Popover(#{
                key: "settings-help",
                trigger: text("?").with_style(
                    style()
                        .width(px(28))
                        .height(px(28))
                        .padding(px(6))
                        .radius(px(14))
                        .background(theme_color("surface_raised"))
                )
                    .with_key("help-trigger")
                    .animate(transition("opacity", 0.65, help_opacity, 180, "ease_out")),
                content: text("Theme changes preserve keyed state and the compiled Rhai AST."),
                open: ctx.get_state("help_open"),
                placement: "left",
                on_open_change: Fn("set_help_open")
            })
        ]).with_style(style().justify_between().items_center()),
        divider::Divider(#{}),
        label::Label(#{
            text: "Color theme",
            description: "Search or use arrows, Enter, Escape, Home and End"
        }),
        dropdown_component::Dropdown(#{
            key: "theme-picker",
            options: [
                #{ value: "default-light", label: "Default Light", keywords: ["light"] },
                #{ value: "default-dark", label: "Default Dark", keywords: ["dark"] },
                #{ value: "tokyo-night", label: "Tokyo Night", keywords: ["dark", "blue"] },
                #{ value: "tokyo-storm", label: "Tokyo Storm", keywords: ["dark", "blue"] },
                #{ value: "catppuccin-latte", label: "Catppuccin Latte", keywords: ["light"] },
                #{ value: "catppuccin-mocha", label: "Catppuccin Mocha", keywords: ["dark"] }
            ],
            mode: "single",
            selected: ctx.get_state("theme"),
            open: ctx.get_state("theme_open"),
            searchable: true,
            query: ctx.get_state("theme_query"),
            placeholder: "Choose a theme",
            search_placeholder: "Search themes",
            empty_text: ctx.t("common.no_results"),
            trigger: row([
                text("Theme palette"),
                text("⌄").with_style(style().text_color(theme_color("text_muted")))
            ]).with_style(
                style()
                    .width(px(280))
                    .height(px(32))
                    .padding_x(px(8))
                    .justify_between()
                    .items_center()
                    .radius(px(6))
                    .border(px(1))
                    .border_color(theme_color("border"))
                    .background(theme_color("surface_raised"))
            ),
            header: text("Available themes")
                .with_style(style().padding(px(6)).text_color(theme_color("text_muted"))),
            footer: text("Selection is stored by stable value")
                .with_style(style().padding(px(6)).text_color(theme_color("text_muted"))),
            empty: text("No theme matches this query")
                .with_style(style().padding(px(8)).text_color(theme_color("warning"))),
            max_visible: 6,
            on_change: Fn("choose_theme"),
            on_open_change: Fn("set_theme_open"),
            on_query_change: Fn("set_theme_query")
        }),
        divider::Divider(#{}),
        label::Label(#{
            text: "Language / 语言",
            description: ctx.t("common.loading")
        }),
        row([
            button::Button(#{
                text: "English",
                variant: "secondary",
                size: "sm",
                on_click: Fn("use_english")
            }),
            button::Button(#{
                text: "简体中文",
                variant: "secondary",
                size: "sm",
                on_click: Fn("use_chinese")
            })
        ]).with_style(style().gap(px(8))),
        divider::Divider(#{}),
        switch_component::Switch(#{
            checked: ctx.get_state("notifications"),
            label: "Notifications",
            on_change: Fn("set_notifications")
        }),
        radio_group::RadioGroup(#{
            value: ctx.get_state("density"),
            label: "Interface density",
            orientation: "horizontal",
            options: [
                #{ value: "comfortable", label: "Comfortable" },
                #{ value: "compact", label: "Compact" }
            ],
            on_change: Fn("set_density")
        }),
        accordion::Accordion(#{
            key: "advanced-settings",
            expanded: ctx.get_state("expanded"),
            mode: "multiple",
            items: [
                #{
                    key: "behavior", title: "Behavior", content_height: 44,
                    content: text("Notifications and density remain keyed Rhai state.")
                },
                #{
                    key: "appearance", title: "Appearance", content_height: 44,
                    content: text("Theme changes do not recompile scripts.")
                }
            ],
            on_change: Fn("set_expanded")
        }),
        divider::Divider(#{}),
        row([
            text("Changes apply immediately without recompilation.")
                .with_style(style().text_color(theme_color("text_muted"))),
            button::Button(#{
                text: "Reset",
                variant: "secondary",
                size: "sm",
                on_click: Fn("reset_theme")
            })
        ]).with_style(style().justify_between().items_center())
    ]).with_style(
        style()
            .width(px(560))
            .padding(px(24))
            .gap(px(16))
            .background(theme_color("surface"))
    )
}
"#;

fn module(name: &str, source: &str) -> (ModuleId, String) {
    (
        ModuleId::parse(name).expect("static example module ID"),
        source.to_owned(),
    )
}

fn main() {
    let visual_theme = std::env::var("GPUI_RHAI_VISUAL_THEME")
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
        .unwrap_or_else(|| "default-dark".to_owned());
    let visual_locale = std::env::var("GPUI_RHAI_VISUAL_LOCALE")
        .ok()
        .filter(|locale| matches!(locale.as_str(), "en" | "zh-CN" | "ar"))
        .unwrap_or_else(|| "en".to_owned());
    let main_source = MAIN
        .replace("__VISUAL_THEME__", &visual_theme)
        .replace("__VISUAL_LOCALE__", &visual_locale);
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        module("main", &main_source),
        module("components/button", BUTTON),
        module("components/label", LABEL),
        module("components/divider", DIVIDER),
        module("components/dropdown", DROPDOWN),
        module("components/popover", POPOVER),
        module("components/switch", SWITCH),
        module("components/radio", RADIO),
        module("components/radio_group", RADIO_GROUP),
        module("components/accordion", ACCORDION),
    ]));
    EmbeddedScriptView::new(
        ModuleId::parse("main").expect("main module"),
        scripts,
        DEFAULT_DARK,
    )
    .theme_sources([
        ("default_light.rhai".to_owned(), DEFAULT_LIGHT.to_owned()),
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
    .asset_sources(choice_assets())
    .development(true)
    .prepare()
    .and_then(|prepared| {
        ScriptApplication::new(prepared)
            .window_size(640.0, 520.0)
            .run()
    })
    .expect("settings_panel failed");
}

fn choice_assets() -> [(String, AssetData); 3] {
    [
        (
            "icons/check".to_owned(),
            svg(include_bytes!("../../../registry/assets/icons/check.svg")),
        ),
        (
            "icons/close".to_owned(),
            svg(include_bytes!("../../../registry/assets/icons/close.svg")),
        ),
        (
            "icons/disclosure_down".to_owned(),
            svg(include_bytes!(
                "../../../registry/assets/icons/disclosure_down.svg"
            )),
        ),
    ]
}

fn svg(bytes: &[u8]) -> AssetData {
    AssetData {
        mime_type: "image/svg+xml".to_owned(),
        bytes: bytes.to_vec(),
    }
}
