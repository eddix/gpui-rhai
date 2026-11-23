---
name: gpui-rhai-design
description: 'Design and review interfaces using gpui-rhai official Modern and Clean presets, including their component recipes, composition, themes, focus and native acceptance. Applications bringing an independent design keep their own visual rules.'
---

# gpui-rhai design rules

The specification is in the reference files below. It is a requirement, not
inspiration: read the file itself before doing UI work. Do not answer from this
page, from an existing screen in the project, or from other design systems.

It governs projects on the official design system: the registry's token base
(`ui/tokens.rhai`) and official components, usually with the `productivity`
profile. The runtime itself is neutral; a project that brings its own design
(see "Layers" in principles.md) keeps its own rules, and these apply only to
the parts of it that use official components.

## How to read it

For a new screen or redesign, read [principles.md](references/principles.md)
and [composition.md](references/composition.md). For a narrow change, read the
relevant contract below. The implementation status is separate from the target:
recipe presence or a passing script loader is not native visual acceptance.

| File | Section | Read when |
|---|---|---|
| principles.md | Layers; Four target configurations; Principles | design authority and runtime boundaries |
| modern-clean-spec.md | Core metrics; Typography; Paint and color | exact maintained component values and unresolved capabilities |
| composition.md | 1 Start from layouts and patterns | building any screen |
| composition.md | 2 Spacing; 3 Alignment; 4 Hierarchy | placing anything |
| composition.md | 4 Hierarchy and copy; 5 Color use; 6 Actions and feedback | status, actions, loading, empty and error states |
| composition.md | 7 Keyboard and overlays; 9 Profiles and audits; 10 Acceptance | focus, overlays and verification |
| atoms.md | 2 Geometry; 3 Typography; 4 Color roles | sizes, insets, type roles, color tokens |
| atoms.md | 5 Markers; 6 Lists; 7 Fields; 8 Overlays | choosing between Button, Tag, Badge, Kbd; lists and tables; fields; dialogs and menus |
| atoms.md | 10 Focus mechanics | focus styles and focus owners |
| atoms.md | 11 New component checklist | writing or changing a component |
| themes.md | all | full preset versus alternate palette, symbolic tokens and effects |
| component-coverage.md | module table | finding a source-backed or explicitly derived public component |
| decisions.md | the dated log | why a rule is what it is |

## Non-negotiables

A floor, not a substitute for the component-specific specification.

- **Layouts and patterns first.** Build screens from `layouts/` (Stack, Inline,
  Toolbar, Region) and `patterns/` (Section, FormLayout, DataView, ListDetail,
  AppShell, ...), not from hand-built rows of padding and gaps.
- **Keep the design axes distinct.** Modern/Clean selects complete geometry,
  type and paint; Light/Dark inside one design changes appearance only.
  Neither design is a density alias. Use each component's size/radius/inset
  recipe, not one universal control height, corner rule or content inset.
- **Keep symbolic values late-bound.** Prefer named metrics, spacing, semantic
  colors and type roles. Preserve component-local differences and fractional
  values; do not normalize the source to a 4px grid. Reading scale must grow
  text and its containing geometry together. Application literals stay literal.
- **Align relationships, not all leaf text.** Align panel/row content edges
  while preserving deliberate trays, inner surfaces, icon frames and field
  insets. Nest spacing according to related/group/section relationships; do
  not apply a numeric gap inequality across independent component internals.
- **Focus follows the actual owner without reflow.** Use the documented focus
  decoration, including the native editor's focus. A shadow edge is not a
  layout border; a focus-ring gap is not a guessed parent-colored fill.
- **One solid action per group;** normal states are quiet and color carries
  status meaning only.
- **Keyboard first.** Preserve each component's documented focus model. Use
  its roving arrow navigation where provided, keep independent actions and
  editors reachable, and let Escape close the topmost overlay and return focus.
  Do not force a whole composite such as a form or panel into one tab stop.
- **Verify real controls.** Run script/schema checks and applicable native
  geometry, interaction and audit checks across both designs/appearances,
  relevant sizes, reading scale and RTL. Investigate findings rather than
  disabling rules globally or silently changing exact reference colors.
  Document remaining paint/text capability gaps; do not call approximations
  pixel-equivalent or turn a pending audit into zero findings.

The runtime stays neutral. Missing drawing capabilities belong in shared
public mechanisms, not private Button painters or per-component substitutes.
Check the project's pinned gpui-pre/gpui-fast source and the SDK API before
using a raw-GPUI helper from memory. Design changes must preserve callbacks,
controlled state, native entities, selection/undo, keyboard and lifecycle.

Finish by running the new component checklist (atoms.md section 11) for
component work, and the audit rules in composition.md for screens.
