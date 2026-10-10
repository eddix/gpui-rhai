# Kbd

`components/kbd` · export `Kbd` · version 0.2.0. Generated from
[`registry/components/kbd.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/kbd.rhai); do not edit.

Kbd presents a key legend: a framed keycap, or an inline legend inside menus and buttons.

`keys` renders one keycap per key. Markers keep their size in both densities.

```rhai
import "components/kbd" as kbd;

kbd::Kbd(#{ keys: ["⌘", "K"], label: "Command K" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `appearance` | `"keycap"` or `"inline"` | `"keycap"` | `keycap` frames each key; `inline` shows bare legends, for use inside menus and buttons. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `keys` | array of string (at most 8) | `[]` | Legends rendered one key each, in order; when non-empty they replace `text`. |
| `label` | string or `()` | — | Accessible name; defaults to `text`, or to the `keys` joined by spaces. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `text` | string | `""` | Legend of a single key, used when `keys` is empty; one of `text` and `keys` is required. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `key`, `root`.

## Theme

- Tokens: `border`, `metrics.marker`, `radius.xs`, `spacing.xs`, `spacing.xxs`, `text_muted`, `typography.label`
- Environment: `corners`
