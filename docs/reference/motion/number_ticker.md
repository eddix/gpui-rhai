# NumberTicker

`motion/number_ticker` · export `NumberTicker` · version 0.1.6. Generated from
[`registry/motion/number_ticker.rhai`](../../../registry/motion/number_ticker.rhai); do not edit.

NumberTicker shows an integer and fades each new value in. State: stateless. Emits no events.

```rhai
import "motion/number_ticker" as number_ticker;

number_ticker::NumberTicker(#{ key: "count", value: 42 })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `key` | string | required | Motion identity of the ticker; keep it stable so only a changed `value` replays the fade. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `value` | integer | required | Number shown in `display` typography; a new value fades in while the old one fades out. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`.
