# Draggable

`components/draggable` · export `Draggable` · version 0.2.0. Generated from
[`registry/components/draggable.rhai`](../../../registry/components/draggable.rhai); do not edit.

Controlled in-window positioning. Native pointer movement updates only optional translation signals; release proposes one final {x,y} value. State: only native pointer preview; application position remains controlled.

```rhai
import "components/draggable" as draggable;

draggable::Draggable(#{ key:"card", label:"Move card", position:#{x:0,y:0},
    handle:text("Move"), content:text("Body"), on_move:Fn("moved") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `axes` | `"both"` or `"horizontal"` or `"vertical"` | `"both"` | Axes the object moves along, by pointer and by arrow keys. |
| `contain` | bool | `true` | Keeps the object inside the root box; while `position` lies outside it, a drag does not start. |
| `content` | node | required | Body of the object, below `handle` when there is one. |
| `disabled` | bool | `false` | Ignores pointer and keyboard input and removes the object from the tab order. |
| `handle` | node or `()` | — | Node above `content` that alone starts a drag; without it the whole object does. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `keyboard_step` | number > 0, ≤ 512 | `8.0` | Logical pixels one arrow-key press moves the object; Shift moves four times as far. With a larger snap step, a press moves to the next grid line. |
| `label` | string | required | Accessible name of the movable object. |
| `on_move` | callback or `()` | — | Called with the proposed `{x, y}` when a drag ends or an arrow key moves the object. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `position` | object | required | Top-left corner of the object in the root box; the caller stores the `move` payload and passes it back. |
| `snap_x` | number > 0 or `()` | — | Grid step in logical pixels that a proposed `x` rounds to, for drags and arrow keys. |
| `snap_y` | number > 0 or `()` | — | Grid step in logical pixels that a proposed `y` rounds to, for drags and arrow keys. |
| `style` | style | — | Style merged over the root part. |
| `threshold` | number 0–64 | `4.0` | Pointer travel in logical pixels before a press becomes a drag. |

### `position` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `x` | number -1000000–1000000 | required | Distance from the root's left edge, in logical pixels. |
| `y` | number -1000000–1000000 | required | Distance from the root's top edge, in logical pixels. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `move` | `on_move` | object | Emitted when a drag ends or an arrow key moves the object; the payload is one complete next `position`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `handle`, `interaction`, `root`, `surface`.
