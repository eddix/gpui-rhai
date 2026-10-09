# Component contracts (L1)

For authors of official and application components. Read
[principles.md](principles.md) first. This document defines what every
component must guarantee so that applications can compose them into aligned,
keyboard-friendly screens without fine tuning, and so that every opinionated
default can still be turned off.

Numbers below are the default values of the L0 token base
(`registry/tokens.rhai`, installed as `ui/tokens.rhai`). Components never
repeat these numbers: they reference the token role, so a Host or profile
override changes every component together.

## 1. Composition contracts

Every component in `components/` satisfies these contracts. The checklist in
section 9 repeats them for reviews.

| # | Contract | Requirement |
|---|---|---|
| C1 | **Size** | Every interactive component accepts `size` (`xs` `sm` `md` `lg`, default inherited, then `md`). An explicit prop is applied by setting the `size` environment value on the component's own root, so the nearest value wins and descendants follow. |
| C2 | **Density** | Heights, insets and spacing come from environment-variant tokens (`metrics.*`, `space.*`). No literal control or row height. Density never changes font size. |
| C3 | **Content inset** | In list-like rows (Menu, Command, Select options, Combobox options, Tree, Table cells, Accordion headers, StatusBar fields, navigation lists) text starts at the container edge plus `metrics.inset`. Panels pad with `metrics.inset`. Tree depth indentation is added after the inset. |
| C4 | **Intrinsic width** | Components size to their content and never stretch themselves. Stretching is the parent layout's decision. |
| C5 | **No outer margin** | Components never add space outside their own root. Spacing between components belongs to the parent. |
| C6 | **Shared line box** | Controls of the same size use the same typography role, so text in a row of mixed controls shares one baseline when centered. Minimum heights are lower bounds; larger type grows the control. |
| C7 | **Declared consumption** | Component metadata lists the tokens (`tokens`) and environment values (`environment`) it reads. Preparation validates the active theme against the union of mounted components. |
| C8 | **Removable signatures** | Each structural signature is a declared part (`indicator_bar`, `lamp`, `facet`, `shortcut`, `label`, `cursor`, `focus_frame`), so `ui/styles.rhai` can restyle or hide it. |
| C9 | **States without layout shift** | State priority is `active > focus > hover/cursor > selected > idle`; `disabled` suppresses interaction and wins over pointer states. Borders are reserved in idle geometry. Selection stays visible under focus. |
| C10 | **Visible focus** | Focus is always a 2px `focus_ring` frame (the ink color by default) and never moves layout. A control frames itself with a reserved 2px border that idles in its fill or, for outline treatments, in `border`. A compound control frames its mark instead of the whole row (`group_focus`), a list frames its cursor row with an overlay, and a field group frames itself while its input has focus (`focus_within`). See section 10. |
| C11 | **Inherited disabled** | A disabled container disables every descendant natively: input, focus, pseudo styles and accessibility semantics. Components do not need to forward `disabled` to children. |
| C12 | **Action binding** | Components that trigger commands (Button, IconButton, Menu items, Command items, ContextMenu items, Tooltip hints) accept `action: "id"`. The displayed shortcut, enabled state and dispatch derive from that action. A free-text `shortcut` remains for commands without an action. |
| C13 | **Data defaults** | When a component knows a value is numeric (a Table column with `numeric: true`, Stat, DescriptionList numeric values), it aligns to the end and enables tabular figures by default. |
| C14 | **Text safety** | Text never clips CJK fallback glyphs. Truncated text keeps its full accessible name. Components do not invent copy or functional emoji. |

## 2. Geometry

### Metrics

| Token | comfortable | compact | Use |
|---|---:|---:|---|
| `metrics.control` by size `xs` / `sm` / `md` / `lg` | 24 / 28 / 32 / 36 | 20 / 24 / 28 / 32 | Button, inputs, Select, Toggle, Tabs slots |
| `metrics.control_pad` by size | 8 / 12 / 16 / 20 | 8 / 8 / 12 / 16 | horizontal padding of pressed controls: Button, Tabs, ToggleGroup |
| `metrics.field_pad` by size | 5 / 7 / 10 / 12 | 3 / 4 / 6 / 10 | horizontal padding of fields (section 7) |
| `metrics.multiline_pad` by size | 0 / 1 / 3 / 5 | 0 / 0 / 0 / 3 | vertical padding of Textarea (section 7) |
| `metrics.row` | 32 | 28 | Menu, Command, option lists, Table, Tree, navigation rows |
| `metrics.inset` | 12 | 8 | text start in rows, panel padding |
| `metrics.marker` | 20 | 20 | Badge, Tag, Kbd height (`metrics.marker_small` 18) |
| `metrics.mark` | 16 | 16 | Checkbox box, Radio ring, Switch track height, slider cap |
| `metrics.icon` by size | 12 / 14 / 16 / 16 | 12 / 14 / 14 / 16 | control icon box |
| `metrics.titlebar` | 36 | 32 | TitleBar |
| `metrics.statusbar` | 24 | 22 | StatusBar |

Structural constants that stay literal in component source, with a comment:
the 1px hairline, the 2px focus border, the 2px indicator bar, the 6px lamp,
the 4px slider track, the 8px splitter grab zone, the 12px close glyph, zero
offsets, semantic circles. The registry lint
(`components_declare_what_they_read_and_keep_geometry_in_tokens`) rejects any
other literal height or font size, and any token a component reads without
declaring it.

### Radius roles and the corner style

Corners are an environment axis, `corners`, like density: `square` (the
default and the design language), `subtle` and `round`. Every role is zero in
`square`, so choosing nothing changes nothing.

| Role | Used by | square | subtle | round |
|---|---|---:|---:|---:|
| `radius.xs` | Checkbox box, Kbd keycap | 0 | 2 | 4 |
| `radius.sm` | Tag (and its facet and close target), strong Badge, Combobox chips, Switch track and thumb | 0 | 2 | capsule |
| `radius.md` | Button, IconButton, Input, Select and DatePicker triggers and day cells, ToggleGroup, ButtonGroup, InputGroup, horizontal Tabs | 0 | 4 | capsule |
| `radius.lg` | Card, Alert, Toast, Dialog, Popover, Menu, Command and field panels, Tooltip, Textarea, vertical Tabs, ToggleGroup and ButtonGroup | 0 | 4 | 8 |

"Capsule" is 999px: radii clamp to half the shorter side, so a control
becomes a capsule and a square control (IconButton, a day cell) a circle.
Panels stay at 8px in `round` because their rows are inset 4px from the edge;
a larger radius would show a highlighted first or last row's corner outside
the curve (clipping is rectangular). Joined groups round only their outer
corners: the first and last items use `radius_start` / `radius_end`, so a
pressed segment or a Tag facet never pokes past the frame.

Some shapes do not follow the corner style, on purpose:

- Checkbox stays a box (`xs`): a round checkbox reads as a Radio.
- Status lamps stay square: they are a recognizability signature (G2).
- Slider and RangeSlider caps stay rectangular faders on square tracks, and
  Progress stays a square bar.
- Tables, lists, trees and their selection rows stay square; the indicator bar
  needs a straight start edge.

Semantic circles (Radio, Avatar, presence) use half their size in every style.

## 3. Typography

Roles are named freely at the runtime level; the design language declares:

| Role | Size / line | Weight | Family | Use |
|---|---:|---:|---|---|
| `caption` | 12 / 18 | 400 | UI | descriptions, helper text, summary rows; smallest text |
| `body_small` | 13 / 20 | 400 | UI | compact-mode cells, xs controls |
| `body` | 14 / 22 | 400 | UI | content, controls, table cells |
| `subtitle` | 16 / 24 | 600 | UI | section titles |
| `title` | 18 / 26 | 600 | UI | panel and page titles |
| `heading` | 20 / 28 | 600 | UI | prominent titles |
| `display` | 24 / 32 | 600 | UI | stat figures |
| `display_large` | 32 / 40 | 600 | UI | hero figures |
| `label` | 12 / 16 | 400 | mono | label voice (uppercase Latin) |
| `code` | 13 / 20 | 400 | mono | code, commands |
| `control_small` | 13 / 16 | 400 | UI | single-line text in `xs` controls |
| `control_regular` | 14 / 20 | 400 | UI | single-line text in `sm` to `lg` controls |
| `control` by size | `control_small` for `xs`, `control_regular` otherwise | — | UI | text inside controls |

Control text keeps the body sizes on a tighter line box: a single line needs no
reading leading, and the reserved 2px focus border must fit every compact height
(20 = 16 + 2 × 2). Density still never changes font size. Multi-line fields
(Textarea) keep `body`.

Only weights 400 and 600 are used. 600 marks titles and the label of a filled
Button; data in lists is never bold. The UI family is the platform font; the
mono family defaults to `Menlo`, then `DejaVu Sans Mono`. Identifiers and
numbers stay in the UI family with `tnum` (tabular figures) and `cv08`
(slashed zero) features; the mono family is for code and the label voice.

The label voice uppercases Latin text in the component (`to_upper`); CJK text
is unaffected. GPUI 0.3.7 has no letter spacing, which is why the label voice
uses the monospace family: its built-in advance keeps uppercase legible.

## 4. Color roles in components

| Role | Component use |
|---|---|
| `surface` | window, panel and overlay canvas |
| `surface_raised` | input and field wells, raised panels, Tabs thumb |
| `surface_hover` | tonal blocks: secondary Button, Tag, Tabs track, hover and cursor rows |
| `selection` | persistent selection and the current item block |
| `accent` | primary Button fill, indicator bar, checked controls, caret, progress |
| `accent_hover` | primary Button hover |
| `on_accent`, `on_danger`, `on_warning`, `on_success` | text and marks on the matching fill |
| `danger`, `warning`, `success` | status lamps, strong Badges, status Buttons, invalid frames |
| `text_primary` | content and data |
| `text_muted` | secondary data, descriptions, unselected tab labels, label voice |
| `disabled` | disabled text and marks |
| `border` | hairlines, field frames, outline treatments, Kbd frames |
| `focus_ring` | the 2px focus border |

Derived roles from the token base: `text.accent`, `text.danger`,
`text.warning`, `text.success` (the status color adjusted to reach 4.5:1 on
tonal blocks, used whenever a status or accent color is text), `control.hover`
(tonal block hover step), `tag.facet` (the darker tonal step of a Tag facet),
`table.selection`, `tabs.foreground`, `scrollbar.thumb` and
`scrollbar.thumb_hover`. See [themes.md](themes.md).

## 5. Marker system: Button, Tag, Badge, Kbd

The four rectangle-and-text components are distinguished by silhouette, not by
color alone; the distinction survives grayscale.

| Component | Silhouette | Height | Text | Fill and line |
|---|---|---|---|---|
| **Button** | large block, centered label | `metrics.control` | `control`, 600 | solid or tonal block; outline uses a frame; ghost has no fill |
| **Tag** | small flat tape, optional facet segment | `metrics.marker` | `caption`, 400 | tonal block, no frame |
| **Badge** | lamp plus text, no container by default | `metrics.marker` | `caption`, 400 | none; `strong` emphasis is a solid status block |
| **Kbd** | keycap | `metrics.marker` | `code` at 12px | 1px `border` frame, no fill; inline legend has no frame |

### Button

Seven variants remain. Selection guidance lives in [composition.md](composition.md).

| Variant | Idle | Hover | Text |
|---|---|---|---|
| `primary` | `accent` fill | `accent_hover` | `on_accent` |
| `secondary` | `surface_hover` fill | `control.hover` | `text_primary` |
| `outline` | transparent, 2px `border` frame | `surface_hover` fill | `text_primary` |
| `ghost` | transparent | `surface_hover` fill | `text_primary` |
| `danger` / `warning` / `success` | status fill | the status color mixed 12% toward `text_primary` | matching `on_*` |

- Padding `metrics.control_pad`; gap between icon and label `spacing.xs`.
- Focus: the reserved 2px border turns `focus_ring`.
- Active: opacity 0.86. Disabled keeps the variant's silhouette and its size:
  block variants become a neutral `surface_hover` block, `outline` keeps its
  `border` frame, `ghost` stays bare; the text turns `disabled`.
- Loading keeps the width of the idle label where possible and announces the
  loading text.
- Optional `shortcut` part shows the action's key legend after the label in
  the `code` role at 70% opacity. It shows at every size: a component cannot
  read the resolved size, so an `xs` caller omits the legend itself.
- IconButton is the square variant (`metrics.control` on both axes, icon
  `metrics.icon`); `selected` fills it with `selection`.
- Toggle is a ghost Button whose pressed state fills with `selection`; its label
  stays at 400 so pressing never changes width.
- ButtonGroup joins blocks with a 2px seam (`space.xxs`), so each button keeps
  its own focus frame and outline frames never double.

### Tag

- Tonal `surface_hover` block, horizontal padding `spacing.sm` in both
  densities (markers do not change with density), `radius.sm`.
- Color variants (`accent`, `success`, `warning`, `danger`) mark a
  **category**, not a status: without a facet they color the text with the
  derived `text.*` role; with a facet they fill the facet segment with the color
  and use the matching `on_*` text.
- `facet` prop: a leading segment in the label voice (mono, uppercase) on a
  darker tonal step (`tag.facet`), joined to the value with no gap
  (`Tag(#{ facet: "env", text: "prod" })`). The prop is not called `key`
  because `key` is the component instance key.
- `closable`: a 12px close glyph at the end with its own hover block and a
  2px focus frame; announces `close_label`, default "Remove <text>".

### Badge

- Default (`emphasis: "subtle"`): a 6px square lamp in the variant color
  (neutral uses `text_muted`) followed by the label in `text_primary`. No
  container, no frame.
- `emphasis: "strong"`: a solid block in the status color with `on_*` text,
  weight 600, padding `spacing.sm`. Use only for states that require action.
- `dot: false` removes the lamp (text-only status), `size: "sm"` uses 18px.
- Read-only: no tab stop, `status` semantics.

### Kbd

- Default: 1px `border` frame, transparent fill, `text_muted`, the `label`
  role (mono 12/16, not uppercased), minimum width equal to its height,
  padding `spacing.xs`.
- `appearance: "inline"`: no frame, used inside menus and buttons.
- `keys: ["⌘", "K"]` renders one keycap per key with `spacing.xxs` between
  them.

### Related: ToggleGroup and Tabs

- **ToggleGroup** expresses pressed tool state. Equal-height segments joined
  without gaps inside one 2px `border` frame; segments overlap the frame by
  2px, so a pressed `selection` fill and a focus frame sit flush with the outer
  edge and the group is exactly `metrics.control` high. Unpressed labels are
  `text_muted`. Keyboard: roving focus, one tab stop on the active segment,
  arrows move focus, Enter or Space toggles.
- **Tabs** selects one associated panel: a continuous `surface_hover` track,
  `metrics.control` high, and one `surface_raised` thumb inset by
  `spacing.xxs`; labels in `tabs.foreground` / `text_primary`, `control` type,
  weight 400 in both states so widths do not jitter. The tab list is one tab
  stop; its focus frames the thumb (`group_focus`). Tabs can render without a
  panel (`panel: false`) when used as a view switcher; the panel itself is
  unframed.

- **TabBar** switches what a panel shows (open documents, a workspace's
  panels), and the panel's extent says what it switches: the bar is
  `tabbar.background` with a 1px `border` line along its bottom; the selected
  tab takes `tabbar.active` (the panel's surface), 1px side hairlines and the
  top corners of `radius.md`, and covers the line, so it and the panel are one
  surface. Unselected tabs are transparent with `tabs.foreground` labels.
  Tabs are `metrics.control` high, start their text on `metrics.inset`, keep
  their width (at most two label columns, then an ellipsis) and scroll
  sideways when they do not fit, a vertical wheel included; the selected tab
  stays revealed. A closable tab reserves a square slot whose close button
  shows while the tab is selected or hovered (`group_hover`); a dirty tab shows
  the 6px mark there until hovered. The strip is one tab stop: the arrows
  move a cursor (the 2px frame) and Enter selects it, because switching may
  remount a whole panel. Use Tabs for a few fixed views inside a region,
  TabBar for the region itself.

## 6. List-like components

Menu, Command, Select and Combobox option lists, ContextMenu, Tree, List,
navigation lists and Table rows share one row grammar:

| State | Treatment |
|---|---|
| idle | transparent |
| hover | `surface_hover` block |
| keyboard cursor in a persistent list (Tree, Accordion, Collapsible) | a 2px `focus_ring` frame drawn over the row (part `cursor` / `focus_frame`), only while the list has focus |
| current item, or the cursor in a transient list (menus, palettes, options) | `selection` block plus the 2px `accent` **indicator bar** on the start edge (part `indicator_bar`, absolutely positioned, so it never pushes text) |
| checked | check mark in a fixed leading column; the column is reserved for every row of a checkable list |
| disabled | `disabled` text, no hover |

- Row height `metrics.row`, text start `metrics.inset`, trailing shortcut in
  the inline Kbd legend aligned to the end.
- Section headers inside lists use the label voice and align with item text
  after the reserved check column.
- Menu and Command items accept `action`; Menu items also accept
  `kind: "label"` for section headers.
- Virtual option lists size their viewport in whole rows
  (`metrics.row * visible`) with `fill_height`, so both densities show the
  same number of rows.

### List

- Keyed rows without columns, for tickets, recent items or plugins: an
  optional leading status Badge, the title, a muted secondary text beside the
  title (`body`, one line box) or below it (`caption`, `secondary_layout:
  "below"`, rows grow to two lines) and trailing `meta` in `text_muted` with
  tabular figures. Rows carry `metrics.inset`, so in a bled DataView or Region
  body their text starts on the title's edge; `dividers: true` adds hairlines.
  Badges of different widths push titles apart: `badge_width` (pixels)
  reserves one leading slot on every row so the titles share an edge.
- Selection follows Table: `table.selection` and the indicator bar, the focus
  frame on the selected row while the list has focus, one tab stop whose
  arrows move the selection, Enter for `row_click`, `on_context_request` for
  a caller menu. Roles are `listbox` / `option` with a selection mode and
  `list` / `listitem` without.

### Table

- No outer frame: the region around the table provides its edges.
- Header row in the label voice on `surface`, hairline bottom border; sticky
  copy indistinguishable from the natural row. Only sortable headers react to
  hover.
- Body rows `metrics.row`, cell text at `metrics.inset`, hairline separators in
  `border`, no zebra striping by default (`striped` remains opt-in).
- Selected rows use `table.selection` and the indicator bar on the row start.
- `numeric: true` columns align to the end with tabular figures unless an
  `align` is given.
- Adornments (Badges) follow the cell text; a column whose row has no value of
  its own shows only its adornments, which is how a status column is written.
- With a selection mode the table is a tab stop: Up/Down/Home/End move the
  selection and keep it revealed, Enter emits `row_click`, and while the table
  has focus the selected row carries the focus frame (`cursor` part,
  `group_focus`).

## 7. Fields

Things you type into have a frame; things you press are blocks.

- Input, Textarea, Select trigger, Combobox trigger and DatePicker share:
  height `metrics.control` by size, `surface_raised` well, 2px `border` frame,
  padding `metrics.field_pad`, `control` text. Text therefore starts at the
  same x in every field. Textarea keeps `body` for multi-line reading.
- The side padding follows the space above the text. With
  v = (`metrics.control` − 2 × 2px frame − type size) / 2, the gap between the
  frame and the text's em box, `field_pad` is about 1.4·v: text sits a little
  further from the side than from the top, as set type does, and never a
  typed space away from the frame. The values are then moved to nearby
  structure: at md the text starts on `metrics.inset` (12 / 8), where row text
  starts; compact xs gets 3px so the caret clears the frame; comfortable lg
  takes 12 from the spacing scale.

  | Size | comfortable v → `field_pad` | compact v → `field_pad` |
  |---|---|---|
  | xs | 3.5 → 5 | 1.5 → 3 (caret room, from 2.1) |
  | sm | 5 → 7 | 3 → 4 |
  | md | 7 → 10 (text at 12, `inset`) | 5 → 6 (text at 8, `inset`, from 7) |
  | lg | 9 → 12 (spacing scale, from 12.6) | 7 → 10 |

- Textarea keeps the same text column. Its vertical padding
  `metrics.multiline_pad` is `field_pad / 1.4` less the half-leading of its
  text (4px for `body`, 3.5 for `body_small` at xs), at least 0, so the first
  line's em box sits as far below the frame as a single-line field's text.
- InputGroup pads its prefix and suffix by `field_pad` on the frame side; the
  inner control keeps its own `field_pad` toward the affix.
- Pressed controls keep `metrics.control_pad`: a label you did not type cannot
  be mistaken for a leading space, and the wider block is part of what reads
  as a button.
- InputGroup owns the frame for a prefix/suffix and its inner control; the
  frame takes `focus_ring` while the inner input has focus (`focus_within`).
- Focus: frame turns `focus_ring`. Invalid: frame turns `danger`; while focused
  the focus color owns the frame and danger returns on blur. Read-only: no
  frame change on hover, muted caret area. Disabled: `disabled` text, frame
  keeps its color at reduced opacity.
- Checkbox: `metrics.mark` box with a 2px `border` frame on the well; checked
  fills `accent` with an `on_accent` mark. Radio: the same frame as a circle
  with a 6px accent dot. Switch: rectangular 32×16 track (`metrics.mark` × 2),
  `surface_hover` off and `accent` on, a 12px square thumb, ink off and
  `on_accent` on. In all three the row is the target and the mark shows focus,
  so the mark stays on the content edge. Rows are `metrics.control` high.
- Slider and RangeSlider: a 4px track, `accent` fill and a fader cap (a 12×20
  cobalt block with a 2px `surface` frame that turns ink while focused),
  `metrics.control` high. A horizontal slider is a 240-wide unit whose label
  and value span exactly the track (the `root` part takes another width).
- Labels are `body` at 400 in `text_primary`; descriptions and errors are
  `caption`, errors in `text.danger`.

## 8. Overlays and containers

- Overlay panels (Menu, Combobox/Select/DatePicker panels, Popover, Dialog,
  Sheet, Toast, Command) are `surface_raised` blocks with a 1px `border`
  hairline and `radius.lg`; no shadow. Menus and field panels align to their
  trigger's start edge (`align: "start"`, RTL-aware); Tooltips center.
- Dialog and Sheet pad with `spacing.xl`; actions are separated by space, not
  a rule. Sheets draw their hairline only on the edge facing the window. A
  Dialog is 420 wide, at most 90% of the window; `title_visible: false` hides
  its title, which still names it (a command palette's search field is its
  header).
- Tooltip inverts ink and paper (`text_primary` block, `surface` text, caption)
  and can show an action's key legend (`action` or `shortcut`).
- Card is a `surface_raised` block (`variant: "outline"` for cards placed on a
  raised surface); parts are separated by `space.group`.
- GroupBox is a hairline rule with a label-voice heading, on the parent's
  content edge.
- Alert and Toast mark status with the 6px square lamp; the block itself stays
  neutral ("normal is quiet").
- Accordion and Collapsible are Swiss lists: hairlines between sections, no
  enclosing box, headers as list rows.
- Empty states are quiet: centered type on the region's surface, no frame.
- ScrollArea, CodeViewer and DiffViewer draw no frame; the layout does.
- TitleBar shows title and subtitle on one line at one size (weight and color
  tell them apart); start and end share the free width, the start truncates
  first and the end never shrinks below its controls. StatusBar uses the same
  three regions.

## 9. Motion in components

Allowed: Tabs thumb movement (`fast`, `standard`), overlay enter/exit (`fast`),
Toast enter/exit, Progress indeterminate stripe, Skeleton shimmer, Spinner.
Everything follows the Host motion policy; `None` places final states directly.
Decorative motion lives in `motion/*` and is never used by `components/`.

## 10. Focus mechanics

Three style hooks keep the single focus metaphor (a 2px ink frame) without
layout shift:

| Hook | Applies when | Used by |
|---|---|---|
| `.focus(style)` | the node itself has focus | Button, fields, Tag close, segments, day cells |
| `.group_focus(style)` | the nearest focusable ancestor-or-self with a `.focus` style has focus; a `tab_stop(false)` child passes ownership up | Checkbox box, Radio ring, Switch track, Tabs thumb, list cursor frames, the focused RadioGroup option |
| `.focus_within(style)` | the node or a descendant has focus | InputGroup frame |
| `.group_hover(style)` | the nearest ancestor declaring a `.hover` style is hovered (paint only: background, border, text, opacity) | TabBar close button and dirty mark |

A focus owner that needs no visual of its own declares `.focus(style())`.

**Containers that hold focus.** A container that can hold focus itself (an
overlay panel of Dialog, Sheet, Popover, Menu, DatePicker or Combobox; an
AppShell region) shows the same 2px `focus_ring` frame over its edge (part
`focus_frame`; a panel's hairline takes the focus color and the frame adds the
inner pixel, since a border paints over its children), only while it holds
focus itself. Once a
control inside has focus, that control shows its frame and the container shows
none. Where keys move a cursor (Table and Tree rows, a Menu's active item, a
DatePicker's focused day), the cursor shows where they act; a Menu opened from
the keyboard shows its active item, not a frame. An overlay panel's own focus
is visible to `group_focus` styles in its content, so the frame is an ordinary
node.

**One tab stop per control.** Tab reaches every interactive component exactly
once (sets of independent targets such as Accordion headers, Pagination or a
closable Tag's close button excepted). The runtime keeps wrappers out of the
order: an overlay's trigger wrapper takes focus only when its trigger has no
focusable content of its own, a container with key handlers (a roving group, an
overlay root) is a stop only when it holds no focusable child, and tooltip panels
are never stops. `tests/native-keyboard/tests/tab_stops.rs` counts the stops of
each interactive component.

## 11. New component checklist

- [ ] Declares `size` if interactive; reads every height from `metrics.*`.
- [ ] Declares the tokens and environment values it reads (C7), and the assets
      it draws with `asset(...)`: only declared assets are preloaded.
- [ ] Rows use `metrics.row` and `metrics.inset`; the indicator bar is a part.
- [ ] No literal color, font size, line height, control height or spacing;
      structural constants commented.
- [ ] Sizes to content; no outer margin.
- [ ] Reserves a 2px focus border if focusable; no layout shift between states.
- [ ] Works inside a disabled container without forwarding props.
- [ ] Commands accept `action`.
- [ ] Every signature is a declared part.
- [ ] Spec page in Gallery: sizes × densities × states × light/dark.
- [ ] Keyboard, accessibility and logic tests; composition audit clean.
