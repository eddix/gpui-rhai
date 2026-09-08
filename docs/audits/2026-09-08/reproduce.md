# Reproducing the audit evidence

Baseline: `d4a7901b26fa91311f3228d768037e1d52531e16`.

These are characterization probes, not a passing correctness suite. Their
output demonstrates existing defects. Convert each relevant case into a
regression assertion for the intended contract when implementing a fix.

Run from any directory:

```sh
bash /path/to/gpui-rhai/docs/audits/2026-09-08/run-probes.sh
```

The runner creates a separate temporary Cargo project, uses the repository's
lockfile as its resolution baseline and compiles against the current workspace crates. It uses
offline Cargo and the workspace target directory. Cached dependencies and a
working platform/Metal toolchain are required. It does not change production
source, install packages, or access real user data. Filesystem failure probes
operate on new, process-specific temporary fixtures. Temporary directories are
printed or identifiable by `gpui-rhai-audit` prefixes and retained for inspection.

The maintainer explicitly accepts breaking changes in 0.1.1. A07 is withdrawn;
the old-version Button probe has been removed. Historical compatibility is not
a correctness requirement for this audit.

Expected evidence on the audited commit:

| Output | Meaning |
| --- | --- |
| `default_resolver_absolute_import_accepted=true` | Default runtime imported a probe-owned absolute path |
| `const_map_panicked=true` | `catch_unwind` caught the pinned Rhai defect |
| `cancel_rollback_subscription_active=0` | Restoring an earlier snapshot did not restore cancellation |
| `worker_panic_deliveries=0 active=1` | Terminated worker left an active task record |
| `nan_ui_value_accepted=true ... roundtrip=false` | Durable value validation and serialization disagree |
| Both `lexer_..._rhai_compile=true` plus import scan error | Valid Rhai was rejected by the custom scanner |
| `cli_partial_apply_failed=true cargo_already_changed=true` | An error left earlier planned writes applied |
| `nested_800000_iterations_accepted=true recorded_operations=34` | Nested evaluator work was not aggregated |
| `failed_preload=true ... no registered render function` | Preparation failure after clearing registrations breaks old code |
| `effect_swap_failed=true old_tree_restored=true old_subscription_active=0` | A rejected component replacement restored the UI tree but lost its old owned subscription |
| Heavy parent plus callback `ErrorTooManyOperations` | Delayed callback retained the parent's used budget |

The intentional panic messages in stderr are part of the evidence. The probe
isolates the current unwind case to continue collecting results; this is not a
recommendation to catch arbitrary panics and continue using a runtime.

## First-principles second pass

```sh
bash docs/audits/2026-09-08/run-probes.sh "$PWD/docs/audits/2026-09-08/second-pass.rs"
```

The second pass tests shared-runtime multi-window geometry/capture, component
incarnation, independent program candidates, pause reasons and timer curry,
rejected provider registration, deserialized schema validation, Host signal
values, and the built-in subscription receiver adapter. See
[first-principles.zh-CN.md](first-principles.zh-CN.md) for expected observations
and [second-pass.log](evidence/second-pass.log) for results. Geometry is explicitly
submitted through public APIs; this is not fresh native multi-window input
certification. The receiver adapter probe uses a real worker and bounded polling
for disconnect; the timer checks themselves use explicit instants without sleep.

## Existing verification commands used

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets --all-features --locked --offline
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline
RUSTDOCFLAGS='-D warnings' cargo doc -p gpui-rhai --all-features --no-deps --locked --offline
RUSTDOCFLAGS='-D warnings' cargo doc -p gpui-rhai-cli --lib --no-deps --locked --offline
cargo package -p gpui-rhai --locked --offline
bash scripts/audit-visual-baselines.sh
GPUI_RHAI_BENCH_SAMPLES=10 GPUI_RHAI_BENCH_WARMUP=3 \
GPUI_RHAI_BENCH_OUTPUT=/tmp/gpui-rhai-audit-table.json \
bash scripts/benchmark.sh
```

The package command was run before adding these report files. Cargo may require
committed changes or an explicitly chosen dirty-package workflow on a modified
checkout. Do not change publication settings just to reproduce the audit.

## Release microbenchmarks

`release-micro.rs` measures state mounting and snapshots through public APIs.
It does not create a GPUI window or measure full UI frames. The recorded run
linked the core rlib freshly rebuilt by `tests/performance` at the audited
commit, using `rustc --edition=2024 -O`, `--extern gpui_rhai=<rlib>` and
`-L dependency=tests/performance/target/release/deps`.

For an ordinary reproducible build, use the temporary project created by the
runner, replace its `src/main.rs` with `release-micro.rs`, and run `cargo run
--release --offline --manifest-path <temporary-project>/Cargo.toml`. Declare the
feature set when comparing results; the original release microbenchmark used
the default-feature core built by the performance workspace, while the main
characterization runner uses all runtime features.

The first audit attempt to compile the temporary consumer hit a restricted
default Clang cache directory. Setting `CLANG_MODULE_CACHE_PATH` to the workspace
cache resolved it. This was an environment issue, not a project test failure.

## Evidence limits

The logs establish the listed checks and observations only. They do not certify
fresh screenshots, native IME or platform accessibility, hosted Linux/Windows
execution, an uncontended hardware performance baseline, 120 Hz frame times, or
registry/CLI clean package installation. The Table report retains machine
metadata; its `hardware` is `unknown`. Document timings are captured from the
same benchmark invocation. Operation-count semantics are themselves an audited
defect, so use those counts only to reproduce that finding until it is fixed.
