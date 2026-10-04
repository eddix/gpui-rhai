# gpui-rhai design principles

This is the entry point of the gpui-rhai design system. It defines who the
system serves, how responsibility is split between layers, and the principles
every later document applies. It is the maintained source of truth for design
decisions; the per-audience documents below specialize it.

| Document | Reader | Content |
|---|---|---|
| [principles.md](principles.md) | everyone | layers, principles, visual language, density model, runtime neutrality |
| [atoms.md](atoms.md) | component authors | composition contracts, geometry, states, focus, marker system, checklist |
| [composition.md](composition.md) | application developers | spacing, alignment, hierarchy, color use, keyboard, layouts/patterns, profiles |
| [themes.md](themes.md) | theme authors | token layers, palette constraints, derived colors, default palettes |
| [decisions.md](decisions.md) | maintainers | dated decision log for details not covered above |
| [gallery-wireframes.md](gallery-wireframes.md) | maintainers | acceptance application structure and the L2 APIs it requires |

## Two kinds of users

gpui-rhai serves two different jobs, and the design system must not trade one
for the other.

1. **Bring your own design.** An application wants Rhai-authored GPUI UI but
   has its own visual language — for example a disk-usage treemap that copies
   another product. It needs an open, neutral runtime: arbitrary token
   vocabularies, literal styles, Canvas, custom fonts, and no design rules
   imposed on its code.
2. **Good by default.** An application such as an internal productivity tool
   wants a widget written in a hurry to already look coherent, aligned and
   keyboard-friendly without fine tuning.

The first job is served by the bottom layers staying open. The second is
served by the upper layers being opinionated, and by making the right thing
the default rather than a rule someone has to remember.

## Layers

| Layer | What it is | Stance |
|---|---|---|
| **R — runtime** | nodes, typed Style, Canvas, motion, token registry, environment mechanism, audit engine | **Design-language neutral.** Provides mechanisms, never vocabulary. No token name, typography role, environment value or design rule is hard-coded. |
| **L0 — design tokens** | `ui/tokens.rhai`: semantic color roles, typography roles, spacing scale, metrics, radius roles, derived colors | The design language's vocabulary and default values, shipped as copyable Rhai source. |
| **L1 — components** | `components/*` | Functionally complete and composable. Declare the tokens and environment values they consume. Opinionated defaults that can be turned off through parts and `ui/styles.rhai`. |
| **L2 — layouts** | `layouts/*`: Stack, Inline, Toolbar, Region | Visually neutral structure: arrangement, spacing relationships, alignment, environment propagation. Usable without the visual language. |
| **L2 — patterns** | `patterns/*`: Section, DescriptionList, Stat, FormLayout, InlineState, DataView, ListDetail, AppShell | Opinionated compositions that encode this document's principles through semantic slots. |
| **Profile** | a set of project files installed by `gpui-rhai init --profile <name>` | An application's chosen policy: density and radius preferences, component stylesheet, audit rule set. No profile means no design rules. |
| **Application** | domain meaning, copy, data | Decides what is normal, what is primary, how data is phrased. |

An application may stop at any layer. A bring-your-own-design application
uses R, and optionally `layouts/`. A good-by-default application uses every
layer and a profile.

## Principles

### 1. The specification governs defaults and choice, not existence

The design system decides what a component looks like by default and when to
choose which option. It does not remove a capability because one style does
not use it. Button keeps all seven variants, Tag keeps its color variants, the
motion effects pack stays available. Guidance, defaults and audit rules — not
deletion — keep usage coherent. A capability is removed only when it is
defective or superseded by an equivalent.

### 2. The runtime is neutral

Every vocabulary belongs to a layer above R. Concretely:

- Themes are an open, typed token registry. The tokens an official component
  needs are declared by that component; preparation validates only what the
  mounted components require.
- Typography roles are named freely. The eight core roles are the design
  language's declaration, not a runtime constant.
- Environment values are declared (name, type, allowed values, default)
  before use. `density`, `size` and `disabled` are declared by the design
  language; applications declare their own.
- Scripts can read token values and derive colors (`alpha`, `mix`,
  `readable`), so a data-driven palette follows theme switches.
- `gpui-rhai check` and the runtime audit apply design rules only from a
  selected profile. Without one they report correctness problems only.

### 3. Rules are enforced by defaults and slots first

A rule that depends on someone reading a guide fails for the casual author.
Each composition rule is implemented, in order of preference, by:

1. a default value (numeric cells right-align and use tabular figures);
2. a semantic slot (Toolbar has one `primary` slot);
3. an audit finding (two solid buttons in one action group);
4. guidance in [composition.md](composition.md), for what no default can know
   (which status is "normal" for this domain).

### 4. Switching palette never reflows layout

Palette themes own colors. Metrics, spacing, typography and radius live in the
token base layer and the Host override layer. A theme switch therefore cannot
change control heights or text wrapping.

### 5. Keyboard first, visibly

Every interactive control is reachable and operable from the keyboard, focus is
always visible, and shortcuts are discoverable where the action is offered.
Displayed shortcuts are derived from the same action that executes, so they
cannot drift from the real binding.

### 6. Hierarchy by space, then tone, then line, then color

Grouping is expressed first by whitespace, then by tonal surface steps, then
by hairlines, and only then by saturated color. Saturated blocks are a scarce
resource reserved for the primary action, the current selection and states
that require action. A screen full of boxes or full of colored blocks has no
hierarchy.

### 7. Normal is quiet

Healthy, online, idle and complete states use neutral treatment. Warning and
danger are reserved for deviations, so the one abnormal item stands out.
Success is for confirming a just-finished action or a recovery.

### 8. Motion serves continuity

Core components animate only to preserve continuity of position or presence
(a moving selection thumb, an overlay entering), within the `fast` duration,
and always follow the Host motion policy. Decorative effects live in the
optional `motion/*` pack and are meant for non-task surfaces.

## Visual language

### Sources

| Source | Taken | Not taken |
|---|---|---|
| Cassette futurism (industrial instrument panels) | printed labels, key legends, indicator lamps, color-coded function blocks | CRT glow, scanlines, faux hardware texture |
| Swiss typography | grid discipline, flush-left text, few sizes, hierarchy through weight and color, sentence case | poster-scale contrast, baseline grid for controls |
| Rams-era industrial design | "less but better", color-coded keys, honest controls | organic mid-century forms |
| Lo-fi | warm, low-chroma neutrals | grain, noise, imperfection |

Vaporwave and mid-century graphic design are intentionally excluded: their
pastel gradients, glow and playful forms conflict with flat, low-motion,
high-contrast productivity UI.

### Structural signatures

Recognizability comes from structure that survives any palette, not from a
fixed color scheme. Each signature also solves a usability problem, and each
can be turned off through parts or `ui/styles.rhai`.

1. **Label voice.** Structural labels — sidebar section names, table headers,
   form group names, status-bar field names — use the monospace family at the
   caption size, uppercase for Latin text, in the muted color. A label voice
   replaces a heading; it never stacks on top of one.
2. **Indicator bar.** The current item or keyboard cursor in any list-like
   component is a tonal block plus a 2px accent bar on the start edge, drawn
   inside the content inset so it never pushes text.
3. **Square lamps.** Status indicators are 6px squares, like device LEDs.
   Circles remain only where the circle is semantic: Radio, Avatar,
   presence. Slider caps are rectangular faders.
4. **Visible shortcuts.** Menus, the command palette, tooltips and buttons can
   show their action's shortcut in a lightweight key legend.
5. **Faceted tags.** A Tag may carry a facet segment (`SITE│i18n`), resolving the
   ambiguity of bare context values.

### Default palette: paper, ink and one cobalt spot color

The default light and dark themes follow print logic: a warm, very low chroma
neutral ground (paper / warm graphite), near-black or near-white ink, and a
single cobalt spot color as the accent. Warm ground with a cool spot color is
uncommon in developer tools, and blue keeps the largest hue distance from the
red, amber and green status roles. Exact values and constraints are in
[themes.md](themes.md).

### Geometry

Rectangles are square by default. Radius roles exist (`sm` for markers, `md`
for controls, `lg` for overlays) so a Host or profile can apply a micro radius
consistently; the recommended ceiling is 4px. Borders are 1px hairlines.
Shadows, glass and gradients are not part of the default component language.

## Density and size

Size and density are independent axes.

- **Size** (`xs`, `sm`, `md`, `lg`) is the height class of one control.
- **Density** (`comfortable`, `compact`) is set on a region and inherited.
  `comfortable` is the default and targets 27-inch 4K displays; `compact`
  serves laptop screens and dense panels. Compact shrinks heights, insets and
  outer spacing steps. It never changes font size: laptop screens are viewed
  closer, so text keeps its angular size while space becomes scarce.

| Metric | comfortable | compact |
|---|---:|---:|
| Control `xs` / `sm` / `md` / `lg` | 24 / 28 / 32 / 36 | 20 / 24 / 28 / 32 |
| Row (list, menu, table, tree) | 32 | 28 |
| Content inset (text start, panel padding) | 12 | 8 |
| Marker (Badge, Tag, Kbd) | 20 | 20 |

Typography is CJK-aware: body is 14/22, caption is 12/18 (the smallest text),
and only weights 400 and 600 are used. See [atoms.md](atoms.md) for the full
scale.

### How environment values resolve

Formal components are constructed eagerly ([ADR 0019](../adr/0019-node-prop-component-ownership.md)):
a Button passed into a Toolbar has already rendered before the Toolbar runs.
Environment values are therefore resolved **during native rendering**, the
way inherited text color, direction and locale already are. Components
express size- and density-dependent geometry as environment-variant tokens;
the renderer resolves them against the inherited environment. Changing density
repaints natively without executing Rhai.

This is the final model for presentation-level environment. Logic-level
environment — a component branching its structure on where it is placed —
would require deferred construction and placement-owned state. It is recorded
as a separate runtime proposal in [ADR 0023](../adr/0023-deferred-construction-for-logic-environment.md)
and is not part of the design system.

## Runtime capability gaps

Runtime neutrality also means the bottom layer must be capable enough for a
bring-your-own-design application. These gaps are tracked on a separate
runtime track and do not block the design system:

- Canvas cannot draw text; labels over custom visualizations need positioned
  text nodes.
- Canvas has no pattern fills (hatching must be generated as paths).
- Canvas geometry cannot bind to native signals, so data-driven shape
  animation requires Rhai execution per frame.
- Style lacks letter spacing, per-edge border colors, node rotation/scale,
  multi-stop gradients and general z-index (several are GPUI 0.3.7 limits).
- Rhai array/map data-size limits constrain large script-side datasets.

## Verification

- **Gallery** is the acceptance application for the upper layers. It is built
  from AppShell and L2, and must pass three gates: zero composition-audit
  findings, every scene completable by keyboard alone, and visual baselines in
  comfortable/compact × light/dark.
- **A bring-your-own-design reference example** guards the bottom layer: a
  theme with only its own vocabulary, no official components, derived colors.
- Logic, keyboard, accessibility and native interaction tests remain mandatory
  and are never replaced by screenshots.
