# Theme authoring

For authors of palette themes and token bases. Read
[principles.md](principles.md) first.

## 1. Three token layers

The resolved token set of a window is built from three layers, lowest first:

| Layer | Source | Owns |
|---|---|---|
| **Token base** | `ui/tokens.rhai` (copied from `registry/tokens.rhai`) | typography roles and families, spacing scale, `metrics.*`, radius roles, derived colors, environment declarations |
| **Palette theme** | `ui/theme.rhai`, `ui/themes/*.rhai` | semantic colors and palette-specific derived colors |
| **Host overrides** | `ThemeTokenOverrides` from the embedding Rust Host | user and platform preferences: radius scale, UI and mono font, text scale, density default, motion timing |

A later layer replaces individual tokens of an earlier one. Palette themes
should contain colors only, so switching palette never changes layout
([principle 4](principles.md#4-switching-palette-never-reflows-layout)). A
theme that sets typography or metrics is valid, but it opts out of that
guarantee.

Applications that bring their own design may skip the token base entirely and
write a theme with any vocabulary they like. Nothing is required until a
component that declares a token is mounted.

## 2. Token values

```rhai
fn tokens() {
    #{
        spacing: #{
            sm: px(8),
            lg: by_env("density", #{ comfortable: px(16), compact: px(12) }),
        },
        metrics: #{
            row: by_env("density", #{ comfortable: px(32), compact: px(28) }),
            control: by_env(["density", "size"], #{
                comfortable: #{ xs: px(24), sm: px(28), md: px(32), lg: px(36) },
                compact: #{ xs: px(20), sm: px(24), md: px(28), lg: px(32) },
            }),
        },
        colors: #{
            "text.accent": readable(theme_color("accent"), theme_color("text_primary"),
                theme_color("surface_hover"), 4.5),
            "control.hover": mix(theme_color("surface_hover"), theme_color("text_primary"), 0.06),
        },
    }
}
```

- Lengths: `px`, `rem`.
- `by_env(names, table)` makes a value depend on inherited environment values,
  resolved during native rendering. Nested tables follow the order of `names`.
- Colors: literal `0xrrggbbaa`, `color("...")`, or expressions over other
  tokens: `theme_color(name)`, `mix(a, b, t)`, `alpha(c, a)`, and
  `readable(color, toward, background, ratio)`, which moves `color` toward
  `toward` only as far as needed to reach the contrast `ratio` on
  `background`. Expressions resolve against the final merged token set, so a
  derived color follows the active palette and Host overrides.
- Environment declarations: `environment: #{ density: #{ values: [...],
  "default": "comfortable" } }`. `default` is a reserved Rhai keyword, so the
  key is quoted. The token base declares `density` and `size`. A subtree
  selects a value with `.env(#{ density: "compact" })`; a Host sets the
  application default with `ThemeTokenOverrides::environment_defaults`. Radius
  roles follow the component's kind, not an environment value.

## 3. Semantic colors

Official components declare the colors they read. The design language's
semantic set is:

```text
surface, surface_raised, surface_hover, text_primary, text_muted,
accent, accent_hover, on_accent, danger, on_danger, warning, on_warning,
success, on_success, border, focus_ring, selection, disabled
```

`on_*` colors are foregrounds for text and marks on the matching fill. Choose
them independently; deriving them from `text_primary` is not reliable across
light and dark palettes.

## 4. Constraints

These are checked by the theme tests for bundled themes and by Theme Studio for
drafts.

| Pair or property | Minimum |
|---|---:|
| `text_primary` and `text_muted` on `surface`, `surface_raised`, `surface_hover`, `selection` | 4.5:1 |
| each `on_*` on its fill | 4.5:1 |
| `focus_ring` on `surface` | 3:1 |
| `accent` on `surface` (indicator bar, checked marks) | 3:1 |
| `surface_hover` distinguishable from `surface` | 1.15:1 |
| derived `text.*` on `surface_hover` | 4.5:1 (guaranteed by `readable`) |
| `text_primary` on the derived `tag.facet` and `table.selection` | 4.5:1 (guaranteed by `readable`); each still differs from its base (`surface_hover`, `surface`) |

Semantic rules:

- **Hue separation.** The accent hue keeps at least 30° (HSL) from the danger,
  warning and success hues, and status hues keep at least 30° from each other.
  An orange accent next to an amber warning makes every highlight look like an
  alert.
- **Same semantics in both modes.** If `focus_ring` is the ink color in the
  dark variant, it is the ink color in the light variant too.
- **Selection is not hover.** `selection` and `surface_hover` must be visibly
  different, since a row can be selected and hovered.
- **Status colors are for status.** A palette must not reuse a status color as
  its accent.
- Check contrast against the surface the text is actually painted on, not only
  the window background.

## 5. Default palettes

Paper, ink and one cobalt spot color.

| Role | Light | Dark |
|---|---|---|
| `surface` | `#f3f1ea` paper | `#151412` warm graphite |
| `surface_raised` | `#fdfbf4` | `#21201e` |
| `surface_hover` | `#dfddd7` | `#2e2d2b` |
| `text_primary` | `#1c1b18` ink | `#e8e4da` |
| `text_muted` | `#514d44` | `#bcb7aa` |
| `accent` | `#1f45b8` cobalt | `#3d5fe0` cobalt |
| `accent_hover` | `#173aac` | `#3454d4` |
| `on_accent` | `#ffffff` | `#ffffff` |
| `danger` / `on_danger` | `#b3261e` / `#ffffff` | `#ef6b5e` / `#151412` |
| `warning` / `on_warning` | `#855600` / `#ffffff` | `#e2a640` / `#151412` |
| `success` / `on_success` | `#2e6b36` / `#ffffff` | `#7cbd78` / `#151412` |
| `border` | `#adaca7` | `#50504e` |
| `focus_ring` | `#1c1b18` ink | `#e8e4da` ink |
| `selection` | `#bdcdee` | `#141e3b` |
| `disabled` | `#817f7b` | `#737270` |

In the dark palette cobalt is dark enough to carry white text but below 4.5:1
as text on graphite; components therefore use the derived `text.accent`
when accent is text; it follows the final merged palette rather than a fixed hex.

## 6. Bundled themes

`gpui-rhai init` installs every bundled variant as editable Rhai source:
`ui/theme.rhai` starts as Default Dark and the other variants live in
`ui/themes/`, embedded only as ordinary project-owned files.

| Variants | Origin |
|---|---|
| Default Light and Dark, Tokyo Night and Storm, Catppuccin Latte and Mocha | project themes |
| Ethereal, Everforest, Gruvbox, Hackerman, Nord, Retro 82 | semantic adaptations of the themes in the MIT-licensed [Omarchy repository](https://github.com/basecamp/omarchy/tree/master/themes) |
| Hermarchy | adapted from [Archer Clawbot's Hermarchy](https://github.com/archer-clawbot/omarchy-hermarchy-theme), MIT |
| Aetheria | adapted from [Dizziee's Aetheria](https://github.com/JJDizz1L/aetheria), MIT |
| Futurism | an original palette inspired by [Bjarne Øverli's Futurism](https://github.com/bjarneo/omarchy-futurism-theme); no source code or artwork from it is redistributed |
| Dracula | adapted from [Dracula](https://github.com/dracula/dracula-theme), MIT |
| Molokai | adapted from [Tomas Restrepo's Molokai](https://github.com/tomasr/molokai), MIT (after Monokai by Wimer Hazenberg) |
| Absolutely Light and Dark | adapted from Absolutely, a built-in theme of the OpenAI Codex app, with the anchors republished in [shaw-baobao/codex-themes](https://github.com/shaw-baobao/codex-themes) (MIT) |

All bundled palettes contain colors only and satisfy section 4. Source roles
are translated into the semantic roles rather than copied mechanically: error,
warning, success, focus and their foreground pairs keep their meaning and stay
readable. Where a source palette violates a semantic rule (for example an
accent and a warning of the same hue), the adaptation adjusts the status color
and records why in a comment. Each adapted file repeats its short attribution
at the top so it survives copying into an application; theme identity and
runtime tokens carry no provenance metadata. Theme Studio renders the
canonical component specimen under any variant.

## 7. Palette calibration and display margin

Bundled palettes keep their original page background, accent, status fills and
paired fill foregrounds. They are semantic adaptations, not byte-identical
upstream palettes. Their neutral/tonal ramp is calibrated in Oklab/OKLCH:
change lightness while retaining hue/chroma, reducing chroma only when needed to
stay in sRGB. HSL lightness and equal RGB increments do not give equal perceptual
steps. [Calibration and the per-theme rationale](palette-calibration.md) record
the maintained targets and verification commands.

For bundled themes, the reading target is stronger than the general minimum:
`text_primary` reaches 7:1 and `text_muted` at least 5.2:1 on all four reading
surfaces. Main and secondary text keep at least 0.075 Oklab L separation. Dark
page → raised → hover steps are about 0.05 L each; light themes use a small
upward paper step and a downward hover step. Hover and selection also differ in
lightness, not only hue. These are calibration guardrails, not a claim about
every physical monitor or a new validation requirement on application palettes.

`border` remains a decorative panel/divider role (at least 2:1 against page and
raised surfaces in bundled themes). `control.border` is a derived functional
outline at 3:1 on both surfaces. Input, Textarea, InputGroup, choice triggers,
Checkbox/Radio and outline buttons consume it; an embedded search line and
disabled controls keep their quiet `border` treatment. This avoids increasing
every divider to the contrast needed to identify an editable field. D70 updates
the earlier low-contrast-outline guidance.

Primary hover preserves contrast with `on_accent`; status hovers use
`control.{danger,warning,success}_hover`, which back off a tint before it can make
the paired text unreadable. Syntax foregrounds use `readable` on the tonal code
surface; a status fill is not automatically a suitable small-text color.
Scrollbar thumbs are independently calibrated for 3:1 instead of an arbitrary
low alpha. All derived roles remain ordinary overrideable color tokens.

Theme Studio checks reading text on every surface, primary hover, and derived
control/scrollbar roles using the runtime's own contrast implementation. A
passing palette check is not a full accessibility certification: status still
needs text/icons, focus must be visible, and actual control composition matters.
