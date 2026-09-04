# Multi-type Grain JIT experiment

Status: implementation and repository validation complete, starting from
`50fbda2` on 2026-09-03. The host maintainer subsequently supplied a successful
manual dogfooding sign-off; see [the checkpoint and follow-ups](jit-dogfooding-2026-09-03.md).
Default-path packaging now also verifies successfully against official Rhai
outside the workspace patch.

## Agreed outcome

Build a general, opt-in AArch64 function JIT, not an application-specific
allow-list. A successful experiment requires semantic parity, representative
repeatable performance gains, and real-host validation. More `jit_calls` alone
is not a performance result. Report cold compilation, warm execution and code
memory separately. No fixed whole-application percentage is promised.

Keep the existing interpreter and Grain fallbacks. Preserve formal-component
subtree reuse. Never restart a function after native execution has performed
effects. Unsupported functions must decline before execution.

## Implementation checkpoints

- [x] Establish deterministic baseline tests and close named-call callback
      environment gaps in Grain.
- [x] Separate selection/cache locking from executable invocation; pin code
      across reentrant calls and eviction. Preserve profiling stack pairing.
- [x] Define the managed/scalar call protocol, ownership and error cleanup.
      Scalar classes are unit, bool, integer and float; managed values retain
      Rhai `Dynamic` semantics, including string, map, array, function pointer
      and arbitrary registered host types.
- [x] Add general native control flow and runtime helpers for supported
      operations. Keep the existing integer fast path; specialize numeric
      operations without making native code depend on `Dynamic` layout.
- [x] Connect script-to-script, script-to-native and native-to-script calls,
      including imported-module/formal-component named functions, retained
      contexts and reload isolation.
- [x] Integrate adaptive admission, actionable rejection reasons and timing;
      add independent semantic/ownership/reentry tests and release benchmarks.
- [x] Run workspace checks and document repeatable benchmark results and
      remaining rejected constructs honestly.
- [x] User's manual dogfooding validation on the real host: reported 28/28 in
      both default and JIT modes; no useful gain established for this debug UI
      workload. Release application performance remains unmeasured.
- [x] Merge gate: all advertised gpui-rhai features package against official
      Rhai. Fork-only calls and types live in the unpublished experiment crate
      behind a general `experimental-backend` interface. CI verifies the
      all-feature package outside the workspace Git patch.

## Boundaries

This iteration does not add x86 code generation, disk caches, background
compilation, general OSR/deoptimization, arbitrary capturing closures or full
JIT debugger stepping. Debugger/unsupported cases retain the ordinary path.

The experimental Rhai changes now live on the unmerged `jit` branch of
`eddix/rhai`, pinned by exact revision. Long-term use of an official release
remains the direction, but upstream acceptance is not assumed and no upstream
PR is part of this implementation.

## Baseline findings

- The new calendar-reuse test started from the wall clock and then selected
  2026-09-03, so it failed on that day without an actual environment change.
- Explicit Grain named calls lacked the callback wrappers that
  `eval_with_callbacks` installs, reproducing `item (UiContext, map)` resolution
  failure in the virtual collection integration test.
- Before this implementation, the accelerator and adaptive cache held mutable
  guards during execution. Prepared invocation now releases those guards before
  executing host calls that may reenter scripts.

## Validation ledger

Repository verification passed; see [the results](jit-multitype-results.md)
and its raw measurements. Independent tests cover values,
ownership, mutation/error cleanup, native panics, exact progress callbacks,
budgets/backward edges, imported functions, retained callbacks, reentrant cache
eviction and map-site fallback. Formal-component integration also exercises
native render, subtree reuse and later event invocation.

Initial release probes found a large scalar gain but a managed-helper
regression. Eliminating trivial stack operations, fusing adjacent helpers and
specializing map sites turned the object-processing sample positive. The
host-call-heavy sample still needs profitability-based fallback; do not hide
that result by reporting only native call counts.

The host maintainer completed the manual run without adapting host code and
elected to keep its experiment feature off by default. Its dependency/config
files were not changed here. The report establishes the agreed functional
checkpoint, not release frame latency, startup cost or long-session memory.
Further optimization follows the general-engine queue in the checkpoint, not
an application-specific type wishlist.
