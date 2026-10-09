# SelectionArea

`components/selection_area` · export `SelectionArea` · version 0.2.0. Generated from
[`registry/components/selection_area.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/selection_area.rhai); do not edit.

Controlled Canvas click/range/marquee object selection.

```rhai
import "components/selection_area" as selection_area;

selection_area::SelectionArea(#{key:"nodes",label:"Nodes",targets:targets,selected_keys:[],content:canvas_node,on_selection_change:Fn("changed")})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `active_key` | string or `()` | — | Target the keyboard is on: arrows, Home and End move from it in `targets` order, Space toggles it. |
| `anchor_key` | string or `()` | — | Target a Shift range extends from; the caller stores it from the `selection_change` payload. |
| `content` | node | required | Canvas that draws the targets; target coordinates are local to it. |
| `disabled` | bool | `false` | Ignores pointer and keyboard input and removes the area from the tab order. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the selection grid. |
| `marquee` | `"intersect"` or `"enclose"` | `"intersect"` | Whether a marquee drag selects targets it touches or only those wholly inside it. |
| `multiple` | bool | `true` | Allows several selected targets, with Shift ranges, Ctrl/Cmd toggles and additive marquees. |
| `on_selection_change` | callback or `()` | — | Called with `{selected_keys, active_key, anchor_key}` when a click, marquee or key changes the selection. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `selected_keys` | array of string (at most 10000) | required | Keys of the selected targets; the caller stores the `selection_change` payload and passes it back. |
| `style` | style | — | Style merged over the root part. |
| `targets` | array of map of any value (at most 10000) | required | Selectable objects, `#{key, x, y, width, height, disabled?}` in the content's logical pixels; later ones win overlaps. |
| `threshold` | number 0–64 | `4.0` | Pointer travel in logical pixels before a press becomes a marquee drag. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `selection_change` | `on_selection_change` | object | Emitted when a click, marquee drag or key changes the selection; the payload is the next selection state. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `content` | yes | no | The canvas the targets are drawn on. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `interaction`, `root`.

## Theme

- Tokens: `border`, `focus_ring`, `surface`
