# Avatar

`components/avatar` · export `Avatar` · version 0.2.0. Generated from
[`registry/components/avatar.rhai`](../../../registry/components/avatar.rhai); do not edit.

Avatar displays an image handle or name-derived initials with presence.

```rhai
import "components/avatar" as avatar;

avatar::Avatar(#{ name: "Ada Lovelace", initials: "AL", presence: "online" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `handle` | `image` handle or `()` | — | Image handle of the picture, shown in the circle instead of the initials. |
| `initials` | string or `()` | — | Text shown when there is no `handle`; defaults to the first character of `name`. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `name` | string | required | The person's name: the accessible name, and the source of the fallback initials. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `presence` | `"none"` or `"online"` or `"away"` or `"busy"` | `"none"` | Adds an 8px dot at the bottom right: `success` for online, `warning` away, `danger` busy. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size whose `metrics.control` is the diameter; without it, the environment's size. |
| `style` | style | — | Style merged over the root part. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `image`, `initials`, `presence`, `root`.

## Theme

- Tokens: `danger`, `metrics.control`, `success`, `surface`, `surface_hover`, `text_muted`, `text_primary`, `typography.control`, `warning`
- Environment: `density`, `size`
