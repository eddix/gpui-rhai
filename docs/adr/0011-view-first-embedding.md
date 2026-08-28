# ADR 0011: View-first host embedding

Status: Accepted

The prepared unit is a single-use `PreparedScriptView`, not an application or
window. An existing GPUI host mounts it as a `ScriptViewHandle`; the optional
`ScriptApplication` adapter owns the standalone event loop and windows.

Every mounted view owns an independent Engine, Runtime, Lifecycle, state,
capabilities, tasks, subscriptions, assets, and diagnostics. A
`ScriptViewHost` shares only cross-view native interaction mechanisms. Physical
`window_id` and mounted `view_id` remain distinct.

The Host must wrap all sibling view elements once per frame. View elements
measure their allocated bounds automatically for responsive classes; overlay
placement uses the separate Host viewport. Embedded views do not mutate global
key bindings and reject script window commands unless run through
`ScriptApplication`.

We rejected exposing the internal renderer Entity directly, conflating view and
window identity, per-view overlay managers, and implicit global key binding.
These alternatives either freeze implementation details or break cross-widget
dismissal, isolation, and auditable host policy.

Protected by the `embedded_views` release example, GPUI integration tests for
three simultaneous views, duplicate local overlay IDs, automatic compact
sizing, escaping placement, click-through outside dismissal, key conflicts,
and explicit dispose/remount.
