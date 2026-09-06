# Rhai execution backends

## Published runtime contract

The AST interpreter is gpui-rhai's semantic oracle and the only execution path
in the default package. The published crate depends on official Rhai 1.26.0.
It does not contain vendored Rhai source, a normal JIT dependency, or a Cargo
feature that promises fork-only Grain APIs.

The optional `experimental-backend` feature exposes two generation-bound
traits using only official Rhai types:

- `ExperimentalScriptBackend` configures an engine and compiles an AST;
- `ExperimentalCompiledScript` invokes named functions through backend-owned
  state while preserving the ordinary result/error boundary.

Enabling this feature alone changes no execution. A host must attach an
implementation before compiling a generation. Passing `None` restores the
ordinary AST path and exact progress observation for future generations.

Every advertised package feature is verified outside the workspace patch:

```sh
cargo package -p gpui-rhai --locked --all-features
```

## Unpublished Grain/JIT experiment

`crates/rhai-jit-experiment` implements the optional interface with
`GrainScriptBackend` behind its `gpui-adapter` feature. It remains
`publish = false` and is tested only inside a
workspace that pins the `jit` branch of `https://github.com/eddix/rhai` to an
exact revision. Cargo patches do not propagate through dependencies, so an
application opting into this experiment must repeat that root patch and use a
matching gpui-rhai experiment revision.

Current Rhai revision: `989efd986d05ce9799ab2d0472ef7f697d022316`.
The fork's default branch is unchanged; this commit exists only on `jit`.

The adapter owns every fork-specific symbol: callback bindings, Grain program
compilation, accelerator injection and permission to batch observation-only
progress callbacks. gpui-rhai core stores only the two trait objects. The Rhai
fork itself owns shared reentry accounting and turns a stored accelerated
context into a fresh retained invocation; direct synchronous reentry continues
to share the live budget.

Typical host configuration:

```rust,ignore
use std::rc::Rc;

use rhai_jit_experiment::adaptive::AdaptiveFunctionTier;
use rhai_jit_experiment::gpui::GrainScriptBackend;

let tier = AdaptiveFunctionTier::default();
engine.set_experimental_script_backend(Some(Rc::new(
    GrainScriptBackend::new(Some(tier.accelerator())),
)));
```

Without that call, the host remains on the AST interpreter even when the
extension feature is compiled. Environment switches such as `OMB_GRAIN_JIT`
belong to the consuming application; gpui-rhai does not activate JIT from a
global environment variable.

## Current implementation

The experiment has two AArch64 lanes:

- an unboxed unit/bool/i64/f64 scalar lane with checked arithmetic, branches,
  loops and exact Rhai error reconstruction;
- a managed lane whose generated control flow calls statically selected Rhai
  helpers while strings, maps, arrays, function pointers and registered host
  values remain owned `Dynamic` values.

Named entry points, nested functions, imported-library functions and reentrant
callbacks can reach the tier. Unsupported functions decline before execution.
Once native execution begins, errors never restart the function. The adaptive
tier profiles exclusive Grain time, specializes runtime signatures, bounds
program/signature/code caches and demotes native code that does not meet its
profitability margin.

See [the multi-type contract](jit-multitype.md), [repository measurements](jit-multitype-results.md)
and [the first host checkpoint](jit-dogfooding-2026-09-03.md).

## Evidence and limits

The recorded release probes found a 17.06–17.35x numeric speedup and a
7.49–8.02% reduction for the object-processing sample. Forced native execution
of the host-call-heavy sample was 3.19–4.34% slower and is therefore handled by
profitability-based fallback. A tiny helper also retains measurable adaptive
routing overhead. These are microbenchmarks, not a whole-application promise.

The first omb debug-build dogfood reported 28/28 assertions in default and JIT
modes. Its real UI specialization compiled and ran, then correctly became
`unprofitable`; no release application gain was established.

This iteration does not provide x86 generation, persistent code caches,
background compilation, general OSR/deoptimization, arbitrary capturing
closures, JIT debugger stepping, or complete Rhai language/configuration
coverage. Formal component subtree reuse remains a core optimization and does
not create synthetic JIT calls.

The fork is not published to crates.io and its `jit` branch is not merged into
its default branch. If equivalent APIs reach an official Rhai release, remove
the workspace patch first; only then consider publishing the adapter as a
separate crate or adding a concrete Grain convenience feature.
