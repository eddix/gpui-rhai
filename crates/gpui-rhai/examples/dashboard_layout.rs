use std::collections::BTreeMap;

use gpui_rhai::{AssetData, EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const TABS: &str = include_str!("../../../registry/components/tabs.rhai");
const TAG: &str = include_str!("../../../registry/components/tag.rhai");
const AVATAR: &str = include_str!("../../../registry/components/avatar.rhai");
const PROGRESS: &str = include_str!("../../../registry/components/progress.rhai");
const POPOVER: &str = include_str!("../../../registry/components/popover.rhai");
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
import "components/tabs" as tabs;
import "components/tag" as tag;
import "components/avatar" as avatar;
import "components/progress" as progress;
import "components/popover" as popover;

fn state_schema() {
    #{ fields: #{
        tab: #{ schema: #{ type: "string", allowed: ["overview", "activity"] },
            "default": #{ type: "string", value: "overview" } },
        profile_open: #{ schema: #{ type: "bool" },
            "default": #{ type: "bool", value: false } },
    } }
}

fn set_tab(ctx, value) { ctx.set_state("tab", value); }
fn set_profile_open(ctx, open) { ctx.set_state("profile_open", open); }
fn ignore(ctx, value) { () }

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

fn overview() {
    column([
        row([
            tag::Tag(#{ text: "Production", variant: "success" }),
            tag::Tag(#{ text: "Rust", variant: "accent", closable: true, on_close: Fn("ignore") })
        ]).with_style(style().gap(px(8))),
        text("Deployment progress"),
        progress::Progress(#{ key: "deployment", value: 72, max: 100, label: "Deployment" }),
        text("Background synchronization"),
        progress::Progress(#{ key: "sync", indeterminate: true, label: "Synchronization" })
    ]).with_style(style().padding(px(16)).gap(px(12)).items_start())
}

fn activity() {
    column([
        text("09:42  Release candidate built"),
        text("09:44  Integration checks passed"),
        text("09:47  Deployment started")
    ]).with_style(style().padding(px(16)).gap(px(8)).items_start())
}

fn view(ctx) {
    let selected = ctx.get_state("tab");
    column([
        row([
            column([
                text("Runtime dashboard").with_style(style().font_size(rem(1.25))),
                text("Source-owned Rhai components").with_style(style().text_color(theme_color("text_muted")))
            ]).with_style(style().gap(px(4))),
            popover::Popover(#{
                key: "profile",
                trigger: avatar::Avatar(#{ name: "Ada Lovelace", initials: "AL", presence: "online" }),
                content: column([
                    text("Ada Lovelace"),
                    text("Runtime maintainer").with_style(style().text_color(theme_color("text_muted")))
                ]).with_style(style().gap(px(6))),
                open: ctx.get_state("profile_open"),
                placement: "left",
                on_open_change: Fn("set_profile_open")
            })
        ]).with_style(style().justify_between().items_center()),
        tabs::Tabs(#{
            value: selected,
            label: "Dashboard sections",
            tabs: [
                #{ value: "overview", label: "Overview", content: overview() },
                #{ value: "activity", label: "Activity", content: activity() }
            ],
            on_change: Fn("set_tab")
        })
    ]).with_style(
        style().width(px(680)).padding(px(24)).gap(px(18))
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
    let visual_theme = visual_theme("default-dark");
    let visual_locale = visual_locale();
    let main_source = MAIN
        .replace("__VISUAL_THEME__", &visual_theme)
        .replace("__VISUAL_LOCALE__", &visual_locale);
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        module("main", &main_source),
        module("components/tabs", TABS),
        module("components/tag", TAG),
        module("components/avatar", AVATAR),
        module("components/progress", PROGRESS),
        module("components/popover", POPOVER),
    ]));
    EmbeddedScriptView::new(ModuleId::parse("main").unwrap(), scripts, DEFAULT_DARK)
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
        .asset_sources([(
            "icons/close".to_owned(),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: include_bytes!("../../../registry/assets/icons/close.svg").to_vec(),
            },
        )])
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(760.0, 560.0)
                .run()
        })
        .expect("dashboard_layout failed");
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
