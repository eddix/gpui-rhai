# Motion review reproduction

Audited commit: `9f4ba04fc9c5198d7164359f47139a5d306de1bc`.

Run the public API probes from the repository root:

```sh
bash docs/audits/2026-09-08/run-probes.sh "$PWD/docs/audits/2026-09-20-motion-runtime-2/probes.rs"
```

The existing runner creates an offline temporary consumer using the current
workspace core/CLI crates and lockfile. The probes print incorrect behavior at
the audited commit; they are characterization evidence, not passing correctness
assertions. They do not modify production files.

Run the actual GPUI mount/layout characterization:

```sh
bash docs/audits/2026-09-20-motion-runtime-2/run-native.sh
```

The two native tests deliberately assert that the current defects occur. A green
test result means the None-policy layout violation and Motion Gallery mount
failure were reproduced. Flip these to correct expectations when fixing them.
They use GPUI's test platform, not screenshots of the physical desktop.

The None-policy test advances a ManualRuntimeClock and uses a no-op repeat of
the same state-setting action to force a fresh frame at 500 ms. It compares
committed layout and visual bounds. The Gallery test extracts the actual MAIN
source from the product example and loads its real component/theme/locale graph.

Original baseline checks:

```sh
cargo test --workspace --all-targets --all-features --locked --offline
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
```

This review did not perform a new 120 Hz benchmark, image-diff certification,
IME/AX certification, all-platform GPU run, package install, or release smoke.
No GitHub CI run was returned for the exact reviewed commit at query time.
