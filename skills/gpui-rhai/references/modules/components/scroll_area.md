# ScrollArea

`components/scroll_area` · export `ScrollArea` · version 0.2.0. Generated from
[`registry/components/scroll_area.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/scroll_area.rhai); do not edit.

Sized retained scroll viewport with themeable overlay scrollbars.

```rhai
import "components/scroll_area" as scroll_area;

scroll_area::ScrollArea(#{ key: "logs", label: "Logs", height: px(240), content: long_content })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `axis` | `"horizontal"` or `"vertical"` or `"both"` | `"vertical"` | Directions the content scrolls in; the scrollbar of an axis that does not scroll stays hidden. |
| `content` | node | required | Node that scrolls inside the viewport. |
| `height` | length | required | Viewport height; the area keeps it whatever the size of its content. |
| `horizontal_scrollbar` | `"auto"` or `"always"` or `"hidden"` | `"auto"` | When the horizontal bar shows: `auto` on hover and for 1.4 s after scrolling, `always` whenever content overflows. |
| `key` | string | required | Stable identity of the viewport; the native scroll handle keeps the scroll offset and scrollbar drag state under it. |
| `label` | string | required | Accessible name of the scroll region. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `vertical_scrollbar` | `"auto"` or `"always"` or `"hidden"` | `"auto"` | When the vertical bar shows: `auto` on hover and for 1.4 s after scrolling, `always` whenever content overflows. |
| `width` | length or `()` | — | Viewport width; `()` leaves the width to the surrounding layout. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `content` | yes | no | Node that scrolls inside the viewport. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `root`, `scrollbar_thumb`, `scrollbar_thumb_hover`, `scrollbar_track`.

## Theme

- Tokens: `scrollbar.thumb`, `scrollbar.thumb_hover`
