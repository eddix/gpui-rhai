# gpui-rhai

The runtime crate for [GPUI Rhai](https://github.com/eddix/gpui-rhai): a
declarative `UiNode` boundary, Rhai lifecycle, Rust capabilities and primitives,
and GPUI rendering mechanisms. It is independent of `gpui-component`.

The core public API is view-first: `FileScriptView` or `EmbeddedScriptView`
prepares a `PreparedScriptView`, which can be mounted into an existing GPUI
window through `ScriptViewHost`, or passed to `ScriptApplication` for a
standalone process/window.

See the repository README and documentation for the CLI quick start, component
registry, embedding guide, security boundary, and production workflow.
