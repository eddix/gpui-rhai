# ChangeSummary

`patterns/change_summary` · export `ChangeSummary` · version 0.1.0. Generated from
[`registry/patterns/change_summary.rhai`](../../../registry/patterns/change_summary.rhai); do not edit.

Summarize accepted changes with separate direction and outcome.

```rhai
import "patterns/change_summary" as change_summary;

change_summary::ChangeSummary(#{key:"weekly",period:"This week",changes:[#{id:"latency",label:"Latency",change:"Down 14 ms to 62 ms",tone:"good",direction:"down"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `changes` | array of object (at most 64) | required | A bounded ordered list of accepted metric changes. Empty renders only the heading. |
| `icon` | node or `()` | — | Decorative header icon in the fixed icon slot. |
| `key` | string | required | Stable summary key. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `period` | string or `()` | — | Optional comparison period shown next to the heading. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Summary heading; defaults to What's changed. |

### `changes[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `change` | string | required | Human-readable change. No numerical direction is inferred from its text. |
| `direction` | `"up"` or `"down"` or `()` | — | Arrow direction. Defaults to up for good, down for bad; neutral always uses a dash. |
| `id` | string | required | Unique stable row identity, independent of display label and rank. |
| `label` | string | required | Name of the metric in the caller's locale. |
| `tone` | `"good"` or `"bad"` or `"neutral"` or `()` | — | Outcome, independent of arrow direction; defaults to neutral. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `bad`, `change`, `good`, `head`, `head_narrow`, `heading`, `label`, `list`, `mark`, `neutral`, `period`, `root`, `row`, `row_divided`, `row_narrow`, `text`, `title`, `trend`.

## Theme

- Tokens: `accent`, `border`, `change_summary.bad`, `change_summary.good`, `change_summary.icon`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.metadata`

## Dependencies

[`patterns/agent_tree`](../patterns/agent_tree.md)
