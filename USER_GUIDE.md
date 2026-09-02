# gpui-rhai User Guide

This guide is for application authors and coding agents building GPUI interfaces
with gpui-rhai. It explains the mental model, the normal workflow, and the
boundary between Rhai UI code and a Rust GPUI host.

gpui-rhai is not a binding that exposes arbitrary GPUI objects to scripts. Rhai
owns declarative UI composition; Rust owns GPUI, platform mechanisms, trusted
services, and performance-sensitive native paths.

## 1. Mental model

```text
Rust GPUI host
├─ Window, App, Context, async work and trusted services
├─ ScriptViewHost: shared overlay/focus domain for embedded views
└─ gpui-rhai runtime
   ├─ Rhai Engine + restricted module resolver
   ├─ lifecycle, typed state, stores, themes and locales
   ├─ retained UiNode tree and transactional reconciliation
   └─ GPUI renderer and native primitives
      └─ Input, Textarea, overlays, scrolling, IME, Canvas, etc.

Rhai source
├─ view(ctx) -> UiNode
├─ formal source components imported from ui/components/
├─ semantic Style and theme tokens
└─ generation-bound callbacks and effect descriptors
```

The important rules are:

1. Rhai returns declarative `UiNode` snapshots. It never receives a GPUI
   `Window`, `App`, `Context`, `Div`, or `AnyElement`.
2. Rust validates a candidate before committing it. Failed events and reloads
   retain the previous AST, state, callbacks, and visible tree.
3. Official components are copied Rhai source. Applications may read and edit
   them; gpui-rhai does not depend on `gpui-component`.
4. Native Rust remains a first-class path. Use it for host integration,
   privileged operations, high-frequency work, or a mechanism scripts should
   compose rather than implement.

See [Architecture](docs/architecture.md) for the complete runtime design.

## 2. Start a project

From a Cargo application root:

```text
gpui-rhai init
gpui-rhai add button input form_field
gpui-rhai check
gpui-rhai dev
```

Inside this repository, invoke the unpublished CLI with:

```text
cargo run -p gpui-rhai-cli -- --root /path/to/app init
```

`init` creates the normal source tree:

```text
ui/
├─ app.toml
├─ main.rhai
├─ theme.rhai
├─ themes/
├─ locales/
├─ components/
├─ assets/
└─ fonts/

.gpui-rhai/
├─ manifest.toml
└─ baselines/        # pristine installed component source
```

The files under `ui/` belong to the application. The files under
`.gpui-rhai/baselines/` let the CLI distinguish local edits from later bundled
source updates.

Useful commands:

```text
gpui-rhai add dropdown select date_picker
gpui-rhai diff
gpui-rhai update
gpui-rhai metadata
gpui-rhai embed
gpui-rhai theme-studio
```

- `add` installs requested components and their source dependencies.
- `update` three-way merges component updates and adds newly bundled themes;
  it never silently overwrites application-owned source.
- `metadata` emits component schemas, snippets, and Rhai language-server
  definitions from the actual installed APIs.
- `embed` generates production Rust `include_str!`/`include_bytes!` wiring.
- `theme-studio [path]` opens the theme editor and complete component specimen.

## 3. Your first Rhai view

```rhai
import "components/button" as button;
import "components/label" as label;

fn state_schema() {
    #{ fields: #{
        count: #{
            schema: #{ type: "integer", min: 0 },
            "default": #{ type: "integer", value: 0 },
        },
    } }
}

fn increment(ctx, payload) {
    ctx.set_state("count", ctx.get_state("count") + 1);
}

fn view(ctx) {
    column([
        label::Label(#{
            text: "Counter",
            description: "State remains owned by the mounted Rhai view",
        }),
        text(`${ctx.get_state("count")}`)
            .with_style(style().font_size(px(24)).font_weight(700)),
        button::Button(#{
            text: "Increment",
            on_click: Fn("increment"),
        }),
    ]).with_style(
        style()
            .width(px(320))
            .padding(theme_spacing("lg"))
            .gap(theme_spacing("sm"))
            .background(theme_color("surface"))
    )
}
```

Only `view(ctx)` is required. `state_schema`, `init(ctx)`, and `dispose(ctx)` are
optional.

Do not perform effects in `view`. Rendering may be retried, rolled back, or run
because a dependency changed. Use callbacks, effects, capabilities, tasks, or
subscriptions for effectful work.

## 4. State and controlled components

State is declared, schema-checked, and scoped to a stable component instance.

```rhai
fn set_name(ctx, value) { ctx.set_state("name", value); }

input::Input(#{
    key: "profile-name",
    value: ctx.get_state("name"),
    placeholder: "Ada Lovelace",
    on_change: Fn("set_name"),
})
```

The component is controlled: the prop is the business value, while the native
Input entity retains only interaction state such as focus, selection, marked
text, undo history, and caret scrolling.

Best practices:

- Give every stateful or interactive component a stable semantic `key`.
- Store business state in the Rhai state schema or an explicit Host store.
- Treat callback payloads as the component's declared event schema; do not
  infer payload shapes from labels or visual implementation.
- Keep one source of truth. Do not mirror the same value in a Rhai state field,
  a Rust store, and an uncontrolled native widget.
- Use exact nested state paths or typed store fields when partial updates
  matter. Ordinary Rhai Map indexing is not transparently observable.

Compatible state survives keyed rerenders and successful hot reloads.
Incompatible schema changes reset only the affected field to its default.

## 5. Components are source

Formal components use the pattern:

```rhai
import "components/button" as button;

button::Button(#{
    text: "Save",
    variant: "primary",
    size: "md",
    on_click: Fn("save"),
})
```

Component schemas define required/optional props, event payloads, slots, parts,
state, dependencies, assets, and supported runtime API range. Unknown props and
invalid values fail before the component is committed.

Use `part_styles` for an intended component customization point. Edit the
copied `.rhai` source when the product needs a structural change. Do not hide a
structural fork behind a growing stack of arbitrary overrides.

The bundled catalog includes:

- foundations: Label, Divider, Icon;
- actions and status: Button, Tag, Avatar, Progress, Skeleton;
- choices: Checkbox, Radio/RadioGroup, Switch;
- forms: Input, Textarea, FormField, Dropdown, Select, DatePicker;
- navigation: Tabs, Accordion, Collapsible, Menu, Pagination;
- data: Table and native virtual collections;
- overlays: Popover, Dialog, Tooltip, Toast;
- primitives for Box/Text/Image/SVG/Canvas, layout, scrolling, refs, signals,
  layers, and generic overlays.

Use [Component authoring](docs/component-authoring-guide.md) and the component
specifications under `docs/components/` when modifying or creating components.

## 6. Callback context: the part agents most often get wrong

### 6.1 Script callbacks

A normal callback is a Rhai `FnPtr`:

```rhai
fn selected(ctx, value) {
    ctx.set_state("selected", value);
}

select::Select(#{
    key: "country",
    value: ctx.get_state("country"),
    options: countries,
    on_change: Fn("selected"),
})
```

The runtime binds the callback to:

- its script generation;
- the owning component instance path;
- the declared event schema;
- its imported-module invocation context, closure environment, and curried
  values when applicable.

This is why a callback defined inside an imported component module continues to
resolve that module's helpers later. Do not reduce callbacks to a function-name
string or call a retained FnPtr against an unrelated AST.

The first callback parameter is gpui-rhai `UiContext`, not GPUI `Context`.
`UiContext` exposes the safe runtime surface: state, stores, locale, themes,
effects, tasks, subscriptions, refs, signals, and validated commands.

Old callbacks are rejected after reload. Deliveries after unmount or disposal
are discarded by generation/scope ownership.

### 6.2 Rust native handlers callable from Rhai

Use a `NativeHandlerRef` when a Rhai-authored component should call trusted Rust
directly:

```rust
use std::collections::BTreeMap;
use gpui_rhai::{
    EventResponse, NativeHandlerDescriptor, NativeHandlerId,
    ScriptViewExtension, ValueSchema,
};

struct HostBridge;

impl ScriptViewExtension for HostBridge {
    fn configure_engine(
        &self,
        engine: &mut gpui_rhai::RuntimeEngine,
    ) -> Result<(), String> {
        let descriptor = NativeHandlerDescriptor::new(
            NativeHandlerId::parse("host.refresh").map_err(|e| e.to_string())?,
            BTreeMap::from([("click".to_owned(), ValueSchema::UiValue)]),
        ).map_err(|e| e.to_string())?;

        engine.register_native_handler(
            descriptor,
            |event, runtime, window, app| {
                // Trusted foreground Rust. Validate ownership and keep work short.
                let _ = (&event, runtime, window, app);
                Ok(EventResponse::new().stop())
            },
        ).map_err(|e| e.to_string())
    }
}
```

Rhai resolves only pre-registered namespaced IDs:

```rhai
button::Button(#{
    text: "Refresh",
    on_click: native_handler("host.refresh"),
})
```

Native handlers work through formal component callback props and native
primitive callback props. Their declared event names and payload schemas are
checked before Rust runs.

Use native handlers for:

- direct host bridging without an adapter Rhai callback;
- short performance-sensitive event paths;
- foreground coordination with a Rust-owned entity or service;
- validated file/window/application operations intentionally granted by the
  host.

Do not block the foreground thread. Start async or background work in Rust and
deliver validated data back through host state, tasks, subscriptions, or
signals.

### 6.3 HostCallback and custom primitives

`HostCallback` is for a Rust-built `UiNode` tree. Rhai cannot construct or
receive it. A custom primitive is trusted Rust implementing a mechanism that
ordinary source components can compose.

Choose the smallest boundary:

| Need | Use |
|---|---|
| Script state transition or ordinary UI event | Rhai `FnPtr` |
| Rhai component calls trusted Rust directly | `NativeHandlerRef` |
| Rust owns the whole declarative tree | `HostCallback` / `StaticUiView` |
| New GPUI/platform mechanism | custom primitive |
| Per-frame typed paint/layout value | `NativeSignal` or retained native subtree |

## 7. Rust host patterns

### Standalone script application

```rust
let prepared = gpui_rhai::FileScriptView::new("ui/main.rhai")
    .prepare()?;

gpui_rhai::ScriptApplication::new(prepared).run()?;
```

### Embedded view inside a Rust-owned GPUI layout

```rust
gpui_rhai::install(cx);
let host = gpui_rhai::ScriptViewHost::new("main-window", cx)?;

let view = gpui_rhai::FileScriptView::new("plugins/user-view/ui/main.rhai")
    .prepare()?
    .mount(
        gpui_rhai::ScriptViewConfig::new("user-view"),
        host.clone(),
        window,
        cx,
    )?;

let root = host.container(
    gpui::div()
        .flex()
        .child(rust_owned_sidebar)
        .child(view.flex_item()?)
);
```

The Rust application continues to own the window and surrounding tree. The
Rhai view owns only its declared view. Use one `ScriptViewHost` for sibling
views that should share overlay dismissal, Escape routing, portal placement,
and focus fallback. `flex_item()` is the normal direct child for Rust flex
layouts: it supplies a zero basis and `min-width/min-height: 0`, preventing a
wide scrollable Table from becoming the host column's automatic min-content
width. Use `element()` for fixed, absolute, grid, or manually styled placement.

Call `view.dispose(cx)?` when removing a mounted view permanently. Merely not
rendering it for one frame does not dispose its tasks, overlays, and scoped
resources.

See [Embedding](docs/embedding.md) and [Multi-window](docs/multi-window.md).

## 8. Layout, text, images, and assets

Use `row` and `column` for ordinary flex containers:

```rhai
row([
    icon_node,
    text("Build complete"),
]).with_style(
    style().height(px(32)).gap(px(8)).items_center()
)
```

`items_*` controls the cross axis; `justify_*` controls the main axis. Styled
fixed-height Text nodes use their declarations to position their text content.
Input and Textarea own separate native line-layout implementations.

Use semantic layout directions and `margin_start`/`padding_end` where RTL must
mirror. Avoid hard-coded left/right behavior for logical navigation.

Declare shared component assets in component metadata. Application assets live
under `ui/assets` and are referenced by logical identity:

```rhai
image_source(asset("app/icons/check"))
```

SVGs may use `currentColor`. The renderer resolves the node's semantic text
color and retints cached SVG bytes. Shared icons should use a consistent 24×24
viewBox and optical center.

See [Style](docs/style.md), [Assets](docs/assets.md), [Canvas](docs/canvas.md),
and [Locale and RTL](docs/locale-and-rtl.md).

## 9. Themes and Theme Studio

Components refer only to semantic roles such as:

```rhai
theme_color("surface")
theme_color("text_primary")
theme_color("accent")
theme_color("focus_ring")
```

Theme values remain symbolic until rendering. Switching a ThemeVariant advances
the theme generation and repaints/rerenders affected native content without
recompiling Rhai or discarding component state.

Open Theme Studio with:

```text
gpui-rhai theme-studio
gpui-rhai theme-studio ui/theme.rhai
```

Theme Studio supports:

- New, Open, Import-as-copy, and Save for gpui-rhai `.rhai` themes;
- live semantic-token preview;
- derived secondary roles from core anchors;
- contrast warnings without secret color rewriting;
- every official component and important visual state in one specimen.

The bundled catalog includes Default, Tokyo Night, Catppuccin, Ethereal,
Everforest, Gruvbox, Hackerman, Nord, Retro 82, Hermarchy, Futurism, and
Aetheria variants. See [Bundled themes](docs/bundled-themes.md),
[Theming](docs/theming.md), and [Theme Studio](docs/theme-studio.md).

## 10. Overlays, focus, and multiple views

Dropdown, Select, DatePicker, Menu, Popover, Dialog, and Tooltip use the generic
Overlay mechanism. Toast uses the generic Layer mechanism plus declarative
timers.

The Rust Host owns:

- portal order and window-level placement;
- flipping and clamping;
- modal and parent/child dismissal order;
- outside click and Escape routing;
- focus trap/fallback and approved key bindings.

Rhai supplies stable local IDs, controlled open state when appropriate,
content, placement policy, and callbacks. IDs are automatically namespaced by
`view_id` inside a shared host.

`Input.autofocus` and `Textarea.autofocus` apply when their keyed native entity
is first mounted. Overlays retain their content identity while closed, so a
reopen must use the overlay lifecycle policy instead: set
`initial_focus: "first"` to move from the panel to its first focusable
descendant on every closed → open edge. The historical default is
`initial_focus: "panel"`. A structural container with key/click handlers gets
an interaction wrapper and is a tab stop by default; use `tab_stop(false)` when
that container should not precede an inner filter input in the focus order.

Do not simulate overlays by absolutely positioning a child inside a clipped
component. Use the generic Overlay/Layer path so the Host can coordinate the
whole window.

## 11. Async, performance, and high-frequency events

Rhai 1.26 is configured without its `sync` feature. Engine, Dynamic, FnPtr, and
stored invocation contexts stay on the GPUI foreground thread. Background work
must move typed Rust data, not Rhai runtime objects.

The retained Rust diff makes GPUI updates efficient, but Rhai still constructs
the declarative tree for every component that reruns. A Rust-side diff does not
make arbitrary script work free.

Guidelines:

- Keep `view` and ordinary callbacks short; 16 ms is the default slow warning.
- Do not parse files, access the network, or perform blocking work in `view`.
- Use `virtual_collection` for large lists and Tables.
- Keep large stable row sets in `NativeCollection`; let Rhai declare the Table
  and controlled state while Rust sorts and projects only visible rows.
- Use native pointer/wheel handlers and `NativeSignal` for coalesced or
  per-frame values such as playheads, drags, and animation parameters.
- A native handler bypasses Rhai dispatch, but if it dirties a Rhai component,
  that component still runs on the update path.
- Use a retained native subtree or signal-bound property when Rhai must leave
  the hot path entirely.
- Measure complete work, including Rhai evaluation, retained reconciliation,
  GPUI layout, prepaint, and paint.

See [Performance](docs/performance.md), [Virtual lists](docs/virtual-list.md),
[Rust-owned collections](docs/native-collections.md), and
[Custom primitives](docs/custom-primitives.md).

## 12. Hot reload, errors, and production

Development reload is transactional:

- changed modules and dependants compile as one candidate;
- success swaps the generation and preserves compatible state;
- failure keeps the last-good AST, callbacks, tree, effects, and state;
- old callbacks and asynchronous deliveries become stale.

Embedded hosts should read `ScriptViewHandle::last_error()` alongside
`root()`. After a failed callback, delivery, or render, `root()` intentionally
remains the last-good tree and `last_error()` explains why the latest candidate
was not committed.

Use the development inspector (`Command + Option + I` or `F12`) for source
locations, state, semantics, traces, timings, and last errors.

Production workflow:

```text
gpui-rhai check
gpui-rhai embed
cargo build --release
```

Do not edit the generated Rust embed module. Edit `ui/`, run `check`, then
regenerate.

See [Hot reload and production](docs/hot-reload-and-production.md) and
[Development inspector](docs/devtools.md).

## 13. Security boundary

Application-owned Rhai UI has the same trust level as that application's source,
but it is not a general system shell.

By default scripts cannot access:

- arbitrary files or module paths;
- URLs, sockets, subprocesses, or environment variables;
- Rust credentials or arbitrary Rust objects;
- GPUI App/Window/Context handles.

Imports use a restricted declared-source resolver. Props, events, stores,
capabilities, assets, Canvas commands, nodes, overlays, timers, signals, and
virtual collections all have schemas or Host budgets.

Only register capabilities and native handlers the application needs. Validate
authorization and opaque-handle ownership again at the service boundary. A
custom primitive is trusted Rust and belongs to the Host boundary, not the
script sandbox.

See [Security boundary](docs/security-boundary.md) and
[Capabilities](docs/capabilities.md).

## 14. Testing checklist

Run before shipping:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
gpui-rhai check
```

Also exercise:

- keyboard forward/reverse focus and visible focus rings;
- Enter/Space activation and disabled suppression;
- Input/IME, Unicode, clipboard, selection, undo/redo, and read-only behavior;
- overlay open/close, outside click, Escape, modal trap, and focus restoration;
- RTL logical layout and navigation;
- light/dark theme contrast and every component state;
- reduced motion;
- release binaries, not only debug builds.

Visual baselines supplement interaction tests; they do not replace them.

## 15. Agent playbook

When an agent modifies a gpui-rhai application:

1. Read the application's intent, this guide, and the relevant component/source
   files before proposing a design.
2. Inspect the pinned gpui-rhai and Rhai versions. Do not rely on remembered
   Rhai APIs when crate source or a small execution probe can decide.
3. Keep product composition and business state in Rhai unless the Host boundary
   provides a concrete reason not to.
4. Keep GPUI/platform mechanisms, privileged services, and sustained hot paths
   in Rust.
5. Prefer existing public Box/Text/Style/Overlay/Layer/Canvas primitives before
   inventing a specialized native constructor.
6. Use official source components; do not add `gpui-component` as a hidden
   dependency.
7. Preserve stable keys, controlled props, semantic theme roles, locale/RTL,
   keyboard paths, and accessibility labels.
8. Register NativeHandlerRef descriptors before compiling scripts and declare
   exact event schemas. Never invent an adapter Rhai script merely because the
   Rust path was not inspected.
9. Evaluate representative script branches. Rhai compilation alone does not
   prove dynamic function overloads exist.
10. Verify changes with logic tests, native interaction tests, release smoke,
    and real visual inspection in proportion to risk.

## 16. Troubleshooting

### `function not found`

Rhai resolves many overloads dynamically. Read the complete missing signature,
then check the registered feature/package surface and the argument's exact
runtime type. Compilation alone may succeed.

### callback works in the entry module but not an imported component

Do not call a stored FnPtr by name against a fresh unrelated scope. Let formal
component props and gpui-rhai retain the callback's module invocation context.

### NativeHandlerRef fails to resolve

Register a namespaced `snake_case` handler before script compilation. The ID has
one namespace separator, for example `host.refresh`, and the descriptor must
declare the actual event name (`click`, `change`, and so on).

### UI state resets unexpectedly

Check stable component keys and state schema compatibility. Changing a key means
mounting a different component instance.

### theme edits do not appear

Validate the `.rhai` theme, confirm family/variant selection, and check that
components use semantic `theme_color` values rather than literals. Theme Studio
shows the active draft and contrast diagnostics.

### overlay is clipped or dismisses the wrong view

Mount every related view under the same `ScriptViewHost::container`, use unique
local overlay IDs, and declare parent overlay IDs for nested ownership.

### application exits immediately in release

Run the release binary with logs, confirm every embedded component/theme/locale
source is included, and use the repository release smoke. Debug-only inspector
registration and source paths must not leak into release artifacts.

## Further reading

The documentation index is [docs/README.md](docs/README.md). Start with the
architecture and embedding guides, then follow the component/theme/performance
links relevant to the application you are building.
