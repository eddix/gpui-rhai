# ProgressTracker

`components/progress_tracker` · export `ProgressTracker` · version 0.1.0. Generated from
[`registry/components/progress_tracker.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/progress_tracker.rhai); do not edit.

A keyed checklist with progress. SetupGuide uses the same projection and meter.

```rhai
import "components/progress_tracker" as progress_tracker;

progress_tracker::ProgressTracker(#{key:"launch",steps:[#{id:"tests",label:"Run tests"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `autoplay` | bool | `false` | Opt-in active-view demo playback; user focus or pointer input stops it. |
| `disabled` | bool | `false` | Blocks changes and autoplay. |
| `interval_ms` | integer 100–60000 | `2200` | Playback interval; the reset holds for twice this duration. |
| `key` | string | required | Stable identity of the checklist. |
| `on_change` | callback or `()` | — | Proposes the completed IDs in display order. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `steps` | array of object (at most 128) | required | The ordered, bounded set of steps. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Visible heading and meter name. |
| `value` | array of string (at most 128) or `()` | — | Controlled completed IDs. Local state is used only when omitted. |

### `steps[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Counts toward the total but cannot be changed here. |
| `done` | bool | `false` | Initial local state for a newly admitted ID. |
| `icon` | node or `()` | — | Optional decorative trailing node. |
| `id` | string | required | Unique nonempty ID; completion follows it across reordering. |
| `label` | string | required | Visible text and accessible checkbox name. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | array of string (at most 128) | A full completion proposal; no optimistic change in controlled mode. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `check`, `count`, `digit`, `fill`, `header`, `icon`, `icon_done`, `label`, `label_done`, `list`, `mark`, `mark_done`, `root`, `row`, `shine`, `strike`, `title`, `track`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `on_accent`, `radius.lg`, `radius.md`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `tracker.sheen`, `typography.caption`, `typography.control`
- Environment: `density`
