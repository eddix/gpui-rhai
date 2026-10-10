# Combobox

`components/combobox` · export `Combobox` · version 0.2.0. Generated from
[`registry/components/combobox.rhai`](../../../registry/components/combobox.rhai); do not edit.

Public Rhai choice composition over Overlay, Input, and virtual_collection. State: caller owns selected/open/query; component owns only active-option navigation.

```rhai
import "components/combobox" as combobox;

combobox::Combobox(#{ key: "country", label: "Country", options: options, selected: ["uk"], open: false, query: "", on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `clear_label` | string or `()` | — | Accessible name of the clear button shown with `clearable`; unset, the localized `common.clear` text. |
| `clearable` | bool | `false` | Shows a clear button in the default trigger while something is selected; it emits `change` with `[]`. |
| `disabled` | bool | `false` | Disables the trigger so the panel cannot open. |
| `empty` | node or `()` | — | Content shown instead of `empty_text` when no option matches. |
| `empty_text` | string | `"No results"` | Text shown when no option matches; the `empty` slot replaces it. |
| `footer` | node or `()` | — | Content at the bottom of the panel, below the options. |
| `header` | node or `()` | — | Content at the top of the panel, above the search field. |
| `key` | string | required | Identity of the combobox; also its overlay id and the prefix of its inner keys (`<key>-trigger`, `<key>-panel`). |
| `label` | string | required | Accessible name of the trigger, the list and the search field. |
| `max_visible` | integer 1–32 | `8` | Rows the list shows before it scrolls; group headings count as rows. |
| `mode` | `"single"` or `"multiple"` | `"single"` | With `single`, a pick replaces the selection and closes the panel; with `multiple`, picks toggle values and it stays open. |
| `on_change` | callback or `()` | — | Called with the full new selection when an option is picked or the selection is cleared. |
| `on_open_change` | callback or `()` | — | Called with the requested open state when the trigger is clicked, an arrow key opens it, a `single` pick closes it, or it is dismissed. |
| `on_query_change` | callback or `()` | — | Called with the new search text as the user types, and with `""` when `reset_query_on_close` resets it. |
| `open` | bool | required | Whether the panel is open; the caller stores the `open_change` payload and passes it back. |
| `options` | array of object (at most 100000) | required | Options in display order; options that share a `group` are gathered where that group first appears. |
| `parent_overlay` | string or `()` | — | Key of the open overlay this one nests in, such as a Dialog; the panel stacks above it and closes with it. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placeholder` | string | `""` | Trigger text while nothing is selected. |
| `placement` | `"top"` or `"bottom"` or `"left"` or `"right"` | `"bottom"` | Side of the trigger the panel opens on. |
| `query` | string | required | Filters options by label or keyword, case-insensitively; the caller stores the `query_change` payload and passes it back. |
| `reset_query_on_close` | bool | `false` | Emits `query_change` with `""` whenever the panel closes or the selection is cleared. |
| `search_placeholder` | string | `"Search"` | Placeholder of the search field. |
| `searchable` | bool | `false` | Shows a search field bound to `query` in the panel; without it, letter keys jump to matching options. |
| `selected` | array of string (at most 100000) | required | Selected option values, at most one in `single` mode; the caller stores the `change` payload and passes it back. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size; leave unset to inherit the environment size. |
| `style` | style | — | Style merged over the root part. |
| `trigger` | node or `()` | — | Custom trigger that replaces the default field with its clear button and chevron; the panel then takes `width`. |
| `width` | length or `()` | — | Width of the default trigger, which the panel matches, or of the panel under a custom `trigger`; unset is 280 logical pixels. |

### `options[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Shows the option but blocks picking it; keyboard navigation skips it. |
| `group` | string or `()` | — | Heading this option is listed under, shown in capitals; must not be blank. |
| `keywords` | array of string (at most 32) | `[]` | Extra terms the query also matches, case-insensitively. |
| `label` | string | required | Option text, matched by the query and joined into the trigger text when selected; must not be blank. |
| `value` | string | required | Value stored in `selected` when this option is picked; must be unique. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | array of string (at most 100000) | Emitted when a pick changes the selection or the selection is cleared; the payload is the full new selection. |
| `open_change` | `on_open_change` | bool | Emitted when the panel should open or close; the payload is the requested open state. |
| `query_change` | `on_query_change` | string | Emitted when the search text changes, or resets to `""` under `reset_query_on_close`; the payload is the new query. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `empty` | no | no | Content shown when no option matches. |
| `footer` | no | no | Content below the options. |
| `header` | no | no | Content above the search field. |
| `trigger` | no | no | Replaces the default field trigger. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `check`, `clear`, `empty`, `focus_frame`, `group`, `indicator`, `indicator_bar`, `list`, `option`, `option_active`, `option_disabled`, `option_selected`, `panel`, `placeholder`, `root`, `search`, `trigger`, `value`.

## Theme

- Tokens: `accent`, `border`, `disabled`, `focus_ring`, `metrics.control`, `metrics.field_pad`, `metrics.icon`, `metrics.inset`, `metrics.row`, `radius.lg`, `radius.md`, `radius.sm`, `selection`, `spacing.sm`, `spacing.xs`, `surface_hover`, `surface_raised`, `text.accent`, `text_muted`, `text_primary`, `typography.body`, `typography.control`, `typography.label`
- Environment: `corners`, `density`, `size`

## Dependencies

[`components/input`](../components/input.md)
