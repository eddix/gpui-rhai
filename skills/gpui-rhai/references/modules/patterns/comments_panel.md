# CommentsPanel

`patterns/comments_panel` · export `CommentsPanel` · version 0.1.0. Generated from
[`registry/patterns/comments_panel.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/comments_panel.rhai); do not edit.

CommentsPanel filters and sorts an accepted comment feed; edit and resolve actions are proposals. State: local filters and sort order; comments, drafts and edit results are application-owned.

```rhai
import "patterns/comments_panel" as comments_panel;

comments_panel::CommentsPanel(#{key:"comments",comments:[],draft:""})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `comments` | array of object (at most 512) | required | Accepted comments. At most 512 rows and 512 KiB of text per view. |
| `current_author` | string or `()` | — | Optional composer initials, not used to assign author identity. |
| `current_avatar` | `image` handle or `()` | — | Optional Host image for the composer. |
| `disabled` | bool | `false` | Blocks data mutations and close. Filtering and sorting remain available. |
| `draft` | string | required | Accepted editor text. Clear only after the application accepts a comment. |
| `key` | string | required | Stable feed identity. |
| `max_height` | length or `()` | — | Positive absolute cap for the list viewport. |
| `message` | string or `()` | — | Optional Host feedback after sending. |
| `on_change` | callback or `()` | — | Receives the raw draft string. |
| `on_close` | callback or `()` | — | Receives the panel key when close is requested. |
| `on_edit` | callback or `()` | — | Receives #{id,revision,action} (run or undo). The Host authorizes and performs it. |
| `on_resolve` | callback or `()` | — | Receives #{id,revision,resolved}. |
| `on_send` | callback or `()` | — | Receives trimmed nonblank plain text. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placeholder` | string or `()` | — | Optional composer placeholder. |
| `posted_id` | string or `()` | — | ID of the user's newly accepted comment. When it appears in the feed, reveal it by returning to All and newest-first; unrelated incoming comments never trigger this. |
| `sending` | bool | `false` | Blocks editing and repeated submission while the Host response is pending. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Visible and accessible heading. |

### `comments[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `author` | string | required | Visible author name. |
| `avatar` | `image` handle or `()` | — | Optional Host image; failed loading falls back to initials. |
| `edit_message` | string or `()` | — | Host-safe detail for an edit result. |
| `edit_state` | `"idle"` or `"running"` or `"applied"` or `"failed"` or `()` | — | Host-enabled edit state. Omit to show an edit without any actions; an /edit string alone never enables one. |
| `id` | string | required | Unique nonempty comment ID. |
| `kind` | `"note"` or `"question"` or `"edit"` or `()` | — | Override the display category. Omitted, /edit plus text is an edit and a terminal question mark is a question. |
| `resolved` | bool | `false` | Accepted resolution state; resolved records stay visible. |
| `revision` | integer ≥ 0 | `0` | Accepted item revision echoed in change requests. |
| `text` | string | required | Plain comment text. An /edit prefix is presentation only. |
| `time` | string or `()` | — | Caller-formatted short timestamp. |
| `timestamp` | integer ≥ 0 | `0` | Caller-supplied timestamp for sorting. Ties preserve supplied order. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | string | Propose draft text. |
| `close` | `on_close` | string | Request closing this panel. |
| `edit` | `on_edit` | object | Propose a Host-owned edit action; no executable content is submitted. |
| `resolve` | `on_resolve` | object | Propose resolving or reopening an accepted comment. |
| `send` | `on_send` | string | Propose a new comment. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `assistant`, `author`, `avatar`, `bar`, `body`, `card`, `close`, `composer`, `edit_actions`, `edit_applied`, `edit_note`, `editor`, `empty`, `entry`, `first`, `header`, `icon`, `list`, `message`, `meta`, `resolve`, `resolved`, `root`, `run`, `send`, `sort`, `tabs`, `time`, `title`, `undo`.

## Theme

- Tokens: `border`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/icon_button`](../components/icon_button.md), [`components/menu`](../components/menu.md), [`components/tabs`](../components/tabs.md), [`patterns/comment_thread`](comment_thread.md)
