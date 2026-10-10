# Input

`components/input` · export `Input` · version 0.2.0. Generated from
[`registry/components/input.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/input.rhai); do not edit.

Input is a controlled single-line native text input.

State: native keyed TextInput Entity.

`appearance: "embedded"` is the search or filter line that heads a panel or a list and keeps focus while it is open: no frame or well of its own, the container's edges and a hairline under it frame it, and its text starts on metrics.inset like the rows below. Its caret shows focus; the hairline stays `border` (`danger` while invalid).

```rhai
import "components/input" as input;

input::Input(#{ key: "name", label: "Name", value: name, on_change: Fn("name_changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `appearance` | `"field"` or `"embedded"` | `"field"` | `field` is a framed well; `embedded` is the frameless search or filter line that heads a panel or list. |
| `autofocus` | bool | `false` | Focuses the field once, when its editor first mounts; ignored while `disabled`. |
| `disabled` | bool | `false` | Takes the field out of the tab order, blocks editing and submit, and dims it. |
| `error` | bool | `false` | Marks the value invalid: a `danger` frame (or hairline when embedded) and the invalid state for assistive tech. |
| `key` | string | required | Names the native editor, which keeps caret, selection and undo history across renders; a new key starts a fresh editor. |
| `label` | string | required | Accessible name of the field; it is not drawn, so pair the input with a `Label` or a `FormField`. |
| `on_blur` | callback or `()` | — | Called when the field loses keyboard focus. |
| `on_change` | callback or `()` | — | Called with the full new text after each edit. |
| `on_focus` | callback or `()` | — | Called when the field gains keyboard focus. |
| `on_submit` | callback or `()` | — | Called with the current text when Enter is pressed in a field that is not `disabled`; without it, Enter reaches the field's ancestors, such as a form's default action. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placeholder` | string or `()` | — | Muted hint shown while `value` is empty. |
| `read_only` | bool | `false` | Keeps the text focusable, selectable and copyable but blocks edits; the text turns `text_muted`. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size of the field; unset, it inherits the `size` environment. |
| `style` | style | — | Style merged over the root part. |
| `value` | string | required | Current text; the caller stores the `change` payload and passes it back. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `blur` | `on_blur` | none | Emitted when the field loses keyboard focus. |
| `change` | `on_change` | string | Emitted after each edit; the payload is the full new text. |
| `focus` | `on_focus` | none | Emitted when the field gains keyboard focus. |
| `submit` | `on_submit` | string | Emitted when Enter is pressed in a field that is not `disabled`; the payload is the current text. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `input`, `root`.

## Theme

- Tokens: `border`, `danger`, `disabled`, `focus_ring`, `metrics.control`, `metrics.field_pad`, `metrics.inset`, `radius.md`, `surface_raised`, `text_muted`, `text_primary`, `typography.control`
- Environment: `corners`, `density`, `size`
