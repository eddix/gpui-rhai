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
| C8 | **Removable signatures** | Each structural signature is a declared part (`indicator`, `lamp`, `key`, `shortcut`, `label`), so `ui/styles.rhai` can restyle or hide it. |
| C9 | **States without layout shift** | State priority is `active > focus > hover/cursor > selected > idle`; `disabled` suppresses interaction and wins over pointer states. Borders are reserved in idle geometry. Selection stays visible under focus. |
| C10 | **Visible focus** | Focusable controls reserve a 2px border. Idle, it takes the control's fill (invisible) or, for outline treatments, the low-contrast `border` role. Focused, it takes `focus_ring`, which defaults to the ink color. |
| C11 | **Inherited disabled** | A disabled container disables every descendant natively: input, focus, pseudo styles and accessibility semantics. Components do not need to forward `disabled` to children. |
| C12 | **Action binding** | Components that trigger commands (Button, IconButton, Menu items, Command items, ContextMenu items, Tooltip hints) accept `action: "id"`. The displayed shortcut, enabled state and dispatch derive from that action. A free-text `shortcut` remains for commands without an action. |
| C13 | **Data defaults** | When a component knows a value is numeric (a Table column with `numeric: true`, Stat, DescriptionList numeric values), it aligns to the end and enables tabular figures by default. |
| C14 | **Text safety** | Text never clips CJK fallback glyphs. Truncated text keeps its full accessible name. Components do not invent copy or functional emoji. |

## 2. Geometry

### Metrics

| Token | comfortable | compact | Use |
|---|---:|---:|---|
| `metrics.control` by size `xs` / `sm` / `md` / `lg` | 24 / 28 / 32 / 36 | 20 / 24 / 28 / 32 | Button, inputs, Select, Toggle, Tabs slots |
| `metrics.control_pad` by size | 8 / 12 / 16 / 20 | 8 / 8 / 12 / 16 | horizontal padding of text controls |
| `metrics.row` | 32 | 28 | Menu, Command, option lists, Table, Tree, navigation rows |
| `metrics.inset` | 12 | 8 | text start in rows, panel padding |
| `metrics.marker` | 20 | 20 | Badge, Tag, Kbd height |
| `metrics.icon` by size | 12 / 14 / 16 / 16 | 12 / 14 / 14 / 16 | control icon box |
| `metrics.titlebar` | 36 | 32 | TitleBar |
| `metrics.statusbar` | 24 | 22 | StatusBar |

Structural constants that stay literal in component source, with a comment:
the 1px hairline, the 2px focus border, the 2px indicator bar, the 6px lamp,
icon view boxes, zero offsets, semantic circles.

### Radius roles

| Role | Used by | Default |
|---|---|---:|
| `radius.sm` | Badge, Tag, Kbd, Checkbox box, indicator-free chips | 0 |
| `radius.md` | Button, IconButton, inputs, Select, Tabs track/thumb, ToggleGroup | 0 |
| `radius.lg` | Popover, Dialog, Menu panel, Tooltip, Toast, Sheet | 0 |

Semantic circles (Radio, Avatar, slider thumb, presence) use half their size.
Square status lamps are not circles.

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
| `code` | 13 / 20 | 400 | mono | code, commands, key legends |
| `control` by size | `body_small` for `xs`, `body` otherwise | — | UI | text inside controls |

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
(tonal block hover step), `control.fill_hover` (solid fill hover step),
`table.selection`, `tabs.foreground`. See [themes.md](themes.md).

## 5. Marker system: Button, Tag, Badge, Kbd

The four rectangle-and-text components are distinguished by silhouette, not by
color alone; the distinction survives grayscale.

| Component | Silhouette | Height | Text | Fill and line |
|---|---|---|---|---|
| **Button** | large block, centered label | `metrics.control` | `control`, 600 | solid or tonal block; outline uses a frame; ghost has no fill |
| **Tag** | small flat tape, optional key segment | `metrics.marker` | `caption`, 400 | tonal block, no frame |
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
| `danger` / `warning` / `success` | status fill | `control.fill_hover` mix | matching `on_*` |

- Padding `metrics.control_pad`; gap between icon and label `space.xs`.
- Focus: the reserved 2px border turns `focus_ring`.
- Active: opacity 0.86. Disabled: `surface_hover` fill, `disabled` text, keeps
  its size.
- Loading keeps the width of the idle label where possible and announces the
  loading text.
- Optional `shortcut` part shows the action's key legend after the label in
  the label's color at 70% opacity; hidden for `xs`.

### Tag

- Tonal `surface_hover` block, horizontal padding `space.sm` in both
  densities (markers do not change with density), `radius.sm`.
- Color variants (`accent`, `success`, `warning`, `danger`) mark a
  **category**, not a status: without a key they color the text with the
  derived `text.*` role; with a key they fill the key segment with the color
  and use the matching `on_*` text.
- `key` prop: a leading segment in the label voice (mono, uppercase) on a
  darker tonal step (`tag.key`), separated from the value by no gap.
- `closable`: a 12px close icon at the end, its own hover block, keyboard
  reachable, announces "Remove <text>".

### Badge

- Default (`emphasis: "subtle"`): a 6px square lamp in the variant color
  (neutral uses `text_muted`) followed by the label in `text_primary`. No
  container, no frame.
- `emphasis: "strong"`: a solid block in the status color with `on_*` text,
  weight 600, padding `space.sm`. Use only for states that require action.
- `dot: false` removes the lamp (text-only status), `size: "sm"` uses 18px.
- Read-only: no tab stop, `status` semantics.

### Kbd

- Default: 1px `border` frame, transparent fill, `text_muted`, `code` role at
  12/16, minimum width equal to its height, padding `space.xs`.
- `appearance: "inline"`: no frame, used inside menus and buttons.
- Multi-key chords are separate keycaps with `space.xxs` between them when
  given as an array.

### Related: ToggleGroup and Tabs

- **ToggleGroup** expresses pressed tool state. Equal-height segments joined
  without gaps; each pressed segment fills its own area with `selection` and
  keeps `text_primary`; unpressed segments are transparent on a 2px `border`
  frame.
- **Tabs** selects one associated panel: a continuous `surface_hover` track and
  one `surface_raised` thumb inset by `space.xxs`; labels in `tabs.foreground`
  / `text_primary`, weight 400 in both states so widths do not jitter. Tabs can
  render without a panel (`panel: false`) when used as a view switcher.

## 6. List-like components

Menu, Command, Select and Combobox option lists, ContextMenu, Tree, navigation
lists and Table rows share one row grammar:

| State | Treatment |
|---|---|
| idle | transparent |
| hover / keyboard cursor in a persistent list | `surface_hover` block |
| current item, or the cursor in a transient list (menus, palettes, options) | `selection` block plus the 2px `accent` **indicator bar** on the start edge, inside the inset |
| checked | check mark in a fixed leading column; the column is reserved for every row of a checkable list |
| disabled | `disabled` text, no hover |

- Row height `metrics.row`, text start `metrics.inset`, trailing shortcut in
  the inline Kbd legend aligned to the end.
- Section headers inside lists use the label voice.

### Table

- Header row in the label voice on `surface`, hairline bottom border; sticky
  copy indistinguishable from the natural row.
- Body rows `metrics.row`, hairline separators in `border`, no zebra striping.
- Selected rows use `table.selection` and the indicator bar on the first cell.
- `numeric: true` columns align to the end with tabular figures.

## 7. Fields

Things you type into have a frame; things you press are blocks.

- Input, Textarea, Select trigger, Combobox trigger and DatePicker share:
  height `metrics.control` by size, `surface_raised` well, 2px `border` frame,
  padding `metrics.control_pad`, `control` text.
- Focus: frame turns `focus_ring`. Invalid: frame turns `danger`; while focused
  the focus color owns the frame and danger returns on blur. Read-only: no
  frame change on hover, muted caret area. Disabled: `disabled` text, frame
  keeps its color at reduced opacity.
- Checkbox: 16px box with a 2px `border` frame; checked fills `accent` with an
  `on_accent` mark. Radio: circular. Switch: rectangular 28×16 track,
  `surface_hover` off and `accent` on, square thumb.

## 8. Motion in components

Allowed: Tabs thumb movement (`fast`, `standard`), overlay enter/exit (`fast`),
Toast enter/exit, Progress indeterminate stripe, Skeleton shimmer, Spinner.
Everything follows the Host motion policy; `None` places final states directly.
Decorative motion lives in `motion/*` and is never used by `components/`.

## 9. New component checklist

- [ ] Declares `size` if interactive; reads every height from `metrics.*`.
- [ ] Declares the tokens and environment values it reads (C7).
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
