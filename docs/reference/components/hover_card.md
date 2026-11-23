# HoverCard

`components/hover_card` · export `HoverCard` · version 0.4.3. Generated from
[`registry/components/hover_card.rhai`](../../../registry/components/hover_card.rhai); do not edit.

A non-modal, interactive preview that opens on intentional hover or keyboard focus.

```rhai
import "components/hover_card" as hover_card;

hover_card::HoverCard(#{key:"service",label:"Service details",trigger:text("Checkout"),content:text("Owned by Platform")})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `align` | `"start"` or `"center"` or `"end"` | `"start"` | Cross-axis alignment. On horizontal edges start/end follow the reading direction. |
| `content` | node | required | The preview body. It may contain text, links, and keyboard-focusable controls. |
| `disabled` | bool | `false` | Renders the trigger alone without starting or showing a preview. |
| `gap` | number 0–64 | `8.0` | Space between the anchor and preview in logical pixels. |
| `hide_delay_ms` | integer 0–60000 | `300` | Hide delay after leaving the trigger, panel and crossing corridor. |
| `key` | string | required | Stable preview anchor identity. |
| `label` | string | required | Accessible name of the preview content. |
| `on_open_change` | callback or `()` | — | Receives a bool proposal. A controlled caller accepts by passing it back in open. |
| `open` | bool or `()` | — | Controlled visibility. Omit it to let the component own local visibility. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placement` | `"bottom"` or `"top"` or `"left"` or `"right"` | `"bottom"` | Preferred side; placement flips when there is no room. |
| `show_delay_ms` | integer 0–60000 | `600` | Dwell delay for pointer hover or keyboard focus, in milliseconds. |
| `style` | style | — | Style merged over the root part. |
| `trigger` | node | required | Usually a labeled button or link. Its own press action is kept; a press dismisses rather than opens the preview. |
| `width` | length or `()` | — | Optional requested width. By default the card fits its content up to 320 logical pixels at reading scale 100. The viewport still caps it. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `open_change` | `on_open_change` | bool | Requested visibility after native hover, focus or dismissal. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `content` | yes | no | The interactive overlay body. |
| `trigger` | yes | no | Visible focusable anchor. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `root`, `trigger`.

## Theme

- Tokens: `border`, `space.spacing_unit`, `space.unit`, `surface_raised`, `text_primary`, `typography.hover_card`
- Environment: `density`
