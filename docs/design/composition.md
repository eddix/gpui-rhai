# Composition guide

**Modern / Clean composition specification.** Read [principles](principles.md).
The four complete preset variants are implemented. Native visual and
interaction acceptance for the complete migration is still in progress.
Exact component values live in the [reference](modern-clean-spec.md).

## 1. Start from reusable layouts and parts

Prefer the SDK's Stack/Inline/Toolbar/Region and patterns when their behavior
fits. Use the public runtime for a custom composition when it does not. No
layout or business pattern gains a private focus, drawing or state capability.

Gallery's sidebar, preview frame and specimen spacing are demonstration
context, not requirements for every application shell. Keep its
navigation/source/acceptance functions; applications compose their own layouts
using the same public components and design tokens.

## 2. Spacing

Use component aliases over reusable primitive steps, preserving exact source
values, including fractional sizes and optical corrections. The previous
comfortable/compact spacing table is not the modern/clean scale.

Relationships still help group content, but "all inner gaps must be strictly
smaller" is not an unconditional source design rule. A tray, a heading and an
optical offset can intentionally use equal or exceptional steps. Audit the
actual source-matched composition instead of rounding it to pass an old rule.
The proximity audit permits equal steps at the same level, includes intervening
content insets in the enclosing separation, and starts a new hierarchy at a
heading. A plain unheaded group with a larger inner gap still reports a finding;
so do genuine peer-height, type, alignment and contrast failures.

Give a visible section title its heading role. A non-interactive icon explicitly
hidden from accessibility can precede it without becoming the group's first
content. A named image, interactive icon or visible prose still counts as
preceding content. Use this distinction to preserve hierarchy, not to exempt an
ordinary row from the audit.

An actionable leaf tree item owns the typography and spacing within its
label, ornament and readout. A role-only item or a branch wrapping a nested
collection is still ordinary composition. Floating content also has its own
audit scope. Tree items present their first actual text to text-start alignment,
not a leading fold button, swatch or frame. The real text remains subject to
alignment checks. Contrast, reachability and other rules remain independent.

Control heights are family-specific: a Modern md Button is 40px and an md
Input is 36px. The row-height audit compares peers within a control family,
including wrapped controls, rather than forcing these two families to the
same height. Icon actions and text actions are distinct families, including
when a formal wrapper directly forwards their root. Unequal peer text buttons,
peer icon buttons or fields still produce findings; instance keys never create
new families.

`key_legend` is a metadata voice, distinct from ordinary caption prose. It
currently aliases the caption's exact metrics, so classifying a shortcut does
not enlarge it to body type. Key legends stay outside mixed-body-type checks;
ordinary captions and deliberately inconsistent body rows remain audited.

Separate:

- root padding and shadow spread;
- inner-card padding and outer-tray padding;
- field header/hint spacing and the editable text inset;
- icon box and its path's painted extent;
- component-to-component spacing owned by the parent.

## 3. Alignment

Align visual centers and actual text line boxes, not nominal font size. Do
not force every component with `size=md` to have one height: modern Button40
and TextField36, clean Button28 and TextField32 are intentional source values.

A local block should have coherent content edges. It does not follow that
every header, field, table cell and panel in the whole application starts at
one global `metrics.inset`. Source Card content16, Button padding16/10,
fields12/10 and table cells14/12 are different relationships.

Keep numeric data aligned and source tabular figures enabled where
appropriate. Parent fill/scroll ownership remains explicit: a Table/list
scrolls itself; do not add a second scrolling body that steals its wheel.

## 4. Hierarchy and copy

Use source weight/color/size roles. In particular Inter500 is a normal
control/title weight; the old 400/600-only rule is superseded. Do not add
monospace uppercase section labels or square lamps to make a screen feel
"more gpui-rhai". The new identity comes from the source's type, shape,
surface layers, soft status fills and interaction continuity.

Keep titles, descriptions, field labels and values distinct. Use sentence
case unless the source component explicitly transforms its label. Application
copy describes its own data and actions; it is not copied from the source's
fake agent, pricing or project examples.

## 5. Color use

Use the chosen preset's local roles and alpha on the actual background.
Primary ink/reverse-ink is different from semantic blue/info. Source green,
amber, red and purple status badges remain available in their normal states.
Do not restore the old cobalt/default palette or automatically neutralize a
healthy status whose reference uses green.

Preserve label/icon/shape distinctions in addition to color. Inspect contrast
in the final layered composition. Record source low-contrast exceptions and
product decisions; neither silent recoloring nor an unsupported accessibility
claim is acceptable.

## 6. Actions and feedback

Choose hierarchy intentionally; the source's variant showcase is not a rule
to put four primary actions on every application toolbar. Keep action-derived
shortcuts and enabled/dispatch state. Global shortcuts belong to the Host or
application; do not hard-code one command-palette key for all apps.

Loading, empty, invalid, failed and stale-data states belong where their
content is. A source loading Button has a content/motion recipe, not just a
spinner glued to an old button. Keep useful data visible during background
refresh, and do not turn "no results" into a failure solely for red styling.

## 7. Keyboard and overlays

Keep SDK tab order, roving focus, activation keys, Escape ownership, command
bindings and focus return. AppShell F6 regions, controlled dock/hidden/overlay
modes and SplitPane/Resizable remain valid behavior; their appearance needs
derived modern/clean targets because the source does not provide those
generic widgets.

An input receives text in its native entity. A hover preview, a focus-opened
tooltip and a modal panel have different lifetime and dismissal protocols;
do not swap one for another because their screenshots look similar.

Pointer-coarse media rules are distinct from narrow-container layout rules.
Preserve the distinction when adapting touch targets or responsive drawers;
a narrow desktop window does not automatically become a coarse pointer.

## 8. Domain compositions

Specialized product references are not automatically SDK features. Keep agent
control, approvals, billing, networking, upload policy, model selection and
other services outside the runtime. New interactions such as editable cell
ranges or OTP input need independent contracts.

DataView/ListDetail/FormLayout/Section/Stat and generic chart APIs remain
useful. A KPI card built from Chart+Stat is a composition, not proof the SDK
already ships the reference design's complete KPIChart behavior.

## 9. Profiles and audits

Profiles still package application policy. Runtime correctness diagnostics
stay design-neutral. The proximity and peer-height checks model content insets,
heading hierarchies and control families; the installed productivity profile's
strict color/type checks are not a certificate of exact-preset accessibility.

In fixed and responsive grids the audit uses presented layout occupancy:
controls and type compare within a row and text starts within a column.
Spanning items do not join separate tracks on the axis they span. Paint
transforms do not change track membership; visual comparisons still use
the container's coordinate frame. The same row and column rules work for
grids and the corresponding flex stacks. A noninteractive decoration hidden
from accessibility and containing no presented text does not lend its edge
to the surrounding text stack; visible text participates even if omitted
from accessibility.

A resolved `display` or `display_large` role marks primary display type.
Primary display values and their caption, unit or delta do not share a
font-size comparison, but two displayed values on the same line do. The
other inline peers on that line are checked independently. A large literal
font size alone does not create a display role. Color and reachability are
still checked for both groups.

`metadata` is the role for a timestamp, qualifier or secondary ledger
count next to its item name. It defaults to the caption type; a theme can
set its metrics independently and a component can refine them for a preset.
The audit compares metadata with metadata, and `initials` with other
monograms, within the same visual line. Unresolved roles still participate
as ordinary inline text. Neither role skips contrast or reachability.

Absolute children remain visible to their own diagnostics and to action
policy. They do not contribute a title, alignment edge or control height
to the enclosing layout's peer comparisons.

Before a new profile ships, classify each old audit against reference cases:
row-height mismatch must understand family-specific heights; mixed-type and
spacing checks must allow source recipes; contrast checks must distinguish
text, placeholders, decorative and meaningful edges. Validate new rules with
source-matched positive controls and genuinely broken negative controls.
Do not disable all audits or scatter `audit_allow` merely to get green output.

Modern/Clean's exact palettes intentionally retain some weak tones. The default
acceptance choice preserves them and reports measured risks, without relaxing
the contrast rule or declaring accessibility conformance. Fixed stock fixtures
may record only explicitly reviewed roles with exact palette, backing, opacity
and numerical evidence; different roles, customized palettes and additional
alpha remain failures. General application/Host audits are not given that
fixture policy. A contrast bound over a varying surface is not an exact
witness and must remain identified as conservative.

`AuditFinding::contrast` carries the model's numeric evidence. The model tracks
native per-primitive element opacity and current frame membership; invisible
measurements and closed overlays are not visible text. Structural fragments
delegate visibility to their children. Runtime animation, native-private text,
shadow/filter composition and actual GPU samples are separate evidence; the
audit is not a complete pixel-level accessibility verifier.

The Gallery keeps all findings available from its status-bar report, including
the selected theme and color facts. Equal counts must not hide changed evidence,
and a failed report update must not advance the last-published cache.

## 10. Acceptance

For component work use the [atom checklist](atoms.md#11-new-component-checklist)
and the reference matrix. Application layout, business copy and accessibility
are additional checks, not replacements for matched component geometry.
Keep old GPUI product baselines until approved replacements are generated
from the real new implementation. Reference research and comparison captures
stay outside the repository; they are not bundled component assets.
