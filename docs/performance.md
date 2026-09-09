# Performance budgets

The budgets are diagnostics, not permission to move per-frame policy into Rhai.

- `view`, event callbacks, and capability delivery warn at 16 ms by default.
- Every compile/render/lifecycle/callback `ExecutionTiming` records wall time,
  success, slow-threshold state, and the runtime's cumulative Rhai operation
  count. Semantics version 2 aggregates nested evaluators; delayed callbacks
  start with fresh quota even when their stored module context was captured
  late in an earlier render. Incremental component and virtual-item calls align
  the tracker to each stored context's absolute counter while preserving the
  enclosing session total, so parent history is not recharged and sibling work
  cannot evade the aggregate cap. Imported component/helper calls are included
  in their outer render/callback total; Inspector shows the latest counts beside
  duration.
- Rhai execution is capped at 1,000,000 operations, 64 call levels, bounded
  expression depth, 10,000 array entries, 100,000 aggregate map fields, and
  1 MiB strings.
- Declarative timers are one-shot, component-scoped, capped by
  `RuntimeBudgets::timers`, and polled with other foreground deliveries; they do
  not create one OS thread per timeout. Timers and animation share a monotonic
  `RuntimeClock`; deterministic probes inject and advance `ManualRuntimeClock`
  instead of sleeping.
- Generic portal Layers, retained Canvas scenes, and Canvas command/segment
  complexity have independent Host-configurable limits; candidate trees cross
  all three gates before commit.
- Store fields, locale reads, discrete viewport classes, and retained geometry
  are exact formal-component dependencies. Native-only locale direction repaint
  is queued separately, so it does not force an otherwise clean Rhai root.
- A 5,000-item `VirtualList` must realize only the viewport plus configured
  overscan; tests enforce bounded realization and stable focus.
- A 10,000-row scalar Table must retain row maps as data and realize only the
  vertical viewport plus overscan. Header/body horizontal scrolling must not
  trigger Rhai evaluation.
- Native Table grouping may perform one O(n) partition when sort/group/collapse
  policy changes, but the flattened order and sticky indices must be cached.
  Selection-only updates reuse that order and continue projecting only the
  visible rows plus group headers.
- CodeViewer and DiffViewer parse immutable text snapshots on cancellable
  background jobs and render only visible document rows. Default Host limits
  are 10 MiB/500,000 lines per document, 20 MiB combined diff input, 100,000
  hunks and a two-second diff deadline. Rhai's separate 1 MiB string limit still
  applies to direct script strings; use `NativeTextDocument` above that size.
- Textarea large paste/delete and width-change auto-grow probes must settle in
  one subsequent layout without height oscillation. Caret scrolling and IME
  bounds stay proportional to laid-out visual lines.
- The representative 1,000-node probe records cold compile plus p95 cached view
  and `UiNode`-to-element conversion over 50 iterations.

Run on a release build:

```text
cargo run --release -p gpui-rhai --example performance_probe
```

## Interactive 1,000-row Table baseline

`table_1000` is the end-to-end interactive baseline for the official copied
Table component:

```text
cargo run --release -p gpui-rhai --example table_1000
```

The workload is intentionally fixed at 1,000 deterministic rows and seven
columns. Rust registers one immutable keyed `NativeCollection`; Rhai declares
the official copied-source Table, columns, controlled sort/selection state and
callbacks. The native data plane caches sort orders and projects normalized
cell payloads only for the `virtual_collection` viewport plus overscan. This
measures the intended large-data architecture rather than hiding an eager Rhai
pass before the timer starts.

Use these repeatable interactions when comparing changes:

1. cold launch until the first complete table frame;
2. scroll vertically from the first row to the last row and back;
3. resize the window across the table's horizontal-overflow breakpoint, then
   scroll across the full column width;
4. press **Re-render same data** repeatedly to measure unchanged-data rebuilds;
5. press **Reverse 1,000 rows** repeatedly to exercise keyed moves and diffing;
6. select and clear row 500 to measure a small controlled-state update against
   the same 1,000-row input.

Column-drag performance is kept as a separate workload so this historical
baseline remains comparable. Run the `data_table` example, drag a header
divider vertically outside the header, and verify that preview remains smooth
with no Rhai callback before the single committed `column_resize` event.

Run release builds on the same machine, display configuration, power state,
and window size. Record the commit, Rust toolchain, macOS version, hardware,
and whether the process was sampled under Instruments. Development Inspector
timings and virtual-collection metrics are useful for attribution, but debug
or `dev-reload` runs are not comparable performance numbers.

## Automated end-to-end baseline

The release benchmark mounts the same `table_1000` fixture in GPUI's test
platform and measures cold prepare/mount plus unchanged rerender, reverse,
selection, and native resize scenarios. It also records a document report that
compares the direct-string and NativeTextDocument Rhai boundaries, native syntax
tokenization, and two-way diff/hunk construction over deterministic Rhai source:

```text
bash scripts/benchmark.sh
```

Override the sample counts or write the complete JSON report when needed:

```text
GPUI_RHAI_BENCH_SAMPLES=50 \
GPUI_RHAI_BENCH_WARMUP=10 \
GPUI_RHAI_BENCH_OUTPUT=/tmp/gpui-rhai-table-1000.json \
bash scripts/benchmark.sh
```

Set `GPUI_RHAI_DOCUMENT_BENCH_LINES` to change the document workload. The
default is 20,000 lines. `gpui-rhai-document-e2e-v2` reports byte/line/hunk
counts plus direct-string Rhai prepare, native-document Rhai prepare, Rust
highlight, and Rust diff durations. It also mounts real CodeViewer and
DiffViewer instances, waits for their background work and first complete frame,
then records foreground dispatch p95 and complete-projection p50/p95 for
viewport-wrap resize. It deliberately includes
Rhai execution and native presentation work; the core diff timer alone is not
presented as end-to-end performance.

Document v2 reference run on 2026-09-06, Macmini9,1, macOS 26.6.2,
`rustc 1.94.0`, release profile, 20,000 generated Rhai lines / 1,008,890
bytes and 10 resize samples:

```text
direct-string Rhai prepare          =  46.69ms
NativeTextDocument Rhai prepare     =   1.10ms
native syntax tokenization          = 289.61ms
native two-way diff                 = 598.47ms; 21 hunks / 20,001 aligned rows
native UI prepare                   =   1.60ms
native UI mount + complete frame    = 944.81ms
resize dispatch p95                 =   4.66ms
resize complete-projection p50/p95  =  26.76ms / 27.09ms
```

The mounted workload contains one 20,000-line CodeViewer and one split
20,000-row DiffViewer. Resize dispatch is the foreground responsiveness metric;
complete-projection latency includes the cancellable background wrap projection
and atomic repaint. The old complete surface remains usable while that second
measurement is in flight.

The native boundary is the intended path for large or frequently replaced
documents. Highlight and diff durations run off the GPUI/Rhai foreground and
commit atomically; these numbers are latency baselines, not foreground frame
budgets. The direct-string result intentionally includes parsing a roughly
1 MiB Rhai literal and quantifies why that convenience path should remain for
ordinary-sized source rather than bulk Host data. The process-global syntax
pack is warmed before the direct/native Rhai comparison so its one-time load
does not unfairly charge only the direct-string branch.

Each `gpui-rhai-e2e-v2` report retains raw samples and p50/p95/p99 summaries
together with commit, dirty state, Rust/macOS/hardware metadata, retained node
counts, data backend, and virtual-collection data/realization metrics. Rhai
duration and operation totals are split into root/component work and delayed
virtual-item work. Persisted `NativeCallContext` calls begin a new execution
session from their stored absolute counter; only newly observed work consumes
that session. Samples also report `reused_component_subtrees` and
`reused_components`. The unchanged-data scenario requires the four stable
buttons and Table to survive a root-state rerender through component bailout.

Resize is a structural gate: it may execute virtual item renderers for rows that
newly enter a taller viewport, but it must not rerun the root or ordinary
components. Shared CI must not enforce absolute wall-clock thresholds; use a
controlled Mac for timing comparisons.

Reference native-collection run on 2026-09-02, Macmini9,1, macOS 26.6.2,
`rustc 1.94.0`, release profile, 5 warmups and 30 samples:

```text
cold mount+first frame = 31.78ms; root=4.23ms/255 ops; virtual=3.11ms/15096 ops
unchanged p95 = 13.99ms; root=2.27ms/266 ops; virtual=2.12ms/10064 ops
reverse p95   = 14.76ms; root=2.19ms/287 ops; virtual=2.57ms/12580 ops
selection p95 = 15.15ms; root=2.25ms/265 ops; virtual=2.57ms/12580 ops
resize p95    = 11.68ms; root=0; virtual=0.47ms/1887 ops
```

After root-dirty formal-component bailout, the same machine/toolchain with 5
warmups and 30 samples produced:

```text
unchanged p95 =  5.03ms; root=0.86ms/266 ops; virtual=0; reused=5
reverse p95   = 13.03ms; root=1.83ms/287 ops; virtual=2.35ms/12580 ops; reused=4
selection p95 = 12.97ms; root=1.79ms/265 ops; virtual=2.34ms/12580 ops; reused=4
resize p95    = 10.41ms; root=0; virtual=0.41ms/1887 ops; reused=0
```

The unchanged-data end-to-end p95 fell by about 64%, and its measured Rhai
root/native-call duration fell by about 62%; every sample reused exactly the
four Button instances and Table. Operation counts do not fall because Rhai's
progress counter counts the `render_component` native calls themselves, while
the removed work is the Rust-hosted component render reached through each call.

The report was produced from a dirty development tree and is an architectural
checkpoint, not a release guarantee. Compare only reports whose
`data_backend` and environment metadata match.

The 0.1.1 node-prop ownership fix was checked with alternating current-main and
candidate release binaries on the same Macmini9,1, using 5 warmups and 30
samples. A mandatory per-component owned-tree snapshot prototype caused a
repeatable 13–17% selection regression and was rejected. Lazy snapshots,
activated only by node transport or outer presentation, produced:

```text
scenario       candidate p95   adjacent main p95 range
unchanged          5.06ms          4.96–5.10ms
reverse           11.25ms         11.10–11.15ms
selection         20.40ms         20.14–20.88ms
native resize     12.74ms         13.22–13.29ms
```

The unchanged candidate retained 26 realized rows and performed zero virtual
Rhai work. This A/B guards both the foreground clone cost and synchronization of
later virtual realization into an active component-owned snapshot.

Record toolchain, hardware, and power state when comparing results. The 16 ms
foreground threshold is enforced as a trace warning; the standalone probe is
advisory until CI has a dedicated, uncontended performance runner.

Reference run on 2026-08-28, `rustc 1.94.1`, aarch64 macOS, release profile:

```text
compile=387µs  view_p95=4.50ms  conversion_1000_nodes_p95=2.39ms
```

Post-runtime-v2 policy/reconciler probe on 2026-08-31, aarch64 macOS 26.6.2,
`rustc 1.94.1`, release profile:

```text
variable_height_policy_100000=33.11ms realized=39 total=3247064
retained_reverse_reorder_2000=3.46ms preserved=2001 moved=2000
```

This probe measures deterministic policy construction/measurement and retained
diff only. It is not the required 120 Hz Mini Timeline total-frame measurement;
GPUI flush/layout/paint and interaction p95 remain a separate certification
gate.

This result is evidence for the initial thresholds, not a cross-machine
benchmark guarantee.
