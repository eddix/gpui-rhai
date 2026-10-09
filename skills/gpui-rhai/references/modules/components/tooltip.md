# Tooltip

`components/tooltip` · export `Tooltip` · version 0.2.0. Generated from
[`registry/components/tooltip.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/tooltip.rhai); do not edit.

Native delayed tooltip using the per-window overlay scheduler.

State: stateless; the overlay scheduler owns the delay.

```rhai
import "components/tooltip" as tooltip;

tooltip::Tooltip(#{ key: "help", label: "Help", trigger: text("?"), content: text("Help") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `action` | string or `()` | — | Registered action whose bound shortcut is shown after the hint; `shortcut` takes precedence. |
| `content` | node | required | Hint shown in the tooltip, in inverted colors and at most 320 px wide. |
| `disabled` | bool | `false` | Renders the trigger alone, with no tooltip. |
| `hide_delay_ms` | integer ≥ 0 | `100` | Time the tooltip stays after the pointer leaves the trigger and the tooltip, in milliseconds. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the tooltip, usually the words of `content`. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placement` | `"top"` or `"bottom"` or `"left"` or `"right"` | `"top"` | Preferred side of the trigger the tooltip appears on. |
| `shortcut` | string or `()` | — | Key legend shown after the hint, written like `cmd-d` and formatted for the platform. |
| `show_delay_ms` | integer ≥ 0 | `500` | Hover time before the tooltip appears, in milliseconds. |
| `style` | style | — | Style merged over the root part. |
| `trigger` | node | required | Element whose hover shows the tooltip after `show_delay_ms`. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `content` | yes | no | Hint shown in the tooltip. |
| `trigger` | yes | no | Element whose hover shows the tooltip. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `root`, `shortcut`, `trigger`.

## Theme

- Tokens: `radius.lg`, `spacing.sm`, `spacing.xxs`, `surface`, `text_primary`, `typography.caption`, `typography.label`
- Environment: `corners`
