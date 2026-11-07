# gpui-rhai-build

Embeds a [gpui-rhai](https://crates.io/crates/gpui-rhai) application's `ui/`
directory into its binary, from the application's `build.rs`.

```toml
[build-dependencies]
gpui-rhai-build = "0.4"
```

```rust,no_run
// build.rs
fn main() {
    gpui_rhai_build::embed_ui("ui").expect("embed ui/");
}
```

Mount the view with `gpui_rhai::script_view!()`. A debug build reads the files
under `ui/` (and reloads them with gpui-rhai's `dev-reload` feature); a release
build reads the copy this crate embedded. Cargo runs the build script again
whenever a file under `ui/` changes, so the copy matches the source.

The crate depends on nothing but the standard library, so it adds no build time
to speak of. `gpui-rhai init` writes the `build.rs` and the dependency.
