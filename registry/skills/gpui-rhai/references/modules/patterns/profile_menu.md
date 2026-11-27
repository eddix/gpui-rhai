# ProfileMenu

`patterns/profile_menu` · export `ProfileMenu` · version 0.1.0. Generated from
[`registry/patterns/profile_menu.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/profile_menu.rhai); do not edit.

A standalone account card; the caller owns navigation, sign-out and setup.

```rhai
import "patterns/profile_menu" as profile_menu;

profile_menu::ProfileMenu(#{key:"account",name:"Ada Lovelace",email:"ada@example.test",setup:#{done:2,total:5,next:"Choose a profile picture"}})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `avatar` | node or `()` | — | Custom decorative avatar. Its content owns its own loading behavior. |
| `avatar_handle` | `image` handle or `()` | — | Native image, over the fallback silhouette. The silhouette remains visible for pending and failed images. Custom avatar takes precedence. |
| `email` | string | required | Account caption, such as an email address. |
| `items` | array of object (at most 64) or `()` | — | Actions in display order. Omit for the built-in account actions; an empty list hides the actions. |
| `key` | string | required | Stable account card instance. Use another key for a different account. |
| `name` | string | required | Account display name; omit visual title by passing an empty name. |
| `on_continue` | callback or `()` | — | Receives the current accepted counts when the next step is requested. |
| `on_select` | callback or `()` | — | Receives the stable ID of the chosen action. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `setup` | object or `()` | — | Optional profile setup section. |
| `style` | style | — | Style merged over the root part. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `badge` | string or `()` | — | Optional short badge. |
| `disabled` | bool | `false` | Disabled rows stay visible but are skipped by keyboard navigation. |
| `icon` | node or `()` | — | Optional decorative action glyph. |
| `id` | string | required | Nonempty stable action ID, unique within the card. |
| `label` | string | required | Action name. |

### `setup` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `done` | integer 0–1000000 | required | Accepted completed steps; clamped to total. |
| `next` | string or `()` | — | Next step caption. Omit it to hide the shortcut. |
| `total` | integer 0–1000000 | required | Accepted number of steps. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `continue` | `on_continue` | object | A next step was requested. Props change only when the Host accepts it. |
| `select` | `on_select` | object | An account action was selected. The Host carries out the action. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `account`, `actions`, `avatar`, `card`, `email`, `fill`, `icon`, `identity`, `item`, `item_badge`, `item_label`, `name`, `next`, `next_hint`, `next_icon`, `next_label`, `root`, `setup`, `setup_count`, `setup_head`, `setup_title`, `setup_with_next`, `track`.

## Theme

- Tokens: `border`, `focus_ring`, `profile_menu.accent`, `profile_menu.badge_bg`, `profile_menu.badge_ink`, `profile_menu.muted`, `profile_menu.setup_ink`, `profile_menu.track`, `profile_menu.tray`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.metadata`

## Dependencies

[`components/progress`](../components/progress.md)
