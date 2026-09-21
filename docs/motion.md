# Motion Runtime 2

gpui-rhai 0.1.3 treats motion as runtime infrastructure. Rhai declares typed
sources and control intent; Rust samples frames, reads geometry and scroll
state, enforces budgets, and paints. Rhai is never called from layout,
prepaint, paint, or a per-frame callback.

Runtime API 2 deliberately removes the old `transition`, `spring`,
`loop_transition`, and `.animate(...)` API. There are no compatibility aliases.

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

Supported properties are `opacity`, `translate_x`, `translate_y`, `rotate`,
`scale_x`, `scale_y`, `skew_x`, `skew_y`, `width`, `height`, `clip_height`, and
`path_progress`. Ordinary GPUI nodes support opacity, translation, and
dimensions. Canvas/path primitives additionally support 2D rotation, scale,
skew, trim, follow, and compatible-topology morphing. A Canvas command with an
axis-aligned clip rectangle rejects rotate/scale/skew motion because GPUI 0.2.2
does not expose an equivalent transformed clip; the runtime does not paint and
hit-test two different shapes. GPUI 0.2.2 has no public arbitrary-subtree
scale/rotate transform, so the generic node API does not claim otherwise.

Use `.motion_replay_key(value)` when a semantically new value should replay an
unchanged declaration. Ordinary rerenders and compatible hot reload preserve
progress and velocity.

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

## Enter, exit, layout, and shared layout

`.enter_motion(source)` runs on a real keyed mount or changed replay key.
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

`motion_group` is a layout-transparent fragment. Shared IDs are unique inside
one view/window presentation domain. Direct window, splitter, Table-column,
and other pointer-following resize remains immediate. The generic GPUI 0.2.2
FLIP layer guarantees positional transforms; typed Canvas/path primitives own
scale and rotation.

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
cargo run --release -p gpui-rhai --example motion_gallery
```

The Gallery is an executable acceptance surface, not a prepare-only catalog:
it includes timeline play/pause/seek/restart controls, controlled Tabs, keyed
list reorder, and shared-layout selection switching.
