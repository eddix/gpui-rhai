# BreakdownChart

`patterns/breakdown_chart` · export `BreakdownChart` · version 0.1.0. Generated from
[`registry/patterns/breakdown_chart.rhai`](../../../registry/patterns/breakdown_chart.rhai); do not edit.

One signed measure by group and segment, with a linked summary table.

```rhai
import "patterns/breakdown_chart" as breakdown_chart;

breakdown_chart::BreakdownChart(#{key:"work",dataset:"workload",records:[#{team:"api"}],dimensions:[#{id:"team",label:"Team",levels:[#{id:"api",label:"API"}]}],measures:[#{id:"count",label:"Tasks",unit:"tasks"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `dataset` | string | required | Stable dataset identity. A different identity starts fresh local choices, visibility and attention. |
| `default_measure` | string or `()` | — | Initial measure ID; defaults to the first. |
| `default_segment` | string or `()` | — | Initial segment dimension; empty string means no segmentation. If omitted, use the next dimension that differs from the slice. |
| `default_slice` | string or `()` | — | Initial group dimension; defaults to the first. |
| `dimensions` | array of object (at most 8) | required | Dimensions available to group and segment the records. |
| `key` | string | required | Stable component key. |
| `measures` | array of object (at most 8) | required | Measures selectable by the reader. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `records` | array of any value (at most 1024) | required | Row maps keyed by dimension or measure field. Missing dimensions do not match a bucket. A missing sum field contributes zero; present measures must be finite numbers. |
| `style` | style | — | Style merged over the root part. |
| `title` | string | `"Workload"` | The card title. |

### `dimensions[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `field` | string or `()` | — | Read a different record field than this dimension's ID. |
| `id` | string | required | Dimension ID and default source field. |
| `label` | string | required | Name in the selectors and table. |
| `levels` | array of object (at most 12) | required | Ordered categories to present, including categories with no observations. |

### `dimensions[].levels[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `glyph` | string or `()` | — | Optional short decorative glyph for slice labels and rows. |
| `id` | string | required | Business value in a record, in display order. |
| `ink` | string or `()` | — | Optional theme ink token when used as a segment. |
| `label` | string | required | Displayed category label; it need not equal the business ID. |

### `measures[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `field` | string or `()` | — | Numeric field to sum. Omit it to count matching records. |
| `id` | string | required | Stable measure ID. |
| `label` | string | required | Name in the measure selector and table. |
| `unit` | string | required | Optional unit text after the total; use an empty string for no unit. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `axis`, `bar`, `cell`, `cell_header`, `cell_muted`, `cell_total`, `chart`, `column`, `controls`, `empty`, `footer`, `footer_narrow`, `gridline`, `heading`, `heading_narrow`, `legend`, `legend_chip`, `legend_chip_narrow`, `legend_hidden`, `legend_hint`, `legend_narrow`, `legend_reset`, `panel`, `picker`, `picker_label`, `plot`, `plot_frame`, `root`, `row_active`, `segment`, `swatch`, `table`, `table_header`, `table_row`, `tick`, `title`, `tooltip`, `tooltip_item`, `tooltip_title`, `tooltip_value`, `total`, `track`, `unit`, `x_active`, `x_axis`, `zero`.

## Theme

- Tokens: `border`, `breakdown_chart.c1`, `breakdown_chart.c2`, `breakdown_chart.c3`, `breakdown_chart.c4`, `breakdown_chart.c5`, `breakdown_chart.c6`, `breakdown_chart.c7`, `breakdown_chart.grid`, `breakdown_chart.muted`, `breakdown_chart.rest`, `breakdown_chart.zero`, `focus_ring`, `radius.lg`, `radius.pill`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `typography.body`, `typography.caption`, `typography.label`

## Dependencies

[`components/select`](../components/select.md), [`patterns/agent_tree`](../patterns/agent_tree.md), [`patterns/compact_breakdown`](../patterns/compact_breakdown.md)
