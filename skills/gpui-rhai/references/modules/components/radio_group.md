# RadioGroup

`components/radio_group` · export `RadioGroup` · version 0.2.0. Generated from
[`registry/components/radio_group.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/radio_group.rhai); do not edit.

Controlled radio group with one tab stop and roving arrow-key selection. State: controlled by the value prop.

```rhai
import "components/radio_group" as radio_group;

radio_group::RadioGroup(#{ value: "one", label: "Choice", options: options, on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Disables every option and takes the group out of the tab order. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the group. |
| `on_change` | callback or `()` | — | Called with the chosen option's value on click or arrow key; without it the group renders disabled. |
| `options` | array of object (at most 256) | required | Options in display order. |
| `orientation` | `"horizontal"` or `"vertical"` | `"vertical"` | Stacks options in a column, or lays them in a row spaced by `space.group`. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `value` | string | required | Value of the selected option; the caller stores the `change` payload and passes it back. |

### `options[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Blocks clicks on this option; arrow keys skip it. |
| `label` | string | required | Text beside the option's circle; also its accessible name. |
| `value` | string | required | Value that `change` reports when this option is chosen. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | string | Emitted when an option is clicked or an arrow key selects the previous or next enabled one; the payload is the new value. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `option`, `root`.

## Theme

- Tokens: `space.group`
- Environment: `density`, `size`

## Dependencies

[`components/radio`](radio.md)
