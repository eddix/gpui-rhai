# Tabs

`components/tabs` · export `Tabs` · version 0.3.0. Generated from
[`registry/components/tabs.rhai`](../../../registry/components/tabs.rhai); do not edit.

Controlled tabs with one tab stop and orientation-aware arrow selection. State: controlled selected value.

`variant: "track"` is a tonal track with a raised thumb, for a compact switcher among other controls. `variant: "underline"` is a row of labels on a hairline with a 2px ink bar under the selected one, for the section navigation that heads a panel or a card.

```rhai
import "components/tabs" as tabs;

tabs::Tabs(#{ value: "overview", label: "Sections", tabs: tabs, on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `keep_mounted` | bool | `false` | Retains all supplied panels, including native scrolling, caret/selection/undo and focus identity. Hidden panels have no layout, input or presented accessibility. Business effects keep running; use ctx.is_visible() for an explicit application pause policy. |
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
| `variant` | `"track"` or `"underline"` | `"track"` | `track` is a tonal track with a raised thumb; `underline` puts the labels on a hairline and marks the selected one with a 2px ink bar. `underline` is horizontal only. |

### `tabs[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `content` | node or `()` | — | Caller-owned panel. Eagerly constructed components keep Rhai state/effects even when omitted (ADR 0019); keep_mounted also preserves native scroll/editing state. |
| `disabled` | bool | `false` | Whether the tab ignores clicks; arrow keys skip it. |
| `icon` | node or `()` | — | Node drawn at icon size before the label. |
| `label` | string | required | Tab text and accessible name. |
| `label_visible` | bool | `true` | Whether the label text shows; a tab that hides it needs an `icon` and keeps the label as its name. |
| `value` | string | required | Unique business identity of the tab, including the empty string; matched against `value` and passed unchanged to `on_change`. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | string | Emitted when a tab is clicked or an arrow key moves the selection; the payload is the new tab's `value`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `focus_frame`, `icon`, `indicator`, `label`, `line`, `list`, `panel`, `root`, `tab`, `tab_selected`.

## Theme

- Tokens: `border`, `disabled`, `focus_ring`, `metrics.control`, `metrics.control_pad`, `metrics.icon`, `metrics.label_column`, `radius.lg`, `radius.md`, `space.group`, `space.related`, `spacing.sm`, `spacing.xs`, `spacing.xxs`, `surface_hover`, `surface_raised`, `tabs.foreground`, `text_primary`, `typography.body`, `typography.control`
- Environment: `corners`, `density`, `size`
