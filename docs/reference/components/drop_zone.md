# DropZone

`components/drop_zone` · export `DropZone` · version 0.4.3. Generated from
[`registry/components/drop_zone.rhai`](../../../registry/components/drop_zone.rhai); do not edit.

Typed same-Host application drop target; with `os.files`, also files and folders the operating system drags in, when the Host accepts file drops. Cross-window drops are excluded. State: frame-local native target eligibility only; application ownership remains controlled.

```rhai
import "components/drop_zone" as drop_zone;

drop_zone::DropZone(#{ key:"lane", label:"Server lane", target_id:"lane-a",
        payload_types:["server"], operations:["move"], content:text("Lane A") })
    DropZone(#{ key:"import", label:"Import", target_id:"import",
        payload_types:["os.files"], operations:["copy"], content:text("Drop files"), on_drop:Fn("imported") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `content` | node | required | Node that marks the drop area; the zone covers it. |
| `disabled` | bool | `false` | Stops the zone from accepting drops. |
| `file_kinds` | array of `"file"` or `"dir"` (at most 2) or `()` | — | With `os.files`, the kinds a file drag may hold: `file`, `dir` or both, the default. A drag holding another kind cannot drop. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the drop area. |
| `on_drop` | callback or `()` | — | Called with the `drop` payload when an accepted drag is released over the zone or dropped by keyboard. |
| `operations` | array of `"copy"` or `"move"` (at most 2) | required | Source `operation` values this zone accepts; at least one. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `payload_types` | array of string (at most 32) | required | Source `payload_type` values this zone accepts; at least one. `os.files` takes files and folders the operating system drags in, when the Host calls `ScriptView::accept_file_drops(true)`. |
| `priority` | integer | `0` | Picks the zone where eligible zones overlap: higher wins, then the smaller zone, then the one painted later. |
| `style` | style | — | Style merged over the root part. |
| `target_id` | string | required | Identity of this zone, 1 to 128 characters; `drop`, the source's `drag_end` and `keyboard_target` use it. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `drop` | `on_drop` | object | Emitted when an accepted drag is released over the zone or a keyboard drop names it; the payload carries the drag. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `interaction`, `root`.

## Theme

- Tokens: `accent`, `danger`
