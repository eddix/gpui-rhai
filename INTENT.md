# GPUI Rhai: Project Intent

## 1. Product definition

**GPUI Rhai is a desktop UI system in which a stable Rust runtime renders a
declarative `UiNode` tree produced by Rhai. Developers build applications and
components primarily from inspectable, editable, hot-reloadable Rhai source,
and extend the system through explicit, typed Rust capabilities.**

The project applies the source-ownership philosophy popularized by shadcn/ui to
GPUI applications without pretending that the runtime itself can be copied as
loose scripts:

- `gpui-rhai` is a normal Cargo dependency. It owns the Rhai engine, GPUI
  integration, rendering mechanisms, state, lifecycle, and safety boundaries.
- Components, themes, locale bundles, and their small assets are copied into the
  user's repository by the `gpui-rhai` CLI. They belong to the user and may be
  modified without waiting for an upstream release.
- A versioned registry and local installation baseline make source updates
  reproducible, inspectable, and mergeable.

GPUI Rhai is an independent project parallel to `gpui-component`. It must not
depend on `gpui-component`, directly or through an optional feature.

## 2. Intended user experience

A Rust developer initializes a minimal host, adds source components, and writes
the application UI in Rhai:

```text
gpui-rhai init
gpui-rhai add button dropdown dialog
gpui-rhai dev
```

The generated Rust host should remain small:

```rust
let view = FileScriptView::new("ui/main.rhai")
    .extension(/* application service */)
    .prepare()?;
ScriptApplication::new(view).run()?;
```

The Rhai entry point imports copied modules and returns a declarative tree:

```rhai
import "components/button" as button;
import "components/dropdown" as dropdown;

fn view(ctx) {
    // Build and return the root UiNode.
}
```

`view(ctx) -> UiNode` is the only required lifecycle function. `init(ctx)` and
`dispose(ctx)` are optional. Rendering is pure: external effects may start only
from `init` or event handlers, never from `view`.

Rhai is not limited to component styling. It is the primary language for view
composition and lightweight UI interaction. Rust remains responsible for
business services, persistence, filesystem and network access, platform APIs,
and other side effects.

## 3. Architecture

### 3.1 Rust runtime

The runtime:

- initializes and configures Rhai;
- compiles, caches, evaluates, and hot-reloads script modules;
- owns the GPUI application and window lifecycle;
- stores component and application UI state;
- reconciles keyed node identity;
- converts `UiNode` values into GPUI elements;
- schedules events, actions, animation frames, tasks, and subscriptions;
- manages focus, accessibility, overlays, themes, locales, assets, and errors;
- exposes explicitly registered primitives and capabilities.

Rhai evaluation and event handling run serially on the GPUI foreground thread in
the first release. Expensive work must execute inside asynchronous Rust
capabilities. Script operation and time budgets provide diagnostics and prevent
accidental UI starvation.

The runtime pins one compatible GPUI release or commit and re-exports its GPUI
types. Rust extensions must use that re-export so an application cannot
accidentally link incompatible GPUI type universes.

### 3.2 Declarative node boundary

Rhai never owns GPUI `Window`, `App`, `Context`, `Div`, or `AnyElement` values.
It constructs a runtime-defined `UiNode` tree. The runtime validates and renders
that tree into GPUI.

This boundary exists to:

- keep the script API stable across GPUI changes;
- avoid leaking Rust lifetimes and context types into Rhai `Dynamic` values;
- enable validation, diagnostics, reconciliation, and developer tooling;
- maintain a clear safety and capability boundary.

The runtime exposes both visual primitives and behavior primitives. Mechanisms
that require native input, frame scheduling, or window coordinates belong in
Rust. Examples include text input and IME, selection and clipboard behavior,
scrolling, overlays, focus scopes, animated values, and virtual lists. Rhai
controls composition, policy, appearance, and semantic state.

### 3.3 Rhai component source

Official and user-authored components use the same declaration mechanism. A
formal component is registered with `export_component` and declares one schema
covering:

- props and defaults;
- local state and defaults;
- named slots and render callbacks;
- emitted events and payloads;
- styleable parts;
- runtime API compatibility;
- component dependencies and required capabilities.

Functions that merely return nodes may be used as render helpers, but they do
not receive component identity, local state, lifecycle, or a devtools entry.

Public components accept a single props map. Unknown props are errors by
default. Component modules are imported under explicit aliases and export
PascalCase component names; they do not inject global constructors.

Each `.rhai` component starts with a structured machine-readable metadata block
for identity, source version, runtime API range, dependencies, and capabilities,
followed by concise human-facing documentation covering purpose, props, events,
statefulness, and a usage example. The exported component schema remains
authoritative for runtime value validation; registry checks reject disagreement
between the header metadata and exported schema.

## 4. State, identity, and data flow

### 4.1 Controlled components and local state

Reusable stateful components are controlled-first: values and change handlers
can be owned by the caller. Convenience uncontrolled behavior is allowed, but
state lives in the Rust runtime rather than mutable Rhai globals.

Formal components declare a state schema. `UiContext` may read or write only
declared keys. A state mutation is batched and invalidates the corresponding
GPUI entity.

Every stateful component instance requires a stable `key`. Dynamic interactive
lists also require stable sibling keys. Identity is derived from the parent
instance path and key. Duplicate sibling keys are development errors. Static,
stateless nodes may use positional identity.

### 4.2 Shared state

Applications may declare typed, in-memory stores at application or window
scope. Store reads are tracked so only affected consumers are invalidated.
Official reusable components must not depend on application stores implicitly;
they receive data through props and communicate through events.

Runtime state and stores survive compatible hot reloads but do not persist
across process restarts. Persistence is an explicit capability.

### 4.3 Events and actions

Rhai callbacks receive a restricted `UiContext` and normalized payloads. They
may update state, emit declared semantic events, dispatch actions, or invoke
declared capabilities. They cannot access raw GPUI contexts or events.

Pointer events bubble from the hit node toward its ancestors and may return
`handled` or `propagate`. There is no DOM-style capture phase. Keyboard input is
first offered to the focused primitive and then to the action/keybinding system.

Semantic actions decouple UI controls from physical shortcuts. Rust or
configuration maps platform key bindings to namespaced action identifiers, and
buttons, menus, and handlers dispatch those identifiers.

## 5. Capabilities and extensibility

### 5.1 Capability boundary

Filesystem, network, persistence, process, and platform services are unavailable
unless the Rust host registers them. The application manifest explicitly
declares every capability and compatible version; startup and `check` fail on a
missing or incompatible implementation.

Values crossing the Rust/Rhai boundary use a validated `UiValue` model:

- null, booleans, integers, floats, and strings;
- arrays and string-keyed maps;
- explicitly registered opaque handles for expensive or native resources.

Arbitrary Rust values are not placed into Rhai `Dynamic`. Capability inputs and
outputs have schemas. Diagnostics redact sensitive values.

One-shot asynchronous work returns a `TaskHandle`; continuous event streams
return a `SubscriptionHandle`. Completion callbacks run on the GPUI thread.
Handles bind to app, window, or component lifetime and are cancelled when their
scope disappears. Callbacks originating from obsolete hot-reloaded ASTs do not
run.

### 5.2 Custom primitives

The runtime provides a stable Rust registration API for namespaced custom
primitives. A registration supplies:

- a node name and props schema;
- a `UiNode`-to-GPUI renderer;
- normalized event adapters;
- optional instance state and lifecycle support.

This is a first-class initial-release extension point. It allows applications to
add editors, canvases, or domain-specific controls without modifying the runtime
or exposing raw GPUI to Rhai.

## 6. Styling, themes, locale, and assets

### 6.1 Styling

`Style` is a stable, typed runtime value rather than an unrestricted mirror of
GPUI's API. Components consume semantic design tokens instead of hard-coded
palette colors and standard spacing.

All visual components accept a caller `style` override. Composite components
declare named `part_styles`. The merge order is fixed:

```text
base -> size -> variant/state -> caller style/part_styles
```

Unknown style parts are errors. Hover, active, focus, disabled, and similar
high-frequency visual states are resolved by Rust pseudo-state styling rather
than Rhai state and rerender loops.

The standard size vocabulary is `xs`, `sm`, `md` (default), and `lg`. The
standard semantic variants are `primary`, `secondary`, `danger`, `warning`,
`success`, `ghost`, and `outline`; individual components may support a declared
subset. Desktop cursor conventions apply: interactive controls retain the
default cursor, while link-like elements may use a pointer cursor.

### 6.2 Themes

Theme definitions are copied Rhai source owned by the application. Rust defines
and validates the semantic token contract. A `ThemeFamily` contains named light
or dark variants and identifies the variants used by system-following mode.

Theme selection supports application defaults, window overrides, and local
`ThemeScope` subtree overrides. Runtime theme changes and theme-file hot reloads
invalidate relevant UI without recompiling component ASTs or discarding state.

The first product line validates the contract with:

- project-owned Default Light and Default Dark;
- two suitable Tokyo Night variants;
- Catppuccin Latte and Catppuccin Mocha.

Components reference only semantic tokens, never palette-specific names.

### 6.3 Locale

Components do not hard-code user-facing text. Props provide application content;
shared internal phrases resolve through a locale bundle. English and Simplified
Chinese bundles validate the initial system. Logical layout direction is
preserved in the architecture, while full RTL visual certification is deferred
until M2.

### 6.4 Icons and images

The runtime defines asset, icon, and image provider protocols without depending
on a full icon library. The official registry includes only the minimal SVG set
required by its components. Complete icon collections may be distributed as
optional source packs.

Rhai addresses assets by semantic `AssetId` or receives a controlled
`ImageHandle` from a capability. Components never read arbitrary filesystem
paths or URLs. Rust owns decoding, caching, scaling, fallback, and cancellation.

## 7. Rendering mechanisms

### 7.1 Overlay system

A host-scoped Rust `OverlayManager` is shared by every script view in one
interaction domain (normally one GPUI window) and by Dropdown, Popover, Tooltip,
Dialog, Menu, and Toast. It owns portal rendering, layer order, anchored
placement, boundary avoidance, dismiss stacks, modal focus, focus restoration,
tooltip delay, and toast queues. Rhai components define content, style, and
policy rather than rebuilding those mechanisms.

`Menu` refers to in-window context and dropdown menus. Native macOS menu-bar
integration is outside the initial component contract and may later consume the
same action registry.

### 7.2 Animation

Rhai declares transitions or springs; Rust performs interpolation and frame
scheduling. Rhai is never invoked once per animation frame. Reduced-motion
preferences shorten or remove nonessential animation automatically.

### 7.3 Responsive layout and large lists

Normal resizing relies on GPUI layout. Structural adaptation uses discrete,
configurable viewport classes (`compact`, `regular`, and `wide`) rather than
continuous script-side pixel calculations.

M1 includes a one-dimensional Rust `VirtualList` behavior primitive for large
collections. It supports stable item keys and equal or predictable row heights.
Data tables and arbitrary two-dimensional virtualization remain out of scope.

## 8. Accessibility and desktop behavior

Accessibility and full keyboard operation are part of the definition of done,
not deferred polish. Components must provide available GPUI semantics for role,
label, value, checked state, descriptions, and invalid state.

At minimum:

- every interactive component works without a pointer;
- focus is visible and traversal order is deterministic;
- disabled behavior is consistent;
- Dialog traps focus and restores it when closed;
- Dropdown and Menu support arrows, Enter, Escape, and type-ahead;
- animations respect reduced-motion settings;
- missing upstream GPUI accessibility capabilities are documented explicitly.

`FormField` is included as a lightweight composite component to consistently
associate labels, controls, descriptions, required state, and validation errors.
It is not a full form framework.

The default visual language is modern, restrained, and desktop-first. It follows
macOS interaction conventions without attempting to imitate AppKit controls
pixel for pixel.

## 9. Loading, hot reload, and diagnostics

Scripts and their dependency graph are compiled and validated at startup. The
runtime caches ASTs; a GPUI rerender does not reread or reparse files.

Development mode watches scripts, themes, locales, and assets. It recompiles
only affected modules. Failed compilation preserves the last successfully
rendered UI and its state. Successful reload preserves keyed state when schemas
remain compatible, resets only incompatible component instances, and removes
unreachable state after a successful render.

Compilation errors retain the last good tree. Render errors are caught by the
nearest `ErrorBoundary`, or by a themeable root error page. Event errors preserve
the current UI. Diagnostics include the Rhai stack, source location, component
path, and key. Rust custom primitive panics must not unwind through the runtime
boundary.

M1 includes a development-only inspector showing the component/`UiNode` tree,
keys, source positions, props, computed style, theme tokens, state, recent
events, rerender causes, and script timings.

## 10. Distribution and tooling

The CLI contains a versioned snapshot of the official component registry so
installation is offline and reproducible. Third-party remote registries are a
future extension.

The user project contains:

```text
ui/                    # editable application and copied source
.gpui-rhai/
  manifest.toml        # installed versions and compatibility
  baselines/           # pristine upstream sources for three-way updates
```

Both the manifest and baselines should be committed to version control.
`update` compares the installed baseline, user source, and new registry source;
it never silently overwrites modifications.

Runtime SemVer, component source versions, and `runtime_api` compatibility are
tracked separately. The CLI enforces compatibility before installation or
update. During `0.x`, a breaking runtime API change increments the minor version.

Required initial commands are:

- `gpui-rhai init` — scaffold a minimal host and UI without overwriting files;
- `gpui-rhai add` — resolve and copy source plus transitive dependencies;
- `gpui-rhai check` — validate modules, compatibility, schemas, and themes;
- `gpui-rhai dev` — run with hot reload and structured diagnostics;
- `gpui-rhai diff` / `update` — inspect and merge upstream source changes.

All mutating CLI operations support a dry run.

Development uses `FileScriptSource`. Production uses a CLI-generated
`EmbeddedScriptSource` that includes scripts, themes, locales, and assets in the
binary. Both use the same resolver and execution semantics.

## 11. Component product line

### Primitives and small components

- Button
- Input
- Checkbox
- Radio / RadioGroup
- Switch
- Label
- Tag
- Avatar
- Icon
- Divider
- Progress
- Skeleton

### Composite components

- Dropdown
- Popover
- Tooltip
- Dialog
- Accordion
- Collapsible
- Tabs
- Menu
- Toast
- FormField

### Example applications

- `hello_world` — the smallest Button and Label end-to-end path;
- `settings_panel` — Switch, Radio, Dropdown, Divider, and Accordion;
- `dashboard_layout` — Tabs, Tag, Avatar, Progress, and Popover;
- `form_showcase` — Input, FormField, Checkbox, Radio, Switch, Dialog, and Toast.

Examples use the same `ui/` source layout and runtime API as downstream
applications and remain independently runnable through Cargo.

## 12. Delivery milestones

- **M0, foundation:** runtime skeleton, `UiNode`, schemas, state/events, theme
  foundations, hot reload, custom primitives, CLI skeleton, Button, Label, and
  `hello_world`.
- **M1, risk validation:** input/IME, accessibility and actions, overlays,
  animation, async tasks/subscriptions, assets, virtual list, multiple themes,
  devtools, Input, Icon, Divider, Popover, Dropdown, Dialog, and
  `settings_panel`.
- **M2, product-line completion:** remaining components and examples, locale and
  RTL validation, broader integration polish, complete documentation, and
  release hardening.

Each milestone is independently runnable and may be published as a preview.
The full component list is a v0.x product-line goal, not a reason to delay early
vertical-slice releases.

## 13. Quality gates

A component is complete only when it has:

1. Rhai logic tests for defaults, schema, variants, and state behavior;
2. stable `UiNode` snapshot tests;
3. representative macOS visual regression screenshots across themes and states;
4. interaction tests for keyboard, focus, events, and reload behavior;
5. accessibility assertions for composite controls;
6. source-header documentation and a working example.

The project initially supports macOS as its fully tested platform. Windows and
Linux remain architectural compatibility targets without complete behavior
guarantees during v0.x.

The workspace uses stable Rust, declares an MSRV compatible with the pinned
GPUI version, and tests both MSRV and current stable. Nightly Rust is not
required.

## 14. Explicit non-goals

The initial project does not:

- execute hostile or untrusted Rhai as a strong security sandbox;
- expose filesystem, network, process, or raw GPUI access implicitly;
- depend on `gpui-component`;
- provide a Web-style URL router;
- implement Dock layouts, rich-text editing, data tables, or 2D virtualization;
- provide a complete form framework;
- provide a complete custom LSP or formatter;
- provide complete multi-window APIs before M2;
- treat the in-window Menu component as the native macOS menu bar;
- attempt simultaneous full-quality certification on all desktop platforms.

Scripts are trusted application source, but the runtime still restricts the
module resolver, disables arbitrary eval/path loading, applies execution and data
limits, and exposes only declared capabilities. Documentation calls this a
restricted capability environment rather than a hostile-code sandbox.

## 15. Repository and licensing

The repository is a Cargo workspace:

```text
gpui-rhai/
├── crates/
│   ├── gpui-rhai/
│   └── gpui-rhai-cli/
├── registry/
│   ├── components/
│   ├── themes/
│   ├── locales/
│   └── assets/
├── examples/
├── docs/
└── tests/
```

Focused infrastructure crates such as serialization, CLI parsing, file
watching, and error-reporting libraries are permitted after license and cost
review. The dependency prohibition targets alternative UI frameworks, script
engines, and heavyweight runtimes—not every third-party Rust utility.

Project Rust and Rhai source is licensed under `MIT OR Apache-2.0`. Theme,
palette, icon, and adapted-source attribution is tracked separately and copied
with assets where required.

English is the normative language for source comments, schemas, errors, and
project documentation. Simplified Chinese documentation may be maintained as a
supplementary translation.
