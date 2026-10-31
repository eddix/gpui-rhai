# Timeline

`components/timeline` · export `Timeline` · version 0.4.0. Generated from
[`registry/components/timeline.rhai`](../../../registry/components/timeline.rhai); do not edit.

A source-owned, controlled timeline. Application work and data loading stay outside it.

```rhai
import "components/timeline" as timeline;

timeline::Timeline(#{key:"runs",label:"Deployment",items:[#{key:"build",title:"Build",status:"success"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `day_headers` | bool | `false` | Adds localized day headings to time-ordered feeds. |
| `details` | map of node | `#{}` | Optional detail nodes keyed by item key, including nested records. |
| `expanded` | array of string (at most 10000) | `[]` | Controlled expanded group keys; completed groups may start folded. |
| `height` | number > 0 | `320` | Virtualized viewport height in logical pixels. |
| `items` | array of any value (at most 10000) | required | Nested keyed records: title, status, start/end epoch milliseconds, current, description, meta and children; keys are globally unique. |
| `key` | string | required | Stable timeline identity. |
| `label` | string | required | Accessible timeline name. |
| `on_expanded_change` | callback or `()` | — | Called with proposed expanded keys on disclosure or Left/Right. |
| `on_item_click` | callback or `()` | — | Called with the activated item key. |
| `on_selection_change` | callback or `()` | — | Called with proposed selected keys on row activation or navigation. |
| `order` | `"ascending"` or `"descending"` | `"ascending"` | Sibling source order or its reverse, without reordering nested groups. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `selected_keys` | array of string (at most 1) | `[]` | Controlled single selection. |
| `selection_mode` | `"none"` or `"single"` | `"none"` | Single selection makes the timeline one tab stop; none still permits group folding. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Explicit size or inherited size. |
| `style` | style | — | Style merged over the root part. |
| `time_display` | `"absolute"` or `"relative"` or `"duration"` | `"absolute"` | Time column presentation; skewed durations and future relative times fall back to absolute. |
| `time_zone` | string | `"local"` | local, UTC, offset:<minutes>, or IANA timezone. |
| `visible` | bool or `()` | — | Optional application visibility override. By default ctx.is_visible() pauses presentation ticks on hidden retained pages. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `expanded_change` | `on_expanded_change` | array of string (at most 10000) | Proposed expansion after group activation. |
| `item_click` | `on_item_click` | string | Activated event key. |
| `selection_change` | `on_selection_change` | array of string (at most 1) | Proposed single selection. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `cursor`, `day_header`, `description`, `detail`, `disclosure`, `lamp`, `meta`, `rail`, `root`, `row`, `time`, `title`.

## Theme

- Tokens: `accent`, `border`, `danger`, `focus_ring`, `metrics.control`, `metrics.inset`, `metrics.label_column`, `metrics.row`, `radius.md`, `selection`, `spacing.lg`, `spacing.sm`, `spacing.xs`, `success`, `surface_hover`, `text.accent`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.label`, `warning`
- Environment: `density`, `size`
