# Native text geometry

An element's bounds are not its caret or selection geometry. An element can
wrap, align different visual rows independently, contain several directions
of writing, or use a font that combines several graphemes into one glyph.

`gpui_rhai::text_geometry::TextGeometry` projects the same `gpui::TextLayout`
that an element paints. Construct it after the element's measurement and
prepaint; pass the alignment from that element's active native text style.
Do not independently shape the same string to predict its painted positions.

- `caret(grapheme, affinity)` uses an extended grapheme boundary, including
  the end. Upstream and downstream can differ at a wrap or a bidi boundary.
  A glyph cluster that covers more than one grapheme produces a real cluster
  edge; the returned byte offset says where it landed. No intermediate
  position is inferred from an average advance.
- `range(start..end)` returns the painted cluster rectangles that touch that
  grapheme range. Rectangles stay disjoint when intervening visual clusters
  are not selected. Newline-only ranges can have no painted area.
- `hit(point)` returns a UTF-8 byte boundary, snapped to a painted cluster
  edge. Pass a layout-space point. Apply any current native transform once
  at the boundary when drawing or handling a window-space event.

This is a snapshot of a native presentation, not a position cache for a
document revision. Recompute from the next painted layout after text,
viewport, typography, or direction changes; a script's requested offsets
must belong to the same accepted document text. A native layout that cannot
be matched to its presented byte stream has no projection. The API does not
declare a Rhai method, an editable-document model, or a collaboration policy.

Selectable text uses the same projection for pointer hits and range painting.
It builds it lazily for a selection or pointer gesture; idle text does not
walk its glyphs. Copy still goes through the view's normal platform action.

## Versioned anchors in Rhai

Inside a formal component render, create one plan for the accepted document
string, revision and requested grapheme ranges:

```rhai
let reference = element_ref("document-text");
let plan = text_anchor_plan(source, revision, [
    #{ id: "caret", start: 4, affinity: "downstream" },
    #{ id: "selection", start: 4, end: 9 },
]);
let provider = text(source).with_ref(reference).capture_text_anchors(plan);
let anchors = ctx.text_anchors(reference, plan);
```

The provider can instead be `text([span(...), ...])` if the spans'
concatenated string is exactly `source`. A capture is published after its
native paint, not synchronously during render. It is absent (`()`) if the
provider is not presented, the native layout truncates the source, or any of
the source, revision or requests differ. A render read is tracked, including
first presentation and binding changes. Keep a structural component ID stable
across ordinary reflow; use a new document or revision when authoritative
content changes.

The result is a map keyed by the request IDs. Each item has `rects` (an
array), and a caret additionally has a visual `caret` point and actual
`byte` offset. Rectangles include `layout`, `visual` and `clipped` bounds
plus `quad`, the four transformed corners in window logical pixels. A
fully clipped rectangle has `clipped: ()`; do not position a floating
control over it. Draw transformed selection with the quad, not the
axis-aligned bounding box. A singular transform has no visual geometry.

The current caps are 16 KiB of source, 32 requests per plan, 256 returned
rectangles per provider and 128 painted providers per view. Excess requests
are rejected; excess output does not publish a partial selection. The
Gallery's **Document anchors** scene exercises an accepted replacement,
same-revision cursor motion, comment placement, reflow and scrolling.
