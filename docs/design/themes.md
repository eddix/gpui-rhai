# Design presets and theme authoring

**Target: modern / clean.** The source default is renamed modern; clean keeps
its name. This document replaces the old visual token defaults, not the
runtime's open theme mechanism. The four complete variants are implemented;
the full native visual and interaction migration is still under verification.

Read [principles](principles.md), [exact reference](modern-clean-spec.md), and
the [component map](component-coverage.md) before selecting values.

## 1. Orthogonal design axis, color themes and overrides

| Axis | Owns | Layout |
|---|---|---|
| `design` environment (`modern` / `clean`) | complete component metrics, type, corners, surfaces, effects and state recipes via `by_env("design", ...)` | may differ and reflow |
| color theme (`Neutral`, `Default`, `Catppuccin`, `Tokyo Night`, etc. — light / dark) | semantic surfaces, borders, text, accents, shadows, and categorical palette colors | same geometry |
| compatibility presets (`Modern` / `Clean` Light & Dark) | exact reference colors + default `design` (`"modern"` or `"clean"`) over the shared dual-design token base | selects default `design` |
| accessibility/Host preferences | `density` (`comfortable` / `compact`), `scale` (reading scale), motion and locale | explicit, separately verified |

`design` is an orthogonal `environment` axis alongside `density`, `size`, and
`scale`. Applications switch design language independently of color theme via
`ctx.set_environment("design", "modern" | "clean")`, `ctx.set_window_environment`,
`ctx.clear_environment("design")`, or subtree `.env(#{ design: "clean" })`.
`Neutral Light` and `Neutral Dark` extract the reference neutral color palette
without locking the design axis, while `Modern Light/Dark` and `Clean Light/Dark`
remain thin compatibility presets that supply exact reference colors and default
`design` to `"modern"` or `"clean"`.

## 2. Token layers and expanded color contract

Keep the runtime neutral and the source vocabulary above it. `registry/tokens.rhai`
holds both `modern` and `clean` component stylesheets, typography, geometry, and
recipes keyed by `by_env("design", ...)` and mode-adaptive paint expressions via
`by_mode(light, dark)`. Every bundled theme file supplies:

- **5 surface tiers**: `surface_sunken`, `surface`, `surface_raised`,
  `surface_elevated`, `surface_hover`;
- **3 border tiers**: `border_subtle`, `border`, `border_strong`;
- **2 mode-calibrated shadow anchors**: `shadow.ambient`, `shadow.key`;
- **8×4 non-semantic categorical palette**: `palette.<hue>`, `palette.<hue>_ink`
  (>= 4.5:1 readable foreground), `palette.<hue>_soft`, and `palette.<hue>_border`
  for `blue`, `violet`, `teal`, `green`, `amber`, `rose`, `orange`, `slate`,
  plus ordered aliases `palette.1`..`palette.8` (and `charts.palette_1..8`).

Color-only palette switches remain geometry-stable; `modern`/`clean` switches
occur on the `design` axis and are independent of `density`. Custom 18-color
third-party palettes still load cleanly because `tokens.rhai` provides mode-aware
fallback derivations for the expanded roles whenever a palette omits them.

## 3. Primitive values and component aliases

Extract primitive values without normalizing away source distinctions. Build
aliases for real semantic contexts, for example:

- text: strongest, primary, secondary, subtle, placeholder, icon, on-solid;
- surface: canvas, tray, card/field/overlay and their state washes;
- semantic fg/bg pairs: info, success, warning, danger, purple;
- edges: decorative hairline, functional edge, strong/focused edge;
- geometry: button/field/icon-button sizes and insets; cards' outer/inner
  layers; family-specific row, marker and icon metrics;
- type: button label, field value/label/hint, card title/body, menu row,
  numeric and code roles;
- effects: ordered shadows, gradient fill/edge stops and clip boxes, focus,
  blur/mask and motion recipes.

These are design concepts, not newly registered token names or unsupported
API examples. Reuse existing representations where exact; extend generic
mechanisms where not. Current theme values do not magically become arbitrary
CSS shadow/filter lists because this document names them.

Do not flatten component-local geometry into one global token. For example,
Modern Card's inner radius and Popover's radius are both 16px; giving a shared
radius a new value must not accidentally add another 6px to the Popover.

## 4. Geometry, type and effects

Use the exact [geometry matrix](modern-clean-spec.md#core-metrics),
not the previous comfortable/compact table. Typical modern Button md40/r12
differs from clean28/r6; modern field md36/r10 differs from clean32/r7.
No single `metrics.control` or radius-by-height formula describes both.

Use Inter and the actual source weights/line heights/tracking. Native font
resources must match the source; WOFF/WOFF2 cannot be passed directly to the
current `FontSource`. Respect the font license when introducing resources.

Shadows are lists, not one vague elevation level. Preserve zero-blur shadow
outlines, fractional widths, negative spread, inset, alpha and layer order.
modern's primary gradient border is not just an accent color; clean's
hairline shadow is not a layout border. Ordered/inset shadows are exposed by
the shared Style API. Remaining binding/rendering gaps are explicit in the
specification and work log.

## 5. Color resolution

Keep raw sRGB values and alpha, including eight-digit hex. Resolve
`color-mix(in srgb, …)` in the same space with correct alpha handling.
Do not replace those expressions with HSL adjustments, Oklab calibration or
an arbitrary darker/lighter result in the reference port.

Light/dark recipes have local distinctions: clean Card dark surface is
`#1b1b1e`, while clean Button/TextField use `#1c1c1e`; modern field/Card dark
surface is `#19191b` while modern secondary Button uses `#232326`. Preserve
these values rather than flattening everything into one raised-surface role.

For normal, hover, focus, checked/selected, disabled, loading, invalid and
read-only, preserve the source combinations. Focus is not universally ink or
universally blue; normal fields strengthen their neutral edge, whereas
buttons commonly have a blue outline. Disabled opacity also differs by family.

## 6. Bundled themes

The existing Default/Tokyo/Catppuccin/Omarchy-derived/Dracula/Molokai/
Absolutely palette files remain current implementation assets, with their
attribution and app-owned update behavior. They are not the modern/clean
reference and are not deleted or automatically renamed by this document.

Keep their existing source credits when copying or updating them:

| Variants | Origin |
|---|---|
| Neutral Light/Dark, Default Light/Dark, Tokyo Night/Storm, Catppuccin Latte/Mocha | project semantic adaptations |
| Ethereal, Everforest, Gruvbox, Hackerman, Nord, Retro 82 | [Omarchy themes](https://github.com/basecamp/omarchy/tree/master/themes), MIT |
| Hermarchy | [Archer Clawbot](https://github.com/archer-clawbot/omarchy-hermarchy-theme), MIT |
| Aetheria | [Dizziee](https://github.com/JJDizz1L/aetheria), MIT |
| Futurism | original palette inspired by [Bjarne Øverli](https://github.com/bjarneo/omarchy-futurism-theme); no source code/artwork redistributed |
| Dracula | [Dracula theme](https://github.com/dracula/dracula-theme), MIT |
| Molokai | [Tomas Restrepo](https://github.com/tomasr/molokai), MIT, after Monokai by Wimer Hazenberg |
| Absolutely Light/Dark | Codex Absolutely anchors republished in [shaw-baobao/codex-themes](https://github.com/shaw-baobao/codex-themes), MIT |

[Legacy palette calibration](palette-calibration.md) explains the installed
colors and their tests. Its global7:1/5.2:1 reading targets,2:1 decorative
borders and warm-paper/cobalt anchors do not authorize altering the new
reference. Source soft grays do not all satisfy those constraints; record the
actual contrast/role and obtain explicit approval for any accessibility-driven
design difference. Do not silently weaken existing palette tests in this round.

For the four exact presets the accepted default is preservation plus truthful
risk reporting. An enhanced-contrast design is an explicit selection, not an
automatic mutation of the original palette. General SDK audits remain strict;
Theme Studio reports body, placeholder, metadata, ghost-label and icon pairs,
compositing foreground alpha and warning when the backing is parent-dependent.
For example, the original Modern Light Table header is `#8f8f8f` on white
(about3.23:1). Command keycaps also retain their original normal/active ink
and opacity. These are reported risks, not WCAG passes. Switch's secondary
description also keeps its original soft gray/alpha; Theme Studio reports that
role separately from the primary label. The fixed-fixture test ledger checks
the exact stock palette, component path, role and numeric backing;
it never turns those colors into a general exemption for applications or Host
overrides. Table surface/grid/selection roles are independent of generic lists.

## 7. Theme Studio and migration acceptance

Theme Studio must eventually distinguish a design preset from an appearance
or alternate palette, and show real components in all applicable states.
Its current validation is not proof of modern/clean parity. A green palette
check cannot validate multiple shadow layers, typography, geometry or motion.

Before shipping the new presets:

- close the public paint/text gaps without per-component private renderers;
- implement exact source-matched defaults and explicit derived targets for
  SDK-only controls/variants;
- update the copied registry tokens/styles, profiles, CLI specimens and docs;
- preserve locale/RTL, live switching, identity/focus and user preferences;
- capture four-configuration GPUI baselines under fixed fonts/viewport/DPR;
- run the functional/native matrix as well as visual comparisons.

Do not replace old baselines with source-browser shots or call a partial
approximation a completed 1:1 migration.
