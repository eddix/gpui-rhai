# PanZoom

`components/pan_zoom` · export `PanZoom` · version 0.2.0. Generated from
[`registry/components/pan_zoom.rhai`](../../../registry/components/pan_zoom.rhai); do not edit.

Controlled Canvas viewport transform with native pan and pointer-anchored zoom. State: transform is caller-owned; native signals contain only current preview.

```rhai
import "components/pan_zoom" as pan_zoom;

pan_zoom::PanZoom(#{ key:"map", label:"Map", transform:viewport, content:canvas_node, on_transform_change:Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `axes` | `"both"` or `"horizontal"` or `"vertical"` | `"both"` | Axes that dragging and arrow keys pan along; zooming is not limited by it. |
| `content` | node | required | Node that is panned and zoomed; it fills the viewport. |
| `disabled` | bool | `false` | Ignores pointer, wheel and keyboard input and removes the viewport from the tab order. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `keyboard_pan_step` | number 0.1–512 | `16.0` | Logical pixels one arrow-key press pans; Shift pans four times as far. |
| `keyboard_zoom_factor` | number 1.001–4 | `1.2` | Zoom multiplier of one `+` or `-` press, about the viewport centre; `0` resets to scale 1 at the origin. |
| `label` | string | required | Accessible name of the viewport region. |
| `max_scale` | number > 0, ≤ 1000000 | `8.0` | Largest zoom factor a wheel or key zoom may propose; at least `min_scale`. |
| `min_scale` | number > 0 | `0.25` | Smallest zoom factor a wheel or key zoom may propose. |
| `on_transform_change` | callback or `()` | — | Called with the proposed `{x, y, scale}` when a pan ends, a wheel zoom settles or a key pans or zooms. |
| `pan_button` | `"left"` or `"middle"` | `"left"` | Mouse button that pans when dragged. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `threshold` | number 0–64 | `4.0` | Pointer travel in logical pixels before a press becomes a pan. |
| `transform` | object | required | Content offset and zoom; the caller stores the `transform_change` payload and passes it back. |
| `wheel_zoom` | `"off"` or `"modifier"` or `"always"` | `"modifier"` | When the wheel zooms about the pointer: never, only with Ctrl or Cmd held, or always. |

### `transform` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `scale` | number > 0 | required | Zoom factor about the viewport centre, 1 for natural size; within `min_scale` and `max_scale`. |
| `x` | number -1000000–1000000 | required | Horizontal offset of the content, in logical pixels. |
| `y` | number -1000000–1000000 | required | Vertical offset of the content, in logical pixels. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `transform_change` | `on_transform_change` | object | Emitted when a pan ends, a wheel zoom settles or a key pans or zooms; the payload is the next `transform`. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `content` | yes | no | The panned and zoomed content. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `interaction`, `root`.

## Theme

- Tokens: `border`, `focus_ring`, `surface`
