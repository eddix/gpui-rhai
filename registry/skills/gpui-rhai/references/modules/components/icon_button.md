# IconButton

`components/icon_button` · export `IconButton` · version 0.2.0. Generated from
[`registry/components/icon_button.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/icon_button.rhai); do not edit.

IconButton presents one icon in a square action target.

State: stateless and controlled by caller props.

```rhai
import "components/icon_button" as icon_button;

icon_button::IconButton(#{ icon: svg(markup), label: "Close", variant: "ghost", size: "sm" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `action` | string or `()` | — | Registered action dispatched on click, in place of `on_click`; it also sets the enabled state and the shortcut told to assistive tech. |
| `disabled` | bool | `false` | Blocks clicks and greys the icon; a disabled `action` disables it too. |
| `hoverable` | bool | `true` | Changes the fill on hover; `false` keeps it still. |
| `icon` | node | required | Icon node, sized `metrics.icon` square and drawn in the button's text color. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name; the button shows no text, so it must not be empty. |
| `loading` | bool | `false` | Blocks clicks and shows the disabled look while work runs; it draws no spinner. |
| `on_click` | callback or `()` | — | Called when the button is pressed; not while disabled or loading, and not when `action` is set. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `selected` | bool | `false` | Pressed state of a toggle; `ghost`, `outline` and `secondary` fill with `selection` and stop reacting to hover. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size, which sets the square's side; unset, it inherits the `size` environment. |
| `style` | style | — | Style merged over the root part. |
| `variant` | `"primary"` or `"secondary"` or `"danger"` or `"warning"` or `"success"` or `"ghost"` or `"outline"` | `"ghost"` | Fill, as on `Button`; the default `ghost` stays bare until hovered or `selected`. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `click` | `on_click` | none | Emitted when the button is pressed while enabled, not loading and without an `action`. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `icon` | yes | no | The icon in the square. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `icon`, `root`.

## Theme

- Tokens: `border`, `disabled`, `focus_ring`, `metrics.control`, `metrics.icon`, `radius.md`, `selection`, `surface_hover`, `text_primary`
- Environment: `corners`, `density`, `size`

## Dependencies

[`components/button`](button.md)
