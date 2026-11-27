# ExperimentCard

`patterns/experiment_card` · export `ExperimentCard` · version 0.1.0. Generated from
[`registry/patterns/experiment_card.rhai`](../../../registry/patterns/experiment_card.rhai); do not edit.

A compact experiment that unfolds into a controlled comparison. State: disclosure and preview attention; metric, candidate and confirmation belong to the Host.

```rhai
import "patterns/experiment_card" as experiment_card;

experiment_card::ExperimentCard(#{key:"signup",experiment:"signup-v1",revision:0,title:"Sign-up test",metrics:[#{id:"rate",label:"Rate"}],variants:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `action_label` | string or `()` | — | Final action text. Defaults to Choose {name}. |
| `busy` | bool | `false` | Keep the current selection while the Host commits. |
| `default_open` | bool | `false` | Initial uncontrolled disclosure. |
| `experiment` | string | required | Stable experiment ID. |
| `hint` | node or `()` | — | Footer node before a variant is selected. |
| `key` | string | required | Stable retained card key. |
| `metric` | string or `()` | — | Accepted metric ID. Defaults to the first metric. |
| `metrics` | array of object (at most 8) | required | Supported metrics. Order determines the default metric. |
| `note` | string or `()` | — | Optional line beside the status. |
| `on_action` | callback or `()` | — | Receives a distinct request to act on the accepted candidate. |
| `on_metric_change` | callback or `()` | — | Receives a versioned metric proposal. |
| `on_open_change` | callback or `()` | — | Called for requested or local disclosure. |
| `on_winner_change` | callback or `()` | — | Receives a versioned candidate proposal. |
| `open` | bool or `()` | — | Optional controlled disclosure. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Accepted snapshot revision. |
| `status` | object or `()` | — | Optional status chip. |
| `style` | style | — | Style merged over the root part. |
| `title` | string | required | Visible experiment title. |
| `trigger` | `"hover"` or `"click"` | `"hover"` | Pointer hover or explicit disclosure. |
| `variants` | array of object (at most 12) | required | Variants in their display and chart order. |
| `winner` | string or `()` | — | Accepted candidate, absent until one is selected. |

### `metrics[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `decimals` | integer 0–6 or `()` | — | Maximum displayed fraction digits. |
| `id` | string | required | Unique metric ID. |
| `label` | string | required | Visible metric name. |
| `lower_is_better` | bool or `()` | — | Smaller readings are preferred. |
| `suffix` | string or `()` | — | Optional unit suffix. |

### `status` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `label` | string | required | Status words. |
| `tone` | `"gray"` or `"violet"` or `"amber"` or `"blue"` or `"green"` or `"red"` or `()` | — | Status tint. |

### `variants[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `color` | integer 0–16777215 or `()` | — | Optional RGB24 paint. |
| `confidence` | map of number 0–100 or `()` | — | Optional confidence percentages by metric. |
| `control` | bool or `()` | — | Designates the baseline; the first variant is the default. |
| `id` | string | required | Unique stable variant ID. |
| `name` | string | required | Visible variant name. |
| `short` | string | required | Short badge, e.g. A. |
| `values` | map of number -1000000000000–1000000000000 | required | Readings keyed by metric ID. Omitted readings remain missing. |
| `visitors` | integer 0–1000000000000 or `()` | — | Accepted visitor count. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Ask the Host to act on the accepted candidate in this snapshot. |
| `metric_change` | `on_metric_change` | object | Ask the Host to show a different metric. |
| `open_change` | `on_open_change` | object | Notify or propose disclosure. |
| `winner_change` | `on_winner_change` | object | Nominate a candidate; no final action is performed. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `badge`, `badge_picked`, `body`, `caption`, `card`, `chart`, `column`, `empty`, `fill`, `foot`, `head`, `head_narrow`, `label`, `lead`, `lead_idle`, `lift`, `note`, `paper`, `root`, `row`, `row_hot`, `row_picked`, `rows`, `status`, `sub`, `switch`, `title`, `titles`, `track`, `value`.

## Theme

- Tokens: `border`, `experiment_card.a`, `experiment_card.b`, `experiment_card.c`, `experiment_card.card`, `experiment_card.chip_amber_ink`, `experiment_card.chip_amber_wash`, `experiment_card.chip_blue_ink`, `experiment_card.chip_blue_wash`, `experiment_card.chip_gray_ink`, `experiment_card.chip_gray_wash`, `experiment_card.chip_green_ink`, `experiment_card.chip_green_wash`, `experiment_card.chip_red_ink`, `experiment_card.chip_red_wash`, `experiment_card.chip_violet_ink`, `experiment_card.chip_violet_wash`, `experiment_card.control`, `experiment_card.d`, `experiment_card.muted`, `experiment_card.track`, `experiment_card.tray`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/button`](../components/button.md), [`components/toggle_group`](../components/toggle_group.md), [`patterns/agent_tree`](../patterns/agent_tree.md), [`patterns/project_card`](../patterns/project_card.md)
