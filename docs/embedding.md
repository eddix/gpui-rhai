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

## GPUI package identity

gpui-rhai 0.4.2 uses the exact `gpui-pre 0.3.11` core/platform family while
preserving the Rust use names `gpui` and `gpui_platform`. Prefer the public
re-exports so every Host type is guaranteed to match the runtime:

```rust
use gpui_rhai::gpui::{App, AppContext, Context, Render, Window};

gpui_rhai::gpui_platform::application().run(|cx: &mut App| {
    gpui_rhai::install(cx);
    // Open the Host window and mount script views.
});
```

Hosts that need direct dependencies must use the same package identity and
exact family version:

```toml
[dependencies]
gpui-rhai = "0.4.2"
gpui = { package = "gpui-pre", version = "=0.3.11", default-features = false, features = ["font-kit"] }
gpui_platform = { package = "gpui-pre-platform", version = "=0.3.11", default-features = false, features = ["font-kit", "runtime_shaders", "wayland", "x11"] }
```

Official `gpui 0.2.2` and `gpui-pre` expose similarly named but incompatible
Rust types. `gpui-rhai check` rejects a project manifest that mixes them; it
reports the required declaration but never rewrites `Cargo.toml` automatically.

### The gpui-fast backend

gpui-rhai also builds on [gpui-fast](https://github.com/longbridge/gpui-fast)
0.1.13, for an application that links another gpui-fast UI into the same
binary (nsh's `nsh-platform-gpui` 0.4 builds on that version). Turn off the
default `gpui-pre` feature and reach GPUI through the re-exports, which then
name gpui-fast's crates:

```toml
[dependencies]
gpui-rhai = { version = "0.4", default-features = false, features = ["gpui-fast"] }
```

`gpui-rhai init --backend fast` writes this dependency. The two forks cannot
share a binary: both register the same Objective-C classes when they load, so
a process that links both aborts at launch, before `main`. `gpui-rhai check`
rejects a direct dependency on the fork the application does not build on and
a gpui-fast version other than 0.1.13 (a binary holds one), and warns when the
workspace's `Cargo.lock` resolves both forks; separate binaries may each link
one.

On gpui-fast:

- **Cache islands are off.** gpui-fast replays a cached view only while no
  entity it read was notified, and an island reads its host, which every
  commit notifies.
- **The runtime's views render with every host frame**, as on gpui-pre:
  gpui-fast's view retention would skip their prepaint, which fills the
  window's overlay, interaction and geometry registries each frame.
- **Scroll layers must be off.** gpui-fast composites the rows of a scrolled
  list from a layer without rendering them, so virtual collections realize the
  wrong rows. Start the process with `GPUI_SCROLL_LAYERS=0`; gpui-fast reads it
  when it creates a window, and Rust 2024 makes `std::env::set_var` unsafe, so
  set it before the process starts: `LSEnvironment` in a macOS bundle's
  `Info.plist`, the launcher that starts the binary, or `[env]` in
  `.cargo/config.toml` for `cargo run`. The variable applies to every window of
  the process, so an application that runs another gpui-fast UI in the same
  process loses its scroll layers too.

## Mounting

A view's source is a `ScriptView`: `gpui_rhai::script_view!()` for the crate's
own `ui/` (files in a debug build, the tree `build.rs` embedded in a release
build), `ScriptView::files(path)` for a directory or entry script elsewhere,
such as a plugin, or `ScriptView::sources(..)` for sources the Host holds in
memory ([production](hot-reload-and-production.md#production)).

Install App-level input actions once, create one Host for the window, then mount
single-use prepared views:

```rust
gpui_rhai::install(cx);
let host = gpui_rhai::ScriptViewHost::new("main-window", cx)?;

let first = gpui_rhai::ScriptView::files("plugins/first/ui/main.rhai")
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
let prepared = gpui_rhai::ScriptView::files("plugins/first/ui/main.rhai")
    .runtime_clock(manual_clock.clock())
    .prepare()?;

manual_clock.advance(std::time::Duration::from_millis(16));
```

The injected clock drives both animation sampling and declarative timer
deadlines. Civil-date behavior remains independently controlled by
`calendar_clock` when explicitly injected. `wall_clock(WallClock::fixed(ms))`
pins presentation instants separately: `ctx.now()` and the default `ctx.today()`
derive from it. An explicit calendar clock remains the date-only override;
timezone-aware temporal controls derive their calendar date from the wall
instant and selected timezone. See [temporal controls](temporal.md).

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

A frame replays the parts of a view that did not change instead of building
them again ([frame replay](performance.md#frame-replay)). When the Host gives a
view a width its content cannot change, as when the view fills the window or a
pane the Host sizes, say so with `ScriptViewConfig::host_sets_width(true)`:
subtrees whose width follows the view's are then replayed too. The Host may
resize such a pane without resizing the window, as a split does; the frame
that sees the new width lays the replayed subtrees out at it. A view the Host
lays out by its content keeps the default. `render_islands(false)` turns
replay off for one view, and `GPUI_RHAI_RENDER_ISLANDS=off` for every view.

Focus frames show after a navigation key (Tab, an arrow, Home, End, Page Up,
Page Down, F6) and hide at the next pointer press, so a click shows none and a
shortcut with Command, Control or Alt brings none back. A Host whose users move
focus by keyboard throughout can keep them visible with
`ScriptViewConfig::focus_indicators(FocusIndicators::Always)`; one driven by
shortcuts can turn them off with `FocusIndicators::Never`. A focused text field
keeps its frame in every mode.

Use `flex_item()` when the view is a direct child of a Rust flex row or column.
It supplies a zero flex basis and `min-width/min-height: 0`, so wide scrollable
script content cannot become the host item's automatic min-content size. Use
the lower-level `element()` for fixed, absolute, grid, or manually styled
placements.

Several Hosts may intentionally coexist in one window. Capture routing is
limited to each Host container, so their Overlay domains do not dismiss one
another. Use one shared Host whenever sibling widgets should coordinate.
Within one Host, every mounted View retains its own pointer-capture route;
painting or suspending a sibling cannot overwrite the actual capture owner.

Suspension quiesces focus, overlays, gestures and native previews before the
API returns, without scheduling a render of the now-suspended child. The Host
must stop requesting `element()` until `resume` succeeds; calling `element()`
while suspended remains an explicit `SuspendedView` error rather than silently
resuming or painting stale UI.

## Rhai shells around Host-owned content

`HostSlotRegistry` lets a script lay out an opaque element or independently
mounted view without exposing GPUI objects or nested-view lifecycle to Rhai.
Register slots before preparing the outer script view:

```rust
let slots = gpui_rhai::HostSlotRegistry::new()
    .with_script_view("content", resident_view.clone())?;

let shell = gpui_rhai::ScriptView::sources(entry, sources, theme)
    .extension(slots)
    .prepare()?;
```

Use a small script helper to make the stable primitive key explicit:

```rhai
fn host_slot(name) {
    gpui_rhai::HostSlot(#{ key: name, name: name })
}

column([
    header_bar(),
    host_slot("content").with_style(
        style().flex_grow().min_height(px(0))
    ),
])
```

The slot is a clipped normal layout box. Pointer, wheel, and click events stop
at its native boundary after the Host content handles them; no Rhai pointer
event or semantic subtree is synthesized for the opaque content. Keyboard
events follow the focused resident path and are deliberately not marked
handled at the boundary—doing so would prevent the macOS IME from delivering
text insertion. The Host must still suspend or dispose a slotted
`ScriptViewHandle` explicitly. An unknown slot or disposed nested view becomes
a normal custom-primitive diagnostic instead of a partial script transaction.

`with_script_view` detects whether the resident view's Host frame is already
active. It reuses a shared Host without nesting the domain, and otherwise wraps
the resident element in its own `ScriptViewHost::container`. Shell and resident
may therefore belong to different overlay/focus domains in the same GPUI
window without transferring lifecycle authority.

Run the source-backed acceptance story with:

```sh
gpui-rhai gallery --story apps/host-embedding
```

Gallery mounts the resident form under a distinct Host and treats parent plus
resident as one cache lifecycle group. The native regression inserts Chinese
text through the real slot boundary.

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
`view_id` and by the component instance that declared it, so two instances of
one component may use the same overlay key. Rhai callbacks continue to receive
their original local IDs. An overlay's `parent` (`parent_overlay`) names the
nearest enclosing overlay with that key, so a submenu or a Combobox inside a
Popover finds its parent even when another component declared it. Toast items
remain owned and limited by their source component; only their generic
positioned Layer elements share the Host portal. The portal paints layer content
outside the view's root, but the content keeps the view's keyboard handling:
Tab and Shift+Tab move focus, the actions bound with `bind_keys` and Copy run in
the view that declared the layer, and Escape still reaches the Host's overlays.

`ScriptViewHost::overlay_placement(view_id, key)` returns the placement of the
overlay a view rendered with that key in the last frame. When several instances
declare the key it returns `OverlayLookupError::Ambiguous` with their instance
paths; `overlay_placement_in(view_id, instance_path, key)` names one
(`/View[main]/Filter[eu]/Select[region]/Combobox[region-combobox]`).

Non-modal outside clicks dismiss the topmost Host overlay during native capture
and continue to the clicked sibling control. Modal backdrops consume the click.
An open modal also reclaims focus when Host code moves it to an ancestor, so
Escape and Tab stay in the modal path. With several modals open, in one view or
in several, only the topmost reclaims focus: the last one in the Host's overlay
stack, which is also the one Escape closes first. Do not refocus a Host root
after each embedded render; attach global shortcuts to the Host interaction
domain and let Overlay own focus while a modal is presented.

A Host that needs to know whether a script overlay is open asks
`ScriptViewHost::open_overlays()`. It lists the overlays open in every view as
of the last frame, bottom to top, including those declared inside components
(a Select's `<key>-combobox`, a Dialog in a third-party widget). Each
`OpenOverlay` carries the `view_id`, the declaring instance path in `owner`
(`None` when the view declared it outside any component), the script `key`,
the `kind`, and `modal` and `dismiss_on_escape`. Gate Host shortcuts on it, for
example skip panel switching while any entry is `modal`:

```rust
let modal_open = host.open_overlays().iter().any(|overlay| overlay.modal);
```

The container's own Escape handler only runs when the key reaches it. A Host
that binds Escape at App level (`cx.bind_keys` + `cx.on_action`) gets the action
before any key-down listener, so it offers the key to the script overlays first
with `ScriptViewHost::dismiss_escape(window, cx)`. That closes the topmost
overlay that Escape closes, as the container does, and returns `false` when no
script overlay took the key:

```rust
cx.on_action(move |_: &HostEscape, cx| {
    let taken = window_handle
        .update(cx, |_, window, cx| host.dismiss_escape(window, cx))
        .unwrap_or(false);
    if !taken {
        close_call_log(cx);
    }
});
```

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
`layout`, `visual`, and `clip` geometry for a retained `ElementRef`. An unresolved
first-render read returns null, follows the ref through commit, and self-heals
after first prepaint. The logical subscription follows appearance, NodeId
replacement and removal for as long as the reader contribution lives; removal
returns null rather than retaining old geometry. Rebinding detaches the old
node's observation. These notifications join the normal foreground dirty
queue and do not re-enter Rhai during prepaint. Event callbacks resolve another node in the same formal
component with `ctx.element_bounds("local_ref_key")`; the custom ref itself is
not durable callback data. Use this API only when a render truly depends on
another element's previous committed geometry; it cannot create same-layout
synchronous feedback.

For dimension-only render dependencies, prefer `ctx.element_size(ref)` or
`ctx.element_size("local_ref_key")`: it returns logical layout
`#{ width, height }`, or `()` until measured. It follows the same ref lifecycle
without invalidating on position, visual-transform or clip changes. Full bounds
reads remain independent and continue receiving those changes; event-time size
reads are untracked.

## Identity and window commands

`ctx.window_id()` identifies the actual host window and is shared by sibling
views. `ctx.view_id()` identifies the mounted widget and is unique inside the
Host.

Ordinary `mount` views reject `open_window`, `focus_window`, `close_window`, and close
handler registration immediately with `UnsupportedInEmbeddedView`. The
standalone `ScriptApplication` adapter enables the existing restricted
multi-window implementation.

Rust can explicitly delegate those commands with `PreparedScriptView::mount_window`.
This registers the native handle and installs a close-confirmation interceptor
for one owner view; sibling ordinary mounts remain disabled. The owner claim is
unique per native window, including across Host aliases, and released on failed
mount/disposal. Native closure also disposes retained view handles. The adapter
does not take over Rust's root/layout, but replaces any previous should-close
callback; keep ordinary `mount` when Rust must retain that policy. See
[Multi-window](multi-window.md) for the queued command and cleanup boundaries.

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
policies it owns or replace the options entirely. To draw the title bar in
Rhai, hide the platform bar there and let the view's drag areas move the
window:

```rust
ScriptApplication::new(prepared)
    .window_options(|mut options, _cx| {
        options.titlebar = Some(TitlebarOptions {
            title: Some("Workbench".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(12.0), px(11.0))),
        });
        options.app_owns_titlebar_drag = true;
        options
    })
    .window_drag_areas(true)
    .run()?;
```

Rhai then renders `TitleBar(#{ ..., inset_start: 72, window_drag: true })`, or
calls `.window_drag_area()` on its own node. Embedded hosts use
`ScriptViewConfig::window_drag_areas(true)` for the view that draws the bar;
every other view keeps drag areas inert.
`traffic_light_position` is fixed when the window opens. A bar whose height
changes, with the density for instance, moves the window buttons with
`Window::set_traffic_light_position` after it is laid out; the Gallery reads
its bar's height from the view's accessibility snapshot after each committed
render and centres them (`gpui_rhai_cli::acceptance::titlebar_traffic_lights`).

Drag areas move the window on macOS and Linux. GPUI's Windows platform
implements neither the window move nor the title-bar double press, so on
Windows they do nothing; keep the platform title bar there.

## System notifications and the Dock badge

GPUI posts system notifications (`cx.show_system_notification`, with
`cx.on_system_notification_response` for clicks and action buttons; one
response handler per application) and rings the bell
(`window.play_system_bell()`, `window.request_attention()`). `gpui_rhai::system`
adds what GPUI lacks (0.4.2):

- `system::notifications_available()`: macOS posts notifications only for an
  application bundle with a bundle identifier, so a binary started by
  `cargo run` shows none and GPUI drops them without an error. Run
  `gpui-rhai dev --bundle` while developing notifications: it runs the debug
  build inside an application bundle ([bundling](hot-reload-and-production.md#bundle-a-macos-application)).
- `system::set_badge_label(Some("3"))` / `set_badge_label(None)`: the macOS
  Dock badge, set from the main thread (where GPUI runs callbacks); other
  platforms return `SystemServiceError::Unsupported`.
- `ScriptApplication::app_identity(identifier, name)`: the identity Linux and
  Windows show on notifications; macOS reads it from `Info.plist`.

macOS banners from GPUI carry no sound; ring the bell beside the notification
when it should be heard.

## Files dropped from the operating system

A `DropZone` whose `payload_types` lists `os.files` takes the files and folders
the operating system drags in, once the Host allows it; it is off by default:

```rust
let view = ScriptView::files("ui/main.rhai").accept_file_drops(true);
```

The zone highlights while a drag it can take hovers over it. Its `drop` event
comes from source `os` with operation `copy`, and its payload is
`#{ files: [#{ file, name, size, kind }] }`: `file` is an opaque handle of kind
`file`, `name` the name without its folders, `size` the bytes of a file (`()`
for a folder) and `kind` `file` or `dir`. `file_kinds: ["file"]` limits a zone
to files; a drag holding another kind cannot drop there.

Scripts never see a path. To act on a file, a script passes its handle to a
capability. `UiRuntimeState::dropped_files` keeps the paths: the Host's
extension clones it when it registers the capability, and the handler turns the
handle back into the path. The handle stays valid while the application runs.

```rust
struct OpenFile { dropped: gpui_rhai::DroppedFiles }

impl CapabilityHandler for OpenFile {
    fn call(&mut self, _: &str, input: UiValue) -> Result<UiValue, String> {
        let UiValue::Handle(handle) = input else { return Err("expected a file".into()) };
        let path = self.dropped.path(&handle).ok_or("not a dropped file")?;
        // ...
        Ok(UiValue::Null)
    }
}

impl ScriptViewExtension for Files {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        let open = OpenFile { dropped: runtime.dropped_files.clone() };
        runtime.capabilities.register(descriptor(), open).map_err(|e| e.to_string())
    }
}
```

Outside a capability, `ScriptViewHandle::dropped_file_path(&handle, cx)` gives
the same path.

## Key bindings

Mounting never modifies the App keymap. Inspect `PreparedScriptView::key_bindings`
and explicitly approve them:

```rust
host.bind_keys(prepared.key_bindings().iter().cloned(), cx)?;
```

The operation is idempotent for identical bindings and rejects conflicting
actions for the same keystrokes/context.

## Retained suspension and Host-managed tombstones

Use `ScriptViewHandle::suspend` when a Host wants to remove a view from its
active layout without paying a later dispose/remount cost. Suspension keeps the
last-good declarative tree, component state, native primitive entities, input
selection/undo state, scroll positions, and virtual-list measurements. It
closes the view's overlays and Layers, releases focus and pointer capture, runs
the optional `suspend(ctx)` hook, cleans every active declarative effect, and
freezes declarative timers and Motion. Retained native primitives receive the
same lifecycle boundary: they quiesce owned subscriptions, timers, background
candidate installation, and active gestures while retaining committed native
state. On resume they re-establish subscriptions before reading the latest
Host-owned revision, so bursts coalesce to one current-state rebuild.
Native hooks are a prepare phase: all mounted instances are notified even when
one fails, and successfully changed peers are compensated. A failed suspend
therefore remains Active and retryable; a failed resume remains Suspended and
retryable. The public view state changes only after native and script phases
both succeed. Compensation back to Active executes both native resume prepare
and `commit_resume`, so two-phase primitives restart subscriptions and activity
time exactly as they do on an ordinary successful resume.
If compensation itself fails, the framework quarantines the view by disposing
it and unmounting every registered primitive. It never reports ordinary
`Suspended` while a native resource may still be active; the Host must recreate
that view.

```rust
view.suspend(window, cx)?;
assert_eq!(view.state(), gpui_rhai::ScriptViewState::Suspended);

// Later, before putting the element back into Host layout:
view.resume(cx)?;
```

`element()`, `flex_item()`, focus, automation, accessibility snapshots, and
element-ref commands reject a suspended view. A Host should branch on
`view.state()` and omit that view from newly constructed layout until `resume`
succeeds. Read-only diagnostics and `root()` remain available. Host writes to
signals, native collections/documents, themes, and locales may continue; they
accumulate invalidation without invoking Rhai.

One-shot tasks already in flight may finish while suspended. Their completions
are held in a bounded queue (256 per runtime) and delivered immediately before
the resume hook. Long-lived subscriptions must be created by declarative
effects and are therefore cancelled by effect cleanup. Effect-owned tasks are
cancelled with the same activation; ordinary one-shot component work continues.
Timers and Motion resume from their frozen progress and never catch up the
elapsed wall time.

The optional script hooks are:

```rhai
fn suspend(ctx) { /* synchronous, bounded quiesce bookkeeping */ }
fn resume(ctx, elapsed_ms) { /* decide whether Host-backed data is stale */ }
```

Resume is one atomic transaction: current-generation task results are applied,
`resume` runs, the tree reconciles once against all Host-side changes, and
effects restart with fresh activation identities. Failure preserves the
suspended last-good view. If development hot reload observed edits while the
view was suspended, it retains only filesystem changes; on resume the latest
successful candidate is compiled and migrated in the same transaction. Async
results from the replaced generation are discarded.

This contract is intended for a small Host-owned LRU of inactive panels.
Eviction still uses normal `dispose`; suspension is not a second persistence or
state-serialization format.

## Disposal

Remove a configured widget with `view.dispose(cx)?` before dropping it. Disposal
is idempotent and immediately cancels tasks/subscriptions, removes scoped state,
actions, overlays, tooltips, layers, and declarative timers, and frees the
`view_id` for remounting.
Dropping the final handle is a fallback. Merely omitting `view.element()` from a
temporary page does not dispose it.

## Last-good trees and host-visible failures

Rendering is transactional. When a callback, delivery, or rerender fails, the
transaction rolls back and `ScriptViewHandle::root` continues to return the last
successfully committed tree, so the host never observes a partial candidate.
`ScriptViewHandle::last_error` reports the failure until the source reloads
successfully or someone acknowledges it (the banner's Dismiss button, or
`ScriptViewHandle::clear_error` from a host-owned error panel). Later successful
events do not clear it: the view keeps working after a rollback, and an error
that vanished on the next event would never be read.

The built-in banner is monospace and selectable through the normal Host copy
action. An application with its own error panel may opt out per mounted view:

```rust
let config = ScriptViewConfig::new("user-view").show_error_banner(false);
```

The opt-out changes presentation only; `last_error` continues to report the
failure and the last-good tree remains mounted.

A Host learns of failures without polling through an observer, called once for
each new failure on the foreground after the frame that presents it:

```rust
let config = ScriptViewConfig::new("user-view")
    .on_error(|report, cx| show_failure(&report.view_id, &report.message, cx))
    .mirror_errors_to_stderr(true);
```

`report.diagnostic` is what `last_diagnostic` returns. Several failures before
that frame report the latest, as `last_error` does, and script windows the view
opens report through the same observer. `mirror_errors_to_stderr` also prints
each new failure as ``gpui-rhai: view `id`: message``; it is off for embedded
views and on for `ScriptApplication`, which turns it off with
`ScriptApplication::mirror_errors_to_stderr(false)`. See
[ADR 0025](adr/0025-host-error-reporting.md).

The same observer also receives recoverable native presentation failures: an
image that did not load, or an `auto_grid_columns` whose token bounds resolved
out of order and laid out adjusted. Check
`report.domain`: `Transaction` describes `last_error` / `last_diagnostic`;
`Presentation` instead carries `report.presentation` (node ID/path, optional
logical asset ID and bounded message). Presentation errors keep the placeholder
and do **not** set `last_error`, roll back a valid script tree or stop input.
They are deduplicated per view/generation/node and error, limited to 256 active
entries; recovery, unmount, reload and disposal release their entries. Stderr
mirroring uses `[presentation]` for these reports. Host observers are deferred
past the view's borrow and may safely read their handle in either domain.

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
