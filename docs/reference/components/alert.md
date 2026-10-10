# Alert

`components/alert` · export `Alert` · version 0.2.0. Generated from
[`registry/components/alert.rhai`](../../../registry/components/alert.rhai); do not edit.

Persistent inline feedback with semantic variants and composable actions.

```rhai
import "components/alert" as alert;

alert::Alert(#{ title: "Saved", description: "Changes are live.", variant: "success" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of node (at most 4) | `[]` | Controls on the end side, `related` apart. |
| `content` | node or `()` | — | Extra content under the description, such as a list or a link. |
| `description` | string or `()` | — | Muted text under the title; it wraps with the alert's width. |
| `icon` | node or `()` | — | Replaces the square lamp; sized to `metrics.icon` and tinted with the variant color. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `title` | string | required | The message in bold body text; also the alert's accessible name. |
| `variant` | `"info"` or `"success"` or `"warning"` or `"danger"` | `"info"` | Lamp or icon color, `text_muted` for `info` and the status color otherwise; `warning` and `danger` announce as an alert, the others as a status. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `actions` | no | yes | Controls on the end side. |
| `content` | no | no | Extra content under the description. |
| `icon` | no | no | A glyph in place of the lamp. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `body`, `content`, `description`, `icon`, `lamp`, `root`, `title`.

## Theme

- Tokens: `danger`, `metrics.icon`, `metrics.inset`, `radius.lg`, `space.related`, `spacing.sm`, `spacing.xxs`, `success`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `warning`
- Environment: `corners`, `density`
