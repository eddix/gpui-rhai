# SplitPane

`components/split_pane` · export `SplitPane` · version 0.2.0. Generated from
[`registry/components/split_pane.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/split_pane.rhai); do not edit.

Controlled, nestable two-panel split layout with a native drag hot lane.

`size` is the controlled start-panel ratio. Pointer moves update only a native signal; `on_resize` fires once when a drag ends or a keyboard step is requested. If the Host rejects that value, the preview returns to `size`. State: only native pointer preview; application ratio and collapse remain controlled.

The default grip is a short capsule, a marker's shape. Hover darkens it, focus takes the focus color, and only a drag extends it, with the theme's fast feedback motion sampled natively; reduced/no-motion policy snaps it. `grip: false` removes the decoration without removing the separator's drag/key surface. `handle` replaces the bar with a static Node. Alternatively, `handle_renderer` is a render-time Fn(ctx, detail) -> Node, bound with ctx.node_factory(Fn("my_grip")): detail contains `state` (the native string signal), `orientation`, `duration_ms` and `easing`. Use `signal_style` and `motion_signal` to define custom states/motion without a Rhai callback on pointer moves or animation frames. Both forms are decoration: they sit in a `grip` box centred on the separator and painted above both panes, and pressing anywhere on that box, overhang included, starts the drag. The keyboard steps, the separator role and the tab stop stay on the native handle. The grip takes the `grip_<state>` part while the handle is hovered, dragged, focused or disabled, from a native signal, so pointer moves never re-render Rhai. The handle itself takes `handle_hover` and `handle_drag` the same way (the `surface_hover` fill by default); give them the `handle` background to keep the lane still. `line: false` drops the native line for a grip that draws its own. Each pane is a flex column, so content with `flex_grow` fills it, and absolute content is placed from the pane's corner. A styled thin separator keeps its grip centred on the measured lane. For an invisible wider hit area, give `grip` a transparent background and `border(px(0))`; clear its state borders too if those should remain invisible on hover, focus and drag. `grip_content` and its state parts style the decoration independently of that hit area. A renderer owns its animated dimensions; the default bar owns its 24px -> 40px length.

```rhai
import "components/split_pane" as split_pane;

split_pane::SplitPane(#{ key:"main", label:"Resize panes", size:0.35,
    start:navigation, end:content, on_resize:Fn("resized") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Ignores drags and key steps and removes the separator from the tab order. |
| `end` | node | required | Content of the end pane: the trailing side, or the bottom when `vertical`. |
| `end_collapsed` | bool | `false` | Hides the end pane and the separator so the start pane fills the group; not with `start_collapsed`. |
| `end_key` | string | `"end"` | Key of the end pane's wrapper; keep it stable so the pane is not remounted. |
| `grip` | bool | `true` | Shows the default or supplied grip decoration; false keeps the native separator without a grip. |
| `grip_active_length` | number > 0, ≤ 512 | `40.0` | Logical-pixel length the default grip extends to while dragged. The stable hit area contains both endpoints. |
| `grip_length` | number > 0, ≤ 512 | `24.0` | Resting logical-pixel length of the default grip's motion range; custom renderers own their dimensions. |
| `handle` | node or `()` | — | Replaces the default animated bar with a static decorative Node; not together with `handle_renderer`. Pressing it still uses the native separator. |
| `handle_renderer` | callback or `()` | — | Node factory bound with ctx.node_factory(Fn(...)), called as Fn(ctx, detail). detail has native `state`, `orientation`, `duration_ms` and `easing`; use motion_signal for custom native animation. Not an input callback; not with `handle`. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `keyboard_step` | number > 0, ≤ 512 | `8.0` | Logical pixels one arrow-key press on the focused separator moves it. |
| `label` | string | required | Accessible name of the separator. |
| `line` | bool | `true` | Paints the native line down the separator; set `false` when `handle` draws its own. |
| `line_inset` | number 0–256 | `4.0` | How far the native line stops short of each end of the separator, in logical pixels. |
| `max_start` | number 0–16384 | `16384.0` | Largest start-pane length a drag or key step may propose, in logical pixels; at least `min_start`. |
| `min_end` | number 0–16384 | `120.0` | Smallest end-pane length a drag or key step may leave, in logical pixels. |
| `min_start` | number 0–16384 | `120.0` | Smallest start-pane length a drag or key step may propose, in logical pixels. |
| `on_resize` | callback or `()` | — | Called with the proposed start-pane ratio when a drag ends or an arrow key moves the separator. |
| `orientation` | `"horizontal"` or `"vertical"` | `"horizontal"` | `horizontal` puts the panes side by side; `vertical` stacks them. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `size` | number 0–1 | required | Start-pane share of the group's length; the caller stores the `resize` payload and passes it back. |
| `start` | node | required | Content of the start pane: the leading side, or the top when `vertical`. |
| `start_collapsed` | bool | `false` | Hides the start pane and the separator so the end pane fills the group; not with `end_collapsed`. |
| `start_key` | string | `"start"` | Key of the start pane's wrapper; keep it stable so the pane is not remounted. |
| `style` | style | — | Style merged over the root part. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `resize` | `on_resize` | number 0–1 | Emitted once when a drag ends or a key step moves the separator; a drag or step that leaves it in place emits nothing. The payload is the next `size`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `end`, `grip`, `grip_content`, `grip_content_disabled`, `grip_content_drag`, `grip_content_focus`, `grip_content_hover`, `grip_disabled`, `grip_drag`, `grip_focus`, `grip_hover`, `handle`, `handle_drag`, `handle_hover`, `root`, `start`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `metrics.control`, `radius.sm`, `spacing.md`, `spacing.xs`, `surface_hover`, `surface_raised`, `text_muted`
- Environment: `density`, `size`
