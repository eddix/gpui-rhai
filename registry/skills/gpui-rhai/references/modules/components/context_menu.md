# ContextMenu

`components/context_menu` · export `ContextMenu` · version 0.2.0. Generated from
[`registry/components/context_menu.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/context_menu.rhai); do not edit.

Controlled pointer-anchored action menu sharing Menu items and keyboard behavior.

State: caller owns open/active values; component retains only the last pointer anchor.

```rhai
import "components/context_menu" as context_menu;

context_menu::ContextMenu(#{ key: "row-menu", label: "Row actions", trigger: row_node, open: open, active_value: "copy", items: items, on_open_change: Fn("opened") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `active_value` | string | required | `value` of the highlighted item, which Enter activates; the caller stores the `active_change` payload. |
| `items` | array of object (at most 512) | required | Rows of the menu, in order, as in Menu. |
| `key` | string | required | Stable identity of this instance; also the overlay id of its menu, which a submenu names in `parent_overlay`. |
| `label` | string | required | Accessible name of the menu panel. |
| `on_action` | callback or `()` | — | Called with the item `value` when a row is clicked or activated with Enter; rows without an `action` leave the menu open. |
| `on_active_change` | callback or `()` | — | Called with the item `value` to highlight on Up, Down or a letter key; store it as `active_value`. |
| `on_open_change` | callback or `()` | — | Called with `true` on a right-button press on the trigger and `false` on dismissal or after an `action` row; store it as `open`. |
| `open` | bool | required | Whether the menu is shown; the caller stores the `open_change` payload and passes it back. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placement` | `"top"` or `"bottom"` or `"left"` or `"right"` | `"bottom"` | Preferred side of the pointer position that the menu opens on. |
| `style` | style | — | Style merged over the root part. |
| `trigger` | node | required | Area that opens the menu at the pointer on a right-button press; it takes no focus and ignores left clicks. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `action` | string or `()` | — | Registered action the row dispatches before closing the menu; it also supplies the legend and enabled state. |
| `checked` | bool | `false` | Shows the check mark at the row start and reports the row as checked. |
| `disabled` | bool | `false` | Dims the row; it cannot be clicked and keyboard navigation skips it. |
| `kind` | `"item"` or `"separator"` or `"submenu"` or `"label"` | required | `item` is an activatable row, `separator` a divider, `label` a heading, `submenu` a nested menu row. |
| `label` | string or `()` | — | Row text, also matched by letter-key type-ahead; on a `label` row, the heading text. |
| `shortcut` | string or `()` | — | Key legend shown at the row end, written like `cmd-c`; overrides the binding of `action`. |
| `submenu` | node or `()` | — | Nested Menu node drawn for a `submenu` row; set its `parent_overlay` to this menu's `key`. |
| `value` | string or `()` | — | Identifier passed to `on_action` and used as `active_value`; rows without one are skipped by the keys. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | string | Emitted when a row is clicked or activated with Enter; the payload is the item `value`. |
| `active_change` | `on_active_change` | string | Emitted when Up, Down or a letter key moves the highlight; the payload is the new item `value`. |
| `open_change` | `on_open_change` | bool | Emitted when a right-button press opens the menu or it closes; the payload is the requested `open`. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `trigger` | yes | no | Area that opens the menu on a right-button press. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `item`, `root`, `separator`, `shortcut`, `submenu`, `trigger`.

## Theme

- Environment: `density`

## Dependencies

[`components/menu`](menu.md)
