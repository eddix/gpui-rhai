# Retained Canvas scenes

`canvas(scene)` paints a retained, keyed data scene without invoking Rhai during
GPUI layout, prepaint, or paint. Scene and command identity participate in the
normal UiNode snapshot/reconcile boundary.

Primitive commands are `canvas_rect`, `canvas_circle`, and `canvas_line`.
Arbitrary paths use typed segments:

```rhai
canvas_fill_path("wave", [
    path_move(0.0, 40.0),
    path_cubic(80.0, 0.0, 20.0, 0.0, 60.0, 80.0),
    path_line(120.0, 100.0),
    path_close()
], linear_gradient(#{
    angle: 90,
    from: color("#7aa2f7"),
    to: color("hsla(280, 60%, 60%, 40%)")
}))
    .translate(12.0, 8.0)
    .scale(1.25)
    .rotate(5.0)
    .clip_rect(0.0, 0.0, 160.0, 120.0)
```

`path_quadratic` and `path_cubic` retain control points. Fill paths accept a
solid ColorValue/PaintColor or the same validated two-stop gradient used by Style;
`canvas_stroke_path` accepts a positive width and ColorValue/PaintColor. A path must start
with `path_move`, contain 2–10,000 segments, and have a safe unique command key.

Open strokes may set `.stroke_cap("butt" | "round" | "square")` and
`.stroke_join("miter" | "miter_clip" | "round" | "bevel")`. The defaults are
butt/miter, with the native default miter limit. They apply to `canvas_line`,
stroked paths and morph strokes; filled-only commands and unknown names are
errors. A trimmed path gets the selected cap at its new endpoint, while a
fully closed path has no end caps:

```rhai
canvas_stroke_path("check", [
    path_move(4, 8.5), path_line(6.75, 11.25), path_line(12, 5)
], 1.8, current_color()).stroke_cap("round").stroke_join("round")
```

Native tessellation is shared by stroke paint and geometric hit testing,
including curves, caps, joins, morph/trim, view-box scale and node transforms.
This fixes phantom round hits beyond a butt-ended line and missing miter
corners. Unrepresentable native widths or projected coordinates produce no
triangles or hits rather than being sent to the tessellator. Stroke width
scales with the view box, not with command/node transforms, preserving the
existing non-scaling-stroke contract.

Rust line/path/morph literals include
`stroke_style: CanvasStrokeStyle::default()`. Use `with_stroke_style` with
the public `CanvasLineCap` / `CanvasLineJoin` enums to replace those defaults;
it rejects commands without a stroke. No private backend patch is needed.

## Native foreground paint

`current_color()` returns a deferred `PaintColor`, not a sampled theme color.
Solid rect/circle/line/path and morph-stroke commands accept it, including
alpha multiplication:

```rhai
canvas(canvas_scene([
    canvas_rect("ink", 0, 0, 16, 16, current_color()),
    canvas_circle("wash", 28, 8, 8, current_color().alpha(0.2))
])).with_style(style().width(px(40)).height(px(16))
    .text_color(theme_color("text_primary"))
    .hover(style().text_color(theme_color("text.accent"))))
```

The actual owning Canvas's native text-style stack supplies foreground during
paint, after inherited and own hover/focus refinements. Reusing a scene does
not freeze its first color, copy a Window into script data, or execute Rhai per
frame. Native element opacity still applies separately; `alpha` multiplies the
foreground's existing alpha rather than replacing it. Ordinary theme/literal
ColorValue expressions retain their normal resolver.

PaintColor is deliberately separate from a context-free theme color.
[Style](style.md) also accepts it for backgrounds, borders, shadows and layered
gradient stops. It is not accepted as a text color or in the original two-stop
Canvas path gradient. This API does not implement isolated group opacity or
text effects. Canvas remains a hot
native node under the existing cache policy; unrelated cold siblings remain
cache eligible.

Rust Canvas command solid colors now use `PaintColor`; convert an existing
`ColorValue` with `.into()`, or use `PaintColor::current_text()`. The command
paint expressions do not themselves change geometry or keys.

## Geometry and interaction

Transforms apply uniform scale, rotation about the Canvas origin, then logical
translation. `clip_rect` is an axis-aligned Canvas-local content mask and does
not rotate with the path. All coordinates/transforms are finite, sizes/scales
are positive, and retained `canvas_commands` budget accounting includes every
path segment so one command cannot hide unbounded work. `canvas_scenes`
separately caps the number of retained Canvas nodes, so many empty or tiny
scenes cannot bypass the command budget.

Pointer down/up/move payloads on a keyed Canvas use committed node geometry for
`local`/`content` coordinates, include the Canvas visual bounds as `target`, and
add `canvas_key`, the topmost hit command in reverse paint order or `()`.
Raw rect/circle queries use analytic tests; presented fills use the projected
polygon/flattened contour, and strokes use the same native triangles as paint,
with the same trim and clip. The
window capture router preserves the same local coordinates and key after a
handler captures the pointer. Rhai attaches an ordinary node handler; no
Canvas-specific callback lane exists.

For a transformed Canvas, the command hit first maps the raw window point
into the actual drawable plane, not the node's visual bounding box. It then
uses the same Canvas motion and view-box projection as paint. SelectionArea
targets use the command source coordinates. Its live marquee stays aligned
to the window-space rectangle used to commit the selection, even when the
content is mirrored or rotated. PanZoom measures translation in the
viewport's logical axes; changing display orientation does not reset the
controlled transform.

The remaining Canvas contract is per-command semantic/accessibility nodes,
signal-bound geometry/paint, multi-stop/path-relative gradients, stroke
dashes, nested transform/clip groups, and automation geometry.
