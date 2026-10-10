# GPUI Rhai

GPUI Rhai builds desktop interfaces from Rhai scripts that your application
owns. A script returns a declarative `UiNode` tree; a Rust runtime keeps its
identity and native state and renders it with GPUI.

- **Components are source.** `gpui-rhai add` copies components, layouts,
  themes and the token base into your repository as editable Rhai. You change
  them like your own code; `gpui-rhai update` merges later upstream changes
  three-way and never overwrites a local edit.
- **A neutral runtime with an optional design system.** The runtime draws what
  a script declares and has no visual opinions. The official registry adds a
  design system for productivity tools: a token base, 64 components, layouts
  and patterns, a composition audit, and the Gallery that shows every component
  in both densities. It is specified in [docs/design](docs/design/).
- **Native where frames matter.** Text editing, virtual lists, tables, charts,
  motion, drag and resize run in Rust; Rhai declares them and handles their
  events. Rhai never runs during layout or paint.
- **Standalone or embedded.** Run a window-owning application, or mount
  isolated script views inside an existing GPUI application.

0.2.0 is the next release (Runtime API 3); the latest published release is
0.1.8. See the [release notes](docs/releases/README.md).

## Quick start

```text
cargo install gpui-rhai-cli --locked
cargo new my-app && cd my-app
gpui-rhai init --profile productivity
gpui-rhai add button input table
gpui-rhai check
gpui-rhai dev
```

`init` adds the runtime dependency, a minimal Rust host (it never overwrites
an existing `main.rs`), the Rhai entry `ui/main.rhai`, the theme, the token
base `ui/tokens.rhai`, the component stylesheet `ui/styles.rhai` and the
manifests; `--profile productivity` adds the design rules `check` reports on.
`add` copies components with their dependencies and an update baseline.
`check` validates scripts, schemas, themes and known calls, then runs one
complete headless first frame. `dev` runs the application. `metadata` writes
editor completions for the installed components and Rhai language-server
definitions for the runtime API.

A view is a Rhai function that returns nodes; callbacks name functions:

<!-- check: pass -->
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

## Explore

```text
gpui-rhai gallery                                   # every component, both densities, with source
gpui-rhai gallery --page table --density compact
gpui-rhai theme-studio                              # edit a theme against every component
```

From this repository, run the same commands through
`cargo run --release -p gpui-rhai-cli -- gallery`. The
[example index](examples/README.md) lists Rust host examples worth copying,
such as `extension_host`, `multi_window` and `byod_treemap` (an application
with its own design and no official components).

## Embed in a GPUI application

A standalone application adapts a prepared view into a window-owning
application:

```rust
let view = gpui_rhai::FileScriptView::new("ui/main.rhai").prepare()?;
gpui_rhai::ScriptApplication::new(view).run()?;
```

An existing GPUI application mounts one or more isolated views through a shared
`ScriptViewHost`, and can suspend inactive views and resume them later; see
the [embedding guide](docs/embedding.md). A host that already owns a plain-data
UI tree can render it without Rhai (`host_owned_tree` example).

## Skills for coding agents

Rhai has little training data and gpui-rhai is a DSL on top of it, so an agent
writes better views with the project's references at hand. Install the two
skills for Claude Code, Codex, Cursor and other agents:

```text
npx skills add eddix/gpui-rhai          # from this repository
gpui-rhai skills .claude/skills         # or the copy matching your CLI version
```

| Skill | Contents |
|---|---|
| `gpui-rhai` | Rhai as views use it, the script API, every module's props and events, tested recipes, the user guide, component authoring and embedding |
| `gpui-rhai-design` | the design rules: principles, component contracts, composition, themes and the decision log |

## Documentation

| To | Read |
|---|---|
| learn the model and build an application | [User Guide](USER_GUIDE.md), [简体中文快速开始](docs/quick-start.zh-CN.md) |
| design screens with the official components | [design specification](docs/design/), [component catalog](docs/components/catalog.md) |
| embed views in a Rust host | [embedding](docs/embedding.md), [multi-window applications](docs/multi-window.md) |
| upgrade | [release notes](docs/releases/README.md), [CHANGELOG](CHANGELOG.md) |
| work on the framework | [architecture](docs/architecture.md), [contributing](CONTRIBUTING.md) |
| find any document | [documentation index](docs/README.md) |

## License

GPUI Rhai is licensed under either Apache-2.0 or MIT, at your option.
