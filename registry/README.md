# gpui-rhai-registry

The versioned first-party source bundle for
[GPUI Rhai](https://github.com/eddix/gpui-rhai). It contains the editable Rhai
components, themes, locales, Theme Studio source, and small SVG assets copied
into application repositories by `gpui-rhai-cli`.

Most application authors should install the CLI rather than depend on this
crate directly:

```text
cargo install gpui-rhai-cli --locked
gpui-rhai init
gpui-rhai add button input form_field
```

The crate intentionally exports immutable source constants so alternative
installers and build tooling can consume the exact same release snapshot. It
contains no runtime engine and grants no component privileged access to GPUI.
