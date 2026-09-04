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

The CLI and runtime are not published yet. Install the CLI from a checkout:

```text
cargo install --path crates/gpui-rhai-cli
```

Then, from a Cargo application root:

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
source updates. `init` never overwrites an existing `src/main.rs`; when one is
present it writes `gpui-rhai-host-snippet.rs` for deliberate integration.

Until `0.1.0` is explicitly released, replace the generated
`gpui-rhai = { version = "0.1", ... }` dependency with the checkout you are
dogfooding:

```toml
gpui-rhai = { path = "/path/to/gpui-rhai/crates/gpui-rhai", features = ["dev-reload"] }
```

For another machine with access to this private repository, pin the exact
tested commit rather than silently following a moving branch:

```toml
gpui-rhai = { git = "https://github.com/eddix/gpui-rhai", rev = "<commit>", features = ["dev-reload"] }
```

Disable `dev-reload` in production builds unless source watching is an explicit
product requirement. The version-only dependency becomes the normal path only
after the runtime crate is published.

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

Imported Rhai modules are declarations, not miniature applications. Their
top-level statements may contain imports, named functions, literal constants,
and direct component declarations. Mutable globals, control flow, arbitrary
calls, and computed registration are rejected before evaluation. Put mutable UI
state in `UiContext` and start work through lifecycle/event APIs.

Rhai compilation validates syntax, not every dynamic overload. The
`gpui-rhai check` command also lints known calls and executes the real initial
lifecycle, but event branches still need tests with representative runtime
value types. Rhai maps are key-sorted maps, not insertion-ordered records, and
scripts do not define Rust-like struct or class types.

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
- Use nested state paths for bounded, schema-checked access, but remember that
  local state still dirties its owning component; the path does not create a
  smaller component boundary. App/window store path accessors additionally
  provide exact nested dependency tracking. Ordinary Rhai Map indexing is not
  transparently observable.

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

Formal component render functions are pure automatic reuse boundaries. When
root state changes, unchanged non-slot props and a clean component subtree let
the runtime reuse the prior component before calling Rhai. State, dependency
subscriptions, callbacks, effects, timers, signals, refs, and virtual
collections remain attached. Node/slot props are compared conservatively and
rerender. Behavior must never depend on how often a render function executes.

The main read paths have deliberately different invalidation semantics:

| Read | Runtime behavior |
|---|---|
| `ctx.get_state(...)` / `get_state_path(...)` | Component-local; a changed field dirties the owning component |
| `ctx.get_app_store(...)` / `get_window_store(...)` | Whole-field dependency |
| `ctx.get_app_store_path(...)` / `get_window_store_path(...)` | Exact bounded path dependency, including keyed-array selectors |
| `ctx.element_bounds(ref)` | Tracked last-committed layout/visual/clip geometry; first render may return `()` |
| `ctx.event_target_bounds()` | Event-only, untracked current-target visual snapshot |
| `ctx.get_signal(...)` during render | Untracked hot value, but makes that component ineligible for render bailout |
| `node.bind_signal(...)` | Native property sampling without a Rhai rerender |

Prefer declarative props and tracked state/store reads for ordinary UI. Prefer
signal bindings or retained native work for values that change every frame.

Use `part_styles` for an intended component customization point. Edit the
copied `.rhai` source when the product needs a structural change. Do not hide a
structural fork behind a growing stack of arbitrary overrides.

The bundled catalog includes:

- foundations: Label, Divider, Icon;
- actions and status: Button, Tag, Avatar, Progress, Skeleton;
- choices: Checkbox, Radio/RadioGroup, Switch;
- forms: Input, Textarea, FormField, Dropdown, Select, DatePicker;
- navigation: Tabs, Accordion, Collapsible, Menu, Pagination;
- data: Table with controlled grouping/collapse and generic sticky-section
  virtual collections;
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
- its imported-module invocation context;
- any `UiValue`-convertible curried arguments.

This is why a callback defined inside an imported component module continues to
resolve that module's helpers later. Do not reduce callbacks to a function-name
string or call a retained FnPtr against an unrelated AST.

A formal component may bind a caller callback to a node it owns, but it must not
pass that callback through another formal component's callback prop. The child
would bind it to its immediate caller, not to the original state owner. A
composite component must receive the child event with its own named handler and
then emit its declared event:

```rhai
fn child_changed(ctx, value) { ctx.emit("change", value); }

child::Child(#{ on_change: Fn("child_changed") })
```

The runtime then dispatches `change` to the composite's `on_change` prop in the
original caller context. This event boundary applies equally to pointer and
keyboard handlers.

Durable callbacks must be named, non-capturing functions. gpui-rhai rejects
anonymous/capturing closures and curry values that cannot cross the `UiValue`
boundary. Pass durable data in component props, state/store fields, or explicit
`Fn("name").curry(value)` arguments instead of relying on a closure environment.

The first callback parameter is gpui-rhai `UiContext`, not GPUI `Context`.
`UiContext` exposes the safe runtime surface: state, stores, locale, themes,
effects, tasks, subscriptions, refs, signals, and validated commands.

Old callbacks are rejected after reload. Deliveries after unmount or disposal
are discarded by generation/scope ownership.

### 6.2 Events, propagation, and geometry

Atomic nodes accept `on(event, handler)`, `on_capture(event, handler)`, and
`on_bubble(event, handler)`. Dispatch order is capture → target → bubble.
Handlers may return `event_response()` and refine it with `prevent_default()`,
`stop()`, `stop_immediate()`, `capture_pointer()`, or `release_pointer()`.

Raw `pointer_down`, `pointer_up`, and `pointer_move` payloads contain pointer
identity/type, `window`, `local`, and scroll-adjusted `content` coordinates,
movement, buttons, modifiers, click count, timestamp, capture state, optional
pressure/tilt, and `target`. Raw `wheel` payloads contain those three coordinate
spaces, delta, precision, modifiers, timestamp, and `target`.

`target` is `{ x, y, width, height }` in window coordinates: the committed
visual bounds of the node owning the currently executing handler. This is
`currentTarget` semantics, not a guessed logical card or the deepest painted
child. The value is captured when the event is dispatched.

Click and component events keep their declared payload type. They do not gain a
hidden map wrapper. Read the same geometry from the callback context instead:

```rhai
fn float_clicked(ctx, payload) {
    let bounds = ctx.event_target_bounds();
    if bounds != () {
        // Pass `#{ id: payload, bounds }` to the app's declared bridge.
    }
}
```

`ctx.event_target_bounds()` is event-only and untracked. Callbacks not
dispatched from a retained node—including actions and custom primitive
emissions—receive `()`. Calling it from render/init/dispose is an error. This is
the correct path for low-frequency geometry use such as positioning a detached
window; do not maintain a resize → store → Rhai geometry channel for it.

Use `element_ref(...)` plus `ctx.element_bounds(ref)` only when rendering must
react to another retained element's last committed geometry. That read creates
an exact dependency, returns `#{ layout, visual, clip }`, and may initially be
`()`. It cannot provide synchronous same-layout feedback.

### 6.3 Rust native handlers callable from Rhai

Use a `NativeHandlerRef` when a Rhai-authored component should call trusted Rust
directly:

```rust
use std::collections::BTreeMap;
use gpui_rhai::{
    EventResponse, FileScriptView, NativeHandlerDescriptor, NativeHandlerId,
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
            BTreeMap::from([("click".to_owned(), ValueSchema::Null)]),
        ).map_err(|e| e.to_string())?;

        engine.register_native_handler(
            descriptor,
            |event, runtime, window, app| {
                // `event.payload` has already passed the declared schema.
                // `event.target` is the optional event-time visual bounds.
                let _ = (event.target, runtime, window, app);
                Ok(EventResponse::new().stop())
            },
        ).map_err(|e| e.to_string())
    }
}

let prepared = FileScriptView::new("ui/main.rhai")
    .extension(HostBridge)
    .prepare()?;
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
checked before Rust runs. `NativeEvent::target` contains the same optional
event-time visual bounds without changing the schema-checked payload.

Use native handlers for:

- direct host bridging without an adapter Rhai callback;
- short performance-sensitive event paths;
- foreground coordination with a Rust-owned entity or service;
- validated file/window/application operations intentionally granted by the
  host.

Do not block the foreground thread. Start async or background work in Rust and
deliver validated data back through host state, tasks, subscriptions, or
signals.

### 6.4 HostCallback and custom primitives

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

While a modal is open, Overlay reasserts that focus remains inside its panel on
every focus-driven GPUI frame. An embedding Host must not repeatedly focus an
ancestor to keep shortcuts alive; register Host shortcuts on the surrounding
interaction domain instead. If a one-off Host focus request races with mount,
the modal reclaims focus so Escape and Tab continue through the modal path.

Do not simulate overlays by absolutely positioning a child inside a clipped
component. Use the generic Overlay/Layer path so the Host can coordinate the
whole window.

## 11. Async, performance, and high-frequency events

Rhai 1.26 is configured without its `sync` feature. Engine, Dynamic, FnPtr, and
stored invocation contexts stay on the GPUI foreground thread. Background work
must move typed Rust data, not Rhai runtime objects.

The retained Rust diff makes GPUI updates efficient, but Rhai still constructs
the declarative tree for every component that actually reruns. Root-dirty
renders bail out unchanged formal component subtrees before Rhai execution;
the retained node diff then handles native changes after rendering. Helper
functions remain part of their owning component boundary.

Guidelines:

- Keep `view` and ordinary callbacks short; 16 ms is the default slow warning.
- Do not parse files, access the network, or perform blocking work in `view`.
- Use `virtual_collection` for large lists and Tables.
- Keep large stable row sets in `NativeCollection`; let Rhai declare the Table
  and controlled state while Rust caches sort/group/collapse order and projects
  only visible rows.
- Use Table's `group_by`, `collapsed_groups`, and `on_group_toggle` contract for
  row grouping. Group values are non-empty strings from a declared column;
  headers count as virtual items, skip selection, and stick by default through
  the generic virtual-collection mechanism.
- Use native pointer/wheel handlers and `NativeSignal` for coalesced or
  per-frame values such as playheads, drags, and animation parameters.
- Bind native signals directly to supported properties. Sampling a signal with
  `ctx.get_signal` during render disables bailout for that component subtree,
  because signal reads intentionally do not create rerender dependencies.
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

The default runtime-error banner uses compact monospace text and the same native
drag-selection/`Cmd-C` path as `text(...).selectable(true)`. A Host that owns a
better error surface may mount with
`ScriptViewConfig::new(id).show_error_banner(false)`; standalone applications
use `ScriptApplication::show_error_banner(false)`. Disabling the banner never
clears `last_error`, so the Host remains responsible for making the failure
visible.

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

Run the portable checks before shipping:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo test --manifest-path tests/native-keyboard/Cargo.toml
bash scripts/audit-visual-baselines.sh
gpui-rhai check
```

The hosted workflow runs these checks on a standard Linux runner. A private
repository receives a limited monthly allowance; removing macOS runners lowers
the multiplier but does not make hosted compute unconditionally free. If the
allowance is exhausted or billing is blocked, jobs stop before their first
step. A public repository or a configured self-hosted runner is required for a
permanently no-GitHub-charge runner path.

On macOS, also build release targets and run `scripts/release-smoke.sh` with the
Metal Toolchain installed. Screenshot, candidate-window IME, native
accessibility, and real input certification require an unlocked interactive
Mac and are intentionally not claimed by portable CI.

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
3. Use named, non-capturing callbacks and `UiValue`-convertible curry data;
   never assume an arbitrary closure can survive the render that created it.
4. Keep product composition and business state in Rhai unless the Host boundary
   provides a concrete reason not to.
5. Keep GPUI/platform mechanisms, privileged services, and sustained hot paths
   in Rust.
6. Prefer existing public Box/Text/Style/Overlay/Layer/Canvas primitives before
   inventing a specialized native constructor.
7. Use official source components; do not add `gpui-component` as a hidden
   dependency.
8. Preserve stable keys, controlled props, semantic theme roles, locale/RTL,
   keyboard paths, and accessibility labels.
9. Register NativeHandlerRef descriptors before compiling scripts and declare
   exact event schemas. Never invent an adapter Rhai script merely because the
   Rust path was not inspected.
10. Distinguish tracked `element_bounds(ref)` from event-only untracked
    `event_target_bounds()`; do not create a resize/store feedback channel for
    one click-time geometry read.
11. Evaluate representative script branches. Rhai compilation alone does not
   prove dynamic function overloads exist.
12. Verify changes with logic tests, native interaction tests, release smoke,
    and real visual inspection in proportion to risk.

## 16. Troubleshooting

### `function not found`

Rhai resolves many overloads dynamically. Read the complete missing signature,
then check the registered feature/package surface and the argument's exact
runtime type. Compilation alone may succeed.

### callback works in the entry module but not an imported component

Do not call a stored FnPtr by name against a fresh unrelated scope. Let formal
component props and gpui-rhai retain the callback's module invocation context.
Also replace anonymous/capturing closures with named functions and durable curry
data.

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

### click-time bounds are missing or stale

For a node event, read `ctx.event_target_bounds()` or `NativeEvent::target`
inside that callback. Do not cache `ctx.element_bounds(ref)` in state on every
resize merely to service a later click. Custom primitive emissions currently
have no renderer-owned event target, so include any primitive-measured geometry
in their declared payload when the mechanism requires it.

### application exits immediately in release

Run the release binary with logs, confirm every embedded component/theme/locale
source is included, and use the repository release smoke. Debug-only inspector
registration and source paths must not leak into release artifacts.

## Further reading

The documentation index is [docs/README.md](docs/README.md). Start with the
architecture and embedding guides, then follow the component/theme/performance
links relevant to the application you are building.
