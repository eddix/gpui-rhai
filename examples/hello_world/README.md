# hello_world

This is the smallest complete GPUI Rhai application. It demonstrates:

- importing copied Button, Input and Label source, including Button's Spinner dependency;
- semantic theme tokens;
- a Rhai event callback;
- mouse click and Enter/Space keyboard activation;
- a `ScriptView` adapted to a standalone `ScriptApplication`.

Run it from the repository root:

```text
cargo run -p gpui-rhai --example hello_world
```

The equivalent production-style embedded source can be run with:

```text
cargo run -p gpui-rhai --example embedded_hello_world
```

Both executables expose the same `view()` factory used by native integration
tests. They register bundled fonts and all four Modern/Clean presets. Real
typing, native focus, controlled value and undo survive the live design switches.
