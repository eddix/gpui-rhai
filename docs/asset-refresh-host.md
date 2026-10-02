# Mutable images: an application-owned foreground bridge

gpui-rhai 0.1.8 can display changed application-owned images without making
`AssetRegistry` thread-safe or adding a new SDK invalidation API. The existing
choices are versioned logical IDs, or a stable ID whose registered namespace
the Host refreshes on the foreground. This guide describes their limits; it is
not a per-asset revision or strongly consistent publication contract.

See [asset loading](assets.md) for declarations, supported formats and dynamic
decode, and [the executable Host fixture](../tests/native-keyboard/tests/asset_refresh_host.rs)
for the complete, schema-validated registration and mounted native test.

## Prefer immutable, versioned IDs when practical

A Host provider can expose `media/cover-r7`, then `media/cover-r8`, and select the
new `AssetId` after preparation. The path/name stays inside the registered
provider; Rhai does not gain arbitrary filesystem or URL access. Content at an
individual ID remains immutable, so an earlier decode cannot be mistaken for a
later version of that same ID. Use an application request revision to discard
obsolete completions before selecting the new ID.

This is a recommended application pattern, not automatic cache management.
The registry retains its logical `AssetId` mappings and image handles for the
registry's lifetime; it does not automatically evict obsolete IDs from an
unbounded version stream. Replacing the last owning registry eventually releases
that registry's data, but GPUI's separate image cache can have a different
lifetime. Do not promise immediate GPU-memory reclamation. Choose bounded
application retention and registry ownership, particularly for frequently
updated covers or thumbnails; do not generate an unlimited new ID per frame.

## Stable ID: background preparation, foreground publication, redraw

There are three different operations:

1. Background work prepares owned typed data, an allowlisted logical identity,
   and an application revision.
2. A foreground Host entry updates its controlled provider and calls
   `assets.refresh_namespace("media")` for already-cached entries.
3. The Host schedules the affected native view/window to redraw. Cache mutation
   alone does not schedule presentation.

`AssetRegistry` contains foreground `Rc<RefCell<...>>` state. A synchronous
`CapabilityHandler` does **not** require `Send` and can safely hold a registry
clone when it runs on the foreground. `TaskWork` does require a `Send + 'static`
closure: it must not capture that clone, a GPUI context, a Rhai `Dynamic` or a
`FnPtr`. `AssetProvider` itself also has no `Send` guarantee; do not assume every
provider can simply be moved to a worker.

The fixture's worker builds this application-owned result before converting it
to the schema-checked `UiValue` task transport:

```rust
struct PreparedCover {
    asset: AssetId,
    revision: i64,
    data: AssetData,
}
```

Its synchronous publisher owns `AssetRegistry` and the small mutable memory
provider. It validates the exact asset allowlist, bounded revision and prepared
data, then replaces the provider bytes and refreshes the namespace. The fixture
has one cached image and accepts only a small application-authored SVG; it does
not introduce arbitrary paths, URLs, filesystem watching, or a generic image
editing permission. Its application revision check belongs to the Host, not to
`refresh_namespace`.

Register both capabilities in `ScriptViewExtension::configure_runtime`, with
separate descriptors for the background prepare and synchronous publish
methods. Declare them in the application's `AppManifest`. A successful task
completion can then use the existing callback path:

```rhai
fn request_cover(ctx, payload) {
    ctx.start_task("app.cover_prepare", "prepare", (),
        Fn("cover_prepared"), Fn("cover_failed"));
}

fn cover_prepared(ctx, result) {
    // This registered synchronous handler owns the foreground registry clone.
    let revision = ctx.call_capability("app.cover_publish", "publish", result);
    ctx.set_state("cover_revision", revision);
}
```

The mounted view can keep using `image_source(asset("media/cover"))`. Refresh
preserves the existing logical `AssetId` and opaque `ImageHandle`; changed bytes
replace the underlying GPUI image/content identity. Old frames that already own
the previous image remain valid. A subsequent render resolves the same logical
ID/handle to the new content.

Changing the callback's application state normally requests an owning-view
update, but it is not a broadcast to every window sharing the registry. The
embedding Host should arrange an explicit repaint at its appropriate foreground
boundary. A current-window-only integration may call `window.refresh()` there.
For intentionally shared registries, the existing GPUI call is:

```rust
// Run at a foreground Host/NativeHandler/GPUI completion entry, after publication.
app.refresh_windows();
```

A synchronous `CapabilityHandler::call` does not receive `App` or `Window`; it
cannot perform that scheduling by itself. Use an embedding Host foreground
completion or a registered `NativeHandler`, whose callback receives
`&mut UiRuntimeState`, `&mut Window` and `&mut App`, when publication and redraw
must be paired in one native entry. Do not dispatch a second fake UI click to
manufacture that boundary.

`App::refresh_windows` redraws all application windows, including cached native
views. This is intentionally broad. A larger Host may instead retain explicit
affected-view/window ownership and schedule only those windows. Independently
prepared views do not automatically share an asset registry; cache sharing must
be an explicit Host choice, separate from each view's application state.

## What namespace refresh does—and does not—promise

`refresh_namespace` synchronously reloads **already cached** images in that
namespace and returns their count. It is a coarse foreground operation: provider
reads and preparation for multiple images can cost noticeable UI time. Reserve
it for a bounded, controlled set, not an unbounded cache or a per-frame update.

Existing provider/MIME/empty-data/preparation failures retain the old cached
data. This is not a guarantee that every raster is fully decoded successfully
before publication: a nonempty malformed raster may still fail later in GPUI.
The fixture's known valid SVG and publisher rollback are application controls,
not a stronger SDK last-good decoding promise.

The operation also does **not** version all pending decodes. An image with only
a pending decode is not in the cached set, so refresh can return zero while that
older decode later installs its prepared bytes. Nor does it provide per-asset
ordering, automatic revision checks, automatic window repaint, or an unchanged
content/no-extra-redraw guarantee. A full per-asset prepare/publish protocol,
failed-decode last-good behavior, and stale-result arbitration are separate
future design work.

Calling `load_image` again returns the cached handle. Calling
`start_image_decode` again for an already cached ID also does not replace that
cached image at installation. Neither is a reload API. Use the explicit Host
refresh path above or select an immutable new versioned ID.

## Executable evidence and its boundary

Run the maintained native fixture without changing the thread stack:

```sh
env -u RUST_MIN_STACK cargo test \
  --manifest-path tests/native-keyboard/Cargo.toml --locked \
  --test asset_refresh_host -- --nocapture --test-threads=1
```

It drives a real Rhai `start_task` completion into the registered synchronous
publisher, verifies the worker/foreground thread boundary, stable asset and
opaque-handle identity, changed GPUI content ID, decoded BGRA red→blue pixels,
and the mounted image's native accessibility geometry. Its shared-registry case
calls `App::refresh_windows`, verifies both actual Host roots redraw, and checks
that peer application state remains isolated.

Those assertions prove the bounded existing-API bridge and native redraw /
geometry behavior. They are not a full-window GPU pixel screenshot, nor proof
of strong revision ordering for arbitrary pending decodes, all formats, or
all lifecycle combinations. The wider future publication contract must test
those cases explicitly.
