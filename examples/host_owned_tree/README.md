# host_owned_tree

A Rust-owned interactive `UiNode` tree with no running Rhai UI engine, script
lifecycle, capability, subscription or adapter script. At startup only, the
example parses bundled theme configuration into validated data snapshots, then
drops that configuration engine.

The Host translates a plain Rust frame into nodes, attaches `HostCallback`
closures, sends TextInput and row interactions to a worker channel, and applies
the next controlled frame with `StaticUiView::set_root`.

The Host registers bundled fonts and gives `StaticUiView` the selected theme
snapshot. The renderer resolves token-based styles through the same path as
script views. The **Next design** button switches all four Modern/Clean presets
without replacing the native input or its edit history.

Text input also performs a synchronous controlled-value echo through a
`WeakEntity` before sending the event to the worker. This prevents fast native
input from being reconciled against a stale Host frame while the authoritative
worker response is in flight.

```sh
cargo run --release -p gpui-rhai --example host_owned_tree
```

Callbacks capture channel senders and, where foreground feedback or theme
switching is needed, `WeakEntity`. A callback stored inside a Host-owned tree
must not strongly capture the Entity that owns that tree. Background work sends
typed data; native state updates still happen on the foreground.

Native integration tests mount the executable's actual `view()` constructor.
They type into the native input, wait for the real worker response, click
**Next design** through all four presets and verify retained identity and native
undo. Theme parsing and font registration are startup configuration in that
constructor; no Engine is retained in the view, callback or worker.
