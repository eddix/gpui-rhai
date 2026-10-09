# Official component catalog

gpui-rhai ships 64 editable Rhai source components. They all use the same
public atoms and generic runtime mechanisms available to application code; no
official component receives a private high-level node constructor.

Run the interactive catalog from this repository:

```text
cargo run --release -p gpui-rhai-cli -- gallery --story components/catalog
```

The gallery is a live component workbench, not a static visual catalog. Enabled
buttons, fields, choices, navigation controls, tables, pagination, commands and
overlays own example state and respond to pointer or keyboard input. The status
bar shows the most recent interaction and a running interaction count. Disabled,
loading, read-only and presentation-only specimens intentionally remain inert.

Theme Studio renders the same exhaustive specimen while editing a theme.

The [registry visual system](../registry-design-system.md) is the maintained
source for component dimensions, color roles, state appearance, and known
visual gaps. This catalog records component semantics and public contracts.

Version 0.1.2 freezes the original 51-component foundation: component IDs and exports,
controlled-state ownership, semantic event payloads, the `xs`/`sm`/`md`/`lg`
size vocabulary, and declared style parts are the maintained base contract.
Future catalog additions must compose the same public atoms and generic runtime
mechanisms; they do not justify parallel private primitives.

## Split Pane

`SplitPane` is a source-owned, nestable two-panel layout component. It
redistributes space inside a shared region; it is not a generic Resizable
wrapper for floating cards or arbitrary elements. It accepts stable start/end
keys, horizontal or vertical orientation,
a controlled start-panel ratio, pixel min/max constraints, controlled collapse
flags, and an accessible separator. Pointer movement stays in the native signal
lane; one `resize(number)` proposal is emitted on release. Rejecting the
proposal restores the controlled ratio. Keyboard arrows use the same proposal
path, and RTL pointer deltas follow the active theme direction.

See the [interaction behavior specification](interaction-behaviors.md#splitpane--implemented)
for its current props and boundary with Resizable. Panel reordering,
cross-window drops, and persistence are outside SplitPane's contract.

## Resizable

`Resizable` controls one absolutely positioned rectangle inside the component's
local boundary. It is independent of SplitPane: resizing a west or north edge
updates `x` or `y` while preserving the opposite edge; no sibling receives the
remaining space. The caller owns the accepted `{x,y,width,height}` rectangle.

The component supports any unique subset of `n/s/e/w/ne/nw/se/sw`, min/max
dimensions, optional boundary containment, optional aspect ratio, a keyboard
step, disabled state, and one `resize({x,y,width,height})` proposal shaped exactly like `rect`.
Pointer moves update four optional-float native signals; Rhai runs only for the
final proposal. Style parts are `root`, `surface`, `content`, and `handle`.

Source: [resizable.rhai](../../registry/components/resizable.rhai).
Runnable story: `gpui-rhai gallery --story components/resizable`.

## Draggable

`Draggable` controls one `{x,y}` position inside its local boundary. The caller
provides content and may provide a distinct handle node; the component owns the
internal element references. Pointer movement updates two optional-float native
signals and release emits one `move({x,y})` proposal. Axis restriction,
containment, per-axis snapping, drag threshold, keyboard step and disabled state
share the Interaction Runtime used by Resizable and SplitPane.

Source: [draggable.rhai](../../registry/components/draggable.rhai).
Runnable story: `gpui-rhai gallery --story components/draggable`.

## DragSource and DropZone

`DragSource` transfers one bounded typed `UiValue` payload without changing the
source object's position. `DropZone` declares accepted payload types and
`copy`/`move` operations. Target resolution is native and Host-domain scoped;
nested targets use explicit priority and then the smallest matching bounds.
Hover feedback never mutates application data. A successful release invokes
the target's single `drop(...)` proposal and the source's `drag_end(...)`
result; cancellation and rejection leave committed ownership unchanged.

An optional `keyboard_target` lets a focused source invoke the same typed target
contract with Enter/Space. OS/file drops and cross-window transfer remain Host
integration responsibilities.

Sources: [drag_source.rhai](../../registry/components/drag_source.rhai) and
[drop_zone.rhai](../../registry/components/drop_zone.rhai).
Runnable story: `gpui-rhai gallery --story components/drag-drop`.

## Sortable

`Sortable` renders a bounded controlled collection of stable keyed items. Its
native grip interaction resolves before/after insertion anchors in Rust and
emits one `reorder({source_key,anchor_key,placement,x,y})` proposal on release.
It never mutates the caller's canonical order during preview, suppresses self
and adjacent no-op moves, and scrolls the active target's nearest eligible
scroll ancestor at an edge. Focus a grip and use Option/Alt + Arrow, Home, or
End for the same identity-based target contract.

Its vertical virtual mode accepts ordinary keyed data or `NativeCollection`.
The generic virtual renderer supplies stable neighboring keys without Rhai
materializing offscreen nodes. While dragging, the shared Interaction Runtime
pins the active source key and its bounded realization halo even after it
scrolls out of view.

Source: [sortable.rhai](../../registry/components/sortable.rhai).
Runnable story: `gpui-rhai gallery --story components/sortable`.

## PanZoom

`PanZoom` controls a Canvas viewport with `{x,y,scale}`. Pointer pan, anchored
wheel zoom, arrows, `+`/`-`, and reset all produce the same bounded controlled
transform proposal. Four related native signals update translation and scale
atomically; Canvas painting, hit testing, and committed geometry consume the
same affine scale/rotation facts. Ordinary wheel input bubbles by default;
Command/Control + wheel opts into zoom unless the caller explicitly selects
`always`.

The 0.1.8 surface deliberately supports Canvas content, not arbitrary GPUI
subtrees or native input controls. That boundary prevents a visual-only scale
from lying about hit testing and layout.

Source: [pan_zoom.rhai](../../registry/components/pan_zoom.rhai).
Runnable story: `gpui-rhai gallery --story components/pan-zoom`.

## Range slider

`RangeSlider` owns no accepted values. It presents two independently focusable
native slider thumbs over one shared axis, previews pointer movement in Rust,
and emits one controlled `{low,high}` proposal on release or keyboard step.
Values snap to the declared step, never cross, and respect `minimum_gap`.
Horizontal arrow semantics reverse in RTL; Home/End remain thumb-specific.

Source: [range_slider.rhai](../../registry/components/range_slider.rhai).
Runnable story: `gpui-rhai gallery --story components/range-slider`.

## Rotatable

`Rotatable` controls a Canvas angle in degrees around an explicit local pivot.
The native interaction keeps the pivot fixed by atomically updating rotation
and its required translation compensation. Pointer preview stays in Rust;
release and keyboard steps emit one normalized `[0,360)` angle. Optional snap
applies to both pointer and keyboard input.

Source: [rotatable.rhai](../../registry/components/rotatable.rhai).
Runnable story: `gpui-rhai gallery --story components/rotatable`.

## Selection area

`SelectionArea` controls stable Canvas object keys through click, platform
toggle, Shift range, keyboard navigation, and an intersect/enclose marquee.
Targets stay as bounded durable rectangles; only marquee preview runs on the
pointer hot path. Canvas-local coordinate conversion uses the same inverse
affine transform as paint and hit testing, so PanZoom/rotation do not create a
second selection coordinate model.

Source: [selection_area.rhai](../../registry/components/selection_area.rhai).
Runnable story: `gpui-rhai gallery --story components/selection-area`.

## Tree

`Tree` is a controlled virtualized outline. A Rust projection validates stable
keys, parents, cycles and depth, then flattens only expanded branches while
preserving source sibling order. The ordinary public `virtual_collection`
owns realization and reveal; Tree alone owns active, expanded, disabled and
selection navigation state.

Source: [tree.rhai](../../registry/components/tree.rhai).
Runnable story: `gpui-rhai gallery --story components/tree`.

The same [specification](interaction-behaviors.md) records the planned
SelectionArea, Rotatable, and DockLayout
capabilities, their composition rules, and acceptance requirements. These
planned entries are not additional implemented components or callable exports.
In particular, the existing Motion ReorderList animates an externally supplied
order; it does not yet implement interactive sorting.

Every interactive field, choice, menu, navigation region, overlay surface, and
progress indicator has an explicit textual accessible name. Input placeholders
are hints, not names. An Icon without its optional `label` is decorative
presentation; IconButton always requires an action label.

TitleBar can replace the platform title bar. The trusted Host hides the
platform bar (`TitlebarOptions { appears_transparent: true, traffic_light_position, .. }`
and, on macOS, `app_owns_titlebar_drag: true` so AppKit does not claim clicks in
the title strip) and allows window drag areas for the view
(`ScriptViewConfig::window_drag_areas(true)` or
`ScriptApplication::window_drag_areas(true)`). The script passes
`inset_start` (room for the macOS window buttons; the Gallery uses 72 with the
buttons at (12, 11)) and `window_drag: true`: pressing the bar's background
moves the window, a double press runs the platform title-bar action (zoom or
minimize), and a press on a control inside the bar stays with the control.
Window moves are platform drags on macOS and Linux. Without the Host's
permission a drag area is inert, so an embedded user-authored view cannot turn
ordinary content into a window-control surface.

TitleBar requires a textual `label` for accessibility. Its `title` and optional
`subtitle` accept strings or nodes. String values receive the standard
typography/truncation treatment; structured nodes keep their own appearance and
handlers while still participating in TitleBar's start inset and clipping.
Breadcrumb separators remain application-owned rather than becoming TitleBar
policy.

## List

`List` is a controlled virtualized list of keyed rows: Table's rows, selection
and keyboard model without columns or a header. Items are data, as Table cells
are, so long lists realize only the visible rows:

| Item field | Contract |
|---|---|
| `key` | Required stable string |
| `title` | Required string; the row's accessible name starts with it |
| `secondary` | Optional muted text, beside the title or below it (`secondary_layout`) |
| `meta` | Optional trailing text such as a time or a count, tabular figures |
| `badge` | Optional leading status `#{ text, variant, dot }` |
| `disabled` | Optional; a disabled row takes no input and keyboard navigation skips it |

The list takes `selection_mode` (`none` default, `single`, `multiple`),
`selected_keys`, `height` or `fill_height`, `dividers`, `badge_width` (pixels;
one leading slot on every row, so titles align when badges differ), `empty_text` /
`empty`, and emits `selection_change`, `row_click` and `context_request`
(`#{ key, anchor, source }`, like Table's without a column).

Source: [list.rhai](../../registry/components/list.rhai).
Runnable story: `gpui-rhai gallery --story components/list`.

## TabBar

`TabBar` is a controlled strip of document tabs that belong to the panel under
them (the panel is the caller's; give it `tabbar.active` so the selected tab
joins it). Tabs keep their width and scroll sideways; the selected tab stays
revealed.

| Tab field | Contract |
|---|---|
| `value`, `label` | Required strings; the label is the tab's accessible name |
| `icon` | Optional decorative node before the label |
| `closable` | Shows a close button (selected or hovered) and closes on a middle press |
| `dirty` | Shows the unsaved mark, which becomes the close button under the pointer |
| `disabled` | Takes no input; the cursor skips it |

Props: `key`, `label`, `value`, `tabs`, `size`, `start` / `end` (nodes beside
the strip, not scrolled), `overflow_menu` (a menu listing every tab,
`menu_label`), `close_label`, `reorderable`. Events: `change(value)`,
`close(value)`, `context_request(#{ value, anchor, source })` from a right
press or Shift+F10, `reorder(#{ value, anchor, placement })` from a drag (a
press that does not move selects) or Alt+Left/Right. The strip is one tab
stop: the arrows, Home and End move a cursor, Enter or Space selects it.

Source: [tab_bar.rhai](../../registry/components/tab_bar.rhai).
Runnable story: `gpui-rhai gallery --story components/tab-bar`.

## Foundations and status

- `Label`, `Divider`, and `Icon` provide semantic text and visual structure.
- `Avatar`, `Badge`, and `Tag` are distinct: Badge is read-only status,
  while Tag may represent removable application metadata. Badge keeps a compact
  text enclosure relative to Button; see the
  [marker contracts](../design/atoms.md#badge).
- `Alert` is persistent inline feedback; `Toast` is transient layered feedback.
- `Card`, `GroupBox`, and `Empty` standardize common composition without hiding
  their node slots.
- `Kbd`, `Progress`, `Spinner`, and `Skeleton` cover shortcut, determinate,
  indeterminate, and placeholder presentation. Progress exposes an explicit
  `0..max` range plus a bounded pixel width. Spinner animation runs on the
  native runtime clock and settles visibly under reduced motion.
- `TitleBar` and `StatusBar` provide source-owned application chrome with
  logical start/center/end slots. They do not acquire native window authority;
  the Rust Host still owns window configuration, movement, and closing.

## Actions, choices, and forms

- `Button`, `IconButton`, and `ButtonGroup` express actions. `IconButton` owns a
  square hit target and requires an accessible label. Its controlled `selected`
  state uses an accent foreground without adding a container and exposes button
  pressed semantics. `Button` uses its typed `prefix`/`suffix` slots for icons
  mixed with text. `Toggle`/`ToggleGroup` express labeled pressed tool state;
  `Checkbox`, `Radio`/`RadioGroup`, and `Switch` retain their separate selection
  and setting semantics.
- ToggleGroup's segmented appearance is specified in the
  [component contracts](../design/atoms.md#related-togglegroup-and-tabs).
- `Input`, `InputGroup`, `Textarea`, and `FormField` use the retained native
  editing core and explicit semantic relationships. `Input` with
  `appearance: "embedded"` is the frameless search line that heads a panel
  (Command, CommandDialog, a searchable Combobox).
- `Select` is scalar choice. `Combobox` is searchable single/multiple choice.
  Both are strictly controlled for value, open state, and query.
- `DatePicker` remains a controlled ISO-date composition over public calendar
  helpers and Overlay.

## Native interaction foundations

`Slider` is single-value and strictly controlled. Pointer movement updates a
retained Rust preview without invoking Rhai for each move; pointer release and
keyboard steps emit one schema-checked `change(number)` request.

`ScrollArea` decorates an ordinary retained scrollable Box. GPUI owns wheel and
trackpad scrolling; the generic runtime owns themed overlay tracks/thumbs,
dragging, RTL edge placement, and per-axis `auto`, `always`, or `hidden`
visibility. The normal element-ref scroll commands remain available.

Table columns may declare up to four structured `adornments`. Each adornment
reads `text_key` from the row and renders a compact Badge after the cell text;
`variant` is fixed or `variant_key` reads one of `neutral`, `accent`, `success`,
`warning`, or `danger` from the row, and `dot` enables the semantic status dot.
The same declaration works for Array and `NativeCollection` rows. Native data
projects only the realized rows and never invokes a per-cell Rhai renderer, so
status/count pills do not discard Table virtualization or keyboard behavior.
Table also accepts controlled `query`/`search_fields` and one-based
`page`/`page_size`; NativeCollection performs sort, filter, page, and group
ordering in Rust before virtual projection.

## Navigation and data

`Tabs`, `Accordion`, `Collapsible`, `Menu`, and `Pagination` implement their
documented keyboard policies. `Table` and public `virtual_collection` accept
Array or Rust-owned NativeCollection data. `ScrollArea` handles arbitrary
non-virtual content.

### Tabs

`components/tabs` exports controlled `Tabs(props)`. `value` selects one
associated content panel; `on_change` receives a `change(string)` proposal.
The caller owns the accepted value. Clicking or keyboard navigation does not
independently commit a different panel or thumb position.

| Prop | Contract |
|---|---|
| `value` | Required selected string value |
| `label` | Required accessible name for the tab group |
| `tabs` | Required array, at most 128 items |
| `orientation` | `horizontal` (default) or `vertical` |
| `layout` | `content` (default) or `equal` |
| `motion_key` | Optional stable string, at most 128 characters; empty means direct positioning, nonempty enables shared-layout thumb motion and must be unique within the view |
| `on_change` | Optional callback receiving the proposed value |

Each item requires `value: string`, `label: string`, and `content: node`.
Optional `icon: node` provides decorative header content; `label_visible`
defaults to true and may be false only when an icon is provided. A textual
label remains required for icon-only tabs. `disabled` defaults to false.
`content` is the page panel, never an alternate header slot. Header icons
must not add a second action or keyboard entry point.

Tabs retains one keyboard entry point, orientation-aware arrow navigation,
disabled-item skipping, and the runtime's horizontal RTL behavior. It exposes
group/tablist/tab semantics and selected state, rather than button pressed
state. The track and inset selection thumb follow the
[Tabs visual contract](../design/atoms.md#related-togglegroup-and-tabs).

| Style part | Responsibility |
|---|---|
| `root` | List/panel composition |
| `list` | Continuous track, padding, radius, and scrolling |
| `indicator` | Selection thumb fill, border, and radius; no independent action |
| `tab` | Header slot layout and interaction appearance |
| `tab_selected` | Selected header styling without a second thumb/background |
| `label`, `icon` | Header content styling |
| `panel` | Selected page content |

Styles follow source defaults → component stylesheet → instance overrides.
`motion/animated_tabs` exposes the same item, orientation, layout, and controlled
selection contract, requires a stable `key`, and forwards that key as the
inner Tabs' motion identity. Its own style part is `root`. It uses the shared
selection thumb instead of replaying opacity over the entire component.

Current limit: horizontal overflow is locally scrollable but controlled
selection does not automatically reveal an offscreen item.

## Command and CommandDialog

`Command` is an embeddable keyboard-first action search. It performs
deterministic fuzzy matching over labels and keywords, preserves group order,
sorts matches stably within groups, skips disabled items during navigation, and
virtualizes the result.

Group labels are intentionally quieter than actions: regular-weight muted text
sits lower in its row to create more separation from the preceding group, while
the owned command rows use additional logical-start indentation. Structural
group rows are not roving-focus targets. Applications can still replace these
defaults through the `group`, `item`, and `item_active` component parts.

`active_value` is controlled by the caller, so a palette can open with its
highlight seated on the currently selected command. `active_change(string)`
reports explicit roving movement from Up/Down/Home/End
or pointer entry. It is distinct from `action(string)`: use the former for a
reversible preview and the latter for confirmation. Repeated movement onto the
same value is deduplicated. Initial render and query-driven fallback selection
remain pure and do not emit an implicit event.

The active command is also the list's controlled reveal target. Keyboard
roving and Host-driven `active_value` changes keep that item visible even when
it was outside the realized window. Wheel/trackpad scrolling is preserved
across ordinary renders and realization; it is overridden only by the next
active-value or result-key-order change.

Array inputs are ranked in component Rhai. Large NativeCollection inputs are
filtered, grouped, ranked, and navigated in Rust; only visible projected rows
cross into Rhai. Array items keep a required textual `label` for filtering and
accessibility, and may add `content: node` for two-tone labels, icons, badges,
or other presentation. The content node never becomes the search or accessible
name implicitly. `CommandDialog` forwards the same controlled `active_value`,
composes the behavior with Dialog, and
forwards `query_change`, `active_change`, and `action` through formal component
events. It does not register a global shortcut. The Host action/keybinding
system owns the shortcut that changes its controlled `open` value.

## CodeViewer and DiffViewer

`CodeViewer` is a virtualized native read-only source surface with built-in
Rhai, Rust, Go, web/configuration and common scripting language definitions.
It accepts a direct string or tracked `NativeTextDocument`, supports continuous
selection, original-text copying, search, line navigation, wrapping, and
one-based location activation without becoming an editor.

`DiffViewer` compares neutral left/right descriptors in unified or split form.
Line hunks, Unicode-grapheme intraline changes, context folds, synchronized
vertical layout, per-pane horizontal scrolling, search and patch copying stay
in Rust. No row renderer or complete diff payload crosses into Rhai. See the
[document viewer overview](../document-viewers.md) for Host registration,
resource limits and extension points.

## Overlay family

- `Popover` and `Tooltip` are anchored non-modal surfaces.
- `Menu` is a trigger-based action menu; `ContextMenu` reuses the same item,
  submenu, typeahead, and roving model while anchoring at a secondary click.
- `Dialog` is a general modal; `AlertDialog` adds explicit confirmation and
  cancellation semantics.
- `Sheet` is a temporary modal attached to logical start/end or physical
  top/bottom. Persistent sidebars remain normal application layout.

Overlay open values are controlled. Portal order, outside/Escape dismissal,
focus containment/restoration, pointer coordinates, and viewport placement are
generic Host behavior.

## Optional Chart source pack

The chart pack is versioned separately from the frozen 51-component foundation
and requires the optional Cargo `charts` feature. It provides formal `Chart`,
`BarChart`, `LineChart`, `PieChart`, and `MapChart` source components. The
generic module also exports single-kind Area, Scatter, Heatmap, Candlestick,
Donut, Radar, Gauge, Funnel, GeoScatter, and GeoLines adapters over the public
native Chart primitive. Mixed series and multiple coordinate regions use
`Chart` directly. See the [Chart Runtime guide](../charts.md).
