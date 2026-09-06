# GPUI Rhai: Project Intent

## 1. Product definition

**GPUI Rhai is a general desktop UI runtime in which Rhai and trusted Rust build
the same typed declarative UI, a retained Rust reconciler owns identity and
native state, and GPUI performs layout, input, and GPU rendering.**

The product has two deliberately separate layers:

- `gpui-rhai` core/runtime is the language binding and application
  infrastructure. It provides atomic visual nodes, typed style, retained
  reconciliation, events, state, native hot values, focus, scrolling, overlays,
  animation, text, Canvas, accessibility, automation, and Rust extension points.
- The registry is a shadcn-like collection of inspectable Rhai source components,
  themes, locales, and assets copied into the user's repository. Registry
  components have no private layout or interaction privileges; they are built on
  the same public core available to application scripts.

GPUI Rhai is parallel to other GPUI language bindings such as GPUIX, not a
wrapper around them. It is independent of `gpui-component` and must not depend
on it directly or through an optional feature.

## 2. Product promise

Rhai is the primary UI language, not a configuration or styling layer. Using
only public Rhai APIs, an application author can build conventional desktop
applications and custom interaction surfaces involving pointer capture,
dragging, resizing, panning, zooming, animation, exact geometry, custom fonts,
rich visual composition, Canvas scenes, and variable-height data views.

A pure-Rhai Mini Timeline is a hard acceptance application. Rust native paths
raise the performance ceiling and integrate business/platform services; they do
not determine whether an interaction is expressible.

Rust remains first-class because `gpui-rhai` is linked into the Host rather than
hidden behind a fixed precompiled language bridge. One UI may combine:

- pure Rhai components and event handlers;
- Rhai handlers calling registered Rust computation;
- Host-registered native handlers on ordinary atomic nodes;
- Rust-owned native signals and retained custom primitives;
- Host-owned `UiNode` trees using the same reconciler.

The Rust host can remain minimal:

```rust
let view = FileScriptView::new("ui/main.rhai")
    .extension(/* application services and native handlers */)
    .prepare()?;
ScriptApplication::new(view).run()?;
```

## 3. Layering

```text
Application screens and domain UI             Rhai, with optional Rust services
Copied registry components                    editable Rhai source
Public atomic and headless APIs                gpui-rhai core
RetainedUiTree and native runtime state        Rust
Immediate layout, input, paint and platform    GPUI
```

Official components may use generic native mechanisms where platform behavior
requires them: text editing/IME, retained scrolling, focus/accessibility bridges,
deferred overlay drawing, generic virtualization, Canvas scene painting, native
animation, and input routing. Table, DatePicker, Select, Menu, Tabs, Toast
presentation, and similar product components are Rhai/headless compositions,
not privileged native UI nodes.

## 4. Declarative and retained rendering

### 4.1 UiNode boundary

Rhai never owns GPUI `Window`, `App`, `Context`, `Div`, `AnyElement`, closures,
or lifetimes. Rhai and trusted Rust construct runtime-defined `UiNode` snapshots.
The runtime validates them and reconciles them into a retained Rust tree.

The built-in visual atoms are:

- `fragment(children)` for layout-transparent grouping;
- `box(children)` for layout, paint, semantics, and interaction;
- `text` and inline `span`;
- `image` and tintable `svg`;
- declarative retained `canvas(scene)`.

`row`, `column`, and `stack` are convenience constructors over `box`, not
independent node kinds. Scrolling is Box overflow behavior rather than a separate
visual atom.

### 4.2 RetainedUiTree

The runtime holds stable nodes identified by `(parent NodeId, explicit key,
node kind)`. A keyed Rust reconciler atomically applies accepted subtree
snapshots. Keyed reorder preserves NodeId, focus, scrolling, pointer capture,
animation, signals, accessibility identity, and retained primitive Entities.
Changing key or kind unmounts the old node and mounts a new one. Duplicate keys
are hard errors. Interactive, focusable, scrollable, animated, signal-bound, and
stateful nodes require keys; static display-only nodes may use positional
identity.

GPUI elements remain short-lived immediate render values. Retained identity and
state live in the runtime and GPUI Entities, never in a cached `AnyElement`.

### 4.3 Incremental formal components

A formal keyed Rhai component is the only independent script rerender boundary.
Helper functions rerun with their owning component. During render, the runtime
records reads of declared component state, exact store paths, theme/locale
tokens, viewport/geometry, and other host-observable values. A mutation dirties
only dependent components. Reading an entire Map/Array establishes a broad
dependency; exact nested tracking requires explicit path or keyed-collection
accessors.

The runtime retains each component's invocation recipe: module/export/key,
render function and module context, normalized structural props, parent context,
generation, dependency set, and prior subtree. Rhai itself does not provide this
reactivity; it is a gpui-rhai host runtime.

When the root view reruns, formal components bail out before Rhai execution if
their normalized non-node props and render environment are unchanged and no
dirty component lies in the retained subtree. Node-valued props are
conservatively unequal. Bailout retains the complete subtree ownership bundle,
including component state, dependencies, callbacks, effects, timers, signals,
refs, and virtual collections; it is not only a `UiNode` splice.

## 5. Rhai execution and component model

### 5.1 Invocation adapter

Rhai is pinned exactly. All use of `NativeCallContextStore`,
`GlobalRuntimeState`, or other volatile `internals` is isolated behind one
`ScriptInvocationContext` adapter with characterization tests for imports,
closures, callbacks, component rerender, effects, errors, and hot reload.

AST interpretation is the semantic oracle. The runtime has a backend boundary
capable of using Rhai Grain when AST/Grain parity, residual coverage,
diagnostics, and end-to-end performance are proven. Backend selection does not
change Rhai source or the UI API.

Rhai evaluation, component render, ScriptCallback dispatch, reconciliation, and
GPUI mutation remain serialized on the foreground thread. Background work moves
validated `UiValue` or typed Host results, never Engine, Dynamic, FnPtr, or
stored call contexts.

### 5.2 Formal components

Component modules register metadata, schema, and render function once per
generation with `define_component`; exported PascalCase constructors call
`render_component(id, props)`. A formal schema covers:

- typed data props and defaults;
- node/style/callback/slot/ref/signal props;
- local state and defaults;
- emitted semantic events;
- named styleable parts;
- effects;
- assets, dependencies, capabilities, and runtime compatibility.

Unknown props and parts are errors. Component metadata and exported schema must
agree. Module top level may declare components, themes, locales, assets,
functions, and pure constants; it cannot hold mutable UI state or start effects.

### 5.3 Value boundary

Durable data uses schema-checked `UiValue`: null, bool, integer, float, string,
arrays, string-keyed maps, and approved opaque resource handles. Component
invocations also carry a closed `ComponentPropValue` union for generation-scoped
structural values such as UiNode, Style, callback, slot renderer, ElementRef,
and NativeSignal. Arbitrary `Dynamic`, captured closure environments, and custom
Rust variants cannot enter retained state.

Retained UI callbacks and effects use named functions. Curried payloads must
convert to immutable `UiValue`. Anonymous/capturing closures may be used only
inside one synchronous evaluation and cannot escape as event, effect, task, or
subscription handlers.

### 5.4 Lifecycle and effects

`view(ctx) -> UiNode` is required; application `init` and `dispose` remain
optional. Formal components declare effects during pure render by key,
dependency payload, named start function, and named cleanup function. Effects
start only after a successful subtree commit, restart after dependency changes,
and cleanup exactly once on replacement, unmount, or hot reload. Tasks,
subscriptions, and timers bind to component/key/generation ownership.

## 6. State, transactions, and hot reload

Component fields and typed app/window stores are versioned at field/path level.
Equal writes do not invalidate readers. Keyed collection access can invalidate
one item; whole-value reads intentionally create broad dependencies.

Script events and renders use Host-owned transactions. State, store, theme,
locale, queued work, and runtime-managed signal writes commit only after
callback, component render, validation, and reconciliation succeed. Failure
keeps the complete last-good tree. External Rust side effects remain the Host's
responsibility and are not rollbackable.

Hot reload compiles a candidate generation. Failure preserves the complete old
generation. Successful reload retains compatible keyed component state and
retained node/native state, runs old effect cleanup while its invocation context
is valid, invalidates old deliveries, and atomically switches generation.
Incompatible schema or node kinds reset only the affected subtree and produce a
diagnostic.

## 7. Events, handlers, and native hot state

### 7.1 Event contract

Atomic nodes expose normalized pointer, click/aux-click, wheel, hover,
focus/blur, keyboard, layout, scroll, outside-press, and semantic events.
Pointer payloads include pointer identity/type, logical window/local/content
coordinates, movement, buttons, modifiers, click count, timestamp, capture
state, the current handler node's event-time visual bounds, and available
pressure/tilt data. Coordinates account for committed layout, scrolling, and
invertible 2D transforms. Event callbacks can query the same current-target
bounds without creating a render dependency; non-node callbacks return null.

Propagation is capture -> target -> bubble. Each node/phase may have an ordered
list of Script and Host handlers. Responses independently control default
behavior, propagation, immediate propagation, pointer capture, and release.
Continuous move/wheel/layout/scroll events coalesce at most once per frame;
down/up/cancel/focus/submit are never discarded.

### 7.2 Rust handler lanes

`UiEventHandler` supports generation-bound Script handlers and trusted Rust Host
handlers. A Host can also register a named, schema-checked `NativeHandlerRef`
that Rhai attaches to an ordinary atomic node. Native handlers receive a typed
event context, can update signals and declared runtime state transactionally,
and may use `Window`/`App` only inside the trusted Rust closure.

### 7.3 NativeSignal

`NativeSignal<T>` is the only high-frequency imperative value path. It binds to
approved style, transform, scroll, animation, or Canvas properties and repaints
without dirtying a Rhai component. Signal reads do not establish component
dependencies; structural synchronization is explicit. Signals are scoped by
component path/key/type, preserve compatible reconcile/hot reload, serialize
nowhere, and update only through the foreground queue.

## 8. Atomic layout, style, text, and Canvas

### 8.1 Typed Style

Style is an exhaustive-by-default typed mapping of stable, safely representable
GPUI capabilities, not a raw GPUI handle and not a promise of browser CSS
compatibility. It covers flex/grid, wrap/grow/shrink/basis, alignment, gaps,
sizing and intrinsic tracks, spacing, positioning/insets, per-axis overflow,
paint order/stacking contexts, borders/radii, shadow, gradient, alpha, cursor,
hit testing, visibility, typography, selection, clipping, and 2D transforms.

Lengths are property-validated: signed px/rem/percent values are allowed where
meaningful, while sizes/padding/gaps/radii reject negatives. Auto, intrinsic
content, fit, and grid fractions are typed variants. Invalid property/value
combinations fail candidate validation.

There is no general CSS cascade, selector, or specificity. Explicit Style
merge and component part composition determine values. Only documented
typography, direction, and selection properties inherit. Runtime states apply
in a fixed order after base styles; disabled wins over pointer states.

### 8.2 Property sources and animation

Each animatable property has one typed source: literal, signal, transition,
spring, keyframes, or derived signal. Rust samples animation; Rhai never runs
per frame. The engine supports delay/easing, retargeting, repeat/reverse,
enter/exit, layout transitions, shared layout IDs within a compatible Host
layer, reduced-motion policy, and a deterministic test clock.

Exit nodes leave layout, input, focus, and accessibility immediately while a
noninteractive paint ghost finishes visual exit. Layout transitions use
committed old/new geometry and paint transforms rather than rerunning layout
each animation frame.

### 8.3 Colors and themes

One ColorValue grammar serves text, fills, borders, gradients, shadows,
selection, and Canvas. It accepts semantic tokens, typed constructors, and
strict validated CSS-like named/hex/rgb/hsl/hwb/lab/lch/oklab/oklch strings,
then converts to GPUI's drawable color space.

Themes are namespaced typed token registries rather than a fixed struct. Core,
applications, and component packs declare color, spacing, radius, typography,
shadow, motion, and metric token schemas. Theme switching validates a complete
candidate then commits atomically at app/window/subtree scope. Official
components consume public semantic tokens; application atoms may also use
literals.

### 8.4 Text and fonts

Text/Span support declared font assets and fallback stacks, family, weight,
style, size, line height, available font features, wrapping, alignment,
ellipsis, line clamp, inline styling, links, hover/click, selection, and
subtree search/highlight with UTF-16 ranges. Selection is continuous across
ordinary and native document text in paint order. Markdown, Code, and Diff use
the same text, selection, search, theme, and automation infrastructure.

Font files are declared through AssetId/provider metadata, loaded before
render, registered under stable aliases, and hot reloaded without render-time
file I/O. Unsupported variable-font axes are reported rather than simulated.

CodeViewer and DiffViewer are the first read-only consumers of the native
document infrastructure. They accept direct strings or revisioned Host-owned
text snapshots, parse syntax and calculate neutral left/right hunks away from
the Rhai foreground, virtualize visual rows, and keep selection/search/scroll
state native. Diff supports unified and aligned split projections, Unicode
intraline refinement and expandable equal context. Neither surface acquires
filesystem, Git, editing, merge, LSP, or patch-application authority.

### 8.5 Canvas

Canvas is a declarative retained vector scene with keyed rectangles, rounded
rectangles, circles/ellipses, lines, paths, fill/stroke, linear gradients,
clipping, opacity, and 2D transforms. It never calls Rhai during GPUI paint.
Shapes participate in diff, signal/animation binding, path hit testing,
capture/bubble events, accessibility, Inspector, automation, and Host resource
budgets. Applications may alternatively handle one Canvas event using content
coordinates and perform their own hit policy.

## 9. Native behavior mechanisms

### 9.1 ElementRef and geometry

Component-scoped typed ElementRefs bind retained NodeId. They provide focus,
blur, scroll, capture, layout/visual bounds, clipping, and automation identity.
Stale refs fail explicitly. Measurement returns the last committed geometry;
same-layout synchronous feedback is forbidden. Exact geometry dependencies and
layout events coalesce once per frame and are loop-budgeted.

Responsive Style supports named application breakpoints and committed
window/view/container conditions without Rhai execution. Scripts may read exact
committed size when custom geometry requires it.

### 9.2 Focus and accessibility

Public behaviors cover focusability, tab order, autofocus, focus-visible,
scopes, traps, restore, roving focus, and directional/grid navigation. Every
retained node may declare role, name, description, value, checked/selected/
expanded/disabled/invalid state, collection metadata, and semantic actions.
Canvas shapes can be accessibility nodes. The same tree drives platform
accessibility, Inspector, and automation.

### 9.3 Scrolling and overlays

Box overflow creates retained per-axis scrolling. A shared router selects the
deepest eligible scroller per axis, preserves vertical input for parents of
horizontal scrollers, chains residual delta at boundaries, and supports
overscroll containment. Ref commands expose offsets, scrollIntoView, and
signal-owned synchronized panes.

Normal nodes use sibling paint order and local stacking contexts. Deferred
layers use the public Host overlay coordinator for anchors, placement, flip/
shift/snap, nested ownership, priority, outside press, Escape routing, modal
occlusion, focus trap/restore, tooltip timing, and toast queues.

### 9.4 Virtualization and text editing

Generic data-backed one-dimensional virtualization supports stable keys,
variable measured heights, estimates, overdraw, bottom alignment, follow-tail,
prepend anchoring, remeasure, focus retention, programmatic scrolling, and
explicit section-header indices with one retained sticky presentation plus
push-off by the next section. Rhai item renderers are formal components
scheduled outside GPUI layout/paint and only instantiated for the window plus
overdraw. Arbitrary two-dimensional spreadsheet virtualization remains a
separate future mechanism.

A retained native TextEditor mechanism provides controlled and uncontrolled
single/multiline modes, immediate native edits, revision-safe reconciliation,
selection, IME, clipboard, undo/redo, grapheme operations, hit testing,
autoscroll, wrapping, and auto-grow. Input and Textarea are Rhai wrappers;
rich/code editing is a separate editor model.

## 10. Host embedding, capabilities, and custom primitives

The prepared unit remains a single-use `PreparedScriptView`. Existing GPUI hosts
mount multiple independent views into one window through a shared
`ScriptViewHost`. Views isolate Engine, script state, capabilities, tasks,
themes, and diagnostics; the Host shares only native mechanisms that must
coordinate across views, such as overlays, input routing, fonts, and window
policy.

Filesystem, network, persistence, process, credentials, and arbitrary platform
services require explicit versioned Rust capabilities. Module imports use the
restricted ScriptSource resolver; scripts never receive paths, URLs, sockets,
process APIs, or raw GPUI handles.

Custom primitives use a retained lifecycle: mount, validated update, render,
and unmount. Keyed instances preserve their GPUI Entity and native state.
Downstream handlers use the runtime's re-exported GPUI types, WeakEntity in
retained closures, explicit cleanup, and the same event/style/theme/automation
contracts as built-ins.

## 11. Window and platform contract

Standalone applications declare title, initial/min/max size, resizability,
fullscreen, opaque/transparent/blurred background, titlebar behavior,
traffic-light position, visibility/focus, appearance, safe-area/insets, standard
platform menus, and approved runtime commands. Embedded views cannot mutate
Host windows unless explicitly authorized.

The public data model is cross-platform. macOS is the first complete visual,
interaction, IME, automation, and 120 Hz reference platform. Windows and Linux
remain compiling/tested architectural targets with explicit capability results;
the first Core Runtime v2 completion does not wait for their full visual
certification.

## 12. Safety, scheduling, and resource budgets

Scripts are trusted application source inside a restricted capability
environment, not hostile code in a strong sandbox. The runtime still replaces
Rhai's default filesystem resolver, uses Simple optimization, configures
operation/depth/collection limits, rejects arbitrary eval/path loading, and
enforces Host budgets at every retained boundary. Rhai limits are defense in
depth; they cannot interrupt blocking Rust and do not cover every collection
mutation path.

The Host budgets UiValue size/depth, retained nodes, styles, handlers, signals,
effects, tasks, Canvas commands, layers, virtual data/overdraw, assets, dirty
components, and diff work. Soft violations produce source-scoped diagnostics;
hard violations reject the candidate and preserve last-good state.

At 120 Hz, script plus reconcile has a 4 ms p95 reference budget, leaving the
rest of the 8.33 ms frame for GPUI layout/paint. Runtime frame scheduling
accumulates all script work independently of per-call Rhai counters. Atomic
commits never split across frames; coalescible input can defer, while mandatory
events remain ordered.

## 13. Tooling, diagnostics, and automation

The runtime emits definitions for atoms, Style, events, extensions,
capabilities, components, parts, and theme tokens. `gpui-rhai check` combines
compile/strict checks, metadata/schema/asset validation, known-call AST lint,
and real headless view evaluation because Rhai compilation alone cannot prove
function availability. LSP/editor tooling consumes the same definitions.

Inspector shows retained/component trees, NodeId/key/source, props, style
provenance, dependencies, state, signals, effects, focus/scroll/capture,
accessibility, recent event paths, dirty causes, animation sources, and frame
timings.

Public automation provides role/name/text/test-ID locators, input, drag/wheel,
bounds/style/text/selection/scroll/accessibility queries, deterministic clocks,
and GPU screenshots through production render/input paths. It has an in-process
Rust API and an explicitly enabled language-neutral stdio/JSON protocol for
standalone applications.

## 14. Source components and distribution

The CLI copies versioned component/theme/locale/asset source plus local pristine
baselines into the application. Diff/update is offline, reproducible,
inspectable, and three-way mergeable. Development uses file-backed sources;
production embeds the same validated graph and assets.

The registry includes the existing component product line—Button, Input,
Textarea, Checkbox, Radio/RadioGroup, Switch, Label, Tag, Avatar, Icon, Divider,
Progress, Skeleton, Combobox, Select, DatePicker, Popover, Tooltip, Dialog,
Accordion, Collapsible, Tabs, Menu, Toast, FormField, Table, and Pagination—but
all compositional implementations migrate to public Core Runtime v2 mechanisms.

Dogfooding permits destructive migration. Until an explicit release/versioning
signal, crate/component version remains `0.1.0`, `RUNTIME_API_VERSION` remains 1,
and no compatibility adapter, deprecated alias, dual parser, old component
format, or SDK-managed downstream migration is retained.

## 15. Completion and acceptance

The maintainer reviews one final delivery, not intermediate product states. The
delivery is complete only when it includes:

- the full retained/incremental Core Runtime v2 and public atomic surface;
- every official source component migrated and unnecessary native UI nodes
  deleted;
- old Style/atomic/component APIs removed without shims;
- an artistic typography/translucent-card/animated-Tabs showcase;
- a pure-Rhai Mini Timeline meeting the reference interaction contract;
- a variable-height Chat/List showcase;
- a Rust native fast-path implementation of the same interaction surface;
- public automation, Inspector, definitions/check tooling, and final docs;
- hot-reload, rollback, accessibility, interaction, visual, and performance
  matrices passing on the macOS reference platform.

Internal commits and tests may proceed in dependency order, but no temporary
public API or intermediate user acceptance target is allowed.

## 16. Explicit non-goals

Core Runtime v2 does not:

- expose raw GPUI values or arbitrary retained-node mutation to Rhai;
- execute hostile code as a process/memory-secure sandbox;
- pretend to implement browser CSS/DOM semantics;
- fake GPUI capabilities such as per-element backdrop blur or unsupported
  variable-font axes;
- include arbitrary two-dimensional spreadsheet virtualization or a rich/code
  editor product;
- require simultaneous full visual certification on every desktop platform;
- depend on GPUIX, `gpui-component`, another script engine, or a Web runtime.

## 17. Repository and licensing

The repository remains a Cargo workspace containing the runtime, CLI, copied
registry, examples, docs, and tests. Project Rust and Rhai source is licensed
under `MIT OR Apache-2.0`; themes, palettes, fonts, icons, and adapted sources
carry their own reviewed attribution. English is normative for source,
schemas, diagnostics, and project documentation; translations may supplement it.
