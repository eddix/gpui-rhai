# Multi-window example

This embedded example exercises the restricted script window-command API.
The main lifecycle opens the settings window automatically so release smoke
also covers real native multi-window creation.

- `ctx.open_window`, `focus_window`, and `close_window` use validated stable IDs.
- Every window runs its own script lifecycle and owns a distinct component-state root.
- The `shared` app store invalidates readers across windows.
- `ScriptViewExtension::configure_window` declares a `session` store for each window.
- Theme and locale overrides are per-window; choosing Arabic also exercises RTL layout.
- Native close requests call a generation-bound Rhai handler. The confirmation dialog
  explicitly calls `close_window`, after which window state, stores, overlays, animation,
  tasks, subscriptions, and pending image work are released.

Run it from the repository root:

```sh
cargo run -p gpui-rhai --example multi_window
```
