# Tag

`components/tag` · export `Tag` · version 0.2.0. Generated from
[`registry/components/tag.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/tag.rhai); do not edit.

Tag labels a category: a flat tonal tape with an optional facet segment.

State: stateless and controlled by caller props.

Variants mark a category, not a status. Without a facet they color the text; with a facet they fill the facet segment. Markers keep their size in both densities.

```rhai
import "components/tag" as tag;

tag::Tag(#{ facet: "env", text: "prod", variant: "danger" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `closable` | bool | `false` | Adds a close button after the text. |
| `close_label` | string or `()` | — | Accessible name of the close button; defaults to `Remove <text>`. |
| `disabled` | bool | `false` | Draws the tag in the `disabled` color and makes the close button inert. |
| `facet` | string or `()` | — | Key drawn uppercase in a segment before the text, as in `ENV prod`; empty means none. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `on_close` | callback or `()` | — | Called with the tag's `text` when the close button is clicked; not called while `disabled`. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `text` | string | required | The value shown; it is also the payload of `close`. |
| `variant` | `"neutral"` or `"accent"` or `"success"` or `"warning"` or `"danger"` | `"neutral"` | Category color: it colors the text, or fills the facet segment when there is a `facet`. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `close` | `on_close` | string | Emitted when the close button is clicked; the payload is the tag's `text`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `close`, `facet`, `label`, `root`.

## Theme

- Tokens: `accent`, `control.hover`, `danger`, `disabled`, `focus_ring`, `metrics.marker`, `on_accent`, `on_danger`, `on_success`, `on_warning`, `radius.sm`, `spacing.sm`, `spacing.xs`, `success`, `surface_hover`, `tag.facet`, `text.accent`, `text.danger`, `text.success`, `text.warning`, `text_primary`, `typography.caption`, `typography.label`, `warning`
- Environment: `corners`
