# Typed Style surface

`Style` is a validated runtime value, not a GPUI handle or CSS string. Explicit
merge, the application component stylesheet, and component parts determine
precedence. On ordinary nodes `hover`, `active` and `focus` are paint
refinements: background/gradient, border/text color, shadow stacks and opacity apply, in
the order `base → hover → focus → active`: while a node holds keyboard focus,
its focus paint wins over hover paint on a property both set, and active paint
wins over both. `group_hover` paints over focus. `group_focus`, `focus_within`
and `disabled` merge the whole nested style before paint; while a node is
disabled its pointer and focus paints stop. A pseudo style nested inside a
pseudo style is ignored.

A merge (`merge`, a stylesheet rule, `part_styles`) gives the same result as
chaining the overlay's setters: `padding(x)`, `margin(x)`, `border(x)` and
`radius(x)` replace the logical edges too, and `background`,
`linear_gradient` and `layered_gradient` replace the same fill. A logical edge (`*_start`, `*_end`,
`inset_start`, `inset_end`) wins over a physical one on the same side in any
order and any layer; to change a component's logical edge from a stylesheet,
set that logical edge or use the all-edge setter.

Formal component authors resolve every public part through
`ctx.component_style("part", base)`. The runtime merges the matching
`ui/styles.rhai` rule before the caller's explicit instance override. See
[component stylesheets](component-styles.md).

The shared public builder targets gpui-pre 0.3.11 and gpui-fast 0.1.13 separately:

- block, flex, and bounded explicit grid layout (`grid_cols/rows`, spans and
  one-based `grid_cell(column, row)` placement);
- row/column direction, wrapping, boolean or positive weighted grow,
  shrink/basis, alignment, and justification; `items_baseline()` sets a row's
  text children on the baseline of the row's own text style, from the font
  metrics GPUI paints with (GPUI gives Taffy no text baselines, so other
  children start);
- intrinsic `overflow_row` packing with optional priority cohorts and an
  explicit `overflow_required` reset. Cohorts collapse before native paint,
  without rebuilding Rhai or duplicating interactive controls. This reserves
  full intrinsic preferred width/line height, not arbitrary flex distribution;
  grid, wrap, baseline and stretch override it. Hidden cells remain owned and
  measured, but have no presented drawing, keyboard target or semantic node;
- min/max/fixed/auto sizing and flex basis, gaps, physical/logical padding and
  definite/auto/signed margins;
- relative/absolute positioning with definite/auto/signed four-edge insets and
  per-axis overflow;
- solid, two-stop or layered gradient backgrounds, physical/logical
  per-edge border widths, one uniform solid/dashed border style, per-corner
  radii, validated drop/inset shadow stacks, non-layout focus rings, opacity,
  visibility, and cursor policy;
- paint translation with signed finite logical pixels;
- explicit `occlude()` or `occlude_except_scroll()` hitbox policy for blocking
  pointer input to painted elements behind a node;
- font family, ordered fallback stack, bounded OpenType feature tags, numeric
  weight, normal/italic style, size/line height, logical text alignment
  (`text_start`, `text_center`, `text_end`; `text_left` and `text_right` are
  the same as `text_start` and `text_end` and mirror in RTL), whitespace,
  ellipsis, and bounded line clamp.

`style().flex_grow()` uses GPUI's ordinary factor of 1. The overloaded
`style().flex_grow(weight)` accepts a positive integer or float and preserves
that numeric factor through merge, serialization, theme resolution, and GPUI
layout. Use a zero relative basis when weights should divide only the remaining
main-axis space:

```rhai
style().flex_basis(relative(0)).flex_grow(2).min_width(px(0))
```

`relative(2)` is not a grow weight; relative lengths are fractions of the
containing size and remain bounded to their own length contract.

`theme_length("metrics.grid").without_reading_scale()` keeps the token
late-bound to the current theme and size/density variants, but does not multiply
it by the reading-scale preference. Use this for structural hairlines; control
and text metrics should normally keep scaling. Explicit `.times(...)` factors
still apply, and literal lengths are unchanged. The policy survives Style JSON,
theme export/reload and retained-layout cache identity. It is not device-pixel
snapping and does not disable the window's display scale.

`grid_cols(n)` uses equal fractional tracks; `grid_cols_max_content(n)` uses
independently content-sized tracks. Use the latter for short adjacent labels or
key combinations whose visible spacing must not include spare column width.
Both accept 1..1024 and work with `grid_cell(column, row)`. They use native
layout/text measurement, not script-computed glyph widths. A later `grid_cols`
or `auto_grid_columns` declaration replaces the column-sizing mode.

`style().inherit_text_color()` removes a lower-layer explicit text color and
restores native parent inheritance. This differs from an empty Style, which
preserves lower-layer declarations during merging. A later `text_color(...)`
replaces that reset. Themes can therefore opt out of a token-base foreground
without guessing the parent's color or hard-coding a different palette. The
reset is serialized and exported with the Style. It does not implement
contextual `currentColor` for backgrounds/SVG tint or isolated group opacity.

Use `style().isolated_opacity(true).opacity(0.5)` on a node's base style to
fade its native painted subtree as one composite. With the switch off,
`opacity(0.5)` still multiplies individual primitives. Isolation does not
change layout, focus, pointer targets, or the ownership of window-level
overlays. The current guarded compositor does not capture external native
surfaces inside a group.

`style().typography("body")` applies a typography role from the token base
(`caption`, `body_small`, `body`, `subtitle`, `title`, `heading`, `display`,
`display_large`, the metadata `label` and `key_legend` voices, `code`, and the
`control` roles) or one
an application adds; components declare the roles they read. A role may carry
its own family and fallbacks: `code` is the monospace face for identifiers,
paths and commands (`Menlo`, then `DejaVu Sans Mono`, `Liberation Mono`). A Host
changes it with `ThemeTokenOverrides` or an application with `ui/tokens.rhai`;
palette-only variants inherit the base type. Full Modern/Clean presets carry
their own type/geometry recipes as well as colors.
Resolution happens during render, so a live change updates text without
recompiling Rhai. Explicit family, fallbacks, size, line height, or weight
chained onto the style override that field while retaining the rest of the
role.

A role the active theme does not define applies nothing and raises no error:
the text keeps its inherited font, so a view also runs without a token base.
Only a malformed role name is rejected when the style is built. An unresolved
`theme_color` or length token behaves the same way: the property stays unset.

`gap_x` and `gap_y` set physical horizontal and vertical spacing independently,
including between wrapped flex lines and in explicit/automatic grids. They
accept the same lengths and theme tokens as `gap`. RTL and flex direction do
not swap these axes. A later `gap(...)` resets both axes, including more
specific declarations from lower style layers; a later axis setter replaces
only that axis. For example, `.gap_x(px(16)).gap_y(px(8))` keeps a compact 8px
line gap when a toolbar wraps, without squeezing horizontal control spacing.

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

`radius_start` and `radius_end` round both corners on one logical inline side
and mirror in RTL, like `padding_start`; the first and last items of a joined
group use them so only the group's outer corners are rounded.

`shadow` and `linear_gradient` reject unknown fields. Opacity, grid counts,
font weights, line clamps, translations, lengths, and shadow geometry are
bounded before a GPUI element is built. Occlusion forces a retained interactive
wrapper even without callbacks; disabled nodes may still deliberately occlude.
Theme-backed colors resolve through the
same `ColorValue` path as solid fills and Canvas.

To paint a gradient body and edge, use separate origin and clip boxes on the
same node. Layers are topmost first:

```rhai
style().border(px(1)).border_color(rgba(0))
    .layered_gradient(layered_gradient([
        #{ origin:"padding", clip:"padding", stops:[
            #{offset:0,color:theme_color("surface_raised")},
            #{offset:1,color:theme_color("surface")}
        ] },
        #{ stops:[
            #{offset:0,color:alpha(current_color(),0.15)},
            #{offset:0.5,color:alpha(current_color(),0.08)},
            #{offset:1,color:alpha(current_color(),0.05)}
        ] }
    ]))
```

Offsets are ordered fractions; equal offsets create a hard edge. Each layer
accepts `angle` (180° by default), `origin` and `clip` (both border by
default), `size` as two size ratios, and `position` as two alignment ratios.
With `repeat_period: 7`, the same stops describe one cycle of seven logical
pixels along the gradient line, from the gradient image's start. The cycle
does not stretch with the box or reading scale; it follows display scale.
Supported periods are 0.5 to 65536 logical pixels. Omit it for one gradient
over the whole span.
The current native limit is 8 layers and 16 stops per layer. An empty stack
explicitly clears the fill. Layered gradients use [premultiplied sRGB
interpolation](https://www.w3.org/TR/css-images-3/#coloring-gradient-line).
The original two-stop gradient remains the Canvas path format.

Solid backgrounds, borders, shadows and layered stops accept `PaintColor`
from `current_color()`. It uses the node's final native foreground, including
parent and own state refinements. It is not a theme palette value and cannot
be passed to `text_color`, where it would refer back to itself.

`shadows([shadow(...), ...])` accepts at most 32 layers, topmost first. A shadow
supports signed spread and `inset:true`; blur remains non-negative. An empty
stack clears inherited shadows, and no layer consumes layout space.
`focus_ring(px(2), px(2), color)` paints an exterior cue with a transparent gap;
`focus_ring(px(2), offset_px(-2), color)` paints an inset cue. Both use the real
focus owner and Host indicator policy without extra layout or hit geometry.
Explicit `group_focus`/`focus_within` refinements can borrow that focus scope.

`px` and `rem` are non-negative `Length` values accepted by every length
property. A `relative` fraction is honored by sizes, flex-basis, gap, padding,
margin and insets; border widths, corner radii, font size and line height have
no fraction in GPUI and ignore it, also when a length token resolves to one. `auto()` is a separate `AutoLength` accepted
only by size, flex-basis, margin, and inset methods. `offset_px`, `offset_rem`,
and bounded `offset_relative` return `SignedLength`, accepted by margins and
insets. Pixel/rem offsets also work as focus-ring offsets; relative ring offsets
are rejected. Passing a signed length to padding, border, radius, gap, font size, or width is a
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

`foreground.over(background)` computes encoded-sRGB source-over with both alpha
channels; `mix` is channel interpolation, not compositing. Color expressions keep
token leaves late-bound. `readable` uses RGB contrast, so composite translucent
inputs onto their known opaque backing before evaluating it. Computing a color
does not implement isolated opacity groups or backdrop filtering.

`style().foreground_blur(2.5)` filters this node's composed foreground, not
what has already been drawn behind it. The Gaussian standard deviation is
0–64 logical pixels (capped at 128 device pixels). Zero clears an inherited
filter. The captured subtree includes its fill, text and children, and its
opacity is applied once after convolution. The halo does not change layout,
hit targets or keyboard focus and remains subject to ancestor clipping.
On a text-only wrapper it can blur just the glyphs. This path is currently
verified on the local Apple Metal candidates for both supported GPUI forks.

`style().foreground_grayscale(0.65)` desaturates the already-composed group
in sRGB. Zero clears a lower-priority filter, one makes the group fully
gray. It does not change coverage, geometry, or the pixels behind the
group; an explicit backdrop effect remains separate. Grayscale combines
with foreground blur and group opacity at the same composition boundary.
Use the `foreground_grayscale` motion property to vary the amount without
re-evaluating the script each frame. Rendering is verified only on the
local Apple Metal candidates.

`style().paint_reveal_x(4, 1)` gives an entry animation on the
`paint_reveal_x` motion property a 4-logical-pixel corner radius. The
fraction starts at the physical left edge. At 0 nothing paints; at 1
the normal unmasked subtree paints. The mask covers the composed fill,
stroke, and content without changing layout, hit regions, or focus
ownership. `without_paint_reveal_x()` clears an inherited reveal.
The current rendering verification is limited to the Apple Metal candidates.

`style().backdrop_blur(8)` reads the already painted pixels of the current
composition surface. It mirrors samples at the rounded panel's rectangular
sampling bounds, then limits output to the rounded border box and ancestor
clips. The panel tint, text and following content paint after the filter.
Sigma is in logical pixels, 0–64 (capped at 128 device pixels); zero clears
a lower-priority value. Descendant reads inside an isolated group see the
group's destination, even at full opacity. On the Apple candidates, an
isolated element's own backdrop instead seeds its group from the parent at
that draw's position. The node and children paint over this seed, then the
group's foreground filter and opacity apply to the result once. Try
“Rotate panel” with or without “Combine filters” on the Gallery's
Backdrop blur page. On the Apple candidates, placement-only transforms
keep both kinds of reads connected to preceding paint. Real opacity or
foreground filter isolation still sets a boundary for descendants.
An ordinary placed read replaces both color and alpha at its coverage,
even when the sampled surface is partially transparent; later drawing
composites over that result. This placement support has been validated
on Apple Metal with both candidate backends.

NativeSignal/animation bindings override literal values at frame sampling time
without rerunning Rhai. Ordinary element subtrees can use a visual affine:

```rhai
style().transform(#{rotate: 12, scale_x: 0.9, scale_y: 0.9, origin_x: 0.5})
```

Scale, skew and rotation are applied in that order about the border-box
origin. Angles are clockwise degrees, origin fractions default to 0.5 and
can be -16–16, and skew is -89–89°. Scale can be zero or have magnitude
1e-6–16; a zero scale collapses paint and pointer hits. An empty map resets the
transform. The effect changes neither layout allocation nor the raw
window-coordinate event payload. The present candidate uses inverse coordinates
for native editor and text-selection hits, and projects platform input and semantic
bounds. It does not yet certify all gesture primitives or overlay anchor placement.
This is a local Apple Metal candidate for both GPUI forks, not an upstream API.

`layout_motion` smooths committed positions; it snaps sizes. It does not provide
content-keyed intrinsic-width animation. A zero duration or removed declaration
cancels that node's active layout program without cancelling peers.

`optional_float_signal(key)` declares an initially inactive native value.
Binding it as `width_override` preserves the node's normal fixed/percent/flex
layout while null; a finite value becomes an authoritative non-growing,
non-shrinking pixel width. This lower-level contract powers Table divider drags
and is useful for native primitives that need live geometry without a Rhai
render. Virtual item renderers bind an owner-declared optional signal in one
boundary call with `node.bind_parent_signal(ctx, "width_override", key)`.

Hosts may register bounded in-memory TrueType/OpenType sources and file apps
automatically discover `ui/fonts`; `font_family` selects the internal family
name. See `assets.md` for ownership and validation.

The remaining final-style gaps include intrinsic/min-content/max-content and
custom fr/grid-track values, per-edge border colors/styles (GPUI stores only one
quad-wide color/style),
cross-surface filter composition and glyph masks, font aliases and hot
replacement, selection styling, and explicit hit-testing/stacking-context
controls beyond GPUI's two occlusion policies. General z-index is not exposed;
window-level ordering uses public Overlay/Layer priorities. They are
tracked as incomplete rather than silently ignored.
