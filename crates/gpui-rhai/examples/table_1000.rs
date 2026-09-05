use std::collections::BTreeMap;

use gpui_rhai::{
    AssetData, EmbeddedScriptSource, EmbeddedScriptView, ModuleId, NativeCollection,
    ScriptApplication, ScriptViewExtension, UiRuntimeState, UiValue,
};

const ROW_COUNT: usize = 1_000;
const TABLE: &str = include_str!("../../../registry/components/table.rhai");
const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const THEME: &str = include_str!("../../../registry/themes/default_dark.rhai");

const MAIN: &str = r#"
import "components/table" as table;
import "components/button" as button;

fn state_schema() {
    #{ fields: #{
        revision: #{ schema: #{ type: "integer", min: 0 },
            "default": #{ type: "integer", value: 0 } },
        sort: #{ schema: #{ type: "optional", value: #{ type: "object",
            allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                direction: #{ schema: #{ type: "string", allowed: ["ascending", "descending"] },
                    required: true, sensitive: false },
            } } }, "default": #{ type: "null" } },
        selected: #{ schema: #{ type: "array", max_items: __ROW_COUNT__,
            items: #{ type: "string" } }, "default": #{ type: "array", value: [] } },
    } }
}

fn rerender_same_data(ctx, payload) {
    ctx.set_state("revision", ctx.get_state("revision") + 1);
}

fn reverse_rows(ctx, payload) {
    let sort = ctx.get_state("sort");
    let descending = sort == () || sort.direction != "descending";
    ctx.set_state("sort", #{
        key: "id",
        direction: if descending { "descending" } else { "ascending" },
    });
}

fn select_middle(ctx, payload) { ctx.set_state("selected", ["row-0500"]); }
fn clear_selection(ctx, payload) { ctx.set_state("selected", []); }
fn set_sort(ctx, value) { ctx.set_state("sort", value); }
fn set_selection(ctx, value) { ctx.set_state("selected", value); }
fn row_clicked(ctx, key) { () }

fn view(ctx) {
    let sort = ctx.get_state("sort");
    let order = if sort != () && sort.direction == "descending" {
        "descending"
    } else {
        "ascending"
    };
    let columns = [
        #{ key: "id", title: "Row", width: #{ kind: "fixed", value: 104 }, sortable: true },
        #{ key: "account", title: "Account", width: #{ kind: "percent", value: 20 } },
        #{ key: "email", title: "Email", width: #{ kind: "percent", value: 30 } },
        #{ key: "region", title: "Region", width: #{ kind: "fixed", value: 120 } },
        #{ key: "requests", title: "Requests", width: #{ kind: "fixed", value: 112 }, align: "end" },
        #{ key: "p95_ms", title: "P95 ms", width: #{ kind: "fixed", value: 96 }, align: "end" },
        #{ key: "status", title: "Status", width: #{ kind: "fixed", value: 120 } },
    ];
    let selected = ctx.get_state("selected");
    column([
        column([
            text("Rust-backed 1,000-row Table performance baseline")
                .with_style(style().font_size(px(18)).line_height(px(24)).font_weight(700)),
            text("Rhai declares the Table; Rust owns, sorts and projects only the visible rows.")
                .with_style(style().font_size(px(12)).line_height(px(16))
                    .text_color(theme_color("text_muted"))),
        ]).with_style(style().gap(px(2))),
        row([
            button::Button(#{ text: "Re-render same data", variant: "primary", size: "sm",
                on_click: Fn("rerender_same_data") }),
            button::Button(#{ text: "Reverse 1,000 rows", variant: "outline", size: "sm",
                on_click: Fn("reverse_rows") }),
            button::Button(#{ text: "Select row 500", variant: "outline", size: "sm",
                on_click: Fn("select_middle") }),
            button::Button(#{ text: "Clear selection", variant: "ghost", size: "sm",
                on_click: Fn("clear_selection") }),
        ]).with_style(style().width(relative(1.0)).min_width(px(0))
            .gap(px(8)).items_center().overflow_x_scroll()),
        text(`rows=__ROW_COUNT__ columns=7 order=${order} selected=${selected.len} revision=${ctx.get_state("revision")}`)
            .with_style(style().font_size(px(11)).line_height(px(16))
                .text_color(theme_color("text_muted"))),
        table::Table(#{
            key: "table-1000",
            label: "One thousand deterministic accounts",
            row_key: "id",
            rows: ctx.get_native_collection("accounts"),
            columns: columns,
            fill_height: true,
            estimated_row_height: 30.0,
            striped: true,
            selection_mode: "single",
            selected_keys: selected,
            sort: sort,
            on_sort_change: Fn("set_sort"),
            on_selection_change: Fn("set_selection"),
            on_row_click: Fn("row_clicked"),
        }),
    ]).with_style(style().width(relative(1.0)).height(relative(1.0))
        .min_width(px(0)).min_height(px(0)).overflow_hidden()
        .padding(px(20)).gap(px(10)).background(theme_color("surface")))
}
"#;

fn module(id: &str, source: &str) -> (ModuleId, String) {
    (
        ModuleId::parse(id).expect("static module ID"),
        source.to_owned(),
    )
}

pub(crate) fn table_1000_view() -> EmbeddedScriptView {
    let main = MAIN.replace("__ROW_COUNT__", &ROW_COUNT.to_string());
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        module("main", &main),
        module("components/table", TABLE),
        module("components/button", BUTTON),
    ]));
    EmbeddedScriptView::new(ModuleId::parse("main").unwrap(), scripts, THEME)
        .asset_sources(table_assets())
        .extension(TableDataExtension {
            rows: benchmark_collection(),
        })
}

fn table_assets() -> Vec<(String, AssetData)> {
    [
        (
            "icons/disclosure_down",
            include_bytes!("../../../registry/assets/icons/disclosure_down.svg").as_slice(),
        ),
        (
            "icons/chevron_right",
            include_bytes!("../../../registry/assets/icons/chevron_right.svg").as_slice(),
        ),
        (
            "icons/sort_ascending",
            include_bytes!("../../../registry/assets/icons/sort_ascending.svg").as_slice(),
        ),
        (
            "icons/sort_descending",
            include_bytes!("../../../registry/assets/icons/sort_descending.svg").as_slice(),
        ),
    ]
    .into_iter()
    .map(|(name, bytes)| {
        (
            name.to_owned(),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: bytes.to_vec(),
            },
        )
    })
    .collect()
}

struct TableDataExtension {
    rows: NativeCollection,
}

impl ScriptViewExtension for TableDataExtension {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .native_collections
            .register("accounts", self.rows.clone())
            .map_err(|error| error.to_string())
    }
}

fn benchmark_collection() -> NativeCollection {
    let regions = ["Shanghai", "Tokyo", "Singapore", "Frankfurt", "Virginia"];
    let rows = (0..ROW_COUNT).map(|index| {
        let code = format!("{index:04}");
        let status = if index % 7 == 0 {
            "Investigating"
        } else if index % 3 == 0 {
            "Degraded"
        } else {
            "Healthy"
        };
        BTreeMap::from([
            ("id".to_owned(), UiValue::String(format!("row-{code}"))),
            (
                "account".to_owned(),
                UiValue::String(format!("Account {code}")),
            ),
            (
                "email".to_owned(),
                UiValue::String(format!("owner-{}@example.com", index % 137)),
            ),
            (
                "region".to_owned(),
                UiValue::String(regions[index % regions.len()].to_owned()),
            ),
            (
                "requests".to_owned(),
                UiValue::Integer(i64::try_from((index * 7919) % 100_000).unwrap()),
            ),
            (
                "p95_ms".to_owned(),
                UiValue::Float(f64::from(u32::try_from((index * 37) % 2400).unwrap()) / 10.0),
            ),
            ("status".to_owned(), UiValue::String(status.to_owned())),
        ])
    });
    NativeCollection::new("id", rows).expect("benchmark rows have stable keys")
}

fn main() {
    table_1000_view()
        .prepare()
        .and_then(|prepared| {
            ScriptApplication::new(prepared)
                .window_size(1180.0, 760.0)
                .run()
        })
        .expect("table_1000 failed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_thousand_row_workload_prepares() {
        assert_eq!(ROW_COUNT, 1_000);
        table_1000_view().prepare().unwrap();
    }
}
