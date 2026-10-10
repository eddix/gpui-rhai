# Tabs

`components/tabs` · export `Tabs` · version 0.2.0. Generated from
[`registry/components/tabs.rhai`](../../../registry/components/tabs.rhai); do not edit.

Controlled tabs with one tab stop and orientation-aware arrow selection. State: controlled selected value.

```rhai
import "components/tabs" as tabs;

tabs::Tabs(#{ value: "overview", label: "Sections", tabs: tabs, on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the tab list, and of the group that holds the list and the panel. |
| `layout` | `"content"` or `"equal"` | `"content"` | Tab widths of a horizontal list: `content` fits each label, `equal` fills the width and shares it equally. |
| `motion_key` | string | `""` | Shared-layout id, at most 128 characters, that slides the selected indicator between tabs; empty turns it off. |
| `on_change` | callback or `()` | — | Called with a tab's `value` when it is clicked or reached with an arrow key; without it the tabs take no input. |
| `orientation` | `"horizontal"` or `"vertical"` | `"horizontal"` | Direction of the list: `horizontal` is a row moved with Left/Right, `vertical` a column beside the panel moved with Up/Down. |
| `panel` | bool | `true` | Whether the selected tab's `content` renders; `false` renders only the tab list, for view switchers. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Overrides the `size` environment for the tabs. |
| `style` | style | — | Style merged over the root part. |
| `tabs` | array of object (at most 128) | required | The tabs in list order. |
| `value` | string | required | The `value` of the selected tab; the caller stores the `change` payload and passes it back. |

### `tabs[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `content` | node or `()` | — | Panel shown while this tab is selected. |
| `disabled` | bool | `false` | Whether the tab ignores clicks; arrow keys skip it. |
| `icon` | node or `()` | — | Node drawn at icon size before the label. |
| `label` | string | required | Tab text and accessible name. |
| `label_visible` | bool | `true` | Whether the label text shows; a tab that hides it needs an `icon` and keeps the label as its name. |
| `value` | string | required | Identity of the tab, matched against `value` and passed to `on_change`. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | string | Emitted when a tab is clicked or an arrow key moves the selection; the payload is the new tab's `value`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `icon`, `indicator`, `label`, `list`, `panel`, `root`, `tab`, `tab_selected`.

## Theme

- Tokens: `disabled`, `focus_ring`, `metrics.control`, `metrics.control_pad`, `metrics.icon`, `metrics.label_column`, `radius.lg`, `radius.md`, `space.group`, `spacing.sm`, `spacing.xs`, `spacing.xxs`, `surface_hover`, `surface_raised`, `tabs.foreground`, `text_primary`, `typography.body`, `typography.control`
- Environment: `corners`, `density`, `size`
