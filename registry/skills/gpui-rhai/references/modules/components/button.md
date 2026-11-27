# Button

`components/button` · export `Button` · version 0.4.3. Generated from
[`registry/components/button.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/button.rhai); do not edit.

Button presents a desktop action: a large block with a centered label.

State: stateless and controlled by caller props.

`size` is inherited from the environment unless set. `action` dispatches a registered action on click and derives the enabled state and shortcut legend from it.

```rhai
import "components/button" as button;

button::Button(#{ text: "Deploy", variant: "primary", action: "deploy.start" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `action` | string or `()` | — | Registered action dispatched on click, in place of `on_click`; it also sets the enabled state and the shortcut legend. |
| `disabled` | bool | `false` | Blocks clicks and greys the button but keeps the variant's silhouette; a disabled `action` disables it too. |
| `hoverable` | bool | `true` | Changes the fill on hover; `false` keeps it still. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `loading` | bool | `false` | Blocks clicks without applying disabled opacity. Dims the label and replaces an existing prefix with a native rotating indicator; never synthesizes a new prefix. Reduced or no motion holds the indicator still. |
| `loading_text` | string | `""` | Replaces the visible label and accessible name while loading, without an invisible idle-label spacer. Empty retains the ordinary label. |
| `on_click` | callback or `()` | — | Called when the button is pressed; not while disabled or loading, and not when `action` is set. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `prefix` | node or `()` | — | Node before the label, such as an icon, drawn in the button's text color. |
| `shortcut` | string or `()` | — | Key legend after the label, such as `cmd-s`, formatted for the platform; it binds no key and overrides the `action` legend. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size of the button; unset, it inherits the `size` environment. |
| `style` | style | — | Style merged over the root part. |
| `suffix` | node or `()` | — | Node after the label, such as a disclosure chevron, drawn in the button's text color. |
| `text` | string | required | Label of the button; also its accessible name, except while `loading` shows `loading_text`. |
| `variant` | `"primary"` or `"secondary"` or `"danger"` or `"destructive"` or `"warning"` or `"success"` or `"ghost"` or `"outline"` | `"primary"` | Preset-owned fill: `primary` ink/reverse ink, `secondary` surface, `ghost` bare, `danger`/`destructive` soft red; warning, success and outline are derived variants. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `click` | `on_click` | none | Emitted when the button is pressed while enabled, not loading and without an `action`. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `prefix` | no | no | Node before the label. |
| `suffix` | no | no | Node after the label. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `label`, `prefix`, `root`, `shortcut`, `spinner`, `suffix`.

## Theme

- Tokens: `accent`, `accent_hover`, `border`, `button.danger_bg`, `button.ghost_fg`, `button.primary_bg`, `button.primary_fg`, `button.secondary_bg`, `control.border`, `control.danger_hover`, `control.hover`, `control.success_hover`, `control.warning_hover`, `danger`, `disabled`, `focus_ring`, `metrics.button_gap`, `metrics.button_h`, `metrics.button_icon`, `metrics.button_icon_pad`, `metrics.button_pad`, `metrics.button_radius`, `metrics.control`, `metrics.control_pad`, `metrics.icon`, `on_accent`, `on_danger`, `on_success`, `on_warning`, `radius.md`, `spacing.xs`, `success`, `surface_hover`, `text_primary`, `typography.button`, `typography.control`, `typography.label`, `warning`
- Environment: `density`, `size`

## Dependencies

[`components/spinner`](spinner.md)
