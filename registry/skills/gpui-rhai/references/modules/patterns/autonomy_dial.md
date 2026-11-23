# AutonomyDial

`patterns/autonomy_dial` · export `AutonomyDial` · version 0.1.0. Generated from
[`registry/patterns/autonomy_dial.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/autonomy_dial.rhai); do not edit.

AutonomyDial groups actions under the currently accepted policy, with a three-way selector and per-action pins. State: controlled. The Host owns policy and execution, including mandatory confirmation.

```rhai
import "patterns/autonomy_dial" as autonomy_dial;

autonomy_dial::AutonomyDial(#{key:"mode",policy:"review",revision:0,level:"risky",pinned:[],actions:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of object (at most 128) | required | Actions to present, in the Host's relative order. |
| `busy` | bool | `false` | Disable proposals during a pending change. |
| `key` | string | required | Stable control identity. |
| `level` | `"always"` or `"risky"` or `"auto"` | required | Accepted policy: always ask, ask if risky, or follow the Host's auto policy. |
| `message` | string or `()` | — | Optional Host result. |
| `on_change` | callback or `()` | — | Receives a versioned level or one pin proposal. Nothing grants permission within the dial. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pinned` | array of string (at most 256) | required | IDs explicitly configured to ask first. |
| `policy` | string | required | Host policy identity. |
| `revision` | integer ≥ 0 | required | Revision seen by the user. |
| `style` | style | — | Style merged over the root part. |
| `subtitle` | string or `()` | — | Optional context under the name. |
| `title` | string or `()` | — | Optional name above the dial. |

### `actions[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `detail` | string or `()` | — | Optional scope or description. |
| `icon` | node or `()` | — | Decorative art instead of the default dot. |
| `id` | string | required | Unique stable ID of the action in this policy. |
| `label` | string | required | Plain action name; not a command. |
| `required_confirmation` | bool or `()` | — | Always asks due to an independent Host rule; this row cannot be unlocked here. |
| `risk` | `"safe"` or `"risky"` or `"destructive"` | required | Risk accepted by the Host. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | object | Propose a change; render only the policy accepted by the Host. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `card`, `count`, `detail`, `empty`, `group_head`, `group_title`, `groups`, `header`, `hint`, `icon`, `label`, `level`, `levels`, `mark`, `message`, `pin`, `risk`, `risk_high`, `risk_low`, `risk_medium`, `root`, `row`, `row_icon`, `subtitle`, `title`.

## Theme

- Tokens: `border`, `focus_ring`, `selection`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/icon_button`](../components/icon_button.md), [`components/radio_group`](../components/radio_group.md), [`patterns/approval_card`](approval_card.md)
