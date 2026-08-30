# Component authoring guide

An official or application component is a Rhai module with three parts:

1. a structured `/* gpui-rhai ... */` JSON header;
2. a `define_component` schema and render declaration;
3. a PascalCase constructor accepting one props map and a named
   `render_Name(ctx, props)` function.

Use `registry/components/label.rhai` as the smallest complete reference.

## Metadata

The header declares component ID, source version, runtime API range,
dependencies, capabilities, and component-owned asset paths. The same metadata appears in
`define_component`; `gpui-rhai check` rejects disagreement.

Asset paths are provider-relative files under `ui/assets`, such as
`icons/chevron_next.svg`. Component source addresses an installed asset through
the application namespace, for example `asset("app/icons/chevron_next")`. The
CLI copies declared assets and records their pristine baselines with the source.

Module IDs are lowercase logical paths such as `components/form_field`.
Components are imported under an explicit alias:

```rhai
import "components/label" as label;
label::Label(#{ text: "Project" })
```

## Schema

Declare every prop, local state field, semantic event, slot, styleable part,
and effect key.
Unknown props are errors. Defaults must satisfy their own schemas. Every event
named `change` requires an optional callback prop named `on_change`.

Use inclusive/exclusive numeric bounds, `one_of`, `Length`, and
UiValue-convertible schemas when a
public contract needs them. Do not replace a precise union with an unrestricted
map or defer every constraint to native construction. Component-specific tagged
maps, such as Table column widths, still receive semantic validation after their
outer schema succeeds.

Every formal component automatically receives optional `key`, `style: Style`
and `part_styles: map<Style>` props. Route the PascalCase constructor through
the runtime wrapper, then validate normalized props in its render function:

```rhai
fn Counter(props) { render_component("components/counter", props) }
fn render_Counter(ctx, props) {
    text(`${ctx.get_state("count")}`)
}
```

The wrapper validates and defaults props, derives the parent/key instance path, mounts declared local state,
scopes callbacks to the component module, rejects duplicate stateful keys, and
cleans unreachable instances only after a successful render. Merge overrides with
`component_style(props, "part_name", base_style)`; the root merge order remains
base, size, variant/state, caller `style`, then caller root `part_styles`.
Named `virtual_collection` item renderers do not retain the original Dynamic
props. They resolve the same validated Style-only snapshot with
`ctx.component_style("part_name", base_style)`, which cannot expose nodes,
callbacks, or arbitrary Dynamic values to deferred rendering.

Stateful components and lifecycle custom primitives require a stable caller
`key`. Never store UI state in mutable script globals.

## Effects

Effects are declarations made during a formal component's pure render. Declare
their stable snake_case keys in the component schema, then provide a
UiValue-convertible dependency payload and named start/cleanup functions:

```rhai
// schema: #{ ..., effects: ["subscription"] }
fn start_subscription(ctx, dependency) { /* start owned work */ }
fn cleanup_subscription(ctx, dependency) { /* release owned work */ }

fn render_Stream(ctx, props) {
    effect(
        "subscription",
        #{ channel: props.channel },
        Fn("start_subscription"),
        Fn("cleanup_subscription")
    );
    text(props.channel)
}
```

Start runs only after the candidate subtree reconciles. A changed dependency or
script generation runs the old cleanup in its original module context before
starting the replacement. Removal, disposal, and successful hot reload clean up
exactly once. Anonymous/capturing closures and non-UiValue dependencies are
rejected. Failed start/cleanup restores runtime state and keeps the last-good
tree; external Rust side effects remain outside rollback and therefore need
idempotent Host design.

## Native signals

Declare a component-local hot value during render and bind it only to an
approved property. Event handlers address the signal by local key, so no Rhai
closure or runtime reference is retained:

```rhai
fn advance(ctx, payload) {
    ctx.set_signal("progress", ctx.get_signal("progress") + 0.1);
}

fn render_Meter(ctx, props) {
    let progress = signal("progress", 0.0);
    text("meter")
        .bind_signal("opacity", progress)
        .on_click(Fn("advance"))
}
```

Signal identity is component path + key + value type. Compatible rerenders and
hot reload preserve the current value; unmount makes old handles explicitly
stale. Signal reads do not establish component dependencies and writes do not
rerun Rhai. Trusted Rust can update a mounted handle through
`ScriptViewHandle::write_signal` on the GPUI foreground thread.

## Styling

Use semantic theme tokens and the typed `Style` builder. Merge order is:

```text
base -> size -> variant/state -> caller style/part_styles
```

Use typed `linear_gradient(#{...})` and `shadow(#{...})` values for component
paint; use Style grid/flex, opacity, cursor, typography, ellipsis/clamp, and
translation methods only where their documented GPUI mapping applies. See
`docs/style.md` for the supported surface and explicit remaining gaps.

Use only `xs`, `sm`, `md`, and `lg` for component sizes. Preserve the desktop
default cursor for controls. Attach normalized accessibility role/label data to
the root node. Use `padding_start/end` and `margin_start/end` for asymmetric
inline spacing so caller locale direction remains correct.

Application/theme families may declare typed namespaced tokens. Components
resolve colors with `theme_color("charts.series_a")`; unknown or wrong-typed
paths do not fall back to arbitrary strings. Namespaced lengths/numbers/strings
are retained for the corresponding final typed Style/data APIs.

Use `box(children)` for layout/paint/interaction and `fragment(children)` only
for transparent snapshot grouping. Fragment cannot carry Style, handlers,
signals, attributes, animations, or refs. `row`, `column`, and `stack` are Box
helpers, not distinct privileged node kinds.

Use `text([span("Label ").bold(), span(value).color(theme_color("accent"))])`
for inline runs. Span is an immutable inline value, not a child node; current
run refinements are color, bold, and italic and render through one GPUI
`StyledText`.

Use `canvas(canvas_scene([...]))` for retained vector drawing. Commands currently
include rect/circle/line plus typed fill/stroke paths with quadratic/cubic
segments, gradient paint, uniform transform, and axis-aligned clip; every
command needs a stable unique key and finite logical geometry. Scene
construction and Host budgets count path segments before commit, while GPUI
paint consumes only the accepted Rust
scene.

## Events

Bind callbacks to nodes rather than calling them during rendering:

```rhai
node.on_click(props.on_click)
```

Handlers receive `(ctx, payload)`. They may update declared state, dispatch an
action, emit an event, or call a manifest-declared capability. They may not
access GPUI contexts. Pointer callbacks are handled by default; return
`propagate()` to allow the normalized event to continue to an ancestor handler.

`on(event, handler)`, `on_capture(event, handler)`, and
`on_bubble(event, handler)` append ordered handlers; they do not replace a prior
binding. A handler may return `event_response()` refined with
`prevent_default()`, `stop()`, `stop_immediate()`, `capture_pointer()`, or
`release_pointer()`. Pointer and wheel handlers receive normalized maps rather
than GPUI event values.

After committed prepaint, pointer maps use node-local coordinates derived from
the retained geometry registry. Canvas pointer maps additionally include
`canvas_key`, the topmost retained command hit or `()`; captured move/up events
retain that production routing path.

Trusted Hosts can register a `NativeHandlerDescriptor` and Rust closure, then
Rhai resolves it with `native_handler("namespace.name")` and attaches it through
the same `on` methods. The descriptor limits accepted event names and validates
payloads before Rust runs. Native and Script handlers share transaction and
response semantics.

Formal components may declare `element_ref("name")` during render and attach it
to a stable keyed node with `with_ref`. `ctx.element_bounds(ref)` returns null
before first committed prepaint and then the last committed layout/visual
geometry; geometry changes dirty only components that read that exact ref.
Event handlers can call `ctx.focus(ref)` or `ctx.focus("local_ref_key")`; focus
is executed as a window-scoped retained command after the transaction commits.
Scrollable keyed ref nodes use `Style().overflow_x_scroll()`,
`overflow_y_scroll()`, or `overflow_scroll()`. Handlers call
`ctx.scroll_to(ref, x, y)` with finite non-negative visible offsets.
For a retained descendant, `ctx.scroll_into_view(ref)` (or its component-local
ref key) uses a GPUI `ScrollAnchor` tied to the nearest retained scrollable
ancestor and applies the minimal native reveal on the next frame.

Declare one-shot foreground callbacks during formal render with
`timeout(key, delay_ms, paused, Fn("callback"), payload)`. Keys are local to the
component and signatures reconcile transactionally: unchanged declarations
keep deadlines, changed declarations restart, completed declarations do not
repeat until removed or changed, and unreachable scopes cancel. Event handlers
may call `ctx.pause_timeout(key)`, `ctx.resume_timeout(key)`, or
`ctx.cancel_timeout(key)`. Only named non-capturing callbacks and UiValue
payloads cross the retained boundary.

Use `layer(content, #{ id, placement, inset?, priority? })` for arbitrary
window-level content. Placements are the four corners, `center`, and `fill`;
IDs are namespaced by embedded `view_id`. Layer is a generic portal primitive,
not an authorization mechanism or a replacement for modal Overlay policy.

`on_hover_change(callback)` emits a boolean transition. The
`on_hover_value(callback, value)` variant emits
`#{ hovered: bool, value: UiValue }`, which lets source components pause keyed
timers without retaining closures.

For structural responsive composition, branch only on
`ctx.viewport_class()` (`compact`, `regular`, or `wide`). Hosts may replace the
default 600/1000 logical-pixel boundaries with a validated
`ViewportBreakpoints`; avoid continuous script-side pixel calculations.

## Required tests

Each component needs logic tests, normalized node snapshots, representative
macOS screenshots, keyboard/focus tests, accessibility assertions, and an
example. Its file header must contain purpose, props, events, statefulness, and a
short usage example.

`gpui-rhai check` resolves the copied module graph, loads the real
theme/locale/assets, mounts the application state schema, and executes the
initial `view(ctx)` through ScriptLifecycle and retained reconciliation.
Render-time errors, duplicate keys, component-state failures, effects/timers,
and retained budget violations therefore fail headless CI rather than waiting
for a native window.
