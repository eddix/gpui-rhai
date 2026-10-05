use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const REGION: &str = include_str!("../../../registry/layouts/region.rhai");
const STACK: &str = include_str!("../../../registry/layouts/stack.rhai");
const FORM_LAYOUT: &str = include_str!("../../../registry/patterns/form_layout.rhai");
const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const ICON_BUTTON: &str = include_str!("../../../registry/components/icon_button.rhai");
const ICON: &str = include_str!("../../../registry/components/icon.rhai");
const TOGGLE_GROUP: &str = include_str!("../../../registry/components/toggle_group.rhai");
const LABEL: &str = include_str!("../../../registry/components/label.rhai");
const INPUT: &str = include_str!("../../../registry/components/input.rhai");
const COMBOBOX: &str = include_str!("../../../registry/components/combobox.rhai");
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
const ETHEREAL: &str = include_str!("../../../registry/themes/ethereal.rhai");
const EVERFOREST: &str = include_str!("../../../registry/themes/everforest.rhai");
const GRUVBOX: &str = include_str!("../../../registry/themes/gruvbox.rhai");
const HACKERMAN: &str = include_str!("../../../registry/themes/hackerman.rhai");
const NORD: &str = include_str!("../../../registry/themes/nord.rhai");
const RETRO_82: &str = include_str!("../../../registry/themes/retro_82.rhai");
const HERMARCHY: &str = include_str!("../../../registry/themes/hermarchy.rhai");
const FUTURISM: &str = include_str!("../../../registry/themes/futurism.rhai");
const AETHERIA: &str = include_str!("../../../registry/themes/aetheria.rhai");
const EN: &str = include_str!("../../../registry/locales/en.rhai");
const ZH_CN: &str = include_str!("../../../registry/locales/zh_cn.rhai");
const AR: &str = include_str!("../../../registry/locales/ar.rhai");

const MAIN: &str = r#"
import "layouts/region" as region;
import "layouts/stack" as stack;
import "patterns/form_layout" as form_layout;
import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/icon" as icon;
import "components/combobox" as combobox_component;
import "components/popover" as popover;
import "components/switch" as switch_component;
import "components/radio_group" as radio_group;
import "components/toggle_group" as toggle_group;
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
    else if choice == "ethereal" { ctx.set_theme("Ethereal", "Dark"); }
    else if choice == "everforest" { ctx.set_theme("Everforest", "Dark"); }
    else if choice == "gruvbox" { ctx.set_theme("Gruvbox", "Dark"); }
    else if choice == "hackerman" { ctx.set_theme("Hackerman", "Dark"); }
    else if choice == "nord" { ctx.set_theme("Nord", "Dark"); }
    else if choice == "retro-82" { ctx.set_theme("Retro 82", "Dark"); }
    else if choice == "hermarchy" { ctx.set_theme("Hermarchy", "Dark"); }
    else if choice == "futurism" { ctx.set_theme("Futurism", "Dark"); }
    else if choice == "aetheria" { ctx.set_theme("Aetheria", "Dark"); }
}

fn set_theme_open(ctx, open) { ctx.set_state("theme_open", open); }
fn set_theme_query(ctx, query) { ctx.set_state("theme_query", query); }
fn set_help_open(ctx, open) { ctx.set_state("help_open", open); }
fn open_help(ctx, payload) { ctx.set_state("help_open", true); }
fn reset_theme(ctx, payload) { choose_theme(ctx, ["default-dark"]); }
fn set_language(ctx, values) {
    if values.len == 0 { return; }
    ctx.set_locale(values[0]);
    ctx.set_state("locale", values[0]);
}
fn set_notifications(ctx, checked) { ctx.set_state("notifications", checked); }
fn set_density(ctx, value) { ctx.set_state("density", value); }
fn set_expanded(ctx, values) { ctx.set_state("expanded", values); }

fn init(ctx) {
    choose_theme(ctx, ["__VISUAL_THEME__"]);
    ctx.set_locale("__VISUAL_LOCALE__");
}

fn theme_options() {
    [
        #{ value: "default-light", label: "Default Light", keywords: ["light"] },
        #{ value: "default-dark", label: "Default Dark", keywords: ["dark"] },
        #{ value: "tokyo-night", label: "Tokyo Night", keywords: ["dark", "blue"] },
        #{ value: "tokyo-storm", label: "Tokyo Storm", keywords: ["dark", "blue"] },
        #{ value: "catppuccin-latte", label: "Catppuccin Latte", keywords: ["light"] },
        #{ value: "catppuccin-mocha", label: "Catppuccin Mocha", keywords: ["dark"] },
        #{ value: "ethereal", label: "Ethereal", keywords: ["dark", "blue"] },
        #{ value: "everforest", label: "Everforest", keywords: ["dark", "green"] },
        #{ value: "gruvbox", label: "Gruvbox", keywords: ["dark", "warm"] },
        #{ value: "hackerman", label: "Hackerman", keywords: ["dark", "green"] },
        #{ value: "nord", label: "Nord", keywords: ["dark", "blue"] },
        #{ value: "retro-82", label: "Retro 82", keywords: ["dark", "retro"] },
        #{ value: "hermarchy", label: "Hermarchy", keywords: ["dark", "cyan"] },
        #{ value: "futurism", label: "Futurism", keywords: ["dark", "magenta"] },
        #{ value: "aetheria", label: "Aetheria", keywords: ["dark", "teal"] }
    ]
}

fn help(ctx) {
    popover::Popover(#{
        key: "settings-help", label: "Settings help", placement: "bottom", align: "end",
        trigger: icon_button::IconButton(#{
            key: "help-trigger", label: "Help", variant: "ghost",
            icon: icon::Icon(#{ source: asset("app/icons/help") }), on_click: Fn("open_help")
        }),
        content: text("Theme changes keep component state and the compiled Rhai AST."),
        open: ctx.get_state("help_open"), on_open_change: Fn("set_help_open")
    })
}

fn appearance(ctx) {
    #{ title: "Appearance", fields: [
        #{ label: "Color theme", description: "Search, or use the arrow keys, Enter and Escape.",
            control: combobox_component::Combobox(#{
                key: "theme-picker", label: "Color theme", options: theme_options(),
                mode: "single", selected: ctx.get_state("theme"), open: ctx.get_state("theme_open"),
                searchable: true, query: ctx.get_state("theme_query"), width: px(240),
                placeholder: "Choose a theme", search_placeholder: "Search themes",
                empty_text: ctx.t("common.no_results"), max_visible: 6,
                on_change: Fn("choose_theme"), on_open_change: Fn("set_theme_open"),
                on_query_change: Fn("set_theme_query")
            }) },
        #{ label: "Language / 语言", control: toggle_group::ToggleGroup(#{
            key: "language", label: "Language", values: [ctx.get_state("locale")],
            items: [#{ value: "en", label: "English" }, #{ value: "zh-CN", label: "简体中文" }],
            allow_empty: false, on_change: Fn("set_language")
        }) },
        #{ label: "Density", control: radio_group::RadioGroup(#{
            value: ctx.get_state("density"), label: "Interface density", orientation: "horizontal",
            options: [#{ value: "comfortable", label: "Comfortable" }, #{ value: "compact", label: "Compact" }],
            on_change: Fn("set_density")
        }) },
    ] }
}

fn notifications(ctx) {
    #{ title: "Notifications", fields: [
        #{ label: "Deploys", control: switch_component::Switch(#{
            checked: ctx.get_state("notifications"), label: "Notify when a deploy finishes",
            on_change: Fn("set_notifications")
        }) },
    ] }
}

fn advanced(ctx) {
    accordion::Accordion(#{
        key: "advanced-settings", expanded: ctx.get_state("expanded"), mode: "multiple",
        items: [
            #{ key: "behavior", title: "Behavior", content_height: 44,
                content: text("Notifications and density are keyed Rhai state.") },
            #{ key: "appearance", title: "Appearance", content_height: 44,
                content: text("Theme changes do not recompile scripts.") }
        ],
        on_change: Fn("set_expanded")
    })
}

fn view(ctx) {
    let form = form_layout::FormLayout(#{
        key: "settings-form", label: "Preferences", groups: [appearance(ctx), notifications(ctx)],
        submit: [button::Button(#{ key: "reset", text: "Reset theme", on_click: Fn("reset_theme") })]
    });
    let body = column([stack::Stack(#{ gap: "section", children: [form, advanced(ctx)] })])
        .with_style(style().flex_col().flex_grow().min_height(px(0)).overflow_y_scroll());
    // The region fills the window, so its footer sits at the bottom.
    column([region::Region(#{
        label: "Settings", title: "Settings", actions: [help(ctx)], body: body,
        footer: [text("Changes apply immediately, without recompiling.")]
    })]).with_style(style().flex_col().width(relative(1.0)).height(relative(1.0)))
        .env(#{ density: ctx.get_state("density") })
}
"#;

fn module(name: &str, source: &str) -> (ModuleId, String) {
    (
        ModuleId::parse(name).expect("static example module ID"),
        source.to_owned(),
    )
}

/// Logical window size of the example.
pub const WINDOW: (f32, f32) = (640.0, 520.0);

const THEMES: &[&str] = &[
    "default-light",
    "default-dark",
    "tokyo-night",
    "tokyo-storm",
    "catppuccin-latte",
    "catppuccin-mocha",
    "ethereal",
    "everforest",
    "gruvbox",
    "hackerman",
    "nord",
    "retro-82",
    "hermarchy",
    "futurism",
    "aetheria",
];

/// Assemble the settings panel for one theme slug and locale.
///
/// # Panics
///
/// Panics only if a static module ID is invalid.
pub fn view(theme: &str, locale: &str) -> EmbeddedScriptView {
    let theme = if THEMES.contains(&theme) {
        theme
    } else {
        "default-dark"
    };
    let locale = if matches!(locale, "en" | "zh-CN" | "ar") {
        locale
    } else {
        "en"
    };
    let main_source = MAIN
        .replace("__VISUAL_THEME__", theme)
        .replace("__VISUAL_LOCALE__", locale);
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        module("main", &main_source),
        module("layouts/region", REGION),
        module("layouts/stack", STACK),
        module("patterns/form_layout", FORM_LAYOUT),
        module("components/button", BUTTON),
        module("components/icon_button", ICON_BUTTON),
        module("components/icon", ICON),
        module("components/label", LABEL),
        module("components/input", INPUT),
        module("components/combobox", COMBOBOX),
        module("components/popover", POPOVER),
        module("components/switch", SWITCH),
        module("components/radio", RADIO),
        module("components/radio_group", RADIO_GROUP),
        module("components/toggle_group", TOGGLE_GROUP),
        module("components/accordion", ACCORDION),
    ]));
    EmbeddedScriptView::new(
        ModuleId::parse("main").expect("main module"),
        scripts,
        DEFAULT_DARK,
    )
    .token_base(include_str!("../../../registry/tokens.rhai"))
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
        ("ethereal.rhai".to_owned(), ETHEREAL.to_owned()),
        ("everforest.rhai".to_owned(), EVERFOREST.to_owned()),
        ("gruvbox.rhai".to_owned(), GRUVBOX.to_owned()),
        ("hackerman.rhai".to_owned(), HACKERMAN.to_owned()),
        ("nord.rhai".to_owned(), NORD.to_owned()),
        ("retro_82.rhai".to_owned(), RETRO_82.to_owned()),
        ("hermarchy.rhai".to_owned(), HERMARCHY.to_owned()),
        ("futurism.rhai".to_owned(), FUTURISM.to_owned()),
        ("aetheria.rhai".to_owned(), AETHERIA.to_owned()),
    ])
    .locale_sources([
        ("en.rhai".to_owned(), EN.to_owned()),
        ("zh_cn.rhai".to_owned(), ZH_CN.to_owned()),
        ("ar.rhai".to_owned(), AR.to_owned()),
    ])
    .asset_sources(registry_icons())
}

#[allow(dead_code)]
fn main() {
    let theme = std::env::var("GPUI_RHAI_VISUAL_THEME").unwrap_or_default();
    let locale = std::env::var("GPUI_RHAI_VISUAL_LOCALE").unwrap_or_default();
    view(&theme, &locale)
        .development(true)
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(WINDOW.0, WINDOW.1)
                .run()
        })
        .expect("settings_panel failed");
}

include!("support/icons.rs");
