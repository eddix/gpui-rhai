# Official component catalog

gpui-rhai ships 48 editable Rhai source components. They all use the same
public atoms and generic runtime mechanisms available to application code; no
official component receives a private high-level node constructor.

Run the interactive catalog from this repository:

```text
cargo run -p gpui-rhai --example component_gallery
```

Theme Studio renders the same exhaustive specimen while editing a theme.

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
  while Tag may represent removable application metadata.
- `Alert` is persistent inline feedback; `Toast` is transient layered feedback.
- `Card`, `GroupBox`, and `Empty` standardize common composition without hiding
  their node slots.
- `Kbd`, `Progress`, `Spinner`, and `Skeleton` cover shortcut, determinate,
  indeterminate, and placeholder presentation. Spinner animation runs on the
  native runtime clock and settles visibly under reduced motion.
- `TitleBar` and `StatusBar` provide source-owned application chrome with
  logical start/center/end slots. They do not acquire native window authority;
  the Rust Host still owns window configuration, movement, and closing.

## Actions, choices, and forms

- `Button` and `ButtonGroup` express actions; `Toggle`/`ToggleGroup` express
  pressed tool state; `Checkbox`, `Radio`/`RadioGroup`, and `Switch` retain
  their separate selection and setting semantics.
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

## Command and CommandDialog

`Command` is an embeddable keyboard-first action search. It performs
deterministic fuzzy matching over labels and keywords, preserves group order,
sorts matches stably within groups, skips disabled items during navigation, and
virtualizes the result.

`active_value` is controlled by the caller, so a palette can open with its
highlight seated on the currently selected command. `active_change(string)`
reports explicit roving movement from Up/Down/Home/End
or pointer entry. It is distinct from `action(string)`: use the former for a
reversible preview and the latter for confirmation. Repeated movement onto the
same value is deduplicated. Initial render and query-driven fallback selection
remain pure and do not emit an implicit event.

Array inputs are ranked in component Rhai. Large NativeCollection inputs are
filtered, grouped, ranked, and navigated in Rust; only visible projected rows
cross into Rhai. `CommandDialog` forwards the same controlled `active_value`,
composes the behavior with Dialog, and
forwards `query_change`, `active_change`, and `action` through formal component
events. It does not register a global shortcut. The Host action/keybinding
system owns the shortcut that changes its controlled `open` value.

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
