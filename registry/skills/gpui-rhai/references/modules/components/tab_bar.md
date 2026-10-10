# TabBar

`components/tab_bar` · export `TabBar` · version 0.2.0. Generated from
[`registry/components/tab_bar.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/tab_bar.rhai); do not edit.

Controlled document tabs that belong to the panel under them: the selected tab takes the panel's surface (`tabbar.active`) and covers the bar's bottom line, so the panel's extent says what the tabs switch. Tabs keep their width and the strip scrolls sideways when they do not fit; the selected tab stays revealed.

Keyboard: the strip is one tab stop; Left/Right/Home/End move a cursor (the 2px focus frame) and Enter or Space selects it, so moving does not switch panels on every step. Shift+F10 or the menu key asks for a context menu for the cursor tab; Alt+Left/Right moves it when the tabs are reorderable. A closable tab shows its close button while it is selected or hovered, and closes on a middle press; a dirty tab shows a square mark that turns into the close button under the pointer. Reorderable tabs are dragged anywhere on the tab.

```rhai
import "components/tab_bar" as tab_bar;

tab_bar::TabBar(#{ key: "docs", label: "Open files", value: current, tabs: tabs,
    on_change: Fn("opened"), on_close: Fn("closed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `close_label` | string | `"Close"` | Accessible name of a close button, followed by the tab's label. |
| `end` | array of node (at most 8) | `[]` | Nodes after the tab strip and the overflow menu. |
| `key` | string | required | Stable identity of the bar; it also scopes its tab refs, its reorder list and its overflow menu. |
| `label` | string | required | Accessible name of the tab strip. |
| `menu_label` | string | `"All tabs"` | Accessible name of the overflow menu button and its menu. |
| `on_change` | callback or `()` | — | Called with a tab's `value` when it is clicked, chosen with Enter or Space, or picked in the overflow menu. |
| `on_close` | callback or `()` | — | Called with a tab's `value` when its close button or a middle press closes it; the caller removes the tab. |
| `on_context_request` | callback or `()` | — | Called with `#{ value, anchor, source }` on a right press, Shift+F10 or the menu key; without it no menu is asked for. |
| `on_reorder` | callback or `()` | — | Called with `#{ value, anchor, placement }` when a tab is dropped or moved by key; the caller reorders `tabs`. |
| `overflow_menu` | bool | `false` | Whether a menu button after the strip lists every tab; picking one selects it. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `reorderable` | bool | `false` | Whether tabs move by dragging or Alt+Left/Right; the bar reports moves through `on_reorder`. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Overrides the `size` environment for the bar. |
| `start` | array of node (at most 8) | `[]` | Nodes before the tab strip. |
| `style` | style | — | Style merged over the root part. |
| `tabs` | array of object (at most 256) | required | The tabs in strip order. |
| `value` | string | required | The `value` of the selected tab; the caller stores the `change` payload and passes it back. |

### `tabs[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `closable` | bool | `false` | Whether the tab has a close button, shown while selected or hovered, and closes on a middle press. |
| `dirty` | bool | `false` | Marks unsaved changes with a square; on a closable tab it turns into the close button under the pointer. |
| `disabled` | bool | `false` | Whether the tab ignores the pointer; the keyboard cursor skips it. |
| `icon` | node or `()` | — | Node drawn at icon size before the label. |
| `label` | string | required | Tab text and accessible name; past twice `metrics.label_column` the label truncates. |
| `value` | string | required | Identity of the tab, matched against `value` and reported by every callback; unique among the tabs. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | string | Emitted when a tab is selected by click, Enter, Space or the overflow menu; the payload is its `value`. |
| `close` | `on_close` | string | Emitted when a close button or a middle press closes a tab; the payload is its `value`. |
| `context_request` | `on_context_request` | object | Emitted on a right press over a tab, or Shift+F10 or the menu key on the cursor tab; the payload says where to open a menu. |
| `reorder` | `on_reorder` | object | Emitted when a tab is dropped in a new place or moved with Alt+Left/Right; the payload says where it goes. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `end` | no | yes | Nodes after the tab strip and the overflow menu. |
| `start` | no | yes | Nodes before the tab strip. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `close`, `cursor`, `dirty`, `end`, `icon`, `label`, `line`, `list`, `menu`, `root`, `slot`, `start`, `tab`, `tab_selected`.

## Theme

- Tokens: `border`, `disabled`, `focus_ring`, `metrics.control`, `metrics.icon`, `metrics.inset`, `metrics.label_column`, `radius.md`, `spacing.xs`, `surface_hover`, `tabbar.active`, `tabbar.background`, `tabs.foreground`, `text_primary`, `typography.control`
- Environment: `corners`, `density`, `size`

## Dependencies

[`components/menu`](menu.md)
