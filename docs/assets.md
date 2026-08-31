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
images. Dropdown, Select, and Table need no private assets; DatePicker and
Pagination ship only the minimal calendar and directional SVGs they need;
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

## Inline SVG atom

For small application-authored vector art that should remain inside Rhai source,
use the public `svg(markup)` atom. It accepts at most 64 KiB and 2,048 elements,
parses through the same `usvg` version as GPUI, preserves local fragment
gradients plus `currentColor`, and rejects DOCTYPE/entity declarations, scripts,
foreign/object/embed/style content, event attributes, and external/data/file/URL
references before a node is accepted.

```rhai
svg("<svg viewBox='0 0 16 16'><path fill='currentColor' d='M2 8L7 13L14 3'/></svg>")
    .with_style(style().width(px(16)).height(px(16)).text_color(theme_color("accent")))
```

Inline SVG does not read paths, fetch resources, or replace declared assets for
shared icons. GPUI remains the rendering backend; invalid markup is a script
evaluation error and preserves the last-good tree.

In file-backed development, supported asset changes refresh the provider
transactionally. Existing logical identity remains stable, tinted cache entries
are replaced, and affected windows invalidate.

## Declared fonts

File applications load `.ttf`, `.otf`, and `.ttc` files under `ui/fonts` before
their first mounted GPUI view. Embedded applications pass validated in-memory
`FontSource` values with `.font_source(...)` or `.font_sources(...)`.
`gpui-rhai embed` emits matching `FONTS` and `font_sources()` artifacts.

The Host validates safe labels, TrueType/OpenType headers, a 16 MiB per-font
limit, at most 64 sources, and duplicate payloads. App-global fingerprints
prevent sibling views from registering identical bytes repeatedly. GPUI
performs final parsing; failure aborts mount before the view is exposed. Rhai
receives only the platform family name used through `Style.font_family`, never
paths or bytes.

`Style.font_fallbacks` supplies 1-16 ordered unique platform family names and
`font_feature(tag, value)` maps validated four-character OpenType tags to GPUI.
Family aliases and development hot replacement remain incomplete. Until aliases
land, `font_family` must use the internal family name declared by the font
itself.
