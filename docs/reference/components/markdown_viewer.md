# MarkdownViewer

`components/markdown_viewer` · export `MarkdownViewer` · version 0.4.3. Generated from
[`registry/components/markdown_viewer.rhai`](../../../registry/components/markdown_viewer.rhai); do not edit.

Native CommonMark/GFM, not an editor or a WebView. Host documents keep source and parse models out of Rhai. Links emit; images require explicit Host assets.

```rhai
import "components/markdown_viewer" as markdown_viewer;

markdown_viewer::MarkdownViewer(#{key:"article",label:"Article",source:ctx.get_native_text_document("article")})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `active_tag_link` | string or `()` | — | Matching tag link presented using the active tag paint recipe. |
| `annotations` | array of object (at most 128) | `[]` | Paint-only annotations of the current visible plain text. Do not pass Markdown source offsets. |
| `code_block_actions` | array of `"copy"` (at most 1) | `["copy"]` | Code block actions; copy writes code without fences. |
| `height` | `"fill"` or `"content"` | `"fill"` | Fill available space or grow to measured content height. |
| `image_assets` | map of string | `#{}` | Explicit Markdown destination to Host asset ID map. The viewer never fetches URLs; missing assets show alt text. |
| `key` | string | required | Stable viewer identity; owns scroll, selection and background-job cancellation. |
| `label` | string | `""` | Accessible document name. Empty, the localized `markdown.label` message, `Markdown` without one. |
| `max_height` | integer ≥ 0 or length or `()` | — | Maximum content height as a logical-pixel integer or absolute-resolvable Length (px/rem); relative lengths are not supported. Overflow scrolls natively. |
| `on_link` | callback or `()` | — | Link activation callback with href and visible text. Host validates and decides what to open. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `source` | string or native document | required | Markdown string or Host-owned NativeTextDocument from ctx.get_native_text_document. |
| `style` | style | — | Style merged over the root part. |
| `table_max_height` | integer ≥ 0 or length or `()` | — | Viewport height of a table over 1,000 cells, which scrolls inside it with an always-shown scrollbar; unset is 400 logical pixels. A smaller table lays out in full and grows with the document. Columns fit their widest cell on one line, 64 to 360 logical pixels, and longer cells wrap. |
| `tag_link_prefix` | string or `()` | — | Makes links with this destination prefix use the inline tag paint recipe. They retain normal link interaction. |

### `annotations[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `end` | integer ≥ 0 | required | Exclusive UTF-8 byte offset into visible plain text. |
| `start` | integer ≥ 0 | required | Inclusive UTF-8 byte offset into visible plain text. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `link` | `on_link` | object | User activated a link by pointer or keyboard. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `annotation`, `code_block`, `error`, `heading`, `inline_code`, `link`, `loading`, `paragraph`, `quote`, `root`, `rule`, `table`, `tag`, `tag_active`.

## Theme

- Tokens: `accent`, `border`, `on_accent`, `selection`, `spacing.sm`, `spacing.xs`, `surface`, `surface_raised`, `text.accent`, `typography.body`, `typography.body_small`, `typography.caption`, `typography.subtitle`, `typography.title`
