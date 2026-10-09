---
name: gpui-rhai-design
description: 'The design rules of the gpui-rhai design system for keyboard-first productivity tools. Read before designing or changing any screen, layout, spacing, density, color, focus, component choice, interaction state, overlay, data-heavy view or interface copy built with gpui-rhai official components, and before reviewing UI work. Also use when asked what the rules are, or whether a UI decision follows them.'
---

# gpui-rhai design rules

The specification is in the reference files below. It is a requirement, not
inspiration: read the file itself before doing UI work. Do not answer from this
page, from an existing screen in the project, or from other design systems.

## How to read it

For a new screen or a redesign read [principles.md](references/principles.md)
and [composition.md](references/composition.md) in full. For a narrow change,
read "Principles" in principles.md, then the section for the change:

| File | Section | Read when |
|---|---|---|
| principles.md | Layers; Principles; Visual language; Density and size | always, once |
| composition.md | 1 Start from layouts and patterns | building any screen |
| composition.md | 2 Spacing; 3 Alignment; 4 Hierarchy | placing anything |
| composition.md | 5 Color use; 6 Actions; 7 Feedback in place | status, buttons, loading, empty and error states |
| composition.md | 8 Keyboard; 9 Page patterns; 11 Copy | focus order, whole pages, any user-facing text |
| atoms.md | 2 Geometry; 3 Typography; 4 Color roles | sizes, insets, type roles, color tokens |
| atoms.md | 5 Markers; 6 Lists; 7 Fields; 8 Overlays | choosing between Button, Tag, Badge, Kbd; lists and tables; fields; dialogs and menus |
| atoms.md | 10 Focus mechanics | focus styles and focus owners |
| atoms.md | 11 New component checklist | writing or changing a component |
| themes.md | all | writing a palette or changing tokens |
| decisions.md | the dated log | why a rule is what it is |

## Non-negotiables

A floor, not a substitute for the specification.

- **Layouts and patterns first.** Build screens from `layouts/` (Stack, Inline,
  Toolbar, Region) and `patterns/` (Section, FormLayout, DataView, ListDetail,
  AppShell, ...), not from hand-built rows of padding and gaps.
- **Tokens before values.** No literal pixels or colors in application source:
  `metrics.*`, `space.*`, `spacing.*`, `radius.*`, semantic colors, typography
  roles. Density, size and corners are environment axes, not separate styles.
- **One content edge.** Text in rows, headers and panels starts on
  `metrics.inset`; fields, lists and regions line up on it.
- **Spacing nests.** Relationships go `unit < related < group < section`; an
  inner gap is always smaller than the gap around it.
- **Focus is visible and never moves layout:** a 2px `focus_ring` frame (ink by
  default).
- **One solid action per group;** normal states are quiet and color carries
  status meaning only.
- **Keyboard first.** Every interactive component is one tab stop; arrows move
  inside it; Escape closes the topmost overlay and returns focus.
- **The composition audit reads zero.** With the `productivity` profile,
  `gpui-rhai check` and the runtime audit report literal geometry, misaligned
  text edges, unnested spacing, several solid actions and low contrast; fix
  them rather than opting out.

Finish by running the new component checklist (atoms.md section 11) for
component work, and the audit rules in composition.md for screens.
