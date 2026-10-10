# Spinner

`components/spinner` · export `Spinner` · version 0.2.0. Generated from
[`registry/components/spinner.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/spinner.rhai); do not edit.

Compact indeterminate activity indicator animated by the native runtime clock.

With reduced motion the spinner stops turning and settles on a visible frame.

```rhai
import "components/spinner" as spinner;

spinner::Spinner(#{ key: "loading", label: "Loading" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `key` | string | required | Stable identity of the spinner; keeps its rotation continuous across renders. |
| `label` | string | required | Accessible name saying what is in progress. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` | `"md"` | Diameter: `xs` 12, `sm` 14, `md` 18 and `lg` 22 logical pixels. |
| `speed_ms` | integer 240–4000 or `()` | — | Duration of one full turn in milliseconds; unset, the theme's `ambient` motion duration. |
| `style` | style | — | Style merged over the root part. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `indicator`, `root`.

## Theme

- Tokens: `accent`
