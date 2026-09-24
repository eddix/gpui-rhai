# Official component catalog

gpui-rhai ships 51 editable Rhai source components. They all use the same
public atoms and generic runtime mechanisms available to application code; no
official component receives a private high-level node constructor.

Run the interactive catalog from this repository:

```text
cargo run -p gpui-rhai --example component_gallery
```

Theme Studio renders the same exhaustive specimen while editing a theme.

The [registry visual system](../registry-design-system.md) is the maintained
source for component dimensions, color roles, state appearance, and known
visual gaps. This catalog records component semantics and public contracts.

Version 0.1.2 freezes this 51-component foundation: component IDs and exports,
controlled-state ownership, semantic event payloads, the `xs`/`sm`/`md`/`lg`
size vocabulary, and declared style parts are the maintained base contract.
Future catalog additions must compose the same public atoms and generic runtime
mechanisms; they do not justify parallel private primitives.

Every interactive field, choice, menu, navigation region, overlay surface, and
progress indicator has an explicit textual accessible name. Input placeholders
are hints, not names. An Icon without its optional `label` is decorative
presentation; IconButton always requires an action label.

For a custom macOS titlebar, configure GPUI's transparent `TitlebarOptions` in
the trusted Host and pass `inset_start: 70` to the Rhai `TitleBar`. Rendering a
bar alone deliberately does not change native window behavior. This keeps an
embedded user-authored view from turning ordinary content into a window-control
surface.

TitleBar requires a textual `label` for accessibility. Its `title` and optional
`subtitle` accept strings or nodes. String values receive the standard
typography/truncation treatment; structured nodes keep their own appearance and
handlers while still participating in TitleBar's start inset and clipping.
Breadcrumb separators remain application-owned rather than becoming TitleBar
policy.

## Foundations and status

- `Label`, `Divider`, and `Icon` provide semantic text and visual structure.
- `Avatar`, `Badge`, and `Tag` are distinct: Badge is read-only status,
  while Tag may represent removable application metadata. Badge keeps a compact
  text enclosure relative to Button; see the
  [density metrics](../registry-design-system.md#button-and-badge-density).
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
- ToggleGroup's future segmented appearance is recorded under
  [known visual gaps](../registry-design-system.md#known-implementation-gaps);
  it is not part of the current component contract.
- `Input`, `InputGroup`, `Textarea`, and `FormField` use the retained native
  editing core and explicit semantic relationships.
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
[Tabs visual contract](../registry-design-system.md#tabs-track-and-selection-thumb).

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
selection does not automatically reveal an offscreen item. See the visual
system's [known implementation gaps](../registry-design-system.md#known-implementation-gaps).

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
