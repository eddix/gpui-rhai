use std::collections::BTreeMap;

use gpui_rhai::{AssetData, EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const TABLE: &str = include_str!("../../../registry/components/table.rhai");
const PAGINATION: &str = include_str!("../../../registry/components/pagination.rhai");
const SKELETON: &str = include_str!("../../../registry/components/skeleton.rhai");
const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const ICON: &str = include_str!("../../../registry/components/icon.rhai");
const SELECT: &str = include_str!("../../../registry/components/select.rhai");
const DROPDOWN: &str = include_str!("../../../registry/components/dropdown.rhai");
const INPUT: &str = include_str!("../../../registry/components/input.rhai");
const DEFAULT_LIGHT: &str = include_str!("../../../registry/themes/default_light.rhai");
const DEFAULT_DARK: &str = include_str!("../../../registry/themes/default_dark.rhai");
const TOKYO_NIGHT: &str = include_str!("../../../registry/themes/tokyo_night.rhai");
const CATPPUCCIN_MOCHA: &str = include_str!("../../../registry/themes/catppuccin_mocha.rhai");
const EN: &str = include_str!("../../../registry/locales/en.rhai");
const ZH_CN: &str = include_str!("../../../registry/locales/zh_cn.rhai");
const AR: &str = include_str!("../../../registry/locales/ar.rhai");

const MAIN: &str = r#"
import "components/table" as table;
import "components/pagination" as pagination;
import "components/button" as button;

fn state_schema() {
    #{ fields: #{
        current_page: #{ schema: #{ type: "integer", min: 1 }, "default": #{ type: "integer", value: 1 } },
        page_size: #{ schema: #{ type: "integer", min: 1 }, "default": #{ type: "integer", value: 25 } },
        sort: #{ schema: #{ type: "optional", value: #{ type: "map", values: #{ type: "ui_value" } } }, "default": #{ type: "null" } },
        selected: #{ schema: #{ type: "array", max_items: 500, items: #{ type: "string" } }, "default": #{ type: "array", value: __VISUAL_SELECTED__ } },
        loading: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_LOADING__ } },
        selection_mode: #{ schema: #{ type: "string", allowed: ["single", "multiple"] },
            "default": #{ type: "string", value: "multiple" } },
        group_by: #{ schema: #{ type: "optional", value: #{ type: "string" } },
            "default": __VISUAL_GROUP_BY__ },
        collapsed_groups: #{ schema: #{ type: "array", max_items: 16,
            items: #{ type: "string" } }, "default": #{ type: "array", value: [] } },
    } }
}

fn generated_row(index) {
    let display_index = if index < 10 { `00${index}` }
        else if index < 100 { `0${index}` } else { `${index}` };
    #{
        id: `user-${index}`,
        name: `User ${display_index}`,
        email: `user-${display_index}@example.com`,
        score: ((index * 37) % 1000) / 10.0,
        joined: if index % 2 == 0 { "2026-08-30" } else { "2026-08-31" },
        status: if index % 3 == 0 { "Active" } else if index % 3 == 1 { "Away" } else { "Offline" },
    }
}

fn page_rows(ctx) {
    let total = __ROW_COUNT__;
    let sort = ctx.get_state("sort");
    let start = (ctx.get_state("current_page") - 1) * ctx.get_state("page_size");
    let end = if start + ctx.get_state("page_size") > total {
        total
    } else { start + ctx.get_state("page_size") };
    let page = [];
    if start < end {
        for logical_index in start..end {
            let index = if sort != () && sort.key == "name" && sort.direction == "descending" {
                total - 1 - logical_index
            } else { logical_index };
            page.push(generated_row(index));
        }
    }
    page
}

fn set_sort(ctx, value) {
    ctx.set_state("sort", value);
    ctx.set_state("current_page", 1);
    ctx.set_state("selected", []);
}
fn set_selection(ctx, value) { ctx.set_state("selected", value); }
fn row_clicked(ctx, key) { () }
fn set_selection_mode(ctx, mode) {
    ctx.set_state("selection_mode", mode);
    ctx.set_state("selected", []);
}
fn set_pagination(ctx, value) {
    ctx.set_state("current_page", value.current_page);
    ctx.set_state("page_size", value.page_size);
    ctx.set_state("selected", []);
}
fn toggle_loading(ctx, payload) { ctx.set_state("loading", !ctx.get_state("loading")); }
fn toggle_grouping(ctx, payload) {
    ctx.set_state("group_by", if ctx.get_state("group_by") == () { "status" } else { () });
    ctx.set_state("collapsed_groups", []);
}
fn toggle_group(ctx, group) {
    let collapsed = ctx.get_state("collapsed_groups");
    let next = [];
    let found = false;
    for value in collapsed {
        if value == group { found = true; }
        else { next.push(value); }
    }
    if !found { next.push(group); }
    ctx.set_state("collapsed_groups", next);
}

fn init(ctx) {
    let theme = "__VISUAL_THEME__";
    if theme == "default-dark" { ctx.set_theme("Default", "Dark"); }
    else if theme == "tokyo-night" { ctx.set_theme("Tokyo Night", "Night"); }
    else if theme == "catppuccin-mocha" { ctx.set_theme("Catppuccin", "Mocha"); }
    else { ctx.set_theme("Default", "Light"); }
    ctx.set_locale("__VISUAL_LOCALE__");
}

fn table_columns() {
    [
        #{ key: "name", title: "Name", width: #{ kind: "fixed", value: 220 }, sortable: true },
        #{ key: "email", title: "Email", width: #{ kind: "fixed", value: 320 } },
        #{ key: "score", title: "Score", width: #{ kind: "flex", value: 100 }, align: "end" },
        #{ key: "joined", title: "Joined", width: #{ kind: "percent", value: 25 }, align: "end" },
        #{ key: "status", title: "Status", width: #{ kind: "fixed", value: 110 } },
        #{ key: "id", title: "Action", width: #{ kind: "fixed", value: 80 },
            align: "center" },
    ]
}

fn table_controls(ctx) {
    row([
        text("Data table").with_style(style().font_size(rem(1.25))),
        row([
            button::Button(#{ text: "Single", size: "xs", variant: "ghost" })
                .on_click_value(Fn("set_selection_mode"), "single"),
            button::Button(#{ text: "Multiple", size: "xs", variant: "ghost" })
                .on_click_value(Fn("set_selection_mode"), "multiple"),
            text(if ctx.get_state("loading") { "Show data" } else { "Show loading" })
                .with_style(style().padding(theme_spacing("sm")).radius(theme_radius("md"))
                    .border(px(1)).border_color(theme_color("border")))
                .on_click(Fn("toggle_loading")),
            if __GROUP_CONTROL__ {
                button::Button(#{
                    text: if ctx.get_state("group_by") == () { "Group status" } else { "Ungroup" },
                    size: "xs", variant: "outline", on_click: Fn("toggle_grouping")
                })
            } else { fragment([]) }
        ]).with_style(style().gap(theme_spacing("xs")).items_center())
    ]).with_style(style().justify_between().items_center())
}

fn view(ctx) {
    let columns = table_columns();
    column([
        table_controls(ctx),
        table::Table(#{
            key: "users", label: "Users", rows: page_rows(ctx), row_key: "id",
            columns: columns, height: 470, striped: true,
            loading: ctx.get_state("loading"),
            selection_mode: ctx.get_state("selection_mode"),
            selected_keys: ctx.get_state("selected"), sort: ctx.get_state("sort"),
            group_by: ctx.get_state("group_by"),
            collapsed_groups: ctx.get_state("collapsed_groups"),
            on_sort_change: Fn("set_sort"), on_selection_change: Fn("set_selection"),
            on_row_click: Fn("row_clicked"), on_group_toggle: Fn("toggle_group")
        }),
        pagination::Pagination(#{
            key: "users-pages", total_items: __ROW_COUNT__,
            current_page: ctx.get_state("current_page"), page_size: ctx.get_state("page_size"),
            page_size_options: [10, 25, 50, 100], on_change: Fn("set_pagination")
        })
    ]).with_style(style().width(px(920)).padding(theme_spacing("lg")).gap(theme_spacing("sm"))
        .background(theme_color("surface")))
}
"#;

fn module(id: &str, source: &str) -> (ModuleId, String) {
    (
        ModuleId::parse(id).expect("static module ID"),
        source.to_owned(),
    )
}

fn main() {
    let theme =
        std::env::var("GPUI_RHAI_VISUAL_THEME").unwrap_or_else(|_| "default-light".to_owned());
    let locale = std::env::var("GPUI_RHAI_VISUAL_LOCALE").unwrap_or_else(|_| "en".to_owned());
    let visual_state =
        std::env::var("GPUI_RHAI_VISUAL_STATE").unwrap_or_else(|_| "default".to_owned());
    data_table_view(&theme, &locale, &visual_state)
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(980.0, 720.0)
                .run()
        })
        .expect("data_table failed");
}

fn data_table_view(theme: &str, locale: &str, visual_state: &str) -> EmbeddedScriptView {
    let main_source = MAIN
        .replace("__VISUAL_THEME__", theme)
        .replace("__VISUAL_LOCALE__", locale)
        .replace(
            "__VISUAL_SELECTED__",
            if visual_state == "selected" {
                "[#{ type: \"string\", value: \"user-0\" }, #{ type: \"string\", value: \"user-1\" }]"
            } else {
                "[]"
            },
        )
        .replace(
            "__VISUAL_LOADING__",
            if visual_state == "loading" {
                "true"
            } else {
                "false"
            },
        )
        .replace(
            "__VISUAL_GROUP_BY__",
            if visual_state == "grouped" {
                "#{ type: \"string\", value: \"status\" }"
            } else {
                "#{ type: \"null\" }"
            },
        )
        .replace(
            "__GROUP_CONTROL__",
            if visual_state == "grouped" { "true" } else { "false" },
        )
        .replace(
            "__ROW_COUNT__",
            if visual_state == "empty" { "0" } else { "500" },
        );
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        module("main", &main_source),
        module("components/table", TABLE),
        module("components/pagination", PAGINATION),
        module("components/skeleton", SKELETON),
        module("components/button", BUTTON),
        module("components/icon", ICON),
        module("components/select", SELECT),
        module("components/dropdown", DROPDOWN),
        module("components/input", INPUT),
    ]));
    EmbeddedScriptView::new(ModuleId::parse("main").unwrap(), scripts, DEFAULT_LIGHT)
        .theme_sources([
            ("default_dark.rhai".to_owned(), DEFAULT_DARK.to_owned()),
            ("tokyo_night.rhai".to_owned(), TOKYO_NIGHT.to_owned()),
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
                svg(include_bytes!("../../../registry/assets/icons/check.svg")),
            ),
            (
                "icons/close".to_owned(),
                svg(include_bytes!("../../../registry/assets/icons/close.svg")),
            ),
            (
                "icons/chevron_left".to_owned(),
                svg(include_bytes!(
                    "../../../registry/assets/icons/chevron_left.svg"
                )),
            ),
            (
                "icons/chevron_right".to_owned(),
                svg(include_bytes!(
                    "../../../registry/assets/icons/chevron_right.svg"
                )),
            ),
        ])
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
    fn every_data_table_visual_state_prepares() {
        for state in ["default", "selected", "loading", "empty", "grouped"] {
            data_table_view("default-light", "en", state)
                .prepare()
                .unwrap_or_else(|error| panic!("state {state}: {error}"));
        }
    }
}
