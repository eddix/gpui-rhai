# Component authoring guide

An official or application component is a Rhai module with three parts:

1. a structured `/* gpui-rhai ... */` JSON header;
2. an `export_component` schema declaration;
3. a PascalCase render function accepting one props map.

Use `registry/components/label.rhai` as the smallest complete reference.

## Metadata

The header declares component ID, source version, runtime API range,
dependencies, capabilities, and component-owned asset paths. The same metadata appears in
`export_component`; `gpui-rhai check` rejects disagreement.

Asset paths are provider-relative files under `ui/assets`, such as
`icons/chevron_next.svg`. Component source addresses an installed asset through
the application namespace, for example `asset("app/icons/chevron_next")`. The
CLI copies declared assets and records their pristine baselines with the source.

Module IDs are lowercase logical paths such as `components/form_field`.
Components are imported under an explicit alias:

```rhai
import "components/label" as label;
label::Label(#{ text: "Project" })
```

## Schema

Declare every prop, local state field, semantic event, slot, and styleable part.
Unknown props are errors. Defaults must satisfy their own schemas. Every event
named `change` requires an optional callback prop named `on_change`.

Use inclusive/exclusive numeric bounds, `one_of`, `Length`, and
UiValue-convertible schemas when a
public contract needs them. Do not replace a precise union with an unrestricted
map or defer every constraint to native construction. Component-specific tagged
maps, such as Table column widths, still receive semantic validation after their
outer schema succeeds.

Every formal component automatically receives optional `key`, `style: Style`
and `part_styles: map<Style>` props. Route the PascalCase constructor through
the runtime wrapper, then validate normalized props in its render function:

```rhai
fn Counter(props) { component_render("components/counter", props, Fn("render_Counter")) }
fn render_Counter(ctx, props) {
    text(`${ctx.get_state("count")}`)
}
```

The wrapper validates and defaults props, derives the parent/key instance path, mounts declared local state,
scopes callbacks to the component module, rejects duplicate stateful keys, and
cleans unreachable instances only after a successful render. Merge overrides with
`component_style(props, "part_name", base_style)`; the root merge order remains
base, size, variant/state, caller `style`, then caller root `part_styles`.

Stateful components and lifecycle custom primitives require a stable caller
`key`. Never store UI state in mutable script globals.

## Styling

Use semantic theme tokens and the typed `Style` builder. Merge order is:

```text
base -> size -> variant/state -> caller style/part_styles
```

Use only `xs`, `sm`, `md`, and `lg` for component sizes. Preserve the desktop
default cursor for controls. Attach normalized accessibility role/label data to
the root node. Use `padding_start/end` and `margin_start/end` for asymmetric
inline spacing so caller locale direction remains correct.

## Events

Bind callbacks to nodes rather than calling them during rendering:

```rhai
node.on_click(props.on_click)
```

Handlers receive `(ctx, payload)`. They may update declared state, dispatch an
action, emit an event, or call a manifest-declared capability. They may not
access GPUI contexts. Pointer callbacks are handled by default; return
`propagate()` to allow the normalized event to continue to an ancestor handler.

For structural responsive composition, branch only on
`ctx.viewport_class()` (`compact`, `regular`, or `wide`). Hosts may replace the
default 600/1000 logical-pixel boundaries with a validated
`ViewportBreakpoints`; avoid continuous script-side pixel calculations.

## Required tests

Each component needs logic tests, normalized node snapshots, representative
macOS screenshots, keyboard/focus tests, accessibility assertions, and an
example. Its file header must contain purpose, props, events, statefulness, and a
short usage example.
