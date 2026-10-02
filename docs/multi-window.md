# Multi-window applications

The restricted window command API is enabled by the standalone
`ScriptApplication` adapter, or by explicit `PreparedScriptView::mount_window`
delegation in a Rust application. Ordinary `mount` rejects these commands
immediately with `UnsupportedInEmbeddedView`.

## Delegating an existing native window

```rust,ignore
let host = ScriptViewHost::new("main", cx)?;
let owner = prepared.mount_window(
    ScriptViewConfig::new("main-view"), host.clone(), window, cx,
)?;
// Other independently prepared views use ordinary mount on the same Host.
// Their window commands remain disabled; authority is not inherited.
```

`mount_window` registers the actual GPUI window handle and gives one view
open/focus/close authority. Rust still owns the native root, layout and application
lifetime. Only one command owner can be mounted on a native window, even through
another Host alias. A Host also cannot be reused across different native windows.
Failed mounts release their claim; disposing the owner releases authority without
closing Rust's window, and a replacement owner can then mount.
Disposal revokes queued native operations that have not executed yet; stale
operations cannot close a replacement owner or bypass its close confirmation.
Releasing the last public Handle also revokes its mount lease, even when an old
GPUI frame still retains the Entity. Deferred source and target authority checks
use mount identity and actual WindowId; names or temporary Entity liveness alone
do not confer permission. A remaining Handle clone keeps the same mount valid.

The queue captures origin and target registration at request time. A secondary
view can pump shared work, but cannot substitute its own authority for the
source. This also holds when a successful async delivery queues work and a later
independent delivery fails. Revoking an Open releases only its own pending ID
reservation. `Open → Focus/Close` in one entry uses that same reservation, then
the actual created native window; it does not retarget a same-name replacement.
Direct Rust `WindowCommandRegistry::request_*` calls explicitly use trusted Host
origin. `drain_commands` returns qualified `QueuedWindowCommand` records; the
payload is available through `command()`.

This opt-in installs the should-close interceptor described below, replacing any
previous Rust should-close callback. Do not opt in when Rust must retain that
policy: keep ordinary `mount` and use application-defined Rust capabilities or
callbacks. Focus and confirmed close execute after the current window update
unwinds; a successful script request queues the operation, it does not prove the
native operation has already completed. Native errors remain observable through
`ScriptViewHandle::last_error`.

Native closure disposes all mounted views even when the application retains their
handles. Window stores, tasks, subscriptions, motion and interaction resources
are released; disposed handles cannot keep a closed window's runtime active.

## Script window commands

Rhai can request native windows without receiving native handles:

```rhai
ctx.open_window("settings", "Settings", 640, 480, true);
ctx.focus_window("settings");
ctx.close_window("settings");
```

IDs are stable and unique; sizes, title length, window count, and pending command
count are bounded. Secondary windows created by either adapter run the same entry in a separate lifecycle and
component-state namespace. App stores are shared; window stores, theme and locale
overrides, overlays, motion, tasks, subscriptions, and image work are scoped.
Geometry, presented-frame membership, and pointer capture live in a per-view
presentation domain, so tree-local NodeId and pointer numbers may overlap
without one window clearing or overwriting another.

Declare per-window Rust resources with
`ScriptViewExtension::configure_window(window_id, runtime)`. It runs before that
window's `init`. Use `ctx.get_window_store`/`set_window_store` for a declared
window store.

To confirm native close requests, install a generation-bound handler during
`init`:

```rhai
fn init(ctx) { ctx.set_close_handler(Fn("request_close")); }
fn request_close(ctx, payload) { ctx.set_state("show_close_dialog", true); }
fn confirm(ctx, payload) { ctx.close_window(ctx.window_id()); }
```

The host blocks the native close while the callback succeeds. Explicit
`close_window` is the confirmed close path. If the handler is stale or fails,
the host allows closing so an application cannot trap the user. See the
`multi_window` example for shared invalidation, per-window stores, RTL locale,
theme overrides, focus, and cleanup.
