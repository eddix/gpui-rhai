# Collapsible

`components/collapsible` · export `Collapsible` · version 0.2.0. Generated from
[`registry/components/collapsible.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/collapsible.rhai); do not edit.

Controlled disclosure with trigger/content slots and Rust clip animation. State: controlled open state.

```rhai
import "components/collapsible" as collapsible;

collapsible::Collapsible(#{ key: "details", open: true, trigger: text("Details"), content: text("Body") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `content` | node | required | Body shown under the header while open. |
| `content_height` | integer ≥ 0 or length or `()` | — | Open panel height: logical pixels, or a Length resolved in this subtree's density; omitted, the panel fits its content. |
| `disabled` | bool | `false` | Whether the header ignores presses. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `on_open_change` | callback or `()` | — | Called with the opposite of `open` when the header is pressed. |
| `open` | bool | required | Whether the content is shown; the caller stores the `open_change` payload and passes it back. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `trigger` | node | required | Header content after the chevron; the whole header row toggles the panel. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `open_change` | `on_open_change` | bool | Emitted when the header is pressed; the payload is the requested open state. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `content` | yes | no | Body shown under the header while open. |
| `trigger` | yes | no | Header content after the chevron. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `focus_frame`, `indicator`, `root`, `trigger`.

## Theme

- Tokens: `disabled`, `focus_ring`, `metrics.icon`, `metrics.inset`, `metrics.row`, `spacing.xs`, `surface_hover`, `text_muted`, `text_primary`, `typography.body`
- Environment: `density`
