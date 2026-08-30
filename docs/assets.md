# Assets and images

Rhai addresses images only through namespaced logical `AssetId` values or typed
image handles. Providers may map IDs to embedded bytes, an application-owned
directory, or another Rust implementation; scripts never receive filesystem
paths or URLs.

## Declarative component assets

Component metadata lists its small registry assets. File and embedded script
preparation register and preload those declarations transactionally. An
asset-backed image node therefore performs only a cached `AssetId` lookup during
render; missing, malformed, or duplicate assets fail preparation rather than
causing render-time filesystem I/O.

The official Icon component uses one `source` prop that accepts either an
`AssetId` or an image handle, plus an optional `rtl_source` of the same kind:

```rhai
icon::Icon(#{
    source: asset("app/icons/chevron_next"),
    rtl_source: asset("app/icons/chevron_previous"),
    size: "sm",
    label: "Next page",
})
```

This is a real union schema, not two precedence-sensitive handle/asset props.
Component metadata records the provider-relative files such as
`icons/chevron_next.svg`; installed component source refers to them through the
application asset namespace.
Capability-provided image handles remain the path for dynamic application
images. Dropdown, Select, DatePicker, Table, and Pagination ship only the
minimal check/clear/disclosure/calendar/sort/directional SVGs they need;
complete icon packs remain optional source packs.

## Dynamic loading

Application lifecycle code may still load or decode dynamic assets:

```rhai
let handle = ctx.load_image(asset("app/check"));
let decode = ctx.start_image_decode(
    asset("app/avatar"), Fn("image_loaded"), Fn("image_failed")
);
```

Raster validation/decode may run on a worker. Rhai callbacks return to the
foreground thread, are generation-bound, and cancel with their app/window/
component scope.

SVG assets may use `currentColor`. The renderer resolves semantic text color,
tints bytes, and caches by asset identity plus RGBA. Theme switching recolors
icons without script recompilation or state loss.

In file-backed development, supported asset changes refresh the provider
transactionally. Existing logical identity remains stable, tinted cache entries
are replaced, and affected windows invalidate.
