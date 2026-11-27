# SmartRecommendation

`patterns/smart_recommendation` · export `SmartRecommendation` · version 0.1.0. Generated from
[`registry/patterns/smart_recommendation.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/smart_recommendation.rhai); do not edit.

A recommendation with a confidence signal, alternatives and reversible Host decisions.

```rhai
import "patterns/smart_recommendation" as smart_recommendation;

smart_recommendation::SmartRecommendation(#{key:"proposal",session:"sync",options:[#{id:"delta",label:"Incremental sync",reason:"Only changed files are copied.",signal:3}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `accepted_id` | string or `()` | — | When present, the accepted option is controlled by the Host; explicit null means none. |
| `default_open` | bool | `true` | Whether to show the reason initially. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `on_accept` | callback or `()` | — | Request acceptance of the selected option, without executing it. |
| `on_undo` | callback or `()` | — | Request reversal of the current acceptance. |
| `options` | array of object (at most 32) | required | Highest-ranked option first, followed by alternatives. An empty list draws nothing. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pending` | bool | `false` | Host decision pending. Disables mutating controls without changing the accepted snapshot. |
| `revision` | integer ≥ 0 | `0` | Host revision carried by every decision proposal. |
| `session` | string | required | Proposal group identity; a new session resets local selection and disclosure. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Overrides the heading for the recommendation group, without changing option order. |

### `options[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Unique, stable ID used for selection and decisions. |
| `label` | string | required | Human-readable name. |
| `reason` | string or node | required | One sentence or a rich SDK node. The reason is never included in emitted events. |
| `signal` | integer 0–3 | required | No, weak, good or strong signal. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `accept` | `on_accept` | object | Requested option ID, in the current proposal session and revision. |
| `undo` | `on_undo` | object | Requested reversal of the accepted option ID. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `alternative_glyph`, `bar`, `bars`, `body`, `button`, `button_text`, `card`, `chevron`, `flag`, `foot`, `head`, `mark`, `menu`, `mini`, `option`, `option_name`, `option_selected`, `pending`, `primary`, `reason`, `receipt`, `recommended`, `root`, `signal`, `title`, `undo_glyph`.

## Theme

- Tokens: `focus_ring`, `radius.lg`, `radius.sm`, `recommendation.action`, `recommendation.action_hover`, `recommendation.card`, `recommendation.chip`, `recommendation.chip_ink`, `recommendation.good`, `recommendation.hover`, `recommendation.idle`, `recommendation.line`, `recommendation.muted`, `recommendation.on_solid`, `recommendation.paper`, `recommendation.popup`, `recommendation.reason`, `recommendation.selected`, `recommendation.selected_edge`, `recommendation.signal0`, `recommendation.solid`, `recommendation.strong`, `recommendation.success`, `recommendation.success_bg`, `recommendation.tag`, `recommendation.tray`, `recommendation.weak`, `space.spacing_unit`, `space.unit`, `text_primary`, `typography.body`, `typography.caption`
