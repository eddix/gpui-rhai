# Host extension example

This example demonstrates the supported Rust host-extension boundary without
depending on `gpui-component`.

- `DemoExtension::configure_engine` registers the namespaced
  content-only `my_app::StatusMessage` primitive before Rhai compilation.
- `DemoExtension::configure_runtime` registers schema-validated synchronous
  (`app.text_transform`), one-shot asynchronous (`app.delayed_text`), and
  continuous subscription (`app.ticker`) capabilities. The embedded
  `AppManifest` explicitly activates compatible versions before lifecycle
  `init`.
- The entry calls the sync capability and starts the one-shot task in `init`.
  A formal Ticker component owns the subscription in its `watch` effect.
  Task/subscription results return to the GPUI foreground thread, update
  component state through `ctx.set_state` and render in the message/tick content.
- The ticker models an external producer with an `mpsc` sender and returns
  `SubscriptionWork::from_receiver`; the work therefore remains alive until the
  producer drops the channel instead of silently closing after setup returns.

Run it from the repository root:

```sh
cargo run -p gpui-rhai --example extension_host
```

The example starts with Modern/Dark and registers all four Modern/Clean presets.
The official Rhai `Card` owns its surface, radii, spacing and typography; the Rust
primitive inherits its text style rather than maintaining a second card design.
Root spacing and sizing also use shared tokens and adapt to the window width.

The native regression mounts the same `view()` factory as the executable. It
checks real task/subscription delivery, live switching through all four presets,
stable content identity and painted Card geometry at two window widths.
