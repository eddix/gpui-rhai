# Toast

`components/toast` · export `Toast` · version 0.2.2. Generated from
[`registry/components/toast.rhai`](../../../registry/components/toast.rhai); do not edit.

Controlled Toast composition over public Layer, declarative timeout, atoms, and hover events. State: caller owns items; runtime timers own one-shot deadline and pause/resume state; the stack layout keeps which corner the pointer and keyboard focus are in. `layout: "stack"` piles each corner's toasts behind its newest, the last of that corner's items: up to two 6px edges peek out and a count marks the pile. Hovering the pile or moving keyboard focus into it expands it into a list, newest nearest the corner, under a heading with a Clear all button, and holds every countdown in that corner until pointer and focus leave; Escape collapses it.

```rhai
import "components/toast" as toast;

toast::Toast(#{ key: "toasts", items: items, on_dismiss: Fn("dismissed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `clear_label` | string | `"Clear all"` | Label of an expanded stack's button that dismisses every toast in its corner. |
| `dismiss_label` | string | `"Dismiss notification"` | Accessible name of each toast's close button. |
| `heading` | string | `"Notifications"` | Heading of an expanded stack, followed by the number of toasts in it; also names the pile. |
| `items` | array of object (at most 100) | required | The notifications in display order; the caller removes an item when it is dismissed. |
| `key` | string | required | Identity of the toast stack; the ids of its four corner layers derive from it. |
| `layout` | `"flat"` or `"stack"` | `"flat"` | `flat` lists each corner's toasts in a column; `stack` piles them behind the newest and expands the pile while it is hovered or holds keyboard focus. In `stack` every item joins its corner's pile and counts down: `max_visible` applies to `flat` only. |
| `max_visible` | integer 1–10 | `3` | How many items, counted from the start of `items` across all regions, show at once; the rest wait without a countdown. |
| `on_activate` | callback or `()` | — | Called with a toast's `id` when it is clicked or Enter is pressed on it; without it a toast is not a button. |
| `on_dismiss` | callback | required | Called with a toast's `id` when its countdown ends, its close button is pressed or Clear all dismisses its stack. |
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
| `activate` | `on_activate` | string | Emitted when a toast is clicked or Enter is pressed on it while `on_activate` is set; the payload is the item's `id`. |
| `dismiss` | `on_dismiss` | string | Emitted when a toast's countdown ends, its close button is pressed or Clear all dismisses its stack; the payload is the item's `id`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `close`, `lamp`, `message`, `region`, `root`, `stack_clear`, `stack_count`, `stack_edge`, `stack_header`, `stack_list`, `title`, `toast`, `toast_danger`, `toast_neutral`, `toast_success`, `toast_warning`.

## Theme

- Tokens: `border`, `danger`, `focus_ring`, `metrics.icon`, `metrics.inset`, `radius.lg`, `radius.sm`, `radius.xs`, `space.related`, `spacing.sm`, `spacing.xs`, `spacing.xxs`, `success`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `warning`
- Environment: `density`
