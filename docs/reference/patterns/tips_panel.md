# TipsPanel

`patterns/tips_panel` · export `TipsPanel` · version 0.1.0. Generated from
[`registry/patterns/tips_panel.rhai`](../../../registry/patterns/tips_panel.rhai); do not edit.

Present bounded onboarding tips and propose actions to the application.

```rhai
import "patterns/tips_panel" as tips_panel;

tips_panel::TipsPanel(#{key:"intro",series:"account",revision:0,tips:[#{id:"search",title:"Find a command",description:"Open the command menu.",action:"Try search"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `close_label` | string or `()` | — | Footer action label. No footer action is shown without on_close. |
| `key` | string | required | Stable panel key. |
| `max_height` | integer 120–800 or `()` | — | Optional viewport height cap in logical pixels; normally the theme supplies 420. |
| `on_action` | callback or `()` | — | Receives a versioned tip action request. Without it the checklist is read-only. |
| `on_close` | callback or `()` | — | Receives a close request. Removing or retaining the panel is the Host's decision. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pending` | string or `()` | — | The ID of an in-flight tip request. Prevents duplicate requests while the Host decides. |
| `revision` | integer ≥ 0 | required | Revision of the accepted tip states. |
| `series` | string | required | Identity of this onboarding checklist. |
| `style` | style | — | Style merged over the root part. |
| `tips` | array of object (at most 48) | required | At most 48 tips in application order. The done counter includes only present tips. |
| `title` | string or `()` | — | Panel title; defaults to Tips. |

### `tips[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `action` | string | required | Button label for the requested action. |
| `description` | string | required | Plain language description, also accessible when detail replaces its display. |
| `detail` | node or `()` | — | Optional formatted description node, e.g. a row with a keyboard keycap. |
| `done` | bool or `()` | — | Accepted completion, published by the Host after an action succeeds. |
| `done_label` | string or `()` | — | Replacement label after the Host marks the tip done. |
| `id` | string | required | Unique stable tip identity. |
| `primary` | bool or `()` | — | Use a strong primary appearance for the most important tip. |
| `title` | string | required | Short heading of the tip. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | A proposal to perform the tip's action. Advance the snapshot with done=true only after acceptance. |
| `close` | `on_close` | object | A proposal to close the panel, not an automatic dismissal. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `action_done`, `action_primary`, `body`, `close`, `count`, `description`, `empty`, `footer`, `grid`, `head`, `mark`, `root`, `root_no_footer`, `tip`, `tip_done`, `tip_title`, `title`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `tips_panel.done`, `typography.body`, `typography.caption`

## Dependencies

[`components/button`](../components/button.md), [`patterns/agent_tree`](../patterns/agent_tree.md)
