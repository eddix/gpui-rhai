# Icon

`components/icon` · export `Icon` · version 0.2.0. Generated from
[`registry/components/icon.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/icon.rhai); do not edit.

Icon wraps a declarative AssetId or a runtime image handle with consistent sizing.

```rhai
import "components/icon" as icon;

icon::Icon(#{ source: asset("app/icons/check"), size: "sm", label: "Complete" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string or `()` | — | Accessible name; without one, or with an empty one, the icon is decorative. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `rtl_source` | asset or `image` handle or `()` | — | Glyph drawn instead of `source` in right-to-left layouts, for directional icons such as arrows. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Glyph box: `xs` 12, `sm` 14, `md` 16, `lg` 20 logical pixels; unset, it follows `metrics.icon`. |
| `source` | asset or `image` handle | required | The glyph: a declared asset such as `asset("app/icons/check")`, or a runtime image handle. |
| `style` | style | — | Style merged over the root part. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`.

## Theme

- Tokens: `metrics.icon`
- Environment: `density`, `size`
