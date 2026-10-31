# NavigationStack

`patterns/navigation_stack` · export `NavigationStack` · version 0.4.0. Generated from
[`registry/patterns/navigation_stack.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/navigation_stack.rhai); do not edit.

Controlled visit stack. Construction ownership stays with the caller (ADR 0019). Back proposes a retained depth; it neither loads data nor changes routes.

```rhai
import "patterns/navigation_stack" as navigation_stack;

navigation_stack::NavigationStack(#{ key: "visits", label: "Service visits", stack: [
        #{ key: "visit-1", title: "Service", content: text("Overview") }
    ], on_pop: Fn("accept_depth") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of node | `[]` | Trailing actions in the navigation header; omitted with header none. |
| `back_key` | string | `""` | Optional focus-scoped raw key such as cmd+[ or alt+left. Empty binds nothing; a Host can bind/dispatch NavigateBack or call navigate_back before its outer Back. |
| `header` | `"back"` or `"breadcrumb"` or `"none"` | `"back"` | Back to the previous title, a responsive breadcrumb trail with middle-overflow menu, or no navigation header. The current page always has a focusable heading. |
| `key` | string | required | Stable stack instance identity; independent in each view and construction scope. |
| `label` | string | required | Accessible name of this navigation region. |
| `max_depth` | integer 1–128 | `8` | Maximum retained entries. Exceeding the bound rejects the candidate instead of evicting a visit. |
| `motion_key` | string | `""` | Non-empty enables replayed push/pop continuity, mirrored in RTL. Host reduced-motion/none preference settles it; empty disables it. |
| `on_pop` | callback or `()` | — | Receives #{ depth, source }; depth is the number of entries to retain. Sources: button, breadcrumb, keyboard, pointer, host. Without it Back is disabled. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Overrides the inherited control size for header controls. |
| `stack` | array of object (at most 128) | required | Root-to-top controlled entries. Must contain at least one entry; removing an entry unmounts its native presentation resources. |
| `style` | style | — | Style merged over the root part. |

### `stack[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `content` | node | required | Eager caller-owned content, retained while covered including native scroll, editor selection and undo. Hiding does not cancel business tasks or subscriptions; ctx.is_visible() supports an explicit application pause policy. |
| `key` | string | required | Unique non-empty visit key, not the business record id. The same record can appear repeatedly with different keys. |
| `title` | string | required | Page heading and full accessible breadcrumb label. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `pop` | `on_pop` | object | Controlled pop proposal. The caller chooses whether to truncate its stack. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `back`, `breadcrumb`, `header`, `heading`, `overflow`, `page`, `pages`, `root`.

## Theme

- Tokens: `metrics.icon`, `space.group`, `space.related`, `spacing.xs`, `text_muted`, `text_primary`, `typography.title`
- Environment: `density`, `size`

## Dependencies

[`components/button`](../components/button.md), [`components/menu`](../components/menu.md)
