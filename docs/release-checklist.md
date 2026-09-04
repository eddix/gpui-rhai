# Release checklist

1. Do not begin compatibility or migration work until the maintainer gives an
   explicit release/version signal. Before that signal, dogfooding changes go
   directly to the best SDK design without aliases or dual parsers. Once a
   release is requested, document every breaking runtime API, component schema,
   locale, registry, manifest, and generated-source change from that baseline.
2. Run format, all-target/all-feature check, strict Clippy, tests, and rustdoc.
3. Test the declared MSRV (`1.94`) and latest stable toolchains.
4. Run `cargo package -p gpui-rhai --locked --all-features` and inspect the
   package file list. Package verification must use official registry Rhai,
   outside the workspace's experimental Git patch.
   The pinned GPUI HTTP dependency currently locks a yanked `chacha20 0.10.1`;
   packaging verifies successfully but emits a warning until upstream updates.
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
8. Run `scripts/audit-release-artifacts.sh` to reject workspace paths and
   development-only inspector strings in embedded example binaries.
9. Run CLI clean and modified-project fixtures for init/add/check/dev metadata,
   diff/update, and embed behavior.
10. Record the exact GPUI/Rhai versions and known accessibility/platform gaps.
    When evaluating Grain/JIT, run the unpublished experiment crate and strict
    Clippy separately; a compiling extension interface is not production parity.
11. For the complex-control line, run fixed-Clock DatePicker locale cases, large
    scalar/custom Table probes, Select group/search cases, Pagination boundary
    transitions, and Textarea multiline IME/auto-grow cases.

The repository remains version 0.1.0 and `RUNTIME_API_VERSION` 1 during the
current dogfooding expansion. A later explicit release signal establishes the
baseline from which changelog and source-update migration contracts apply.
