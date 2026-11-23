# AgentCursors

`patterns/agent_cursors` · export `AgentCursors` · version 0.1.0. Generated from
[`registry/patterns/agent_cursors.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/agent_cursors.rhai); do not edit.

Present a collaboratively reviewed document with live grapheme-anchored cursors.

```rhai
import "patterns/agent_cursors" as agent_cursors;

agent_cursors::AgentCursors(#{key:"review",document:"draft",revision:0,agents:[],blocks:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `active_tab` | string or `()` | — | Accepted active tab. Required when tabs are supplied. |
| `agents` | array of object (at most 12) | required | Up to 12 presence records with optional cursors. Text indices are extended grapheme offsets. |
| `blocks` | array of object (at most 48) | required | Ordered document blocks. Table rows contain equally many keyed cells; row zero is the header. |
| `busy` | bool | `false` | Disable proposals while the Host is settling a mutation. |
| `comments` | array of object (at most 12) or `()` | — | Host-accepted document threads; pin to a grapheme or use a position relative to a block. |
| `document` | string | required | Host-owned document identity. Replacing it resets local inspection and drafts. |
| `fade` | bool | `true` | Fade the sheet paint toward its bottom. Review controls, scrolling, hit regions and semantics are not masked. |
| `following` | string or `()` | — | Accepted followed agent; omit for an undimmed document. |
| `height` | integer 120–1200 | `384` | Document viewport height in logical px. |
| `key` | string | required | Stable retained review surface. |
| `notice` | string or `()` | — | Host reconciliation feedback. |
| `on_action` | callback or `()` | — | Receives versioned review requests. Omit for a read-only review. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Revision of the accepted document and review data used for a proposal. |
| `style` | style | — | Style merged over the root part. |
| `suggestions` | array of object (at most 24) or `()` | — | Proposed, nonoverlapping text edits keyed by text target and accepted grapheme offsets. |
| `tabs` | array of object (at most 12) or `()` | — | Controlled tabs; blocks and reviews belong to the active tab. |
| `title` | string or `()` | — | Workspace heading. |

### `agents[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `avatar` | node or `()` | — | Decorative reviewer mark. Defaults to an initial. |
| `cost_micros` | integer 0–1000000000000 or `()` | — | Accumulated cost in millionths of a US dollar. |
| `cursor` | object or `()` | — | Accepted presence coordinates; omitted means away. |
| `id` | string | required | Unique stable reviewer identity. |
| `ink` | integer 0–16777215 or `()` | — | Optional RGB24 cursor/markup ink. Otherwise use the theme's primary ink. |
| `name` | string | required | Reviewer name, limited to 256 UTF-8 bytes. |

### `agents[].cursor` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `at` | integer 0–16384 or `()` | — | Extended-grapheme caret offset; omit for a pointer to a block's box. |
| `doing` | string or `()` | — | Short activity hint alongside the name. |
| `in_suggestion` | string or `()` | — | Interpret offsets within this edit's proposed insertion, not the accepted block. |
| `point` | `"center"` or `"corner"` or `"add_row"` or `()` | — | Placement of a box-only pointer. Add-row targets require a table. |
| `pressed` | bool or `()` | — | Highlight the box target under the simulated press; no input is dispatched. |
| `select` | integer 1–16384 or `()` | — | Selection length in graphemes or, for a box-only image pointer, request an image frame. |
| `target` | string | required | Current block, table cell, or comment thread ID. |
| `typing` | bool or `()` | — | Show live writing activity and a solid caret. |

### `blocks[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `add_row` | bool or `()` | — | Offer a Host-owned row addition; emits the table ID. |
| `alt` | string or `()` | — | Accessible image description and missing-asset fallback label. |
| `float` | `"start"` or `"end"` or `()` | — | Place the image beside the following text in a wide pane, in logical direction. |
| `id` | string | required | Stable region identity, distinct from other blocks, cells and threads. |
| `kind` | `"title"` or `"heading"` or `"paragraph"` or `"item"` or `"code"` or `"image"` or `"table"` | required | Document block layout. |
| `loading` | bool or `()` | — | Render the motion-aware placeholder without changing the image box. |
| `media` | node or `()` | — | Optional prepared image or custom illustration. Takes precedence over src. |
| `ratio` | number 0.5–4 or `()` | — | Image width divided by height. |
| `rows` | array of array of object (at most 8) (at most 16) or `()` | — | Rectangular table rows starting with the header. |
| `src` | string or `()` | — | An app-declared, prepared image asset ID, not a URL. |
| `text` | string or `()` | — | Accepted copy for textual blocks, or the caption when kind is image. |
| `width` | number 0.2–1 or `()` | — | Image width as a share of the document content width. |

### `blocks[].rows[][]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable anchor identity for this cell. |
| `text` | string | required | Accepted, anchorable cell content. |
| `tone` | `"muted"` or `"info"` or `"success"` or `"warning"` or `()` | — | Optional status pill appearance; text still carries the meaning. |

### `comments[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string | required | Author's reviewer ID. |
| `at` | integer 0–16384 or `()` | — | Optional grapheme offset. Without it use a box-relative pin. |
| `block` | string | required | Document block or cell where the thread is pinned. |
| `id` | string | required | Stable thread identity, distinct from document targets. |
| `open` | bool or `()` | — | Keep this thread open when there is no other local inspector. |
| `replies` | array of string (at most 12) or `()` | — | Host-accepted replies, in order. |
| `text` | string | required | Accepted comment text. |
| `x` | number 0–1 or `()` | — | Logical-leaf box fraction from the physical left for a box-relative pin. |
| `y` | number 0–400 or `()` | — | Offset in logical px below the top of the box. |

### `suggestions[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string | required | Author's reviewer ID. |
| `at` | integer 0–16384 | required | Start grapheme in the accepted text. |
| `block` | string | required | Accepted text block, image caption, or cell target. |
| `id` | string | required | Unique stable proposal ID. |
| `insert` | string or `()` | — | Proposed replacement or insertion. |
| `note` | string or `()` | — | Reason shown in the suggestion inspector. |
| `remove` | integer 0–16384 or `()` | — | Number of accepted graphemes to remove. |

### `tabs[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Unique stable tab identity. |
| `label` | string | required | Visible tab label. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Validate the document, revision and tab; do not treat this proposal as accepted state. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `add_row`, `caption`, `cell`, `code`, `commands`, `doc_title`, `flag`, `flag_activity`, `flag_name`, `flow`, `header`, `heading`, `image`, `image_loading`, `item`, `notice`, `page`, `page_narrow`, `paragraph`, `pin`, `popup`, `popup_text`, `popup_title`, `presence`, `presence_face`, `presence_following`, `presence_menu`, `presence_more`, `replies`, `root`, `sheet`, `stage`, `summary`, `tab`, `tab_active`, `table`, `table_header`, `table_row`, `tabs`, `title`, `tone`.

## Theme

- Tokens: `accent`, `agent_cursors.card`, `agent_cursors.ink`, `border`, `focus_ring`, `radius.md`, `radius.pill`, `radius.sm`, `space.spacing_unit`, `space.unit`, `success`, `surface`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.code`, `typography.label`, `typography.title`, `warning`

## Dependencies

[`components/button`](../components/button.md), [`components/input`](../components/input.md), [`components/spinner`](../components/spinner.md), [`patterns/agent_tree`](agent_tree.md), [`patterns/approval_card`](approval_card.md)
