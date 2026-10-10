# Textarea

`components/textarea` · export `Textarea` · version 0.2.0. Generated from
[`registry/components/textarea.rhai`](../../../registry/components/textarea.rhai); do not edit.

Textarea is a controlled multiline field with native IME, selection, wrapping, and auto-grow.

State: keyed native transient editor state.

```rhai
import "components/textarea" as textarea;

textarea::Textarea(#{ key: "notes", label: "Notes", value: notes, max_length: 500, show_count: true, on_change: Fn("notes_changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `autofocus` | bool | `false` | Focuses the field once, when its editor first mounts; ignored while `disabled`. |
| `disabled` | bool | `false` | Takes the field out of the tab order, blocks editing, and dims it. |
| `error` | bool | `false` | Marks the value invalid: a `danger` frame while unfocused and the invalid state for assistive tech. |
| `key` | string | required | Names the native editor, which keeps caret, selection, scroll and undo history across renders; a new key starts a fresh editor. |
| `label` | string | required | Accessible name of the field; it is not drawn, so pair the textarea with a `Label` or a `FormField`. |
| `max_length` | integer 0–1000000 or `()` | — | Most characters (grapheme clusters) typing or paste can reach; the controlled `value` must not exceed it. |
| `max_rows` | integer 1–1000 | `8` | Most visual (wrapped) lines the field grows to before it scrolls inside. |
| `min_rows` | integer 1–1000 | `3` | Fewest visual (wrapped) lines the auto-growing field shows; must not exceed `max_rows`. |
| `on_blur` | callback or `()` | — | Called when the field loses keyboard focus. |
| `on_change` | callback or `()` | — | Called with the full new text after each edit. |
| `on_focus` | callback or `()` | — | Called when the field gains keyboard focus. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placeholder` | string or `()` | — | Muted hint shown while `value` is empty. |
| `read_only` | bool | `false` | Keeps the text focusable and selectable but blocks edits; the text turns `text_muted`. |
| `rows` | integer 1–1000 or `()` | — | Fixed height in visual lines; the field scrolls inside it and stops auto-growing between `min_rows` and `max_rows`. |
| `show_count` | bool | `false` | Shows the character count under the field, as `count / max_length` with a limit; it turns `text.danger` at the limit. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size; `xs` sets the text in `body_small`. Unset, it inherits the `size` environment. |
| `style` | style | — | Style merged over the root part. |
| `value` | string | required | Current text, newlines included; the caller stores the `change` payload and passes it back. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `blur` | `on_blur` | none | Emitted when the field loses keyboard focus. |
| `change` | `on_change` | string | Emitted after each edit; the payload is the full new text. |
| `focus` | `on_focus` | none | Emitted when the field gains keyboard focus. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `caret`, `counter`, `editor`, `limit`, `placeholder`, `root`, `scroll`, `selection`.

## Theme

- Tokens: `accent`, `border`, `danger`, `disabled`, `focus_ring`, `metrics.field_pad`, `metrics.multiline_pad`, `radius.lg`, `selection`, `spacing.xxs`, `surface_raised`, `text.danger`, `text_muted`, `text_primary`, `typography.body`, `typography.body_small`, `typography.caption`
- Environment: `corners`, `density`, `size`
