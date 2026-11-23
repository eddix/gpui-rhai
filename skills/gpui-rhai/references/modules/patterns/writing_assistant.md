# WritingAssistant

`patterns/writing_assistant` · export `WritingAssistant` · version 0.1.0. Generated from
[`registry/patterns/writing_assistant.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/writing_assistant.rhai); do not edit.

WritingAssistant shows a controlled conversation, suggestion chips, an editor and optional dictation. State: native scroll following only. The application owns messages, drafts, microphone access and work.

```rhai
import "patterns/writing_assistant" as writing_assistant;

writing_assistant::WritingAssistant(#{key:"writer",conversation:"first",revision:1,messages:[],draft:""})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `conversation` | string | required | Identity of the accepted conversation; choose a new ID on reset. |
| `disabled` | bool | `false` | Blocks new application actions. |
| `draft` | string | required | Accepted editor value up to 4,000 graphemes. Do not clear it until you accept send. |
| `draft_locked` | bool | `false` | Make the draft read-only while the Host decides whether to accept its send. Otherwise the editor stays available during a response. |
| `greeting` | string or `()` | — | Welcome heading. |
| `key` | string | required | Stable identity of this assistant surface. |
| `message` | string or `()` | — | Host-safe status or error shown without replacing the draft. |
| `messages` | array of object (at most 512) | required | Accepted messages in display order; at most 512 KiB of text. |
| `name` | string or `()` | — | Assistant name. |
| `on_change` | callback or `()` | — | Receives #{conversation,revision,text}. |
| `on_dictate` | callback or `()` | — | Receives #{conversation,revision,action,lease}. Never opens the microphone itself. |
| `on_reset` | callback or `()` | — | Receives #{conversation,revision} and leaves reset to the application. |
| `on_send` | callback or `()` | — | Receives #{conversation,revision,text,source,suggestion}, with an empty suggestion ID for draft sends. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `phase` | `"idle"` or `"waiting"` or `"streaming"` or `"failed"` | `"idle"` | Accepted request/answer phase. Waiting/streaming block a second send. |
| `placeholder` | string or `()` | — | Empty native editor hint. |
| `revision` | integer ≥ 0 | required | Host revision echoed in every proposal. |
| `speech` | object or `()` | — | Omit to hide dictation. The Host owns capture and authorization. |
| `style` | style | — | Style merged over the root part. |
| `suggestions` | array of object (at most 8) | `[]` | Welcome actions, identified by stable caller keys. |
| `tagline` | string or `()` | — | Detail underneath the name. |

### `messages[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable message ID; keep it unchanged when updating a streamed answer. |
| `role` | `"user"` or `"assistant"` | required | Who authored this message. |
| `text` | string | required | Current accepted text. Empty partial messages are not drawn. |

### `speech` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `lease` | string | required | Accepted dictation session ID. Stop proposes this lease, not any earlier take. |
| `preview` | string or `()` | — | Interim display only, never proposed for send. |
| `state` | `"idle"` or `"starting"` or `"listening"` or `"stopping"` or `"failed"` | required | Dictation state accepted by the Host. |

### `suggestions[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Unique suggestion identity. |
| `label` | string | required | Visible and accessible chip caption. |
| `prompt` | string or `()` | — | Text to propose, defaulting to label. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | object | Propose a draft change, without submitting it. |
| `dictate` | `on_dictate` | object | Request a Host dictation action; no speech data is emitted. |
| `reset` | `on_reset` | object | Request a new conversation. |
| `send` | `on_send` | object | Propose typed text or one explicitly chosen suggestion. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `assistant`, `bar`, `body`, `controls`, `deck`, `dot`, `editor`, `feedback`, `field`, `glow_blue`, `glow_chat`, `glow_pink`, `glow_violet`, `greeting`, `header`, `icon`, `jump`, `mic`, `mic_active`, `name`, `reset`, `root`, `row`, `send`, `stage`, `suggestion`, `suggestions`, `tagline`, `transcript`, `typing`, `user`, `wave`, `welcome`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `on_accent`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.title`, `writing.glow_blue`, `writing.glow_pink`, `writing.glow_violet`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/icon_button`](../components/icon_button.md), [`components/textarea`](../components/textarea.md)
