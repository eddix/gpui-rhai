# Official registry visual system

The copied Rhai components use one compact desktop visual language. It is
informed by Omarchy's terminal-native product UI, but it is not an Omarchy
brand surface: components never embed a wordmark, fixed palette, or product
copy. Applications continue to own the copied source and active theme.

This document is the maintained visual contract for the official registry.
The [component catalog](components/catalog.md) describes public component
contracts; [visual testing](visual-testing.md) defines their acceptance matrix.
Component changes update these maintained documents rather than leaving a
separate iteration requirements document as a competing source of truth.

## Character

- Dense, keyboard-first desktop UI rather than touch-sized web controls.
- Tiled structure, thin borders, and explicit hierarchy instead of floating
  cards, blur, glow, or decorative shadows.
- Square geometry. Semantic circles remain valid for Avatar, Radio, presence
  dots, and slider thumbs; Tag, Button, panel, and input defaults have
  no corner radius in the bundled themes. All rectangular surfaces, including
  Tabs tracks and thumbs, consume the applicable theme radius role. A Host
  radius override changes them together; Tabs has no screenshot-derived
  exception to the theme contract.
- One semantic accent connects focus, selection, current navigation, and the
  primary action. Status colors are reserved for actual status meaning.
- Short labels and aligned values. Component defaults do not invent marketing
  copy or functional emoji.

## Metrics

| Role | Default |
|---|---:|
| Caption | `11px / 16px` |
| Body small | `12px / 16px` |
| Body | `13px / 18px` |
| Subtitle | `14px / 20px` |
| Title | `16px / 22px`, bold |
| Heading | `18px / 24px`, bold |
| Display | `24px / 32px`, bold |
| Display large | `28px / 36px`, bold |
| List and menu row | `28px` |
| Default control | `32px` |
| Control sizes | `24 / 28 / 32 / 36px` |
| Border | `1px` |
| Radius scale | `0 / 0 / 0px` |
| Inline control gap | `6–8px` |
| Panel padding | `10–16px` |

Typography values above describe the current Default Light/Dark themes; other
themes and Host font overrides may resolve different metrics. Components use
the resolved line box rather than measuring visible glyph ink. The visual
revision does not change global typography tokens.

Themes own semantic colors, spacing, radii, typography, and motion. Components
must consume those roles for panel padding, control insets, content gaps and
decorative corners. A literal equal to today's token value is not equivalent
to using the token: it will not follow a theme or Host preference change.
Component stylesheets provide an explicit application override, not a substitute
for correctly themed defaults.

Structural geometry may remain component-owned: a 1px hairline, icon viewBox,
zero offsets, minimum hit target, virtualization row contract, or a semantic
circle's half-diameter. Record the reason for such constants. Control minimum
heights are lower bounds; resolved text, padding and borders must be allowed
to increase actual height. Do not introduce a required theme token for every
calendar cell or individual component. Reuse the shared scale; introduce a
shared semantic role only when its meaning is not covered by existing roles.

The shared spacing mapping for migration is:

| Use | Existing token |
|---|---|
| Dense track inset / slot gap | xxs |
| Icon-label, label-helper, compact metadata gap | xs |
| Compact control horizontal inset | sm |
| Standard control horizontal inset / panel padding | md |
| Large control inset / section separation | lg |

These are design uses of the existing `theme_spacing` API, not new required
token names. Button xs/sm use sm horizontal inset, md uses md, and lg uses lg;
vertical inset uses xxs for xs and xs for the remaining sizes. Badge uses xs
horizontal inset and xxs vertical padding for md, with its minimum height and
line box preserving the compact enclosure. Component styles can intentionally
override these defaults for an application.

Reference images communicate hierarchy, interaction and surface relationships.
They are not distributable project assets or sources of mandatory pixel values.
Do not copy third-party reference images into this codebase. Requirements must
identify token roles, state behavior, geometry relationships and theme-switch
acceptance; numeric examples describe one theme, not permanent component rules.

JetBrains Mono is the intended Omarchy-family typeface. The runtime does not
silently bundle or force it: hosts register the font as a `FontSource`, then a
theme may set `typography.family` and `fallbacks`. Built-in themes leave the
family unset. Component metrics remain stable with the platform font, and CJK
must be checked with an appropriate host-provided fallback.

Functional icons use SVG rather than font glyphs. Assets share a centered
`24×24` coordinate space and `2px` stroke. Control icons use a `12–16px` box
chosen against the active text line box, so they read at the same optical scale
and align through `items_center`.

## Surfaces and states

Components consume only semantic theme roles:

- `surface`: window, input, menu, dialog, and ordinary panel canvas.
- `surface_raised`: open/current controls and subordinate raised regions.
- `surface_hover`: pointer hover and keyboard cursor.
- `selection`: persistent selected rows and navigation items.
- `accent` / `accent_hover`: primary action, current indicator, and caret.
- `border`: idle structure; `focus_ring`: focused reserved border.
- `text_primary`, `text_muted`, `disabled`: text hierarchy.
- danger, warning, and success roles only for matching states.

Check text contrast against the surface on which it is actually painted, not
only against the page canvas. Enabled small labels target at least 4.5:1;
an unselected Tab is still an enabled control, not disabled text. Adjust the
role pairing or theme palette together so quiet labels remain readable on
tracks, raised panels, selected rows and hover surfaces.

State priority is `active > focus > hover/cursor > selected > idle`, with
disabled suppressing interaction. Borders are reserved in the idle geometry so
focus never moves neighboring content. Rhai pseudo styles support background,
border, text, and opacity; adding a state in source must produce native paint.
For Tabs, the selection thumb persists while focus is drawn independently;
hover never replaces the selected surface or creates a second thumb. Disabled
suppresses input without erasing which content page remains selected.

## Component rules

- Buttons use a filled accent only for the primary action. Secondary and
  outline actions retain a visible border; disabled state reduces contrast
  without destroying variant identity. Buttons reserve a generous action area;
  Badge uses a compact text enclosure. Their density metrics are specified
  below. The distinction must remain visible with the same text and color
  treatment; color and radius alone are insufficient.
- Inputs, Textarea, Select, and DatePicker share height, padding, border,
  background, focus, error, and disabled treatment.
- Fixed-height Text controls center their resolved typography line box inside
  the declared control height. Native single-line Input uses the same centered
  line box for its selected typography role;
  font ascent and descent provide the normal slight optical bias below center.
- Checkbox, Radio, and Switch expose state through geometry and fill, not text
  color alone. Radio remains circular; Switch uses a compact rectangular track
  and thumb.
- Tabs use one continuous background track and one inset selection thumb.
  Unselected tabs show only text/icons, without per-item button chrome or
  separators. The thumb retains the resolved `spacing.xxs` track inset on all
  sides, including the first and last positions; track and thumb use
  `radius.sm`. This sliding-track relationship also works with square corners.
  Content-width
  items are the default, with an explicit equal-width layout available.
- ToggleGroup expresses pressed tool state, with one or multiple selections;
  Tabs selects one associated content panel. The planned equal-size segmented
  ToggleGroup appearance is recorded below as an implementation gap, not as
  current behavior.
- Accordion uses tiled rows, and Menu/Combobox use a fixed check column plus
  one cursor border.
- Popover, Dialog, Tooltip, Toast, and menus use opaque theme surfaces and thin
  borders. They do not add glass, elevation shadows, or exaggerated rounding.
- Tables use compact clipped cells, low-contrast separators, one selected fill,
  local horizontal scrolling, and raised section headers whose sticky copy is
  visually indistinguishable from its natural row. Progress and Skeleton match
  final geometry.

### Button and Badge density

All dimensions are logical pixels. Text uses its resolved typography line box,
not the visible glyph bounds; Host fonts and CJK fallback must remain unclipped.
The density hierarchy is the contract: a Badge encloses a short status label;
a Button reserves space for an action. Compare them with identical text and
colors. Padding and gaps must be selected from the shared spacing scale and
remain smaller for Badge. Keep the choice consistent across Button, IconButton,
Input and related controls, instead of maintaining unrelated pixel tables.

| Component / size | Typography | Current default minimum height | Radius |
|---|---|---:|---|
| Button xs | body_small | 24px | md |
| Button sm / md / lg | body | 28 / 32 / 36px | md |
| Badge sm | caption | 18px | sm |
| Badge md | body_small | 22px | sm |

Both reserve a 1px border. Actual height must accommodate the line box, padding,
and borders as well as the minimum height. With Default Light/Dark typography,
medium Badge is 22px high and medium Button is 32px. These are current baseline
measurements, not a requirement to keep either component at that height under
a different theme. Smaller typography does not shrink below the minimum height;
larger typography grows the control naturally.

Button retains its action variants and hover, focus, active, disabled, and
loading states. Prefix/suffix and loading icons align with the text without
changing control height. Badge is read-only and adds no action tab stop or
pressed feedback. Its optional status dot is a semantic circle; the label gap
uses the compact spacing role. Status
meaning comes from the text and optional dot, not color alone.

A filled, subtle, or outlined application style uses the same Badge geometry;
these are visual treatments, not additional Badge variant enum values. A dark
filled Badge still reads as a compact label. Pill rounding is not required:
Button and Badge continue to consume the theme radius scale. Tag remains
compact application metadata, optionally removable, rather than an action
button or a substitute for status Badge.

### Tabs track and selection thumb

Tabs has two separate surfaces: the `list` is the continuous track, and the
`panel` contains the selected page. The track never encloses the panel.
Unselected tabs are text/icons on the track without independent button fills,
item borders, separators, or a selected underline/side rail. Hover strengthens
the foreground without creating a second selected surface.

For a valid controlled value there is one thumb behind the selected label.
It adds no input handler, focus stop, or semantic node. Its geometry follows
the actual selected slot, including unequal widths; index multiplied by the
first slot width is not a valid positioning rule.

| Property | Default |
|---|---|
| Horizontal track height | Auto: measured slot height plus twice the resolved track inset |
| Tab slot / thumb height | Accommodate resolved text line box, borders and padding; current baseline slot is 26px |
| Track padding, all sides | spacing.xxs (currently 2px in bundled themes) |
| Gap between slots | spacing.xxs of exposed track, no separator |
| Text slot horizontal padding | spacing.sm; follows theme density |
| Icon box / icon-label gap | Component icon size / spacing.xs |
| Content-layout icon-only slot | Square target sized to the resolved row height |
| Vertical track minimum width | 120px; rows share its inner width |
| Track / thumb radius | radius.sm / radius.sm; zero in the bundled themes |
| Label | body, weight 500 in selected and unselected states |
| List-panel gap / panel padding | spacing.sm / spacing.md |

The resolved inset is real layout padding, not an illusion created by rounded
corners. It remains visible above and below the thumb, and at the outer edge
when the first or last tab is selected; the same rule applies vertically.
Focus uses reserved border space without moving neighbors. Decorations must
not fill the inset or clip the label. Defaults use no independent thumb border
and no shadow. A theme may choose square or rounded corners without changing
this structure. Changing spacing or typography must recompute the containing
height; do not combine a theme inset with an unrelated fixed track height.

`layout="content"` uses natural header widths and a track that fits its
contents; `layout="equal"` distributes a supplied horizontal width among the
slots. Vertical tabs share the sidebar width without stretching rows to fill
its height. Horizontal overflow stays on one locally scrollable row; labels
may truncate while their full accessible names remain available. Automatic
reveal of an offscreen controlled selection is not yet implemented.

| Part / state | Semantic color |
|---|---|
| Track | surface_hover |
| Thumb | surface_raised; no independent default border |
| Unselected foreground | text_muted |
| Selected or hovered foreground | text_primary |
| Focus border | focus_ring |
| Disabled foreground | disabled |

Highlight means a distinguishable selection surface; it need not be the
brand accent or literal white. Applications can override `indicator` and
`tab_selected` with a matched fill/foreground pair. Selection persists while
focus is drawn independently; hover must not hide it. A selected disabled tab
retains its selected identity but cannot activate.

A nonempty, view-unique `motion_key` enables shared-layout movement of the
thumb using the Motion `fast` duration and `standard` easing. Without a key,
selection positions directly. `AnimatedTabs` supplies its required `key` as
that motion identity; it does not fade the entire Tabs tree. Movement must
preserve the track inset, follow measured slot width, and respond to the
acknowledged controlled value. It must not make native animation frames emit
Rhai change callbacks or infer selection from a thumb's intermediate position.
Host motion policy, including Reduced/None, continues to govern presentation.
Public props and style parts are listed in the [Tabs contract](components/catalog.md#tabs).

### Known implementation gaps

- Tabs supports local horizontal scrolling but does not automatically reveal
  an offscreen item after controlled selection changes. A future implementation
  must use a post-commit generic scroll-to-ref effect, not a render-time side
  effect, and preserve the track inset at scroll endpoints.
- ToggleGroup's planned visual distinction is equal-width/equal-height button
  segments with one persistent 1px separator at each internal edge. A pressed
  segment fills its own area, without Tabs' surrounding thumb inset; pure-icon
  groups may use square segments. This revision is not implemented in 0.1.5.
  It must preserve single/multiple pressed state and `allow_empty`, and must
  not impose equal widths on independent Buttons or ordinary ButtonGroup.

## Verification

Visual review covers Default Light/Dark, Tokyo Night/Storm, and Catppuccin
Latte/Mocha. At minimum inspect idle, hover/cursor, focus, selected/current,
active, disabled, loading, error, empty, and open overlay states. Theme Studio
is the compact drift detector; form, settings, and data-table examples
remain the product-context checks. Logic, keyboard, accessibility, and native
interaction tests remain mandatory and are never replaced by screenshots.
Control verification includes same-text Button/Badge density comparisons,
content/equal Tabs specimens, first/middle/last thumb inset checks, unequal
labels, icon-only headers, RTL/vertical layouts, and controlled-value rejection.
The maintained [visual testing matrix](visual-testing.md) and native tests in
`tests/native-keyboard/tests/control_visuals.rs` cover these contracts. Test
coverage must be extended as the known implementation gaps are completed.
