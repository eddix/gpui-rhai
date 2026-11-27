---
name: gpui-rhai
description: 'Build or change gpui-rhai scripts, official components and Rust hosts using the matching SDK references, callback/state contracts and CLI checks. Use gpui-rhai-design alongside it for the official Modern/Clean visual system; independent designs retain their own rules.'
---

# GPUI Rhai

Official starter applications keep Rhai source under `ui/`: an entry `ui/main.rhai` with
`view(ctx)`, the official components copied into `ui/components/` (and
`ui/layouts/`, `ui/patterns/`), the selected full preset `ui/theme.rhai`, the token base
`ui/tokens.rhai` and part styles `ui/styles.rhai`. A thin Rust host mounts the
view. The CLI installs, checks and updates that source:

```text
gpui-rhai init --profile productivity    # in a Cargo project
gpui-rhai add button input table         # copy official modules
gpui-rhai check                          # parse, validate calls, render one headless frame
gpui-rhai dev                            # run the application
```

A Rust Host may instead own the tree and callbacks without a script view.
Read [embedding.md](references/embedding.md), section "Host-owned interactive
trees", for `StaticUiView`, optional resolved theme snapshots and `HostCallback`.
Do not add a Rhai bridge or a second runtime when the Host already owns that UI.

## Read first

| Read | Before |
|---|---|
| skill `gpui-rhai-design` | any visible change in a project on the official token base and components: choosing components, layout, spacing, density, color, focus, states, overlays, copy |
| [rhai.md](references/rhai.md) | writing Rhai: values, functions, callbacks, modules, limits, and the mistakes `check` catches |
| [user-guide.md](references/user-guide.md), section 6 "Callback context" | handling events, state, effects or async work |

Rhai has little training data and gpui-rhai is a DSL on top of it. Read the
reference files; do not answer from this page, from a similar file in the
project, or from memory of other UI frameworks.

These references ship with the SDK; check the project's `gpui-rhai` version,
actual backend alias and enabled features in `Cargo.toml` / `Cargo.lock` first.
After upgrading the CLI, refresh both skills with `gpui-rhai skills <directory>`.
Do not treat historical raw-GPUI helpers or gpui-component conventions as this
independent SDK's API. The runtime is
neutral: a project that brings its own design (its own token vocabulary, no
`ui/tokens.rhai` from the registry, no profile) follows its own rules, and the
design skill and the token rules below apply to projects on the official
design system.

## Non-negotiables

- **Never invent an API.** A built-in function or method exists only if
  [script-api.md](references/script-api.md) lists it; an official component's
  prop, event, slot or part exists only if its page under
  [modules/](references/modules/README.md) lists it. The project may add its
  own: functions, primitives and capabilities its Rust host registers, and
  components it defines with `define_component`. Read those in the project
  (the host's registration code, the component's schema), not here. Rhai
  reports a wrong name at run time, not when the script loads.
- **Run `gpui-rhai check` after every change** and fix its warnings too. It
  validates known calls in every branch and renders one headless first frame.
- **Retained script event callbacks are named functions:** `on_click: Fn("save")`, with
  `Fn("pick").curry(id)` for arguments; the function receives
  `(curried..., ctx, payload)`. The SDK rejects anonymous closures here and
  converts curry data through `UiValue`; rich node factories and native Rust
  handlers have different contracts. This is not a limitation of Rhai's
  general-purpose function pointers.
- **A function sees only its parameters.** It cannot read a script `let`; use
  `const NAME` and `global::NAME`, or pass values in.
- **Components are controlled.** Pass the value (`value`, `open`, `selected_keys`)
  and store what the change callback receives; a component does not keep it for
  you.
- **Stable keys.** Give repeated nodes and anything with an element ref a key
  derived from the data (`with_key`, the component `key` prop), never an index.
- **Tokens, not pixels.** On the official token base, lengths from `theme_length("metrics.*")` and
  `theme_length("space.*")`, colors from `theme_color(...)`, type from
  `.typography(role)`. Lengths are never negative: `offset_px(-4)`, not `px(-4)`.
- **Names.** Do not name a function like a built-in (`index_of`, `contains`,
  `filter`, `split`, `text`): it takes over calls inside imported components.
  Reserved words (`default`, `go`, `match`, `new`, `case`, `with`, ...) cannot
  name anything; quote them as map keys (`"default": ...`).
- **Small functions.** Deeply nested node/style expressions can exceed the
  configured Rhai expression-depth budget. Build a screen from small region
  helpers and evaluate the real branches rather than relying on a fixed
  nesting count or compilation alone.
- **Official components are your source.** Change the copy in `ui/components/`
  when a component must behave differently; `gpui-rhai update` merges later
  upstream changes without overwriting yours.

## A view

```rhai
import "components/button" as button;

fn state_schema() {
    #{ fields: #{ count: #{ schema: #{ type: "integer" },
        "default": #{ type: "integer", value: 0 } } } }
}

fn clicked(ctx, payload) { ctx.set_state("count", ctx.get_state("count") + 1); }

fn view(ctx) {
    button::Button(#{ key: "count", text: `Clicked ${ctx.get_state("count")} times`,
        on_click: Fn("clicked") })
}
```

## References

| File | Load when |
|---|---|
| [rhai.md](references/rhai.md) | writing any Rhai |
| [recipes.md](references/recipes.md) | starting a screen: complete, tested views (settings, form, data view with a table, dashboard) |
| [modules/README.md](references/modules/README.md) | choosing a component, layout or pattern; then load that module's page for its props, events, slots and parts |
| [script-api.md](references/script-api.md) | calling a native function, a `ctx` method, a node or style method, or a primitive |
| [user-guide.md](references/user-guide.md) | the full model: lifecycle, state, callback context, Rust host patterns, layout and assets, themes, overlays, async, hot reload, security, testing, troubleshooting |
| [component-authoring.md](references/component-authoring.md) | writing a new component with `define_component` |
| [embedding.md](references/embedding.md) | mounting views in a Rust GPUI application |

## Workflow

1. For a visible change, read the design skill first.
2. Find the components in [modules/README.md](references/modules/README.md)
   and read their pages; start from the closest [recipe](references/recipes.md).
3. Write the view from layouts and patterns, with tokens.
4. Run `gpui-rhai check` until it passes without warnings, then `gpui-rhai dev`.
5. Run the design review checklist from the design skill against the result.
