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
solid ColorValue or the same validated two-stop gradient used by Style;
`canvas_stroke_path` accepts a positive width and ColorValue. A path must start
with `path_move`, contain 2–10,000 segments, and have a safe unique command key.

Transforms apply uniform scale, rotation about the Canvas origin, then logical
translation. `clip_rect` is an axis-aligned Canvas-local content mask and does
not rotate with the path. All coordinates/transforms are finite, sizes/scales
are positive, and retained `canvas_commands` budget accounting includes every
path segment so one command cannot hide unbounded work.

Pointer down/up/move payloads on a keyed Canvas use committed node geometry for
`local`/`content` coordinates and add `canvas_key`, the topmost hit command in
reverse paint order or `()`. Rect/circle/line use analytic tests; paths flatten
quadratic/cubic curves deterministically and test fill/stroke plus clip. The
window capture router preserves the same local coordinates and key after a
handler captures the pointer. Rhai attaches an ordinary node handler; no
Canvas-specific callback lane exists.

The remaining Canvas contract is per-command semantic/accessibility nodes,
signal-bound geometry/paint, multi-stop/path-relative gradients, stroke
joins/caps/dashes, nested transform/clip groups, and automation geometry.
