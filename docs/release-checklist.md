# Release checklist

1. Treat 0.1.0 as the first public compatibility baseline. Document every later
   breaking runtime API, component schema, locale, registry, manifest, and
   generated-source change against its published predecessor.
2. Run format, all-target/all-feature check, strict Clippy, tests, and rustdoc.
   Run `python3 scripts/verify-target-manifest.py` first; CI, release smoke, and
   the checked-in target inventory must agree.
3. Test the declared MSRV (`1.95`) and latest stable toolchains.
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
   Shared Linux CI must also run `scripts/linux-window-smoke.sh` against the
   release `phase0_probe` under both Xvfb/X11 and headless Weston/Wayland.
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
Chart Gallery release smoke, and `gpui-rhai-chart-e2e-v2` together. Do not add
boundary datasets or network/geocoding authority to the release package.
Also verify custom namespace colors through ordinary/native/virtual snapshots,
non-default spacing/radius and enlarged typography, bundled Tabs contrast,
theme-scaled registry components, and matching native/export Chart typography.
Version 0.1.6 stays on Runtime API 2 but changes the public Rust GPUI package
identity and MSRV. Verify the exact core/platform family on every dependency
graph; standalone, embedded and Host-owned entrypoints; the complete official
component AX inventory; native VoiceOver actions; bounded virtual/Table/Chart
semantics; X11 and Wayland backend smoke; and macOS builds without a separately
downloaded Metal Toolchain. Application dogfooding after publication feeds the
next patch/feature line and is not a publication gate. A
published family without upstream #64672 or equivalent behavior is not
releasable.
Version 0.1.7 retains Runtime API 2. Verify every bundled Gallery story/case,
the complete shell and Operations Workbench workflows, theme/locale/Motion hot
switching, HostSlot IME, subscription cleanup, Table search/page projection,
Chart wheel/diagnostic behavior, SplitPane and Resizable pointer/keyboard
parity, the 100k streaming benchmark at zero Rhai operations, checked-in visual
baselines, and launch from an empty working directory.
Version 0.1.8 retains Runtime API 2. In addition to the ordinary gates, run the
interaction adversarial matrix in
`docs/audits/2026-09-30-interaction-0.1.8`: same-Host multi-View capture,
unmount during drag, clipped/overlapped targets, same-local-key Sortables,
virtual keyboard Home/End, source/constraint replacement during gestures,
suspend cancellation, caller-ref composition, rejected Table resize, extreme
affines, rotated marquee, null/collapsed/disabled Tree state, deep collapsed
outlines, recursive async delivery limits, and CLI capability/static-check
separation. Verify post-mount NativeCollection registration while active and
suspended. #83, #89 and #91 remain explicitly deferred design work rather than
implicit 0.1.8 promises.
Also run `docs/audits/2026-09-30-interaction-0.1.8-round2`: idle PanZoom
suspend/resume, timer-driven gesture invalidation, every Table resize entry,
endpoint RangeSlider feasibility, Rotatable handle/resize geometry, virtual
offscreen source leases and ListState edge scrolling, degenerate/single
selection, wheel Escape, complex disabled Tree ancestry, early async rejection,
Host-init-dependent CLI validation, missing NativeCollection readers, and
synchronous native Automation failure reporting.
Also run `docs/audits/2026-09-30-interaction-0.1.8-round3`: presented primitive
identity and focus survival, mixed-success async batches, drained stale effect
activations, Host/window owner cancellation, external-source virtual target
scrolling, virtual-row formal callbacks, current Rotatable Canvas geometry,
off-grid range endpoints, repeated uncontrolled Table resize/cancel,
scale-independent marquee selection, bounded render-only negative collection
dependencies, shared disabled Tree ancestors and fail-fast online schema
diagnostics.
Also run `docs/audits/2026-10-01-interaction-0.1.8-round4`: full virtual target
resources and rollback, prune-only cleanup, unseeded row callbacks/read
dependencies, real Canvas paint/hit/pivot with padding/border, translation/zoom
invariance, foreground occlusion, row-gap/stationary auto-scroll, debounce
completion and silent cancelled-effect delivery. Verify #93 default diagnostics
and preserve the recursive tagged-default contract.

## Final 0.1.8 convergence gate

Use the maintained [runtime contract index](runtime-contract-tests.md) and
[final runtime model](adr/0022-final-runtime-invariants.md) as the executable
entrypoints, rather than stopping at one historical audit round. Run the full
workspace and independent `tests/native-keyboard` suite on the frozen product
source, with the default thread stack and unchanged safety defaults.

The R5–R7 regressions remain mandatory: transparent virtual-row wrappers,
path-level reads, nested and sibling target batches, prune-only cleanup,
callback ownership, bounded raw/formal contributions, occlusion and current
Canvas geometry; revoked queued-window origins and reused IDs; shared Table
widths/extent in LTR/RTL and all four body states; and persistent ElementRef
appearance/rebind/removal with last-good rollback. The index names their
formal tests and controls. Historical audit probes/logs stay unchanged;
characterization programs must be interpreted by their documented assertions,
not mechanically by their exit status.

Additionally verify POLICY/KEY/THEME/ASSET together: cumulative custom Host
quotas and independent parser limits across constructor/reload/window paths;
focused key phase routing without stealing native input or Escape ownership;
actual mounted appearance before init and explicit effect dependencies;
foreground publication and explicitly scheduled shared-window image redraw.
Check CLI init/add/check/update/embed in an owned clean directory, including
modified-source preservation, Table source metadata and the generated Rust
consumer. A local-path consumer is not a crates.io clean-install result.

Re-capture the five Table PNGs from the final release binary under the exact
logical viewport/DPI contract in the
[approved plan](plans/2026-10-02-0.1.8-release-convergence.zh-CN.md#9-五张table视觉基线工作包-g).
Complete or explicitly mark pending the real keyboard/focus, clipboard, IME,
VoiceOver, window/multi-View/theme and physical 120Hz gates. Native TestPlatform
behavior and a 69-file PNG audit do not certify these manual gates. Record
source SHA, commands, counts, platform, binary hashes and pass/fail/pending in
the candidate verification manifest; final-head Linux X11/Wayland CI must
finish, not merely start.

On macOS run the maintained actual-appearance binaries from the independent
native workspace (`theme_appearance_probe`, with `--initial light` and `dark`,
and `theme_appearance_lifecycle`). Pass `--output` and the verified
`--source-sha`; assert their structured error/pass and cleanup results. AppKit
termination can report process status 0 even for a failing assertion. These
gates use an app-only appearance override, restore it and close their owned
windows; they neither alter OS preferences nor require floating. GPUI's external
TestAppContext does not expose its private TestWindow appearance simulator;
do not replace this gate with a no-op TestPlatform setter or a forced refresh.

The scope deferrals #83/#89/#91/#95 and #14 are not acceptance failures or
delivered capabilities. Follow the explicit PR/issue disposition table; no
merge, closure, package publication or tag is authorized by green checks alone.
