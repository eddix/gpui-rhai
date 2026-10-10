# Progress

`components/progress` · export `Progress` · version 0.2.0. Generated from
[`registry/components/progress.rhai`](../../../registry/components/progress.rhai); do not edit.

Determinate or indeterminate progress indicator animated entirely in Rust.

```rhai
import "components/progress" as progress;

progress::Progress(#{ key: "upload", value: 60, max: 100, label: "Upload" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `indeterminate` | bool | `false` | Ignores `value` and loops a short indicator across the track, for work of unknown length. |
| `key` | string | required | Stable identity of the bar; its indicator is keyed `<key>-indicator` to keep its animation. |
| `label` | string | required | Accessible name saying what is in progress. |
| `max` | number > 0 | `100.0` | The value of a complete bar. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `value` | number ≥ 0 | `0.0` | Amount done, in the units of `max`; a value above `max` shows a full bar. |
| `width` | integer 24–4096 | `200` | Track width in logical pixels; the track is 4px tall. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `indicator`, `root`, `track`.

## Theme

- Tokens: `accent`, `surface_hover`
