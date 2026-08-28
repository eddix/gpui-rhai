# ADR 0003: Formal component schema

Status: Accepted

Formal Rhai components call `export_component` with one schema covering props,
defaults, state, events, slots, style parts, dependencies, capabilities, source
version, and runtime API range. Files also start with machine-readable metadata;
header and export must agree.

This strict contract enables runtime validation, CLI compatibility checks,
editor metadata, and generated documentation.

Formal constructors execute through `component_render`. The runtime owns a
render-local parent stack and state transaction, and binds returned callbacks to
their component path and event schema. Rhai 1.26 is pinned exactly; its internal
clonable native call-context store is used only inside `ScriptCallback` so
module-private handlers remain resolvable after render. This volatile Rhai type
is not exposed in GPUI Rhai's public API and must be revalidated on any Rhai
upgrade.
