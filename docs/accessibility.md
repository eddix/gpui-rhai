# Accessibility status

Accessibility and keyboard operation are release requirements.

## Implemented

- Interactive nodes are GPUI tab stops.
- Atomic nodes may declare bounded `tab_index` values and `tab_group()`; the
  renderer maps them to GPUI's native grouped tab-order instead of assigning
  every node index zero. `tab_stop(false)` keeps programmatic focus while
  removing keyboard reachability.
- Button activates with mouse click, Enter, or Space.
- Disabled/loading Button instances do not dispatch callbacks.
- Rhai nodes preserve normalized role, label, disabled, value, and related
  semantic attributes independently of GPUI.
- Focus, hover, active, and disabled visuals are represented by native GPUI
  interaction styles rather than high-frequency Rhai rerenders.
- Input and searchable Dropdown use the native GPUI input handler for IME,
  UTF-16 ranges, selection, and clipboard operations.
- The Rhai Dropdown source supports arrows, Home, End, Enter, Escape,
  first-character type-ahead, and disabled
  option skipping. Dialog traps focus and the window overlay coordinator
  restores focus on dismissal.
- Animation respects one central `MotionPreference`. Hosts may set it with
  `.motion_preference(...)`, applications may call
  `ctx.set_reduced_motion(bool)`, and `GPUI_RHAI_REDUCED_MOTION=1` provides a
  process-level default. Reduced motion settles active animations immediately.
- RTL windows reverse row ordering, resolve logical spacing/alignment, map
  horizontal navigation keys logically, and support paired directional icons.
- FormField retains stable semantic IDs plus labelled-by, described-by,
  required, and invalid relationships in the runtime tree.
- Retained reconciliation copies semantic attributes and static text into a
  stable `AccessibilityTree`, flattens layout-only nodes, resolves
  labelled-by/described-by text, rejects duplicate semantic IDs, attaches last
  committed geometry, and supports role/name or semantic-ID lookup through
  `ScriptViewHandle::accessibility_snapshot`.

## Complex-control behavior

Source/runtime tests prove:

- Select uses public combobox/listbox metadata, skips group headers and disabled
  options, clears transient search on close, and relies on Overlay focus restore;
- Rhai DatePicker supports trigger opening, logical day/week/month navigation,
  locale week boundaries, disabled-range suppression, commit, Escape, and focus
  restoration;
- Table retains table/row/header metadata and controlled selection payloads;
- Pagination controls have localized labels, current-page state, disabled edge
  behavior, and explicit LTR/RTL directional resources;
- Textarea retains multiline/read-only/invalid metadata, visible focus,
  grapheme-safe selection and limits, correct IME candidate bounds, and Tab form
  traversal.

Platform-level manual certification and fresh visual baselines remain governed
by `visual-testing.md`.

The 2026-08-30 macOS pass includes real `鼠须管` candidate commit and marked-text
Escape cancellation in Textarea; the exact evidence is recorded with the
visual baselines.

## Pinned GPUI limitation

The pinned GPUI 0.2.2 release uses AccessKit internally but does not expose a
public element API for assigning arbitrary AccessKit roles, labels, checked
state, or descriptions. GPUI Rhai therefore retains these semantics in UiNode
snapshots and the stable AccessibilityTree, but cannot yet forward all values to the
platform accessibility tree without relying on GPUI internals.

GPUI 0.2.2 also does not expose the macOS Reduce Motion preference. The host
and environment APIs above are therefore the supported bridge until GPUI adds a
public system-preference signal.

GPUI 0.2.2 does not expose a public per-element base-direction or bidi-isolation
API. GPUI Rhai controls logical layout direction and delegates text shaping to
the platform. Mixed-direction text that requires explicit isolation remains an
upstream limitation.

This is an explicit upstream gap, not a reason to remove semantic metadata from
component contracts. The adapter must be added as soon as the pinned GPUI API
supports it. M1 composite controls still implement deterministic keyboard and
focus behavior and document any platform-semantic value that cannot be exposed.

## Visual evidence

macOS screenshot baselines must be generated from deterministic example windows
using the matrix in `visual-testing.md`.
When updating a baseline, record the theme, scale factor, viewport, state, and
reason for the change. Screenshots supplement keyboard and semantic assertions;
they do not replace them.
