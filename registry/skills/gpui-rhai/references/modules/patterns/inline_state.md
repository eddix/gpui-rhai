# InlineState

`patterns/inline_state` · export `InlineState` · version 0.2.0. Generated from
[`registry/patterns/inline_state.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/inline_state.rhai); do not edit.

InlineState shows loading, empty, error, stale and refreshing feedback where the content would appear.

Loading and empty center in the space; errors read top-down from the content edge with the raw error selectable; stale and refreshing are one quiet line above kept data. The title is required because only the caller knows what is loading or why it is empty. Long text wraps in a narrow container.

```rhai
import "patterns/inline_state" as inline_state;

inline_state::InlineState(#{ key: "hosts", state: "empty", title: "No hosts yet",
    description: "Register a host to see it here.", actions: [add_button] })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of node (at most 4) | `[]` | Buttons after the text, such as Retry; not shown while loading. |
| `description` | string or `()` | — | A muted second line, such as the next step or the reason; not shown while loading. |
| `detail` | string or `()` | — | Raw error text, selectable, in the code face; shown for `error`, `stale` and `refreshing`. |
| `key` | string | required | Stable identity of the feedback; its spinner is keyed `<key>-spinner`. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `state` | `"loading"` or `"empty"` or `"error"` or `"stale"` or `"refreshing"` | required | Which feedback to show; `stale` and `refreshing` are one line meant to sit above kept data. |
| `style` | style | — | Style merged over the root part. |
| `title` | string | required | What is loading, why it is empty, or what failed; also the accessible name. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `actions` | no | yes | Buttons after the text. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `description`, `detail`, `lamp`, `root`, `title`.

## Theme

- Tokens: `danger`, `metrics.inset`, `metrics.row`, `space.related`, `spacing.sm`, `spacing.xxs`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.code`, `typography.subtitle`, `warning`
- Environment: `density`

## Dependencies

[`components/spinner`](../components/spinner.md)
