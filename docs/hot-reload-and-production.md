# Hot reload and production embedding

## Development

Run `gpui-rhai dev` or a file-backed `ScriptView` with the `dev-reload` feature.
The watcher tracks Rhai modules, themes, the component stylesheet, locale bundles, and image assets. Script candidates are
compiled transactionally. A reverse dependency graph recompiles changed modules
and their transitive dependants while the resolver reuses content-matching ASTs;
the entry is then compiled self-contained for atomic commit. Failed candidates keep the last-good AST, tree,
callbacks, state, and component metadata. Successful reloads preserve compatible
state and invalidate callbacks from the previous generation.

A suspended `ScriptViewHandle` continues to collect changed filesystem paths
but does not compile modules or invoke Rhai in the background. `resume` processes
the latest batch first. The latest successful program candidate migrates through
the same atomic resume transaction; a failed edit keeps the prior generation
and appears through normal last-good diagnostics. Pending async deliveries from
an older generation are discarded rather than replayed into migrated code.

Mounted views surface the latest failed callback/render/reload above the
last-good tree as selectable monospace text. Embedded Hosts that render their
own error UI may set `ScriptViewConfig::show_error_banner(false)` and continue
reading `ScriptViewHandle::last_error`; standalone adapters expose the matching
`ScriptApplication` option. `ScriptViewConfig::on_error` tells the Host of each
new failure without polling, and `mirror_errors_to_stderr` prints it, as
`ScriptApplication` does by default, so unattended runs fail loudly. `ScriptViewHandle::last_diagnostic` returns the
same failure as structured data: bounded Rhai termination details, typed timing
and operation budget, deepest failing component identity, and a pre-redacted
snapshot of only that component's state. The failed transaction rolls back
and the view keeps working, so the error stays until a successful reload, the
banner's Dismiss button, or `ScriptViewHandle::clear_error`, which clear the
message and diagnostic together; a later successful event does not. Ordinary
error formatting never emits the snapshot.

The inspector is available only in development and opens with Command-Option-I
or F12. It shows source locations, redacted state, computed semantics, traces,
and execution timings.

## Production

A Host mounts one view for every build:

```rust
let view = gpui_rhai::script_view!()
    .extension(/* application services */)
    .prepare()?;
```

A debug build reads the files under the crate's `ui/` (with hot reload under
the `dev-reload` feature). A release build reads the copy that `build.rs`
embedded through `gpui-rhai-build`:

```toml
[build-dependencies]
gpui-rhai-build = "0.4"
```

```rust
// build.rs
fn main() {
    gpui_rhai_build::embed_ui("ui").expect("embed ui/");
}
```

`gpui-rhai init` writes both. The build script embeds every file under `ui/`
except hidden ones, and Cargo runs it again whenever one changes, so the copy
always matches the source; there is no generated file to keep in step. The
embedded tree loads as the directory does: `app.toml` names the entry,
`theme.rhai`, `tokens.rhai`, `styles.rhai` and `profile.rhai`, the other themes
and locales, the images under `assets/` (as the `app` namespace) and the fonts
under `fonts/`. Then:

```text
gpui-rhai check
cargo build --release
```

Production needs no filesystem watcher or source-path access, and file-backed
and embedded sources use the same restricted module-resolution contract.
`ScriptView::files(path)` and `ScriptView::embedded(&UI)` choose a source
explicitly, and `ScriptView::sources(..)` runs sources the Host holds in memory.

A debug build of `script_view!()` reads another tree when `GPUI_RHAI_UI=<dir>`
is set. A release build ignores it: swapping the scripts of a signed
application would be a way to inject code into it. A Host that wants such an
override names its own variable with `.ui_override_env("MY_APP_UI")`. Keep
`dev-reload` disabled in release builds unless a product has an explicit,
reviewed reason to ship source watching.

## Bundle a macOS application

`gpui-rhai bundle` builds the release binaries and wraps them in a signed
`.app`, configured in the application's Cargo.toml:

```toml
[package.metadata.gpui-rhai.bundle]
name = "Disk Tree"                    # display name; the package name by default
identifier = "com.example.disktree"   # required
icon = "bundle/AppIcon.png"           # a square PNG, ideally 1024px
binaries = ["disktree", "helper"]     # the executable first; the package's only binary by default
resources = [{ from = "licenses", to = "licenses" }]
```

The bundle lands in `target/release/bundle/<name>.app`: the binaries in
`Contents/MacOS`, an `Info.plist` with the package version, the icon converted
to `.icns`, and the resources in `Contents/Resources`. `licenses/` at the
project root, for third-party notices, is copied when the configuration lists
no resources of its own. The bundle is
signed ad hoc, enough to run on the machine that built it; `--sign <identity>`
signs with a certificate, and notarization for distribution stays the
application's step. `--name`, `--identifier` and `--icon` override the
configuration, and `--debug` bundles a debug build.

`gpui-rhai dev --bundle` builds the debug binaries into the same kind of bundle
and runs its executable, so macOS treats the process as the application: system
notifications and the Dock badge, which need a bundle identifier, work while
`script_view!()` still reads `ui/` from source with hot reload.

`gpui-rhai embed`, which generated `src/gpui_rhai_embedded.rs`, was removed in
0.4.3. To migrate, add the build script and dependency above, mount with
`script_view!()`, and delete the generated file.
