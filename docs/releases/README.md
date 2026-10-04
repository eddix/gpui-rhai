# Release notes

| Version | Focus | Integration impact |
|---|---|---|
| [0.2.0](0.2.0.md) | Design system for productivity tools: neutral token runtime, token base, 62 rebuilt components, layouts and patterns, composition audit and profiles, the Gallery as acceptance application | Runtime API 3; add the token base for official components; review removed props and changed defaults |
| [0.1.8](0.1.8.md) | Unified Interaction Runtime, direct-manipulation components, Host policy/theme queries and Table/Ref convergence | Upgrade all three crates and copied Table source together; review Rust/key breaking changes; Runtime API stays 2 |
| [0.1.7](0.1.7.md) | Formal Gallery, Operations Workbench, source-backed acceptance stories | Replace removed Gallery examples; adopt optional Host environment APIs and Table search/page props |
| [0.1.6](0.1.6.md) | GPUI package-family migration and native AccessKit projection | Rust Host/MSRV migration to the exact gpui-pre 0.3.7 family |
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
cargo install gpui-rhai-cli --version 0.1.8 --locked --force
gpui-rhai update
gpui-rhai check
```

Runtime API and crate semver are separate contracts. 0.1.3 introduced Runtime
API 2; versions 0.1.4 through 0.1.8 retain it. 0.2.0 introduces Runtime API 3.
