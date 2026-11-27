# DiffTimeline

`patterns/diff_timeline` · export `DiffTimeline` · version 0.1.0. Generated from
[`registry/patterns/diff_timeline.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/diff_timeline.rhai); do not edit.

Plot cumulative additions, deletions and signed net changes from a Host ledger.

```rhai
import "patterns/diff_timeline" as diff_timeline;

diff_timeline::DiffTimeline(#{key:"patch",run:"patch-1",agent:#{id:"ada",name:"Ada"},edits:[#{id:"import",at:3,file:"src/lib.rs",added:8,removed:1}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agent` | object | required | The agent whose changes are shown. |
| `context` | string or `()` | — | Optional context, such as a review reference. |
| `duration` | number 0–604800 or `()` | — | Optional axis length in seconds, never shorter than the last edit. |
| `edits` | array of object (at most 192) | required | Ordered ledger; ties retain the input order. Future edits set the scale, but are not exposed by inspection. |
| `elapsed` | number 0–604800 or `()` | — | The Host playback cursor. Omit it to show the completed run. |
| `key` | string | required | Stable card instance. |
| `live` | bool | `false` | Show the currently edited file, live marker and auto-follow. |
| `max_rows` | integer 1–24 | `7` | Maximum number of file rows before scrolling. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `run` | string | required | Host run identity. Changing it clears the inspected edit. |
| `style` | style | — | Style merged over the root part. |

### `agent` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable identity of the agent's mirrored dot mark. |
| `name` | string | required | Agent display name. |

### `edits[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `added` | integer 0–10000000 | required | Lines added by this edit. |
| `at` | number 0–604800 | required | Elapsed seconds at this edit. |
| `diff` | array of object (at most 16) or `()` | — | Optional caller-supplied diff excerpt. |
| `file` | string | required | Display path, not a file-system lookup. |
| `id` | string | required | Stable edit ID; timestamps can coincide. |
| `removed` | integer 0–10000000 | required | Lines removed by this edit. Zero and zero denote a binary edit. |

### `edits[].diff[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `kind` | `"add"` or `"del"` or `"ctx"` | required | Change kind. |
| `text` | string | required | Literal preview line. The inspector shows at most four. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `across`, `add`, `area_add`, `area_del`, `axis`, `axis_text`, `body`, `context`, `count_measure`, `del`, `dots`, `empty`, `file_bar`, `file_binary`, `file_count`, `file_counts`, `file_dir`, `file_edit`, `file_icon`, `file_name`, `file_panel`, `file_piece`, `file_row`, `file_track`, `files`, `footer`, `header`, `legend`, `mark`, `net`, `net_value`, `panel`, `plot`, `root`, `status`, `time_axis`, `tip_code`, `tip_code_add`, `tip_code_del`, `tip_code_panel`, `tip_code_sign`, `tip_header`, `tip_line`, `tip_path`, `tip_rows`, `tip_time`, `title`, `tooltip`, `totals`, `wash`.

## Theme

- Tokens: `accent`, `border`, `diff_timeline.add`, `diff_timeline.add_ink`, `diff_timeline.binary`, `diff_timeline.card`, `diff_timeline.code`, `diff_timeline.del`, `diff_timeline.del_ink`, `diff_timeline.faint`, `diff_timeline.grid`, `diff_timeline.image`, `diff_timeline.muted`, `diff_timeline.net`, `diff_timeline.sheet`, `diff_timeline.tray`, `diff_timeline.wash`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.code`, `typography.metadata`
