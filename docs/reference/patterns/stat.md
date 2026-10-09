# Stat

`patterns/stat` · export `Stat` · version 0.2.0. Generated from
[`registry/patterns/stat.rhai`](../../../registry/patterns/stat.rhai); do not edit.

Stat shows one figure: a label-voice label, the value with its unit, and an optional change against a reference.

Figures use tabular digits; the unit follows the number in the muted color. `tone` colors only the delta and only when it deviates ("normal is quiet"). Use `stats(items)` for a row of figures.

```rhai
import "patterns/stat" as stat;

stat::Stat(#{ label: "Hosts", value: "128", delta: "+4 this week", tone: "success" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `delta` | string or `()` | — | Change against a reference, such as "+4 this week"; a caption under the figure. |
| `description` | string or `()` | — | Muted caption line below the figure and delta. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | What the figure counts; shown in capitals in the label voice. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `tone` | `"neutral"` or `"accent"` or `"success"` or `"warning"` or `"danger"` | `"neutral"` | Color of `delta` only; keep `neutral` (muted) unless the change deviates from normal. |
| `unit` | string or `()` | — | Unit after the value, in the muted body size. |
| `value` | string | required | The figure itself, already formatted; set in the display size with tabular digits. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `delta`, `description`, `label`, `root`, `unit`, `value`.

## Theme

- Tokens: `space.section`, `spacing.xs`, `spacing.xxs`, `text.accent`, `text.danger`, `text.success`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.display`, `typography.label`
