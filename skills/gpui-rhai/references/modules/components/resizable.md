# Resizable

`components/resizable` · export `Resizable` · version 0.2.0. Generated from
[`registry/components/resizable.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/resizable.rhai); do not edit.

Controlled single-element rectangle resizing inside one local boundary. State: only native pointer preview; application rectangle remains controlled.

A grip is decoration, centred on its handle and painted above the content; pressing it anywhere starts that handle's drag, and it takes `grip_<state>` from a native signal, as SplitPane's grip does. `line: false` drops the native marks.

```rhai
import "components/resizable" as resizable;

resizable::Resizable(#{ key:"card", label:"Resize card",
    rect:#{x:40.0,y:32.0,width:280.0,height:180.0}, content:card,
    on_resize:Fn("resized") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `aspect_ratio` | number > 0 or `()` | — | Width-to-height ratio to keep; an edge handle also resizes the other side, about its centre. |
| `contain` | bool | `true` | Keeps the rectangle inside the root box; while `rect` lies outside it, drags and keys do nothing. |
| `content` | node | required | Node that fills the rectangle. |
| `disabled` | bool | `false` | Ignores drags and key steps and removes the handles from the tab order. |
| `grips` | object or `()` | — | Decorative node per handle, as in `#{ se: node }`; only handles listed in `handles` show theirs. |
| `handles` | array of `"n"` or `"s"` or `"e"` or `"w"` or `"ne"` or `"nw"` or `"se"` or `"sw"` (at most 8) | `["n", "s", "e", "w", "ne", "nw", "se", "sw"]` | Edges and corners that get a resize handle, each at most once. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `keyboard_step` | number > 0, ≤ 512 | `8.0` | Logical pixels an arrow key moves the focused handle's edge; Shift moves four times as far. |
| `label` | string | required | Accessible name; each handle reads as `<label>: <handle> resize handle`. |
| `line` | bool | `true` | Paints the native marks (edge lines, corner squares); set `false` when grips draw their own. |
| `line_inset` | number 0–256 | `4.0` | How far an edge handle's native line stops short of each end, in logical pixels. |
| `max_height` | number > 0, ≤ 16384 | `16384.0` | Largest height a resize may propose, in logical pixels; at least `min_height`. |
| `max_width` | number > 0, ≤ 16384 | `16384.0` | Largest width a resize may propose, in logical pixels; at least `min_width`. |
| `min_height` | number > 0, ≤ 16384 | `24.0` | Smallest height a resize may propose, in logical pixels. |
| `min_width` | number > 0, ≤ 16384 | `24.0` | Smallest width a resize may propose, in logical pixels. |
| `on_resize` | callback or `()` | — | Called with the proposed `rect` when a handle drag ends or an arrow key is pressed. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `rect` | object | required | Position and size in the root box; the caller stores the `resize` payload and passes it back. |
| `style` | style | — | Style merged over the root part. |

### `grips` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `e` | node or `()` | — | Grip for the right edge handle. |
| `n` | node or `()` | — | Grip for the top edge handle. |
| `ne` | node or `()` | — | Grip for the top-right corner handle. |
| `nw` | node or `()` | — | Grip for the top-left corner handle. |
| `s` | node or `()` | — | Grip for the bottom edge handle. |
| `se` | node or `()` | — | Grip for the bottom-right corner handle. |
| `sw` | node or `()` | — | Grip for the bottom-left corner handle. |
| `w` | node or `()` | — | Grip for the left edge handle. |

### `rect` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `height` | number > 0 | required | Height in logical pixels. |
| `width` | number > 0 | required | Width in logical pixels. |
| `x` | number | required | Left edge, in logical pixels from the root's left edge. |
| `y` | number | required | Top edge, in logical pixels from the root's top edge. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `resize` | `on_resize` | object | Emitted when a drag ends or a key step resizes; the payload is the next `rect`, ready to store as is. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `grip`, `grip_disabled`, `grip_drag`, `grip_focus`, `grip_hover`, `handle`, `root`, `surface`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `surface_raised`
