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
    viewport: viewport,
    viewport_revision: viewport_revision,
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
series or multiple coordinate regions. The separately installable formal
`charts/bar_chart`, `charts/line_chart`, `charts/pie_chart`, and
`charts/map_chart` components forward the same controlled viewport fields,
including `viewport_revision`.

Every data event identifies a datum structurally with `dataset`, `series_key`,
`datum_key`, `region_key`, and the presented data `revision`. Business keys are
never interpreted as legend, axis, or annotation roles. Brush events retain a
bounded `keys` list for simple controlled state and additionally expose
unambiguous `data` references for linked or multi-series charts.

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

Transforms execute in Rust on immutable candidates. Every stage preserves or
declares its output schema; encode validation happens after the pipeline, so an
aggregate output can be bound directly. Empty filters and all-null subsets keep
their typed columns. Large line/area series are automatically downsampled only
for drawing. The transformed semantic data and all stable keys remain available
to selection and accessibility, while null gaps remain separate draw segments.
Numeric/time lines use LTTB directly; categorical lines use their stable
category ordinal only for sampling geometry, while retaining the original
category values and business keys.

Trusted Hosts may register a typed transform with
`RuntimeEngine::register_chart_transform`. Per-row Rhai transforms and
per-datum Rhai render functions are deliberately unsupported.

## Interaction and motion

Hover, tooltip, crosshair, wheel zoom, middle-button pan, brush tracking, and
keyboard active-mark navigation stay native. Arrow keys move focus, Home/End
jump, and Enter/Space activate the focused datum. Semantic callbacks are
bounded commit events rather than raw pointer streams.

Selection (`selected_keys`), legend visibility (`hidden_series`), and the typed
`viewport` are controllable. `zoom`/`pan_x`/`pan_y` remain concise shorthand
when one scalar camera is sufficient; the runtime normalizes them at the input
boundary. Charts sharing both
`link_group` and `link_domain` synchronize hover, selection highlight, zoom,
and pan through weak native entity links without Rhai event fan-out.
Cartesian links carry explicit region/axis logical windows. Geo links carry a
normalized camera plus map/projection identity; only compatible Geo regions
consume them. Polar and incompatible coordinate bindings do not silently reuse
Cartesian axis metadata. The LinkRegistry retains the latest sourced projection
with its group/domain and globally unique commit identity, so target views
preserve it across redraw and suspend/resume without aliasing another group.
Each Cartesian axis window enters the coordinate plan independently; it is
never collapsed into one shared zoom scalar.

Viewport gestures maintain a transient native preview. Each `zoom_change`
payload includes the exact typed `viewport` and a monotonic
`viewport_revision`; after accepting, clamping, or rejecting that proposal,
the Host writes the same revision back together with its controlled viewport.
Unrelated renders do not
acknowledge or reset a pending preview, while a same-value write with the new
revision explicitly rejects it. For example:

```rhai
fn zoomed(ctx, proposal) {
    ctx.set_state("viewport", proposal.viewport); // accept exact windows/camera
    ctx.set_state("viewport_revision", proposal.viewport_revision);
}
```

A Cartesian viewport is
`#{kind:"cartesian", region, x:#{key,min,max}, y:#{key,min,max}}` (either axis
may be omitted). A Geo viewport is
`#{kind:"geo", region, map, projection, zoom, pan_x, pan_y}` where pan is
normalized to the plot size. Keeping the typed value is required when linked
charts have independent X/Y windows that cannot be represented by one scalar
zoom/pan tuple.

Cartesian zoom compiles a visible data-domain window and regenerates both
scales and ticks. Explicit Started/Moved/Ended gestures commit only at Ended;
the short idle timer is reserved for platform input that supplies no reliable
end phase.

The runtime distinguishes requested input, prepared data, a layout candidate,
and the presented frame. `DataKey` includes source identity and data revision;
`FrameKey` additionally changes for viewport, theme, selection, spec, and size.
Only successful foreground installation advances the presented key. A prepared
revision or cancelled task is never treated as proof that the requested frame
is visible; accessibility exposes both data revision and presented frame epoch.

Chart titles consume the theme `title` typography role; axes, legends, values,
and tooltips consume `body_small`. Native labels and SVG/PNG export share the
resolved family, fallback stack, size, line height, and weight. Plot margins
grow with the resolved line box instead of assuming a permanent 12px font.

Chart transitions use one aggregated chart resource and the existing Motion
theme duration/easing plus Host normal/reduced/none policy. Compatible keyed
rectangles, circles, polylines, and polygons interpolate; entering lines draw by
path progress and removed marks fade. A chart does not register thousands of
global Motion sources. View suspension shifts the aggregated transition time
origin, so the first resumed frame equals the frozen frame and suspended wall
time is never consumed as animation progress.

## Axes, time, formatting, and locale

Axes support auto/linear/log/category/time scales, explicit normal/reversed
direction, position, domain, title, and declarative formatting. RTL changes
text and legend layout, never axis direction implicitly.

One `(region, axis key)` compiles exactly one shared scale from every bound
series. Stacked bars contribute positive and negative interval endpoints to the
domain and distinct stack groups occupy distinct band slots. Gauge uses its
radial-axis domain; Radar aligns stable indicators and uses one shared domain.
For an unqualified Cartesian annotation, each channel binds to the first
declared axis in that region that has non-empty visible series. Axis IDs and
series iteration order therefore do not choose annotation mathematics; an
empty axis group falls through to the next declared active axis. X and Y scales
are compiled independently, so an annotation does not require a real series to
reference that exact axis pair.

Axis type/domain inference is cached once per scene and includes preserved
empty schemas. Series geometry, ticks, annotations, and custom-series contexts
therefore cannot infer different mappers for the same axis ID.

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
series renderer. It receives typed data, custom options, theme, and a typed
Cartesian, Polar, or Geo coordinate context. Returned marks declare a role,
region, and optional `ChartDatumRef`; identity, geometry, coordinate ownership,
and budgets are validated before commit. Duplicate registration is atomic and
never replaces the installed implementation.

Host-only `export_chart_svg` and `export_chart_png` render an explicit terminal
scene with bounded size, complete locale context, optional acknowledged
viewport/selection, data revision, and registered geo sources. PNG uses the
same system-font environment as the SVG adapter, including CJK fallback.

The retained figure's accessibility projection includes the presented revision,
summary, stable active datum, selected data outside the draw sample, and bounded
counts. Keyboard focus is retained by mark identity rather than array index.

## Performance contract

The hard envelope is 10,000 interactive marks, 200,000 total marks,
2,000,000 prepared vertices, or 100,000
Rust-downsampled/streaming points on the reference macOS environment. The
`chart_gallery` example includes all built-in series and a 100,000-row
`NativeChartData` line series:

```text
cargo run --release -p gpui-rhai --features charts --example chart_gallery
```

Graph, Tree, Treemap, Sunburst, Sankey, freehand lasso, PDF/video/animated SVG,
editing marks to mutate data, 3D, and Globe are explicit later work rather than
silent partial implementations.
