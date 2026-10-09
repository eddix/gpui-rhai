# ToggleGroup

`components/toggle_group` · export `ToggleGroup` · version 0.2.0. Generated from
[`registry/components/toggle_group.rhai`](../../../registry/components/toggle_group.rhai); do not edit.

ToggleGroup holds pressed tool state: equal-height segments joined inside one frame.

State: caller owns values; component owns only the roving active item.

Keyboard: one tab stop on the active segment; arrows move focus between enabled segments; Enter or Space toggles the focused segment.

```rhai
import "components/toggle_group" as toggle_group;

toggle_group::ToggleGroup(#{ key: "align", label: "Alignment", values: ["left"], items: items, on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `allow_empty` | bool | `true` | In `single` mode, whether clicking the pressed segment unpresses it; `multiple` mode ignores it. |
| `disabled` | bool | `false` | Disables every segment and the arrow-key navigation. |
| `items` | array of object (at most 64) | required | Segments in display order. |
| `key` | string | required | Identity of the group and prefix of its segment keys (`<key>-<value>`), which arrow keys focus. |
| `label` | string | required | Accessible name of the group. |
| `mode` | `"single"` or `"multiple"` | `"single"` | With `single`, one segment is pressed at a time; with `multiple`, each segment toggles on its own. |
| `on_change` | callback or `()` | — | Called with the next `values` array when a segment is clicked or activated from the keyboard. |
| `orientation` | `"horizontal"` or `"vertical"` | `"horizontal"` | Joins segments in a row or a column; arrow keys follow it (left and right, or up and down). |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size; leave unset to inherit the environment size. |
| `style` | style | — | Style merged over the root part. |
| `values` | array of string (at most 64) | required | Pressed segment values, at most one in `single` mode; the caller stores the `change` payload and passes it back. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Blocks clicks on this segment; arrow keys skip it. |
| `label` | string | required | Segment text; also its accessible name. |
| `value` | string | required | Value this segment adds to or removes from `values`. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | array of string (at most 64) | Emitted when a segment is clicked or activated; the payload is the full next `values` array. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `item`, `root`.

## Theme

- Tokens: `border`, `disabled`, `focus_ring`, `metrics.control`, `metrics.control_pad`, `radius.lg`, `radius.md`, `selection`, `surface_hover`, `text_muted`, `text_primary`, `typography.control`
- Environment: `corners`, `density`, `size`
