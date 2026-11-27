# Spinner

`components/spinner` · export `Spinner` · version 0.4.3. Generated from
[`registry/components/spinner.rhai`](../../../registry/components/spinner.rhai); do not edit.

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
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Inherits the size environment unless set. The preset's `metrics.spinner` sets the diameter: xs 12, sm/md 14 and lg 18, scaled with reading size. |
| `speed_ms` | integer 240–4000 or `()` | — | Duration of one full turn in milliseconds; unset, `spinner.speed_ms` (Modern 750, Clean 800). Reduced/None motion holds a visible frame. |
| `style` | style | — | Style merged over the root part. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `indicator`, `root`.

## Theme

- Tokens: `metrics.spinner`, `spinner.speed_ms`, `spinner.stroke_width`, `spinner.track_alpha`
- Environment: `density`, `size`
