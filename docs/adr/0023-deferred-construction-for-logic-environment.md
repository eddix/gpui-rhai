# ADR 0023: Deferred construction for logic-level environment

Status: proposed, deferred. Not scheduled; see "Trigger conditions".

## Context

Formal component constructors execute eagerly
([ADR 0019](0019-node-prop-component-ownership.md)). A component passed to
another component as a node prop has already rendered, with its state,
callbacks and effects owned by the constructing scope. The receiver only
decides presentation.

The 0.2.0 design system introduces environment values (`density`, `size`,
`disabled`, and application-declared values) that flow from a container to its
descendants. Because of eager construction, a receiver such as Toolbar cannot
influence how a passed Button executes its Rhai render. 0.2.0 therefore
resolves environment values **during native rendering**: components express
environment-dependent geometry as environment-variant tokens, and the renderer
resolves them against the inherited environment, like inherited text color,
direction and locale.

That model is complete for presentation. It cannot express **logic-level**
environment: a component choosing different structure, content or behavior
depending on where it is placed (for example a compact list item dropping a
secondary line, or an application "edit mode" that changes which controls a
child renders).

Mainstream declarative frameworks support logic-level environment by
constructing children lazily: React elements, SwiftUI view values, Flutter
widgets and Compose content lambdas are descriptions rendered after placement,
and state belongs to the placement position.

## Decision

Do not change construction semantics in 0.2.0.

- Presentation-level environment is resolved natively; this remains the model
  for geometry and paint even if deferred construction is adopted later,
  because it changes density without executing any Rhai.
- Logic-level environment is out of scope. Components needing context for
  logic receive it through props or stores.
- A limited `ctx.env()` read during Rhai rendering is rejected: it would be
  reliable for a component's own subtree and silently wrong for nodes passed in
  slots.

## Sketch of the deferred model

If adopted, constructors such as `Button(props)` would return an unrendered
component element (id, props, presentation layers). Rendering would happen when
a receiver places the element, inside the receiver's render scope:

- component instance paths and all state, effects, timers, signals and refs
  derive from the placement path instead of the construction path;
- `ctx.env(name)` becomes a tracked read of the placement environment;
- presentation layers recorded on an unrendered element (`with_style`,
  handlers) are applied when it renders, reusing ADR 0019's layer replay;
- an element that a receiver omits is not mounted, so its state is dropped
  (placement ownership), unlike today's retained caller-owned instance.

## Consequences if adopted

- Rewrites ADR 0019's ownership matrix: initial render, bailout, dirty child,
  dirty receiver, hot reload and suspension all change owner.
- Callback scope markers and invocation contexts move from constructor to
  placement.
- Existing scripts that inspect or reuse returned nodes change meaning.
- Performance must be re-measured with the same alternating A/B method as ADR
  0019 (rerender, reverse, selection, native resize p95); ADR 0019 recorded a
  13–17% selection regression for a rejected eager-snapshot prototype, so the
  budget is not free.

## Trigger conditions

Reopen this ADR when one of these is true:

1. A bring-your-own-design application or an official pattern needs structure
   to depend on placement and props or stores make it materially worse.
2. Gallery scene pages repeatedly need a component to change structure by
   density or size, not only geometry.
3. A runtime model revision is already planned that touches ADR 0019 ownership.
