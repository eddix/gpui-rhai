# InputGroup

`components/input_group` · export `InputGroup` · version 0.2.0. Generated from
[`registry/components/input_group.rhai`](../../../registry/components/input_group.rhai); do not edit.

Composes an input-like control with integrated prefix and suffix content.

```rhai
import "components/input_group" as input_group;

input_group::InputGroup(#{ label: "Search", prefix: search_icon, control: input_node })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `control` | node | required | The input-like node to frame, usually an `Input`; it loses its own frame and keeps its own editing events. |
| `disabled` | bool | `false` | Disables and dims the whole group; the control inside is disabled with it. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the group; it is not drawn. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `prefix` | node or `()` | — | Leading content inside the frame, such as a search icon, in `text_muted`. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size of the group and the control inside; unset, it inherits the `size` environment. |
| `style` | style | — | Style merged over the root part. |
| `suffix` | node or `()` | — | Trailing content inside the frame, such as a unit or a clear button, in `text_muted`. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `control` | yes | no | The framed control, between `prefix` and `suffix`. |
| `prefix` | no | no | Content before the control. |
| `suffix` | no | no | Content after the control. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `control`, `prefix`, `root`, `suffix`.

## Theme

- Tokens: `border`, `focus_ring`, `metrics.control`, `metrics.field_pad`, `radius.md`, `surface_raised`, `text_muted`, `typography.control`
- Environment: `corners`, `density`, `size`
