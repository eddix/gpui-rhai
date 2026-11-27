# CitedAnswer

`components/cited_answer` · export `CitedAnswer` · version 0.1.0. Generated from
[`registry/components/cited_answer.rhai`](../../../registry/components/cited_answer.rhai); do not edit.

CitedAnswer presents linked evidence, inline citations and a source preview. State: Host-owned content; preview is local unless preview is supplied.

```rhai
import "components/cited_answer" as cited_answer;

cited_answer::CitedAnswer(#{key:"evidence",segments:[#{text:"The build passed.",cite:["log"]}],sources:[#{id:"log",site:"Build",title:"Job log"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `action_label` | string or `()` | — | Optional tray action label, shown with on_action. |
| `default_open` | string or `()` | — | Initial source ID to preview in local mode. |
| `key` | string | required | Stable evidence identity. |
| `label` | string or `()` | — | Accessible name of the answer. |
| `on_action` | callback or `()` | — | Activate the optional tray button. |
| `on_link` | callback or `()` | — | Ordinary Markdown link activation #{href,text}. |
| `on_open_source` | callback or `()` | — | Called only when Open source is activated; receives #{id,url}. |
| `on_preview_change` | callback or `()` | — | Source ID proposal or empty string to close. |
| `pages_read` | integer ≥ 0 or `()` | — | Optional count supplied by the application; does not fetch pages. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `preview` | string or `()` | — | Accepted source ID; use "" for closed, omit for local preview. |
| `segments` | array of object (at most 128) | `[]` | Text and evidence IDs when not passing a Markdown source. |
| `show_header` | bool | `true` | Show the evidence summary above the answer. |
| `show_tray` | bool | `true` | Show the evidence caption below the preview. |
| `source` | string or native document or `()` | — | Optional Markdown string or Host document. Links with destinations gpui-cite:1, gpui-cite:2, etc. cite the source at that 1-based position. Use this or segments. |
| `sources` | array of object (at most 32) | `[]` | Evidence in presentation order; IDs must be unique. |
| `style` | style | — | Style merged over the root part. |

### `segments[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `cite` | array of string (at most 8) | — | Cited source IDs ordered as presented. Duplicate or unknown IDs are ignored. |
| `paragraph` | bool | — | Start a new paragraph before this segment. |
| `text` | string | required | Literal sentence or phrase; leading and trailing spaces are preserved. |

### `sources[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `date` | string or `()` | — | Ready-to-display date. |
| `highlight` | array of string (at most 16) or `()` | — | Exact phrases to mark in the excerpt. Omit to mark words shared with structured citing statements. |
| `icon` | node or `()` | — | Decorative icon. Defaults to the site's first character. |
| `id` | string | required | Stable nonempty source ID. |
| `quote` | string or `()` | — | Evidence excerpt displayed and copied as literal text. |
| `site` | string | required | Short source name for its chip. |
| `title` | string | required | Heading or document name. |
| `url` | string or `()` | — | Opaque destination passed to the application, never opened by the component. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | none | Tray action activated. |
| `link` | `on_link` | object | Ordinary Markdown link activated. |
| `open_source` | `on_open_source` | object | Explicit request to open an opaque source destination. |
| `preview_change` | `on_preview_change` | string | Proposed source ID; empty closes. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `annotation`, `body`, `caption`, `card`, `chip`, `chip_active`, `chip_label`, `embedded`, `glyph`, `header`, `meta`, `number`, `paragraph`, `preview`, `preview_actions`, `preview_header`, `preview_title`, `quote`, `quote_annotation`, `root`, `sources`, `sources_label`, `summary`, `tag`, `tag_active`, `title`, `tray`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `radius.lg`, `radius.sm`, `selection`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.accent`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.control`
- Environment: `density`

## Dependencies

[`components/accordion`](../components/accordion.md), [`components/button`](../components/button.md), [`components/markdown_viewer`](../components/markdown_viewer.md)
