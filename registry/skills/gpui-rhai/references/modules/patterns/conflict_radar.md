# ConflictRadar

`patterns/conflict_radar` · export `ConflictRadar` · version 0.1.0. Generated from
[`registry/patterns/conflict_radar.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/conflict_radar.rhai); do not edit.

A branch activity matrix, exact line overlap, and Host-owned merge proposals.

```rhai
import "patterns/conflict_radar" as conflict_radar;

conflict_radar::ConflictRadar(#{key:"radar",session:"sync",agents:[],files:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agents` | array of object (at most 16) | required | Agent columns in display order. |
| `files` | array of object (at most 64) | required | Accepted file edits, listed in display order. |
| `key` | string | required | Stable component key. |
| `on_merge` | callback or `()` | — | Receives an explicit merge proposal when all guards pass. |
| `on_resolve` | callback or `()` | — | Receives an explicit overlap resolution proposal. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pending` | bool | `false` | Block proposing another resolution or merge while the Host decides. |
| `resolved` | map of object or `()` | — | Optional controlled resolution receipts by overlap ID. Omission allows local choices. |
| `revision` | integer ≥ 0 | `0` | Accepted Host snapshot revision carried by proposals. |
| `session` | string | required | Identity of the collaboration run; changing it resets local conflict choices. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Visible and accessible heading. |

### `agents[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `icon` | node or `()` | — | Optional decoration replacing the initials. |
| `id` | string | required | Stable unique branch owner ID. |
| `model` | string or `()` | — | Optional model or branch name. |
| `name` | string | required | Agent display name. |
| `status` | `"running"` or `"done"` | — | Unset counts as running; every agent must finish before merge is offered. |

### `files[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `edits` | array of object (at most 24) | required | Edits on this file. At most 256 edits across all files. |
| `id` | string | required | Stable unique file ID. |
| `path` | string | required | File label; the SDK does not open the path. |

### `files[].edits[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `active` | bool or `()` | — | Whether the branch is currently writing this edit. |
| `agent` | string | required | Owner ID from agents. |
| `end` | integer 1–100000000 | required | Last affected line, inclusive and not smaller than start. |
| `id` | string | required | Stable unique edit ID within the file. |
| `revision` | integer 0–4294967295 | required | Change when this edit changes, even if its range stays unchanged. |
| `start` | integer 1–100000000 | required | First affected line, inclusive and one-based. |
| `summary` | string or `()` | — | Short description of this branch's approach. |

### `resolved.<key>` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string | required | Accepted agent for this exact overlap. |
| `witness` | string | required | Opaque witness from the accepted resolve proposal. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `merge` | `on_merge` | object | Ask the Host to merge; the component never executes a merge. |
| `resolve` | `on_resolve` | object | Propose which branch to keep in one exact overlapping band. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `agent`, `avatar`, `bar`, `bar_conflict`, `bar_edit`, `bar_kept`, `bar_mark`, `bar_mixed`, `calm`, `card`, `cell`, `connection`, `decision`, `decision_head`, `decision_title`, `directory`, `file`, `head`, `header_file`, `indicator`, `kept`, `left`, `legend`, `merge`, `merge_ready`, `pager`, `path`, `root`, `row`, `row_alt`, `row_open`, `row_selected`, `summary`, `title`, `viewport`, `way`, `where`.

## Theme

- Tokens: `focus_ring`, `radar.accent`, `radar.card`, `radar.chip`, `radar.edit`, `radar.edit_edge`, `radar.faint`, `radar.fill`, `radar.hit`, `radar.hit_edge`, `radar.hit_ink`, `radar.hit_row`, `radar.hover`, `radar.ink`, `radar.inverse`, `radar.kept`, `radar.kept_edge`, `radar.kept_ink`, `radar.line`, `radar.muted`, `radar.sheen`, `radar.stripe`, `radar.text`, `radar.tray`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `typography.body`, `typography.caption`, `typography.initials`

## Dependencies

[`components/tooltip`](../components/tooltip.md)
