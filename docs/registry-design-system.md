# Official registry visual system

The copied Rhai components use one compact desktop visual language. It is
informed by Omarchy's terminal-native product UI, but it is not an Omarchy
brand surface: components never embed a wordmark, fixed palette, or product
copy. Applications continue to own the copied source and active theme.

## Character

- Dense, keyboard-first desktop UI rather than touch-sized web controls.
- Tiled structure, thin borders, and explicit hierarchy instead of floating
  cards, blur, glow, or decorative shadows.
- Square geometry. Semantic circles remain valid for Avatar, Radio, presence
  dots, and slider thumbs; Tag, Button, panel, and input defaults have
  no corner radius.
- One semantic accent connects focus, selection, current navigation, and the
  primary action. Status colors are reserved for actual status meaning.
- Short labels and aligned values. Component defaults do not invent marketing
  copy or functional emoji.

## Metrics

| Role | Default |
|---|---:|
| Caption | `10px / 14px` |
| Body small | `11px / 14px` |
| Body | `12px / 16px` |
| Subtitle | `13px / 18px` |
| Title | `14px / 20px`, bold |
| Heading | `16px / 22px`, bold |
| Display | `24px / 30px`, bold |
| Display large | `28px / 34px`, bold |
| List and menu row | `28px` |
| Default control | `32px` |
| Control sizes | `24 / 28 / 32 / 36px` |
| Border | `1px` |
| Radius scale | `0 / 0 / 0px` |
| Inline control gap | `6–8px` |
| Panel padding | `10–16px` |

Official components express structural values in their copied Rhai source.
Themes own semantic colors, spacing, radii, and the eight typography roles;
they do not accumulate component-specific dimensions.

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

State priority is `active > focus > hover/cursor > selected > idle`, with
disabled suppressing interaction. Borders are reserved in the idle geometry so
focus never moves neighboring content. Rhai pseudo styles support background,
border, text, and opacity; adding a state in source must produce native paint.

## Component rules

- Buttons use a filled accent only for the primary action. Secondary and
  outline actions retain a visible border; disabled state reduces contrast
  without destroying variant identity.
- Inputs, Textarea, Select, and DatePicker share height, padding, border,
  background, focus, error, and disabled treatment.
- Fixed-height Text controls center their 16px line box inside the declared
  control height. Native single-line Input uses the same centered line box;
  font ascent and descent provide the normal slight optical bias below center.
- Checkbox, Radio, and Switch expose state through geometry and fill, not text
  color alone. Radio remains circular; Switch uses a compact rectangular track
  and thumb.
- Tabs use a selected rail, Accordion uses tiled rows, and Menu/Combobox use a
  fixed check column plus one cursor border.
- Popover, Dialog, Tooltip, Toast, and menus use opaque theme surfaces and thin
  borders. They do not add glass, elevation shadows, or exaggerated rounding.
- Tables use compact clipped cells, low-contrast separators, one selected fill,
  local horizontal scrolling, and raised section headers whose sticky copy is
  visually indistinguishable from its natural row. Progress and Skeleton match
  final geometry.

## Verification

Visual review covers Default Light/Dark, Tokyo Night/Storm, and Catppuccin
Latte/Mocha. At minimum inspect idle, hover/cursor, focus, selected/current,
active, disabled, loading, error, empty, and open overlay states. Theme Studio
is the compact drift detector; form, settings, and data-table examples
remain the product-context checks. Logic, keyboard, accessibility, and native
interaction tests remain mandatory and are never replaced by screenshots.
