# TabOverflowMenu

`components/tab_overflow_menu` · export `TabOverflowMenu` · version 0.4.0. Generated from
[`registry/components/tab_overflow_menu.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/tab_overflow_menu.rhai); do not edit.

A chevron button whose menu lists every tab of a strip; picking one reports its value. It owns the menu's open state and active row, so the tabs beside it can stay stateless: Tabs uses it for `overflow_menu`, and a caller may put it beside any strip of tabs.

```rhai
import "components/tab_overflow_menu" as tab_overflow_menu;

tab_overflow_menu::TabOverflowMenu(#{ key: "files-menu", label: "All tabs", value: current,
    items: [#{ value: "main", label: "main.rs", disabled: false }], on_pick: Fn("opened") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `items` | array of object (at most 256) | required | Every tab, in strip order. |
| `key` | string | required | Stable identity of the menu, which keeps its open state. |
| `label` | string | required | Accessible name of the button and its menu. |
| `on_pick` | callback or `()` | — | Called with the `value` of the picked tab; the menu closes. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `value` | string | required | The selected tab, where the menu's highlight starts each time it opens. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Whether the row is skipped and cannot be picked. |
| `label` | string | required | The tab's label, shown in the menu. |
| `value` | string | required | The tab's value, reported when it is picked. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `pick` | `on_pick` | string | Emitted when a row is picked; the payload is its `value`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `trigger`.

## Theme

- Tokens: `metrics.control`, `metrics.icon`, `surface_hover`, `tabs.foreground`, `text_primary`
- Environment: `density`, `size`

## Dependencies

[`components/menu`](menu.md)
