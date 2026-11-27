# ReactionBar

`components/reaction_bar` · export `ReactionBar` · version 0.1.0. Generated from
[`registry/components/reaction_bar.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/reaction_bar.rhai); do not edit.

ReactionBar shows a compact set of social actions and their exact accessible counts. State: local unless the application provides a reactions value.

```rhai
import "components/reaction_bar" as reaction_bar;

reaction_bar::ReactionBar(#{key:"release",actions:["like","share"]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of `"like"` or `"comment"` or `"repost"` or `"share"` (at most 4) | `["like", "comment", "repost", "share"]` | Visible actions in order; duplicates are shown once. |
| `disabled` | bool | `false` | Blocks all reactions. |
| `key` | string | required | Stable reaction group identity. |
| `label` | string or `()` | — | Accessible name of the reaction group. |
| `on_react` | callback or `()` | — | Receives one proposal: #{id,count,active}. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `reactions` | array of object (at most 4) or `()` | — | Controlled states; omitted reactions start at zero. Omit the whole property for local reactions. |
| `style` | style | — | Style merged over the root part. |

### `reactions[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `active` | bool | required | Accepted pressed state. |
| `count` | integer 0–999999999999999 | required | Exact accepted count. |
| `id` | `"like"` or `"comment"` or `"repost"` or `"share"` | required | Reaction ID. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `react` | `on_react` | object | One proposed reaction state. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `action_active`, `comment`, `count`, `digit`, `icon`, `like`, `repost`, `root`, `share`.

## Theme

- Tokens: `accent`, `focus_ring`, `radius.sm`, `reaction.comment`, `reaction.ink`, `reaction.like`, `reaction.repost`, `reaction.share`, `space.spacing_unit`, `space.unit`, `surface_hover`, `text_primary`, `typography.caption`
- Environment: `density`
