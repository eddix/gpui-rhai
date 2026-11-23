# Component contracts (L1)

**Target: modern / clean.** Read [principles](principles.md) and the
[extracted reference](modern-clean-spec.md). This document replaces the old
visual atom rules; it does not change the current Rust/Rhai API or assert the
new looks already ship. Actual props/events/parts remain in the generated
[module reference](../../gpui-rhai/references/modules/README.md).

## 1. Composition contracts

| Contract | Target |
|---|---|
| Identity | Styling never remounts a native entity or changes component/callback ownership. |
| Size | Use each family's source sizes, usually sm/md/lg, not a universal height. An SDK-only xs needs an explicit derived recipe. |
| Preset | modern/clean changes geometry/type/effects; light/dark changes color recipes. Do not implement clean as compact density. |
| Tokens | Declare consumed tokens/environment. Component-specific aliases preserve source distinctions; shared roles are shared only when actually equivalent. |
| Layout | Size to content or fill according to the component contract. Keep parent arrangement separate from internal insets. |
| Parts | Expose meaningful subparts and state styling without requiring private core APIs. |
| Interaction | Existing controlled state, semantic events, native input and disable propagation remain authoritative. |
| Text | Correct font, weight, tracking and line box; no CJK clipping; ellipsis retains the accessible name. |
| States | Preserve source combinations and paint layers; no accidental layout shift. |
| Focus | Style the real owner, including the inner native editor. Never decorate a wrapper that does not track focus. |
| Commands | Keep action-derived shortcut/enabled/dispatch behavior; appearance does not create a second command model. |
| Motion | Use native sampling, honor None/Reduced, and keep paint/hit geometry synchronized. |

## 2. Geometry

### Metrics

All values are logical pixels at scale100, not device pixels. Exact matrices
are in the [core metric matrix](modern-clean-spec.md#core-metrics).

| Family | modern sm / md / lg | clean sm / md / lg |
|---|---|---|
| Button height | 32 / 40 / 48 | 24 / 28 / 32 |
| Button horizontal padding | 12 / 16 / 20 | 8 / 10 / 12 |
| Button icon box | 16 / 18 / 20 | 14 / 16 / 16 |
| TextField/Select/DatePicker/TimePicker height | 32 / 36 / 40 | 28 / 32 / 36 |
| Field base inset (`--px`; parts adjust each side) | 10 / 12 / 14 | 8 / 10 / 12 |

IconButton is a square: modern sm32/md40 with icons16/18; clean sm24/md28 with
icons14/15. It never inherits the intrinsic width of a text button. In clean
Button, a leading/trailing icon reduces that side's padding by2, matching the
source's optical correction.

Do not force Button md and Input md to be equal-height. Align the control
centers or the appropriate baseline; verify their measured geometry. Enlarge
for text scaling without clipping, but do not use scaling to erase the
baseline source differences.

### Radius roles

No old global kind-to-radius table survives. The baseline values are:

| Shape | modern | clean |
|---|---|---|
| Button sm/md/lg | 10 / 12 / 14 | 6 / 6 / 6 |
| Field sm/md/lg | 9 / 10 / 12 | 6 / 7 / 8 |
| Checkbox | 5 | 4.5 |
| Card | outer20 / inner16 | single10 |
| Popover | 16 | 12 |
| Tooltip | 8 | 6 |
| ordinary Badge | pill | 4 |

Count Badge, person/workspace/agent Avatar, Switch track/thumb and segmented
thumb have their own formulas. Preserve concentric relationships where the
source uses them; do not round every marker or every row by kind. For Tag,
Radio, Slider, Tree and other components without independent source targets,
derive and document a recipe rather than silently keeping the old signature.

### Borders, insets and shadows

Preserve 0.5px/1px hairlines, transparent border space, shadow outlines and
paint order. A shadow outline takes no layout space. A source focus shadow
cannot be replaced by an added layout border. Keep source padding separate
from borders: text-start is not always a universal `metrics.inset`.

modern Card has an outer tray and inner card; clean has a single surface.
modern Dialog also has a layered surface, while clean uses its own panel and
footer recipe. An outer/inner surface is not a second independent component
instance: it must preserve one public owner and a coherent parts boundary.

## 3. Typography

Both designs use the bundled Inter 400/500/600 faces. Code uses its own mono family;
ordinary labels and headers are not globally mono or uppercase. Use fractional
sizes and source tracking rather than rounding to the legacy body/caption
roles. The exact resource/format requirements are in the reference.

| Role | modern | clean |
|---|---|---|
| Button md | 14, 500, tracking-.01em | 13/20, 500, tracking0 |
| Field md value | 13.5/20 | 13/20 |
| common field label | 13/20, 500 | 13/20, 500 |
| small description | commonly12.5/17 | commonly12/16 |
| Card title | 14/20, 500 | 13/20, 500 |
| Card body | 13/20 | 13/20 |
| Dialog title | 15/22, 500 | 15/22, 500 |

These are component roles, not one global text role applied everywhere.
Keep source use of tabular numbers. Some source declarations use other
weights; record the actual loaded-font resolution before adding or
synthesizing a different face. Platform UI fonts and compressed webfont bytes
are not a substitute for the matching native font resources.

## 4. Color roles in components

Use the local role: title/body/secondary/subtle/placeholder/icon, surface/tray,
hairline/strong edge, selected/hover, semantic fg/bg, focus, gradient stop and
shadow layer. Preserve alpha and sRGB mixes. Do not collapse all weak text
into one muted color or all surfaces into one fill.

Primary means ink/reverse ink in this source, not a cobalt fill. Info,
success, warning, danger and purple badges keep paired subtle backgrounds
and foregrounds. Do not remove a source green normal-status badge merely to
enforce the superseded "normal is quiet" rule.

## 5. Actions and markers

- **Button:** source primary/secondary/ghost/destructive recipes; active
  scale0.97, disabled opacity0.4, loading shimmer plus conditional spinner.
  Preserve width/content morph and source icon-side padding. Existing SDK
  extra variants require derived styles, not deletion or a false source match.
- **IconButton:** independent square, centered SVG paint and slot, label
  accessibility. Ordinary actions omit `selected`; an explicit `false`/`true`
  declares a controlled toggle and its accessible pressed state. A tiny glyph centered inside a
  too-large SVG viewBox is not accepted as a correctly sized icon.
- **Badge:** modern soft pill, clean small rounded rectangle; optional dot or
  icon, live-dot motion, count circle/pill. No compulsory old square lamp.
- **Kbd:** use source keycap treatment and actual platform key binding.
- **Tag:** a derived removable chip: 24px high, radius7 modern / radius5 clean,
  13px /12.5px type on a 20px line; 18px close target with a 10px glyph. The
  optional facet uses a paired soft fill/ink rather than a solid-action inverse
  foreground. The outer frame is an inset shadow, not extra layout thickness.

## 6. Lists, tables and selection

Menus, options and commands use their own row height/inset/corner/highlight
recipes. Keep hover, keyboard active item, selection and disabled distinct.
Do not impose one global row-height or outline style on every list family.

Source DataTable body rows are44 (modern) /40 (clean); clean header36. Its
editing, rectangular cell/range selection, clipboard and undo are not the
current SDK Table row-selection model. Style the existing Table without
silently introducing a second spreadsheet model; record functional gaps.

Native data sources, virtualization, stable row keys, sorting, grouping,
sticky behavior and keyboard ownership remain SDK contracts. Derive a target
for standalone List/Tree/SelectableList/ColumnBrowser rather than treating a
business Agent Inbox as the same generic component.

## 7. Fields

TextField's combined API decomposes into SDK Input/InputGroup/FormField;
native editing, IME, caret, selection and undo remain native. Implement the
whole source frame, label/hint/error/count and affix geometry, not just the
input's background. Do not draw a double frame around InputGroup's editor.

- modern Textarea uses padding8×12, radius10, content line21; clean uses
  padding7×10, radius8, line20. Its expand/count/send behavior is separately
  mapped to supported APIs.
- A normal focused field strengthens its neutral edge/shadow; an invalid
  field has its error recipe. Use the source selector combinations, not the
  old global rule that focus always replaces error with ink.
- Checkbox16×16 has rounded box5/4.5 and an animated mark, not a Radio.
- Switch tracks32×20 /26×16 and round thumbs stretch while pressed.
- Source-only NumberInput, OTPInput, DateRangePicker and TagInput are explicit
  gaps, not aliases for Input, Slider or TimeRangePicker.

## 8. Overlays and containers

Match the source surface, shadow stack, radius, insets, max-size and fixed
header/body/footer organization. Menus, tooltips, popovers and dialogs do not
all use one8px shadowless frame. clean Tooltip includes a translucent blur;
omitting it remains an explicit paint gap.

Keep the SDK overlay owner, focus trap/return, Escape routing, clipping,
parent-child overlay relationships and outside press behavior. Browser
z-index integers are evidence of relative stacking, not permission to bypass
native Host/window domains. HoverCard has a separate interactive hover/focus
protocol; it is not merely a larger text Tooltip.

## 9. Motion in components

Use source timing, curves and shape continuity. Button content/width morph
can take480–520ms; a rule limiting every core animation to the old fast
duration is no longer valid. Preserve reduced-motion and Host None/Reduced.
Do not run per-frame Rhai to imitate React timers or RAF.

Motion must update presentation and hit geometry consistently, follow keyed
owners, and stop/clean up on lifecycle transitions. Missing glyph mask, blur
or multi-stop gradient support is not fixed by a different easing curve.

## 10. Focus mechanics

The current public hooks remain `.focus`, `.group_focus`, `.focus_within`
and `.group_hover`. Use them with the actual focus/hover owner; a field frame
follows the inner native editor. Paint an indicator without changing the
source's reserved geometry. Source button outlines often use2px blue with
offset2, but field focus is not the same recipe.

`Style.focus_ring(width, offset, color)` is the shared non-layout outline.
Place it in `group_focus` to outline a Checkbox's box or Switch's track from
the actual row owner; do not add a second focus stop. The same decoration is
available on native Slider/RangeSlider thumb parts. View focus-indicator
policy, transparent gap, clipping and controlled values remain authoritative.

Keep one roving tab stop for a compound control where appropriate; Tab moves
between controls, arrows within a group, and overlay close returns focus.
Do not copy `aria-*` strings as a substitute for the SDK's native semantic
tree. Existing `focus_indicators` policy and text-input focus behavior remain
runtime features, not grounds to invent new style APIs.

## 11. New component checklist

- [ ] Identify source family, preset, appearance and applicable states; for
      source-absent components, document a derived target.
- [ ] Use its actual geometry/font/radius/effect recipe, not legacy defaults.
- [ ] Declare consumed tokens/environment/assets; expose public parts.
- [ ] Preserve controlled state, callback scope, native editing and identity.
- [ ] Confirm actual bounds, SVG paint bounds, text baseline and focus owner.
- [ ] Verify idle/hover/active/focus/selected/disabled/loading/invalid together.
- [ ] Use shared paint/motion mechanisms, not a private component renderer.
- [ ] Honor motion/reading preferences, RTL and long/CJK text.
- [ ] Test keyboard, semantics, failure/reload/lifecycle and native data paths.
- [ ] Capture four-configuration GPUI comparisons only after paint gaps close;
      source screenshots alone do not accept a port.
