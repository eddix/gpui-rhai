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

The shared mechanism is `.retained_page(visible)` on an ordinary keyed node.
The accepted subtree remains in `RetainedUiTree`, the primitive registry and
handle maps. The renderer stops before building a hidden page's descendant
GPUI elements; it does not lay native scroll/editing surfaces out at zero size.
Ordinary `style().hidden()` is not this lifecycle contract. Nested visibility
is the conjunction of retained-page ancestors. Hidden pages lose pointer
capture and leave native focus traversal and presented accessibility while
their native editor and scroll identities remain owned. Node-motion scopes
freeze without elapsed-time catch-up. No business work is cancelled.
Page hiding and whole-view suspension compose in the existing motion runtime:
the first applicable suspension reason freezes active time, intermediate
reasons do not resample it, and only the final removal restarts the clock.
Ordinary selectable-text ranges also remain retained, but a hidden owner cannot
handle the view's Copy action. Revealing it restores the same range.

`ctx.is_visible()` tracks **last committed placement**, not ambient construction
environment. It returns false before first placement; a changed committed
placement invalidates only readers through the normal incremental-render path.
Thus eagerly constructed covered deep-link pages never read an optimistic
visible value. Initially visible pages and the root become true after their
first placement commit. Failed candidates restore placement/dependency/motion
state with the existing runtime checkpoint. Construction ownership remains ADR
0019's; this does not adopt ADR 0023's deferred/placement-owned construction.
Placement includes all co-located construction owners when wrappers directly
return an inner component or an eagerly constructed Node prop, rather than
only the outermost visual replacement marker.
Applications use visibility in effect dependencies or explicitly pause their
own polling. Purely visual clocks may use it to avoid hidden redraws.

`Tabs.keep_mounted` defaults to false. When enabled, every supplied panel uses
the shared mechanism with its tab value as identity. Even in selected-only
mode, eagerly constructed components keep caller-owned Rhai state and effects;
only retained presentation promises native continuity. Mount-on-first-visit is
implemented by the caller choosing which content to construct.

`patterns/navigation_stack.NavigationStack` accepts a non-empty controlled
`stack` of `{key,title,content}`, bounded by `max_depth`. Unique visit keys allow
repeated record ids. `on_pop({depth,source})` proposes the number of entries to
keep; it does not change routes or load data. Back, breadcrumb and none headers
share the same retained pages. Narrow breadcrumb trails collapse middle visits
into a keyboard-operable Menu. RTL mirrors the back affordance and page motion;
motion follows Host preferences. Push focuses the current heading; pop restores
a surviving opener and otherwise falls back to the page heading.
Recovery is eligible only while that stack owns focus, or the window has no
focus. A deferred request records the exact focus owner, window and transition
epoch; newer user/Host focus intent, another transition, suspension or reload
invalidates it. Initial mounting cannot take focus from another view or a
Gallery sidebar, and revealing an unchanged stack inside a tab leaves focus on
the tab list.

Hosts inspect `navigation_stacks(window,cx)` and call
`navigate_back(window,cx) -> bool` before outer navigation. Only the deepest
visible focused scope with a pop handler handles Back; another view never wins
by recency. False delegates outward. Ignored or failed proposals still count
as handled, preventing a failed inner pop from also navigating the outer Host.
`NavigateBack` is the equivalent focus-routed GPUI action. The SDK installs no
global chord; Hosts bind their platform policy. A component's optional
`back_key` is local to that scope.

### Scrolling

ScrollHandle, uniform-list handles and ListState adapt to a shared themed
scrollbar without making the internal window interaction coordinator public.
Official virtual lists and document surfaces use the same rendering, theme,
RTL and cancellation policy as Host extensions. Scrollbar dragging preserves
ListState's measured-height freeze contract.

Trusted Rust obtains a `ScrollbarContext` from
`PrimitiveContext::scrollbars(&PrimitiveTheme)` or
`ScriptViewHost::scrollbars(scope, &PrimitiveTheme)`. Its `element(id, source,
spec)` accepts the public `ScrollbarSource` contract, implemented for all three
GPUI handles. The factory owns only interaction/theme scope, never a second
position engine. `with_part_styles(track, thumb, thumb_hover)` resolves the
same background overrides as retained nodes, and `SCROLLBAR_TRACK_WIDTH`
lets a Host reserve row space. Host surfaces render under the Host container;
primitive scopes inherit retained-node teardown. The scrollbar is not a focus
or keyboard owner.

Native ancestry can be declared with `with_scroll_parent(source, axis)`, which
adds the new nearest parent before the inherited SDK chain. Add farther parents
first: document then table yields table → document → Rhai/SDK ancestors.
Retained/primitive factories inherit the SDK chain automatically. Track wheel
routing applies only actual per-axis movement and chains
the residual; fully unhandled input is not swallowed. This scoped policy does
not replace GPUI's whole-window wheel dispatch. `ScrollbarSourceHandle` is a
retained lease for sources replaced under a stable element id, so a captured
gesture cannot continue against a retired source. Raw adapters require an id
change on replacement when the Host does not retain such a lease.

Virtual collection following is a state machine, not an unconditional scroll
on render. Reader input pauses following; reaching the tail or an explicit
jump resumes it. Updating realized rows must not change reading position.
Keyed data edits preserve the reader's item and intra-item offset where that
key survives. There is one authoritative native scroll state; SDK key caches
must not introduce another competing position or follow state.

`follow_tail` uses GPUI `FollowMode::Tail`. Component-local start/end/by commands
are transactional; `on_follow_change` reports only changed presented native
following and unseen-key counts. The adapter preserves native height facts as
hints when the pinned splice/width-invalidation API requires re-seeding, and a
one-pixel native measurement sentinel prevents an unknown suffix from looking
like the true tail. Stable frames and realization-only updates never re-seed
the native tree. The source component, not virtualization, owns keyboard
selection and viewport shortcuts.

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

### Controlled collections and column browsing

Table display identity and rank data are distinct: controlled sort and emitted
sort changes name the declared column key, and Rust resolves its optional
`sort_key` before sorting. Arrays and NativeCollection share this grammar. An
absent sort preserves source order; explicit Array sorting now runs in Rust
instead of preserving an already sorted caller Array. Missing/null ranks stay
last independently of direction, ties preserve source order, and incompatible
non-null types reject the candidate before any order is cached or committed.

ColumnBrowser composes the ordinary List row grammar and native projection;
there is no browser-specific virtualizer or second selection engine. It owns
only a column-key keyboard cursor and its single focus target. Columns and the
selected path are controlled. Fetching, retrying, truncating descendants and
rejecting late results by request/path revision remain application duties.
The managed List keyboard mode does not replace a List's internal ElementRef.
Horizontal reveal uses the ordinary retained scroll command and respects RTL.

Explicit path/code copy uses a shared `ctx.copy_text` command, queued only from
an attached view/window responding to direct user input or an explicit automation
command. Render, effect, timer and asynchronous delivery origins cannot write
the clipboard, including work originally started by user input. Clipboard text
uses the ordinary bounded UiValue string domain, the command is window-scoped,
and a failed callback/render transaction discards it before native application.

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
