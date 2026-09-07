# Typed Style surface

`Style` is a validated runtime value, not a GPUI handle or CSS string. Explicit
merge, the application component stylesheet, and component parts determine
precedence; pseudo refinements resolve in
`base → hover → active → focus → disabled` order.

Formal component authors resolve every public part through
`ctx.component_style("part", base)`. The runtime merges the matching
`ui/styles.rhai` rule before the caller's explicit instance override. See
[component stylesheets](component-styles.md).

The current public builder maps directly to stable GPUI 0.2.2 behavior:

- block, flex, and bounded explicit grid layout (`grid_cols/rows`, spans);
- row/column direction, wrapping, grow/shrink/basis, alignment, justification;
- min/max/fixed/auto sizing and flex basis, gaps, physical/logical padding and
  definite/auto/signed margins;
- relative/absolute positioning with definite/auto/signed four-edge insets and
  per-axis overflow;
- solid or typed two-stop linear-gradient backgrounds, physical/logical
  per-edge border widths, one uniform solid/dashed border style, per-corner
  radii, validated box shadows, opacity, visibility, and cursor policy;
- paint translation with signed finite logical pixels;
- explicit `occlude()` or `occlude_except_scroll()` hitbox policy for blocking
  pointer input to painted elements behind a node;
- font family, ordered fallback stack, bounded OpenType feature tags, numeric
  weight, normal/italic style, size/line height, logical text alignment,
  whitespace, ellipsis, and bounded line clamp.

`style().typography("body")` applies one of the theme's eight validated semantic
roles (`caption`, `body_small`, `body`, `subtitle`, `title`, `heading`,
`display`, `display_large`). Resolution happens against the active theme during
render, so a live theme switch updates text without recompiling Rhai. Explicit
family, fallbacks, size, line height, or weight chained onto the style override
that field while retaining the rest of the role.

```rhai
style()
    .grid_cols(3).gap(px(12)).padding(px(20))
    .linear_gradient(linear_gradient(#{
        angle: 145,
        from: rgba(0x24283be8),
        to: rgba(0x16161ee8)
    }))
    .shadow(shadow(#{
        x: 0, y: 18, blur: 48, spread: 2,
        color: rgba(0x00000070)
    }))
    .font_family("Avenir Next")
    .font_fallbacks(["PingFang SC", "Noto Sans"])
    .font_feature("liga", 1).font_feature("ss01", 1)
    .font_weight(650)
    .border_top(px(1)).border_start(px(2)).border_dashed()
    .margin_x(auto()).top(offset_px(-8))
    .radius_top_left(px(12)).radius_bottom_right(px(4))
    .occlude_except_scroll()
    .opacity(0.92).cursor_pointer()
```

`shadow` and `linear_gradient` reject unknown fields. Opacity, grid counts,
font weights, line clamps, translations, lengths, and shadow geometry are
bounded before a GPUI element is built. Occlusion forces a retained interactive
wrapper even without callbacks; disabled nodes may still deliberately occlude.
Theme-backed colors resolve through the
same `ColorValue` path as solid fills and Canvas.

`px/rem/relative` remain non-negative `Length` values and are accepted by every
compatible layout/paint property. `auto()` is a separate `AutoLength` accepted
only by size, flex-basis, margin, and inset methods. `offset_px`, `offset_rem`,
and bounded `offset_relative` return `SignedLength`, accepted only by margins
and insets; passing one to padding, border, radius, gap, font size, or width is a
Rhai type error. The retained `LayoutLength` preserves the distinction through
serialization and maps directly to GPUI's `Length::Auto` or signed definite
length rather than being approximated in paint.

`color(string)` accepts strict `#rgb/#rgba/#rrggbb/#rrggbbaa`, CSS basic named
colors plus `transparent`, comma-form `rgb/rgba`, and `hsl/hsla` with explicit
percentage saturation/lightness, plus comma-form `hwb` with percentage
whiteness/blackness and optional alpha. Channels are range checked; malformed
or unsupported strings are errors. Lab/LCH and OKLab/OKLCH remain explicit
future extensions until the runtime defines color-space and gamut-mapping
policy rather than silently approximating them.

NativeSignal/animation bindings override literal opacity/translation/dimension
values at frame sampling time without rerunning Rhai. Static translation uses
the same paint wrapper, so visual geometry reporting remains the next required
step before transformed hit testing can be marked complete.

Hosts may register bounded in-memory TrueType/OpenType sources and file apps
automatically discover `ui/fonts`; `font_family` selects the internal family
name. See `assets.md` for ownership and validation.

The remaining final-style gaps are intrinsic/min-content/max-content and custom
fr/grid-track values, multi-stop gradients, rotation/scale and
transform-origin, per-edge border colors/styles (GPUI 0.2.2 stores only one
quad-wide color/style), font aliases and hot
replacement, selection styling, and explicit hit-testing/stacking-context
controls beyond GPUI's two occlusion policies. General z-index is not exposed;
window-level ordering uses public Overlay/Layer priorities. They are
tracked as incomplete rather than silently ignored.
