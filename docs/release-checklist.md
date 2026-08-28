# Release checklist

1. Add migration notes for every breaking runtime API, component schema, registry
   baseline, manifest, or generated-source change.
2. Run format, all-target/all-feature check, strict Clippy, tests, and rustdoc.
3. Test the declared MSRV (`1.94`) and latest stable toolchains.
4. Run `cargo package -p gpui-rhai` and inspect the package file list.
   The pinned GPUI HTTP dependency currently locks a yanked `chacha20 0.10.1`;
   packaging verifies successfully but emits a warning until upstream updates.
5. Re-run the full Cargo metadata license matrix; investigate unknown licenses
   and update `THIRD_PARTY_LICENSES.md` for copied source or assets.
6. Build every example in release mode and run `scripts/release-smoke.sh` on
   macOS with the Metal Toolchain installed.
7. Run `scripts/audit-visual-baselines.sh` and complete the unlocked keyboard,
   focus, IME, clipboard, overlay, and multi-window interaction matrix.
8. Run `scripts/audit-release-artifacts.sh` to reject workspace paths and
   development-only inspector strings in embedded example binaries.
9. Run CLI clean and modified-project fixtures for init/add/check/dev metadata,
   diff/update, and embed behavior.
10. Record the exact GPUI/Rhai versions and known accessibility/platform gaps.

The `0.1.0` public surface is intentionally pre-1.0, but changes are never
silently breaking: the changelog and source-update baseline are the migration
contract.
