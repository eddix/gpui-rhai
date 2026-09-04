# Multi-type JIT validation — 2026-09-03

Implementation and repository validation are complete. The host maintainer
subsequently reported a successful manual dogfooding run; see the separate
[application checkpoint](jit-dogfooding-2026-09-03.md). This report preserves the
original release probes and does not claim a measured whole-application improvement.

## Functional checks

`cargo test --workspace --all-features --no-fail-fast` passed:

| Target | Passed |
| --- | ---: |
| gpui-rhai unit tests | 268 |
| Custom primitive integration | 1 |
| Formal component integration | 15 |
| Official component registry | 33 |
| CLI unit tests | 20 |
| JIT library tests | 14 |
| Original JIT probe tests | 13 |
| gpui-rhai backend adapter integration | 2 |
| Multi-type/ownership/reentry tests | 18 |
| Total | 384 |

Additional checks:

- Pinned Rhai fork `jit` revision `--lib grain::`: 131 passed (one unrelated
  test filtered) after rebasing the fork patch onto its current default branch.
- Before backend extraction, explicit `GPUI_RHAI_BACKEND=grain` runs passed all
  15 formal-component and 30 official-registry tests. These historical runs
  overlap the table; do not add them to its total. Current integration uses the
  unpublished `GrainScriptBackend` adapter instead of a core environment switch.
- Default backend build: `cargo check -p gpui-rhai --no-default-features --lib`
  passed.
- The independent default-path performance workspace builds against official
  Rhai and passes its enabled end-to-end preparation test. Its timing benchmark
  remains explicitly ignored unless invoked by `scripts/benchmark.sh`.
- Workspace formatting, diff whitespace checks and clippy with all targets,
  all features, locked dependencies and `-D warnings` passed.
- Merge-preparation public documentation builds for gpui-rhai (all features)
  and the CLI library passed with `RUSTDOCFLAGS='-D warnings'`.

The new integration coverage includes actual native execution of imported
component render functions, preserving subtree reuse, later event callbacks,
reentrant code eviction, shared operation limits, fresh retained-callback
budgets, map guards/getters, managed drops, panic containment and arithmetic
error text/position parity with Grain. It is not exhaustive Rhai-language or
platform coverage.

After backend extraction, the imported-render native assertion lives in the
unpublished adapter integration suite. Core subtree reuse remains covered by
the ordinary formal-component suite; a reused subtree does not produce a
second component timing stream or a synthetic JIT call.

## Release measurements

Raw data: [jit-multitype-results.json](jit-multitype-results.json).

Three separate forced-native runs and one adaptive-policy run were measured
on AArch64. Each backend gets 40 rotated/interleaved samples; each sample is
the average of 100 calls, after value-parity checks and warmup. The reported
percentiles are of those batch averages, not individual-frame UI latencies.
No builds ran concurrently with the recorded measurements.

First forced-native run, p50:

| General workload | Grain | Native tier | Result |
| --- | ---: | ---: | --- |
| Mixed integer/float calculation | 60.848 us | 3.507 us | 17.35x |
| Array/map/string processing | 7.625 us | 7.054 us | 7.49% lower time |
| Host-call-heavy construction | 15.987 us | 16.681 us | 4.34% higher time |

Across the three runs, calculation speedup was 17.06–17.35x; object-processing
time fell 7.49–8.02%. Host-call-heavy native execution remained 3.19–4.34%
slower. These samples explain why type support and profitability are separate
decisions, and why native-call counts alone are insufficient.

With the default profitability check enabled:

| Workload | Grain p50 | Adaptive p50 | End state |
| --- | ---: | ---: | --- |
| Calculation | 60.345 us | 3.609 us | compiled |
| Object processing | 7.495 us | 6.879 us | compiled |
| Host-call-heavy construction | 15.250 us | 15.455 us | unprofitable |

The last entry executed 476 native calls during trials instead of the 4,006
forced-native calls, then predominantly used Grain. Its measured policy
overhead was still about 1.34%; this is not a zero-overhead or universal
no-regression guarantee. Occasional reference/trial calls also affect p95.

Native compilation across the three forced runs was 30.9–137.8 us for the
calculation, 24.7–51.1 us for object processing and 14.3–21.9 us for the mixed
host case. These are single-compilation observations, not robust compilation
latency percentiles. AST/Grain compilation is recorded separately in JSON;
callback-binding setup and whole-application startup are not included.

The emitted functions contain 632, 736 and 880 instruction bytes respectively.
Each retains a 4,096-byte executable-buffer allocation in this run. These
capacity values exclude OS page rounding and other Rust/runtime allocations.

The original integer probe also passed value/guard/operation parity, now with
20,001 operations including backward edges. One release run measured its
guarded loop at 70.6 us versus fresh-VM Grain at 1.353 ms. Its tiny product
helper is a remaining overhead case: bare Grain measured 375 ns p50, the
manual tier 250 ns, and the general adaptive tier 500 ns. The adaptive routing
and observation cost is not amortized at that scale; this regression against
bare Grain is recorded rather than hidden by the larger-loop wins. It remains
a limitation to consider when evaluating the real application.

## Handoff

Merge preparation later showed that guarding individual calls was insufficient:
the advertised Grain feature still named fork-only types when packaged. The
published core now exposes only an `experimental-backend` trait boundary made
from official Rhai types. `cargo package -p gpui-rhai --locked --all-features`
is the required gate outside the workspace Git patch; the JIT/dynasm code and
all fork-specific symbols live in the unpublished experiment crate.

The production-shaped experiment attaches `GrainScriptBackend` to
RuntimeEngine with an AdaptiveFunctionTier handle. The
current defaults additionally check profitability and keep up to 64 Programs.
`unprofitable` means the function is implemented but not currently worthwhile;
`rejected` means unsupported or over the allocation limit.

The host's subsequent report supplies functional smoke results and new tier
statistics, with no host code adaptation. Its files were not changed here.
Cold-start/frame latency, release application profitability and long-session
memory remain future measurements; debug selftest timings do not answer them.
