# Chart

`charts/chart` · export `Chart` · version 0.1.7. Generated from
[`registry/charts/chart.rhai`](../../../registry/charts/chart.rhai); do not edit.

Native composable visualization surface. Small arrays are converted once to typed columns; NativeChartData keeps streaming and large data in Rust. State is controlled by the caller.

```rhai
import "charts/chart" as chart;

chart::Chart(#{ key: "sales", data: rows, key_dimension: "id", spec: spec })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `data` | array of any value (at most 10000) or chart data | required | Row maps, converted once to typed columns, or a Host-owned `NativeChartData` handle for large or streaming data. |
| `hidden_series` | array of string (at most 1000) | `[]` | Keys of the series to hide; update it from `on_legend_change`. |
| `key` | string | required | Stable chart identity; keeps native focus, viewport preview and transitions across renders. |
| `key_dimension` | string or `()` | — | Dimension whose values identify each datum across updates, selection, focus and motion; without it updates replace every datum. |
| `on_annotation_activate` | callback or `()` | — | Called with the `annotation_activate` payload when an annotation is clicked or activated from the keyboard. |
| `on_brush_change` | callback or `()` | — | Called with the `brush_change` payload when a brush drag ends; `spec.brush` turns brushing on, and a press in the plot becomes a brush once the pointer moves more than 4 pixels. |
| `on_legend_change` | callback or `()` | — | Called with the `legend_change` payload when a legend entry is clicked. |
| `on_select` | callback or `()` | — | Called with the `select` payload when a data mark is clicked or activated with Enter or Space; with `spec.brush`, a press that moves more than 4 pixels brushes instead. |
| `on_zoom_change` | callback or `()` | — | Called with the `zoom_change` payload when a zoom or pan gesture commits. |
| `pan_x` | number | `0.0` | Horizontal pan of the scalar camera in logical pixels. |
| `pan_y` | number | `0.0` | Vertical pan of the scalar camera in logical pixels. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `selected_keys` | array of string (at most 10000) | `[]` | Datum keys drawn as selected; the chart never selects by itself, so update it from `on_select` or `on_brush_change`. |
| `spec` | any value | required | Typed chart description: `title`, `regions`, `axes`, `series`, `legend`, `brush`, `annotations` and link settings. |
| `style` | style | — | Style merged over the root part. |
| `viewport` | any value or `()` | — | Controlled typed viewport, Cartesian axis windows or a Geo camera; store the `viewport` of each `zoom_change`. |
| `viewport_revision` | integer ≥ 0 | `0` | Write back the `viewport_revision` of each `zoom_change`, with `viewport`, to accept or reject its native preview. |
| `zoom` | number 0.5–20 | `1.0` | Zoom factor of the scalar camera, a shorthand for `viewport` when one camera is enough. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `annotation_activate` | `on_annotation_activate` | object | Emitted when an annotation is clicked or activated from the keyboard; the payload is its key and label. |
| `brush_change` | `on_brush_change` | object | Emitted when a brush drag ends; the payload is the data inside the brush and the brush rectangle. |
| `legend_change` | `on_legend_change` | object | Emitted when a legend entry is clicked; the payload names the series and the visibility it asks for. |
| `select` | `on_select` | object | Emitted when a data mark is clicked or activated from the keyboard; the payload identifies the datum. |
| `zoom_change` | `on_zoom_change` | object | Emitted when a zoom or pan gesture commits; the payload is the proposed camera and viewport with its revision. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `error`, `loading`, `root`.
