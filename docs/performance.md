# Performance budgets

The budgets are diagnostics, not permission to move per-frame policy into Rhai.

- `view`, event callbacks, and capability delivery warn at 16 ms by default.
- Every compile/render/lifecycle/callback `ExecutionTiming` records wall time,
  success, slow-threshold state, and Rhai's actual operation counter. Imported
  component/helper calls are included in their outer render/callback total;
  Inspector shows the latest operation counts beside duration.
- Rhai execution is capped at 1,000,000 operations, 64 call levels, bounded
  expression depth, 10,000 array entries, 100,000 aggregate map fields, and
  1 MiB strings.
- Declarative timers are one-shot, component-scoped, capped by
  `RuntimeBudgets::timers`, and polled with other foreground deliveries; they do
  not create one OS thread per timeout.
- A 5,000-item `VirtualList` must realize only the viewport plus configured
  overscan; tests enforce bounded realization and stable focus.
- A 10,000-row scalar Table must retain row maps as data and realize only the
  vertical viewport plus overscan. Header/body horizontal scrolling must not
  trigger Rhai evaluation.
- Table custom-cell probes separately record eager Rhai node generation and
  bounded GPUI element realization; reports must not conflate the two.
- Textarea large paste/delete and width-change auto-grow probes must settle in
  one subsequent layout without height oscillation. Caret scrolling and IME
  bounds stay proportional to laid-out visual lines.
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

Complex-control reference run on 2026-08-30 in the same environment:

```text
compile=76µs  view_p95=3.73ms  conversion_1000_nodes_p95=1.88ms
table_scalar_10000=4.57ms  table_realized=20
table_custom_nodes_10000=10.88ms
```

The custom number includes eager construction of 10,000 `UiNode` values and
Table validation; it is intentionally separate from the 20-row native
realization count.

This result is evidence for the initial thresholds, not a cross-machine
benchmark guarantee.
