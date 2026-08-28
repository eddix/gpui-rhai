# hello_world

This is the smallest complete GPUI Rhai application. It demonstrates:

- importing copied Button and Label source;
- semantic theme tokens;
- a Rhai event callback;
- mouse click and Enter/Space keyboard activation;
- the `ScriptApp` Rust host.

Run it from the repository root:

```text
cargo run -p gpui-rhai --example hello_world
```

The equivalent production-style embedded source can be run with:

```text
cargo run -p gpui-rhai --example embedded_hello_world
```
