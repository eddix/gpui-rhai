# ADR 0021: Native composable Chart Runtime

- Status: Accepted
- Target: 0.1.5
- Date: 2026-09-22

## Context

GPUI Rhai can already express arbitrary retained Canvas geometry and provides a
shared Motion Runtime, but an application should not have to implement axes,
scales, sampling, hit testing, accessibility, and streaming reconciliation in
Rhai for every chart. That approach would also put per-datum work and pointer
hot paths across the Rhai/Rust boundary.

The goal is an original GPUI Rhai visualization system. ECharts and other chart
libraries are functional references only; their option schema, product design,
and compatibility surface are not APIs to reproduce.

## Decision

### Product and package boundary

0.1.5 ships one native composable Chart runtime behind the optional `charts`
Cargo feature in `gpui-rhai`. Source-owned registry components such as
`BarChart` are ergonomic adapters over that runtime. They do not receive
private data, layout, event, theme, motion, or export privileges.

The CLI enables `charts` when chart components are installed. The runtime may
be split into a separate crate in a future release only if dependency weight or
compile time justifies it; 0.1.5 does not expose an artificial crate boundary.

### Scene model

A Chart is one retained visualization scene containing:

- named coordinate regions and explicit series-to-region bindings;
- typed datasets and declarative transforms;
- axes, legends, annotations, interaction state, and semantic projection;
- an aggregated chart motion resource;
- one prepared paint/hit-test/export scene.

One scene may mix compatible series and contain multiple named grids, axes, or
polar/geo regions. The first coordinate systems are Cartesian2D, Polar, and
Geo2D.

The first public series set is Bar/Stacked Bar, Line/Area, Scatter, Pie/Donut,
Heatmap, Map/Choropleth, GeoScatter, GeoLines, Candlestick, Radar, Gauge, and
Funnel. Graph, Tree, Treemap, Sunburst, and Sankey are deferred because they
require distinct layout engines and semantic navigation models. 3D and Globe
remain a separate future feature/version.

### Data and update boundary

Small data may be supplied as Rhai arrays. The native representation is a typed
columnar dataset with named dimensions and null bitmaps. `encode` maps
dimensions to x, y, value, name, size, color, and series-specific channels.

Large or streaming applications use Rust-owned `NativeChartData` handles.
Updates are versioned atomic replace, append, or sliding-window revisions.
When producers outrun the display clock, the runtime coalesces intermediate
revisions and bounds all queues/history.

Built-in native transforms are filter, sort, aggregate, bin, stack, normalize,
moving-window, and downsample. Trusted Hosts may register Rust transforms.
Per-row Rhai transforms and per-datum Rhai render functions are not supported.

Series keys are required. A datum may provide an explicit key or use a unique
category dimension. Only stable datum identity preserves selection, focus, and
update/exit motion. Unkeyed numeric arrays are displayable but update with
whole-series replacement semantics.

The supported performance contract is approximately 10,000 fully interactive
marks and 100,000 Rust-downsampled points on the reference macOS environment.
Downsampling affects drawing only. The transformed semantic dataset, stable
keys, active/selected accessibility projection, and provenance remain intact;
null gaps divide independent draw segments. Categorical series sample against
stable ordinal geometry without replacing their original values or keys.
Semantic aggregation is explicit and inspectable.

### Foreground, background, and Rhai

Rhai evaluation, retained reconciliation, GPUI mutation, and installation of a
prepared chart scene remain on the foreground thread. Typed, owned data and
pure preparation jobs may cross to the background executor. Rhai `Engine`,
`Dynamic`, `FnPtr`, and stored call contexts never cross threads.

Transform and layout candidates, including scale calculation, sampling and
geometry preparation, run as typed Rust background work. Candidate identity is
versioned and stale jobs cannot install. GPUI mutation remains foreground.
Pointer move, wheel, brush, and drag hot paths are native. Rhai receives bounded
semantic events such as selection committed, zoom changed, legend toggled, and
annotation activated.

Each `(region, axis ID)` is compiled once per scene from one contribution set,
including preserved empty schemas. Series, ticks, annotations, custom series,
hit testing, and export consume that same coordinate fact.

### Frame transaction and ownership

Chart state has four distinct stages: requested input, prepared data, frame
candidate, and presented frame. A `DataKey` combines source identity with data
revision. A `FrameKey` also includes a foreground-owned frame epoch advanced by
viewport, bounds, spec, theme, selection, or link changes. Background tasks
carry both a cancellation generation and their content key. Only an active,
matching candidate installed on the foreground executor may replace the
presented frame.

Prepared data is reusable input, never evidence of presentation. Paint, hit
testing, accessibility, and semantic events read one presented scene/key pair;
they may intentionally lag the requested key while a candidate is pending.
Suspend, preview cancellation, resize, and theme changes use the same key
comparison to resume missing work.

The owning ScriptView commits native resume only after every prepare hook and
the script resume transaction succeed. Chart activity time starts at that
commit hook. A failed prepare is compensated without advancing animation time;
failed compensation disposes and unmounts the view instead of advertising a
quiescent tombstone.

### Interaction and linkage

Hover, tooltip, crosshair, and active drag state are native transient state.
Selection, legend visibility, and zoom are controllable state and emit
low-frequency semantic events. Brush supports Cartesian x/y/xy rectangles and
Geo region/rectangle selection; freehand lasso is deferred.

Controlled viewport state has a committed value, transient gesture preview,
and an explicit monotonic Host acknowledgement revision. A redraw is not an
acknowledgement; the Host writes the proposal revision back after accepting,
clamping, or rejecting it. Proposal snapshots and later input generations are
separate, so a delayed acknowledgement cannot consume a newer preview.
Cartesian viewport state compiles into a visible domain window, so axes and
marks always share one mapper. Mark role, structural mark identity, and
`DatumRef` are separate types; business data cannot collide with legend,
annotation, grid, or axis identities.

Charts may join an explicit link group with a declared domain. Crosshair,
zoom, selection, and highlight synchronization stays native and does not route
each pointer event through Rhai.

Viewport link payloads are coordinate-typed. Cartesian payloads bind named
region/axis logical windows. Geo payloads bind map/projection identity and a
normalized camera. Incompatible or unsupported coordinate payloads are not
reinterpreted through another coordinate model. LinkRegistry owns the latest
source/version projection for each group. Targets retain that projection across
suspension, and Cartesian X/Y windows enter the coordinate compiler directly
instead of being reduced to one zoom/pan pair.

Charts are visualization surfaces, not editors. Dragging marks to mutate the
underlying dataset is outside the runtime contract; Hosts can build explicit
editors using the public event and data APIs.

### Theme, motion, locale, and accessibility

Host-level `ThemeTokenOverrides` is a prerequisite. The `charts` namespace
defines typed palette, axis, grid, tooltip, positive/negative, selection, map,
and motion roles. Component-specific structure remains in component styles.

The chart scene owns one aggregated animation resource but uses the existing
Motion Runtime clock, samplers, theme roles, quality, budgets, and reduced
motion policy. Bars interpolate geometry, lines support path progress and
compatible morphing, pie uses angles, and maps interpolate visual values.
Reduced motion simplifies/shortens transitions; `None` immediately commits
terminal state. Suspension freezes the aggregated transition origin and resume
does not consume suspended wall time or restart an unchanged transition. A
chart consumes one declared chart budget rather than one global Motion source
per datum.

Locale affects labels, legends, and tooltip layout. RTL does not implicitly
reverse axes; axis direction is explicit. Time dimensions use typed timestamps
or epochs plus an explicit UTC, fixed-offset, or IANA timezone. Ambiguous date
strings are rejected. Business calendars remain a Host preprocessing concern.

Every chart exposes a bounded semantic summary, keyboard navigation, focused
and selected datum semantics, and a projection of visible/focused data. A Host
may supply a data-table fallback. Meaning must not depend on color alone;
symbols, line styles, hatch/decal, accessible palettes, and high-contrast
tokens are part of the public visual contract.

### Geo boundary

The runtime accepts Host-registered GeoJSON or SVG map sources, provides
Equirectangular and Mercator projection, and owns map hit testing, zoom, pan,
choropleth, GeoScatter, and GeoLines presentation. Trusted Hosts may register
Rust geo sources and projections.

The library bundles no world/country boundary datasets, performs no network
access, and does no address geocoding. Licensing and acquisition of map data
belong to the application. Rhai cannot provide a per-coordinate projection
callback.

### Extensions and export

Trusted Rust Hosts may register custom series that reuse the public data,
coordinate, theme, motion, hit-test, tooltip, and semantic services. Extensions
are compile-time Rust only: no dynamic libraries, downloaded plugins, Wasm
plugins, or arbitrary Rhai `renderItem` equivalent.

Rust Hosts may export a prepared scene to SVG or PNG with explicit size, theme,
locale, data revision/sample, motion policy, and terminal-time choice. Rhai
receives no filesystem authority. PDF, video, animated SVG, and script-owned
export are deferred.

### Failure and responsive policy

Invalid configuration rejects the complete candidate and retains the last-good
scene. Invalid individual data becomes a skipped mark or gap with bounded
diagnostics; it never panics. Ordinary resize is fully native and does not
rerun Rhai. Only an application-level viewport class read creates a script
dependency.

## Consequences

- The runtime carries more native infrastructure than a thin Canvas wrapper,
  but high-frequency behavior and large data no longer depend on Rhai speed.
- Chart source components remain inspectable and replaceable without forking
  scale, hit-test, or streaming correctness.
- Map datasets and advanced graph/hierarchy layouts stay out of the common
  binary and licensing surface.
- One prepared scene becomes the common truth for paint, hit testing,
  accessibility, automation, Inspector, and export, preventing those paths
  from drifting.

## Acceptance

0.1.5 is not complete until Chart Gallery/Studio mounts real frames for every
series type and covers all themes, Host token overrides, viewport sizes,
locale/RTL, light/dark, reduced/none motion, 10k interactive data, 100k
streaming/downsampled data, malformed input, export, and accessibility. Public
intermediate Chart APIs are not released.
