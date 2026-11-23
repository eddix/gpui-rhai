# gpui-rhai design principles

**Target specification, adopted 2026-11-15.** The official visual language is
the project's **modern** and **clean** presets. This replaces the previous
cassette/Swiss/Omarchy-influenced visual rules. Implementation and native
acceptance status are tracked in [the migration log](migration-modern-clean.md);
recipe availability alone is not a declaration of completed visual parity.

| Document | Purpose |
|---|---|
| [principles.md](principles.md) | authority, layers, naming and migration boundaries |
| [modern-clean-spec.md](modern-clean-spec.md) | maintained metrics, type, paint and state contracts |
| [atoms.md](atoms.md) | component geometry, typography, states and behavioral invariants |
| [composition.md](composition.md) | putting the components together without distorting their design |
| [themes.md](themes.md) | complete presets, token organization, palette and effect rules |
| [component-coverage.md](component-coverage.md) | official module recipe and acceptance coverage |
| [decisions.md](decisions.md) | decision history; earlier visual choices are not the current target |
| [gallery-wireframes.md](gallery-wireframes.md) | existing acceptance application structure, not the new visual authority |

## Two kinds of users

The framework serves both an application bringing its own design and an
application choosing official defaults. The runtime stays design-neutral;
the registry's official defaults follow modern/clean. Adopting the reference design does
not make React, browser CSS, gpui-component, an agent business model or a fixed
token vocabulary part of the runtime.

## Layers

| Layer | Responsibility |
|---|---|
| R — core/runtime | generic style/paint, text, layout, focus, events, motion, tokens and retained/native state; no hard-coded modern/clean branches |
| L0 — design presets | the exact colors, dimensions, fonts, corners, state colors and effect recipes of modern/clean |
| L1 — source components | public parts and state-to-style mapping; compose the same public runtime API applications can use |
| L2 — layouts/patterns | reusable arrangement and composition, with explicit design consumption rather than private rendering privileges |
| Profile | application choices: design, appearance, optional palette/accessibility overrides and audits |
| Application/extension | domain data, services, business actions and specialized compositions |

Richer paint needed for this design must be exposed once as a generic
mechanism. A private Button painter, a second focus system or copied shadow
implementations in every component are not acceptable substitutes.

## Four target configurations

| Design | Light | Dark |
|---|---|---|
| `modern` | Modern / Light | Modern / Dark |
| `clean` | Clean / Light | Clean / Dark |

Names are fixed; a new script selection API is not specified here. A preset
includes geometry, type, shape, effects and state recipes, not only a palette.
Changing modern ↔ clean may reflow. Changing light ↔ dark inside one preset
keeps geometry. Neither preset is an alias for comfortable/compact.

## Principles

### 1. Reproduce the design, not its approximate mood

The source component, its state selectors and the rendered reference govern.
Preserve fractional dimensions, alpha, multiple layers, variable fallbacks,
font metrics and motion. Do not replace a 0.5px shadow outline with a 2px
border, a gradient edge with a solid fill, or Inter with the platform font
and call the result 1:1.

Reference recipes are component-local. Shared tokens should remove real
duplication, not force different original values to become identical.

### 2. Separate design authority from implementation availability

The [reference](modern-clean-spec.md) identifies what the current runtime can
express, what the binding has not exposed, and what needs lower-level drawing
work. A gap is an implementation requirement, not permission to change the
target. Do not write examples using proposed APIs as if they already exist.

The published registry/reference still documents the actual current API.
During migration its legacy appearance is not evidence for a new visual rule.
The old product PNGs remain regression evidence until deliberately replaced
with approved new GPUI renders.

### 3. Do not confuse component categories with feature parity

Category similarity does not imply feature parity. A read-only DiffViewer is
not an accept/reject editor; a row table is not an editable spreadsheet.

Keep useful SDK capabilities the source does not have: native editing/IME,
virtualization, custom rows, panel splits, Canvas, charts, keyboard and Host
integration. Components without a source counterpart need an explicit derived
target; they are not removed and are not falsely marked as 1:1 accepted.

### 4. Preserve native and controlled-state contracts

Reproduce observable design/state behavior, not React hooks or DOM internals.
Rhai uses the SDK's state ownership, callbacks, focus and lifecycle. Rust/native
motion handles frame-frequency work. A style switch must not replace a keyed
editor, lose selection/undo, restart tasks, or change who owns an event.

### 5. Typography and optical geometry are part of the design

Use the source font, weight, line height, tracking and icon frame. A clean
IconButton's 15px icon need not equal a clean Button's 16px icon. Align the
painted glyph and control center using the real line box; padding cannot
compensate for a wrong font or missing letter spacing.

The source has component-specific heights and fractional sizes; there is no
universal 32px md control, 12px smallest text or 4px-only spacing grid.

### 6. States are full recipes

Normal, hover, pressed, focus, selected/checked, disabled, loading, read-only
and invalid must follow the source combinations. Loading is not just opacity
or disabled. State changes must not accidentally reflow a control unless the
source intentionally morphs its content/width.

### 7. Accessibility remains functional, not cosmetic

Keep keyboard routing, names/roles, native input, IME, focus return, disabled
propagation and user motion/reading preferences. Focus follows the actual
owner. Source color/size risks are recorded separately; do not silently alter
the design or claim accessibility certification from a screenshot.

### 8. Keep business policy outside the SDK

Visual reference material does not automatically add new framework requirements.
Keep domain service policy in applications or extensions. The current migration
retains the existing public module surface.

## Superseded visual rules

The following are no longer requirements: cassette/industrial signatures;
forced square lamps and fader caps; mono/uppercase structural labels; warm
paper plus cobalt as the default; one radius per generic kind; no shadows,
gradients or glass; only weights 400/600; one fixed control height for every
family; a universal 2px ink focus border; and all core motion limited to the
old `fast` duration. Their historical rationale remains in the decision log.

Mechanisms and behavior are not superseded: open tokens/environment, public
parts, callbacks, roving focus, action-derived shortcuts, bounded native work,
RTL, retained identity, and last-good transactional state still apply.

## Verification

- Every official module has a source-matched or explicitly derived design recipe.
- Compare fixed viewport/font/DPR/locale/scale across all four configurations,
  sizes and applicable interaction states. Web reference shots are not GPUI
  passing shots; missing paint features remain explicit failures/gaps.
- Treat font antialiasing separately from wrong geometry, color or layers.
- Source absent → derive and document a target before claiming visual parity.
- Preserve logic, keyboard, accessibility and native performance tests. No
  visual migration may hide failures by relaxing budgets or disabling audits
  globally.
