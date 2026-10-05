# Theming

Themes are editable Rhai source. Rust defines and validates the semantic token
contract; components never refer to palette-specific color names.

## Token layers

Since 0.2.0 the runtime requires no fixed token set. A window's tokens come from
three layers, lowest first (details in [design/themes.md](design/themes.md)):

| Layer | Source | Owns |
|---|---|---|
| token base | `ui/tokens.rhai`, copied from `registry/tokens.rhai`; Hosts pass it with `.token_base(...)` | typography roles, spacing scale and relationship aliases, `metrics.*`, radius roles, derived colors, environment declarations (`density`, `size`) |
| palette theme | `ui/theme.rhai`, `ui/themes/*.rhai` | the semantic colors |
| Host overrides | `ThemeTokenOverrides` | user and platform preferences |

Each component declares the tokens and environment values it reads in its
metadata. Preparation validates the active theme against the union of the
mounted components, so an application that brings its own design (see
`examples/byod_treemap`) needs neither the token base nor the official color
names. Official registry components require the token base.

The default palette roles are:

```text
surface, surface_raised, surface_hover, text_primary, text_muted,
accent, accent_hover, on_accent, danger, on_danger, warning, on_warning,
success, on_success, border, focus_ring, selection, disabled
```

The `on_*` colors are foregrounds on the matching fill. The token base derives
the rest from them: `text.accent`/`text.danger`/`text.warning`/`text.success`
(the status color nudged toward `text_primary` until it reads at 4.5:1),
`control.hover`, `tag.facet`, `table.selection` (28% accent over surface,
precomposited so it stays stable in virtual paint layers), `tabs.foreground`,
`scrollbar.thumb`, syntax and chart colors. A theme may override any derived
token by name.

Typography roles in the token base are CJK-aware (body 14/22, caption 12/18)
plus the control roles `control_small` 13/16 and `control_regular` 14/20 and
the mono `label` voice; weights are 400 and 600 only. Families default to the
platform UI font and the system mono family (`Menlo`, then `DejaVu Sans Mono`).
Components call `style().typography("body")`; explicit font properties chained
afterward override individual values.

Lengths can vary with an environment value: `by_env("density", #{ comfortable:
px(32), compact: px(28) })`. They resolve during native rendering against the
nearest `.env(#{ density })` ancestor, so one subtree can be compact inside a
comfortable window.

Motion has semantic duration (`instant`, `fast`, `normal`, `slow`, `ambient`),
easing (`standard`, `entrance`, `exit`, `emphasized`), spring (`responsive`,
`gentle`, `bouncy`), distance (`subtle`, `moderate`, `large`), and stagger
(`tight`, `normal`, `relaxed`) roles. Themes may override the validated default
table with a `tokens.motion` block. Components read roles through
`ctx.motion_duration`, `motion_easing`, `motion_spring`, `motion_distance`, and
`motion_stagger`; those reads are tracked so a hot theme switch rerenders only
the affected component sources and retargets from the current sample.

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

## Host user-preference overrides

An embedding Host can apply one validated partial token layer to every loaded
theme. This is the appropriate boundary for application/user preferences such
as a platform-specific corner scale, preferred UI font, text scale, motion
timing, or chart palette. It avoids cloning or rewriting bundled and user theme
source:

```rust
use std::collections::BTreeMap;
use gpui_rhai::{EmbeddedScriptView, Length, ThemeTokenOverrides};

let view = EmbeddedScriptView::new(entry, scripts, default_theme)
    .theme_sources(additional_themes)
    .theme_token_overrides(ThemeTokenOverrides {
        radii: BTreeMap::from([
            ("sm".into(), Length::Pixels(4.0)),
            ("md".into(), Length::Pixels(7.0)),
            ("lg".into(), Length::Pixels(10.0)),
        ]),
        ..ThemeTokenOverrides::default()
    });
```

`FileScriptView` exposes the same builder. Overrides merge colors, spacing,
radii, individual typography roles, motion roles, and individual namespaced
tokens. Typography family and fallback entries replace their respective theme
values when supplied. Theme identity and mode are deliberately not
overridable. The resulting variant must pass the normal token validation or
preparation/reload rejects the candidate and retains the last-good theme. File
theme hot reload reapplies the Host layer automatically.

Rhai may select fixed variants with `set_theme`, `set_window_theme`, and
`set_local_theme`, or follow the actual GPUI `WindowAppearance` with the
corresponding `*_theme_system(family)` methods. App-level changes invalidate all
windows; window and subtree changes remain local.

Scripts may also read what resolved: `ctx.theme_variant()` returns
`#{ family, name, mode }` for the context's window and component scope (or
`()` for missing/unresolvable themes or an unavailable borrow). Reading during
render tracks a theme dependency. Effects use explicit dependencies; reading
only inside the effect body does not subscribe or restart that activation:

```rhai
fn render_Palette(ctx, props) {
    let info = ctx.theme_variant();
    effect("palette", info, Fn("start_palette"), Fn("stop_palette"));
    // Return the UI derived from info.
}
```

`ctx.theme_variants()` lists every loaded variant as the same maps, ordered by
family and name, for a theme picker; the Gallery's title bar builds its theme
Select from it and calls `set_theme` on change.

Declare that effect in the formal component schema. Its start callback receives
the same metadata as deps and restarts only when they change. Event-time reads
are imperative, not subscriptions. Rust uses lightweight `ThemeVariantInfo` or
`resolved_theme_selection()`; these are resolved identity, not ThemePreference.
Motion getters share the resolver and clone only motion tokens, never the full
theme. Token overrides do not rewrite family/name/mode.

Mount establishes native appearance before init, effects and initial render,
including secondary windows; it does not replay init to correct a guessed mode.
Standalone/headless contexts with no native appearance explicitly resolve
System using Dark until real window information is available. First actual
Light appearance invalidates pre-existing fallback readers normally.

Mounted views retain a weak, View-owned subscription to native window
appearance notifications. Changed modes enter the same runtime resolver before
deferred foreground work, including an otherwise idle window. Native token-only
UI also repaints without executing Rhai. Identical modes do not invalidate
again. Suspended views retain the new environment until resume without starting
effects; disposal cancels the subscription, so an old Handle cannot update a
replacement. This does not change the application's OS theme preference.

Literal colors are supported for exceptional geometry, but official components
should use `theme_color("semantic_name")`.

## Token and component-style boundary

Tokens hold values whose meaning crosses component boundaries: semantic colors
(palette), the spacing scale, shared metrics such as `metrics.control` and
`metrics.row`, radius roles and typography (token base). They must not grow one
token for every calendar cell or control-specific width. Adding a component
must not force unrelated application themes to migrate.

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
