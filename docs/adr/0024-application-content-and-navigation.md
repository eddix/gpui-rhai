# ADR 0024: Application content and navigation

## Status

Accepted, 2026-10-16. Extends the ownership contracts in ADRs 0017–0019.

## Decision

The SDK supplies reusable native mechanisms and source-owned components for
long content, retained pages and temporal workflows. The application continues
to own data loading, business tasks, routing and controlled values. Final
interactive examples belong to `gpui-rhai-cli gallery`; separate probes and
test binaries are verification, not the public showcase.

### Retained pages

Tabs and NavigationStack share one retained-page policy. Mounting, visibility
and activity are distinct. A hidden page retains component state, scroll
position, native editing state, selection and undo history, but does not
participate in layout, input traversal or presented accessibility. Invisible
motion does not consume frames. Hiding alone does not cancel business tasks or
subscriptions: visibility is observable and pausing is an explicit application
policy. Showing a page restores its retained state without a second mount.

A navigation entry identifies a visit, not a business entity. The same record
may be visited more than once with different entry keys. Back returns to the
actual opener and restores focus where possible. The application owns the
stack; Back emits a proposal rather than changing application routes itself.
Host Back delegates to the focused inner stack before its outer navigation.

### Scrolling

ScrollHandle, uniform-list handles and ListState adapt to a shared themed
scrollbar without making the internal window interaction coordinator public.
Official virtual lists and document surfaces use the same rendering, theme,
RTL and cancellation policy as Host extensions. Scrollbar dragging preserves
ListState's measured-height freeze contract.

Virtual collection following is a state machine, not an unconditional scroll
on render. Reader input pauses following; reaching the tail or an explicit
jump resumes it. Updating realized rows must not change reading position.
Keyed data edits preserve the reader's item and intra-item offset where that
key survives. There is one authoritative native scroll state; SDK key caches
must not introduce another competing position or follow state.

### Documents and composition

MarkdownViewer renders CommonMark and GFM natively. Text, parse models and
selection stay native; background results carry revision and owner identity.
Streaming invalidation must respect complete Markdown semantics, not assume
that only the last block can change. It reuses document typography, syntax
highlighting, theme and lifecycle infrastructure.

Selection spans blocks within one viewer, not separate viewers or messages.
Normal copy yields visible text; explicit source copy yields Markdown; code
block copy yields code without its fences. HTML never executes, links emit
callbacks and images use Host-provided assets rather than initiating network
requests. This is not an editor or a WebView.

Textarea submit belongs to the native editing layer. Enter-to-send never
submits IME confirmation; Shift+Enter remains a native newline. Composer is a
thin controlled composition: busy blocks submission, not drafting; the Host
owns send/stop and clearing the accepted value.

Selectable cards have a presentational choice surface and a separate actions
area. An action does not change selection. Keyboard, checked state and focus
remain distinct and reuse the existing group-control rules.

### Time

A Host-injectable wall clock, time zone model and locale formatting are shared
by temporal components and available without the optional charts feature.
Relative ranges remain relative until resolved at query time; absolute ranges
are epoch instants. Editing an absolute draft does not change the controlled
range until Apply. Controls neither fetch nor start automatic refresh.

Nonexistent local times are validation errors. Ambiguous local times require
an explicit choice of instant. Future times are permitted unless the caller's
bounds forbid them; the SDK never silently clamps to now.

## Acceptance invariants

- Identity, ownership, cancellation, revision checks and failed-render rollback
  apply equally to full rendering, incremental updates and asynchronous work.
- Two views or equal local keys do not share navigation, documents or scroll
  state. Late work cannot affect a replacement owner.
- Both densities, theme modes, square/subtle/round corners, RTL and CJK remain
  valid. Values control state; tokens and part styles control presentation.
- Gallery pages mount and support real interaction, not prepare-only success.
  Native probes assert presented geometry and input behavior, not only props
  or realized counts.
- No `gpui-component` dependency, arbitrary Rhai size-limit increases,
  duplicate selection engines or application-specific network services.
