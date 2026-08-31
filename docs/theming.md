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

`registry/themes/default_light.rhai` and `default_dark.rhai` demonstrate the
serialized `ThemeVariant` shape.

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

## Token and component-metric boundary

Themes own values whose meaning crosses component boundaries: semantic colors,
the standard spacing scale, and radii. They must not grow one required token for
every row height, calendar cell, or control-specific width. Adding a component
must not force unrelated application themes to migrate.

Source-owned `.rhai` components define structural metrics such as control
height, fixed Table row height, calendar cell size, and Textarea line-height
mapping through named `xs`/`sm`/`md`/`lg` helper functions. They pass those
values to native behavior; Rust must not hide component visual constants.
Applications own the copied source and may change these metrics directly.

Official components never hard-code palette colors. New semantic color tokens
are added only when a state has cross-component meaning that cannot be expressed
by the existing surface/accent/border/disabled vocabulary.
