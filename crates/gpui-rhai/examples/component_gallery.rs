use std::collections::BTreeMap;

use gpui_rhai::{AssetData, EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const COLLAPSIBLE: &str = include_str!("../../../registry/components/collapsible.rhai");
const DIVIDER: &str = include_str!("../../../registry/components/divider.rhai");
const ICON: &str = include_str!("../../../registry/components/icon.rhai");
const MENU: &str = include_str!("../../../registry/components/menu.rhai");
const SKELETON: &str = include_str!("../../../registry/components/skeleton.rhai");
const TOOLTIP: &str = include_str!("../../../registry/components/tooltip.rhai");
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
import "components/collapsible" as collapsible;
import "components/icon" as icon;
import "components/menu" as menu;
import "components/skeleton" as skeleton;
import "components/tooltip" as tooltip;

fn state_schema() {
    #{ fields: #{
        details_open: #{ schema: #{ type: "bool" },
            "default": #{ type: "bool", value: true } },
        menu_open: #{ schema: #{ type: "bool" },
            "default": #{ type: "bool", value: __VISUAL_MENU__ } },
        active_item: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "open" } },
        last_action: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "None" } },
    } }
}

fn set_details(ctx, open) { ctx.set_state("details_open", open); }
fn set_menu(ctx, open) { ctx.set_state("menu_open", open); }
fn set_active(ctx, value) { ctx.set_state("active_item", value); }
fn run_action(ctx, value) {
    ctx.set_state("last_action", value);
    ctx.set_state("menu_open", false);
}

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

fn gallery_menu_items() {
    [
        #{ kind: "item", value: "new", label: "New", shortcut: "⌘N" },
        #{ kind: "item", value: "open", label: "Open", shortcut: "⌘O", checked: true },
        #{ kind: "separator" },
        #{ kind: "item", value: "disabled", label: "Unavailable", disabled: true },
    ]
}

fn image_section() {
    column([
        text("Images and loading"),
        row([
            icon::Icon(#{
                source: asset("app/icons/check"),
                rtl_source: asset("app/icons/close"),
                size: "lg", label: "Directional status icon"
            }),
            skeleton::Skeleton(#{ key: "avatar-placeholder", width: 44, height: 44, radius: 22 }),
            column([
                skeleton::Skeleton(#{ key: "title-placeholder", width: 150, height: 12 }),
                skeleton::Skeleton(#{ key: "body-placeholder", width: 210, height: 10, animated: false })
            ]).with_style(style().gap(px(8)))
        ]).with_style(style().gap(px(14)).items_center())
    ]).with_style(style().gap(px(12)))
}

fn overlay_section(ctx) {
    let menu_node = menu::Menu(#{
        key: "file-menu", trigger: text("File menu").with_style(
            style().padding(px(8)).radius(px(6)).background(theme_color("surface_raised"))
        ),
        open: ctx.get_state("menu_open"), active_value: ctx.get_state("active_item"),
        items: gallery_menu_items(), on_action: Fn("run_action"),
        on_active_change: Fn("set_active"), on_open_change: Fn("set_menu")
    });
    let tooltip_node = tooltip::Tooltip(#{
        key: "gallery-help",
        trigger: text("Hover help").with_style(
            style().padding(px(8)).radius(px(6)).background(theme_color("surface_raised"))
        ),
        content: text("Delayed native tooltip"), placement: "right",
        show_delay_ms: 250, hide_delay_ms: 100
    });
    column([
        text("Overlays"),
        row([menu_node, tooltip_node]).with_style(style().gap(px(12)).items_center()),
        text(`Last action: ${ctx.get_state("last_action")}`)
            .with_style(style().text_color(theme_color("text_muted")))
    ]).with_style(style().gap(px(12)))
}

fn details_section(ctx) {
    let expanded = ctx.get_state("details_open");
    let trigger = row([
        text(if expanded { "▾" } else { "▸" }),
        text("Release details")
    ]).with_style(style().gap(px(8)).padding(px(8)).radius(px(6))
        .background(theme_color("surface_raised")));
    let content = column([
        text("Collapsible content is clipped and animated by Rust."),
        text("State remains controlled by the Rhai caller.")
            .with_style(style().text_color(theme_color("text_muted")))
    ]).with_style(style().padding(px(10)).gap(px(6)));
    collapsible::Collapsible(#{
        key: "release-details", open: expanded, trigger: trigger,
        content: content, content_height: 72, on_open_change: Fn("set_details")
    })
}

fn view(ctx) {
    column([
        text("Component gallery").with_style(style().font_size(rem(1.25))),
        text("Source-owned mechanisms not shown in the four product examples")
            .with_style(style().text_color(theme_color("text_muted"))),
        row([
            image_section(),
            overlay_section(ctx)
        ]).with_style(style().gap(px(40)).items_start()),
        details_section(ctx)
    ]).with_style(
        style().width(px(660)).padding(px(24)).gap(px(20))
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

fn gallery_app() -> EmbeddedScriptView {
    let visual_theme = visual_theme("default-light");
    let visual_locale = visual_locale();
    let menu_open = std::env::var("GPUI_RHAI_VISUAL_STATE")
        .ok()
        .is_some_and(|state| state == "menu");
    let main_source = MAIN
        .replace("__VISUAL_THEME__", &visual_theme)
        .replace("__VISUAL_LOCALE__", &visual_locale)
        .replace("__VISUAL_MENU__", if menu_open { "true" } else { "false" });
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        module("main", &main_source),
        module("components/collapsible", COLLAPSIBLE),
        module("components/divider", DIVIDER),
        module("components/icon", ICON),
        module("components/menu", MENU),
        module("components/skeleton", SKELETON),
        module("components/tooltip", TOOLTIP),
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
                "icons/check".to_owned(),
                AssetData {
                    mime_type: "image/svg+xml".to_owned(),
                    bytes: include_bytes!("../../../registry/assets/icons/check.svg").to_vec(),
                },
            ),
            (
                "icons/close".to_owned(),
                AssetData {
                    mime_type: "image/svg+xml".to_owned(),
                    bytes: include_bytes!("../../../registry/assets/icons/close.svg").to_vec(),
                },
            ),
        ])
}

fn main() {
    gallery_app()
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(720.0, 520.0)
                .run()
        })
        .expect("component_gallery failed");
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_gallery_prepares_all_component_sources() {
        super::gallery_app().prepare().unwrap();
    }

    #[test]
    fn gallery_uses_component_declared_icon_assets() {
        assert!(!super::MAIN.contains("ctx.load_image"));
        assert!(super::MAIN.contains("asset(\"app/icons/check\")"));
        assert!(super::MAIN.contains("asset(\"app/icons/close\")"));
    }
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
