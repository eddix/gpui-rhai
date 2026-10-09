# Toast

`components/toast` · export `Toast` · version 0.2.0. Generated from
[`registry/components/toast.rhai`](../../../registry/components/toast.rhai); do not edit.

Controlled Toast composition over public Layer, declarative timeout, atoms, and hover events. State: caller owns items; runtime timers own one-shot deadline and pause/resume state.

```rhai
import "components/toast" as toast;

toast::Toast(#{ key: "toasts", items: items, on_dismiss: Fn("dismissed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `dismiss_label` | string | `"Dismiss notification"` | Accessible name of each toast's close button. |
| `items` | array of object (at most 100) | required | The notifications in display order; the caller removes an item when it is dismissed. |
| `key` | string | required | Identity of the toast stack; the ids of its four corner layers derive from it. |
| `max_visible` | integer 1–10 | `3` | How many items, counted from the start of `items` across all regions, show at once; the rest wait without a countdown. |
| `on_dismiss` | callback | required | Called with a toast's `id` when its countdown ends or its close button is pressed. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `dismissible` | bool | `true` | Whether the toast has a close button. |
| `duration_ms` | integer 1–86400000 | `5000` | Milliseconds from showing until the toast asks to be dismissed; hovering it pauses the countdown. |
| `id` | string | required | Identity of the toast, passed to `on_dismiss`; unique and not blank. |
| `message` | string | `""` | Muted text under the title; empty shows none. |
| `paused` | bool | `false` | Holds the countdown while `true`; the toast stays until it is closed or `paused` turns `false`. |
| `region` | `"top_left"` or `"top_right"` or `"bottom_left"` or `"bottom_right"` | `"top_right"` | Window corner the toast stacks in. |
| `title` | string | required | Bold first line and accessible name; not blank. |
| `variant` | `"neutral"` or `"success"` or `"warning"` or `"danger"` | `"neutral"` | Tone of the toast: a status variant adds a colored square lamp, and `danger` is announced as an alert. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `dismiss` | `on_dismiss` | string | Emitted when a toast's countdown ends or its close button is pressed; the payload is the item's `id`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `close`, `lamp`, `message`, `region`, `root`, `title`, `toast`, `toast_danger`, `toast_neutral`, `toast_success`, `toast_warning`.

## Theme

- Tokens: `border`, `danger`, `metrics.icon`, `metrics.inset`, `radius.lg`, `space.related`, `spacing.sm`, `spacing.xxs`, `success`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `warning`
- Environment: `corners`, `density`
