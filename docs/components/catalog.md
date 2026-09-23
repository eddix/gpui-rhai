# Official component catalog

gpui-rhai ships 51 editable Rhai source components. They all use the same
public atoms and generic runtime mechanisms available to application code; no
official component receives a private high-level node constructor.

Run the interactive catalog from this repository:

```text
cargo run -p gpui-rhai --example component_gallery
```

Theme Studio renders the same exhaustive specimen while editing a theme.

The implemented 0.1.5 visual revision for Tabs, Button, and Badge is documented in the
[control visual specification (简体中文)](control-visual-spec.zh-CN.md).
It includes dimensions, theme roles, props/style parts, motion, and native
acceptance criteria. ToggleGroup notes remain future design reference.

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
  while Tag may represent removable application metadata. The accepted Badge
  revision tightens its text enclosure relative to Button: medium Badge uses
  a line-height-plus-4px box and 6px horizontal padding per side, independently
  of whether its appearance is filled, subtle, or outlined.
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
- The deferred ToggleGroup visual target uses equal-size button segments and
  a single 1px separator at each internal edge. Its selected fill belongs to
  each pressed segment, unlike the single inset thumb used by Tabs. Ordinary
  ButtonGroup does not acquire an equal-width requirement.
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

Tabs now replaces the selected rail with a continuous
track and a single inset thumb. Unselected items have no button container or
separator. Content-width and explicit equal-width layouts, icon headers, the
3px inset, and target `indicator` / `tab_selected` parts are specified in the
[control visual specification](control-visual-spec.zh-CN.md); new fields and
parts are available from the 0.1.5 source schema.

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
