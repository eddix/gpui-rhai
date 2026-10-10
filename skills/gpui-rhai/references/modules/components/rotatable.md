# Rotatable

`components/rotatable` · export `Rotatable` · version 0.2.0. Generated from
[`registry/components/rotatable.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/rotatable.rhai); do not edit.

Controlled Canvas rotation around an explicit local pivot. State: caller owns angle.

```rhai
import "components/rotatable" as rotatable;

rotatable::Rotatable(#{key:"dial",label:"Rotate dial",angle:0.0,pivot:#{x:100.0,y:100.0},content:canvas_node,on_rotate:Fn("changed")})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `angle` | number | required | Clockwise rotation in degrees; the caller stores the `rotate` payload and passes it back. |
| `content` | node | required | Node that rotates; it fills the component. |
| `disabled` | bool | `false` | Ignores pointer and keyboard input and removes the control from the tab order. |
| `handle` | node or `()` | — | Grip in the top-right corner that alone starts a rotation drag; without it the whole content does. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `keyboard_step` | number 0.1–180 | `5.0` | Degrees one arrow key turns, Right/Up clockwise; Shift turns four times as far and Home returns to 0. |
| `label` | string | required | Accessible name of the rotation slider. |
| `on_rotate` | callback or `()` | — | Called with the proposed angle in degrees when a rotation drag ends or a key turns the content. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pivot` | object | required | Point the content turns around, in logical pixels from the content's top-left corner. |
| `snap` | number > 0, ≤ 360 or `()` | — | Angle step in degrees that proposed angles round to; it also replaces `keyboard_step`. |
| `style` | style | — | Style merged over the root part. |
| `threshold` | number 0–64 | `4.0` | Pointer travel in logical pixels before a press starts rotating. |

### `pivot` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `x` | number -1000000–1000000 | required | Pivot distance from the content's left edge, in logical pixels. |
| `y` | number -1000000–1000000 | required | Pivot distance from the content's top edge, in logical pixels. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `rotate` | `on_rotate` | number | Emitted when a rotation drag ends or a key turns the content; the payload is the next angle in [0, 360). |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `content` | yes | no | The rotated content. |
| `handle` | no | no | The rotation grip in the top-right corner. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `handle`, `interaction`, `root`.

## Theme

- Tokens: `border`, `focus_ring`, `metrics.control`, `surface`
