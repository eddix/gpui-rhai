# DescriptionList

`patterns/description_list` · export `DescriptionList` · version 0.2.0. Generated from
[`registry/patterns/description_list.rhai`](../../../registry/patterns/description_list.rhai); do not edit.

DescriptionList shows facts as label and value pairs on one label column.

Labels are muted data, values are primary data; numbers and identifiers use tabular figures and the slashed zero. Rows are metrics.row high so lists align with tables.

```rhai
import "patterns/description_list" as description_list;

description_list::DescriptionList(#{ label: "Host", items: [#{ label: "Region", value: "us-east" },
    #{ label: "Cores", value: "32", numeric: true }] })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `items` | array of object (at most 256) | required | Term and value pairs, in display order. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the list. |
| `label_width` | length or `()` | — | Width of the term column in the `columns` layout; `()` uses the `metrics.label_column` token. |
| `layout` | `"columns"` or `"stacked"` | `"columns"` | `columns` puts each term beside its value on one row; `stacked` puts the term above the value. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `identifier` | bool | `false` | Treats the value as an atom: tabular figures, slashed zero, one line truncated with an ellipsis. |
| `label` | string | required | The term, in the muted color; also the accessible name of its pair. |
| `numeric` | bool | `false` | Sets the value in tabular figures with the slashed zero. |
| `value` | string or node | required | The fact: text in the primary color, or a node that gets the `value` part style. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `item`, `root`, `term`, `value`.

## Theme

- Tokens: `metrics.label_column`, `metrics.row`, `space.group`, `space.related`, `space.unit`, `text_muted`, `text_primary`, `typography.body`
- Environment: `density`
