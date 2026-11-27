# Tag

`components/tag` · export `Tag` · version 0.4.3. Generated from
[`registry/components/tag.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/tag.rhai); do not edit.

Tag labels a category with a removable-chip surface and an optional facet segment.

State: stateless and controlled by caller props.

Variants mark a category, not a status. Without a facet they color the text; with a facet they use a paired soft fill and ink. Markers keep their size in both densities.

```rhai
import "components/tag" as tag;

tag::Tag(#{ facet: "env", text: "prod", variant: "danger" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `closable` | bool | `false` | Adds a close button after the text. |
| `close_label` | string or `()` | — | Accessible name of the close button; defaults to the localized `tag.remove` message, `Remove <text>` in English. |
| `close_tab_stop` | bool | `true` | Whether the close button owns a Tab stop; use false for an editor that navigates its chip rows itself. |
| `disabled` | bool | `false` | Dims the complete chip to 55% opacity and makes the close button inert. |
| `facet` | string or `()` | — | Key drawn as written in a segment before the text, as in `env prod`; empty means none. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `on_close` | callback or `()` | — | Called with the tag's `text` when the close button is clicked; not called while `disabled`. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `prefix` | node or `()` | — | An optional avatar or icon before the label, inside the value segment. |
| `style` | style | — | Style merged over the root part. |
| `text` | string | required | The value shown; it is also the payload of `close`. |
| `variant` | `"neutral"` or `"accent"` or `"success"` or `"warning"` or `"danger"` | `"neutral"` | Category tone: it colors the text, or supplies a paired soft fill/foreground for a facet segment. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `close` | `on_close` | string | Emitted when the close button is clicked; the payload is the tag's `text`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `close`, `close_icon`, `facet`, `label`, `prefix`, `root`, `value`.

## Theme

- Tokens: `metrics.tag_close`, `metrics.tag_end`, `metrics.tag_gap`, `metrics.tag_h`, `metrics.tag_icon`, `metrics.tag_pad`, `metrics.tag_radius`, `tag.facet_accent`, `tag.facet_danger`, `tag.facet_neutral`, `tag.facet_success`, `tag.facet_warning`, `tag.normal`, `tag.text_accent`, `tag.text_danger`, `tag.text_success`, `tag.text_warning`, `typography.tag`
