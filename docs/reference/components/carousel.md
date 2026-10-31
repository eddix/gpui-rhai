# Carousel

`components/carousel` · export `Carousel` · version 0.3.1. Generated from
[`registry/components/carousel.rhai`](../../../registry/components/carousel.rhai); do not edit.

A controlled pager that shows one item at a time, for a compact place that steps through items of one kind: alerts, cards, previews. The caller owns `index`; the arrows, the dots and the keys propose another one. It never turns by itself.

`header` puts the label and the controls in a row above the item; `overlay` lays the arrows over the item's sides, shown while the item is hovered or the controls have focus, with the position below it. The position is a count (`2 / 5`), dots or nothing.

The controls are one tab stop beside the item, never around it, so keys typed in an item stay the item's: Left and Right step, Home and End go to the ends. The arrows and the dots are reached from it and are not tab stops. A new item slides in from the side it comes from (`SlidingPanel`).

```rhai
import "components/carousel" as carousel;

carousel::Carousel(#{ key: "alerts", label: "Active alerts", index: current,
    items: [#{ label: "Disk full", content: disk }, #{ label: "CPU high", content: cpu }],
    on_change: Fn("shown") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `index` | integer ≥ 0 | required | Position of the shown item, from 0; the caller stores the `change` payload and passes it back. |
| `indicator` | `"count"` or `"dots"` or `"none"` or `()` | — | How the position shows: `count` (`2 / 5`), `dots` (one per item, which a press selects) or `none`; unset, `count` for `header` and `dots` for `overlay`. |
| `items` | array of object (at most 64) | required | The items in order; at least one. |
| `key` | string | required | Stable identity of the carousel, which names its sliding panel and keeps its focus state. |
| `label` | string | required | Accessible name of the carousel and its controls; the `header` layout also shows it as the title. |
| `label_visible` | bool | `true` | Whether the `header` layout shows the label as its title; it stays the accessible name. |
| `next_label` | string | `"Next"` | Accessible name of the next arrow. |
| `on_change` | callback or `()` | — | Called with the proposed `index` when an arrow, a dot or a key steps; without it the controls are inert. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `previous_label` | string | `"Previous"` | Accessible name of the previous arrow. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Overrides the `size` environment for the carousel. |
| `style` | style | — | Style merged over the root part. |
| `variant` | `"header"` or `"overlay"` | `"header"` | `header`: the label and the controls in a row above the item. `overlay`: arrows over the item's sides, shown while it is hovered or the controls have focus, and the position below it. |
| `wrap` | bool | `false` | Whether stepping past the last item goes to the first and back; without it the arrow at an end is disabled, or left out in `overlay`. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `content` | node | required | The item, owned by the caller; only the shown one renders. |
| `label` | string | required | Name of the item: its accessible name, its dot's, and part of the controls' value. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | integer | Emitted when an arrow, a dot or a key proposes another item; the payload is its `index`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `arrows`, `controls`, `count`, `dot`, `dot_selected`, `dots`, `focus_frame`, `header`, `item`, `next`, `previous`, `root`, `stage`, `title`, `viewport`.

## Theme

- Tokens: `control.border`, `focus_ring`, `metrics.control`, `metrics.icon`, `radius.md`, `radius.sm`, `space.related`, `spacing.xs`, `spacing.xxs`, `surface_raised`, `text_muted`, `text_primary`, `typography.control`
- Environment: `density`, `size`

## Dependencies

[`components/icon`](../components/icon.md), [`components/icon_button`](../components/icon_button.md), [`components/sliding_panel`](../components/sliding_panel.md)
