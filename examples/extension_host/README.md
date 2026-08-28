# Host extension example

This example demonstrates the supported Rust host-extension boundary without
depending on `gpui-component`.

- `DemoExtension::configure_engine` registers the namespaced
  `my_app::StatusCard` primitive before Rhai compilation.
- `DemoExtension::configure_runtime` registers schema-validated synchronous
  (`app.text_transform`), one-shot asynchronous (`app.delayed_text`), and
  continuous subscription (`app.ticker`) capabilities. The embedded
  `AppManifest` explicitly activates compatible versions before lifecycle
  `init`.
- The embedded Rhai entry calls all three capability forms in `init`, receives
  task/subscription results on the GPUI foreground thread, stores the values,
  and renders them through normal nodes and the custom primitive in `view`.

Run it from the repository root:

```sh
cargo run -p gpui-rhai --example extension_host
```
