# Composer

`patterns/composer` · export `Composer` · version 0.2.2. Generated from
[`registry/patterns/composer.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/composer.rhai); do not edit.

Controlled drafting surface. Busy blocks sending, never drafting; the application owns sending, stopping and clearing the accepted value.

```rhai
import "patterns/composer" as composer;

composer::Composer(#{ key: "prompt", label: "Prompt", value: draft,
    on_change: Fn("draft_changed"), on_submit: Fn("send"), on_stop: Fn("stop"), busy: busy })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `autofocus` | bool | `false` | Focuses the draft once on mount unless disabled. |
| `busy` | bool | `false` | Shows Stop and blocks send while leaving the draft editable. |
| `can_submit` | bool | `true` | Application validation gate for sending, independent of busy. |
| `disabled` | bool | `false` | Disables the editor and Send/Stop; caller content in leading retains its own policy. |
| `key` | string | required | Stable drafting identity; changing it resets native editing state. |
| `label` | string | required | Accessible name of the drafting field. |
| `leading` | node or `()` | — | Caller-owned attachment/model/slash-command controls; no attachment or task policy is built in. |
| `max_length` | integer 0–1000000 or `()` | — | Optional grapheme limit, also displayed in the counter. |
| `max_rows` | integer 1–1000 | `8` | Maximum visual rows before internal scrolling. |
| `min_rows` | integer 1–1000 | `2` | Minimum native visual rows. |
| `on_change` | callback or `()` | — | Full draft after each native edit. |
| `on_stop` | callback or `()` | — | Stop request while busy; the application owns cancellation. |
| `on_submit` | callback or `()` | — | Accepted full draft from keyboard or Send. The application clears value if desired. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placeholder` | string or `()` | — | Hint while the draft is empty. |
| `read_only` | bool | `false` | Preserves selection/copy but blocks drafting and submission. |
| `show_count` | bool | `true` | Displays the grapheme count in the action row. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Optional control size, otherwise inherited by drafting and action controls. |
| `style` | style | — | Style merged over the root part. |
| `submit_key` | `"enter"` or `"mod_enter"` | `"enter"` | Enter sends with Shift+Enter newline, or platform modifier+Enter sends with Enter newline. |
| `value` | string | required | Controlled draft; sending does not clear it. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | string | Full edited draft. |
| `stop` | `on_stop` | none | Application cancellation request. |
| `submit` | `on_submit` | string | Full accepted draft, without clearing it. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `leading` | no | no | Independent caller-owned controls in the action row. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `counter`, `editor`, `leading`, `limit`, `root`, `send`, `stop`.

## Theme

- Tokens: `border`, `focus_ring`, `metrics.inset`, `radius.md`, `space.related`, `surface_raised`, `text.danger`, `text_muted`, `typography.caption`
- Environment: `density`, `size`

## Dependencies

[`components/button`](../components/button.md), [`components/textarea`](../components/textarea.md)
