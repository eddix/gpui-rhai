# Select

`components/select` · export `Select` · version 0.4.3. Generated from
[`registry/components/select.rhai`](../../../registry/components/select.rhai); do not edit.

Scalar controlled Select composed from the public Rhai Combobox component. State: caller owns value/open/query; Combobox owns only active-option navigation.

```rhai
import "components/select" as select;

select::Select(#{ key: "country", label: "Country", options: countries, value: "cn", open: false, query: "", on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `clear_label` | string or `()` | — | Accessible name of the clear button shown with `clearable`; unset, the localized `common.clear` text. |
| `clearable` | bool | `false` | Shows a clear button in the trigger while a value is selected; clicking it emits `change` with `()`. |
| `disabled` | bool | `false` | Disables the trigger so the list cannot open. |
| `empty_text` | string | `""` | Text shown when no option matches; empty uses the localized `common.no_results` message. |
| `error` | bool | `false` | Marks the value invalid: the trigger border turns `danger` and accessibility reports it invalid. |
| `key` | string | required | Identity of the select; its inner Combobox, and so the list's overlay id, is keyed `<key>-combobox`. |
| `label` | string | required | Accessible name of the trigger and the list. |
| `max_visible` | integer 1–32 | `8` | Rows the list shows before it scrolls; group headings count as rows. |
| `on_change` | callback or `()` | — | Called with the picked value, or `()` when the value is cleared. |
| `on_open_change` | callback or `()` | — | Called with the requested open state when the trigger is clicked, an arrow key opens the list, an option is picked, or it is dismissed. |
| `on_query_change` | callback or `()` | — | Called with the new search text as the user types, and with `""` when the list closes or the value is cleared. |
| `open` | bool | required | Whether the list is open; the caller stores the `open_change` payload and passes it back. |
| `option_trailing` | callback or `()` | — | Node factory bound with ctx.node_factory(Fn(...)), called as Fn(ctx, option) for content after an option's label, such as a Tag, in the list and beside the trigger's value; option is #{ value, label, selected, active, disabled, place }, `place` being `option` or `trigger`. Curry the caller's data into the factory to look an option up by value. |
| `options` | array of object (at most 100000) | required | Options in display order; options that share a `group` are gathered where that group first appears. |
| `panel_width` | `"trigger"` or `"content"` | `"content"` | `content` makes the list at least as wide as the trigger and wider for longer options, measured up to 480 logical pixels; `trigger` makes it exactly as wide as the trigger. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placeholder` | string | `""` | Trigger text while nothing is selected. |
| `placement` | `"top"` or `"bottom"` or `"left"` or `"right"` | `"bottom"` | Side of the trigger the list opens on. |
| `query` | string | required | Search text that filters the options; the caller stores the `query_change` payload and passes it back. |
| `search_placeholder` | string | `"Search"` | Placeholder of the search field shown with `searchable`. |
| `searchable` | bool | `false` | Shows a search field at the top of the list; without it, letter keys jump to matching options. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size; leave unset to inherit the environment size. |
| `style` | style | — | Style merged over the root part. |
| `value` | string or `()` | — | Selected option value, or `()` for none, shown as is when no option has it; the caller stores the `change` payload and passes it back. |
| `width` | length or `"content"` or `()` | — | Width of the trigger. Unset or `"content"`, it fits the widest option and the placeholder, at most 360 logical pixels, and ends a longer value in an ellipsis; past 64 options it measures the 16 longest labels. A stretching column still stretches it, up to that cap. |

### `options[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Shows the option but blocks picking it; keyboard navigation skips it. |
| `group` | string or `()` | — | Heading this option is listed under; must not be blank. |
| `keywords` | array of string (at most 32) | `[]` | Extra terms the search query also matches, case-insensitively. |
| `label` | string | required | Option text, also matched by the search query; must not be blank. |
| `value` | string | required | Value that `change` reports when this option is picked; must be unique. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | string or `()` | Emitted when the user picks a different option or clears the value; the payload is the new value or `()`. |
| `open_change` | `on_open_change` | bool | Emitted when the list should open or close; the payload is the requested open state. |
| `query_change` | `on_query_change` | string | Emitted when the search text changes or is reset on close; the payload is the new query. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `clear`, `empty`, `group`, `indicator`, `list`, `option`, `option_active`, `option_disabled`, `option_selected`, `panel`, `placeholder`, `root`, `search`, `trailing`, `trigger`, `value`.

## Theme

- Tokens: `border`, `danger`
- Environment: `density`, `size`

## Dependencies

[`components/combobox`](../components/combobox.md)
