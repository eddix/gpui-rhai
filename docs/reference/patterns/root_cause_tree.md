# RootCauseTree

`patterns/root_cause_tree` · export `RootCauseTree` · version 0.1.0. Generated from
[`registry/patterns/root_cause_tree.rhai`](../../../registry/patterns/root_cause_tree.rhai); do not edit.

Shows a Host-confirmed fault tree, evidence and scoped action proposals.

```rhai
import "patterns/root_cause_tree" as root_cause_tree;

root_cause_tree::RootCauseTree(#{key:"review",session:"review",causes:[#{id:"failure",title:"Review could not finish"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `causes` | array of object (at most 96) | required | Flat ordered tree; sibling order follows input, and parents can occur anywhere. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `on_action` | callback or `()` | — | Fix/retry request for a proven, confirmed leaf. The component never executes it. |
| `on_selection_change` | callback or `()` | — | Receives the requested cause ID or null for automatic selection. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pending` | bool | `false` | Do not emit another action while the Host decides the previous proposal. |
| `revision` | integer ≥ 0 | `0` | Accepted investigation revision included in each action proposal. |
| `selected_id` | string or `()` | — | Optional controlled selection. An explicit null restores default selection. |
| `session` | string | required | Identity of the investigation; a new session resets local selection. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Optional heading for the whole investigation. |

### `causes[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `at_ms` | integer 0–9007199254740991 or `()` | — | Elapsed time of the accepted verdict. |
| `detail` | string or `()` | — | Short supporting caption displayed on the hypothesis card. |
| `evidence` | object or `()` | — | Optional snapshot supporting this accepted cause, separate from its status. |
| `files` | array of string (at most 24) | `[]` | Related path labels for inspection; the component does not open them. |
| `gate` | `"and"` or `"or"` | `"or"` | How this cause combines its immediate children's proofs. |
| `icon` | node or `()` | — | Optional visual mark for this cause. |
| `id` | string | required | Stable cause ID, unique in this investigation. |
| `parent` | string or `()` | — | Parent ID. Exactly one cause without a parent represents the failure. |
| `status` | `"open"` or `"checking"` or `"ruled-out"` or `"confirmed"` | `"open"` | Host's accepted verdict for a hypothesis, not an auto-generated conclusion. |
| `title` | string | required | Readable name of the failure or proposed cause. |
| `verdict` | string or `()` | — | Host's explanation for the accepted result, shown in details. |

### `causes[].evidence` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `content` | string | required | Host-supplied read-only evidence text displayed in the inspector. |
| `kind` | string | required | Heading for this evidence, for example Build log. |
| `language` | string or `()` | — | Syntax language for the evidence viewer; defaults to plain text. |
| `source` | string or `()` | — | Optional origin caption; never fetched by the widget. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Explicit requested Host action. Raw evidence and file paths never leave the widget in this event. |
| `selection_change` | `on_selection_change` | string or `()` | Selected ID, or null to restore the automatic selection. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `badge`, `button`, `card`, `card_excluded`, `card_selected`, `diagram`, `evidence`, `evidence_label`, `file`, `file_name`, `file_row`, `gate`, `guide`, `head`, `inner`, `inner_excluded`, `inner_selected`, `live`, `mark`, `meta`, `more`, `name`, `panel`, `panel_body`, `panel_head`, `panel_status`, `panel_title`, `pending`, `primary`, `root`, `source`, `status`, `summary`, `tile`, `title`, `verdict`, `wires`.

## Theme

- Tokens: `focus_ring`, `radius.lg`, `radius.sm`, `root_cause.brand`, `root_cause.brand_bg`, `root_cause.card`, `root_cause.danger`, `root_cause.danger_bg`, `root_cause.faint`, `root_cause.fill`, `root_cause.ink`, `root_cause.line`, `root_cause.meta`, `root_cause.muted`, `root_cause.on_solid`, `root_cause.path`, `root_cause.rail`, `root_cause.shine`, `root_cause.solid`, `root_cause.text`, `root_cause.tray`, `root_cause.well`, `space.spacing_unit`, `space.unit`, `text_primary`, `typography.body`, `typography.caption`, `typography.code`

## Dependencies

[`components/code_viewer`](../components/code_viewer.md), [`components/spinner`](../components/spinner.md)
