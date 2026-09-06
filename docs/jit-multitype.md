# Multi-type Grain JIT

This is a general, opt-in AArch64 experiment, not an application-specific
function allow-list. It targets the repository's default 64-bit Rhai build.

## Execution and ownership

- **Scalar lane:** unboxed unit, bool, i64 and f64, mixed numeric arithmetic,
  locals, branches and loops. Checked arithmetic errors preserve Grain's
  messages/positions. Float comparisons use Rhai's relative-epsilon rules,
  including NaN and signed zero. Ticks and backward edges share Grain's budget.
- **Managed lane:** native control flow and statically selected Rust helpers.
  Strings, maps, arrays, function pointers and arbitrary registered host types
  retain their Dynamic ownership. Simple map-property reads are specialized
  by bytecode shape, with guarded fallback for getters and missing keys.

There is no UiNode/UiContext-specific ABI. Assembly never reads Dynamic's
private representation: managed values stay in initialized Rust-owned scope
and operand containers; the scalar lane has a private unboxed frame.

The selector releases accelerator/cache locks before prepared code executes.
Shared executable owners pin code and embedded map-site data during callbacks,
recursion and eviction. Declining leaves arguments untouched. Once execution
begins, an error propagates without restarting the function.

Live reentrant calls share their budget. Retained callbacks use an explicit
new-invocation boundary: module environment survives, consumed budget/depth do
not. Ordinary Result errors preserve cleanup. Unwinding helper panics become
terminal system errors, not script-catchable errors; panic-abort/OOM cannot be
recovered. If a panic payload's destructor itself panics, its second payload is
intentionally forgotten to prevent unwinding through generated code.

## Integration

The published core and unpublished experiment are separated:

| Configuration | Execution |
| --- | --- |
| Default gpui-rhai dependency | Official Rhai AST path; no JIT dependency or Rhai patch |
| gpui-rhai with `experimental-backend`, no attached implementation | The same AST path |
| Unpublished adapter attached without an accelerator | Grain VM with ordinary residual fallback |
| Adapter attached with an adaptive tier | Hot supported functions may execute native code; others retain Grain/AST |

The core feature exposes only a fork-independent trait boundary built from
official Rhai types. Fork-specific compilation and invocation live in the
`publish = false` experiment crate. The host attaches one shared tier:

```rust
use std::rc::Rc;
use rhai_jit_experiment::gpui::GrainScriptBackend;

let tier = AdaptiveFunctionTier::new(AdaptiveTierConfig::default());
engine.set_experimental_script_backend(Some(Rc::new(
    GrainScriptBackend::new(Some(tier.accelerator())),
)));
```

The adapter prepares callback bindings once per generation. They live outside
Program ownership to avoid cycles and cover native calls by name as well as
explicit Fn pointers.

Named entry points, nested Grain calls, imported-library functions and named
native callbacks can reach the tier. Library lowering is cached using pinned
library identity and source. An accelerated caller can call an unsupported
callee through the ordinary runtime. Subtree reuse still skips rendering; it
does not generate fake JIT calls.

Until official Rhai supplies equivalent APIs, an opt-in application must patch
`rhai` to the exact `eddix/rhai` `jit` revision from this workspace. Patches do
not propagate through dependencies. The fork is not published to crates.io and
its `jit` branch is not merged into its default branch.

## Admission, memory and diagnostics

Defaults require 32 completed Grain calls and 1 ms of exclusive Grain time.
The cache keeps up to 64 Programs and four signatures per function, bounded by
16 MiB of executable-buffer capacity. `code_bytes` means emitted instructions;
`allocated_code_bytes` means cached backing-buffer capacity. Neither is process
RSS; capacity excludes OS page rounding, Rust metadata and in-flight code
already evicted from the cache.

`check_profitability` defaults to true. After ignoring warmup, the tier compares
rolling medians of 16 successful self-time samples. Native execution must show
a 5% margin; otherwise calls use Grain and the entry becomes `unprofitable`.
The cached code gets a new trial after 256 calls. Periodic Grain samples
refresh the reference without executing any invocation twice. This is a
heuristic over observed inputs, not a guarantee for every future invocation.

Stats distinguish `profiling`, `compiled`, `rejected` (unsupported/over budget)
and `unprofitable` (implemented but not beneficial). They include lane,
argument types, up to eight observed return types, guard misses, failures,
native compilation time, execution time, medians and code capacity. Summary
separately reports library-lowering count/time.

FunctionProfiler uses the same named-call boundaries, including host entries
and imported libraries, but always declines native execution. FunctionTier
remains the narrow manual integer experiment. Normal integration uses
AdaptiveFunctionTier.

## Validation

See [the validation report and raw measurements](jit-multitype-results.md).

```sh
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p rhai-jit-experiment --all-features
cargo package -p gpui-rhai --locked --all-features
cargo run --release -p rhai-jit-experiment --example multitype_probe
JIT_CHECK_PROFITABILITY=1 cargo run --release -p rhai-jit-experiment --example multitype_probe
```

The first probe disables profitability checking to measure native capability;
the second measures the adaptive policy, which may choose Grain. Measurements
interleave AST, Grain and tier calls and verify equivalent values. Debug timing
and jit_calls alone are not performance evidence. Both VM paths use reusable
callback bindings, so their callback capabilities are equivalent.

The original integer/product probe remains available with
`cargo run --release -p rhai-jit-experiment`.

## Remaining boundary

No x86 code generation, disk cache, background compilation or general OSR is
included. Anonymous/capturing managed bodies, statement-block residuals and
native try/catch handlers remain in Grain/AST. Expressions such as qualified
calls can use a runtime helper inside a compiled body. Debugger execution uses
the ordinary path; exact progress callbacks bypass batched scalar execution.
Reference, constant, shadowing and variable-limit guards retain ordinary
behavior whenever scalar assumptions do not hold.

This is an experimental implementation, not a claim of complete Rhai-language
coverage or a promised whole-application speedup. The first host's
[dogfooding checkpoint](jit-dogfooding-2026-09-03.md) reports functional success
and profitability-based demotion for its current debug UI workload. No
business-code changes were needed to select named functions.
