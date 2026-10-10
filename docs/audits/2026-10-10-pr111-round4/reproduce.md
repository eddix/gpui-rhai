# Reproduce PR #111 round 4

Reviewed commit: `95253c65d5f97a1e64ca51567255e6337ae02c96`.

Use an isolated checkout of that commit and the pinned Rust 1.95 toolchain. The
native tests use GPUI test support; the recorded results are from macOS. Download
this material directory intact, so runner-relative Rust sources are present.

```sh
python3 run-probes.py --repo /absolute/path/to/gpui-rhai --target-dir /path/to/reusable/native-target
```

At the reviewed head the result is **2 passed / 8 failed**, Cargo exit 101. The
assertions describe desired behavior; failures are expected before remediation.
The runner temporarily creates two native test files and removes only its own
unchanged copies in `finally`. It does not edit product source. It removes
`RUST_MIN_STACK` from the child environment. Do not run it while another Cargo
invocation is discovering all native test targets. `--offline` requires cached
dependencies; fetch the repository's locked dependencies first on a new machine.

`issue131.rs` is the reporter's original public reproduction, extracted from
GitHub issue #131. `probes.rs` uses the checked-in TabBar fixture's mount helpers
with independent empty/close/reorder/list-size scenarios, real metadata checks,
and installed skill recipes in fresh temporary CLI projects.

The skill recipe test uses the bundled skill bytes and the CLI's Project APIs
(`plan_init`, `plan_add`, `plan_skills`, `check`). This validates the same source
and operation as the corresponding CLI commands without building a separate CLI
binary. The SKILL.md Button example is a passing control. All four recipe blocks
fail in independent fresh projects. It does not compile their generated Rust
hosts against crates.io; the candidate version bump remains a separate gate.

Historical regression replay (unchanged sources):

```sh
python3 historical/run-probes.py --repo /absolute/path/to/gpui-rhai --target-dir /path/to/reusable/native-target
```

Expected **10/10 pass** on this head. This enables `gpui-rhai/dev-reload` for the
hot-reload regression; it also adds and removes temporary test files.

Product checks executed from the reviewed checkout:

```sh
env -u RUST_MIN_STACK cargo test --workspace --all-targets --all-features --locked --offline
env -u RUST_MIN_STACK cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked --offline -- --test-threads=1
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
cargo package --manifest-path registry/Cargo.toml --list --locked --offline
git diff --check d87ebb9f781ecaaa53e265d39b9cacc7ded07f25..95253c65d5f97a1e64ca51567255e6337ae02c96
```

The workspace/native logs record 695/331 passes respectively. Separate local
`CARGO_TARGET_DIR` caches were used to avoid rebuilding dependencies. No custom
thread stack size was used.

Stored visual comparison (requires Pillow):

```sh
python3 compare-baselines.py --repo /absolute/path/to/gpui-rhai --base 2c5e8a079304bb0c8910ddbd73eca772f49fd453 --head 95253c65d5f97a1e64ca51567255e6337ae02c96
```

This reads committed PNG blobs in RGB with a known-different and identical-image
control. It does not create screenshots or edit any images. Result: 43 changed
pairs out of 57; all 14 Dashboard pairs unchanged.

The two skill entrypoints were also checked with the local `skill-creator`
`quick_validate.py`: both passed. All 167 relative file links and five local
anchors resolved; `cargo package --list` includes all 105 skill Markdown files.
These structural checks do not replace the failing behavior checks above.
