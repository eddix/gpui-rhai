# Label

`components/label` · export `Label` · version 0.2.0. Generated from
[`registry/components/label.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/label.rhai); do not edit.

Label presents a control label with optional required and description text.

State: none.

```rhai
import "components/label" as label;

label::Label(#{ text: "Project name", required: true })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `description` | string or `()` | — | Secondary text in caption type and `text_muted`, under the label text. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `required` | bool | `false` | Appends a ` *` required mark in `text.danger` to the text. |
| `style` | style | — | Style merged over the root part. |
| `text` | string | required | Label text in body type; also the accessible name of the label. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `description`, `required`, `root`, `text`.

## Theme

- Tokens: `spacing.xxs`, `text.danger`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
