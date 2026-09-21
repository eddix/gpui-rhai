# ADR 0020: Motion Runtime v2 and effect-layer boundaries

## Status

Accepted for 0.1.3.

## Context

The 0.1.2 component foundation exposes a small node-attached animation model:
numeric transition, looping transition and spring declarations over seven
properties. It proves native-clock sampling and reduced-motion behavior, but it
cannot express one-source-per-property ownership, keyframes, timelines,
enter/exit, committed-geometry layout transitions, shared layout, scroll and
gesture progress, typed Host control, per-span text motion or vector-path
motion. Retaining it beside the final system would create two incompatible
animation lifecycles.

The product target is not a set of privileged visual-effect widgets. Arbitrary
application-authored Rhai and trusted Rust Hosts must share the same motion
engine, policies and diagnostics. First-party `motion/*` source components are
both useful products and completeness tests for that public substrate.

GPUI 0.2.2 provides GPU-backed quads, glyphs, images, paths, shadows and
platform surfaces, but no stable cross-platform arbitrary-shader/render-pass
API and no general element/text blur filter. Browser WebGL/GLSL code therefore
cannot be treated as a portable GPUI capability.

## Decision

### Capability stack

The implementation has three vertical infrastructure layers:

1. **Motion** owns typed property sources, transition/spring/keyframe/inertia
   sampling, timelines, triggers, lifecycle, reduced motion and deterministic
   clocks.
2. **Effect primitives** (the earlier `EffectCanvas` concept) own bounded,
   typed 2D batching for Canvas/path/particles and per-span text effects. It is
   a family of cohesive primitives, not one universal object.
3. **ShaderSurface** is a future cross-platform typed GPU-effect/3D project.
   0.1.3 exposes the custom-primitive extension seam but does not expose
   arbitrary shader source or promise general 3D/perspective semantics.

Rhai and Rust are horizontal authoring/control frontends over the same layers.
The Rust model is canonical; Rhai constructors decode into the same typed data.
A Host may register a typed GPU primitive with declared platform support,
schema, budgets, theme snapshot, motion policy and scoped lifecycle. Direct
unmanaged GPUI animation remains possible but receives no automatic theme,
reduced-motion, budget, automation or cleanup guarantees.

### Property ownership and clocks

Each animatable property has one active owner above its literal style baseline:
native signal, transition, spring, keyframes, inertia, progress binding, or one
resolved timeline. Rust validates the complete target/ownership plan before it
commits and samples it using the Host `RuntimeClock`; Rhai never executes during
frame sampling, GPUI layout, prepaint or paint. Retargeting begins from the
currently sampled value and velocity. Presentation domain, retained NodeId,
component incarnation, script generation, property, and allocated motion
instance jointly determine authority and lifetime. Human-readable timeline
targets and fallback headless paths are planning inputs, not mounted identity.
Keeping a timeline owner preserves playback state, not stale child bindings:
each accepted retained tree recompiles target/property bindings to the current
NodeIds. Resource admission evaluates the fully installed candidate plan, not
the order in which individual declarations were visited.

The old public `AnimationSpec`, `transition`, `spring`, `loop_transition` and
`node.animate` surface is removed without aliases. Official source, examples,
metadata and documentation migrate together. This is Runtime API 2.

### Timeline and triggers

Declarative state-driven property sources remain the default. Explicit
timelines add sequence, parallel, stagger, delay, repeat, reverse,
play/pause/resume/restart and seek. Typed `MotionHandle` control is available to
Rhai and Rust and is scoped to view, component, incarnation and generation;
stale or cross-view use fails explicitly.

Mount, controlled-target change, hover, focus, press, in-view and
scroll-progress use one trigger model. Scroll/geometry sampling is coalesced in
Rust and exposed as typed progress, not continuous Rhai events. Pointer velocity
feeds bounded inertia/decay with clamp, bounce and snap points. Native GPUI
scrolling remains authoritative for ScrollArea.

Only explicit timelines support `on_complete` and `on_cancel`. Sampling freezes
one domain event batch for that frame; only that batch is delivered after the
frame commits through the generation-bound foreground callback path. Events
created by later input wait for a later rendered frame. Retargeting is not
cancellation; stop, unmount and invalid generation are. No per-frame or
per-property Rhai callback exists.

### Lifecycle and retained identity

Enter runs only for a real mount/new incarnation, exit only for removal from a
successful candidate, and ordinary rerender never replays either. Compatible
hot reload preserves progress; compatible parameter changes retarget; explicit
`replay_key` or restart replays.

Removed nodes leave layout, hit testing, focus and accessibility immediately.
An immutable noninteractive paint ghost may finish the exit. It owns no Rhai
state, callbacks, resources or component lifetime. Shared-layout transitions
similarly make the target the sole real node while the source is only a visual
snapshot. IDs are unique inside an explicit MotionGroup and one compatible
window/presentation domain; duplicate source/target identity is an error.

View suspension freezes timelines and resumes without adding elapsed wall
time. Unmount, incarnation replacement and window close cancel scoped motion.
Presentation teardown also clears suspension tombstones and uses segmented
domain ownership, so reusing a window ID cannot inherit a prior pause and
prefix-neighbor windows cannot be reclaimed accidentally.
Business progress remains in state/store/NativeSignal; motion state is a
transient presentation projection.

### Layout, text and vector motion

Layout motion is opt-in. It samples committed old/new geometry and animates a
paint transform without rerunning layout each frame. Continuous window,
splitter, Table-column and other direct-manipulation resize stays immediate;
discrete reorder/expand/selection changes may animate. Hit testing and visual
bounds follow the sampled transform.

RichText spans receive stable keys and typed opacity sources. Grapheme clusters
are never split, while accessibility remains one continuous string. Offset,
color, blur, and richer per-span effects are not generic 0.1.3 properties;
bounded first-party text effects use a typed effect primitive rather than
pretending unavailable GPUI filters are ordinary Style fields.

Canvas and inline compatible paths share measurement, trim/dash, point/tangent
following and compatible-topology morphing. Incompatible morph topology is a
validation error. Morph, trim, stroke, clip and the sampled affine transform
produce one presented Canvas geometry used by painting and hit testing. Affine
motion on commands with axis-aligned path clips is rejected because GPUI 0.2.2
cannot represent the equivalent transformed clip. Path lookup tables and
interpolation stay in Rust.

### Theme, accessibility and quality policy

Themes define semantic motion duration, easing, spring, distance and stagger
tokens. Component motion references roles; literals and application stylesheet
overrides remain possible. Theme hot switching retargets from the current
sample rather than restarting.

The Host resolves system/user preference into the scoped runtime policy
`normal`, `reduced`, or `none`; scripts classify declarations as `decorative`,
`feedback`, or `essential` and cannot override that upper bound. The core
runtime uses deterministic static projections for reduced/none sources and a
recognizable midpoint for indefinite progress. A Host or typed effect may
provide a richer crossfade fallback, but no declaration can bypass `none`.

Host-selected high/medium/low quality tiers are explicit declaration inputs.
Exceeding active-animation, timeline-step, keyframe, ghost, shared-snapshot,
particle or effect budgets rejects the candidate/transaction and retains the
last-good UI; runtime never silently drops or randomizes content.

### Product layers and acceptance

The frozen 51-component `components/*` catalog may adopt restrained functional
micro-motion through public primitives. It never depends on a concrete effect
component. Optional source-owned effects live under `motion/*`; the initial
pack validates text reveal/number ticker, marquee/orbit/particles,
shimmer/border beam, animated Tabs/list reorder and shared-layout cards.
Component Gallery remains the foundation catalog; Motion Gallery is separate.

0.1.3 is delivered only as the complete final system. Internal commits may
follow dependency order but no intermediate public API, compatibility shim or
user acceptance target is retained. macOS is the full real-frame/visual/120 Hz
certification platform; Linux CI certifies portable logic and compilation.
Windows/Linux GPU certification follows hardware availability and does not
permit macOS-only public semantics.

## Consequences

- Motion becomes a generic runtime subsystem instead of component policy.
- Rust Hosts can create and control motion with identical theme, lifecycle,
  reduced-motion, budget, Inspector and automation behavior to Rhai.
- Official effect components cannot acquire private motion, layout or GPU
  privileges.
- Runtime API 1 animation source is deliberately incompatible and must be
  updated through the CLI registry/update flow.
- General ShaderSurface and 3D remain explicit future work instead of partial
  browser-API emulation.

## Rejected alternatives

- Keeping `node.animate` as a compatibility overload creates two ownership and
  replay models.
- Per-frame Rhai callbacks violate the hot-path and foreground budgets.
- `AnimatedButton`/`AnimatedCard` parallel component families duplicate the
  frozen foundation and hide capabilities from application code.
- A universal EffectCanvas or arbitrary shader string weakens schema,
  portability, lifecycle and resource enforcement.
- CPU-generating full RGBA frames for shader-like effects has unacceptable
  upload bandwidth and does not form a scalable GPU abstraction.
- Automatic animation for every layout change makes resize and direct
  manipulation lag behind the pointer and obscures performance ownership.
