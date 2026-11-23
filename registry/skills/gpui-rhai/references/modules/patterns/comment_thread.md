# CommentThread

`patterns/comment_thread` · export `CommentThread` · version 0.1.0. Generated from
[`registry/patterns/comment_thread.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/comment_thread.rhai); do not edit.

CommentThread shows accepted messages, optional assistant details and a controlled reply field. State: disclosure and scroll follow are local; replies, drafts and activity are application-owned.

```rhai
import "patterns/comment_thread" as comment_thread;

comment_thread::CommentThread(#{key:"review",title:"Review comments",messages:[],draft:""})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `assistant_avatar` | `image` handle or `()` | — | Optional Host avatar shown in the thinking row. |
| `assistant_name` | string or `()` | — | Name shown while the assistant is thinking. |
| `disabled` | bool | `false` | Disables reply, close, and disclosure controls. |
| `draft` | string | required | Accepted editor text. Clear only after the application accepts a reply. |
| `key` | string | required | Stable conversation identity; use a new key for a different conversation. |
| `max_height` | length or `()` | — | Positive absolute length that caps only the transcript viewport. |
| `message` | string or `()` | — | Optional Host rejection or confirmation. |
| `messages` | array of object (at most 512) | required | Accepted messages in chronological display order. At most 512 rows and 512 KiB of text per view. |
| `on_change` | callback or `()` | — | Receives the raw draft string. |
| `on_close` | callback or `()` | — | Receives the thread key when close is requested. |
| `on_send` | callback or `()` | — | Receives trimmed nonblank reply text; no optimistic insertion. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placeholder` | string or `()` | — | Optional reply field placeholder. |
| `sending` | bool | `false` | Blocks a second send while the Host response is pending. |
| `style` | style | — | Style merged over the root part. |
| `thinking` | bool | `false` | Shows an assistant activity row; does not run a reply task. |
| `title` | string or `()` | — | Visible and accessible heading. |

### `messages[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `assistant` | bool | `false` | Draws an assistant emblem instead of initials. |
| `author` | string | required | Visible author name. |
| `avatar` | `image` handle or `()` | — | Optional Host image; failed loading falls back to initials. |
| `details` | array of string (at most 24) | `[]` | Plain detail lines for the status disclosure. |
| `id` | string | required | Nonempty unique message ID. Reuse only for updates to this message. |
| `status` | string or `()` | — | Short result, shown as a disclosure when details are present. |
| `text` | string | required | Plain message text, not Rhai, HTML or a command. |
| `time` | string or `()` | — | Caller-formatted short timestamp; no timer or date parsing is performed. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | string | Propose draft text. |
| `close` | `on_close` | string | Request closing this thread. |
| `send` | `on_send` | string | Propose a nonblank reply. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `assistant`, `author`, `avatar`, `body`, `bullet`, `card`, `close`, `composer`, `count`, `detail`, `details`, `editor`, `empty`, `entry`, `header`, `icon`, `jump`, `list`, `message`, `meta`, `root`, `send`, `status`, `time`, `title`.

## Theme

- Tokens: `border`, `focus_ring`, `radius.lg`, `radius.pill`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.accent`, `text.success`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.initials`, `typography.title`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/icon_button`](../components/icon_button.md), [`components/input`](../components/input.md), [`components/thinking_loader`](../components/thinking_loader.md)
