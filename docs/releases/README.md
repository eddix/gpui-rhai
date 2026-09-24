# Release notes

| Version | Focus | Integration impact |
|---|---|---|
| [0.1.0](0.1.0.md) | First public runtime, CLI, registry, themes and 50 components | Initial adoption |
| [0.1.1](0.1.1.md) | Runtime identity, transactions, async ownership, component styles | Replace the removed global `component_style` helper |
| [0.1.2](0.1.2.md) | Frozen 51-component foundation, Table resizing, accessibility | Add explicit labels required by eleven components |
| [0.1.3](0.1.3.md) | Motion Runtime 2 | Runtime API 2; migrate removed animation prototypes |
| [0.1.4](0.1.4.md) | SVG/image pipeline, Host slots, adaptive overlays | No Runtime API migration |
| [0.1.5](0.1.5.md) | Native Chart Runtime and complete theme-driven UI | Add `spacing.xxs` to custom themes; typed chart viewport recommended |

All three crates use the same version. Update the Rust dependency, installed
registry snapshot and CLI together so component schemas and runtime behavior do
not drift.

```bash
cargo update -p gpui-rhai
cargo install gpui-rhai-cli --version 0.1.5 --locked --force
gpui-rhai update
gpui-rhai check
```

Runtime API and crate semver are separate contracts. 0.1.3 introduced Runtime
API 2; versions 0.1.4 and 0.1.5 retain it.
