# Region

`layouts/region` · export `Region` · version 0.2.0. Generated from
[`registry/layouts/region.rhai`](../../../registry/layouts/region.rhai); do not edit.

Region is one working area: a header with the region's title and actions, an optional toolbar, a body that fills the remaining height, and a footer pinned to the bottom.

Content starts at metrics.inset; the header, toolbar and body are `group` apart, so anything nested inside them must use a smaller relationship. `bleed` lets the body reach the region's sides, for tables and lists whose rows carry their own inset: their text then starts on the same edge as the title.

`scroll` makes the body scroll vertically when it is taller than the region, with the same overlay scrollbar as ScrollArea (`scrollbar`); the header, toolbar and footer stay put. Leave it off for bodies that scroll themselves (Table, VirtualList). `external_title` says the region's title is drawn elsewhere, in a TitleBar or a tab: the region takes no `title`, and its parts are not held to a missing title.

```rhai
import "layouts/region" as region;

region::Region(#{ label: "Hosts", title: "Hosts", toolbar: toolbar_node, body: table_node,
    footer: [text("128 hosts")] })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of node (at most 8) | `[]` | Controls on the end side of the header, `related` apart. |
| `bleed` | bool | `false` | Lets the body reach the sides, for tables and lists whose rows carry the inset. |
| `body` | node | required | Main content; fills the remaining height and clips, or scrolls with `scroll`. |
| `external_title` | bool | `false` | The title is drawn elsewhere (a TitleBar or a tab); the region then takes no `title`. |
| `fill` | bool | `true` | Grows the region to the remaining height of its parent, so the body can fill it. |
| `footer` | array of node (at most 16) | `[]` | Status items in a caption row pinned to the bottom under a hairline, `group` apart. |
| `inset` | bool | `true` | Pads the region by `metrics.inset` at the top and bottom and each part at the sides. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the region. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `scroll` | bool | `false` | Scrolls a body taller than the region while the header, toolbar and footer stay put. |
| `scrollbar` | `"auto"` or `"always"` or `"hidden"` | `"auto"` | Visibility of the body's overlay scrollbar, as in ScrollArea; only used with `scroll`. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or node or `()` | — | Header title: a string is drawn as the region's heading; a node is placed as given. |
| `toolbar` | node or `()` | — | A bar between the header and the body, usually a `Toolbar`; it keeps its height. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `actions` | no | yes | Header controls on the end side. |
| `body` | yes | no | The content that fills the region. |
| `footer` | no | yes | Status items pinned to the bottom. |
| `toolbar` | no | no | The bar between the header and the body. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `body`, `footer`, `header`, `root`, `scrollbar_thumb`, `scrollbar_thumb_hover`, `scrollbar_track`, `title`.

## Theme

- Tokens: `border`, `metrics.control`, `metrics.inset`, `metrics.statusbar`, `scrollbar.thumb`, `scrollbar.thumb_hover`, `space.group`, `space.related`, `text_muted`, `text_primary`, `typography.caption`, `typography.title`
- Environment: `density`
