# Release checklist

1. Treat 0.1.0 as the first public compatibility baseline. Document every later
   breaking runtime API, component schema, locale, registry, manifest, and
   generated-source change against its published predecessor.
2. Run format, all-target/all-feature check, strict Clippy, tests, and rustdoc.
   Run `python3 scripts/verify-target-manifest.py` first; CI, release smoke, and
   the checked-in target inventory must agree.
3. Test the declared MSRV (`1.94`) and latest stable toolchains.
4. Run `cargo package` and inspect the file list for `gpui-rhai`,
   `gpui-rhai-registry`, and `gpui-rhai-cli`. A CLI candidate may use
   `--no-verify` only while its exact core/registry version is not yet indexed;
   after publishing those dependencies, its publish dry-run must perform the
   full clean rebuild.
5. Re-run the full Cargo metadata license matrix; investigate unknown licenses
   and update `THIRD_PARTY_LICENSES.md` for copied source or assets.
6. Build every example in release mode and run `scripts/release-smoke.sh` on
   macOS with the Metal Toolchain installed.
7. Run `scripts/audit-visual-baselines.sh` and complete the unlocked keyboard,
   focus, IME, clipboard, overlay, and multi-window interaction matrix.
   Before manual work, run the independent `tests/native-keyboard` workspace's
   tests and strict test-target Clippy; it is intentionally outside the main
   Cargo workspace so GPUI `test-support` cannot enter release dependency
   resolution.
   Run the independent `tests/performance` structural suite on shared CI; keep
   absolute latency thresholds on the controlled Mac benchmark path.
8. Run `scripts/audit-release-artifacts.sh` to reject workspace paths and
   development-only inspector strings in embedded example binaries.
9. Run CLI clean and modified-project fixtures for init/add/check/dev metadata,
   diff/update, and embed behavior.
10. Record the exact GPUI/Rhai versions and known accessibility/platform gaps.
    When evaluating Grain, run the opt-in `grain-backend` tests and strict
    Clippy separately; a compiling feature is not production parity.
11. For the complex-control line, run fixed-Clock DatePicker locale cases, large
    scalar/custom Table probes, Select group/search cases, Pagination boundary
    transitions, and Textarea multiline IME/auto-grow cases.
    For a component-foundation release, also verify the catalog count, imported
    component metadata, required accessible-name schemas, decorative Icon
    semantics, Progress range metadata, Theme Studio coverage, and Gallery
    preparation across every category.

12. Verify crates.io authentication without printing the token. Publish
    dependency-first: `gpui-rhai`, then `gpui-rhai-registry`, then
    `gpui-rhai-cli`, waiting for each package to enter the index before verifying
    or publishing its dependents. Confirm a clean `cargo install
    gpui-rhai-cli --locked` from crates.io.
13. Tag the exact protected-main release commit with the target version and create the
    matching GitHub release only after all three crates are available.

Versions 0.1.0 through 0.1.2 use `RUNTIME_API_VERSION` 1. Version 0.1.3 moves
to Runtime API 2 and deliberately removes the old animation surface; verify
official components, copied examples, CLI metadata, Motion Gallery, reduced
motion, timelines, exit ghosts, layout/shared-layout behavior, and motion
budgets together. Version 0.1.4 stays on Runtime API 2; additionally verify
background SVG preparation and fonts/color cascade, bounded variant-cache
eviction, HostSlot cross-Host framing, adaptive overlay widths, IconButton
selected semantics, and inherited motion-group replay.
Version 0.1.5 also stays on Runtime API 2. Verify default and `charts` feature
contracts independently; all built-in series/coordinates, every theme and
locale direction, normal/reduced/none motion, Host token overrides, 10k
interactive and 100k streaming/downsampled paths, malformed data, GeoJSON/SVG
maps, linked interaction, custom Rust extensions, terminal SVG/PNG export,
Chart Gallery release smoke, and `gpui-rhai-chart-e2e-v1` together. Do not add
boundary datasets or network/geocoding authority to the release package.
