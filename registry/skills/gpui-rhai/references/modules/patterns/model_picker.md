# ModelPicker

`patterns/model_picker` · export `ModelPicker` · version 0.1.0. Generated from
[`registry/patterns/model_picker.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/model_picker.rhai); do not edit.

ModelPicker presents the application-provided model list and request options. State: settings and open state have independent controlled or local ownership.

```rhai
import "patterns/model_picker" as model_picker;

model_picker::ModelPicker(#{key:"assistant",models:[#{id:"auto",name:"Auto"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `align` | `"start"` or `"end"` | `"start"` | Logical alignment of the panel with the trigger. |
| `disabled` | bool | `false` | Prevent interaction and remove the open panel. |
| `key` | string | required | Stable picker and overlay identity. |
| `models` | array of object (at most 64) | required | Display data only; stats are application estimates. |
| `on_change` | callback or `()` | — | Full proposed configuration. |
| `on_open_change` | callback or `()` | — | Proposed overlay open state. |
| `open` | bool or `()` | — | Accepted overlay state; omit for local disclosure. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placement` | `"top"` or `"bottom"` | `"bottom"` | Prefer top when used near the bottom of a Composer. |
| `style` | style | — | Style merged over the root part. |
| `value` | object or `()` | — | Accepted configuration; omit for local settings. |

### `models[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `cost` | integer 1–5 or `()` | — | Relative cost score; five is most expensive. |
| `description` | string or `()` | — | Brief subtitle and accessible description. |
| `disabled` | bool | `false` | Keep the model visible but unavailable for selection. |
| `icon` | node or `()` | — | Decorative icon. The default tile shows an initial. |
| `id` | string | required | Unique nonempty model ID. |
| `latency_ms` | number 0–86400000 or `()` | — | Application estimate for first-token latency in milliseconds. |
| `name` | string | required | Display name. |
| `price_per_million` | number 0–1000000 or `()` | — | Application estimate in US dollars per million tokens. |
| `quality` | integer 1–5 or `()` | — | Relative quality score; five is highest. |
| `speed` | integer 1–5 or `()` | — | Relative speed score; five is fastest. |

### `value` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `effort` | `"low"` or `"medium"` or `"high"` | required | Requested reasoning effort, displayed while thinking is enabled. |
| `model` | string | required | Accepted model ID; empty means none. |
| `search` | bool | required | Whether web search is enabled. |
| `thinking` | bool | required | Whether extended thinking is enabled. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | object | Application-owned configuration proposal. |
| `open_change` | `on_open_change` | bool | Application-owned overlay proposal. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `active`, `description`, `effort`, `effort_label`, `empty`, `footer`, `glyph`, `head_label`, `heading`, `item`, `item_name`, `list`, `model_icon`, `model_icon_fallback`, `name`, `panel`, `root`, `score`, `score_on`, `scores`, `segment`, `segment_selected`, `segments`, `selected`, `settings`, `stat`, `switch`, `trigger`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `radius.md`, `selection`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.caption`, `typography.control`
- Environment: `density`

## Dependencies

[`components/accordion`](../components/accordion.md), [`components/popover`](../components/popover.md), [`components/switch`](../components/switch.md)
