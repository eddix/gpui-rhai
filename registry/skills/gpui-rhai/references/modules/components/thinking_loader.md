# ThinkingLoader

`components/thinking_loader` · export `ThinkingLoader` · version 0.1.0. Generated from
[`registry/components/thinking_loader.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/thinking_loader.rhai); do not edit.

ThinkingLoader presents a phase label and motion-owned work/completion cue. State: controlled phase and completion; optional local phase playback is for previews.

```rhai
import "components/thinking_loader" as thinking_loader;

thinking_loader::ThinkingLoader(#{key:"work",phases:["Reading","Writing"],phase:0})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `autoplay` | bool | `false` | Opt into local phase playback for demonstrations. It changes labels only, not completion. |
| `done` | bool | `false` | Accepted completion; stops activity and replaces its spinner with a check. |
| `done_label` | string or `()` | — | Overrides the final completion caption. |
| `elapsed_ms` | integer 0–9000000000000 or `()` | — | Application measured duration used for the final caption. Does not create a wall-clock timer. |
| `interval_ms` | integer 200–60000 | `2200` | How long each preview label is shown. |
| `key` | string | required | Stable activity identity. Use a new key for a new run. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `phase` | integer 0–15 or `()` | — | Accepted zero-based phase; capped by the supplied labels. When present this disables local playback. |
| `phases` | array of string (at most 16) | `[]` | Phase captions. An empty array uses a localized Thinking label. |
| `repeat` | bool | `false` | Preview playback returns to its first label after the final one. |
| `size` | `"sm"` or `"md"` or `()` | — | Override the inherited size with sm or md. |
| `stopped` | bool | `false` | Freeze the work cue and show the stopped caption; done takes precedence. |
| `style` | style | — | Style merged over the root part. |
| `variant` | `"pill"` or `"plain"` | `"pill"` | Draw a contained pill or render flush inside a larger surface. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `check`, `clip`, `finished`, `icon`, `label`, `plain`, `ray`, `root`, `sheen`, `stop`, `stopped`.

## Theme

- Tokens: `border`, `radius.pill`, `space.spacing_unit`, `space.unit`, `surface_raised`, `text.success`, `text_muted`, `text_primary`, `thinking.sheen`, `typography.caption`
- Environment: `density`, `size`
