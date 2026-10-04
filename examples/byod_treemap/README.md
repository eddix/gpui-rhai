# Bring your own design: treemap

A disk usage treemap that uses none of the gpui-rhai design language: no
`ui/tokens.rhai` token base, no official components, and a theme with its own
color names (`ground`, `ink`, `tile.code`, ...) and its own length namespace
(`treemap.gutter`, `treemap.pad`, `treemap.bar`). The squarified layout is
plain Rhai.

```sh
cargo run -p gpui-rhai --example byod_treemap
```

It is the reference for applications that ship their own design: the runtime
validates only what mounted components declare, so a theme needs no semantic
color set until an official component is mounted. CI builds the example and
`crates/gpui-rhai/tests/byod_example.rs` prepares and renders it.
