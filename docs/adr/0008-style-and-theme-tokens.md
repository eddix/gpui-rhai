# ADR 0008: Typed style and semantic tokens

Status: Accepted

Rhai uses stable `Style`, `Length`, and `ColorValue` types rather than a direct
mirror of GPUI styling. Official components use semantic theme token names.
Runtime pseudo-state styles handle hover, active, focus, and disabled visuals.
Filled semantic colors have explicit `on_accent`, `on_danger`, `on_warning`,
and `on_success` foreground tokens; deriving foregrounds from `text_primary`
does not preserve contrast across light and dark palettes.

Theme selection changes invalidate rendering without recompiling ASTs or
discarding state.
