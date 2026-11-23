# Motion Runtime 2

gpui-rhai 0.1.3 treats motion as runtime infrastructure. Rhai declares typed
sources and control intent; Rust samples frames, reads geometry and scroll
state, enforces budgets, and paints. Rhai is never called from layout,
prepaint, paint, or a per-frame callback.

Since 0.1.3 (Runtime API 2) the old `transition`, `spring`,
`loop_transition`, and `.animate(...)` API is gone. There are no compatibility
aliases.

## Property sources

Every keyed node may own one source for each property:

```rhai
row([]).with_key("status").motion(
    motion_transition("opacity", 0.0, 1.0, #{
        duration_ms: ctx.motion_duration("normal"),
        easing: ctx.motion_easing("entrance"),
        intent: "feedback",
    })
)
```

The constructors are `motion_transition`, `motion_spring`,
`motion_keyframes`, and `motion_inertia`. Their configuration maps are strict:
unknown fields, invalid physics, non-finite values, zero iterations, excessive
keyframes, or duplicate property ownership reject the candidate transaction.
`iterations` accepts a positive integer or `"infinite"`.

An `easing` is `"linear"`, `"ease_in"`, `"ease_out"`, `"ease_in_out"`, or a
CSS cubic Bézier from `cubic_bezier(x1, y1, x2, y2)`, which validates the
control points (`x1` and `x2` in [0, 1]; `y` may overshoot) and returns the
curve's spelling. Rust samples every curve; `ctx.motion_easing(role)` returns
the same spelling, so a theme's easing token may be a Bézier too:

```rhai
motion_transition("scale_x", 3.0, 1.0, #{
    duration_ms: ctx.motion_duration("normal"),
    easing: cubic_bezier(0.33, 1.0, 0.68, 1.0), // an ease-out cubic
})
```

Supported properties are `opacity`, `foreground_blur`, `foreground_grayscale`, `translate_x`, `translate_y`,
`translate_x_fraction`, `translate_y_fraction`, `rotate`,
`scale_x`, `scale_y`, `skew_x`, `skew_y`, `width`, `width_fraction`, `height`, `clip_height`,
`number_value`, `number_weight`, `overlay_reveal`, `primitive_progress`, and
`path_progress`. Ordinary nodes support opacity, foreground Gaussian blur and grayscale,
translation, rotation, scale, skew and dimensions. Blur samples are standard
deviations in logical pixels, clamped to 0–64. Grayscale samples desaturate
the composed foreground in sRGB and are clamped to 0–1. The affine motion samples
override the corresponding style fields and use the same border-box pivot.
Fractional translations use the corresponding width or height of the laid-out
box (e.g. 0.5 = half a box) and do not relayout the subtree. Canvas retains its existing vector-space
motion, with path trim, follow, and compatible-topology morphing. A Canvas
command with an axis-aligned clip rectangle still rejects vector rotate/scale/skew
motion: the runtime does not paint and hit-test two different shapes.

For a node whose accepted style changes, the optional argument to
`paint_transition(300, "ease_out", #{group_effects: true, delay_ms: 110})`
includes the final opacity and grayscale values in the paint transition.
Both values use one MotionRuntime track, start from the displayed pair, and
preserve explicit group isolation through the transition. The first frame
and Reduced/None take the final style immediately. Explicit opacity or
grayscale sources keep compositing ownership even if this option is set.

`width_fraction` sets native layout width to a fraction of the containing
block (sample clamped to 0–1), rather than a number of logical pixels as
`width` does. It reflows as the containing block resizes. If both are
sampled on one node, `width_fraction` wins; use one width driver per node.
Clipping is explicit with `style().clip()`. Use a positioned child
retaining its own full width for an undistorted reveal.

`number_value` can sample the visual value of a localized `ctx.number_text`
node or supply a value to other nodes from a keyed element marked
`.number_motion_source().with_ref(reference)`. Ordinary nodes that are
neither numeric text nor marked sources reject the property.

Call `motion_numbers([first_ref, second_ref, ...])` with 1–64 distinct refs
in a single component. Each ref must belong to a committed value track
in the same ScriptView. Apply `.sample_numbers(group, projection, index)`
to a numeric text node (`"amount"`, `"percent"`, `"sum"`, or
`"weighted_sum"`) or a box (`"width_fraction"`). Indices are zero-based;
pass 0 for a sum. Set the box's ordinary width or the numeric text's
value to its accepted state as a fallback. Prefer a constrained flex row
for contiguous width projections; style clips, gutters and min/max
constraints still apply after projection.

Use `.sample_flex_share(group, index, cutoff)` for a distribution with
real flex gaps and minimum lane sizes. Positive lanes above the fractional
cutoff grow from zero basis; collapsed lanes contribute neither size nor
gap. This projection divides by the sampled sum, not a quota, so use
`width_fraction` for capacity meters. Layout, paint and hit testing still
use the same native frame.

Use `.sample_strip_hint(group, index, fallback)` on a full-width box with
exactly one child aligned to its logical start. The lane midpoint uses
the same denominator as `width_fraction`. A fraction of 0 aligns the
child at the start, and 1 aligns it at the end, after measuring the child.
Use 0–1 for the accepted fallback before the lane can be sampled or when
it is empty. The projection changes only position, not focus or popup behavior.

All projections read the same native frame. For example, if the amounts
are 60 and 140 right now, the first width is 30% and the first percent
label reads 30, even if the accepted target amounts already changed.
An optional `number_weight` track on a source (default 1, clamped to 0–1)
only affects weighted sums. It does not affect the denominator, shares,
or widths. This supports animated attention without distorting the
distribution. No Rhai code runs on the sampling path. Missing tracks
leave all projections of that group at their declared fallback rather
than partially renormalizing the UI. Zero total projects zero width and
percent. Retained and accessibility text report accepted values.

For a capacity meter, `group.with_capacity(capacity)` defines a
nonnegative fixed capacity. Widths divide by `max(capacity, sampled total)`,
so an under-capacity track leaves unused space and an over-capacity track
does not overflow. `"remaining"` is `max(0, capacity - sampled total)`;
`"capacity_percent"` is the sampled total divided by capacity times 100.
Positive use with zero capacity falls back to the declared text instead
of formatting infinity. Per-lane `"percent"` still describes the share of
usage, not of capacity.

`overlay_reveal` attaches to a non-modal overlay, not its trigger or body.
`primitive_progress` attaches only to a native primitive node. Its current
normalized sample is exposed to Rust in `PrimitiveTheme::motion_progress()`;
the adapter can paint with it without owning a second clock or frame loop.
The native positioner samples it in 0–1 and reveals only the actual panel.
It fades and scales from the edge of the final (possibly flipped) placement,
with a short translation from the anchor. The trigger keeps its layout and
focus. HoverCard uses this path through the same MotionRuntime.

Use `.motion_replay_key(value)` when a semantically new value should replay an
unchanged declaration. Ordinary rerenders and compatible hot reload preserve
progress and velocity.

Use `.motion_on_change(value)` instead when a finite entry should **not** play
on mount or remount. The initial presentation uses its terminal value. A new
key replays from the declared start; an equal key keeps progress. Changing only
the source (for example its themed duration) settles at the new terminal value
without inventing a content change:

```rhai
text(status).with_key("status")
    .motion(motion_transition("opacity", 0.0, 1.0, #{duration_ms:480}))
    .motion_on_change(status)
```

This modifier controls the node's ordinary motion sources and its animated
rich-text spans, not descendants, timelines, exit sources or progress bindings.
Infinite sources are rejected; mount must have a defined resting value. The
last `motion_on_change` / `motion_replay_key` modifier wins. Both use the same
retained identity, transactional budget, native clock, suspension and Host
motion policy; neither adds frame-time script work. Native intrinsic-width,
arbitrary-subtree scale and text blur are separate rendering capabilities,
not effects implemented by this trigger.

## Timelines

Timelines compose tracks without frame-time Rhai:

```rhai
let intro = motion_timeline("intro", motion_sequence([
    motion_track(".", motion_transition("opacity", 0.0, 1.0,
        #{ duration_ms: 160 })),
    motion_parallel([
        motion_track("title", motion_spring("translate_y", 12.0, 0.0, #{})),
        motion_stagger([
            motion_track("row-a", motion_transition("opacity", 0.0, 1.0,
                #{ duration_ms: 120 })),
            motion_track("row-b", motion_transition("opacity", 0.0, 1.0,
                #{ duration_ms: 120 })),
        ], ctx.motion_stagger("normal")),
    ]),
]), #{ autoplay: true, on_complete: Fn("intro_complete") });

column(children).with_key("panel").timeline(intro)
```

Inside a callback, `ctx.motion_handle("intro")` returns a scoped typed handle.
The handle is bound to the runtime, presentation domain, component incarnation,
script generation, and one allocated timeline instance. A compatible rerender
keeps it; removal/remount, replacement, or generation change makes the old
handle stale. Use `play_motion`, `pause_motion`, `resume_motion`, `seek_motion`,
`restart_motion`, and `cancel_motion`. `play_motion` is idempotent while playing
and resumes a paused position; only `restart_motion` rewinds. Pause and seek
immediately produce a complete snapshot for the requested position. Terminal
callbacks are frozen into the batch of the frame that sampled them, then run
after that frame commits in independent transactions. Later input cannot be
drained by an older frame, and one failure cannot discard its neighbors.
Missing, stale, cross-view, and duplicate handles fail explicitly.

## Native triggers and progress

Native progress values are qualified by their driver as well as retained node
and property. A binding removed or changed to another driver cannot read the
old driver's value or clock. First presentation uses the declared inactive
value. A mouse release outside a press owner still ends its press; disabling
returns it to rest. Retained-page hiding, suspension and binding removal revoke transient
press state, without rewinding unrelated motion sources. Cached visible
islands keep their active press ownership; an unpresented owner does not.

`motion_hover`, `motion_press`, and `motion_focus` animate between inactive and
active progress. `motion_in_view`, `motion_viewport`, and
`motion_scroll("x" | "y", source)` use committed geometry and native
`ScrollHandle` state:

```rhai
node.progress_motion(motion_hover(
    motion_transition("opacity", 0.82, 1.0, #{ duration_ms: 120 })
));
```

These paths never synthesize high-frequency Rhai events. Raw pointer payloads
still expose `movement`, `velocity`, and timestamps for seeding inertia.

`motion_signal(state, ["hover", "drag"], source)` uses the same native trigger
sampler for a string signal supplied by a component. It animates toward the
active endpoint while the signal matches one of the named states, and reverses
from its current sample otherwise. Each property has its own clock and duration.
Reduced/None policy selects the current state's static endpoint, not always the
active endpoint. Signal lifetime is checked before committing a view transaction.
For example, SplitPane's `handle_renderer` receives `detail.state`; a custom
decoration can attach `progress_motion(motion_signal(...))` without taking over
pointer capture, focus, cancellation, or the controlled panel ratio.

## Enter, exit, layout, and shared layout

`.enter_motion(source)` is `.motion(source)` under a name that states the
intent: a property source plays from its start value when the keyed node
mounts or its replay key changes.
`.exit_motion(source)` creates an immutable paint ghost after the real node has
left layout, hit testing, focus, accessibility, callbacks, and resource
ownership. Text, RichText, Canvas, SVG, images, and boxes/fragments composed
entirely from those kinds are supported. Custom primitives, overlays, layers,
virtual collections, and error boundaries are rejected during reconciliation
instead of silently losing content.
Live-tree reconciliation and exit-scene retention are separate: unrelated
state/store updates cannot reclaim a ghost before its source completes.

Layout motion is opt-in:

```rhai
motion_group("cards", [
    card.with_key("target")
        .shared_layout("server-42")
        .layout_motion(ctx.motion_duration("normal"), ctx.motion_easing("standard"))
])
```

`motion_group` is a layout-transparent fragment. A shared-layout group belongs
to the component instance that named it (the one calling `motion_group` or
`.shared_layout(group, id)`), so two instances of a component keep separate
identities; within one instance a group and id are unique in the view/window
presentation domain. Direct window, splitter, Table-column,
and other pointer-following resize remains immediate. The generic gpui-pre 0.3.11
FLIP layer guarantees positional transforms; typed Canvas/path primitives own
scale and rotation.

Group membership is inherited presentation context. When a formal component
or virtual item declares only `.shared_layout(id)`, an enclosing
`motion_group(group, ...)` supplies the group and the runtime replays that
context after component-local incremental updates, nested replacement, hot
reload, and later virtual realization. A node-local
`.shared_layout(group, id)` remains explicit and takes precedence.

## Text and paths

`motion_text_spans(text, config)` segments Unicode extended grapheme clusters
in Rust and produces stable, native-sampled RichText spans. Accessibility still
sees one continuous string.

Canvas supports `path_progress` stroke trim,
`canvas_morph_stroke_path` with strict matching topology, and
`motion_path_follow(scene, key, "translate_x" | "translate_y" | "rotate", config)`.
Path lookup tables and sampling stay in Rust.
Morph interpolation, trim, clip, stroke width and the outer affine transform
produce one presented Canvas geometry shared by paint and hit testing.

## Theme, accessibility, and quality

Themes provide semantic motion tokens, with validated defaults when omitted:

- durations: `instant`, `fast`, `normal`, `slow`, `ambient`;
- easings: `standard`, `entrance`, `exit`, `emphasized`;
- springs: `responsive`, `gentle`, `bouncy`;
- distances: `subtle`, `moderate`, `large`;
- staggers: `tight`, `normal`, `relaxed`.

Read them with `ctx.motion_duration`, `motion_easing`, `motion_spring`,
`motion_distance`, and `motion_stagger`. These reads are tracked theme
dependencies, so hot switching rerenders exact readers and retargets from the
current sample.

Hosts choose `MotionPreference::{Normal, Reduced, None}` and
`MotionQuality::{High, Medium, Low}`. Scripts classify intent as `decorative`,
`feedback`, or `essential` and cannot relax stricter Host policy. `None`
immediately presents the deterministic terminal/static projection across
property, timeline, layout, and native trigger paths; later play/restart calls
cannot re-enable it. View suspension freezes the same Host clock position even
when another window continues sampling the shared runtime.

Direct property sources and timeline tracks use the same transition,
keyframe, spring, and inertia samplers. Physical retargets inherit sampled
velocity; completed snapshots retain the actual terminal sample. The runtime
validates timeline targets and one-owner-per-property plans atomically before
committing them. Layout/trigger paths reserve capacity in the same per-runtime
active-work budget, and play/restart recheck the combined total.
Mounted nodes use `presentation domain + retained NodeId` identity; readable
timeline targets are resolved to that identity during planning. Headless Rust
reconciliation uses collision-free encoded path segments, so keys containing
`/`, numeric text, or reserved-looking prefixes do not alias. Inertia uses
closed-form exponential/collision sampling rather than reintegrating its full
age every frame.
Compatible timeline rerenders preserve time position while replacing their
resolved target bindings, so a child remount receives the current sample and
the old NodeId disappears immediately. Candidate budgets are evaluated once
from the completely installed plan, independent of declaration order. Window
release is a stronger boundary than cancel: it clears suspension state and
cannot match neighboring IDs such as `w2`. An inertia duration cap freezes its
actual sample; only explicit snap points may move it to an attachment.

## Rust Hosts and the effect seam

Rust uses the same `MotionSource`, `MotionTimeline`, `MotionHandle`, clock,
theme, budget, and Inspector types. `PrimitiveDescriptor::effect` declares a
typed effect primitive's supported platforms, instance/cost budgets, scoped
lifecycle, reduced-motion behavior, and quality tiers. `PrimitiveTheme`
contains the effective policy and resolved motion tokens. Arbitrary shader
source and general 3D are intentionally deferred beyond 0.1.3.

The optional source pack lives under `motion/*`:

```bash
cargo run --release -p gpui-rhai-cli -- gallery --story motion/catalog
```

The Gallery is an executable acceptance surface, not a prepare-only catalog:
it includes timeline play/pause/seek/restart controls, controlled Tabs, keyed
list reorder, and shared-layout selection switching.
