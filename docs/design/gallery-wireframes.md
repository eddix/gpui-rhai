# Gallery as the acceptance application

The Gallery is rebuilt as a real application assembled from `patterns/` and
`layouts/`. Its pages are both the component catalog and the acceptance test of
[composition.md](composition.md). This document fixes its structure and derives
the L2 APIs from it.

## Gates

1. Composition audit (productivity rule set) reports zero findings on every
   page, in both densities.
2. Every scene can be completed by keyboard alone (native tests drive it).
3. Visual baselines for representative pages in comfortable/compact ×
   light/dark, plus one RTL and one CJK capture.

## Shell

```
┌───────────────────────────────────────────────────────────────────────────┐
│ ● ● ●   GPUI RHAI / Button                    [Comfortable|Compact] [◐] ⌘K │ TitleBar
├───────────────┬───────────────────────────────────────────┬───────────────┤
│ FOUNDATIONS   │ Button                         [Source ⌘⇧S]│ SOURCE        │
│  Tokens       │ Seven variants, four sizes.               │ 1 import …    │
│  Typography   │                                           │ 2 …           │
│  Color        │ VARIANTS ──────────────────────────────── │               │
│ COMPONENTS    │ ■Primary ░Secondary  Outline  Ghost …     │               │
│ ▌Button       │                                           │               │
│  Badge        │ SIZES ─────────────────────────────────── │               │
│  …            │ xs sm md lg                               │               │
│ LAYOUTS       │                                           │               │
│ PATTERNS      │ DENSITY ───────────────────────────────── │               │
│ SCENES        │ comfortable │ compact                     │               │
│  Operations   │                                           │               │
│  Data browser │ STATES ────────────────────────────────── │               │
│  Form         │ disabled loading focus                    │               │
│  Settings     │                                           │               │
│ EFFECTS       │                                           │               │
├───────────────┴───────────────────────────────────────────┴───────────────┤
│ DENSITY comfortable   THEME Default Dark   LOCALE en      AUDIT 0   ⌘K    │ StatusBar
└───────────────────────────────────────────────────────────────────────────┘
```

- Regions: `nav` (sidebar), `main`, `source` (inspector, toggled), `status`.
  F6 cycles nav → main → source; `Cmd+1` nav, `Cmd+2` main.
- Title bar end: density ToggleGroup, theme mode toggle (IconButton), locale
  Select (`en`, `zh-CN`, `ar`), `⌘K` legend that opens the palette.
- The status bar shows the live audit finding count; it must read 0.
- The source inspector is a CodeViewer of the current page's story module.

## Spec page template

Every component and pattern has one spec page:

```
Region
  title: <Component>      actions: [Source toggle]
  description (caption): one line
  Section VARIANTS   — every variant at md
  Section SIZES      — xs/sm/md/lg in one Inline per variant group
  Section DENSITY    — the same specimen in two env regions side by side
  Section STATES     — disabled, loading, selected, invalid, read-only as applicable
  Section IN CONTEXT — a small realistic composition
```

The DENSITY section wraps two copies of a specimen in containers with
`.env(#{ density: "comfortable" })` and `.env(#{ density: "compact" })`; it is
the visual test of environment resolution.

## Scenes

### Operations workbench

```
Region "Hosts"
 Toolbar  [SITE│i18n][ENV│prod]  [Filter hosts…      ]   [Export][Refresh][■Deploy■]
 Stats    3 hosts │ 2 healthy │ 1 needs attention
 ListDetail (horizontal)
   DataView: Table HOST · CPU% (numeric) · MEM (numeric) · STATUS (lamp)
             ▪ online  ▪ online  ■ degraded(warning, strong when action needed)
   Detail:   Section "edge-03"  [Restart]
             DescriptionList  Region  sg1 / Version  1.4.2 / Uptime  12d
             InlineState(error) "Health check failed" + selectable raw output
 Footer   ▪ 1 warning                         3 hosts · fetched 09:20
```

Keyboard: F6 into main → Tab to filter → type → Down into table → arrows →
Enter opens detail → Tab to Restart → Enter → Esc back to the table.

### Data browser

DataView with a sortable, resizable Table of 500 rows, numeric columns,
multi-selection, Pagination in the footer, and a compact-density toggle that
switches only this region. Keyboard: sort by column header with Enter,
select with Space, page with the Pagination controls.

### Form

FormLayout with two groups (Identity, Access), Input, Select, Combobox,
DatePicker, Checkbox and RadioGroup, inline validation, and a submit row with
one primary Button. Keyboard: Tab through fields, Space/arrows on choices,
Enter submits, invalid field focuses first.

### Settings

Tabs as a view switcher (`panel: false`) above Sections of Switch, Select and
Slider rows; a disabled Section demonstrates inherited `disabled`.

### Command palette

`Cmd+K` CommandDialog listing every registered action with its shortcut from
`ctx.action_shortcut`. Keyboard: type to filter, arrows, Enter runs, Esc closes
and restores focus.

## Derived L2 APIs

| Component | Props and slots | Behavior |
|---|---|---|
| `layouts/stack` | `items`, `gap` (alias or scale step), `align`, `fill` | vertical; never stretches children unless `align: "stretch"` |
| `layouts/inline` | `items`, `gap`, `align` (`center` default), `wrap`, `size` | horizontal; `size` sets the size environment for items |
| `layouts/toolbar` | `context`, `filters`, `actions`, `primary`, `size` | start group (context, filters), end group (actions, primary last); `related` inside groups, `group` between; filters grow |
| `layouts/region` | `title`, `description`, `actions`, `toolbar`, `body`, `footer`, `fill` | header row, toolbar, body fills remaining height and scrolls, footer pinned; content inset padding; `section` gaps |
| `patterns/section` | `title`, `description`, `actions`, `body` | subtitle title row with end-side actions; `group` gap to body |
| `patterns/description_list` | `items` (`label`, `value`, `numeric`), `label_width`, `columns` | label column in muted body, values in body; numeric values end-aligned with tabular figures |
| `patterns/stat` | `value`, `unit`, `label`, `variant` | display figure, muted unit on the same baseline, label voice caption |
| `patterns/stats` | `items` | stats separated by hairlines, equal gaps |
| `patterns/form_layout` | `groups` (`title`, `fields`), `label_position`, `label_width`, `submit` | aligned label column, `related` between fields, `group` between groups |
| `patterns/inline_state` | `state` (`loading`, `empty`, `error`, `stale`, `refreshing`), `title`, `detail`, `action` | in-place feedback; error detail selectable |
| `patterns/data_view` | `toolbar`, `content`, `state`, `footer_start`, `footer_end` | Region-like body that fills; shows InlineState instead of or above content |
| `patterns/list_detail` | `list`, `detail`, `orientation`, `list_size`, `empty` | fixed list, filling detail; arrow keys stay in the list |
| `patterns/app_shell` | `title`, `title_actions`, `sidebar`, `main`, `inspector`, `inspector_open`, `status`, `regions` | title bar with traffic-light reserve, three columns, status bar, F6 region cycling, command palette host |
