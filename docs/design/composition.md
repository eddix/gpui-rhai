# Composition guide

For application developers. It explains how to combine components into
screens. Most rules are already built into `layouts/` and `patterns/`; this
guide explains why they behave that way and covers what no default can know.
Read [principles.md](principles.md) first.

## 1. Start from layouts and patterns

Write screens from the largest structure inward:

1. `patterns/app_shell` — title bar, sidebar, main area, optional inspector,
   status bar, command palette and keyboard regions.
2. `layouts/region` — one working area: header, toolbar, body that fills the
   remaining height, footer pinned to the bottom.
3. `layouts/toolbar`, `patterns/data_view`, `patterns/list_detail`,
   `patterns/form_layout`, `patterns/section` — the content of a region.
4. `layouts/stack` and `layouts/inline` — everything else.
5. Raw `row`/`column` only for what none of the above express.

Patterns use **semantic slots**. A slot decides order, spacing relationship and
the environment passed to its children:

```rhai
toolbar::Toolbar(#{
    label: "Hosts",
    context: [tag::Tag(#{ facet: "site", text: "i18n" })],
    filters: [input::Input(#{ key: "q", label: "Filter", placeholder: "Filter", value: q,
        on_change: Fn("q") })],
    actions: [button::Button(#{ text: "Export", variant: "ghost", action: "data.export" })],
    primary: button::Button(#{ text: "Deploy", variant: "primary", action: "deploy.start" }),
})
```

Slots accept any node. Putting something unusual in a slot is allowed; if it
breaks a rule, the audit reports it.

| Module | Export | Use |
|---|---|---|
| `layouts/stack` | `Stack` | a column with one relationship (`gap: "related"` by default) |
| `layouts/inline` | `Inline` | a row with one relationship and one control `size` |
| `layouts/toolbar` | `Toolbar` | context, filters, actions, one primary; wraps when narrow |
| `layouts/region` | `Region` | title and actions, toolbar, filling body, footer; `bleed` for rows |
| `patterns/section` | `Section` | a subtitle (or label voice), description, end actions, content |
| `patterns/description_list` | `DescriptionList` | label and value pairs on one label column |
| `patterns/stat` | `Stat`, `stats(items)` | a figure with unit and delta |
| `patterns/form_layout` | `FormLayout` | aligned labels, field groups, submit row on the field edge |
| `patterns/inline_state` | `InlineState` | loading, empty, error, stale, refreshing in place |
| `patterns/data_view` | `DataView` | Region + Toolbar + body + footer, with states |
| `patterns/list_detail` | `ListDetail` | a fixed list beside or above a filling detail |
| `patterns/app_shell` | `AppShell` | title bar, sidebar, main, inspector, status bar, F6 regions |

Rhai limits expression depth (64 per script, 32 inside a function). Build deep
screens from small functions, one per region or section, as in the examples;
it reads better too.

## 2. Spacing

Use the spacing scale everywhere (`theme_spacing(name)` or
`theme_length("spacing.<name>")`); it is density aware.

| Token | comfortable | compact |
|---|---:|---:|
| `spacing.xxs` | 2 | 2 |
| `spacing.xs` | 4 | 4 |
| `spacing.sm` | 8 | 8 |
| `spacing.md` | 12 | 8 |
| `spacing.lg` | 16 | 12 |
| `spacing.xl` | 24 | 16 |

Relationship aliases (`theme_length("space.<alias>")`, or the `gap` names of
Stack and Inline) name the intent and map onto the scale:

| Alias | Scale | Relationship | Examples |
|---|---|---|---|
| `unit` | `xs` | parts of one unit | icon and label, label and helper text |
| `related` | `sm` | siblings in one group | buttons in a toolbar group, fields in a form |
| `group` | `lg` | groups in one area | toolbar start and end groups, form groups, region parts |
| `section` | `xl` | areas of one page | sections of a region, dialog padding |

**Rule: inside a nesting, the inner gap is strictly smaller than the outer
gap.** The audit checks this on resolved geometry, however the gap was written,
with three refinements that match how proximity is read:

- Only nesting along the same axis competes (a row's gaps against the
  enclosing row, a column's against the enclosing column).
- A row distributed with `justify_between` treats its gap as a floor, not a
  relationship.
- A heading leads its content: the gap under a heading is typographic, so the
  content inside answers to the next gap out. This is why a Region (`group`
  between its parts) can hold sections separated by `section`.

Patterns choose these relationships for you; when writing a Stack yourself,
prefer the aliases over raw steps so the intent is readable.

## 3. Alignment

- **One text edge.** Panel titles, toolbars, table headers and the first column
  of rows start at the same x: the container edge plus `metrics.inset`.
  Components already follow this; do not add padding around them.
- **Same height in a row.** Controls in one row share one size. Put them in a
  Toolbar or Inline, which passes `size` down, instead of setting `size` on each.
- **Numbers.** Right-align numeric columns and figures with tabular digits.
  Table does this for `numeric: true` columns; Stat and DescriptionList do it
  for numeric values.
- **Rows bleed.** Tables and lists carry their own row inset; a Region with
  `bleed: true` (DataView does this) lets them reach its sides so row text
  starts on the title's edge.
- **Units.** Put units after the number in the muted color, sharing the
  baseline: `881 GiB`.
- **Fill, do not fix.** Main content fills the remaining height of its region;
  give fixed heights only to lists stacked above a detail view.

## 4. Hierarchy

Distinguish importance with color and weight before size.

- **One size per row.** A row uses one typography role (markers excepted).
  Mixed sizes in a row misalign baselines and make similar information look
  different.
- **Data or description?** If deleting the text removes information, it is
  data: `body` in `text_primary`, or `text_muted` for secondary data (people,
  times, IDs, counts). If it only explains, it is description: `caption`.
- **Weight 600 only for titles** (section and panel titles, filled button
  labels). Names in lists, versions and types are not bold.
- **One title per region.** A region shows one title; sections inside it use
  `subtitle`. The label voice replaces a title, never sits above one.
- **Identifiers** (hosts, IDs, versions, keys) use the UI font with tabular
  and slashed-zero features; real code uses the mono family.

## 5. Color use

| Color | Use | Do not use for |
|---|---|---|
| neutral (`text_primary`, `text_muted`, `surface_*`) | almost everything, including healthy states | — |
| `accent` | the primary action, current location, selection, focus-adjacent marks, links | decoration, headings, "info" status |
| `warning` | a deviation that needs attention but is not a failure | normal states, emphasis |
| `danger` | failures, destructive actions, invalid input | "not found", empty results |
| `success` | confirming an action just completed, recovery from failure | the steady healthy state |

- **Normal is quiet.** A steady healthy item shows a neutral lamp. Color only
  the deviation, so the one problem stands out among many rows.
- **Not by color alone.** Every status has text and a lamp shape; a strong
  Badge also changes silhouette.
- **Budget.** Most of a screen is neutral; accent appears in one primary
  action and the current item; status colors appear only where status deviates.
- **Category colors** (Tag variants) mark kinds of things, not states. Do not
  use a red Tag to say something failed; use a Badge.

## 6. Actions

- **One solid button per action group.** The primary action is the only filled
  button in its group; other actions are `secondary`, `ghost` or `outline`.
  Toolbar's single `primary` slot encodes this.
- **Status-colored buttons** are for verdict actions whose result is the
  status: Approve (`success`), Force or Override (`warning`), Delete
  (`danger`). They count as the group's solid button.
- **Order.** In toolbars, actions on the end side, primary last. In dialogs,
  the confirming action last, Cancel before it.
- **Buttons never stretch** to the container width except in narrow sheets.
- **Destructive actions** ask for confirmation in a Dialog whose confirm button
  is `danger` and is not the default for Enter.
- **Every command is an action.** Register it with `ctx.register_action`, give
  components `action: "id"`, and the shortcut, enabled state and command
  palette entry come for free.

## 7. Feedback in place

Loading, empty and error states appear where the content would appear.

| State | Pattern | Copy |
|---|---|---|
| first load | `InlineState(#{ state: "loading" })` or Skeleton for fixed shapes | say what is loading |
| empty | `InlineState(#{ state: "empty" })` | why it is empty and what to do next |
| error | `InlineState(#{ state: "error", detail })` | one sentence of what happened; the raw error stays selectable |

InlineState requires a `title`: only the caller knows what is loading or why a
list is empty. DataView takes the same map as `state:` and places it.
| refresh failed with cached data | keep the data, show a stale note above it | "Showing cached data" plus the reason |
| background refresh | keep the data, show a quiet refreshing marker | — |

"Nothing found" is empty, not an error. Never replace a whole region with an
error when other sources still have data.

## 8. Keyboard

AppShell defines regions (sidebar, main, inspector, status bar).

- **F6 / Shift+F6** move between regions; each region can also bind a direct
  action (for example `Cmd+1` for the sidebar). A region shows the 2px ink
  frame while it holds focus itself; Tab then enters its first control. Key
  handlers accept modifier-qualified names (`on_key_value("shift+f6", ...)`);
  a plain name fires whatever modifiers are held.
- Inside a region, arrow keys move within lists and tables; **Tab** moves
  between controls; **Enter** activates; **Space** toggles.
- **Esc** steps back one level: close the overlay, then clear in-region state
  (selection, filter focus), then return to the main region.
- **Cmd+K** opens the command palette everywhere. It lists every enabled
  action with its shortcut.
- Show shortcuts where the action is offered: menus, palette, tooltips, and
  buttons where space allows.

## 9. Page patterns

```
AppShell
┌──────────────────────────────────────────────────────────┐
│ ● ● ●  Title                                    ⌘K       │ TitleBar
├────────────┬─────────────────────────────────┬───────────┤
│ SECTION    │ Region title                    │ INSPECTOR │
│▌Current    │ [context][filters]  [ghost][■■] │           │ Toolbar
│ Item       │ body (fills)                    │           │
│ Item       │                                 │           │
│            │ ■ status            12 items    │           │ Footer
├────────────┴─────────────────────────────────┴───────────┤
│ field  value                                 field value │ StatusBar
└──────────────────────────────────────────────────────────┘
```

- **DataView**: toolbar, table or list, footer with count, selection and data
  time. Use for any browsable collection.
- **ListDetail**: a list and the selected item's detail, stacked or side by
  side; the list keeps a fixed height or width, the detail fills.
- **FormLayout**: label column aligned, fields in groups separated by `group`,
  submit row aligned with the fields.
- **Section**: a titled part of a region with optional actions on the end side.

## 10. Profiles

A profile packages an application's policy:

| File | Content |
|---|---|
| `ui/styles.rhai` | application-wide component skin |
| `ui/tokens.rhai` overrides | density default, radius scale (keep ≤ 4px), font families |
| `ui/profile.rhai` | audit rule set and its severity |

`gpui-rhai init --profile productivity` installs the reference profile: the
design language defaults, comfortable density, square corners and all audit
rules below. Without a profile, `gpui-rhai check` and the audit report only
correctness problems.

### Audit rules (productivity profile)

| Rule | Checks |
|---|---|
| `row-height-mismatch` | sibling controls in one row with different heights |
| `text-edge-misaligned` | stacked text starts that differ by less than one inset step |
| `spacing-not-nested` | an inner gap greater than or equal to its outer gap |
| `multiple-solid-actions` | more than one solid button in an action group |
| `mixed-type-in-row` | more than one typography size in a row of data |
| `low-contrast-text` | text below 4.5:1 against the color actually painted behind it |
| `literal-geometry` | literal control heights, spacing, font sizes or colors in application source |
| `unresolved-font` | a font family whose whole fallback chain does not resolve on this platform |

Controls and markers align by their own edge, not by the text inside them.
When a composition breaks a rule on purpose, say so on the node:
`row([value, unit]).audit_allow(["mixed-type-in-row"])` (Stat does this for a
figure and its unit). Keep these rare; they are greppable.

## 11. Copy

- Sentence case for labels and titles. Buttons start with a verb.
- Short labels; no marketing copy; no emoji as function.
- Say what is loading, why something is empty, and what failed in plain words;
  keep raw errors selectable.
