# MemoryPanel

`patterns/memory_panel` · export `MemoryPanel` · version 0.1.0. Generated from
[`registry/patterns/memory_panel.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/memory_panel.rhai); do not edit.

Edit an explicit Host memory snapshot. The panel does not discover, persist or use memories.

```rhai
import "patterns/memory_panel" as memory_panel;

memory_panel::MemoryPanel(#{key:"memory",workspace:"profile",memories:[#{id:"tea",text:"Prefers tea.",category:"Preferences"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `action_label` | string or `()` | — | Optional unobtrusive footer action label. |
| `added_date` | string or `()` | — | Optional caller-formatted date for new items. |
| `added_source` | string or `()` | — | Source caption for a new item; defaults to You added. |
| `categories` | array of string (at most 32) | `[]` | Category ordering and extra choices for new memories. |
| `controlled` | bool or `()` | — | Override snapshot ownership; by default on_change makes memories controlled. |
| `default_enabled` | bool | `true` | Initial local toggle. |
| `enabled` | bool or `()` | — | Controlled use-memory toggle when present. |
| `key` | string | required | Stable panel instance. |
| `max_height` | integer 96–2000 | `360` | Visible list height before native vertical scrolling. |
| `memories` | array of object (at most 128) | required | Accepted snapshot when controlled; otherwise the local seed. |
| `on_action` | callback or `()` | — | Optional footer action. |
| `on_change` | callback or `()` | — | Receives the operation, entry, revision and full resulting list. |
| `on_enabled_change` | callback or `()` | — | Use-memory toggle proposal. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | `0` | Accepted Host revision; increment to acknowledge or reject a controlled proposal. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Defaults to Memory. |
| `undo_ms` | integer 250–60000 | `5000` | Time to offer Undo after a confirmed removal. |
| `workspace` | string | required | Identity of the displayed memory workspace. |

### `memories[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `category` | string | required | Group name. |
| `date` | string or `()` | — | Caller-formatted date caption. |
| `id` | string | required | Unique stable memory ID. |
| `paused` | bool | `false` | Retained but not used, according to the Host. |
| `source` | string or `()` | — | Visible provenance label. |
| `text` | string | required | Short memory value; at most 2,048 graphemes and 8,192 bytes. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Named footer action for this snapshot. |
| `change` | `on_change` | object | A revisioned add, edit, pause, forget or restore proposal. Payload memory text is sensitive. |
| `enabled_change` | `on_enabled_change` | object | Toggle proposal without modifying records. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `actions`, `bottom`, `caption`, `card`, `chip`, `chip_count`, `chip_selected`, `chips`, `count`, `danger`, `edit_buttons`, `editor`, `editor_field`, `empty`, `fact`, `fact_paused`, `filters`, `filters_off`, `foot`, `group`, `head`, `hot`, `icon`, `item`, `knob`, `list`, `list_off`, `meta`, `metadata`, `metadata_separator`, `new`, `notice`, `paused`, `plain`, `root`, `search`, `search_field`, `solid`, `switch`, `switch_label`, `title`, `track`, `undo`.

## Theme

- Tokens: `border`, `design.name`, `focus_ring`, `memory_panel.card`, `memory_panel.danger`, `memory_panel.danger_fill`, `memory_panel.fill`, `memory_panel.hover`, `memory_panel.ink`, `memory_panel.knob`, `memory_panel.muted`, `memory_panel.on_solid`, `memory_panel.paused`, `memory_panel.paused_fill`, `memory_panel.solid`, `memory_panel.text`, `memory_panel.track`, `memory_panel.track_on`, `memory_panel.tray`, `memory_panel.well`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/input`](../components/input.md), [`components/textarea`](../components/textarea.md)
