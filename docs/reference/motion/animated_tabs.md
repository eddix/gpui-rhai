# AnimatedTabs

`motion/animated_tabs` · export `AnimatedTabs` · version 0.1.6. Generated from
[`registry/motion/animated_tabs.rhai`](../../../registry/motion/animated_tabs.rhai); do not edit.

AnimatedTabs is a controlled `Tabs` whose selection indicator slides to the selected tab. State: caller-owned.

```rhai
import "motion/animated_tabs" as animated_tabs;

animated_tabs::AnimatedTabs(#{ key: "docs", value: "one", label: "Docs", tabs: tabs })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `key` | string | required | Instance key and motion identity: the indicator slides within this key's shared-layout group; at most 128 characters. |
| `label` | string | required | Accessible name of the tab list. |
| `layout` | `"content"` or `"equal"` | `"content"` | `content` sizes each tab to its label; `equal` gives horizontal tabs equal widths across the full row. |
| `on_change` | callback or `()` | — | Called with the chosen tab's `value` when a tab is clicked or an arrow key moves; without it the tabs ignore input. |
| `orientation` | `"horizontal"` or `"vertical"` | `"horizontal"` | Lays the tabs out in a row or a column; the arrow keys follow it. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `tabs` | array of object (at most 128) | required | Tabs in display order. |
| `value` | string | required | Value of the selected tab; the caller stores the `change` payload and passes it back. |

### `tabs[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `content` | node | required | Panel shown while this tab is selected. |
| `disabled` | bool | `false` | Ignores clicks on the tab and skips it for the arrow keys. |
| `icon` | node or `()` | — | Icon node drawn before the label. |
| `label` | string | required | Tab text; also its accessible name. |
| `label_visible` | bool | `true` | Shows the label; a tab that hides it needs an `icon`. |
| `value` | string | required | Value reported when this tab is chosen; unique among the tabs. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | string | Emitted when a tab is clicked or an arrow key moves the selection; the payload is the chosen tab's `value`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`.

## Dependencies

[`components/tabs`](../components/tabs.md)
