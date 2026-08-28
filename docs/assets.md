# Assets and images

Rhai addresses images only through namespaced logical `AssetId` values. A
provider may map those IDs to embedded bytes, an application-owned directory,
or another Rust implementation; scripts never receive filesystem paths or
URLs.

Synchronous initialization may use:

```rhai
let handle = ctx.load_image(asset("app/check"));
```

For raster work during application lifetime, use generation-bound background
decode:

```rhai
let decode = ctx.start_image_decode(
    asset("app/avatar"), Fn("image_loaded"), Fn("image_failed")
);
// ctx.cancel_image_decode(decode);
```

The provider load is validated before spawning. Raster bytes are decoded on a
worker thread; Rhai callbacks remain on the foreground thread and receive an
opaque image handle or error string. Cancellation and hot reload prevent stale
deliveries.

SVG assets may use `currentColor`. At render time the image node inherits its
resolved semantic `text_color`; the registry rewrites and caches SVG bytes by
handle and RGBA value. Theme switching therefore recolors icons without file
access, script recompilation, or state loss.

Directional icons should provide explicit LTR and RTL image handles through the
official Icon component's `handle` and `rtl_handle` props. The runtime selects
the pair by window locale; it never mirrors arbitrary artwork.

In file-backed development, changes to supported SVG/raster files refresh the
`app` provider transactionally. Existing opaque handles keep their identity,
tinted SVG cache entries are replaced, and all windows are invalidated without
recompiling component ASTs or resetting state.
