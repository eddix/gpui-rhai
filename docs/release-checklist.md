# Release checklist

Run these gates on every release candidate. The gates specific to 0.1.0
through 0.1.8 are in the checklist listed under [History](history.md).

## Every release

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
   focus, IME, clipboard, overlay, and multi-window interaction matrix
   ([real-window scripts](../scripts/macos-real-window/)).
   Before manual work, run the independent `tests/native-keyboard` workspace's
   tests and strict test-target Clippy; it is intentionally outside the main
   Cargo workspace so GPUI `test-support` cannot enter release dependency
   resolution.
   Run the independent `tests/performance` structural suite on shared CI; keep
   absolute latency thresholds on the controlled Mac benchmark path.
   Frame budget (from 0.2.0, replacing the physical 120Hz gate): in
   `tests/native-keyboard`, `cargo run --release --bin gallery_profile`; every
   interaction's `Window::draw` p95 must be at most 8.3 ms, one 120 Hz frame.
   Rhai render and callback times are recorded, not gated.
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

## Runtime and native gates

- Use the [runtime contract index](runtime-contract-tests.md) and the
  [final runtime model](adr/0022-final-runtime-invariants.md) as the executable
  entry points. Run the full workspace and the independent
  `tests/native-keyboard` suite on the frozen source, with the default thread
  stack and unchanged safety defaults.
- On macOS run the actual-appearance binaries from `tests/native-keyboard`:
  `theme_appearance_probe` with `--initial light` and `--initial dark`, and
  `theme_appearance_lifecycle`. Pass `--output` and the verified
  `--source-sha`, and assert their structured pass and cleanup results: AppKit
  can report exit status 0 for a failing assertion. They use an app-only
  appearance override, restore it and close their own windows.
- Record the candidate: source SHA, commands, counts, platform, binary hashes
  and pass, fail or pending for every gate. Final-head Linux X11 and Wayland CI
  must finish, not merely start.
- Green checks alone authorize no merge, issue closure, publication or tag.

## 0.2.0 gates

- Frame budget: release `gallery_profile`, `Window::draw` p95 at most 8.3 ms
  for every interaction (item 7). The physical 120Hz check is no longer a gate.
- Visuals: `gallery_baselines` (19 cases) and `example_baselines` (38) match the
  committed PNGs; `gallery_baselines <dir> --pages <density>` for all 85 pages
  in both densities, compared with the previous candidate, with every change
  attributed to a commit. Compare in RGB (`Image.convert("RGB")`): Pillow's
  `getbbox()` on an RGBA difference looks at alpha only and reports opaque
  captures as identical. Include a known-different pair as a positive control.
- Audit: `gallery_acceptance` passes every page in both densities.
- Real windows: keyboard, focus, clipboard, multi-window, the Rhai title bar's
  drag and double press, and direct manipulation settling without further
  input are driven by the [real-window scripts](../scripts/macos-real-window/)
  on a real macOS window and recorded. IME preedit and VoiceOver are checked by
  the maintainer.
- The PR #111 disposition: review R1-R5 and issues #83 #89 #91 #95 #109 #110
  #112-#130 are resolved in the PR.
