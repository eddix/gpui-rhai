# TitleBar

`components/title_bar` · export `TitleBar` · version 0.2.0. Generated from
[`registry/components/title_bar.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/title_bar.rhai); do not edit.

Source-owned application chrome with logical start, center, and end regions. With window_drag, the bar replaces the platform title bar: pressing its background moves the window and a double press zooms it. The Host decides whether the window draws a platform title bar; it never closes or resizes a window.

```rhai
import "components/title_bar" as title_bar;

title_bar::TitleBar(#{ label: "Workbench", title: breadcrumb_node, end: [settings_button] })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `center` | array of node (at most 8) | `[]` | Nodes centered in the bar. |
| `end` | array of node (at most 8) | `[]` | Nodes at the end edge; they keep their width while the title truncates. |
| `inset_start` | integer 0–256 | `0` | Logical pixels kept free at the start edge, room for the macOS window buttons. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the bar, which is announced as a toolbar. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `start` | array of node (at most 8) | `[]` | Nodes before the title, at the start edge. |
| `style` | style | — | Style merged over the root part. |
| `subtitle` | string or node or `()` | — | Muted text or node on the title's line; it truncates first. |
| `title` | string or node | required | Title text, or a node such as a breadcrumb; a string is set semibold and the subtitle truncates before it. |
| `window_drag` | bool | `false` | Whether pressing the bar's background moves the window and a double press zooms it. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `center` | no | yes | Nodes centered in the bar. |
| `end` | no | yes | Nodes at the end edge. |
| `start` | no | yes | Nodes before the title, at the start edge. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `center`, `end`, `root`, `start`, `subtitle`, `title`.

## Theme

- Tokens: `border`, `metrics.inset`, `metrics.titlebar`, `space.related`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`
- Environment: `density`
