# Embedding multiple script views

GPUI Rhai is view-first. A host may mount several independent Rhai views into
one existing GPUI window without transferring ownership of `Application`, the
window, or the surrounding Rust layout.

## Ownership model

```text
GPUI Window
└─ ScriptViewHost (one interaction and overlay domain)
   ├─ ScriptViewHandle A -> independent Engine/Runtime/Lifecycle
   ├─ ScriptViewHandle B -> independent Engine/Runtime/Lifecycle
   └─ ScriptViewHandle C -> independent Engine/Runtime/Lifecycle
```

Views never share Rhai state, app/window stores, themes, locales, tasks,
subscriptions, assets, capabilities, or diagnostics. Share service behavior
through explicit host capabilities and large immutable row sets through
registered `NativeCollection` values. The Host shares native interaction
mechanisms that must coordinate across components: Overlay, Tooltip, Layer,
outside dismissal, Escape routing, focus fallback, and approved key bindings.

## Mounting

Install App-level input actions once, create one Host for the window, then mount
single-use prepared views:

```rust
gpui_rhai::install(cx);
let host = gpui_rhai::ScriptViewHost::new("main-window", cx)?;

let first = gpui_rhai::FileScriptView::new("plugins/first/ui/main.rhai")
    .prepare()?
    .mount(
        gpui_rhai::ScriptViewConfig::new("first-widget"),
        host.clone(),
        window,
        cx,
    )?;
```

For deterministic tests, inject one monotonic clock before `prepare`:

```rust
let manual_clock = gpui_rhai::ManualRuntimeClock::new(std::time::Instant::now());
let prepared = gpui_rhai::FileScriptView::new("plugins/first/ui/main.rhai")
    .runtime_clock(manual_clock.clock())
    .prepare()?;

manual_clock.advance(std::time::Duration::from_millis(16));
```

The injected clock drives both animation sampling and declarative timer
deadlines. Civil-date behavior remains independently controlled by
`calendar_clock`.

`PreparedScriptView::mount` consumes the prepared value. Prepare again for a
second instance, even when it uses the same source. This preserves handler and
runtime isolation.

The Rust root must wrap the complete layout containing all sibling views:

```rust
host.container(
    row()
        .child(first.flex_item()?)
        .child(second.flex_item()?)
        .child(third.flex_item()?)
)
```

The container is layout-transparent. It initializes the shared Host once per
frame and installs native capture routing. Rendering a handle outside its Host
container produces a visible diagnostic.

Use `flex_item()` when the view is a direct child of a Rust flex row or column.
It supplies a zero flex basis and `min-width/min-height: 0`, so wide scrollable
script content cannot become the host item's automatic min-content size. Use
the lower-level `element()` for fixed, absolute, grid, or manually styled
placements.

Several Hosts may intentionally coexist in one window. Capture routing is
limited to each Host container, so their Overlay domains do not dismiss one
another. Use one shared Host whenever sibling widgets should coordinate.

## Host-owned chrome and the active theme

Each mounted view exposes its own read-only `ThemeHandle`. Its snapshot is the
root ScriptView's effective family/variant after app, window, subtree, and
system-appearance selection. It contains the complete color, spacing, radius,
typography, and namespaced token tables; reading it never invokes Rhai.

```rust
let theme = view.theme()?;
let current = theme.snapshot(cx);
let _surface = current.variant.tokens.colors["surface"];
let _body = &current.variant.tokens.typography.roles["body"];
```

Host entities should retain an observation subscription rather than polling on
every frame:

```rust
let subscription = theme.observe_in(cx, |host_view, snapshot, cx| {
    host_view.script_theme = snapshot;
    cx.notify();
});
```

`ThemeSnapshot::revision` advances only when the effective variant changes,
including a system light/dark transition. Sibling ScriptViews remain isolated
and may expose different snapshots. Local theme overrides below the root are
intentionally not projected onto adjacent Host UI because one view may contain
several differently themed subtrees.

## Bounds and overlays

`ScriptViewHandle::element()` automatically measures the bounds allocated by
the host layout. The measured width drives the view's own
`compact`/`regular`/`wide` class. Crossing a breakpoint schedules one additional
view render; ordinary pixel resizing remains native GPUI layout.

The Overlay viewport is separate. It defaults to the complete GPUI window, so a
Combobox in a 200-point widget can render at its measured 280-point width and
flip/clamp against window edges. `ScriptViewHost::set_overlay_viewport` may set
an explicit absolute rectangle for an intentionally isolated domain.

Every local Overlay, Tooltip, Layer, and focus ID is internally namespaced by
`view_id`. Rhai callbacks continue to receive their original local IDs. Toast
items remain owned and limited by their source component; only their generic
positioned Layer elements share the Host portal.

Non-modal outside clicks dismiss the topmost Host overlay during native capture
and continue to the clicked sibling control. Modal backdrops consume the click.
An open modal also reclaims focus when Host code moves it to an ancestor, so
Escape and Tab stay in the modal path. Do not refocus a Host root after each
embedded render; attach global shortcuts to the Host interaction domain and let
Overlay own focus while a modal is presented.

## Event-time geometry bridges

Do not stream host resize measurements into a store merely so one later click
can position native UI. Every retained node event carries an immutable snapshot
of the current handler node's committed visual bounds in window coordinates:

- Rhai handlers read `ctx.event_target_bounds()`;
- raw pointer/wheel maps also expose `payload.target`;
- registered Rust handlers read `NativeEvent::target`.

The shape is `{ x, y, width, height } | ()`. It uses `currentTarget` semantics:
capture and bubble handlers see the bounds of the node that owns the currently
running handler, not an inferred application-level card. Click and custom
component payloads keep their declared schema; their geometry remains separate
in the callback context or `NativeEvent`.

This snapshot is event-only and untracked, so reading it cannot dirty a
component. By contrast, `ctx.element_bounds(ref)` reads tracked last-committed
`layout`, `visual`, and `clip` geometry for a retained `ElementRef`. Use that API
only when a render truly depends on another element's previous committed
geometry; it cannot create same-layout synchronous feedback.

## Identity and window commands

`ctx.window_id()` identifies the actual host window and is shared by sibling
views. `ctx.view_id()` identifies the mounted widget and is unique inside the
Host.

Embedded views reject `open_window`, `focus_window`, `close_window`, and close
handler registration immediately with `UnsupportedInEmbeddedView`. The
standalone `ScriptApplication` adapter enables the existing restricted
multi-window implementation.

Trusted standalone hosts may customize the primary native window while keeping
that authority out of Rhai:

```rust
ScriptApplication::new(prepared)
    .window_options(|mut options, _cx| {
        options.titlebar = None;
        options
    })
    .run()?;
```

The callback receives the normal centered defaults, so it can change only the
policies it owns or replace the options entirely.

## Key bindings

Mounting never modifies the App keymap. Inspect `PreparedScriptView::key_bindings`
and explicitly approve them:

```rust
host.bind_keys(prepared.key_bindings().iter().cloned(), cx)?;
```

The operation is idempotent for identical bindings and rejects conflicting
actions for the same keystrokes/context.

## Disposal

Remove a configured widget with `view.dispose(cx)?` before dropping it. Disposal
is idempotent and immediately cancels tasks/subscriptions, removes scoped state,
actions, overlays, tooltips, layers, and declarative timers, and frees the
`view_id` for remounting.
Dropping the final handle is a fallback. Merely omitting `view.element()` from a
temporary page does not dispose it.

## Last-good trees and host-visible failures

Rendering is transactional. When a callback, delivery, or rerender fails,
`ScriptViewHandle::root` continues to return the last successfully committed
tree so the host never observes a partial candidate. Embedding hosts must pair
that snapshot with `ScriptViewHandle::last_error`: a non-`None` error means the
tree is last-good fallback state rather than the result of the latest attempted
update. Successful script work clears the error; native-only repaint and
animation frames do not hide it.

The built-in banner is monospace and selectable through the normal Host copy
action. An application with its own error panel may opt out per mounted view:

```rust
let config = ScriptViewConfig::new("user-view").show_error_banner(false);
```

The opt-out changes presentation only; `last_error` continues to report the
failure and the last-good tree remains mounted.

See `cargo run -p gpui-rhai --example embedded_views` for three isolated views,
automatic compact sizing, escaping Combobox placement, duplicate local IDs,
cross-view dismissal, shared Layer placement, and explicit dispose/remount.

## Host-owned interactive trees

Embedding a script view is not required when the Host already owns the complete
UI tree. Build `UiNode` values in Rust, attach `HostCallback` event closures, and
apply controlled frames with `StaticUiView::set_root` or render through
`GpuiNodeRenderer` directly.

Host callbacks run synchronously on the GPUI foreground thread, receive the
normalized `UiValue` payload plus `Window` and `App`, and return
`EventPropagation`. They do not create a RuntimeEngine, lifecycle, capability,
subscription, automatic trace, or frame scheduler. Send work to Host channels
and capture `WeakEntity` rather than a strong reference to the Entity owning the
tree.

Unlike `NativeHandlerRef`, `HostCallback` has no `NativeEvent` wrapper or
script `UiContext`. Raw pointer/wheel geometry is available in the normalized
payload; semantic click payloads remain exactly what the Host placed on the
node.

Callback-typed custom primitive props accept the same `UiEventHandler`, so a
Host-built TextInput does not need a Rhai adapter function. See
`cargo run --release -p gpui-rhai --example host_owned_tree` for the complete
Rust frame → UiNode → Host event → worker → `set_root` cycle.
