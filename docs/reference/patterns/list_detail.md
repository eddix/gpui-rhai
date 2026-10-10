# ListDetail

`patterns/list_detail` · export `ListDetail` · version 0.2.0. Generated from
[`registry/patterns/list_detail.rhai`](../../../registry/patterns/list_detail.rhai); do not edit.

ListDetail shows a list and the selected item's detail, side by side or stacked.

The list keeps a fixed width (or height when stacked); the detail fills. One hairline separates them.

```rhai
import "patterns/list_detail" as list_detail;

list_detail::ListDetail(#{ label: "Hosts", list: host_list, detail: host_detail })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `detail` | node | required | The selected item's detail; it fills the remaining space. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the group holding both panes. |
| `list` | node | required | The list pane; it keeps a fixed width, or a fixed height when stacked. |
| `list_size` | length or `()` | — | List width, or height when vertical; defaults to 320px wide or 8 `metrics.row` tall. |
| `orientation` | `"horizontal"` or `"vertical"` | `"horizontal"` | `horizontal` puts the list beside the detail; `vertical` stacks it above. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `detail` | yes | no | The detail pane that fills. |
| `list` | yes | no | The list pane. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `detail`, `list`, `root`.

## Theme

- Tokens: `border`, `metrics.row`
