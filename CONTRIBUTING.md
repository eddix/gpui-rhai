# Contributing to GPUI Rhai

GPUI Rhai is built against the contracts in `INTENT.md`, the decision records
in `docs/adr/` and the design specification in `docs/design/`. Changes should
move those contracts forward without exposing GPUI context or element types to
Rhai.

## Local requirements

- stable Rust as selected by `rust-toolchain.toml`;
- Xcode and the Metal Toolchain for local macOS GUI/release-smoke work;
- no dependency, direct or optional, on `gpui-component`.

Install the Metal compiler with:

```text
xcodebuild -downloadComponent MetalToolchain
```

Before submitting a change, run:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc -p gpui-rhai --all-features --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc -p gpui-rhai-cli --lib --no-deps
cargo package -p gpui-rhai
cargo build --workspace --all-targets --release
bash scripts/audit-release-artifacts.sh
bash scripts/audit-visual-baselines.sh
cargo test --manifest-path tests/native-keyboard/Cargo.toml
cargo clippy --manifest-path tests/native-keyboard/Cargo.toml --tests -- -D warnings
```

On an unlocked macOS machine with Metal installed, additionally run:

```text
bash scripts/release-smoke.sh
```

The hosted workflow uses only a standard Linux runner and avoids duplicate
push/PR runs. Screenshot, platform
IME, native accessibility, and real pointer/keyboard certification remain local
macOS gates.

A public runtime decision needs a short record in `docs/adr/` before its API is
treated as stable; a design-system decision goes to `docs/design/decisions.md`.
Audits, plans and other records of a working round are not kept in the tree:
merge what lasts into the maintained documents and list the round in
`docs/history.md`.
