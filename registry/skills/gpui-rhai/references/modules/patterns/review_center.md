# ReviewCenter

`patterns/review_center` · export `ReviewCenter` · version 0.1.0. Generated from
[`registry/patterns/review_center.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/review_center.rhai); do not edit.

Prioritized review queue with versioned proposals and independently inspectable evidence. State: filter, selection, one edit session, file and reason disclosure. Accepted decisions and receipts are Host-owned.

```rhai
import "patterns/review_center" as review_center;

review_center::ReviewCenter(#{key:"review",now_ms:1000,items:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `default_filter` | `"all"` or `"high"` or `"unsure"` | `"all"` | Initial filter when filter is not controlled. |
| `filter` | `"all"` or `"high"` or `"unsure"` or `()` | — | Controlled filter. The queue changes only when this value changes. |
| `items` | array of object (at most 48) | required | Pending requests. The Host removes an accepted decision. |
| `key` | string | required | Stable instance key. |
| `low_confidence` | number 0–1 | `0.7` | Threshold for the Unsure filter. |
| `now_ms` | integer ≥ 0 | required | Host clock for wait time, SLA and undo expiry; no component-owned wall timer. |
| `on_decide` | callback or `()` | — | Versioned approval or rejection proposal. |
| `on_filter_change` | callback or `()` | — | Optional filter selection proposal. |
| `on_skip` | callback or `()` | — | Optional notification of a presentation-only skip. |
| `on_source` | callback or `()` | — | Activation of a source identity for the current revision. |
| `on_undo` | callback or `()` | — | Proposed undo of the exact accepted receipt, never optimistic restoration. |
| `order` | `"risk"` or `"oldest"` or `"given"` | `"risk"` | Queue ordering: risk then oldest, oldest regardless of risk, or caller order. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `reasons` | array of object (at most 8) or `()` | — | Omit for preset reasons. An empty list enables rejecting without a reason. |
| `receipt` | object or `()` | — | Most recent accepted decision that the Host still permits undoing. |
| `reviewed` | integer 0–9000000000000000 | `0` | Accepted review count. |
| `sla_ms` | integer ≥ 1 | `1800000` | Wait limit; 75% is near deadline and 100% is overdue. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Queue heading. |
| `total` | integer ≥ 0 or `()` | — | Optional total for progress; at least reviewed plus pending. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string | required | Who proposed this request. |
| `body` | any value | — | Proposal text or a list of plain and #{text,note,source?} evidence spans. Text edits propose a plain string. |
| `body_label` | string or `()` | — | Optional caption above proposal text. |
| `busy` | bool or `()` | — | Host is considering a proposal; decision actions are disabled for this request. |
| `confidence` | number 0–1 | required | Agent confidence, from 0 to 1. |
| `created_ms` | integer ≥ 0 | required | Wait start on the Host clock; breaks ties by oldest first. |
| `fields` | array of object (at most 12) or `()` | — | Facts supplied by the Host; only changed values are included in approval proposals. |
| `files` | array of object (at most 12) or `()` | — | File comparisons are read-only and use DiffViewer; changes are never applied by this component. |
| `id` | string | required | Unique stable request ID. |
| `kind` | string | required | Task type caption such as Refund, Code change or Plan. |
| `model` | string or `()` | — | Optional model name. |
| `notice` | string or `()` | — | Non-final Host feedback. A refused proposal stays in this queue. |
| `reasoning` | string or `()` | — | Explanation for the proposal. |
| `revision` | integer ≥ 0 | required | Increase when business evidence changes. |
| `risk` | `"low"` or `"medium"` or `"high"` | required | Risk category; higher risks sort first. |
| `sources` | array of object (at most 12) or `()` | — | Evidence labels; optional source activation is a Host proposal, not external navigation. |
| `steps` | array of object (at most 20) or `()` | — | Ordered plan evidence. |
| `title` | string | required | Request name in both panels. |

### `items[].fields[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Unique editable field identity, independent of its label. |
| `label` | string | required | Visible field caption. |
| `value` | any value | required | Plain text or flagged evidence spans, like body. |

### `items[].files[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `added` | integer ≥ 0 or `()` | — | Optional added line count supplied by the Host. |
| `after` | string or native document | required | Proposed text or Host-owned native document. |
| `before` | string or native document | required | Before text or Host-owned native document. |
| `id` | string | required | Unique file ID. |
| `name` | string | required | Display path and syntax hint; never read from disk. |
| `removed` | integer ≥ 0 or `()` | — | Optional removed line count supplied by the Host. |

### `items[].sources[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Unique source identity. |
| `label` | string | required | Visible source name. |

### `items[].steps[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `detail` | any value | — | Optional plain or flagged step explanation. |
| `title` | any value | required | Plain or flagged step title. |
| `tool` | string or `()` | — | Tool caption; never invoked by the viewer. |

### `reasons[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable reject reason ID. |
| `label` | string | required | Caption shown to the reviewer. |

### `receipt` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `expires_ms` | integer ≥ 0 | required | Undo no longer offered when now_ms reaches this time. |
| `id` | string | required | Accepted receipt identity, never the request title. |
| `outcome` | `"approved"` or `"rejected"` | required | Outcome the Host accepted. |
| `request` | string | required | Original request ID. |
| `revision` | integer ≥ 0 | required | Accepted decision revision. |
| `title` | string | required | Request caption in the receipt. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `decide` | `on_decide` | object | Approval and rejection are proposals; only the Host changes the queue. |
| `filter_change` | `on_filter_change` | `"all"` or `"high"` or `"unsure"` | Filter chosen by the user. |
| `skip` | `on_skip` | object | Skip moves selection without resolving or removing the request. |
| `source` | `on_source` | object | The Host resolves an allowed source target, if any. |
| `undo` | `on_undo` | object | The Host validates expiry and decides whether to restore the request. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `avatar`, `body`, `card`, `count`, `detail`, `detail_pane`, `detail_scroll`, `diff`, `editor`, `effect`, `empty`, `field`, `field_label`, `field_value`, `file`, `file_active`, `files`, `filter_active`, `filters`, `flag`, `flag_label`, `flag_tip`, `head`, `icon`, `item_title`, `list`, `meta`, `notice`, `oldest`, `pager`, `progress`, `queue`, `reason`, `receipt`, `risk`, `risk_dot`, `risk_high`, `risk_low`, `risk_medium`, `root`, `row`, `row_active`, `row_title`, `section`, `section_title`, `source`, `source_button`, `sources`, `step`, `step_no`, `steps`, `title`, `tool`, `why`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `radius.lg`, `selection`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.review_title`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/diff_viewer`](../components/diff_viewer.md), [`components/icon_button`](../components/icon_button.md), [`components/input`](../components/input.md), [`components/progress`](../components/progress.md), [`components/scroll_area`](../components/scroll_area.md), [`components/textarea`](../components/textarea.md), [`patterns/approval_card`](approval_card.md), [`patterns/decision_card`](decision_card.md), [`patterns/decision_tree`](decision_tree.md)
