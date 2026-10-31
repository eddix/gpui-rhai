# Tabs

`components/tabs` · export `Tabs` · version 0.3.1. Generated from
[`registry/components/tabs.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/tabs.rhai); do not edit.

Controlled tabs with one tab stop; the arrow keys select. Three looks share every capability: `track` is a tonal track with a raised thumb, for a compact switcher among other controls; `underline` puts the labels on a hairline with a 2px ink bar under the selected one, for the sections of a panel or a card; `bar` is a strip of document tabs whose selected tab takes the panel's surface (`tabbar.active`) and covers its line.

A tab carries only what the component draws: an icon, the label, a count capsule or a tone lamp, a dirty mark and a close button. None of them is a tab stop; the list reaches each one: Left/Right (Up/Down when vertical), Home and End select, Delete or Backspace closes the selected tab, Alt+Arrow moves it when the tabs are reorderable, and Shift+F10 or the menu key asks for a context menu. Actions that are not about one tab go in `start` and `end`, outside the list; `on_add` draws a "+" after the last tab. Tabs keep their width and the strip scrolls when they do not fit, keeping the selected tab revealed. With `panel_motion: "slide"` a newly selected panel slides in from the side of the tab it replaces.

```rhai
import "components/tabs" as tabs;

tabs::Tabs(#{ value: "overview", label: "Sections", tabs: tabs, on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `add_label` | string | `"Add tab"` | Accessible name of the add button. |
| `close_label` | string | `"Close"` | Accessible name of a close button, followed by the tab's label. |
| `end` | array of node (at most 8) | `[]` | Nodes at the far end of the strip, after the add button and the overflow menu. |
| `keep_mounted` | bool | `false` | Retains all supplied panels, including native scrolling, caret/selection/undo and focus identity. Hidden panels have no layout, input or presented accessibility. Business effects keep running; use ctx.is_visible() for an explicit application pause policy. |
| `key` | string | — | Stable identity of the tabs; required with `reorderable`, `overflow_menu` or a sliding `panel_motion`, whose drag list, menu and sliding panel it names. |
| `label` | string | required | Accessible name of the tab list, and of the group that holds the list and the panel. |
| `layout` | `"content"` or `"equal"` | `"content"` | Tab widths of a horizontal list: `content` fits each label and scrolls when the tabs do not fit, `equal` fills the width and shares it equally. |
| `menu_label` | string | `"All tabs"` | Accessible name of the overflow menu button and its menu. |
| `motion_key` | string | `""` | Shared-layout id, at most 128 characters, that slides the selected indicator between tabs; empty turns it off. |
| `on_add` | callback or `()` | — | Draws a "+" button after the last tab, pinned at the strip's end when the tabs overflow; called when it is pressed. |
| `on_change` | callback or `()` | — | Called with a tab's `value` when it is clicked, reached with an arrow key, Home or End, or picked in the overflow menu; without it the tabs do not select. |
| `on_close` | callback or `()` | — | Called with `#{ value, next }` when a close button, a middle press or Delete closes a tab; `next` is the tab to select instead (see `close_successor`). The caller removes the tab. |
| `on_context_request` | callback or `()` | — | Called with `#{ value, anchor, source }` on a right press, Shift+F10 or the menu key; without it no menu is asked for. |
| `on_reorder` | callback or `()` | — | Called with `#{ value, anchor, placement }` when a tab is dropped or moved by key; the caller reorders `tabs`. |
| `orientation` | `"horizontal"` or `"vertical"` | `"horizontal"` | Direction of the list: `horizontal` is a row moved with Left/Right, `vertical` a column beside the panel moved with Up/Down. |
| `overflow_menu` | bool | `false` | Whether a menu button lists every tab; picking one selects it. Needs `key`. |
| `panel` | bool | `true` | Whether the selected tab's `content` renders; `false` renders only the strip, for view switchers. |
| `panel_motion` | `"none"` or `"slide"` | `"none"` | `slide` brings a newly selected panel in from the side of the tab it replaces, a short way as it fades in (from above or below when vertical); the panel it replaces is gone at once, and the Host's motion policy settles it. `none` switches at once. `slide` needs `key`. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `reorderable` | bool | `false` | Whether tabs move by dragging or Alt+Arrow; moves are reported through `on_reorder`. Needs `key`. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Overrides the `size` environment for the tabs. |
| `start` | array of node (at most 8) | `[]` | Nodes before the list (above it when vertical), outside it: each keeps its own tab stop. |
| `style` | style | — | Style merged over the root part. |
| `tabs` | array of object (at most 256) | required | The tabs in list order; empty is a strip with no tabs. |
| `value` | string | required | The `value` of the selected tab, or one no tab has (such as `""`) for none; the caller stores the `change` payload and passes it back. |
| `variant` | `"track"` or `"underline"` or `"bar"` | `"track"` | `track` is a tonal track with a raised thumb; `underline` puts the labels on a hairline and marks the selected one with a 2px ink bar; `bar` is a strip of document tabs whose selected tab joins the panel under it. `underline` and `bar` are horizontal only. |

### `tabs[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `closable` | bool | `false` | Whether the tab has a close button, shown while selected or hovered; a middle press closes it, and Delete closes it while selected. |
| `content` | node or `()` | — | Caller-owned panel. Eagerly constructed components keep Rhai state/effects even when omitted (ADR 0019); keep_mounted also preserves native scroll/editing state. |
| `count` | integer ≥ 0 or `()` | — | Number shown in a capsule after the label, `99+` past 99; zero shows too. The full number joins the accessible name. |
| `dirty` | bool | `false` | Marks unsaved changes with a square; on a closable tab it turns into the close button under the pointer. |
| `disabled` | bool | `false` | Whether the tab ignores the pointer; the arrow keys skip it. |
| `icon` | node or `()` | — | Node drawn at icon size before the label. |
| `label` | string | required | Tab text and accessible name; past twice `metrics.label_column` the label truncates. |
| `label_visible` | bool | `true` | Whether the label text shows; a tab that hides it needs an `icon` and keeps the label as its name. |
| `tone` | `"accent"` or `"warning"` or `"danger"` or `()` | — | Fills the count capsule with the tone; without a `count` it shows a 6px lamp after the label. |
| `tone_label` | string or `()` | — | Accessible text for the tone, such as `Has updates`; it joins the accessible name, since a lamp has no text. |
| `value` | string | required | Unique business identity of the tab, including the empty string; matched against `value` and reported unchanged by every callback. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `add` | `on_add` | none | Emitted when the add button is pressed. |
| `change` | `on_change` | string | Emitted when a tab is clicked, reached by key or picked in the overflow menu; the payload is its `value`. |
| `close` | `on_close` | object | Emitted when a close button, a middle press or Delete closes a tab. |
| `context_request` | `on_context_request` | object | Emitted on a right press over a tab, or Shift+F10 or the menu key on the selected tab. |
| `reorder` | `on_reorder` | object | Emitted when a tab is dropped in a new place or moved with Alt+Arrow. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `end` | no | yes | Nodes at the far end of the strip. |
| `start` | no | yes | Nodes before the list. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `add`, `close`, `counter`, `dirty`, `end`, `focus_frame`, `icon`, `indicator`, `label`, `lamp`, `line`, `list`, `menu`, `panel`, `root`, `slot`, `start`, `strip`, `tab`, `tab_selected`.

## Theme

- Tokens: `accent`, `border`, `control.hover`, `danger`, `disabled`, `focus_ring`, `metrics.control`, `metrics.control_pad`, `metrics.counter`, `metrics.icon`, `metrics.inset`, `metrics.label_column`, `on_accent`, `on_danger`, `on_warning`, `radius.md`, `radius.sm`, `radius.xs`, `space.group`, `space.related`, `spacing.sm`, `spacing.xs`, `spacing.xxs`, `surface_hover`, `surface_raised`, `tabbar.active`, `tabbar.background`, `tabs.counter`, `tabs.foreground`, `text_primary`, `typography.body`, `typography.control`, `typography.counter`, `warning`
- Environment: `density`, `size`

## Dependencies

[`components/icon_button`](icon_button.md), [`components/sliding_panel`](sliding_panel.md), [`components/tab_overflow_menu`](tab_overflow_menu.md)
