# FilterMenu

`components/filter_menu` · export `FilterMenu` · version 0.1.0. Generated from
[`registry/components/filter_menu.rhai`](../../../registry/components/filter_menu.rhai); do not edit.

A grouped multi-select filter surface. The caller owns filtering and data.

```rhai
import "components/filter_menu" as filter_menu;

filter_menu::FilterMenu(#{ key:"filters",groups:[#{id:"state",label:"State",options:[#{id:"open",label:"Open"}]}],value:#{} })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `default_open` | array of string (at most 32) or `()` | — | Initially expanded groups; unset opens the first group, [] opens none. |
| `default_value` | map of array of string (at most 128) | `#{}` | Initial selections when value is unset. |
| `disabled` | bool | `false` | Disables navigation and all edits without hiding applied selections. |
| `groups` | array of object (at most 32) | required | Groups and options in display and proposal order. |
| `hints` | bool | `true` | Draw keyboard hints below the groups. |
| `key` | string | required | Stable identity for this filter surface. |
| `on_change` | callback or `()` | — | Receives the complete proposed selection map from a toggle, removal or clear. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Visible and accessible title. Defaults to localized Filters. |
| `value` | map of array of string (at most 128) or `()` | — | Controlled selections by group ID. Empty groups are omitted in proposals. Unknown keys are not presented. |

### `groups[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `chip_prefix` | string or `()` | — | Optional prefix in the selected chip, not part of the value. |
| `id` | string | required | Stable, unique group ID. |
| `label` | string | required | Display name of the group. |
| `options` | array of object (at most 128) | required | Ordered options. At most 256 options across the surface. |

### `groups[].options[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Remains visible but cannot be toggled or used as a roving target. |
| `id` | string | required | Stable ID unique within this group. |
| `label` | string | required | Display name of the option. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | map of array of string (at most 128) | A complete selection proposal, in definition order within each group. Expand/collapse does not emit. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `card`, `chevron`, `chip`, `chip_body`, `chip_icon`, `chip_label`, `chip_remove`, `chips`, `choice_box`, `choice_control`, `choice_mark`, `choice_on`, `clear`, `group`, `group_label`, `groups`, `header`, `hint`, `hint_key`, `hints`, `more`, `option`, `option_label`, `options`, `root`, `selected_count`, `title`, `total`, `trigger`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `radius.md`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.caption`, `typography.control`, `typography.filter_title`
- Environment: `density`

## Dependencies

[`components/accordion`](../components/accordion.md), [`components/checkbox`](../components/checkbox.md), [`components/kbd`](../components/kbd.md), [`components/tag`](../components/tag.md)
