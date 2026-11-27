# KpiChart

`patterns/kpi_chart` · export `KpiChart` · version 0.1.0. Generated from
[`registry/patterns/kpi_chart.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/kpi_chart.rhai); do not edit.

Multi-series KPI card with a shared sample focus and honest gaps.

```rhai
import "patterns/kpi_chart" as kpi_chart;

kpi_chart::KpiChart(#{key:"latency",metric:"service/latency",revision:0,title:"Latency",value:"42 ms",samples:[#{id:"now",label:"Now"}],series:[#{id:"p50",label:"Median",values:[42]}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `default_sample` | string or `()` | — | Sample highlighted when neither the pointer nor the keyboard is reading the plot. |
| `delta` | number -1000000000000–1000000000000 or `()` | — | Signed change against a prior period. Zero shows no delta. |
| `empty_text` | string or `()` | — | Alternative message when no reading exists. |
| `footer` | string or `()` | — | Optional card footnote. |
| `format` | `"number"` or `"percent_points"` or `"percent_fraction"` | `"number"` | Numeric readout representation; percent points use 0–100 and fractions use 0–1. |
| `key` | string | required | Stable visual instance key. |
| `metric` | string | required | Stable identity of the measured metric. Change it when replacing the dataset rather than updating it. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `prefix` | string | `""` | App-defined prefix for numeric axes, readouts and change. |
| `revision` | integer ≥ 0 | required | Accepted data revision. A new revision draws the new series using the current Motion policy. |
| `samples` | array of object (at most 96) | required | Samples in chronological order. Every series is aligned to this order. |
| `series` | array of object (at most 6) | required | Bounded series for one KPI card; use Chart with native datasets for larger workloads. |
| `style` | style | — | Style merged over the root part. |
| `suffix` | string | `""` | App-defined suffix for readouts and change. Percent modes provide % by default. |
| `title` | string | required | Metric title, also used for the accessible plot name. |
| `value` | string | required | Already formatted headline value. |

### `samples[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `detail` | string or `()` | — | Long label for the tooltip and accessible readout. |
| `id` | string | required | Stable sample ID; label may repeat. |
| `label` | string | required | Short label for the horizontal axis. |

### `series[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable series ID. |
| `ink` | string or `()` | — | Optional theme color token for the series. |
| `label` | string | required | Series label displayed in the sample readout. |
| `values` | array of number -1000000000000–1000000000000 or `()` (at most 96) | required | One number or () per sample. Missing readings break the line; zero remains a reading. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `body`, `body_narrow`, `chart`, `chart_narrow`, `delta`, `delta_down`, `delta_up`, `empty`, `footer`, `footer_narrow`, `headline`, `plot`, `plot_narrow`, `root`, `swatch`, `tick`, `title`, `tooltip`, `tooltip_label`, `tooltip_row`, `tooltip_title`, `tooltip_value`, `value`, `x_axis`, `y_axis`, `y_axis_narrow`.

## Theme

- Tokens: `border`, `compact_breakdown.c4`, `compact_breakdown.c5`, `focus_ring`, `kpi_chart.first`, `kpi_chart.grid`, `kpi_chart.guide`, `kpi_chart.second`, `kpi_chart.third`, `radius.lg`, `radius.pill`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_raised`, `text.danger`, `text.success`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.display`

## Dependencies

[`patterns/agent_tree`](agent_tree.md)
