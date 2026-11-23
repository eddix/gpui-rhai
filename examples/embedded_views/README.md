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

The full Rust `EmbeddedViewsDemo` constructor is also mounted by native tests,
not only its individual widget source. All four designs are selected through
the public view handles; native Host chrome follows the first widget. A remounted
widget explicitly inherits that current design while restarting its own state.
This is the application's Host policy, not an SDK rule that forces independently
mounted views to share themes. Peer widget state is preserved through remounts.
