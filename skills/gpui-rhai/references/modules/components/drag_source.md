# DragSource

`components/drag_source` · export `DragSource` · version 0.2.0. Generated from
[`registry/components/drag_source.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/drag_source.rhai); do not edit.

Typed in-application data drag source. It does not move the source object. State: Host-domain native drag session only; application data remains controlled.

```rhai
import "components/drag_source" as drag_source;

drag_source::DragSource(#{ key:"server", label:"Move server", source_id:"server-a",
    payload_type:"server", payload:#{id:"a"}, content:text("Server A") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `content` | node | required | Node the user presses and drags. |
| `description` | string or `()` | — | Optional accessible description of the item or move gesture, attached to the actual focused source. |
| `disabled` | bool | `false` | Ignores pointer and keyboard input and removes the source from the tab order. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `keyboard_target` | string or `()` | — | `target_id` of the zone that Enter or Space drops onto; without it the keyboard cannot drop. |
| `label` | string | required | Accessible name of the drag source button. |
| `on_drag_end` | callback or `()` | — | Called with `{accepted, target_id, operation, cancelled}` when a drag or keyboard drop ends. |
| `on_drag_start` | callback or `()` | — | Called once when a pointer drag crosses its threshold, not on a tap or keyboard drop. |
| `operation` | `"copy"` or `"move"` | `"move"` | What the drop means; only zones listing it in `operations` accept the drag. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `payload` | any value | required | Data handed unchanged to the receiving zone's `drop` event. |
| `payload_type` | string | required | Type name, 1 to 128 characters, that drop zones match against their `payload_types`. |
| `source_id` | string | required | Identity of the dragged item, 1 to 128 characters; drop zones receive it as `source_id`. |
| `source_ref` | element ref or `()` | — | Caller-owned ref to the native focusable drag source, for returning focus after a custom keyboard flow. |
| `style` | style | — | Style merged over the root part. |
| `threshold` | number 0–64 | `4.0` | Pointer travel in logical pixels before a press becomes a drag. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `drag_end` | `on_drag_end` | object | Emitted when a drag past the threshold ends or is cancelled, or a keyboard drop runs; the payload is the outcome. |
| `drag_start` | `on_drag_start` | none | Emitted when a pointer gesture becomes an active application drag. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `interaction`, `root`.

## Theme

- Tokens: `accent`, `focus_ring`
