# Component stylesheets

`ui/styles.rhai` is the application-wide visual override layer for formal Rhai
components. It complements theme tokens: themes define reusable semantic
values, while the component stylesheet decides how Button, Input, Dialog, and
their named parts consume layout and paint properties.

```rhai
fn component_styles() {
    #{
        "components/button": #{
            root: style()
                .height(px(34))
                .padding_x(theme_spacing("md"))
                .radius(theme_radius("md")),
            label: style().typography("body").font_weight(600),
        },
        "components/input": #{
            root: style()
                .height(px(36))
                .radius(theme_radius("sm")),
        },
    }
}
```

The outer keys are exact formal component IDs. Inner keys must be parts declared
by that component's schema. Values are the ordinary typed `Style` object, so
hover/active/focus/disabled refinements and symbolic theme colors, spacing,
radii, and typography remain available. There are no CSS strings, descendant
selectors, specificity scores, or access to arbitrary component props.

The merge order is deterministic:

```text
component source defaults
→ ui/styles.rhai rule for the component part
→ explicit instance style / part_styles
```

The final step remains ordinary application code rather than another global
style layer. It is useful for a deliberate one-off, while `styles.rhai` is the
place for a coherent application skin.

The loader validates every component ID, part name, value type, and resource
limit before mounting the view. A failed development reload preserves the last
good sheet and visible tree. A successful stylesheet reload invalidates formal
components without recompiling their modules or resetting state. Theme changes
still resolve symbolic values at render time.

File-backed applications discover `ui/styles.rhai` automatically. An embedded
application installs the generated source explicitly:

```rust
let view = gpui_rhai::EmbeddedScriptView::new(
    generated::app_manifest().entry.clone(),
    generated::script_source(),
    generated::THEME_SOURCE,
)
.component_styles(generated::COMPONENT_STYLES_SOURCE)
.manifest(generated::app_manifest());
```

Component parts are a public styling contract. `gpui-rhai check` rejects stale
rules after a component is removed or changes its declared parts instead of
silently ignoring them.
