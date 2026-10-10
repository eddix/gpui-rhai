use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const REGION: &str = include_str!("../../../registry/layouts/region.rhai");
const STACK: &str = include_str!("../../../registry/layouts/stack.rhai");
const INLINE: &str = include_str!("../../../registry/layouts/inline.rhai");
const SECTION: &str = include_str!("../../../registry/patterns/section.rhai");
const STAT: &str = include_str!("../../../registry/patterns/stat.rhai");
const DESCRIPTION_LIST: &str = include_str!("../../../registry/patterns/description_list.rhai");
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
import "layouts/region" as region;
import "layouts/stack" as stack;
import "layouts/inline" as inline;
import "patterns/section" as section;
import "patterns/stat" as stat;
import "patterns/description_list" as description_list;
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

fn heading() {
    column([
        text("Runtime dashboard").with_style(style().typography("title").text_color(theme_color("text_primary")))
            .accessibility_role("heading").accessibility_level(1),
        text("Source-owned Rhai components").with_style(style().typography("caption")
            .text_color(theme_color("text_muted"))),
    ]).with_style(style().flex_col().gap(theme_spacing("xxs")).min_width(px(0)))
}

fn profile(ctx) {
    popover::Popover(#{
        key: "profile", label: "Profile", placement: "bottom", align: "end",
        trigger: avatar::Avatar(#{ name: "Ada Lovelace", initials: "AL", presence: "online" }),
        content: description_list::DescriptionList(#{ label: "Profile", items: [
            #{ label: "Name", value: "Ada Lovelace" }, #{ label: "Role", value: "Runtime maintainer" },
        ] }),
        open: ctx.get_state("profile_open"), on_open_change: Fn("set_profile_open")
    })
}

fn overview() {
    stack::Stack(#{ gap: "section", children: [
        inline::Inline(#{ children: [
            tag::Tag(#{ facet: "env", text: "production" }),
            tag::Tag(#{ facet: "lang", text: "rust", closable: true, on_close: Fn("ignore") })
        ] }),
        stat::stats([
            #{ label: "Hosts", value: "128" },
            #{ label: "Healthy", value: "127" },
            #{ label: "Deploy", value: "72", unit: "%" },
        ]),
        section::Section(#{ title: "Deployment", description: "Rolling out 1.4.2 to eu-west.",
            content: progress::Progress(#{ key: "deployment", value: 72, max: 100, label: "Deployment" }) }),
        section::Section(#{ title: "Synchronization", description: "Background sync of the host inventory.",
            content: progress::Progress(#{ key: "sync", indeterminate: true, label: "Synchronization" }) })
    ] })
}

fn activity() {
    description_list::DescriptionList(#{ label: "Activity", items: [
        #{ label: "09:42", value: "Release candidate built", numeric: true },
        #{ label: "09:44", value: "Integration checks passed", numeric: true },
        #{ label: "09:47", value: "Deployment started", numeric: true },
    ] })
}

fn view(ctx) {
    let selected = ctx.get_state("tab");
    let switcher = tabs::Tabs(#{
        value: selected, label: "Dashboard sections", panel: false,
        tabs: [#{ value: "overview", label: "Overview" }, #{ value: "activity", label: "Activity" }],
        on_change: Fn("set_tab")
    });
    // The region fills the window, so its footer sits at the bottom.
    column([region::Region(#{
        label: "Runtime dashboard", title: heading(), actions: [profile(ctx)], toolbar: switcher,
        body: if selected == "activity" { activity() } else { overview() },
        footer: [text("Updated 09:47")]
    })]).with_style(style().flex_col().width(relative(1.0)).height(relative(1.0)))
}
"#;

fn module(id: &str, source: &str) -> (ModuleId, String) {
    (
        ModuleId::parse(id).expect("static module ID"),
        source.to_owned(),
    )
}

/// Logical window size of the example.
pub const WINDOW: (f32, f32) = (760.0, 560.0);

/// Assemble the dashboard for one theme slug and locale.
///
/// # Panics
///
/// Panics only if a static module ID is invalid.
pub fn view(theme: &str, locale: &str) -> EmbeddedScriptView {
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
        module("layouts/inline", INLINE),
        module("patterns/section", SECTION),
        module("patterns/stat", STAT),
        module("patterns/description_list", DESCRIPTION_LIST),
        module("components/tabs", TABS),
        module("components/tag", TAG),
        module("components/avatar", AVATAR),
        module("components/progress", PROGRESS),
        module("components/popover", POPOVER),
    ]));
    EmbeddedScriptView::new(ModuleId::parse("main").unwrap(), scripts, DEFAULT_DARK)
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
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(WINDOW.0, WINDOW.1)
                .run()
        })
        .expect("dashboard_layout failed");
}

include!("support/icons.rs");
