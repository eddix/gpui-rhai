use std::collections::BTreeMap;

use gpui_rhai::{EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptApplication};

const REGION: &str = include_str!("../../../registry/layouts/region.rhai");
const TOOLBAR: &str = include_str!("../../../registry/layouts/toolbar.rhai");
const INLINE_STATE: &str = include_str!("../../../registry/patterns/inline_state.rhai");
const DATA_VIEW: &str = include_str!("../../../registry/patterns/data_view.rhai");
const TOGGLE_GROUP: &str = include_str!("../../../registry/components/toggle_group.rhai");
const TOGGLE: &str = include_str!("../../../registry/components/toggle.rhai");
const SPINNER: &str = include_str!("../../../registry/components/spinner.rhai");
const TABLE: &str = include_str!("../../../registry/components/table.rhai");
const BADGE: &str = include_str!("../../../registry/components/badge.rhai");
const PAGINATION: &str = include_str!("../../../registry/components/pagination.rhai");
const SKELETON: &str = include_str!("../../../registry/components/skeleton.rhai");
const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const ICON: &str = include_str!("../../../registry/components/icon.rhai");
const SELECT: &str = include_str!("../../../registry/components/select.rhai");
const COMBOBOX: &str = include_str!("../../../registry/components/combobox.rhai");
const INPUT: &str = include_str!("../../../registry/components/input.rhai");
const DEFAULT_LIGHT: &str = include_str!("../../../registry/themes/default_light.rhai");
const DEFAULT_DARK: &str = include_str!("../../../registry/themes/default_dark.rhai");
const TOKYO_NIGHT: &str = include_str!("../../../registry/themes/tokyo_night.rhai");
const CATPPUCCIN_MOCHA: &str = include_str!("../../../registry/themes/catppuccin_mocha.rhai");
const EN: &str = include_str!("../../../registry/locales/en.rhai");
const ZH_CN: &str = include_str!("../../../registry/locales/zh_cn.rhai");
const AR: &str = include_str!("../../../registry/locales/ar.rhai");

const MAIN: &str = r#"
import "patterns/data_view" as data_view;
import "components/table" as table;
import "components/pagination" as pagination;
import "components/toggle_group" as toggle_group;
import "components/toggle" as toggle;
import "components/input" as input;

fn state_schema() {
    #{ fields: #{
        query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
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
    let status = if index % 3 == 0 { "Active" } else if index % 3 == 1 { "Away" } else { "Offline" };
    #{
        id: `user-${index}`,
        name: `User ${display_index}`,
        email: `user-${display_index}@example.com`,
        score: ((index * 37) % 1000) / 10.0,
        joined: if index % 2 == 0 { "2026-08-30" } else { "2026-08-31" },
        status: status,
        // Normal is quiet: only "Away" deviates enough to take a color.
        tone: if status == "Away" { "warning" } else { "neutral" },
    }
}

fn matching_rows(ctx) {
    let query = ctx.get_state("query").to_lower();
    let rows = [];
    for index in 0..__ROW_COUNT__ {
        let row = generated_row(index);
        if query == "" || row.name.to_lower().contains(query) || row.email.contains(query) {
            rows.push(row);
        }
    }
    let sort = ctx.get_state("sort");
    if sort != () && sort.key == "name" && sort.direction == "descending" { rows.reverse(); }
    rows
}

fn page_rows(ctx, rows) {
    let start = (ctx.get_state("current_page") - 1) * ctx.get_state("page_size");
    let end = if start + ctx.get_state("page_size") > rows.len {
        rows.len
    } else { start + ctx.get_state("page_size") };
    if start < end { rows.extract(start, end - start) } else { [] }
}

fn set_query(ctx, value) {
    ctx.set_state("query", value);
    ctx.set_state("current_page", 1);
    ctx.set_state("selected", []);
}

fn set_sort(ctx, value) {
    ctx.set_state("sort", value);
    ctx.set_state("current_page", 1);
    ctx.set_state("selected", []);
}
fn set_selection(ctx, value) { ctx.set_state("selected", value); }
fn row_clicked(ctx, key) { () }
fn set_selection_mode(ctx, values) {
    if values.len == 0 { return; }
    ctx.set_state("selection_mode", values[0]);
    ctx.set_state("selected", []);
}
fn set_pagination(ctx, value) {
    ctx.set_state("current_page", value.current_page);
    ctx.set_state("page_size", value.page_size);
    ctx.set_state("selected", []);
}
fn set_loading(ctx, pressed) { ctx.set_state("loading", pressed); }
fn set_grouping(ctx, pressed) {
    ctx.set_state("group_by", if pressed { "status" } else { () });
    ctx.set_state("collapsed_groups", []);
}
fn toggle_group_rows(ctx, group) {
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
        #{ key: "name", title: "Name", width: #{ kind: "fixed", value: 180 }, sortable: true },
        #{ key: "email", title: "Email", width: #{ kind: "flex", value: 1 } },
        #{ key: "score", title: "Score", width: #{ kind: "fixed", value: 96 }, numeric: true },
        #{ key: "joined", title: "Joined", width: #{ kind: "fixed", value: 128 }, numeric: true },
        #{ key: "status", title: "Status", width: #{ kind: "fixed", value: 120 },
            adornments: [#{ text_key: "status", variant_key: "tone", dot: true }] },
    ]
}

fn actions(ctx) {
    let actions = [
        toggle_group::ToggleGroup(#{
            key: "selection-mode", label: "Selection", allow_empty: false,
            values: [ctx.get_state("selection_mode")],
            items: [#{ value: "single", label: "Single" }, #{ value: "multiple", label: "Multiple" }],
            on_change: Fn("set_selection_mode")
        }),
        toggle::Toggle(#{ key: "loading", text: "Loading", pressed: ctx.get_state("loading"),
            on_pressed_change: Fn("set_loading") }),
    ];
    if __GROUP_CONTROL__ {
        actions.push(toggle::Toggle(#{ key: "grouping", text: "Group by status",
            pressed: ctx.get_state("group_by") != (), on_pressed_change: Fn("set_grouping") }));
    }
    actions
}

fn filter(ctx) {
    input::Input(#{ key: "users-filter", label: "Filter users", value: ctx.get_state("query"),
        placeholder: "Filter by name or email", on_change: Fn("set_query") })
        .with_style(style().width(px(240)))
}

fn view(ctx) {
    let selected = ctx.get_state("selected");
    let rows = matching_rows(ctx);
    let grid = table::Table(#{
        key: "users", label: "Users", rows: page_rows(ctx, rows), row_key: "id",
        columns: table_columns(), fill_height: true, resizable_columns: true,
        empty_text: "No users match the filter",
        selection_mode: ctx.get_state("selection_mode"),
        selected_keys: selected, sort: ctx.get_state("sort"),
        group_by: ctx.get_state("group_by"),
        collapsed_groups: ctx.get_state("collapsed_groups"),
        on_sort_change: Fn("set_sort"), on_selection_change: Fn("set_selection"),
        on_row_click: Fn("row_clicked"), on_group_toggle: Fn("toggle_group_rows")
    });
    let pager = pagination::Pagination(#{
        key: "users-pages", label: "Users pagination", total_items: rows.len,
        current_page: ctx.get_state("current_page"), page_size: ctx.get_state("page_size"),
        page_size_options: [10, 25, 50, 100], show_summary: false, on_change: Fn("set_pagination")
    });
    // Loading and empty are shown in place of the table by DataView.
    let state = if ctx.get_state("loading") {
        #{ state: "loading", title: "Loading users" }
    } else if __ROW_COUNT__ == 0 {
        #{ state: "empty", title: "No users yet", description: "Invite people to see them here." }
    } else { () };
    column([data_view::DataView(#{
        key: "users-view", label: "Users", title: "Users", body: grid, state: state,
        toolbar: #{ filters: [filter(ctx)], actions: actions(ctx) },
        footer: #{ status: pager, selection: `${selected.len} selected`, count: `${rows.len} users` }
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
pub const WINDOW: (f32, f32) = (980.0, 720.0);

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
        .expect("data_table failed");
}

/// Assemble the table for one theme slug, locale and visual state (`default`,
/// `selected`, `loading`, `empty` or `grouped`).
///
/// # Panics
///
/// Panics only if a static module ID is invalid.
pub fn view(theme: &str, locale: &str, visual_state: &str) -> EmbeddedScriptView {
    let theme = if matches!(
        theme,
        "default-light" | "default-dark" | "tokyo-night" | "catppuccin-mocha"
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
        module("layouts/region", REGION),
        module("layouts/toolbar", TOOLBAR),
        module("patterns/inline_state", INLINE_STATE),
        module("patterns/data_view", DATA_VIEW),
        module("components/table", TABLE),
        module("components/badge", BADGE),
        module("components/pagination", PAGINATION),
        module("components/skeleton", SKELETON),
        module("components/button", BUTTON),
        module("components/icon", ICON),
        module("components/select", SELECT),
        module("components/combobox", COMBOBOX),
        module("components/input", INPUT),
        module("components/toggle_group", TOGGLE_GROUP),
        module("components/toggle", TOGGLE),
        module("components/spinner", SPINNER),
    ]));
    EmbeddedScriptView::new(ModuleId::parse("main").unwrap(), scripts, DEFAULT_LIGHT)
        .token_base(include_str!("../../../registry/tokens.rhai"))
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
        .asset_sources(registry_icons())
}

include!("support/icons.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_data_table_visual_state_prepares() {
        for state in ["default", "selected", "loading", "empty", "grouped"] {
            view("default-light", "en", state)
                .prepare()
                .unwrap_or_else(|error| panic!("{state}: {error}"));
        }
    }
}
