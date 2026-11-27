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
                .min_height(px(0)).height(px(34))
                .padding_x(theme_spacing("md"))
                .radius(theme_radius("md")),
            label: style().typography("body").font_weight(600),
        },
        "components/input": #{
            root: style()
                .height(px(36))
                .radius(theme_radius("pill")),
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
→ theme preset rule for the component part
→ controlled-state refinement
→ ui/styles.rhai rule for the component part
→ explicit instance style / part_styles
```

Each step merges like chaining its setters onto the one before, so a rule's
`padding(px(0))` also clears a part's `padding_start`; a logical edge set by the
rule itself wins over a physical one ([Style](style.md)).

Preset recipes are ordinary public Style values, not a native-only skin.
Stylesheet overrides remain above them. CSS-like size constraints still apply:
setting `height` below an inherited `min_height` does not erase that minimum;
clear or replace both when an exact smaller size is intentional.

Within the theme layer, a palette normally inherits the token base's
component recipes field by field. A complete visual preset can set
`component_styles_replace: true` in its theme token map. In that case, its
sheet replaces the base sheet, including omitted parts and properties. Other
tokens still inherit normally, and Host token overrides and `ui/styles.rhai`
still apply on top. Theme Studio preserves this choice when exporting a preset.

When one rendered part combines several public names, resolve them together:
`ctx.component_style(["tab", "tab_selected"], base, state)`. The array order
applies within each layer above; it does not let a selected preset rule jump
over application or instance rules for the common part. Arrays must contain
1–32 unique snake_case part names. The third argument is the component's
controlled state, not an application override.

Do not nest those calls for names on the same element. Resolving `input` and
then passing its already-resolved result through `root` would put the root's
preset above the application's input rule. Input/TimeInput use
`["root", "input"]`, Progress uses `["root", "track"]`, and the standalone
Spinner uses `["root", "indicator"]`. Their array order gives a specific part
the last word *within* each layer, without promoting a theme rule over a Host
override. TimeInput also delegates its normal/error/focus surface to Input;
it does not add a second border or shadow recipe.

An unbounded Input has a native, font-measured preference of 20 zero-character
advances. This is an intrinsic preference, not a fixed width or minimum; the
same editor stretches in a column and shrinks when a row is constrained. Its
value and placeholder do not change that preference. Use ordinary `width`,
`min_width` and `max_width` rules for an application's deliberate field sizing.

Native Input/TimeInput and Textarea editors inherit the hosting node's resolved
font family, fallbacks, features, size, weight and line height. Ordinary
stylesheet or controlled style changes update shaping, caret and hit geometry
without replacing the editor or its focus. Disabled opacity is applied once at
that styled node, so an explicit disabled refinement can replace it. Textarea
uses the same stable 20-character width preference and derives wrapped row
height from the actual line height. As on other nodes, `hover`/`focus` remain
paint-only refinements, not a separate font/layout switching mechanism
([Style](style.md)).

Use `.focus_ring(px(2), px(2), theme_color("focus_ring"))` for an exterior
focus indicator on a UiNode. It follows the view's indicator policy, consumes
no layout space and preserves a genuinely transparent offset gap. It follows
the node's radii/translation without enlarging its hit area; ancestor clips
still apply. A zero width clears an inherited ring. Inside an active
`group_focus` or `focus_within` refinement, it decorates that explicit focus
scope instead; base rings do not automatically appear on every descendant.
Slider and RangeSlider thumb parts share this decoration without replacing
their native focus handles. Other native parts follow the handler's
documented part-paint contract.

For an inset cue, use `focus_ring(px(2), offset_px(-2), color)`. Signed pixel
and rem offsets share the same native painter; neither inset nor outset
consumes layout space. Spans and one-based `grid_cell(column, row)` placement
can overlay ordinary grid children while retaining their intrinsic maximum
size. An invisible measurement copy remains in layout but not the semantic
tree; this is how segmented labels reserve both normal and selected weights
without reflow or duplicate accessible names.

Overlay `focus_frame` parts (Dialog, Sheet, Popover, Menu, Combobox and
DatePicker) now configure this shared decoration on their real panel. Use a
`focus_ring` style for that part, not an absolutely positioned replacement
child or layout-border compensation. The ring follows the final panel's own
corners, paints after its children and disappears when a child takes focus.
The Host's indicator policy also applies to automatically focused panels.

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
let view = gpui_rhai::ScriptView::sources(
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
