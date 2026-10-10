# Badge

`components/badge` · export `Badge` · version 0.2.0. Generated from
[`registry/components/badge.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/badge.rhai); do not edit.

Badge marks a read-only status: a square lamp followed by its label.

`emphasis: "strong"` turns the badge into a solid status block; reserve it for states that require action. Markers keep their size in both densities.

```rhai
import "components/badge" as badge;

badge::Badge(#{ text: "Ready", variant: "success" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `dot` | bool | `true` | Shows the 6px square lamp before the label; a `strong` badge never shows it. |
| `emphasis` | `"subtle"` or `"strong"` | `"subtle"` | `strong` draws a solid block with a bold label and no lamp, for states that need action. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `size` | `"sm"` or `"md"` | `"md"` | Badge height: `md` is `metrics.marker`, `sm` is `metrics.marker_small`. |
| `style` | style | — | Style merged over the root part. |
| `text` | string | required | The status label; also the accessible name. |
| `variant` | `"neutral"` or `"accent"` or `"success"` or `"warning"` or `"danger"` | `"neutral"` | Status color of the lamp, or of the block when `strong`; `neutral` is muted ink. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `label`, `lamp`, `root`.

## Theme

- Tokens: `accent`, `danger`, `metrics.marker`, `metrics.marker_small`, `on_accent`, `on_danger`, `on_success`, `on_warning`, `radius.sm`, `spacing.sm`, `spacing.xs`, `success`, `surface`, `text_muted`, `text_primary`, `typography.caption`, `warning`
- Environment: `corners`
