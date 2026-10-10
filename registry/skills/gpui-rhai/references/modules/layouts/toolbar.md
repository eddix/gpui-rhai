# Toolbar

`layouts/toolbar` · export `Toolbar` · version 0.2.0. Generated from
[`registry/layouts/toolbar.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/layouts/toolbar.rhai); do not edit.

Toolbar arranges a region's controls by role: context and filters at the start, secondary actions and the one primary action at the end.

Items in a group are `related`; the two groups are at least `group` apart. The single `primary` slot encodes "one solid action per group". `size` reaches every control. `fill` is one node that takes the width the groups leave, between them: a search field, a query input, a path bar. It is at least `metrics.label_column` wide; the bar wraps only when the groups and that minimum do not fit.

```rhai
import "layouts/toolbar" as toolbar;

toolbar::Toolbar(#{ label: "Hosts", filters: [filter_input], actions: [export_button],
    primary: deploy_button })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of node (at most 16) | `[]` | Secondary actions in the end group, before `primary`. |
| `context` | array of node (at most 16) | `[]` | What the bar acts on (tags, a breadcrumb), first in the start group, `related` apart. |
| `fill` | node or `()` | — | One field that takes the width between the groups, at least `metrics.label_column` wide. |
| `filters` | array of node (at most 16) | `[]` | Controls that narrow the content, after `context` in the start group. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the toolbar. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `primary` | node or `()` | — | The one solid action of the bar, last in the end group. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size passed to every control in the bar, so they share one height. |
| `style` | style | — | Style merged over the root part. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `actions` | no | yes | Secondary actions at the end. |
| `context` | no | yes | What the bar acts on, at the start. |
| `fill` | no | no | The field that takes the free width. |
| `filters` | no | yes | Controls that narrow the content. |
| `primary` | no | no | The one solid action, last. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `end`, `fill`, `root`, `start`.

## Theme

- Tokens: `metrics.control`, `metrics.label_column`, `space.group`, `space.related`
- Environment: `size`
