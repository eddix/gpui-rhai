# GPUI providers

The default builds use public crates: Pre `0.3.14` or Fast `0.1.19`.
The repository also includes an opt-in macOS vendor profile (Pre `0.3.11`,
Fast `0.1.13`) for the extended Modern/Clean rendering effects. The same SDK
and registry sources support both profiles. Choose one provider per process.

## Compare in Gallery

```sh
bash scripts/gallery-profile.sh public pre --page email_draft --theme modern-light
# Close the first window before launching the next one.
bash scripts/gallery-profile.sh vendor pre --page email_draft --theme modern-light
```

Try `--page button`, `--page tabs`, `--page input`, `--page tooltip`,
`--page foundations.backdrop_blur` and `--page foundations.affine` with the
same theme. Vendor is an older, modified GPUI baseline; this is a comparison
of the available profiles, not a controlled performance benchmark.

| Surface | Public profile fallback |
| --- | --- |
| Layered fills and repeated stripes | A native two-stop fill or native slash pattern. The exact multi-stop edge, clip boxes and moving gradient plane are vendor-only. |
| Hairlines and shadows | The public native shadow stack; no alternate translucent-surface cutout. |
| Hover, focus, loading and expansion | Native states and callbacks are retained; computed-paint color transitions and content-size interpolation can snap. |
| Glass and filters | Ordinary non-blurred surfaces; no group filters, glyph-sweep mask or composed-group opacity. |
| Typography and wrapping | Native tracking; ordinary wrapping without figure exclusion or glyph-level underline animation. |
| Transformed interactive subtrees | Remain in the ordinary layout plane so pointer, focus and editing do not diverge from paint. Canvas retains its command-space transforms. |

Use Rust's `gpui_rhai::EXTENDED_RENDERING` to distinguish the profiles.
VoiceOver announcement and system IME candidate-window anchoring are manual
acceptance items; compiling or drawing an offscreen frame does not prove them.

## Build the SDK or reference application

The regular Cargo workspaces and lockfiles are the release/public build.
`with-vendor-gpui.sh` performs a separate build with the checked-in vendor
locks and the opt-in SDK feature. It does not change your checkout lockfile.

```sh
bash scripts/with-vendor-gpui.sh sdk fast check -p gpui-rhai-cli
bash scripts/with-vendor-gpui.sh native pre test --test native gallery::module_coverage
bash scripts/with-vendor-gpui.sh apps pre run --release -p disktree-rhai -- /path/to/scan
```

To embed another GPUI consumer such as NSH, keep the whole host graph on the
same provider. The host's root Cargo patch must select every modified package
it uses; a downstream SDK Cargo feature cannot change those package sources.
The older vendor Fast profile also exposes extended native types. Do not
assume a given NSH release supports those types or that the older Fast is
within its published version requirements. Use released Fast for the standard
host, or test the exact combined host graph before opting into the vendor
profile. See [vendor provenance](../vendor/gpui/README.md).
