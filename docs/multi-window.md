# Multi-window applications

The restricted window command API is enabled by the standalone
`ScriptApplication` adapter. Views mounted into an existing host reject these
commands immediately with `UnsupportedInEmbeddedView`; the Rust host already
owns those windows.

Rhai can request native windows without receiving native handles:

```rhai
ctx.open_window("settings", "Settings", 640, 480, true);
ctx.focus_window("settings");
ctx.close_window("settings");
```

IDs are stable and unique; sizes, title length, window count, and pending command
count are bounded. Every standalone script window runs the same entry in a separate lifecycle and
component-state namespace. App stores are shared; window stores, theme and locale
overrides, overlays, animations, tasks, subscriptions, and image work are scoped.

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
