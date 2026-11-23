# ThinkingState

`components/thinking_state` · export `ThinkingState` · version 0.1.0. Generated from
[`registry/components/thinking_state.rhai`](../../../registry/components/thinking_state.rhai); do not edit.

ThinkingState presents application-visible workflow steps and their evidence. State: steps and status are fully controlled; expansion may be local.

```rhai
import "components/thinking_state" as thinking_state;

thinking_state::ThinkingState(#{key:"steps",steps:[#{id:"verify",label:"Inspect build log"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `default_open` | bool | `true` | Initial local disclosure state. |
| `done_label` | string or `()` | — | Text of the terminal success step. |
| `elapsed_ms` | integer 0–9000000000000 or `()` | — | Host-supplied elapsed duration; displayed without maintaining a second clock. |
| `key` | string | required | Stable identity of the work summary. |
| `label` | string or `()` | — | Heading while work is active. |
| `on_open_change` | callback or `()` | — | Proposed disclosure boolean. |
| `on_open_source` | callback or `()` | — | Explicit source activation. Receives #{step,id,href}. |
| `open` | bool or `()` | — | Accepted disclosure state; omit to keep disclosure local. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `status` | `"working"` or `"done"` or `"stopped"` or `"failed"` | `"working"` | Accepted aggregate status. Terminal states stop decorative activity. |
| `steps` | array of object (at most 48) | `[]` | Public steps in display order. The widget never synthesizes workflow events. |
| `style` | style | — | Style merged over the root part. |

### `steps[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `active_label` | string or `()` | — | Optional caption while this step is active. |
| `id` | string | required | Stable nonempty step ID. Must be unique. |
| `kind` | `"step"` or `"search"` | — | An ordinary step or a search with a distinct marker. |
| `label` | string | required | Public description of the work, not internal model reasoning. |
| `sources` | array of object (at most 16) | — | Application supplied sources for this step. |
| `state` | `"running"` or `"done"` or `"failed"` | — | Accepted result of this step; defaults to done. |

### `steps[].sources[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `extra` | bool | — | Hide behind the +N button until expanded. |
| `hint` | string or `()` | — | Optional pointer and keyboard tooltip. |
| `href` | string or `()` | — | Opaque destination; the component never opens it. |
| `icon` | node or `()` | — | Decorative source icon. |
| `id` | string | required | Nonempty source ID, unique within the step. |
| `label` | string | required | Source chip text. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `open_change` | `on_open_change` | bool | Proposed disclosure open state. |
| `open_source` | `on_open_source` | object | Only emitted for explicit activation of a source. Host decides what to do. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `active`, `active_dot`, `body`, `chevron`, `copy`, `done_mark`, `elapsed`, `failed_mark`, `head_label`, `head_mark`, `header`, `marker`, `more`, `rail`, `root`, `row`, `sheen`, `source`, `source_icon`, `source_title`, `sources`, `terminal`, `title`.

## Theme

- Tokens: `border`, `focus_ring`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text_muted`, `text_primary`, `typography.caption`, `typography.control`
- Environment: `density`

## Dependencies

[`components/accordion`](../components/accordion.md), [`components/thinking_loader`](../components/thinking_loader.md), [`components/tooltip`](../components/tooltip.md)
