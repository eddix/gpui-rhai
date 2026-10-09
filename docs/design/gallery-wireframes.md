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

### As built (0.2.0)

The implementation follows these wireframes with these differences, decided
during step 5 (see decisions D20 to D33):

- Baselines add a second CJK case (`table` and `description_list` in
  Simplified Chinese) and are rendered offscreen.
- The title bar shows title and page on one line (`GPUI RHAI  Button`); the
  mode toggle is a text Button, and there are no `Cmd+1`/`Cmd+2` region keys
  (F6 / Shift+F6 and Cmd+K only).
- The data browser has 120 rows, a sortable CPU column, multiple selection,
  Pagination in the footer and a density toggle for the region.
- The form scene uses Input, RadioGroup, Select, DatePicker and Checkbox (no
  Combobox); the settings scene uses Switch and Slider rows.

## Shell

```
┌───────────────────────────────────────────────────────────────────────────┐
│ ●●● GPUI RHAI [Comfortable|Compact] [Square|Subtle|Round] [Theme▾][en▾]⌘K │ TitleBar
├───────────────┬───────────────────────────────────────────┬───────────────┤
│ FOUNDATIONS   │ Button                            [Source]┃ SOURCE        │
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
│ DENSITY comfortable THEME Default Dark CORNERS square LOCALE en  AUDIT 0  │ StatusBar
└───────────────────────────────────────────────────────────────────────────┘
```

- Regions: `nav` (sidebar), `main`, `status`. F6 moves between nav and main.
- Title bar end: density and corner-style ToggleGroups, a theme Select, a
  locale Select (`en`, `zh-CN`, `ar`), and a Commands button with the `⌘K`
  legend that opens the palette.
- The status bar shows the environment and the live audit finding count; the
  count must read 0.
- `Source` shows a CodeViewer of the current page module beside the page, in a
  SplitPane inside `main` (`┃` above): drag the divider to widen it. AppShell
  keeps fixed side widths (D54), so the Gallery does not use its inspector.

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

## L2 APIs as implemented

The wireframes above derived these modules; this table records the props they
shipped with in 0.2.0 (the header comment of each module is authoritative).
Differences from the first draft: children are `children` (not `items`), the
Region body is `body` and it does not scroll by itself, DataView takes a
Toolbar slot map and an InlineState map, AppShell takes TitleBar and StatusBar
prop maps and hosts overlays, and Stat rows are the `stats(items)` helper.

| Module | Props and slots | Behavior |
|---|---|---|
| `layouts/stack` | `children`, `gap` (`unit`/`related`/`group`/`section`, a scale step, or `none`; default `related`), `align` (`stretch`/`start`/`center`/`end`), `fill`, `label` | a column with one relationship |
| `layouts/inline` | `children`, `gap` (default `related`), `align` (`start`/`center`/`end`, default `center`), `justify` (`start`/`between`/`end`), `wrap`, `size`, `label` | a row; `size` sets the size environment of every control inside |
| `layouts/toolbar` | `label`, `context`, `filters`, `actions`, `primary`, `size` | start group (context, filters) and end group (actions, primary last); `related` inside a group, `group` between; wraps when narrow |
| `layouts/region` | `label`, `body`, `title` (string or node), `actions`, `toolbar`, `footer`, `fill` (true), `inset` (true), `bleed` | header, toolbar, filling body and pinned footer, `group` apart; `bleed` lets rows reach the sides |
| `patterns/section` | `title`, `content`, `description`, `actions`, `voice` (`title`/`label`) | a subtitle or label-voice heading with end-side actions |
| `patterns/description_list` | `label`, `items` (`label`, `value`, `numeric`, `identifier`), `label_width`, `layout` (`columns`/`stacked`) | one label column at `metrics.label_column`; stacked pairs a unit apart inside, a group apart between |
| `patterns/stat` | `Stat`: `label`, `value`, `unit`, `delta`, `tone`, `description`; `stats(items)` | a figure with a muted unit on its baseline; only a deviating delta takes color |
| `patterns/form_layout` | `key`, `label`, `groups` (`title`, `fields`) or `fields`, `submit`, `orientation`, `label_width`; a field has `label`, `control`, `description`, `error`, `required` | aligned label column, `related` between fields, `group` between groups, submit on the field edge |
| `patterns/inline_state` | `key`, `state` (`loading`/`empty`/`error`/`stale`/`refreshing`), `title`, `description`, `detail`, `actions` | feedback where the content would be; error detail selectable |
| `patterns/data_view` | `key`, `label`, `body`, `title`, `toolbar` (Toolbar slot map), `state` (InlineState map), `footer` (`status`, `selection`, `count`, `updated`) | Region + Toolbar + body + footer; the state replaces or tops the body |
| `patterns/list_detail` | `label`, `list`, `detail`, `orientation`, `list_size` | a fixed list beside or above a filling detail, one hairline between |
| `patterns/app_shell` | `key`, `label`, `main`, `title_bar` (TitleBar props), `sidebar`, `inspector`, `status` (StatusBar props), `overlays`, `sidebar_width`, `inspector_width` | title bar, sidebar, main, inspector, status bar; F6 / Shift+F6 cycle the regions |
