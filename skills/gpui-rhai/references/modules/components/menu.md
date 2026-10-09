# Menu

`components/menu` · export `Menu` · version 0.2.0. Generated from
[`registry/components/menu.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/menu.rhai); do not edit.

Controlled in-window menu with roving selection and nested submenu nodes.

State: controlled open/active values.

```rhai
import "components/menu" as menu;

menu::Menu(#{ key: "file", label: "File menu", trigger: text("File"), open: true, active_value: "open", items: items })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `activate_on_trigger` | bool | `true` | Lets a click, Enter or Space on the trigger toggle the menu; when off the menu opens only through `open`. |
| `active_value` | string | required | `value` of the highlighted item, which Enter activates; the caller stores the `active_change` payload. |
| `anchor` | object or `()` | — | Window rectangle the menu is placed against instead of the trigger, such as the pointer position. |
| `initial_focus` | `"panel"` or `"first"` | `"panel"` | Where focus lands on each open: `panel` for arrow-key menus, `first` for the first focusable element inside. |
| `items` | array of object (at most 512) | required | Rows of the menu, in order. |
| `key` | string | required | Stable identity of this instance; also the overlay id a submenu names in `parent_overlay`. |
| `label` | string | required | Accessible name of the menu panel. |
| `on_action` | callback or `()` | — | Called with the item `value` when a row is clicked or activated with Enter; rows without an `action` leave the menu open. |
| `on_active_change` | callback or `()` | — | Called with the item `value` to highlight on Up, Down or a letter key; store it as `active_value`. |
| `on_context_request` | callback or `()` | — | Called with the pointer event when the trigger is pressed with the right button. |
| `on_open_change` | callback or `()` | — | Called with `true` when the trigger opens the menu and `false` on dismissal or after an `action` row; store it as `open`. |
| `open` | bool | required | Whether the menu is shown; the caller stores the `open_change` payload and passes it back. |
| `parent_overlay` | string or `()` | — | `key` of the open menu this one is a submenu of; closing that menu closes this one too. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placement` | `"top"` or `"bottom"` or `"left"` or `"right"` | `"bottom"` | Preferred side of the trigger, or of `anchor`, that the menu opens on. |
| `style` | style | — | Style merged over the root part. |
| `trigger` | node | required | Node that anchors the menu; clicking it, or Enter and Space on it, toggles the menu. |

### `anchor` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `height` | number ≥ 0 | `0.0` | Anchor height in logical pixels; 0 anchors the menu to a point. |
| `width` | number ≥ 0 | `0.0` | Anchor width in logical pixels; 0 anchors the menu to a point. |
| `x` | number | required | Left edge of the anchor in window coordinates, in logical pixels. |
| `y` | number | required | Top edge of the anchor in window coordinates, in logical pixels. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `action` | string or `()` | — | Registered action the row dispatches before closing the menu; it also supplies the legend and enabled state. |
| `checked` | bool | `false` | Shows the check mark at the row start and reports the row as checked. |
| `disabled` | bool | `false` | Dims the row; it cannot be clicked and keyboard navigation skips it. |
| `kind` | `"item"` or `"separator"` or `"submenu"` or `"label"` | required | `item` is an activatable row, `separator` a divider, `label` a heading, `submenu` a nested menu row. |
| `label` | string or `()` | — | Row text, also matched by letter-key type-ahead; on a `label` row, the heading text. |
| `shortcut` | string or `()` | — | Key legend shown at the row end, written like `cmd-o`; overrides the binding of `action`. |
| `submenu` | node or `()` | — | Nested Menu node drawn for a `submenu` row; set its `parent_overlay` to this menu's `key`. |
| `value` | string or `()` | — | Identifier passed to `on_action` and used as `active_value`; rows without one are skipped by the keys. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | string | Emitted when a row is clicked or activated with Enter; the payload is the item `value`. |
| `active_change` | `on_active_change` | string | Emitted when Up, Down or a letter key moves the highlight; the payload is the new item `value`. |
| `context_request` | `on_context_request` | any value | Emitted on a right-button press on the trigger; the payload is the pointer event, with its `window` position. |
| `open_change` | `on_open_change` | bool | Emitted when the menu asks to open or close; the payload is the requested `open`. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `trigger` | yes | no | Node that anchors and toggles the menu. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `check`, `content`, `focus_frame`, `indicator_bar`, `item`, `label`, `root`, `separator`, `shortcut`, `submenu`, `trigger`.

## Theme

- Tokens: `accent`, `border`, `disabled`, `focus_ring`, `metrics.icon`, `metrics.inset`, `metrics.row`, `radius.lg`, `selection`, `spacing.sm`, `spacing.xs`, `surface_hover`, `surface_raised`, `text.accent`, `text_muted`, `text_primary`, `typography.body`, `typography.label`
- Environment: `corners`, `density`

## Dependencies

[`components/divider`](divider.md), [`components/kbd`](kbd.md)
