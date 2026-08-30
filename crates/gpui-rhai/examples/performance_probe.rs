use std::collections::{BTreeMap, BTreeSet};
use std::hint::black_box;
use std::time::{Duration, Instant};

use gpui_rhai::{
    GpuiNodeRenderer, Length, NumberFormatOptions, RuntimeEngine, TableAlign, TableCellFormat,
    TableColumnSpec, TableColumnWidth, TableNodeSpec, TableRowSpec, TableSelectionMode, TableState,
    UiNode, UiValue,
};

const ITERATIONS: usize = 50;

fn percentile95(samples: &mut [Duration]) -> Duration {
    samples.sort_unstable();
    samples[(samples.len() * 95 / 100).min(samples.len() - 1)]
}

fn main() {
    let source = r"
        fn view() {
            let children = [];
            for index in 0..1000 {
                children.push(text(`row ${index}`));
            }
            column(children)
        }
    ";
    let mut runtime = RuntimeEngine::new();
    let compile_started = Instant::now();
    let compiled = runtime
        .compile(source)
        .expect("performance source compiles");
    let compile = compile_started.elapsed();
    let root = runtime
        .render(&compiled)
        .expect("performance source renders");

    let mut views = Vec::with_capacity(ITERATIONS);
    let mut conversions = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let started = Instant::now();
        black_box(runtime.render(&compiled).expect("repeat view renders"));
        views.push(started.elapsed());

        let started = Instant::now();
        black_box(GpuiNodeRenderer::render(&root));
        conversions.push(started.elapsed());
    }

    let locale = gpui_rhai::load_locale_source(
        runtime.engine(),
        "en.rhai",
        include_str!("../../../registry/locales/en.rhai"),
    )
    .expect("performance locale validates");
    let rows = (0..10_000)
        .map(|index| TableRowSpec {
            key: format!("row-{index}"),
            values: BTreeMap::from([("value".to_owned(), UiValue::Integer(i64::from(index)))]),
        })
        .collect::<Vec<_>>();
    let scalar_started = Instant::now();
    let scalar = TableState::new(table_spec(
        rows.clone(),
        None,
        locale.calendar.clone(),
        locale.number.clone(),
    ))
    .expect("10k scalar Table validates");
    let scalar_build = scalar_started.elapsed();
    let metrics = scalar
        .metrics(120_000.0, 480.0)
        .expect("10k scalar Table metrics");

    let custom_started = Instant::now();
    let custom_nodes = rows
        .iter()
        .map(|row| UiNode::text(row.key.clone()))
        .collect::<Vec<_>>();
    black_box(
        TableState::new(table_spec(
            rows,
            Some(custom_nodes),
            locale.calendar,
            locale.number,
        ))
        .expect("10k custom-cell Table validates"),
    );
    let custom_build = custom_started.elapsed();

    println!(
        "compile={compile:?} view_p95={:?} conversion_1000_nodes_p95={:?} table_scalar_10000={scalar_build:?} table_realized={} table_custom_nodes_10000={custom_build:?}",
        percentile95(&mut views),
        percentile95(&mut conversions),
        metrics.realized_count,
    );
}

fn table_spec(
    rows: Vec<TableRowSpec>,
    custom_cells: Option<Vec<UiNode>>,
    calendar: gpui_rhai::CalendarMetadata,
    number: gpui_rhai::NumberMetadata,
) -> TableNodeSpec {
    TableNodeSpec {
        key: "performance".to_owned(),
        label: "Performance".to_owned(),
        columns: vec![TableColumnSpec {
            key: "value".to_owned(),
            title: "Value".to_owned(),
            width: TableColumnWidth::Flex(1.0),
            align: TableAlign::End,
            format: TableCellFormat::Number(NumberFormatOptions::default()),
            sortable: false,
            custom_cells,
        }],
        rows,
        height: Length::Pixels(480.0),
        row_height: 32.0,
        flex_min_width: 80.0,
        selection_width: 32.0,
        selection_size: 16.0,
        horizontal_scrollbar_height: 12.0,
        horizontal_scrollbar_thumb_min_width: 64.0,
        horizontal_scrollbar_inset: 4.0,
        overscan: 2,
        loading: false,
        loading_slot: None,
        loading_rows: Vec::new(),
        empty_slot: None,
        empty_text: String::new(),
        striped: false,
        selection_mode: TableSelectionMode::None,
        selected_keys: BTreeSet::new(),
        sort: None,
        calendar,
        number,
        check_asset: gpui_rhai::AssetId::parse("app/icons/check").expect("static asset ID"),
        sort_ascending_asset: gpui_rhai::AssetId::parse("app/icons/sort_ascending")
            .expect("static asset ID"),
        sort_descending_asset: gpui_rhai::AssetId::parse("app/icons/sort_descending")
            .expect("static asset ID"),
    }
}
