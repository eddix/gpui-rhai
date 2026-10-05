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
  key is quoted. The token base declares `density`, `size` and `corners`
  (`square`, `subtle`, `round`; radius roles depend on it). A subtree selects
  one with `.env(#{ corners: "round" })`; a Host sets the application default
  with `ThemeTokenOverrides::environment_defaults`.

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
| `surface_raised` | `#fbfaf6` | `#1d1c19` |
| `surface_hover` | `#e0ddd2` | `#2e2d28` |
| `text_primary` | `#1c1b18` ink | `#e8e4da` |
| `text_muted` | `#5d5950` | `#a19c90` |
| `accent` | `#1f45b8` cobalt | `#3d5fe0` cobalt |
| `accent_hover` | `#183892` | `#5474ea` |
| `on_accent` | `#ffffff` | `#ffffff` |
| `danger` / `on_danger` | `#b3261e` / `#ffffff` | `#ef6b5e` / `#151412` |
| `warning` / `on_warning` | `#855600` / `#ffffff` | `#e2a640` / `#151412` |
| `success` / `on_success` | `#2e6b36` / `#ffffff` | `#7cbd78` / `#151412` |
| `border` | `#d2cec3` | `#3a3833` |
| `focus_ring` | `#1c1b18` ink | `#e8e4da` ink |
| `selection` | `#d5d9e3` | `#212a50` |
| `disabled` | `#9c978c` | `#6b675e` |

In the dark palette cobalt is dark enough to carry white text but below 4.5:1
as text on graphite; components therefore use the derived `text.accent`
(about `#7c90de`) when accent is text.

## 6. Bundled themes

Tokyo Night/Storm and Catppuccin Latte/Mocha are project themes; Ethereal,
Everforest, Gruvbox, Hackerman, Nord and Retro 82 are semantic adaptations of
Omarchy themes; Hermarchy, Aetheria and Futurism are community adaptations.
All bundled palettes contain colors only and satisfy section 4. Adaptations
keep their source attribution at the top of the file. Where a source palette
violates a semantic rule (for example an accent and a warning of the same
hue), the adaptation adjusts the status color and records why in a comment.
