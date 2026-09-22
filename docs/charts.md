# Chart Runtime

The optional `charts` feature is GPUI Rhai's native retained 2D visualization
system. Rhai declares chart intent; Rust owns typed data, transforms, scales,
layout, motion sampling, hit testing, keyboard navigation, streaming revisions,
and static export. ECharts and similar libraries are feature references only;
this is not an ECharts option-schema compatibility layer.

```toml
[dependencies]
gpui-rhai = { version = "0.1.5", features = ["charts"] }
```

`gpui-rhai add chart` installs `ui/charts/chart.rhai` and enables the Cargo
feature automatically.

## Rhai surface

The source component accepts a stable key, one typed `spec`, data, controlled
selection/legend/viewport state, and low-frequency semantic callbacks:

```rhai
import "charts/chart" as chart;

chart::Chart(#{
    key: "requests",
    key_dimension: "id",
    data: rows,
    selected_keys: selected,
    hidden_series: hidden,
    zoom: zoom,
    pan_x: pan_x,
    pan_y: pan_y,
    spec: #{
        title: "Requests",
        series: [
            #{ key: "actual", name: "Actual", kind: "line",
               encode: #{ x: "time", y: "requests" } },
            #{ key: "capacity", name: "Capacity", kind: "bar",
               encode: #{ x: "time", y: "capacity" } },
        ],
        brush: "xy",
        link_group: "operations",
        link_domain: "time",
    },
    on_select: Fn("selected"),
    on_zoom_change: Fn("zoomed"),
    on_brush_change: Fn("brushed"),
    on_legend_change: Fn("legend_changed"),
});
```

`BarChart`, `LineChart`, `AreaChart`, `ScatterChart`, `HeatmapChart`,
`CandlestickChart`, `PieChart`, `DonutChart`, `RadarChart`, `GaugeChart`,
`FunnelChart`, `MapChart`, `GeoScatterChart`, and `GeoLinesChart` are ergonomic
single-kind adapters exported by the same source module. Use `Chart` for mixed
series or multiple coordinate regions.

The first coordinate systems are `cartesian_2d`, `polar`, and `geo_2d`.
Regions are named and series bind to them explicitly. One chart may contain a
grid of multiple regions.

## Data ownership

Arrays of at most 10,000 Rhai object rows are appropriate for small charts.
They are converted once into typed columns with a separate null bitmap. Set
`key_dimension` to preserve datum identity across replacement, selection,
focus, and motion. Without a key dimension, row data remains displayable but
updates use replacement identity.

Large or streaming applications construct `NativeChartData` in Rust and
register it before lifecycle `init`:

```rust
impl ScriptViewExtension for DataExtension {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .register_native_chart_data("metrics", self.metrics.clone())
            .map_err(|error| error.to_string())
    }
}
```

Rhai reads the opaque handle with `ctx.get_native_chart_data("metrics")` and
passes it to `Chart`. `replace`, `append`, and `append_sliding` publish one
atomic revision. The mounted chart listens to the handle directly and consumes
the newest snapshot, so producer bursts do not create a Rhai callback queue or
rerender the application component for every update.

Column and dataset constructors reject mixed incompatible types, non-finite
numbers, inconsistent lengths, duplicate keys, and configured row/string
limits before work is scheduled.

## Encode and transforms

Series encode named dimensions into `x`, `y`, `value`, `name`, `size`, `color`,
OHLC, longitude/latitude, or source/target geo channels. Built-in transform
pipelines support:

- filter and stable sort;
- aggregate and bin;
- stack and normalize;
- moving window;
- LTTB line/area downsampling.

Transforms execute in Rust on immutable candidates. Large line/area series are
automatically downsampled for drawing when no explicit policy is supplied.
Rendered marks retain original datum keys; Inspector diagnostics disclose the
sample count. Aggregation intentionally creates new semantic data.

Trusted Hosts may register a typed transform with
`RuntimeEngine::register_chart_transform`. Per-row Rhai transforms and
per-datum Rhai render functions are deliberately unsupported.

## Interaction and motion

Hover, tooltip, crosshair, wheel zoom, middle-button pan, brush tracking, and
keyboard active-mark navigation stay native. Arrow keys move focus, Home/End
jump, and Enter/Space activate the focused datum. Semantic callbacks are
bounded commit events rather than raw pointer streams.

Selection (`selected_keys`), legend visibility (`hidden_series`), and viewport
(`zoom`, `pan_x`, `pan_y`) are controllable. Charts sharing both
`link_group` and `link_domain` synchronize hover, selection highlight, zoom,
and pan through weak native entity links without Rhai event fan-out.

Chart transitions use one aggregated chart resource and the existing Motion
theme duration/easing plus Host normal/reduced/none policy. Compatible keyed
rectangles, circles, polylines, and polygons interpolate; entering lines draw by
path progress and removed marks fade. A chart does not register thousands of
global Motion sources.

## Axes, time, formatting, and locale

Axes support auto/linear/log/category/time scales, explicit normal/reversed
direction, position, domain, title, and declarative formatting. RTL changes
text and legend layout, never axis direction implicitly.

Time columns are typed epoch milliseconds. A time axis requires an explicit
`UTC`, `offset:<minutes>`, or IANA timezone; ambiguous date strings are not
parsed. Tick labels use conventional calendar/time forms through the locked
Jiff timezone database.

Number labels consume the active locale's validated digits, grouping and
decimal metadata. Format specs add precision, compact, percent, prefix and
suffix policy. Trusted applications can register an application formatter with
`RuntimeEngine::register_chart_formatter`; it receives a finite value, selected
locale ID, and typed format specification. No per-mark Rhai formatter runs on
the paint or pointer path.

## Geo boundary

`ChartGeoMap::from_geojson` accepts Polygon and MultiPolygon
FeatureCollections. `ChartGeoMap::from_svg` accepts self-contained visible SVG
paths with stable `id` values and flattens curves for deterministic hit tests.
The registry includes equirectangular and Mercator projection; Hosts can add a
typed Rust projection.

GPUI Rhai bundles no world/country boundary data, does no network access, and
does no address geocoding. The application owns map acquisition and licensing.

## Annotations and brush

`annotations` supports `mark_point`, `mark_line`, `mark_area`,
`threshold_band`, and `baseline`. Annotation activation emits its stable key
and label. Cartesian brush modes are `x`, `y`, and `xy`; Geo supports declared
rectangle/region selection. Freehand lasso is not part of 0.1.5.

## Rust extensions and export

`RuntimeEngine::register_chart_series` installs a compile-time trusted custom
series renderer. It receives the typed dataset, coordinate scales, plot bounds,
and resolved chart theme, and returns public `ChartMark` geometry. Returned
marks are validated for series identity, duplicate keys, finite geometry, and
budgets, then join the same motion, hit-test, tooltip, semantic, Inspector, and
export scene as built-ins. Dynamic libraries, downloaded plugins, Wasm, and a
Rhai `renderItem` equivalent are outside the trust model.

Host-only `export_chart_svg` and `export_chart_png` render an explicit terminal
scene with bounded size, locale, theme, data revision, and registered geo
sources. They return bytes/text and grant Rhai no filesystem authority.

## Performance contract

The target envelope is roughly 10,000 fully interactive marks or 100,000
Rust-downsampled/streaming points on the reference macOS environment. The
`chart_gallery` example includes all built-in series and a 100,000-row
`NativeChartData` line series:

```text
cargo run --release -p gpui-rhai --features charts --example chart_gallery
```

Graph, Tree, Treemap, Sunburst, Sankey, freehand lasso, PDF/video/animated SVG,
editing marks to mutate data, 3D, and Globe are explicit later work rather than
silent partial implementations.
