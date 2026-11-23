# AIAnswer

`components/ai_answer` · export `AIAnswer` · version 0.1.0. Generated from
[`registry/components/ai_answer.rhai`](../../../registry/components/ai_answer.rhai); do not edit.

AIAnswer composes a streamed reply, evidence, status, feedback and follow-ups. State: the Host owns the document, status, streaming and business requests.

```rhai
import "components/ai_answer" as ai_answer;

ai_answer::AIAnswer(#{key:"reply",question:"What changed?",source:"The test now passes."})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `copy_text` | string or `()` | — | Explicit clipboard text. Inline Markdown can copy its literal source by default; native documents remain opaque. |
| `feedback_reasons` | array of string (at most 8) or `()` | — | Optional quick reasons supplied to MessageActions. |
| `follow_ups` | array of string (at most 12) | `[]` | Suggested prompts shown after completion. |
| `framed` | bool | `false` | Use the standalone answer card instead of embedded conversation presentation. |
| `key` | string | required | Stable reply ID across Host document revisions. |
| `label` | string or `()` | — | Accessible response label. |
| `on_copy` | callback or `()` | — | Reports committed copy without including contents. |
| `on_feedback` | callback or `()` | — | Feedback #{version,kind,reason}; version is zero for this reply. |
| `on_follow_up` | callback or `()` | — | Proposes the selected prompt text. |
| `on_link` | callback or `()` | — | Ordinary Markdown link activation #{href,text}. |
| `on_open_source` | callback or `()` | — | Explicit open-source request #{id,url}. |
| `on_regenerate` | callback or `()` | — | Ask the application to regenerate. |
| `on_stop` | callback or `()` | — | Request stopping; the application changes status and cancels its producer. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `question` | string or `()` | — | Optional question above the reply. |
| `show_actions` | bool | `true` | Show available copy, retry and feedback actions when idle. |
| `source` | string or native document or `()` | — | Accepted Markdown text or immutable Host document. Pass updated revisions during writing. Links with gpui-cite:1, gpui-cite:2, etc. cite the corresponding source. |
| `sources` | array of object (at most 32) | `[]` | Evidence available for this reply. |
| `status` | `"reading"` or `"writing"` or `"done"` or `"stopped"` | `"done"` | Application-owned phase; stopped preserves the latest accepted content. |
| `style` | style | — | Style merged over the root part. |

### `sources[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `date` | string or `()` | — | Ready-to-display date. |
| `highlight` | array of string (at most 16) or `()` | — | Exact phrases emphasized in the excerpt. |
| `icon` | node or `()` | — | Decorative logo for the source. |
| `id` | string | required | Stable evidence ID. |
| `quote` | string or `()` | — | Evidence excerpt. |
| `site` | string | required | Short source name. |
| `title` | string | required | Source headline. |
| `url` | string or `()` | — | Opaque destination; nothing is opened automatically. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `copy` | `on_copy` | none | Text copied as part of the input transaction. |
| `feedback` | `on_feedback` | object | Vote and optional reason. |
| `follow_up` | `on_follow_up` | string | The selected follow-up prompt. |
| `link` | `on_link` | object | Non-citation link activated. |
| `open_source` | `on_open_source` | object | Explicit source activation. |
| `regenerate` | `on_regenerate` | none | User asked to regenerate. |
| `stop` | `on_stop` | none | User asked to stop. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `answer`, `card`, `follow_button`, `follow_label`, `follow_ups`, `framed`, `indicator`, `question`, `root`, `status`, `status_row`, `stop`, `stopped`.

## Theme

- Tokens: `accent`, `border`, `radius.lg`, `space.spacing_unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/cited_answer`](../components/cited_answer.md), [`components/message_actions`](../components/message_actions.md), [`components/thinking_loader`](../components/thinking_loader.md)
