# Rhai execution backends

The AST interpreter is gpui-rhai's semantic oracle and the only production
backend. Named entry, lifecycle, and root-render calls pass through one static
`ExecutionBackend` boundary; imported callbacks and incremental component
recipes continue through the separately characterized invocation-context
adapter.

Rhai 1.26's Grain VM is available only to the optional characterization suite:

```sh
cargo test -p gpui-rhai --features grain-backend backend::tests
cargo clippy -p gpui-rhai --all-targets \
  --features "dev-reload grain-backend" -- -D warnings
```

The harness compiles the same self-contained AST to Grain, records
`residual_count`/`residual_nodes`, and compares registered native calls, literal
imports, named functions, `FnPtr` curry, structured values, and failures. Value
cases currently agree. Failure diagnostics do not: for the characterized divide
by zero, Grain appends a function-call frame that the AST error string does not
contain. The test records that exact blocker instead of weakening production
diagnostic semantics.

Do not add a runtime switch merely because the optional feature compiles.
Production Grain activation additionally requires parity for imported formal
components, stored callback/effect contexts, operation/progress limits, source
positions and reload failure behavior, followed by an end-to-end performance
win including Rhai execution, reconciliation, GPUI layout, and paint.
