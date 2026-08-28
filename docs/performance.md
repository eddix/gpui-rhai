# Performance budgets

The budgets are diagnostics, not permission to move per-frame policy into Rhai.

- `view`, event callbacks, and capability delivery warn at 16 ms by default.
- Rhai execution is capped at 1,000,000 operations, 64 call levels, bounded
  expression depth, 10,000 array entries, 100,000 aggregate map fields, and
  1 MiB strings.
- A 5,000-item `VirtualList` must realize only the viewport plus configured
  overscan; tests enforce bounded realization and stable focus.
- The representative 1,000-node probe records cold compile plus p95 cached view
  and `UiNode`-to-element conversion over 50 iterations.

Run on a release build:

```text
cargo run --release -p gpui-rhai --example performance_probe
```

Record toolchain, hardware, and power state when comparing results. The 16 ms
foreground threshold is enforced as a trace warning; the standalone probe is
advisory until CI has a dedicated, uncontended performance runner.

Reference run on 2026-08-28, `rustc 1.94.1`, aarch64 macOS, release profile:

```text
compile=387µs  view_p95=4.50ms  conversion_1000_nodes_p95=2.39ms
```

This result is evidence for the initial thresholds, not a cross-machine
benchmark guarantee.
