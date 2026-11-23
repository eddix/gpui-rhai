# NotificationStack

`patterns/notification_stack` · export `NotificationStack` · version 0.1.0. Generated from
[`registry/patterns/notification_stack.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/notification_stack.rhai); do not edit.

Only the front notice is in the input and accessibility trees.

```rhai
import "patterns/notification_stack" as notification_stack;

notification_stack::NotificationStack(#{key:"notices",stack:"updates",revision:0,notices:[#{id:"release",title:"Released",text:"See what changed."}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `busy` | bool | `false` | Preserve the front and disable proposals while the Host settles. |
| `key` | string | required | Stable stack layout key. |
| `label` | string or `()` | — | Accessible stack label. |
| `notices` | array of object (at most 64) | required | Accepted notice queue, front first. Only the front notice is interactive. |
| `on_action` | callback or `()` | — | Propose the current notice's action; no built-in URL opening. |
| `on_dismiss` | callback or `()` | — | Propose moving past the front notice. The Host accepts by removing it. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Accepted snapshot revision. |
| `stack` | string | required | Stable identity of the notice queue. |
| `style` | style | — | Style merged over the root part. |

### `notices[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `action` | object or `()` | — | Optional single action. |
| `badge` | string or `()` | — | Optional short caption. |
| `id` | string | required | Unique notice ID. |
| `pattern` | `"orbit"` or `"grid"` or `"rays"` or `()` | — | Top trailing line art; defaults to orbit. |
| `text` | string | required | Notice body. |
| `title` | string | required | Notice heading. |

### `notices[].action` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `external` | bool or `()` | — | Show the external-action arrow; the Host still owns navigation. |
| `id` | string | required | Opaque action ID interpreted by the Host. |
| `label` | string | required | Action label. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Request the action on the accepted front notice. |
| `dismiss` | `on_dismiss` | object | Dismissal proposal scoped to the current front and revision. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `arrow`, `art`, `badge`, `body`, `dismiss`, `dismiss_icon`, `empty`, `panel`, `rear`, `rear_far`, `root`, `text`, `title`, `top`.

## Theme

- Tokens: `border`, `focus_ring`, `metrics.icon`, `notification_stack.art`, `notification_stack.art_dot`, `notification_stack.badge`, `notification_stack.peek`, `radius.lg`, `radius.pill`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/button`](../components/button.md), [`patterns/agent_tree`](agent_tree.md)
