# Badge

`components/badge` · export `Badge` · version 0.4.3. Generated from
[`registry/components/badge.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/badge.rhai); do not edit.

Badge presents a soft status surface, optional dot/icon, or a numeric count.

`emphasis: "strong"` turns the badge into a solid status block; reserve it for states that require action. Markers keep their size in both densities.

```rhai
import "components/badge" as badge;

badge::Badge(#{ text: "Ready", variant: "success" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `count` | integer or `()` | — | Count instead of text; negative values display as zero. |
| `dot` | bool | `false` | Shows a circular status dot before the label; strong/count variants omit it. |
| `emphasis` | `"subtle"` or `"strong"` | `"subtle"` | `strong` draws a solid block with a paired readable label and no lamp, for states that need action. |
| `icon` | node or `()` | — | An icon before the label; dot/live and count take precedence. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `live` | bool | `false` | Shows a native-sampled pulsing status dot; honors motion preferences. |
| `max` | integer ≥ 0 | `99` | Counts above this value display max+. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `size` | `"sm"` or `"md"` | `"sm"` | Badge height: `md` is `metrics.marker`, `sm` is `metrics.marker_small`. |
| `style` | style | — | Style merged over the root part. |
| `text` | string | `""` | The status label; also the accessible name. |
| `variant` | `"neutral"` or `"info"` or `"accent"` or `"success"` or `"warning"` or `"danger"` or `"purple"` | `"neutral"` | Paired foreground/background tone; accent is an alias for info. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `icon` | no | no | Optional leading icon. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `icon`, `label`, `lamp`, `root`.

## Theme

- Tokens: `badge.danger_bg`, `badge.danger_fg`, `badge.danger_strong_bg`, `badge.danger_strong_fg`, `badge.info_bg`, `badge.info_fg`, `badge.info_strong_bg`, `badge.info_strong_fg`, `badge.neutral_bg`, `badge.neutral_fg`, `badge.neutral_strong_bg`, `badge.neutral_strong_fg`, `badge.purple_bg`, `badge.purple_fg`, `badge.purple_strong_bg`, `badge.purple_strong_fg`, `badge.success_bg`, `badge.success_fg`, `badge.success_strong_bg`, `badge.success_strong_fg`, `badge.warning_bg`, `badge.warning_fg`, `badge.warning_strong_bg`, `badge.warning_strong_fg`, `metrics.badge_lead`, `metrics.marker`, `metrics.marker_small`, `radius.pill`, `spacing.xs`, `text.subtle`
