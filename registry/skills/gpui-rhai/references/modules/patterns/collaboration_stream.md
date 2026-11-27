# CollaborationStream

`patterns/collaboration_stream` · export `CollaborationStream` · version 0.1.0. Generated from
[`registry/patterns/collaboration_stream.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/collaboration_stream.rhai); do not edit.

A chronological, virtualized agent conversation with Host-owned messages and draft.

```rhai
import "patterns/collaboration_stream" as collaboration_stream;

collaboration_stream::CollaborationStream(#{key:"feed",session:"review",agents:[],messages:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agents` | array of object (at most 24) | required | Participants in the conversation. |
| `draft` | string | `""` | Controlled draft, retained until the Host accepts or clears it. |
| `focus_agent` | string or `()` | — | Optional controlled participant focus; null means everyone. |
| `key` | string | required | Stable instance key. |
| `live` | bool | `false` | Enable native tail following until the user scrolls away. |
| `max_height` | number 72–1400 | `520.0` | Transcript cap or, with a composer, fixed height in logical pixels. |
| `messages` | array of object (at most 384) | required | Accepted chronological messages; up to 384 rows and 512 KiB of transcript contents. |
| `on_change` | callback or `()` | — | Receives proposed draft text. |
| `on_focus_agent_change` | callback or `()` | — | Receives an optional proposed participant ID. |
| `on_selection_change` | callback or `()` | — | Receives an optional proposed message ID. |
| `on_send` | callback or `()` | — | Enables the composer and receives a send proposal. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pending` | bool | `false` | Disables editing and duplicate sends while the Host decides. |
| `placeholder` | string or `()` | — | Optional composer placeholder. |
| `revision` | integer ≥ 0 | `0` | Accepted revision carried by send proposals. |
| `selected` | string or `()` | — | Optional controlled message selection; null clears selection. |
| `session` | string | required | Conversation identity; changing it resets local interaction state and scroll. |
| `style` | style | — | Style merged over the root part. |
| `task` | string or `()` | — | Optional task description in the participant bar's accessible description. |
| `title` | string or `()` | — | Accessible conversation title. |

### `agents[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `icon` | node or `()` | — | Optional artwork replacing the pixel mark. |
| `id` | string | required | Unique participant ID. The IDs you and all are reserved. |
| `model` | string or `()` | — | Optional model label. |
| `name` | string | required | Visible participant name. |

### `messages[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `at` | string or `()` | — | Caller-formatted time label. |
| `attachments` | array of object (at most 8) | `[]` | Inline attachment snapshots. |
| `facts` | array of object (at most 8) | `[]` | Facts shown for the selected message. |
| `from` | string | required | Participant ID or you. |
| `id` | string | required | Unique stable message ID. |
| `items` | array of object (at most 12) | `[]` | Related artifacts and tools for the selected message. |
| `kind` | `"ask"` or `"reply"` or `"handoff"` or `"share"` or `"question"` or `"spawn"` | required | Message type; spawn is a quiet system event. |
| `reply_to` | string or `()` | — | Stable original message ID for quote navigation. |
| `text` | string | required | Plain message text, never evaluated. |
| `to` | array of string (at most 24) | required | Recipient IDs; use the single ID all for everyone. |
| `typing` | bool | `false` | Show writing activity rather than unfinished text. |

### `messages[].attachments[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable attachment ID within its message. |
| `kind` | `"file"` or `"folder"` or `"table"` or `"tool"` or `"link"` or `()` | — | Preview presentation, inferred from the label if absent. |
| `label` | string | required | Display label, never a path or URL to fetch. |
| `meta` | string or `()` | — | Optional preview caption. |
| `preview` | string or `()` | — | Optional supplied snapshot; never loads a file or executes a tool. |

### `messages[].facts[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `label` | string | required | Fact name. |
| `value` | string | required | Fact value. |

### `messages[].items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `detail` | string or `()` | — | Optional item detail. |
| `label` | string | required | Item name. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | object | Propose a draft edit. |
| `focus_agent_change` | `on_focus_agent_change` | string or `()` | Propose a participant focus. |
| `selection_change` | `on_selection_change` | string or `()` | Propose a message selection. |
| `send` | `on_send` | object | Propose a send; the SDK does not send or append messages. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `active`, `at`, `badge`, `body`, `bubble`, `chip`, `composer`, `cursor`, `details`, `editor`, `empty`, `fact`, `item`, `latest`, `live`, `mark`, `mention`, `mentions`, `meta`, `needs`, `own`, `panel`, `preview`, `preview_cell`, `preview_meta`, `preview_table`, `quote`, `quote_name`, `quote_own`, `quote_text`, `reply`, `root`, `row`, `send`, `sender`, `system`, `tab`, `tab_selected`, `tabs`, `target_overflow`, `target_person`, `targets`, `time`, `tools`, `top`, `viewport`.

## Theme

- Tokens: `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `stream.accent`, `stream.blue`, `stream.bubble`, `stream.card`, `stream.cyan`, `stream.faint`, `stream.fill`, `stream.green`, `stream.hover`, `stream.ink`, `stream.inverse`, `stream.line`, `stream.muted`, `stream.orange`, `stream.pink`, `stream.shine`, `stream.solid`, `stream.text`, `stream.tray`, `stream.violet`, `stream.wait_bg`, `stream.wait_fg`, `stream.well`, `stream.you`, `typography.body`, `typography.caption`, `typography.code`, `typography.metadata`

## Dependencies

[`components/dialog`](../components/dialog.md), [`components/textarea`](../components/textarea.md)
