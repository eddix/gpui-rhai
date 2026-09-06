# JIT dogfooding checkpoint — 2026-09-03

## Decision

Close the current implementation stage and propose merging it as an opt-in
experiment. The first real host reports working integration and no functional
regression in its selftest, but no useful JIT gain for its current UI workload.
Keep the default runtime independent of the Rhai fork. Do not enable JIT by
default or add application-specific specializations to manufacture a gain.

This is an experimental acceptance checkpoint, not production certification,
complete Rhai coverage, or a claim about whole-application release performance.

## Evidence and provenance

Source: the host maintainer's report supplied on 2026-09-03. The selftest and
application timings were not independently rerun in this repository. Full host
logs and frame-latency distributions were not supplied with that report.

- Host: omb GUI, AArch64 macOS, **debug build**.
- Runtime: main baseline `5b18dc1` plus the experiment branch and multi-type
  working-tree implementation described in [the repository results](jit-multitype-results.md).
- Integration: existing `OMB_GRAIN_JIT=1` / AdaptiveFunctionTier injection;
  no host code adaptation was needed for the expanded ABI.
- Workload: table fetch/selection/linkage, palette keyboard interaction,
  repeated floating windows, CSV export and panel switching.
- Functional result: **28/28 assertions in both default and JIT modes**.
- Host disposition: keep its experiment feature off by default and return its
  ordinary dependency to main; re-enable the experiment for future workloads.

### A formerly rejected function now compiles, runs and demotes

```text
entry_row(2) [unprofitable] grain=134 jit=34 guard_miss=0 compile=1
             self=58.401504ms jit_time=14.826917ms args=[map,bool]
```

This confirms native execution of a real function with managed arguments,
previously rejected by the integer-only lane, followed by profitability-based
demotion. No entry guard misses were reported for this specialization.

Dividing the reported totals by their call counts gives approximately 435.8 us
per Grain call and 436.1 us per native call. These aggregate means are not the
rolling medians used by the policy, are not paired same-input measurements,
and do not include all tier-selection/library-compilation overhead. They are
consistent with the reported demotion, not an independent proof of its exact
decision threshold or whole-application overhead.

The host attributes the lack of benefit to native node construction dominating
this workload. That explanation is consistent with our release host-heavy
probe, but the supplied function totals alone do not separate native host work
from runtime-helper overhead. The report also does not count individual
map-read fast-path hits.

### Imported functions and cache activity

The host first observed imported render functions such as `render_Table`,
`render_Widget`, `render_table_row` and `Input` in the tier's profiling table.
This establishes that they reach the observation/admission boundary. Because
the reported entries were still `profiling`, it does **not** establish native
execution of those particular functions. Repository integration tests
separately assert native execution of an imported component renderer.

Reported summary:

| Counter | Value | Interpretation |
| --- | ---: | --- |
| `library_compilations` | 104 | Cumulative script-library lowering operations |
| Library compile time | 65.6 ms | Cumulative lowering time, not native compile time or a single startup pause |
| `allocated_code_bytes` | 32,768 | Cached executable-buffer capacity, not process RSS |
| `specializations` | 369 | Retained signature records, including non-native states |
| `evictions` | 47 | Aggregate program/specialization evictions, not necessarily code-byte pressure |

These counters show real cache activity, not a long-run memory bound or absence
of churn. Library lowering can repeat after eviction or reload; its reported
cost must not be described as unconditionally one-time.

## What this closes, and what it leaves open

The maintainer accepts this run as the current stage's real-host functional
sign-off. Together with the independent semantic tests and release probes it
supports merging an explicitly experimental capability.

Debug selftest timing does not establish a release profitability ceiling.
There was no reported whole-application AST/Grain/adaptive release comparison,
startup/frame p95, or long-session memory measurement. Those remain future
measurements when a host opts in; they are not promised gains or new blockers
for this explicitly limited merge.

Future arithmetic or data-processing scripts can reach the existing adaptive
pipeline without a function allow-list, **if** their calls reach the boundary,
their constructs are supported and their observed cost justifies compilation.
The microbenchmark's approximately 17x numeric gain is not guaranteed for a
future application.

## Follow-up queue

Maintain these as general engine work, driven by evidence from any host:

1. **Published-core isolation (merge gate closed).** Fork-only methods and types
   live in the unpublished experiment crate. gpui-rhai exposes only a general
   interface using official Rhai types, and CI packages all advertised features
   outside the workspace Git patch. The packaged core contains neither
   `rhai-jit-experiment` nor dynasm. Retain this check.
2. **Adaptive overhead.** Track the existing tiny-helper regression (375 ns
   Grain versus 500 ns adaptive in the recorded probe). If material in a host,
   reduce admission/routing/sampling cost; verify both tiny and larger cases,
   including demoted functions. Do not promise zero policy overhead.
3. **Library/cache churn and cold cost.** On repeated interaction/reload runs,
   correlate lowering counts, evictions and latency; distinguish cold loading
   from repeated compilation. Add finer counters only when needed. Measure
   process memory separately from executable-buffer capacity.
4. **Broader workloads and semantics.** Add independent regression cases for
   reproducible unsupported constructs, errors, reentry or ownership failures.
   Expand the generic lowering/helper surface when those cases justify it;
   do not hard-code host function names or UI object types.
5. **Rhai upstream preparation.** The exact experiment is retained on the
   `eddix/rhai` `jit` branch without changing its default branch. Separate
   runtime correctness, backend extension and observability into reviewable
   proposals only after another interface pass. Upstream acceptance is not
   assumed.

x86 emission, persistent code caches, background compilation, general
OSR/deoptimization, arbitrary capturing closures and JIT single-stepping remain
outside this iteration. See [the support boundaries](jit-multitype.md).
