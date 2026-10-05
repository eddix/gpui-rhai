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
  `operation_limit` records the effective quota at the start of that execution;
  `round_operations` records its enclosing round's cumulative consumption.
  Structured errors report these values, not an assumed default or one
  component's timing delta. `operations` keeps its existing timing-span meaning.
- Rhai execution is capped at 1,000,000 operations by default (host-configurable
  via the view builders' `operation_limit`), 64 call levels, bounded
  expression depth, 10,000 array entries, 100,000 aggregate map fields, and
  1 MiB strings.
- Declarative timers are one-shot, component-scoped, capped by
  `RuntimeBudgets::timers`, and polled with other foreground deliveries; they do
  not create one OS thread per timeout. Timers and Motion share a monotonic
  `RuntimeClock`; deterministic probes inject and advance `ManualRuntimeClock`
  instead of sleeping.
- Generic portal Layers, retained Canvas scenes, and Canvas command/segment
  complexity have independent Host-configurable limits; candidate trees cross
  all three gates before commit.
- Motion declarations, active sources, keyframes, timelines, timeline steps,
  exit ghosts, shared-layout snapshots, and particles have independent
  `RuntimeBudgets`. Crossing a limit rejects the candidate/operation; the
  runtime never silently drops random particles or callbacks. High/medium/low
  quality is an explicit Host-selected input.
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

## Frame cost

GPUI lays out the whole window on every frame it draws, including frames that
only repaint (hover, scrolling, a caret). Two rules keep that cheap:

- **Stretched children have a definite width.** A child of a column that
  stretches its children, and has no width of its own, is rendered with
  `width: 100%`. Without it Taffy measures the child at its content width and
  then lays it out again at the stretched width, and the two passes compound at
  every nesting level: a `ListDetail` inside a page inside an `AppShell` took
  28 ms per frame and takes 0.4 ms. The output is pixel-identical (verified on
  all 57 baselines and 415 Gallery page renders).
- **Nodes are shared.** `UiNode` is a copy-on-write handle: Rhai passes nodes
  by value (variables, arrays, arguments), and copying subtrees was half of a
  render. Cloning a node is now a reference-count increment.
- **Host audits run after renders, not after frames.**
  `ScriptViewHandle::committed_revision` advances only when a render commits;
  the Gallery Host audits once per revision. Font enumeration for the
  `unresolved-font` rule (about 100 ms on macOS) is cached and skipped when the
  rule is off.

`tests/native-keyboard/src/bin/gallery_profile.rs` measures the Gallery with the
real renderer, offscreen: per interaction the Rhai work, the CPU frame time
(`Window::draw`) and the audit, as p50 / p95.

```sh
cd tests/native-keyboard
cargo run --release --bin gallery_profile -- 20
GALLERY_PROFILE_PAGES=1 cargo run --release --bin gallery_profile -- 3   # per-page frame cost
GALLERY_PROFILE_RHAI=view.rhai cargo run --release --bin gallery_profile # any view, all registry modules
```

Measured on an Apple Silicon Mac (release, 2026-10-05), before and after:

| | Before | After |
|---|---:|---:|
| idle frame, Button page | 19.7 ms | 1.9 ms |
| idle frame, `list_detail` page | 52.2 ms | under 2 ms |
| audit per frame in the Gallery | 103 ms | not run (0.1 ms after a render) |
| mount and first frame | 453 ms | 160 ms |
| Rhai render, navigate to a page | 15 ms | 7.4 ms |
| Rhai render, navigate to the 120-row data scene | 22 ms | 13 ms |
| Rhai work per keystroke in a filter | 10.6 ms | 5.1 ms |
| Rhai render, light/dark switch | 21 ms | 14 ms |

## Trusted Host execution policy

Ordinary applications keep `DEFAULT_SCRIPT_OPERATION_LIMIT = 1_000_000` and
parser depths global=64/function=32 in both debug and release. A trusted Host
can opt into different policies for its known workload:

```rust,ignore
let prepared = gpui_rhai::FileScriptView::new("ui/main.rhai")
    .operation_limit(4_000_000)
    .expression_depth_limits(64, 64)
    .prepare()?;
```

`EmbeddedScriptView` has the same builders. Direct `RuntimeEngine` users call
`set_operation_limit` and `set_expression_depth_limits`. Each zero value
normalizes to one, never unlimited. These are Rust-only settings: scripts cannot
raise their own quota. Builder policy is applied before trusted
`ScriptViewExtension::configure_engine`; an extension may override it. Secondary
engines use the same ordering, and reload candidates copy the active values
into independent storage. A failed candidate leaves the active policy intact.

Operation quota, parser expression depth, recursive call depth and durable data
limits are independent. Raising one does not raise the others. Nested/sibling
work shares one cumulative execution quota; retained and delayed entry points
start fresh rounds rather than recharging parent history. Parser policy must be
set before compilation; operation policy applies to subsequent evaluation.

An operation count is not a latency guarantee or native-code timeout. A larger
budget can allow a long foreground calculation, and the 16ms warning only
reports it afterwards. Native Rust calls remain cooperative and cannot be
preempted by the Rhai quota. Keep high-frequency layout/interaction policy in
Rust and cache expensive script results; do not raise the global default to
accommodate one application.

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

The same harness emits `gpui-rhai-chart-e2e-v2`: it mounts the source-backed
`charts/streaming` story, lays out and paints a 100,000-row `NativeChartData` line series,
alternates native window sizes, and measures bounded 128-row sliding-window
revisions through foreground scene installation. Version 2 asserts that the
target `NativeChartData` revision is actually installed before recording a
sample; earlier reports could stop after scheduling background work. Set
`GPUI_RHAI_CHART_BENCH_POINTS` to change the point count. Compare chart reports
only when theme, window/display configuration, feature set, and point count
match.

Chart acceptance baseline on 2026-09-26, Macmini9,1, macOS 26.6.2,
`rustc 1.95.0`, release profile, 100,000 points, five warmups and 30 measured
samples. Motion is disabled so the report isolates data, layout, scene
preparation, and presentation:

```text
story prepare                           = 268.92ms
mount and terminal first frame          = 189.54ms
native resize p50 / p95                 = 131.04 / 154.61ms
128-row sliding update p50 / p95        = 160.12 / 187.07ms
Rhai operations after streaming updates =   0
```

This run used a busy development desktop and is a reproducible functional
baseline, not a cross-machine wall-clock threshold. The important architectural
invariant is that each acknowledged NativeChartData revision presents while
streaming performs zero Rhai operations.

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

## gpui-pre 0.1.6 candidate A/B

The 2026-09-25 backend comparison used the same Macmini9,1, macOS 26.6.2,
Rust 1.95.0, release profile, 5 warmups and 30 samples for all three points:
the v0.1.5 tag, the exact post-gpui-pre/pre-AX commit `202e6e73`, and the
native-AX candidate. Native AX element projection was inactive in the test
platform; the committed semantic frame and automation semantics remained live.

```text
                                      v0.1.5     pre-AX      AX candidate
Chart mount + first frame             112.05ms    123.79ms    119.83ms
Chart resize p95                       23.87ms     17.74ms     17.83ms
Chart 128-row stream p95               76.00ms     73.08ms     74.48ms
Document mount + complete frame       910.45ms    914.95ms    888.89ms
Document resize settle p95             25.12ms     24.04ms     24.10ms
Table cold mount + first frame         63.13ms     67.46ms     69.71ms
Table native resize p95                23.67ms     23.73ms     26.44ms
Table resize Rhai operations            3,417       3,417       3,714
```

The first AX implementation created semantic element wrappers even while the
platform adapter was inactive and reached a 32.75ms Table resize p95. Gating
native projection with `Window::is_a11y_active()` removed that architectural
regression. The remaining roughly 2.7ms / 11.4% Table resize delta is accounted
for by bounded realized cell/row semantics: root Rhai operations remain zero,
while 297 additional virtual operations create table cell roles and collection
positions. Typed compound semantic setters keep this below the initial 435-op
implementation without dropping cell navigation. Chart streaming still runs
zero Rhai operations.

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
