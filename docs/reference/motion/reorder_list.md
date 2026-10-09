# ReorderList

`motion/reorder_list` · export `ReorderList` · version 0.1.6. Generated from
[`registry/motion/reorder_list.rhai`](../../../registry/motion/reorder_list.rhai); do not edit.

ReorderList slides its rows to their new places when the caller reorders `items`. State: controlled order. Emits no events.

```rhai
import "motion/reorder_list" as reorder_list;

reorder_list::ReorderList(#{ key: "queue", items: [#{ key: "a", label: "A" }] })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `items` | array of object (at most 256) | required | Rows in display order; reorder the array and each row slides to its new place. |
| `key` | string | required | Motion identity of the list; keep it stable so rows animate between renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `key` | string | required | Row identity that its motion follows; unique within the list. |
| `label` | string | required | Row text. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `item`, `root`.
