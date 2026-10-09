# DataView

`patterns/data_view` · export `DataView` · version 0.2.0. Generated from
[`registry/patterns/data_view.rhai`](../../../registry/patterns/data_view.rhai); do not edit.

DataView is a browsable collection in one region: toolbar, table or list, and a footer with status, selection, count and data time.

Loading, empty and error states replace the body in place; stale and refreshing keep the data and add one quiet line above it.

```rhai
import "patterns/data_view" as data_view;

data_view::DataView(#{ key: "hosts", label: "Hosts", title: "Hosts", body: table_node,
    toolbar: #{ filters: [filter_input], primary: add_button },
    footer: #{ count: "128 hosts", updated: "Updated 12:04" } })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `bleed` | bool | `true` | Lets the body reach the sides; right for a Table or List, whose rows carry their own inset. |
| `body` | node | required | The collection, usually a Table or List; it fills the region. |
| `footer` | object or `()` | — | Footer fields: `status` and `selection` at the start, `count` and `updated` at the end. |
| `inset` | bool | `true` | Region's `inset`: pads the view by `metrics.inset` around its parts. |
| `key` | string | required | Stable identity of the view; the `state` feedback is keyed `<key>-state`. |
| `label` | string | required | Accessible name of the region; the toolbar is named `<label> toolbar`. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `state` | object or `()` | — | InlineState props without `key`; shown instead of the body, or above it when stale or refreshing. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Region title, drawn as its heading. |
| `toolbar` | object or `()` | — | Slots of a `Toolbar` drawn between the title and the body. |

### `footer` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `count` | string or `()` | — | Item count at the end, with tabular digits. |
| `selection` | string or `()` | — | What is selected, after `status`, in the primary text color. |
| `status` | node or `()` | — | A status node, such as a Badge, first at the start. |
| `updated` | string or `()` | — | Data time after `count`, with tabular digits. |

### `state` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of node (at most 4) | `[]` | Buttons after the text, such as Retry; not shown while loading. |
| `description` | string or `()` | — | A muted second line, such as the next step; not shown while loading. |
| `detail` | string or `()` | — | Raw error text, shown selectable for `error`, `stale` and `refreshing`. |
| `state` | `"loading"` or `"empty"` or `"error"` or `"stale"` or `"refreshing"` | required | Which feedback to show; `stale` and `refreshing` keep the body below the line. |
| `title` | string | required | What is loading, why the collection is empty, or what failed. |

### `toolbar` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of node (at most 16) | `[]` | Secondary actions in the end group, before `primary`. |
| `context` | array of node (at most 16) | `[]` | What the bar acts on (tags, a breadcrumb), first in the start group. |
| `fill` | node or `()` | — | One search or query field that takes the width between the groups. |
| `filters` | array of node (at most 16) | `[]` | Controls that narrow the collection, after `context` in the start group. |
| `primary` | node or `()` | — | The one solid action of the bar, last in the end group. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size passed to every control in the bar. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `body` | yes | no | The collection that fills the region. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`.

## Theme

- Tokens: `space.group`, `space.related`, `text_muted`, `text_primary`

## Dependencies

[`layouts/region`](../layouts/region.md), [`layouts/toolbar`](../layouts/toolbar.md), [`patterns/inline_state`](../patterns/inline_state.md)
