# Breadcrumb

`components/breadcrumb` · export `Breadcrumb` · version 0.4.3. Generated from
[`registry/components/breadcrumb.rhai`](../../../registry/components/breadcrumb.rhai); do not edit.

A controlled ancestry trail. Width comes from retained presentation, not from the window, a breakpoint table, or an estimate based on string length.

```rhai
import "components/breadcrumb" as breadcrumb;

breadcrumb::Breadcrumb(#{key:"route",items:[#{value:"home",label:"Home"},#{value:"jobs",label:"Jobs"}],on_navigate:Fn("go_to")})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `items` | array of object (at most 128) | required | Root-to-current ordered trail. The last entry is never an action; empty shows no controls. |
| `key` | string | required | Stable identity for the trail and its overflow. |
| `label` | string | `"Breadcrumb"` | Accessible label of the navigation region. |
| `max_items` | integer 2–32 | `4` | Maximum visible crumbs, excluding the ellipsis. A narrow container can fold more. |
| `on_navigate` | callback or `()` | — | Receives the same ancestor value for visible and folded crumbs. The caller owns navigation. |
| `overflow_label` | string | `"Earlier pages"` | Accessible name of the ellipsis and its menu. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `separator` | node or `()` | — | Decorative node used in place of the built-in separator. |
| `separator_kind` | `"chevron"` or `"slash"` | `"chevron"` | Built-in decorative separator; chevrons mirror in RTL. |
| `style` | style | — | Style merged over the root part. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Inert ancestor, skipped by keyboard navigation. |
| `icon` | asset or `image` handle or `()` | — | Decorative glyph shown inline and in overflow. |
| `label` | string | required | Full page title, including when shortened on screen. |
| `value` | string | required | Unique stable ancestor key. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `navigate` | `on_navigate` | string | Requests navigation to an enabled ancestor. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `separator` | no | no | Optional decorative replacement separator. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `current`, `icon`, `item`, `label`, `link`, `more`, `more_icon`, `popup`, `popup_icon`, `popup_item`, `root`, `separator`, `separator_icon`, `trail`.

## Theme

- Tokens: `focus_ring`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.placeholder`, `text_muted`, `text_primary`, `typography.breadcrumb`
- Environment: `density`

## Dependencies

[`components/menu`](../components/menu.md), [`components/tooltip`](../components/tooltip.md)
