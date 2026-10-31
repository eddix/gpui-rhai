# Official component catalog

gpui-rhai ships 73 editable Rhai source components. They all use the same
public atoms and generic runtime mechanisms available to application code; no
official component receives a private high-level node constructor.

This page records how components differ and what their contracts promise. The
props, events, slots and parts of every module, with their types, defaults and
meaning, are in the generated [module reference](../reference/README.md).

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

The [design specification](../design/) is the maintained source for component
dimensions, color roles and state appearance. This catalog records component
semantics and public contracts.

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
A decorative grip (`grips`) covers its handle, so an edge grip shows the
handle's resize cursor itself; a corner grip keeps the default cursor, since
styles offer no diagonal resize cursor.

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

The same [specification](interaction-behaviors.md) records composition rules
and acceptance requirements for these interaction components, as well as the
planned DockLayout. DockLayout is not an implemented component or callable export.
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
`selected_keys`, `height` (pixels or a Length, such as
`theme_length("metrics.row") * 6` for six rows in either density) or
`fill_height`, `dividers`, `badge_width` (pixels or a Length; one leading slot
on every row, so titles align when badges differ), `loading` / `loading_content`
(shown in the list's place, as in Table), `empty_text` / `empty`, and emits
`selection_change`, `row_click` and `context_request`
(`#{ key, anchor, source }`, like Table's without a column; without a selection
mode, `on_context_request` alone makes the list a tab stop whose Shift+F10 asks
for the whole list). A row carries only its selected flag and a click builds the
next selection when it happens, so the row data stays linear in the items
however many are selected. A click leaves the keyboard focus on the list, so
the arrows continue from the clicked row and its focus frame shows.

Source: [list.rhai](../../registry/components/list.rhai).
Runnable story: `gpui-rhai gallery --story components/list`.

## TabOverflowMenu

`TabOverflowMenu` is a chevron button whose menu lists every tab of a strip;
picking a row reports its `value`. It owns the menu's open state and active
row, so the tabs beside it stay stateless: `Tabs` uses it for
`overflow_menu`, and a caller may put it beside any strip of tabs.

Props: `key` (required), `label` (the button's and the menu's accessible
name), `value` (where the highlight starts each time the menu opens), `items`
(`value`, `label`, `disabled`, in strip order), `on_pick(value)`; the menu
closes after a pick. Style part: `trigger`.

Source: [tab_overflow_menu.rhai](../../registry/components/tab_overflow_menu.rhai).

## SlidingPanel

`SlidingPanel` shows `content` and, when `value` changes, slides the new
content in from the side of the one it replaces as it fades in: a short way
(`motion_distance("moderate")`, 12px by default) in the `fast` duration, with
the `entrance` easing. `order` lists every value: content after the one it
replaces comes from the end side, content before it from the start side; when
either value is missing from `order` the content comes in without moving, as
the first content does. `orientation: "vertical"` slides
from above or below; `wrap` treats `order` as a loop and takes the shorter way
round. Right-to-left text mirrors the horizontal slide, and the Host's reduced
or no-motion policy settles it.

Only the new content moves; the one it replaces is gone at once. Nothing is
painted after it leaves, so any content can slide, native text fields and
virtual collections included (exit motion cannot paint a copy of those). It
remembers the value it showed last, so it needs `key`; the components around it
stay stateless. Tabs uses it for `panel_motion: "slide"`, Carousel for its
items. Style parts: `root` (stays put and carries the caller's surface) and
`slide` (moves).

Source: [sliding_panel.rhai](../../registry/components/sliding_panel.rhai).

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
- `Select` is scalar choice. `Combobox` is searchable single/multiple choice;
  with `on_submit`, Enter in its search field submits typed free text when no
  enabled option matches. Both are strictly controlled for value, open state,
  and query.
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

`Tabs`, `Carousel`, `Accordion`, `Collapsible`, `Menu`, and `Pagination`
implement their documented keyboard policies. `Table` and public `virtual_collection` accept
Array or Rust-owned NativeCollection data. `ScrollArea` handles arbitrary
non-virtual content.

### Tabs

`components/tabs` exports controlled `Tabs(props)`: one component for every
strip of tabs, in three looks that share every capability. `value` selects a
tab; `on_change` receives a `change(string)` proposal. The caller owns the
accepted value. Clicking or keyboard navigation does not independently commit
a different panel or thumb position.

| Variant | Look | Choose it for |
|---|---|---|
| `track` (default) | a tonal track with a raised thumb | a compact switcher among other controls: a toolbar, a header |
| `underline` | labels on a `border` hairline, a 2px `text_primary` bar under the selected one | the sections a panel or a card is divided into |
| `bar` | document tabs on `tabbar.background`; the selected tab takes `tabbar.active` and covers the strip's line | the region itself: open documents, a workspace's panels, a collection the user manages |

| Prop | Contract |
|---|---|
| `value` | Required selected string value; a value no tab has (`""`) selects none |
| `label` | Required accessible name for the tab group |
| `tabs` | Required array, at most 256 items; may be empty |
| `variant` | `track` (default), `underline` or `bar`; `underline` and `bar` are horizontal only and throw with `orientation: "vertical"` |
| `orientation` | `horizontal` (default) or `vertical` |
| `layout` | `content` (default): tabs keep their width, up to two label columns, and the strip scrolls; `equal`: horizontal tabs share the width |
| `panel`, `keep_mounted` | Render the selected tab's `content` (default); keep every supplied panel mounted |
| `panel_motion` | `none` (default) switches at once; `slide` brings a newly selected panel in from the side of the tab it replaces (from above or below when vertical) through `SlidingPanel`; needs `key` |
| `start`, `end` | Up to 8 nodes before the list and at the far end of the strip (above and below when vertical), outside the list, each its own tab stop |
| `on_add`, `add_label` | Draws a "+" `IconButton` right after the last tab, pinned at the strip's end when the tabs overflow |
| `overflow_menu`, `menu_label` | A `TabOverflowMenu` listing every tab after the add button; needs `key` |
| `reorderable`, `on_reorder` | Tabs move by drag or Alt+Arrow; `reorder(#{ value, anchor, placement })`; needs `key` |
| `on_close`, `close_label` | `close(#{ value, next })` from a close button, a middle press or Delete; `next` is the enabled tab after it, else the one before it, else `""` |
| `on_context_request` | `context_request(#{ value, anchor, source })` from a right press, or Shift+F10 or the menu key on the selected tab |
| `motion_key` | Optional stable string, at most 128 characters; nonempty slides the selection indicator and focus frame and must be unique within the view |
| `size`, `key` | Size environment override; stable identity, required with `reorderable`, `overflow_menu` or a sliding `panel_motion` |
| `on_change` | Optional callback receiving the proposed value; without it the tabs do not select |

Each item requires `value: string` and `label: string`. Optional fields:

| Item field | Contract |
|---|---|
| `icon`, `label_visible` | A decorative node at icon size before the label; a label may hide only when an icon is given, and stays the accessible name |
| `count` | A capsule after the label, as tall as the label (`metrics.counter`, the control line height) and at least as wide, in the `counter` type with tabular figures; `99+` past 99, zero shown; corners follow `radius.sm`, the markers' capsule. The full number joins the accessible name (`"Repos, 12"`) |
| `tone`, `tone_label` | `accent`, `warning` or `danger`: fills the capsule (with the matching `on_*` text), or without a `count` shows a 6px lamp after the label. `tone_label` joins the accessible name, since a lamp has no text; the component invents no copy |
| `closable` | A close button in a reserved square, shown while the tab is selected or hovered |
| `dirty` | The 6px unsaved mark in the same square; on a closable tab it turns into the close button under the pointer |
| `content` | The page panel, never an alternate header slot |
| `disabled` | Ignores the pointer; the arrows skip it |

A tab carries only what the component draws: icon, label, count or lamp,
dirty mark and close button. None of them is a tab stop; the list reaches each
one. The strip is one tab stop and the arrows select: Left/Right (Up/Down when
vertical), Home and End; Delete or Backspace closes the selected tab;
Alt+Arrow moves it; Shift+F10 or the menu key asks for its context menu.
Actions that are not about one tab (add, refresh, remove the current one, the
previous and next tab) go in `start` and `end`, where each control keeps its
own tab stop. On an unfilled strip a slot's space faces the list only, so its
first control sits on the content edge of the panel below; the filled bar
insets its slots on both sides. `close_successor(tabs, value)` is exported for
a caller that closes the selected tab from such a slot.

The strip scrolls along its axis (a vertical wheel scrolls a horizontal strip)
and keeps the selected tab revealed, also when the caller reorders, adds or
removes tabs while it is in view; a strip scrolled away from it by hand stays
where it is. Tabs is stateless, so it needs no `key` unless it is reorderable,
has an overflow menu or slides its panels; it reveals through an effect, so it renders inside a
mounted view (a `ScriptLifecycle`), as every application does.

Tabs exposes group/tablist/tab semantics and selected state, rather than
button pressed state. The looks follow the
[Tabs visual contract](../design/atoms.md#related-togglegroup-and-tabs). With a
panel an underline or bar strip spans the root, so its line rules the whole
panel; the underline panel sits `space.related` below the hairline, the bar's
panel joins the strip with no gap on `tabbar.active`; without a panel the
strip follows the caller's alignment.

| Style part | Responsibility |
|---|---|
| `root` | Strip/panel composition |
| `strip` | The row (column when vertical) of slots, list, add button and menu; the bar's surface |
| `list` | The scrolling tablist; the track's continuous track, padding and radius |
| `line` | Underline and bar: the 1px `border` line under the strip; restyle it to remove the line |
| `indicator` | Track: selection thumb fill, border and radius. Underline: the 2px bar under the selected label. No independent action |
| `focus_frame` | Underline and bar: the 2px focus frame over the selected tab |
| `tab`, `tab_selected` | Header slot layout and interaction appearance; selected styling without a second thumb/background |
| `label`, `icon`, `counter`, `lamp` | Header content |
| `slot`, `close`, `dirty` | The reserved square, its close button and its unsaved mark |
| `add`, `menu`, `start`, `end` | The add button, the overflow menu and the slots |
| `panel` | Selected page content |

Styles follow source defaults → component stylesheet → instance overrides.
`motion/animated_tabs` exposes the item, orientation, layout, variant,
`panel_motion` and controlled selection contract, requires a stable `key`, and forwards that key as the
inner Tabs' motion identity; collection features are used on Tabs itself with
`motion_key`. Its own style part is `root`. It uses the shared selection
indicator instead of replaying opacity over the entire component.

Source: [tabs.rhai](../../registry/components/tabs.rhai).
Runnable stories: `gpui-rhai gallery --story components/tabs` and
`--story components/tabs-collection`.

### Carousel

`components/carousel` exports controlled `Carousel(props)`: a pager that shows
one item at a time, for a compact place that steps through items of one kind
(alerts, cards, previews). `index` selects the item; arrows, dots and keys
propose another through `change(integer)`, and the caller stores it. It never
turns by itself.

| Prop | Contract |
|---|---|
| `key`, `label` | Required: identity (it names the sliding panel and keeps the focus state) and the accessible name of the carousel and its controls |
| `index` | Required position, from 0; an index past the items throws |
| `items` | Required, 1 to 64 `#{ label, content }`; `label` names the item, its dot and the controls' value, and only the shown `content` renders |
| `variant` | `header` (default): the label as the title and the controls in a row above the item. `overlay`: arrows over the item's sides, the position below it |
| `indicator` | `count` (`2 / 5`), `dots` (one per item, a press selects it) or `none`; unset, `count` for `header` and `dots` for `overlay` |
| `wrap` | Steps past the last item to the first and back; without it the arrow at an end is disabled (`header`) or left out (`overlay`) |
| `label_visible` | `header` shows the label as its title (default); `false` keeps it only as the accessible name |
| `previous_label`, `next_label` | The arrows' accessible names, `Previous` and `Next` by default |
| `size`, `on_change` | Size environment override; without `on_change` the controls are inert |

The controls are one tab stop beside the item, never around it, so keys typed
in an item (a text field, a list) stay the item's: Left and Right step, Home and
End go to the ends. The arrows and the dots are pointer targets reached from
it, not tab stops, so a disabled arrow never holds focus. In `header` the
controls are the arrows and the position after the title; in `overlay` they
are the position under the item (with `indicator: "none"` they take no room),
and the arrows over the item show while it is hovered or the controls, or an
arrow, hold focus. The tab order follows the reading order: an overlay item
comes before its controls.

A new item slides in from the side it comes from through `SlidingPanel`, the
shorter way round with `wrap`, mirrored in right-to-left text. Its height is
the shown item's, so a caller that needs a steady height sets one on the root.
The count uses plain figures, so the carousel needs no locale. The looks follow
the [Carousel visual contract](../design/atoms.md#carousel).

| Style part | Responsibility |
|---|---|
| `root` | The column of the header or stage and the controls |
| `header`, `title` | The `header` row and its title |
| `controls`, `focus_frame` | The tab stop and the 2px frame its focus draws |
| `previous`, `next` | The arrows |
| `count`, `dots`, `dot`, `dot_selected` | The position |
| `stage`, `arrows` | `overlay`: the item with its arrow layer, which is the hover group |
| `viewport`, `item` | The sliding panel and the shown item |

Source: [carousel.rhai](../../registry/components/carousel.rhai).

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

## Retained pages and drill-down navigation

`Tabs(#{ keep_mounted: true, ... })` retains every supplied panel, including
native scroll position, caret/selection and undo. Hidden panels take no layout,
input traversal or presented accessibility space. The default remains selected
presentation only; eagerly constructed components still retain caller-owned
Rhai state/effects even when not presented (ADR 0019).

`patterns/navigation_stack.NavigationStack` takes `stack: [{key,title,content}]`
in root-to-top order and `on_pop({depth,source})`. Keys identify visits, so two
visits to the same record use distinct keys and distinct component keys. The
caller accepts a proposal by truncating its controlled stack. `max_depth`
rejects excess entries; it never silently evicts drafts. Back, breadcrumb and
none headers do not change page identity. Breadcrumb overflow uses Menu, and
the Host's motion and RTL policies apply.

Hiding is not view suspension and never automatically cancels business tasks
or subscriptions. `ctx.is_visible()` reads committed placement (false before
initial placement), invalidates render readers on changes, and lets the
application explicitly gate polling/effect dependencies. Gallery pages
`retained_tabs` and `navigation_stack` demonstrate local state and real native
editing/scrolling.

## Overlay family

- `Popover` and `Tooltip` are anchored non-modal surfaces.
- `Menu` is a trigger-based action menu; `ContextMenu` reuses the same item,
  submenu, typeahead, and roving model while anchoring at a secondary click,
  or at its trigger on Shift+F10 or the menu key. Activating any row closes
  either menu.
- `Dialog` is a general modal; `AlertDialog` adds explicit confirmation and
  cancellation semantics: Escape and backdrop presses report `cancel` too.
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
