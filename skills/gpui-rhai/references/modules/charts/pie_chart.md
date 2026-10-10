# PieChart

`charts/pie_chart` · export `PieChart` · version 0.1.7. Generated from
[`registry/charts/pie_chart.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/charts/pie_chart.rhai); do not edit.

Source-owned single-series Pie adapter over Chart. State is controlled by the caller. Events are forwarded by charts/chart.

```rhai
import "charts/pie_chart" as pie_chart;

pie_chart::PieChart(#{ key: "share", data: rows, key_dimension: "id", encode: #{ name: "service", value: "requests" } })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `data` | array of any value (at most 10000) or chart data | required | Row maps, converted once to typed columns, or a Host-owned `NativeChartData` handle for large or streaming data. |
| `encode` | any value or `()` | — | Channels of the generated series, such as `#{ name: "service", value: "requests" }`; required when `spec` is absent. |
| `hidden_series` | array of string (at most 1000) | `[]` | Keys of the series to hide; update it from `on_legend_change`. |
| `key` | string | required | Stable chart identity; keeps native focus, viewport preview and transitions across renders. |
| `key_dimension` | string or `()` | — | Dimension whose values identify each datum across updates, selection, focus and motion; without it updates replace every datum. |
| `on_annotation_activate` | callback or `()` | — | Called with the `charts/chart` `annotation_activate` payload when an annotation is clicked or activated from the keyboard. |
| `on_brush_change` | callback or `()` | — | Called with the `charts/chart` `brush_change` payload when a brush drag ends; `spec.brush` turns brushing on, and a press in the plot becomes a brush once the pointer moves more than 4 pixels. |
| `on_legend_change` | callback or `()` | — | Called with the `charts/chart` `legend_change` payload when a legend entry is clicked. |
| `on_select` | callback or `()` | — | Called with the `charts/chart` `select` payload when a data mark is clicked or activated with Enter or Space; with `spec.brush`, a press that moves more than 4 pixels brushes instead. |
| `on_zoom_change` | callback or `()` | — | Called with the `charts/chart` `zoom_change` payload when a zoom or pan gesture commits. |
| `pan_x` | number | `0.0` | Horizontal pan of the scalar camera in logical pixels. |
| `pan_y` | number | `0.0` | Vertical pan of the scalar camera in logical pixels. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `selected_keys` | array of string (at most 10000) | `[]` | Datum keys drawn as selected; the chart never selects by itself, so update it from `on_select` or `on_brush_change`. |
| `series_key` | string | `"series"` | Key of the generated series, as used by `hidden_series` and event payloads; ignored when `spec` is given. |
| `series_name` | string | `"Series"` | Legend name of the generated series; ignored when `spec` is given. |
| `spec` | any value or `()` | — | Full `Chart` spec used instead of `encode`, `title` and the series props; every series in it is drawn as `pie`. |
| `style` | style | — | Style merged over the root part. |
| `title` | string | — | Chart title when `spec` is absent; a blank title reserves no space. |
| `viewport` | any value or `()` | — | Controlled typed viewport, Cartesian axis windows or a Geo camera; store the `viewport` of each `zoom_change`. |
| `viewport_revision` | integer ≥ 0 | `0` | Write back the `viewport_revision` of each `zoom_change`, with `viewport`, to accept or reject its native preview. |
| `zoom` | number 0.5–20 | `1.0` | Zoom factor of the scalar camera, a shorthand for `viewport` when one camera is enough. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`.

## Dependencies

[`charts/chart`](chart.md)
