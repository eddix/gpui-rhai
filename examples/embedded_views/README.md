# embedded_views

The authoritative host-embedding example. A plain Rust GPUI window mounts three
independent Rhai views into one host-owned layout.

```sh
cargo run -p gpui-rhai --example embedded_views
```

It demonstrates per-view engine/runtime/lifecycle isolation, automatic widget
bounds for responsive classes, a 280-point Combobox escaping a 200-point
widget, cross-view outside dismissal, namespaced duplicate Combobox/Toast IDs,
a shared Host-level Toast queue, transparent embedded backgrounds, and explicit
dispose/remount behavior.
