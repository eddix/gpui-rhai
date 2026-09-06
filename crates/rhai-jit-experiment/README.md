# Rhai JIT experiment

The current experiment has scalar and managed execution lanes, reentrant named
calls, imported-library coverage and profitability-based adaptive admission.
See [the multi-type design and integration guide](../../docs/jit-multitype.md).
The notes below describe the original scalar-only baseline/manual tier.

This crate measures the upper bound of compiling one fully-lowered, typed
Grain function to native code. gpui-rhai uses it only as an AArch64 dev-test of
the generic accelerator injection API; it is not a production backend.

The original baseline lane was deliberately narrow:

- AArch64 only;
- a selected Grain function whose parameters and return value specialize to
  integers, inside an otherwise multi-function program;
- integer, boolean, unit, local-slot, branch, checked arithmetic, and `Tick`
  instructions;
- whole-function rejection for every unsupported instruction or type merge;
- no mid-function fallback or deoptimization.

The pinned `eddix/rhai` `jit` branch exposes a read-only `grain::ProgramView` for function,
chunk, constant, name, operator, assignment, chain, and switch pools. The
experiment consumes that API directly and no longer parses the serialized
artifact format.

Run the release probe:

```sh
cargo run --release -p rhai-jit-experiment
```

On AArch64, another workspace crate can attach the same tier to a VM:

```rust
use rhai_jit_experiment::tier::FunctionTier;

let tier = FunctionTier::for_function(3, "total_pages", 2);
let mut vm = rhai::grain::Vm::new(&engine)
    .with_function_accelerator(tier.accelerator());

// Calling a Rhai entry such as `workload()` on `vm` now offers its direct
// Rhai call to `total_pages` to this tier.
```

gpui-rhai hosts attach the unpublished adapter from
`ScriptViewExtension::configure_engine`:

Enable the experiment crate's `gpui-adapter` feature for this integration.

```rust,ignore
fn configure_engine(&self, engine: &mut gpui_rhai::RuntimeEngine) -> Result<(), String> {
    engine.set_experimental_script_backend(Some(std::rc::Rc::new(
        rhai_jit_experiment::gpui::GrainScriptBackend::new(Some(
            self.tier.accelerator(),
        )),
    )));
    Ok(())
}
```

`FunctionProfiler` attaches through the same adapter and always declines, then
reports inclusive and exclusive/self Grain time, operations, calls and failures
for every direct helper. One profiler may aggregate multiple view Programs;
per-thread pending stacks keep nested calls paired correctly. The tier requires
Rhai's default fast-operator mode; it declines when primitive operators are
routed through overrideable function dispatch.

`AdaptiveFunctionTier` closes the profiling loop without a function allow-list.
It keys entries by Program, function index, and runtime `TypeId` signature;
promotes only after both completed-call and exclusive-time thresholds; caches
compiled and rejected states; and bounds Programs, specializations, and native
code bytes with LRU eviction. The default policy requires 32 Grain calls and 1
ms of exclusive time. Stats retain compile latency plus total/max/average JIT
time so a host can verify that promotion amortizes its cost. Hosts can tune
`AdaptiveTierConfig` and inject `adaptive.accelerator()` through the same
RuntimeEngine setter.

The JSON output includes dynamic opcode counts, AST/Grain/typed/JIT value and
operation parity, code size, compilation latency, and interleaved p50/p95
execution latency. `dynasmrt` owns the writable-to-executable memory transition;
the crate-local unsafe boundary only turns its live entry pointer into the
matching AAPCS64 function type.

The pinned Rhai `jit` branch exposes a `Vm::with_function_accelerator` hook at direct
Grain script-to-script calls. At that boundary, every argument is checked
before the JIT frame is created. An all-integer match executes native code; an
arity or type miss runs the original Grain function with the untouched
`Dynamic` arguments. Errors after native execution begins are reported rather
than retried, so a future side-effecting subset cannot accidentally execute
twice. Native `Tick`s are replayed into the calling VM's operation/progress
budget. A batchable observation callback can coalesce that accounting without
changing operation limits; exact termination callbacks remain per-operation.

The experiment also owns a per-`name/arity` tier table. Calls execute in Grain
until their hotness threshold, then the selected `ProgramView` function is
lowered, compiled once, and cached. Rejected functions are cached as rejected
and continue through Grain without repeated compilation. The checked-in product
case calls `total_pages(total_items, page_size)` from a Rhai `workload()` inside
the full Pagination component program, exercising the same internal path a real
application would use. Cache entries include Rhai's opaque runtime `Program`
identity. A change flushes the current tier table, so recompilation or artifact
reload cannot reuse native code from the previous script generation and old
generations do not accumulate.
