# Contributing to GPUI Rhai

GPUI Rhai is being built against the contracts in `INTENT.md` and the delivery
order in `IMPLEMENTATION_PLAN.md`. Changes should move those contracts forward
without exposing GPUI context or element types to Rhai.

## Local requirements

- stable Rust as selected by `rust-toolchain.toml`;
- Xcode and the optional Metal Toolchain on macOS;
- no dependency, direct or optional, on `gpui-component`.

Install the Metal compiler with:

```text
xcodebuild -downloadComponent MetalToolchain
```

Before submitting a change, run:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc -p gpui-rhai --all-features --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc -p gpui-rhai-cli --lib --no-deps
cargo package -p gpui-rhai
bash scripts/release-smoke.sh
bash scripts/audit-release-artifacts.sh
```

Public runtime decisions listed in the ADR section of
`IMPLEMENTATION_PLAN.md` require a short decision record before their API is
treated as stable.
