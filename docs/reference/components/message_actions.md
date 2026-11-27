# MessageActions

`components/message_actions` · export `MessageActions` · version 0.1.0. Generated from
[`registry/components/message_actions.rhai`](../../../registry/components/message_actions.rhai); do not edit.

MessageActions presents clipboard, version, speech request and feedback controls for one message. State: local feedback and transient indicators; version and playback can be application-owned.

```rhai
import "components/message_actions" as message_actions;

message_actions::MessageActions(#{key:"response",text:"The tests passed",versions:2})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of `"copy"` or `"regenerate"` or `"feedback"` or `"speak"` (at most 4) | `["copy", "regenerate", "feedback", "speak"]` | Action groups in display order. |
| `default_version` | integer 0–999 or `()` | — | Initial local version; defaults to newest. |
| `key` | string | required | Stable message identity; changing it resets local feedback. |
| `label` | string or `()` | — | Accessible group name. |
| `on_copy` | callback or `()` | — | Receives the version after queuing the transaction's clipboard write. |
| `on_feedback` | callback or `()` | — | Feedback map with version, kind, and optional reason. |
| `on_regenerate` | callback or `()` | — | Request another version. |
| `on_speak` | callback or `()` | — | Proposed version and playing state. No speech engine is invoked. |
| `on_version_change` | callback or `()` | — | Proposed version index. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `quiet` | bool | `false` | Deemphasize the inactive bar, restoring ink on hover or keyboard focus. |
| `reasons` | array of string (at most 8) or `()` | — | Quick reasons; a free-text Other option is always available. |
| `regenerating` | bool | `false` | Application-owned busy state. |
| `speaking` | bool | `false` | Application-owned speech playback state. |
| `style` | style | — | Style merged over the root part. |
| `text` | string or `()` | — | Text for the Copy action. Omit to hide Copy. |
| `version` | integer 0–999 or `()` | — | Accepted zero-based version. Omit for local navigation. |
| `versions` | integer 1–1000 | `1` | Number of versions. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `copy` | `on_copy` | integer | Version copied; message contents are not included. |
| `feedback` | `on_feedback` | object | Vote or reason. Unit kind clears a vote. |
| `regenerate` | `on_regenerate` | integer | Version to regenerate. |
| `speak` | `on_speak` | object | Playback request. |
| `version_change` | `on_version_change` | integer | Proposed zero-based version. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `bar`, `copied`, `counter`, `current`, `feedback`, `glyph`, `heading`, `other`, `pressed`, `quiet`, `reason`, `reasons`, `root`, `send`, `thanks`, `total`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.success`, `text_muted`, `text_primary`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/accordion`](../components/accordion.md), [`components/button`](../components/button.md), [`components/textarea`](../components/textarea.md)
