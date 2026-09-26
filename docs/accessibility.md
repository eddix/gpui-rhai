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
- Input and searchable Combobox use the native GPUI input handler for IME,
  UTF-16 ranges, selection, and clipboard operations.
- The Rhai Combobox source supports arrows, Home, End, Enter, Escape,
  first-character type-ahead, and disabled
  option skipping. Dialog traps focus, reclaims it after an embedding Host's
  accidental ancestor-focus request, and the window overlay coordinator
  restores the previous focus on dismissal.
- Toggle and ToggleGroup preserve pressed state and orientation. Slider exposes
  value/min/max/orientation, commits pointer changes once, and supports logical
  arrow plus Home/End keyboard control.
- Command exposes listbox/option/group semantics, skips disabled commands, and
  shares the same keyboard model for Array and NativeCollection data.
- ContextMenu reuses Menu semantics at a secondary-click anchor; Sheet and
  AlertDialog use the modal focus boundary.
- Motion respects one central `MotionPreference`. Hosts may set it with
  `.motion_preference(...)`, applications may call
  `ctx.set_reduced_motion(bool)`, and `GPUI_RHAI_REDUCED_MOTION=1` provides a
  process-level default. Sources declare `decorative`, `feedback`, or
  `essential` intent but cannot relax Host policy. Reduced motion settles
  spatial/decorative motion and preserves a recognizable frame for infinite
  progress indicators.
- RTL windows reverse row ordering, resolve logical spacing/alignment, map
  horizontal navigation keys logically, and support paired directional icons.
- FormField retains stable semantic IDs plus labelled-by, described-by,
  required, and invalid relationships in the runtime tree.
- Retained reconciliation copies semantic attributes and static text into a
  stable `CommittedSemanticFrame`; native GPUI/AccessKit and automation consume
  that same immutable projection. Mounted snapshots join only geometry from
  the latest committed presentation frame. The tree flattens layout-only nodes, resolves
  labelled-by/described-by text, rejects duplicate semantic IDs, attaches last
  committed geometry, and supports role/name or semantic-ID lookup through
  `ScriptViewHandle::accessibility_snapshot`.
- The gpui-pre adapter projects role, localized name/description, author ID,
  selected/expanded/toggled/current state, scalar and numeric values, ranges,
  orientation, required/invalid/disabled/read-only state, placeholders,
  shortcuts, set position and table row/column metadata to AccessKit.
- Plain retained text maps to GPUI's native `Label` contract with a direct
  AccessKit value. It is never published as a value-less `TextRun`; native
  consumers may traverse every text run without an optional-value fallback.
- Native AX Click/Focus follows GPUI's window dispatch. TextInput, Textarea,
  Slider and Chart register bounded primitive actions; SetValue and numeric
  steps re-enter the same controlled event flow as pointer and keyboard input.
  Disabled, read-only and stale instances never gain authority through AX.
- Invalid roles are rejected before the retained candidate commits. Legal
  platform gaps remain visible in the internal semantic frame instead of
  crashing or inventing unsupported actions.

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

## Official component coverage

Every official component is assigned one deliberate category; nested controls
retain their own role and action rather than making the entire composite one
opaque node.

- **Operable controls/composites:** Accordion, AlertDialog, Button,
  ButtonGroup, Checkbox, Collapsible, Combobox, Command, CommandDialog,
  ContextMenu, DatePicker, Dialog, FormField, IconButton, Input, InputGroup,
  Menu, Pagination, Popover, Radio, RadioGroup, Select, Sheet, Slider, Switch,
  Table, Tabs, Tag when dismissible, Textarea, Toast when dismissible, Toggle,
  ToggleGroup and Tooltip triggers.
- **Semantic/read-only surfaces:** Alert, Avatar, Badge, Card, CodeViewer,
  DiffViewer, Divider, Empty, GroupBox, Kbd, Label, Progress, ScrollArea,
  Spinner, StatusBar and TitleBar.
- **Decorative by default:** Skeleton and unlabeled Icon. A labeled Icon is an
  image node. Presentation/group headers do not become selectable rows.

Virtual Table, Command and Combobox project overall collection metadata plus a
bounded realized semantic window. Chart projects its title/summary and at most
the focused or selected data summary rather than materializing every source
datum as an AX node.

## Current upstream and platform boundaries

The 0.1.6 implementation candidate uses the exact `gpui-pre 0.3.6` family.
This snapshot exposes public AccessKit roles, properties, synthetic children and
actions, so the former GPUI 0.2.2 native-semantic limitation no longer applies.
It remains a community-published snapshot of a traceable Zed commit, not a Zed
release.

The snapshot still lacks upstream text hit-test fix #64672. It is suitable for
implementation and validation but remains blocked from release until a complete
published family contains that fix or equivalent behavior is proved unreachable
on every supported text path.

GPUI's application-level reduced-motion flag is synchronized only from the
existing Host-owned policy; this release does not add a second OS preference
watcher. GPUI also still lacks a public per-element bidi-isolation contract, so
mixed-direction text requiring explicit isolation remains an upstream gap.

## Visual evidence

macOS screenshot baselines must be generated from deterministic example windows
using the matrix in `visual-testing.md`.
When updating a baseline, record the theme, scale factor, viewport, state, and
reason for the change. Screenshots supplement keyboard and semantic assertions;
they do not replace them.
