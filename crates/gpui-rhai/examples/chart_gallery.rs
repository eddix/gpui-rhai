use std::collections::BTreeMap;

use gpui_rhai::{
    ChartDataLimits, ChartDataset, ChartGeoMap, ChartValue, EmbeddedScriptSource,
    EmbeddedScriptView, ModuleId, NativeChartData, RuntimeEngine, ScriptApplication,
    ScriptViewExtension, UiRuntimeState,
};

#[allow(dead_code)]
#[path = "../../../registry/src/lib.rs"]
mod registry_snapshot;
use registry_snapshot::{
    AR_LOCALE, BUNDLED_THEME_SOURCES, CHART_SOURCE, DEFAULT_THEME, EN_LOCALE, ZH_CN_LOCALE,
};

const MAIN: &str = r#"
import "charts/chart" as chart;

fn state_schema() { #{ fields: #{
    theme_index: #{ schema: #{ type: "integer", min: 0, max: 14 },
        "default": #{ type: "integer", value: 0 } },
    locale_index: #{ schema: #{ type: "integer", min: 0, max: 2 },
        "default": #{ type: "integer", value: 0 } },
} } }
fn next_theme(ctx, payload) {
    let index = (ctx.get_state("theme_index") + 1) % 15;
    ctx.set_state("theme_index", index);
    if index == 0 { ctx.set_theme("Default", "Dark"); }
    else if index == 1 { ctx.set_theme("Default", "Light"); }
    else if index == 2 { ctx.set_theme("Tokyo Night", "Night"); }
    else if index == 3 { ctx.set_theme("Tokyo Night", "Storm"); }
    else if index == 4 { ctx.set_theme("Catppuccin", "Latte"); }
    else if index == 5 { ctx.set_theme("Catppuccin", "Mocha"); }
    else if index == 6 { ctx.set_theme("Ethereal", "Dark"); }
    else if index == 7 { ctx.set_theme("Everforest", "Dark"); }
    else if index == 8 { ctx.set_theme("Gruvbox", "Dark"); }
    else if index == 9 { ctx.set_theme("Hackerman", "Dark"); }
    else if index == 10 { ctx.set_theme("Nord", "Dark"); }
    else if index == 11 { ctx.set_theme("Retro 82", "Dark"); }
    else if index == 12 { ctx.set_theme("Hermarchy", "Dark"); }
    else if index == 13 { ctx.set_theme("Futurism", "Dark"); }
    else { ctx.set_theme("Aetheria", "Dark"); }
}
fn next_locale(ctx, payload) {
    let index = (ctx.get_state("locale_index") + 1) % 3;
    ctx.set_state("locale_index", index);
    if index == 0 { ctx.set_locale("en"); }
    else if index == 1 { ctx.set_locale("zh-CN"); }
    else { ctx.set_locale("ar"); }
}
fn gallery_control(key, label, handler) {
    text(label).with_key(key).accessibility_role("button").on_click(handler)
        .with_style(style().padding_x(px(10)).padding_y(px(6)).border(px(1))
            .border_color(theme_color("border")))
}

fn series_rows() {
    [
        #{ id: "jan", category: "Jan", value: 18.0, other: 12.0, size: 40.0 },
        #{ id: "feb", category: "Feb", value: 31.0, other: 22.0, size: 80.0 },
        #{ id: "mar", category: "Mar", value: 26.0, other: 35.0, size: 25.0 },
        #{ id: "apr", category: "Apr", value: 44.0, other: 29.0, size: 120.0 },
        #{ id: "may", category: "May", value: 38.0, other: 41.0, size: 64.0 },
    ]
}
fn candle_rows() {
    [
        #{ id: "m", day: "Mon", open: 24.0, close: 29.0, low: 20.0, high: 33.0 },
        #{ id: "t", day: "Tue", open: 29.0, close: 25.0, low: 22.0, high: 32.0 },
        #{ id: "w", day: "Wed", open: 25.0, close: 36.0, low: 24.0, high: 39.0 },
        #{ id: "r", day: "Thu", open: 36.0, close: 34.0, low: 30.0, high: 41.0 },
    ]
}
fn geo_rows() {
    [#{ id: "north", name: "north", value: 72.0, lon: 4.0, lat: 7.0,
       source_lon: 2.0, source_lat: 2.0, target_lon: 8.0, target_lat: 8.0 },
     #{ id: "south", name: "south", value: 34.0, lon: 6.0, lat: 3.0,
       source_lon: 8.0, source_lat: 2.0, target_lon: 2.0, target_lat: 8.0 }]
}
fn card(title, content) {
    column([
        text(title).with_style(theme_typography("subtitle")),
        content.with_style(style().width(px(360)).height(px(250))),
    ]).with_style(style().padding(px(14)).gap(px(8)).border(px(1))
        .border_color(theme_color("border")).background(theme_color("surface_raised")))
}
fn single(kind, key, encode, data) {
    chart::Chart(#{ key: key, data: data, key_dimension: "id", spec: #{
        title: kind,
        series: [#{ key: key, name: kind, kind: kind, encode: encode }],
        annotations: if kind == "bar" { [
            #{ key: "target", kind: "baseline", axis: "y", value: 30.0, label: "Target" }
        ] } else { [] },
        brush: if kind == "scatter" { "xy" } else { "none" },
    } })
}
fn polar(kind, key) {
    chart::Chart(#{ key: key, data: series_rows(), key_dimension: "id", spec: #{
        title: kind,
        regions: [#{ key: "main", kind: "polar" }],
        series: [#{ key: key, name: kind, kind: kind, encode: #{ name: "category", value: "value" } }],
    } })
}
fn geo(kind, key, encode) {
    chart::Chart(#{ key: key, data: geo_rows(), key_dimension: "id", spec: #{
        title: kind,
        regions: [#{ key: "main", kind: "geo_2d", map: "demo_map", projection: "equirectangular" }],
        series: [#{ key: key, name: kind, kind: kind, encode: encode }],
    } })
}

fn view(ctx) {
    let rows = series_rows();
    let streaming = ctx.get_native_chart_data("stream");
    column([
        text("CHART GALLERY").with_style(theme_typography("heading")),
        text("One native scene · stable datum identity · Rust transforms · Motion Runtime policy")
            .with_style(theme_typography("body_small").text_color(theme_color("text_muted"))),
        row([
            gallery_control("theme", `Next theme (${ctx.get_state("theme_index") + 1}/15)`, Fn("next_theme")),
            gallery_control("locale", `Next locale (${ctx.get_state("locale_index") + 1}/3)`, Fn("next_locale")),
        ]).with_style(style().gap(px(8))),
        row([
            card("Bar + annotation", single("bar", "bar", #{ x: "category", y: "value" }, rows)),
            card("Line · 100k Host rows", single("line", "line", #{ x: "x", y: "y" }, streaming)),
            card("Area", single("area", "area", #{ x: "category", y: "value" }, series_rows())),
        ]).with_style(style().gap(px(16))),
        row([
            card("Scatter + brush", single("scatter", "scatter", #{ x: "value", y: "other", size: "size" }, series_rows())),
            card("Heatmap", single("heatmap", "heat", #{ x: "category", y: "other", value: "value" }, series_rows())),
            card("Candlestick", chart::CandlestickChart(#{ key: "candle", data: candle_rows(), key_dimension: "id",
                encode: #{ x: "day", open: "open", close: "close", low: "low", high: "high" } })),
        ]).with_style(style().gap(px(16))),
        row([
            card("Pie", polar("pie", "pie")), card("Donut", polar("donut", "donut")),
            card("Radar", polar("radar", "radar")),
        ]).with_style(style().gap(px(16))),
        row([
            card("Gauge", polar("gauge", "gauge")), card("Funnel", polar("funnel", "funnel")),
            card("Choropleth", geo("map", "map", #{ name: "name", value: "value" })),
        ]).with_style(style().gap(px(16))),
        row([
            card("Geo scatter", geo("geo_scatter", "geo_scatter", #{ longitude: "lon", latitude: "lat", value: "value" })),
            card("Geo lines", geo("geo_lines", "geo_lines", #{ source_longitude: "source_lon", source_latitude: "source_lat", target_longitude: "target_lon", target_latitude: "target_lat" })),
        ]).with_style(style().gap(px(16))),
    ]).with_style(style().width(relative(1.0)).padding(px(24)).gap(px(18))
        .overflow_y_scroll().background(theme_color("surface")).text_color(theme_color("text_primary")))
}
"#;

#[derive(Clone)]
struct GalleryExtension {
    stream: NativeChartData,
}

impl ScriptViewExtension for GalleryExtension {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        let map = ChartGeoMap::from_geojson(
            "demo_map",
            r#"{"type":"FeatureCollection","features":[
              {"type":"Feature","id":"north","properties":{"name":"north"},"geometry":{"type":"Polygon","coordinates":[[[0,5],[10,5],[10,10],[0,10],[0,5]]]}},
              {"type":"Feature","id":"south","properties":{"name":"south"},"geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,5],[0,5],[0,0]]]}}
            ]}"#,
        )
        .map_err(|error| error.to_string())?;
        engine
            .register_chart_map(map)
            .map_err(|error| error.to_string())
    }

    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .register_native_chart_data("stream", self.stream.clone())
            .map_err(|error| error.to_string())
    }
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn stream_data(rows: usize) -> NativeChartData {
    let values = (0..rows)
        .map(|index| {
            let x = index as f64;
            BTreeMap::from([
                ("id".to_owned(), ChartValue::String(format!("p{index}"))),
                ("x".to_owned(), ChartValue::Number(x)),
                (
                    "y".to_owned(),
                    ChartValue::Number((x / 270.0).sin() * 20.0 + (x / 67.0).cos() * 4.0),
                ),
            ])
        })
        .collect::<Vec<_>>();
    let dataset = ChartDataset::from_chart_rows(
        "main",
        &values,
        Some("id".to_owned()),
        ChartDataLimits::default(),
    )
    .expect("static stream dataset is valid");
    NativeChartData::new([dataset], ChartDataLimits::default())
        .expect("static native chart data is valid")
}

pub(crate) fn gallery_view(stream: NativeChartData) -> EmbeddedScriptView {
    let scripts = EmbeddedScriptSource::new(BTreeMap::from([
        (ModuleId::parse("main").unwrap(), MAIN.to_owned()),
        (
            ModuleId::parse("charts/chart").unwrap(),
            CHART_SOURCE.to_owned(),
        ),
    ]));
    EmbeddedScriptView::new(ModuleId::parse("main").unwrap(), scripts, DEFAULT_THEME)
        .theme_sources(
            BUNDLED_THEME_SOURCES
                .iter()
                .filter(|(name, _)| *name != "default_dark.rhai")
                .map(|(name, source)| ((*name).to_owned(), (*source).to_owned())),
        )
        .locale_sources([
            ("en.rhai".to_owned(), EN_LOCALE.to_owned()),
            ("zh_cn.rhai".to_owned(), ZH_CN_LOCALE.to_owned()),
            ("ar.rhai".to_owned(), AR_LOCALE.to_owned()),
        ])
        .extension(GalleryExtension { stream })
}

pub(crate) fn prepared_with_stream(
    stream: NativeChartData,
) -> Result<gpui_rhai::PreparedScriptView, gpui_rhai::ScriptViewError> {
    gallery_view(stream).prepare()
}

pub(crate) fn prepared() -> Result<gpui_rhai::PreparedScriptView, gpui_rhai::ScriptViewError> {
    prepared_with_stream(stream_data(100_000))
}

fn main() {
    ScriptApplication::new(prepared().expect("chart gallery prepares"))
        .window_size(1_180.0, 820.0)
        .run()
        .expect("chart gallery runs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_chart_gallery_prepares_with_host_map_and_100k_data() {
        prepared().unwrap();
    }
}
