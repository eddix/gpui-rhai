# TextReveal

`motion/text_reveal` · export `TextReveal` · version 0.1.6. Generated from
[`registry/motion/text_reveal.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/motion/text_reveal.rhai); do not edit.

Grapheme-safe native text reveal. Rhai builds spans once; Rust samples opacity. State: stateless. Emits no events.

```rhai
import "motion/text_reveal" as text_reveal;

text_reveal::TextReveal(#{ key: "hero", text: "Hello" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `key` | string | required | Motion identity of the reveal; keep it stable across renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `text` | string | required | Text revealed one grapheme cluster at a time, with a `tight` stagger, in `title` typography. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`.
