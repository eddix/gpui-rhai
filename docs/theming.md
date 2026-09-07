# Theming

Themes are editable Rhai source. Rust defines and validates the semantic token
contract; components never refer to palette-specific color names.

## Required initial tokens

Colors:

```text
surface, surface_raised, surface_hover, text_primary, text_muted,
accent, accent_hover, on_accent, danger, on_danger, warning, on_warning,
success, on_success, border, focus_ring, selection, disabled
```

The `on_*` colors are foregrounds for text and marks rendered on their matching
filled semantic color. Themes choose them independently; deriving them from
`text_primary` is not reliably accessible across light and dark palettes.

Spacing uses `xs`, `sm`, `md`, and `lg`. Radii use `sm`, `md`, and `lg`.
Official themes map all three radii to `0px`: rectangular controls and panels
are square by default. Components give explicit half-size radii only to
semantic circles such as Avatar, Radio, presence dots, and slider thumbs.

Typography requires eight semantic roles:

| Role | Size / line | Weight |
|---|---:|---:|
| `caption` | `10 / 14px` | 400 |
| `body_small` | `11 / 14px` | 400 |
| `body` | `12 / 16px` | 400 |
| `subtitle` | `13 / 18px` | 400 |
| `title` | `14 / 20px` | 700 |
| `heading` | `16 / 22px` | 700 |
| `display` | `24 / 30px` | 700 |
| `display_large` | `28 / 34px` | 700 |

The typography block may also select one shared `family` and ordered
`fallbacks`. Built-in themes leave both unset so the host's platform font
policy remains intact. Components call `style().typography("body")`; explicit
font properties chained afterward override individual role values.

`registry/themes/default_light.rhai` and `default_dark.rhai` demonstrate the
serialized `ThemeVariant` shape.

See [bundled themes](bundled-themes.md) for the installed catalog and source
attribution.

## Theme families

A `ThemeFamily` contains light and dark variants and names the defaults used by
system-following mode. Selection can be overridden at application, window, or
component-subtree scope. The closest subtree override wins.

Changing a selection increments only the theme generation. It does not
recompile Rhai modules or discard component state. Editing `theme.rhai` during
development hot-reloads the validated variant in the existing window.

Rhai may select fixed variants with `set_theme`, `set_window_theme`, and
`set_local_theme`, or follow the actual GPUI `WindowAppearance` with the
corresponding `*_theme_system(family)` methods. App-level changes invalidate all
windows; window and subtree changes remain local.

Literal colors are supported for exceptional geometry, but official components
should use `theme_color("semantic_name")`.

## Token and component-style boundary

Themes own values whose meaning crosses component boundaries: semantic colors,
the standard spacing and radius scales, and typography. They must not grow one
required token for every row height, calendar cell, or control-specific width.
Adding a component must not force unrelated application themes to migrate.

Source-owned `.rhai` components define their sound structural defaults.
Application-wide visual changes belong in `ui/styles.rhai`, whose typed rules
target exact formal component IDs and declared parts. This keeps Button height,
Input padding, or Dialog shadow out of the universal token schema without
forcing applications to fork every call site. Applications may still edit the
copied component source for an actual structural or behavioral fork.

Textarea and Input pass a semantic typography role into native shaping so their
metrics cannot drift from ordinary text. Rust must not hide component visual
constants. See [component stylesheets](component-styles.md) for precedence,
validation, hot reload, and embedded-source wiring.

Official components never hard-code palette colors. New semantic color tokens
are added only when a state has cross-component meaning that cannot be expressed
by the existing surface/accent/border/disabled vocabulary.
