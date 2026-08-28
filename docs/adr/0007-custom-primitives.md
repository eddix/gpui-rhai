# ADR 0007: Custom primitive registration

Status: Accepted

Applications may register namespaced native primitives with a props schema,
event schemas, optional state/lifecycle, and a renderer receiving GPUI
`Window/App`. Rhai receives a normal namespaced constructor returning `UiNode`.

This is the escape hatch for native mechanisms without expanding or forking the
core runtime API.
