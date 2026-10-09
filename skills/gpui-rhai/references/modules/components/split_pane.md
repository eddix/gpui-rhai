# SplitPane

`components/split_pane` · export `SplitPane` · version 0.2.0. Generated from
[`registry/components/split_pane.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/split_pane.rhai); do not edit.

Controlled, nestable two-panel split layout with a native drag hot lane.

`size` is the controlled start-panel ratio. Pointer moves update only a native signal; `on_resize` fires once when a drag ends or a keyboard step is requested. If the Host rejects that value, the preview returns to `size`. State: only native pointer preview; application ratio and collapse remain controlled.

`handle` is decoration: it sits in a `grip` box centred on the separator and painted above both panes, and pressing anywhere on that box, overhang included, starts the drag. The keyboard steps, the separator role and the tab stop stay on the native handle. The grip takes the `grip_<state>` part while the handle is hovered, dragged, focused or disabled, from a native signal, so pointer moves never re-render Rhai. `line: false` drops the native line for a grip that draws its own.

```rhai
import "components/split_pane" as split_pane;

split_pane::SplitPane(#{ key:"main", label:"Resize panes", size:0.35,
    start:navigation, end:content, on_resize:Fn("resized") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Ignores drags and removes the separator from the tab order. |
| `end` | node | required | Content of the end pane: the trailing side, or the bottom when `vertical`. |
| `end_collapsed` | bool | `false` | Hides the end pane and the separator so the start pane fills the group; not with `start_collapsed`. |
| `end_key` | string | `"end"` | Key of the end pane's wrapper; keep it stable so the pane is not remounted. |
| `handle` | node or `()` | — | Decorative grip node centred on the separator above both panes; pressing it starts the drag. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `keyboard_step` | number > 0, ≤ 512 | `8.0` | Logical pixels one arrow-key press on the focused separator moves it. |
| `label` | string | required | Accessible name of the separator. |
| `line` | bool | `true` | Paints the native line down the separator; set `false` when `handle` draws its own. |
| `line_inset` | number 0–256 | `4.0` | How far the native line stops short of each end of the separator, in logical pixels. |
| `max_start` | number 0–16384 | `16384.0` | Largest start-pane length a drag or key step may propose, in logical pixels; at least `min_start`. |
| `min_end` | number 0–16384 | `120.0` | Smallest end-pane length a drag or key step may leave, in logical pixels. |
| `min_start` | number 0–16384 | `120.0` | Smallest start-pane length a drag or key step may propose, in logical pixels. |
| `on_resize` | callback or `()` | — | Called with the proposed start-pane ratio when a drag ends or an arrow key is pressed. |
| `orientation` | `"horizontal"` or `"vertical"` | `"horizontal"` | `horizontal` puts the panes side by side; `vertical` stacks them. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `size` | number 0–1 | required | Start-pane share of the group's length; the caller stores the `resize` payload and passes it back. |
| `start` | node | required | Content of the start pane: the leading side, or the top when `vertical`. |
| `start_collapsed` | bool | `false` | Hides the start pane and the separator so the end pane fills the group; not with `end_collapsed`. |
| `start_key` | string | `"start"` | Key of the start pane's wrapper; keep it stable so the pane is not remounted. |
| `style` | style | — | Style merged over the root part. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `resize` | `on_resize` | number 0–1 | Emitted once when a drag ends or a key step is requested; the payload is the next `size`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `end`, `grip`, `grip_disabled`, `grip_drag`, `grip_focus`, `grip_hover`, `handle`, `root`, `start`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `surface_hover`, `surface_raised`
