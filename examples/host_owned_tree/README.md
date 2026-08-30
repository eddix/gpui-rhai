# host_owned_tree

A Rust-owned interactive `UiNode` tree with no Rhai engine, lifecycle,
capability, subscription, or adapter script.

The Host translates a plain Rust frame into nodes, attaches `HostCallback`
closures, sends TextInput and row interactions to a worker channel, and applies
the next controlled frame with `StaticUiView::set_root`.

Text input also performs a synchronous controlled-value echo through a
`WeakEntity` before sending the event to the worker. This prevents fast native
input from being reconciled against a stale Host frame while the authoritative
worker response is in flight.

```sh
cargo run --release -p gpui-rhai --example host_owned_tree
```

Callbacks capture only channel senders. A callback stored inside a Host-owned
tree must not strongly capture the Entity that owns that tree; use channels or
`WeakEntity` to avoid retain cycles.
