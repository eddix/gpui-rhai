# Accordion

`components/accordion` · export `Accordion` · version 0.2.0. Generated from
[`registry/components/accordion.rhai`](../../../registry/components/accordion.rhai); do not edit.

Controlled single/multiple accordion with Rust-side clip motion. State: controlled expanded values.

```rhai
import "components/accordion" as accordion;

accordion::Accordion(#{ key: "faq", items: items, expanded: [], on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Whether every header ignores presses. |
| `expanded` | array of string (at most 256) | required | Keys of the open items; the caller stores the `change` payload and passes it back. |
| `items` | array of object (at most 256) | required | The sections, in order. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `mode` | `"single"` or `"multiple"` | `"single"` | With `single` at most one item is open; with `multiple` items open and close independently. |
| `on_change` | callback or `()` | — | Called with the next `expanded` keys when a header is pressed. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `content` | node | required | Panel shown under the header while the item is open. |
| `content_height` | integer ≥ 0 or length or `()` | — | Open panel height: logical pixels, or a Length resolved in this subtree's density; omitted, the panel fits its content. |
| `disabled` | bool | `false` | Whether this item's header ignores presses. |
| `key` | string | required | Identity of the item; `expanded` lists it while the item is open. |
| `title` | string | required | Header text of the item. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | array of string (at most 256) | Emitted when a header is pressed; the payload is the keys of the items open after the toggle. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `focus_frame`, `item`, `root`, `trigger`.

## Theme

- Tokens: `border`, `disabled`, `focus_ring`, `metrics.icon`, `metrics.inset`, `metrics.row`, `surface_hover`, `text_muted`, `text_primary`, `typography.body`
- Environment: `density`
