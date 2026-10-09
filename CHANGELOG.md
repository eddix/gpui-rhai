# Changelog

All notable runtime, CLI, and registry changes are documented here. Version
0.1.0 establishes the first public compatibility baseline; later public API,
component schema, manifest, locale, and generated-source changes follow
semantic versioning from this release.

## 0.2.0 - Unreleased

Runtime API 3. A design system for productivity tools, separated from a neutral
runtime. See the [0.2.0 release notes](docs/releases/0.2.0.md) for migration
and [docs/design/](docs/design/) for the specification.

- **Breaking:** the runtime no longer hard-codes design tokens. Themes are an
  open token registry; components declare the tokens (`tokens`) and environment
  values (`environment`) they read, and preparation validates the active theme
  against the mounted components. Typography roles and lengths are open names;
  lengths may vary with environment values set by `.env(#{ ... })` and resolved
  during native rendering; `mix`/`alpha`/`readable` color expressions evaluate
  after the token base, palette and Host layers merge.
- **Breaking:** the design language moves to the token base
  `registry/tokens.rhai` (`TOKEN_BASE_SOURCE`, `ui/tokens.rhai`), which official
  components require. Bundled palettes contain colors only; Default Light/Dark
  use the paper, ink and cobalt palette.
- **Breaking:** all 62 components are rebuilt on the 0.2 contracts. Button
  defaults to `secondary`; Badge shows its square lamp by default and gains
  `emphasis`; Card `elevated` becomes `variant: "outline"`; Combobox
  `row_height`/`trigger_height` and Command `row_height` are removed in favor of
  `size`, density and `metrics.*`. Button, IconButton, Menu, ContextMenu,
  Command and Tooltip bind `action`; Tag gains a `facet` segment; Kbd gains
  keycaps; Tabs can be a view switcher.
- Adds `layouts/` (Stack, Inline, Toolbar, Region) and `patterns/` (Section,
  DescriptionList, Stat, FormLayout, InlineState, DataView, ListDetail, AppShell
  with F6 / Shift+F6 regions).
- Adds the composition audit (`ScriptViewHandle::composition_audit`,
  `composition_audit_with`) and application profiles (`ui/profile.rhai`,
  `EmbeddedScriptView::profile_source`, `gpui-rhai init --profile
  productivity`); `gpui-rhai check` reports literal geometry under a profile.
  Nodes opt out of a rule with `.audit_allow([...])`.
- Runtime hooks: `group_focus` and `focus_within` styles; focus-styled nodes
  honor their tab-stop policy; cross-axis overlay `align`; modifier-qualified
  key handler names (`shift+f6`); deferred first reveal for fill-height virtual
  lists; `justify_start`/`justify_end` follow the flex direction, so they mirror
  in RTL rows.
- Table with a selection mode is keyboard operable (arrows, Home/End, Enter);
  a column with no value of its own shows only its adornments.
- **Breaking:** `gpui-rhai gallery` opens the new Gallery, a Rhai application on
  AppShell (`registry/gallery/`, `GALLERY_SOURCES_BY_ID`) with 83 pages, four
  keyboard scenes and a live audit count; every page passes the productivity
  audit in both densities. `--page`, `--density`, `--theme`, `--locale` select
  the launch; `--story` opens a development story in a standalone window;
  `--list` prefixes lines with `page`/`story`. The Rust Gallery shell
  (`gallery_app`) and `GALLERY_NAVIGATION_SOURCE`/`GALLERY_SOURCE_VIEW_SOURCE`
  are removed; the `test-support` feature remains and gates nothing.
- Gallery baselines are rendered offscreen with the real macOS renderer
  (`scripts/capture-macos-gallery-baselines.sh`); 19 captures replace the 31
  story-shell captures.
- Adds `examples/byod_treemap`, an application with its own palette and no token
  base or official components.
- Every interactive component is one tab stop: overlay trigger wrappers, key
  routing containers and tooltip panels no longer take focus (Select, Combobox,
  Popover, Tooltip, DatePicker, ToggleGroup and closed Dialogs had phantom stops).
- RTL: definite-width children of stretching columns sit on the start edge; the
  audit compares right edges in RTL views.
- Performance: a stretched child of a column renders with a definite width, so
  Taffy no longer lays out nested columns twice per level (Gallery frames went
  from 20-52 ms to about 2 ms, pixel-identical); `ScriptViewHandle::committed_revision`
  lets Hosts skip work on repaint-only frames, and the composition audit caches
  system font names (the Gallery no longer runs a 100 ms audit per frame).
  `UiNode` is a shared copy-on-write handle, which halves Rhai render time;
  `UiNode::kind_tag` and `UiNode::element_ref` are no longer `const`.
- The five visual examples are rewritten with the 0.2 layouts and patterns and
  pass the productivity audit; their 38 baselines are recaptured offscreen
  (`scripts/capture-macos-example-baselines.sh`).
- **Breaking:** a failed callback, delivery or render stays reported until a
  successful reload, the banner's Dismiss button, or the new
  `ScriptViewHandle::clear_error`. A later successful transaction no longer
  clears it, so a failure that rolled back one event is readable instead of
  flashing away. Automation commands still report only their own failures.
- **Breaking:** Resizable's `resize` payload is `{x,y,width,height}`, exactly the
  next `rect`; the `handle` field is removed. Storing the payload as the rect
  used to fail validation and roll the drag back.
- Gallery: PanZoom shows a plane larger than the viewport, the transform and a
  Reset view button; Rotatable takes the drag anywhere in its area.
- Corners are an environment axis (`corners`: `square` default, `subtle`,
  `round`) and radius roles depend on it; a new `radius.xs` role keeps
  Checkbox and Kbd boxes. `round` makes controls and markers capsules and
  square controls circles; panels stay at 8px. Square output is unchanged.
  Style gains logical `radius_start` / `radius_end`, and joined groups
  (ToggleGroup, ButtonGroup, a Tag facet) round only their outer corners.
  **Breaking** for copied token bases: components that read radius tokens
  declare the `corners` environment value.
- `.window_drag_area()` and TitleBar/AppShell `window_drag` let a Rhai title
  bar replace the platform one: its background moves the window, a double press
  runs the platform title-bar action. Drag areas are inert unless the Host
  allows them (`ScriptViewConfig::window_drag_areas`,
  `ScriptApplication::window_drag_areas`). On macOS the Gallery uses it.
- `ctx.theme_variants()` lists the loaded theme variants. The Gallery title bar
  picks any of them and launches with any bundled theme slug; it also switches
  the corner style.
- `gpui-rhai check` warns (`builtin-shadow`) about entry functions that take
  over a built-in: `fn f(a, b)` captures method calls `x.f(a, b)` and direct
  calls `f(a, b)`, also inside imported components
  (`RuntimeEngine::lint_shadowed_builtins`, `ShadowedBuiltin`). The Gallery's
  locale callback, two stories and two examples were renamed.
- Review fixes before release: a stretched child with a horizontal margin lays
  out like an explicit stretch; hot-reloaded tokens and scripts are checked
  against component token requirements like preparation and commit together
  or not at all; Table keyboard navigation works for NativeCollection data
  (`native_table_neighbors`) and Table rows are no longer tab stops; key
  handlers match `key:shift+f6` in the capture phase too; a `by_env` table
  must cover every declared combination (**breaking** for incomplete tables).
- A style a signal selects (`.signal_style`) and motion or signal sizes are
  merged before the stretch rules read a node's style, so margins, alignment
  and position from them stretch like the same static style, and a
  signal-sized child of an RTL column sits on the start edge.
- Slot content (virtual list rows) no longer tracks its focus owner's handle,
  which made the last realized row receive a Table's keys.
- A caller-written `shortcut` (`cmd-p`) shows the platform legend (`⌘P`) on
  Button, Menu, Command and Tooltip, like an action's; text that is already a
  legend stays. `key_shortcut(text)` exposes the formatting. Button draws the
  legend in the label voice of Menu and Command's inline Kbd.
- A sticky group header that the next one pushes out no longer paints over or
  takes clicks from the column header (virtual lists clip to their viewport).
- The composition audit covers content behind `error_boundary`, Layer content
  and realized virtual rows. It no longer compares controls across unrelated
  panes, checks a Badge's own gap, or limits content led by a view switcher;
  `.heading_elsewhere()` marks a container whose heading is drawn outside.
- Toolbar gains a `fill` slot: one field that takes the width between the
  start and end groups, at least `metrics.label_column` wide, so the bar wraps
  only when that minimum does not fit. DataView passes `toolbar.fill` and
  `toolbar.size` through and takes Region's `inset`.
- Region gains `scroll` (the body scrolls, header and footer stay; otherwise
  the body clips) and `external_title` (the title is drawn elsewhere;
  exclusive with `title`).
- Table group headers use the label voice (mono, uppercase, muted), like the
  column headers.
- Overlay panels that hold focus themselves show the 2px focus frame over their
  edge (part `focus_frame`; the hairline turns the focus color and the frame
  adds the inner pixel): Dialog, Sheet and Popover, which showed nothing, and
  Menu, DatePicker and Combobox, which showed a 1px border. The view owns an
  overlay's panel focus handle, so `group_focus` styles in the panel content
  see the panel holding focus.
- Identifiers (#95): a Table column's `typography` (`"code"`) sets its cells'
  role, for array rows and NativeCollection data; spans take
  `typography(role)` (the role's family and weight, the paragraph's size) and
  `background(color)` for inline code. The monospace face is the token base's
  `code` role; Hosts and `ui/tokens.rhai` change it, palettes do not.
- **Breaking:** components rendered inside a virtual item get an
  `Item[<item key>]` path segment
  (`.../VirtualCollection[rows]/Item[r5]/Badge[...]`). Keys inside an item only
  need to be unique within it, and a keyless component in a newly realized row
  can no longer take a retained row's path (#110). Component state inside
  virtual rows resets once on upgrade.
- **Breaking:** an overlay's identity is (declaring component instance, key)
  (#109). Two instances of one component with a Select of the same key each
  open and close their own; before, every dismissal went to the instance
  rendered last. `parent_overlay` names the nearest enclosing overlay with that
  key. `ScriptViewHost::overlay_placement` returns
  `Result<Option<PlacementResult>, OverlayLookupError>` and reports an
  ambiguous key; `overlay_placement_in(view_id, instance_path, key)` names the
  instance. Layers are scoped the same way, and a shared-layout group belongs
  to the instance that named it. `OverlayNodeSpec` and `LayerNodeSpec` gain
  `owner`, `VirtualCollectionNodeSpec` gains `inherited_motion_scope`.
- Capability handlers can see what a call responds to (#91):
  `CapabilityHandler::call_with(&InvocationContext, method, input)` (default:
  `call`) receives the `InvocationOrigin` (user input with the event name,
  automation, timer, task completion, subscription, effect, lifecycle), the
  view and the calling component. Timers and task completions carry the root
  origin that started them; `is_user_input()` follows it.
  `CapabilityRegistry::call_with` passes a context; `AsyncDelivery` gains
  `origin` (**breaking** for code that builds deliveries). An action
  dispatched or an event emitted during an invocation runs with that
  invocation's origin, also when it runs after a task completion, timer,
  subscription or effect has returned: `ActionInvocation` and `PendingEvent`
  gain `origin` (**breaking** for code that builds them).
- Table `on_context_request` (#89): a right press on a cell selects its row
  (unless the selection holds it) and emits `#{ key, column, anchor, source }`
  with the pointer as anchor; Shift+F10 or the menu key emits it for the
  current row with the row's bounds. The caller shows a Menu at `anchor`.
  `ctx.virtual_item_bounds(collection_key, index)` reads a laid-out virtual
  item's window bounds at event time.
- PanZoom and Rotatable keep showing a pan, zoom or turn after release: the
  content no longer jumps back to its old place until some other input redraws
  the window. They keep the proposed transform until the Host answers, return
  to the source when it rejects the proposal, and ask for a frame after the
  signal writes they make while a frame is drawn. Keyboard steps show at once.
- Fields take their own side padding: Input, Textarea, the Select, Combobox
  and DatePicker triggers and InputGroup affixes use the new
  `metrics.field_pad` instead of the button padding `metrics.control_pad`.
  Their text starts half the control height plus 1px from the outer edge
  (comfortable 11 / 13 / 15 / 17, compact 9 / 11 / 13 / 15 by size), which
  clears a capsule's ends in the round corner style, the same in every corner
  style. Textarea's vertical padding becomes `metrics.multiline_pad`. Button,
  Tabs and ToggleGroup keep `metrics.control_pad`.
- Input gains `appearance: "embedded"` for the search or filter line that
  heads a panel: no frame or well, a 1px `border` line under it (`danger`
  while invalid), text on `metrics.inset` like the rows below; its caret shows
  focus. Command, CommandDialog and a searchable Combobox panel use it: the
  search spans the panel at the top instead of a framed field inside it.
  Command's search is the `lg` size (36 / 32) with an `xs` gap above the rows;
  CommandDialog's panel has no padding by default (`dialog_part_styles.panel`
  still overrides it) and a visible title sits on `metrics.inset`.
- Resize handles take a decorative grip (#83): SplitPane `handle` and Resizable
  `grips` (one node per handle) sit in a `grip` box centred on the handle and
  painted above the panes or content; pressing it anywhere it is visible and
  not covered, overhang included, starts the drag, while keyboard steps, the
  separator role and the tab stop stay on the native handle. A node with an
  element ref takes part in hit testing in its place in paint order, so
  clipping and `.occlude()` content in front apply to it. The grip takes `grip_hover`, `grip_drag`,
  `grip_focus` or `grip_disabled` from a native state signal. `line: false`
  drops the native line and `line_inset` sets its end inset. The general
  `.signal_style(signal, #{ state: style() })` picks a style variant by a
  string signal without a Rhai render; primitives can ask whether their host
  node has focus (`PrimitiveContext::is_focused`).
- InlineState draws `detail` in the stale and refreshing states too, under the
  line (#122); Alert and InlineState text wraps in a narrow container instead
  of running past it (#123).
- Toolbar groups shrink below their content, so a view switcher or field in
  `filters` scrolls or shrinks inside the bar; an empty end group is left out
  and no longer adds a second line (#124).
- Section keeps at least a label column for its heading beside the actions and
  wraps the actions below the heading when both do not fit (#125).
- FormLayout no longer reports `spacing-not-nested` on its own submit row or on
  vertical fields with a description (#126).
- SplitPane's panes are separate regions for the composition audit: no
  text-edge, row-height or mixed-type comparison crosses them (#114).
- The composition audit leaves absolutely positioned nodes (an overlay
  panel's focus frame, an indicator) out of `text-edge-misaligned`: they are
  out of flow and do not stack with the text under them.
- Region's scrolling body draws ScrollArea's overlay scrollbar, so it can be
  seen and dragged; `scrollbar` (`auto`, `always`, `hidden`) and the
  `scrollbar_*` parts style it like ScrollArea's (#121).
- Collapsible and Accordion items fit their content when `content_height` is
  omitted, and take a Length (`theme_length("metrics.row") * 3`) as well as
  pixels, so the panel follows density; the measured height is read after
  layout (#127). A panel mounted open no longer grows in; toggles still
  animate from the current height.
- Dialogs are as wide as they ask (420px by default): the 90% cap resolved
  against the panel's own wrapper, so every dialog was 10% narrower. The
  overlay now caps a dialog at 90% of the window.
- CommandDialog can replace a hand-built palette (#120): `title_visible:
  false` hides the title (Dialog gains the same prop; the label still names
  the dialog), `on_escape` receives Escape before the dialog closes so a
  palette with levels can go back one, `width` sets the panel width, and
  `command_part_styles` / `dialog_part_styles` reach the inner Command and
  Dialog parts.
- Adds `List` (#119): keyed, virtualized rows with a status badge, title,
  secondary text (beside or below the title) and meta, on the row inset, with
  Table's selection, keyboard and context-request model; `badge_width` lines
  the titles up when badges differ. DataView gains `bleed` (default true) for
  bodies that keep the inset.
- Runtime: `.group_hover(style)` paints a node while its nearest hover-styled
  ancestor is hovered; `.translate_wheel()` lets a one-axis scroll container
  take the other wheel axis; `ctx.scroll_into_view` reveals a direct child of
  the scroll container minimally (it aligned it to the start), waits a frame
  for a node mounted in the same transaction (it reported an error) and drops
  a request whose node was unmounted. The sortable item primitive gains
  `take_focus` and `tap` (`on_tap`). Rhai builds at `opt-level = 1` in dev
  profiles, giving deep views stack headroom in debug test threads.
- Adds `TabBar`: document tabs that belong to the panel under them. The
  selected tab takes `tabbar.active` (a new token, the panel's surface) and
  covers the bar's line, square in every corner style, with no start line on
  the bar's edge; tabs scroll sideways, also under a vertical wheel, and
  the selected one stays revealed; closable and dirty tabs, middle-press close,
  context requests, drag and Alt+Arrow reordering, an all-tabs menu and
  `start` / `end` slots. The keys move a cursor and Enter selects.
- Table's header is unfilled like its rows, so on a raised layer (Dialog,
  Sheet, Popover) it no longer shows a `surface` band (#129).

## 0.1.8 - 2026-10-04

- Qualifies queued window commands by their original mount, independently of
  which View drains them. Revoked sources and replaced targets cannot operate
  newer windows; pending Open/Focus/Close retains one reservation identity.
- Resolves Table columns once against the native viewport and shares the same
  widths, horizontal extent and offset across the header, virtual rows, and
  loading/empty states. Native resize requests its own follow-up frame without
  re-running the Rhai root. ElementRef readers now remain subscribed through
  appearance, rebind and removal, with contribution-owned cleanup.
- Preserves Table logical scroll distance across LTR/RTL direction changes and
  smaller ranges; replays caller track/column markers after component-only
  updates while retaining last-good presentation on failure.
- Adds trusted Host operation-limit and expression-depth configuration without
  raising defaults or data limits. ExecutionTiming reports the configured
  round limit and cumulative consumption separately from each span's cost.
- Unifies named/punctuation node-key validation and native capture/target/bubble
  routing. Adds lightweight resolved-theme metadata and initializes System
  appearance before mounted init/effects in primary and secondary windows.
- Ingests native appearance notifications through a View-owned weak subscription,
  including idle windows and token-only UI; suspended views retain the new mode
  for resume and disposed views detach. Embedded manifest generation is warning
  free for empty as well as nonempty capability lists.
- Adds a bounded foreground asset-publication bridge example and interactive
  Gallery wide/loading/empty/projected-empty Table cases. See the
  [0.1.8 release notes](docs/releases/0.1.8.md) for Rust/key breaking changes,
  integration steps, verified boundaries and explicit 0.1.9 deferrals.

- Adds one Host-domain Interaction Runtime for gesture ownership, pointer
  capture, cancellation, typed application drag sessions, drop-target priority,
  atomic native preview patches, edge auto-scroll and lifecycle cleanup.
  Resizable, SplitPane, Table column resize, Slider, Scrollbar and Chart now
  share that foundation instead of maintaining parallel move/up state machines.
- Adds editable source components Draggable, DragSource, DropZone, Sortable,
  PanZoom, SelectionArea, Rotatable, RangeSlider and Tree. Pointer hot paths
  remain in Rust; Rhai receives one bounded controlled proposal per completed
  interaction. Every component has a Gallery story and native pointer/keyboard
  acceptance coverage.
- Adds finite reversible `Affine2D`/presented geometry shared by Canvas paint,
  hit testing, PanZoom, explicit-pivot rotation and Canvas-local selection.
  Scale/rotation native signals and Motion use one property-ownership and
  transform path.
- Sortable supports rich bounded items plus vertical Array/NativeCollection
  virtualization. Virtual renderer payloads expose stable neighboring keys;
  active drags pin one key and bounded halo while target edge auto-scroll moves
  realization. Reorder proposals never depend on stale numeric indices.
- Tree validates and flattens stable-key outlines in Rust, then reuses the
  public variable-height `virtual_collection`; keyboard activity, expansion,
  selection and disabled policy remain controlled by Tree rather than the list
  renderer.
- Fixes retained/delayed callback call-depth accumulation (#80) without raising
  Rhai recursion limits. Adds typed primitive prop access, `PrimitiveContext`,
  deferred semantic proposals, atomic signal batches, shared stable grouping,
  and Canvas-local inverse coordinate reads.
- Hardens the Interaction Runtime after adversarial combination testing:
  per-View pointer-capture routing, retained mount identity, GPUI-native
  clip/occlusion/paint-order drop resolution, stale-source cancellation,
  component-scoped Sortable channels, virtual keyboard focus/Home/End, and
  constant-time virtual drag pin routing.
- Makes controlled replacement authoritative during active gestures, restores
  rejected Table width previews, keeps RangeSlider gaps on the global step
  grid, preserves caller node identity in SplitPane/Draggable slots, and routes
  Resizable Automation through the same native key policy as real input.
- Validates Tree structure/depth in near-linear time independent of expansion;
  fixes null/hidden/disabled active and reveal behavior. Rotatable initializes
  non-center pivots, SelectionArea uses exact affine polygons, and invalid
  Canvas transform compositions safely reject instead of panicking.
- Preflights recursive Rhai delivery limits for tasks/subscriptions and bounds
  async error payloads. Adds post-mount Host `NativeCollection` registration
  and separates CLI static/View validation from unavailable Host capability
  lifecycle execution.
- Completes the cross-entry consistency pass: native lifecycle signal access,
  node/native/async render invalidation, Table pointer/keyboard/autofit cleanup,
  synchronous native Automation results, virtual source leases plus ListState
  edge scrolling, content-sized rotation pivots, joint RangeSlider solving,
  Rust-indexed Tree ancestors, missing-collection dependencies, and early
  async quota rejection now share their domain boundaries.
- Finishes the third adversarial consistency pass: canonical retained primitive
  identity, Host/window-owned cancellation, destination-owned virtual scrolling,
  real virtual-row callback provenance, effect activation leases, per-delivery
  commit invalidation, current Canvas geometry, bounded negative dependencies,
  parent-first Tree ancestry and fail-fast online schema diagnostics.
- Closes the virtual node/resource lifecycle contract for retained, new and
  removed rows; preserves delayed callback/read owners; measures actual Canvas
  drawable geometry; follows real scroll ancestry through unoccluded row gaps;
  releases completed wheel ownership and silently discards cancelled effects.
  Documents recursive state defaults and adds field-path diagnostics (#93).
- Adds `workbench/interaction-lab`, a connected acceptance application that
  composes the complete direct-manipulation stack and executes a real
  cross-component DragSource → DropZone workflow. Runtime API remains **2**;
  DockLayout, OS/cross-window drag, arbitrary GPUI subtree scaling, 3D and
  platform certification beyond the existing matrix remain out of scope.

## 0.1.7 - 2026-09-28

- Adds the formal `gpui-rhai gallery` acceptance application: registry-driven
  source-backed Component/Motion/Chart stories, exact live Rhai source,
  category/search/case navigation, per-story Reset and bounded retention,
  theme/locale/Motion hot switching, and responsive viewport presets.
- Adds the connected Operations Workbench reference application over
  deterministic Rust NativeCollection/NativeChartData/subscription fixtures,
  including normal/cancel/failure, loading, empty, streaming, theme override,
  configuration diff, and 1,000-row workflows.
- Table adds semantic cell adornments plus controlled Rust-side search, paging,
  sort and grouping projection. Chart adds wheel policy, title fixes,
  observable invalid diagnostics/semantics, and layout regressions. Tabs and
  HostSlot gain content-fill and Chinese IME acceptance coverage.
- Adds trusted Host theme, locale, and Motion mutation methods that preserve
  mounted state. Gallery applies them to cached parent/dependent view groups.
- Removes the duplicate Component/Motion/Chart Gallery Cargo examples. Internal
  performance/smoke examples move to explicit classified targets while
  retaining their binary names; `examples/README.md` is the canonical index.
- Native and release gates reuse the same stories, mount/draw every case,
  hot-switch all bundled themes, verify subscription cleanup, launch Gallery
  from an empty cwd, and benchmark the 100k streaming story at zero Rhai
  operations per data revision.

## 0.1.6 - 2026-09-28

- The Rust backend moves from official `gpui 0.2.2` to the exact
  `gpui-pre 0.3.7` core/platform family and raises MSRV to Rust 1.95. Rust
  Hosts must use the same package identity; `gpui-rhai::gpui` and
  `gpui-rhai::gpui_platform` are the canonical re-exports. Rhai Runtime API 2
  and existing script/component contracts remain unchanged.
- A committed semantic frame is now shared by automation and GPUI/AccessKit.
  Roles, localized names, state, values/ranges, form properties, collection
  metadata and bounded virtual content reach the native platform tree.
- Native Click/Focus follows GPUI dispatch. Input, Textarea, Slider and Chart
  expose bounded AX operations that re-enter existing controlled callbacks;
  disabled, read-only and stale targets reject actions.
- Native plain text uses a valued AccessKit `Label`; VoiceOver does not receive
  value-less `TextRun` wrappers that can abort traversal.
- `gpui-rhai check` diagnoses old or mixed GPUI package families without
  rewriting application manifests. Standalone apps use
  `gpui_platform::application()` internally, while embedded and Host-owned
  trees preserve one window accessibility/focus hierarchy.
- Component Gallery is now a live workbench instead of a mostly static visual
  specimen. Enabled fields, choices, navigation, data controls, commands and
  overlays keep controlled example state, while its status bar reports the
  latest action and cumulative interaction count.
- The final GPUI family contains upstream wrapped-line hit-test fix #64672.
  macOS native accessibility/input and Linux X11/Wayland window smoke remain
  release gates; application dogfooding feeds the 0.1.8 follow-up line.

## 0.1.5 - 2026-09-24

- Button and Badge retain distinct action/status density while consuming the
  shared spacing/radius/typography scale. Tabs replaces the selected rail with
  one adaptive continuous track and inset selection surface, adds content/equal
  layouts plus icon headers and style parts, and moves the shared indicator
  through Motion Runtime when given a stable key.
- Rust Hosts can apply one validated `ThemeTokenOverrides` preference layer to
  every file-backed or embedded theme. Partial color, spacing, radius,
  typography, motion, and namespaced-token overrides survive theme switching
  and file-theme hot reload without rewriting third-party Rhai source.
- Complete theme snapshots preserve arbitrary validated namespace colors in
  ordinary, virtual, overlay and native paths. Official components now map
  visual density to `xxs/xs/sm/md/lg`; `tabs.foreground` guarantees readable
  enabled tabs and `table.selection` owns a stable selected-row surface.
- The optional `charts` feature adds one native composable Chart Runtime with
  Cartesian2D, Polar and Host-owned Geo2D regions; 15 built-in series; typed
  columnar and streaming `NativeChartData`; native transforms/downsampling;
  hover, tooltip, crosshair, zoom/pan, brush, selection, keyboard and linked
  charts; annotations; theme/motion/locale integration; Host transform,
  formatter, projection and custom-series extensions; and Host-only SVG/PNG
  export. The CLI installs `charts/chart` and enables the feature, while the
  mounted `chart_gallery` exercises every series plus a 100,000-row data path.
- Chart preparation keeps transformed semantic data separate from drawing,
  compiles one shared scale per named axis, computes signed stack and Polar
  domains correctly, uses typed mark roles and structural datum identity,
  acknowledges controlled viewport proposals, exposes active/selected native
  semantics, preserves text in PNG export, and rejects duplicate Host extension
  registration without replacing the installed implementation.
- Chart viewport state is coordinate-typed and versioned across Host control,
  link groups, gestures, resize and suspend/resume. Native and exported labels
  share Host typography, including family/fallback, size, line height and
  weight.
- Prepared/presented frame identity, lifecycle compensation, streaming
  coalescing and activity time now close atomically; irrecoverable mixed native
  state disposes the affected view rather than masquerading as suspended.

## 0.1.4 - 2026-09-22

- Inline and asset-backed SVG `currentColor` now inherits the nearest effective
  semantic text color as an SVG cascade default while preserving document-local
  `color` overrides, including inside deferred virtual rows. The complete
  adapter preserves fixed colors, gradients, semantic alpha, system-font text,
  generic families, and fallback. Public async decode performs parsing,
  rasterization, encoding, and image preparation off the foreground thread;
  cold inline/tint variants use the GPUI background executor and a shared
  256-entry/128-MiB LRU with observable hit/miss/byte/eviction counters.
- IconButton adds a controlled `selected` state. Transparent and secondary
  variants use the semantic accent foreground without adding a filled
  container, while the node exposes pressed accessibility semantics.
- Generic Overlay adds an opt-in trigger-width policy. Combobox and Select use
  it so relative/flex widths follow their realized parent allocation and the
  deferred panel remains aligned with the trigger across resize.
- Rust Hosts can register opaque named elements, entities, or independently
  mounted script views through `HostSlotRegistry`. Rhai can place and size the
  keyed box, while events and lifecycle authority remain on the Host side.
  Slotted script views retain their own `ScriptViewHost` frame boundary even
  when the surrounding shell belongs to another Host.
- Inherited `motion_group` membership is now a replayable presentation
  mutation. Component-local incremental rerenders, nested component
  replacement, and later virtual item realization preserve the group context.

## 0.1.3 - 2026-09-21

- Runtime API 2 replaces the node animation prototype with one generic Motion
  engine shared by Rhai and Rust. Strict transition, spring, keyframe, inertia,
  replay-key, reduced-motion, quality, budget, Inspector, and deterministic
  clock contracts replace `.animate`, `transition`, `spring`, and
  `loop_transition` without compatibility aliases.
- Explicit timelines support delay, sequence, parallel, stagger,
  repeat/reverse, typed scoped handles, play/pause/resume/restart/seek, and
  post-frame complete/cancel delivery. Compatible hot reload preserves active
  timeline progress and refreshes generation-bound callbacks.
- Native hover/press/focus, in-view, viewport, and scroll progress avoid
  frame-time Rhai. Pointer payloads expose movement and velocity for bounded
  inertia. Enter/replay, paint-only exit ghosts, opt-in committed-geometry
  layout motion, transparent MotionGroup, and same-domain shared-layout IDs
  participate in transactional lifecycle and budgets.
- RichText supports grapheme-safe native span opacity motion. Canvas adds
  native 2D scale/skew/rotation, path trim, arc-length follow, and strict
  compatible-topology morphing. General arbitrary-subtree transform, arbitrary
  shaders, and 3D remain explicit future capabilities.
- Themes now carry duration/easing/spring/distance/stagger motion roles with
  tracked hot switching. Host effect primitives declare platforms, lifecycle,
  instance/cost budgets, reduced-motion support, quality tiers, and receive the
  resolved policy through `PrimitiveTheme`.
- Ten optional public-substrate components ship under `motion/*`, including
  TextReveal, NumberTicker, Marquee, Shimmer, BorderBeam, Orbit, Particles,
  AnimatedTabs, ReorderList, and SharedLayoutCards. The CLI installs them into
  `ui/motion`, and the separate `motion_gallery` example exercises the pack.
- Motion reconciliation compiles timeline targets and property ownership
  atomically. Handles are bound to runtime/view/component incarnation and
  generation; play is idempotent, pause/seek are position-complete, control
  commands recheck policy and budget, and direct/timeline physics share one
  finite sampler with velocity-preserving retargets.
- Display-frame sampling is scoped per presentation domain; terminal event
  batches are frozen by the frame that samples them and delivered after that
  frame commits, in independent transactions.
  Suspended views, virtual rows, exit ghosts, layout/trigger motion, and slot
  rendering share the Host clock, policy, and stable identity rules.
- Canvas affine, morph, trim, clip and non-scaling stroke motion use one
  presented geometry for painting and hit testing. Axis-aligned clipped paths
  explicitly reject affine motion, and unsupported exit-ghost subtrees fail
  reconciliation instead of silently degrading.
- Mounted motion identity is now `presentation domain + retained NodeId`, while
  headless Rust reconciliation uses collision-free encoded path segments.
  Root remount, hot reload and resume-reload transfer authority explicitly;
  live reconciliation cannot reclaim independently playing exit scenes.
- Unconstrained and non-bouncing inertia use closed-form exponential sampling;
  bouncing constraints use analytical collision segments instead of replaying
  a 240 Hz history on every frame.
- Compatible timelines now refresh resolved retained-target bindings without
  resetting playback position. Presentation teardown uses exact domain
  boundaries and clears suspension tombstones; candidate budgets are checked
  once against the complete final plan. Inertia safety caps preserve the actual
  cap-time sample unless explicit snap points request an attachment.
- `motion_gallery` now mounts in the native GPUI harness and exposes live
  timeline controls, controlled tabs, keyed reorder, and shared-layout
  selection rather than only verifying source preparation.

- Mounted script failures now expose one atomic human/structured error record.
  `ScriptViewHandle::last_diagnostic()` preserves the Rhai error kind, bounded
  token, source position and stack, typed execution timing and operation budget,
  deepest failing component path/key, and only that component's pre-redacted
  state snapshot across lifecycle, callback, native-triggered rerender, async
  delivery, virtual realization, resume, and hot-reload paths. A successful
  transaction clears both representations together; ordinary error display
  never serializes the diagnostic payload implicitly.

## 0.1.2 - 2026-09-20

- `Style::flex_grow_weight` and Rhai's overloaded `style().flex_grow(weight)`
  now expose GPUI's positive numeric grow factor. Table `flex` column values are
  weights over remaining width instead of falling through to raw pixels;
  fixed/percent columns do not shrink, and Array/NativeCollection headers and
  cells share the same width contract.
- Table can opt into native divider resizing with per-column eligibility and
  min/max bounds. Pointer movement updates an optional fixed-width NativeSignal
  without running Rhai; release emits one persistable `column_resize` event,
  while logical arrow keys provide an accessible 8px step. Double-click
  auto-fit measures the header and bounded realized rows through GPUI text
  layout for both Array and NativeCollection inputs.
- Bundled themes now use a readable dense-desktop typography scale: caption
  11/16, body-small 12/16, body 13/18, subtitle 14/20, title 16/22, heading
  18/24, display 24/32, and display-large 28/36px.
- The 51-component source catalog is frozen as the 0.1.2 foundation. Input,
  Textarea, Combobox, Select, DatePicker, Pagination, Menu, ContextMenu,
  Popover, Tooltip, and Progress now require an explicit accessible `label`
  instead of deriving names from placeholders or empty fallbacks. This is an
  intentional pre-1.0 source-schema break with no compatibility shim.
- Unlabeled Icon instances now expose presentation semantics, Divider exposes
  orientation, Progress exposes its `0..max` range and a bounded width contract,
  and Toast accepts an application-localizable `dismiss_label` without making
  locale configuration a rendering requirement.
- IconButton owns square icon-only action geometry, Button owns typed
  prefix/suffix icon-and-text composition, inline SVG uses the node box
  consistently, and the CLI/editor metadata, Theme Studio, Component Gallery,
  examples, tests, and guide all consume the same frozen schemas.

## 0.1.1 - 2026-09-08

- Runtime identity now distinguishes globally unique program candidates,
  logical component paths, mount incarnations, and per-view presentation
  domains. Multi-window geometry/capture no longer collide, and callbacks,
  timers, and NativeSignal handles from an unmounted same-key component cannot
  target its replacement.
- Runtime transactions now checkpoint the lifecycle tree as well as Engine and
  UI state. Task, subscription, and image-decode cancellation is provisional
  until the outer commit, while rollback restores old delivery authority and
  cancels newly created work. Independent async deliveries commit separately;
  a failed message no longer discards its neighbors.
- Rhai operation accounting now aggregates nested evaluators under semantics
  version 2 and gives delayed callbacks, retained component rerenders, and
  virtual item renderers fresh absolute baselines without resetting their
  enclosing execution budget. The default Engine rejects filesystem imports,
  import extraction walks an unoptimized Rhai AST (including nested template
  interpolation and dead branches), and the pinned Rhai 1.26 constant-container
  assignment panic is rejected during compilation.
- Durable values reject non-finite floats and enforce recursive item/depth/byte
  limits across Rhai, Rust, serde, state, stores, capabilities, handlers, and
  signals. Sensitive diagnostic payloads are redacted before storage, and
  automation returns execution failures instead of reporting dispatch success.
- Component and Store snapshots use copy-on-write state, removing quadratic
  instance mounting and deep-copying of unrelated large stores. No-op viewport
  and unrelated virtual-request paths return before opening a transaction.
- Background tasks use a bounded shared worker pool with panic delivery,
  admission budgets, and optional cooperative cancellation. Lossless receiver
  subscriptions wait for capacity instead of treating backpressure as stream
  termination. Subscription close and capacity waits share one synchronization
  protocol; stale generations discard buffered values, wake producers, and are
  reclaimed immediately.
- Semantic event and action traces retain names and scopes but never their
  payload values. Sensitive state/store values and all event/action payloads are
  therefore absent from retained diagnostics and Inspector snapshots.
- `virtual_collection` is now strictly a presentation/scroll/reveal/sticky
  mechanism and no longer creates a second internal roving row or generic
  background highlight. Command and Combobox remain the sole owners of their
  enabled-item navigation and controlled active style, so Array and
  NativeCollection groups, disabled items, and `active_value` cannot diverge.
  The unused fixed-row `VirtualListSpec`, `VirtualListState`, and
  `VirtualListMetrics` Rust APIs are removed with that duplicate state model.
- CLI updates recompute the complete dependency/asset graph. Multi-file apply
  stages every write, uses a project lock, and rolls back ordinary commit
  failures. A single verification manifest now covers every shipped example,
  while PNG baseline checks fully validate chunks, CRCs, and decoded data.
- Formal components passed through node-valued props now replay their latest
  caller-owned component snapshot when the receiver rerenders instead of an
  initial stale `UiNode`. Receiver rerenders do not re-execute or restart the
  passed component, same-batch dirty owners are order-independent, and caller
  removal still performs normal resource cleanup. Component-owned output is
  separated from typed outer presentation mutations so child updates preserve
  slot styles/handlers/refs/signals/animations without accumulating them. Lazy
  weak-linked snapshots avoid ordinary component clone cost, synchronize later
  virtual realization, and restore their prior value on transaction rollback.
- First-render `element_bounds(ref)` now returns null while retaining a pending
  ref-identity dependency, self-heals after committed prepaint, and follows
  retained node replacement. Event callbacks can resolve another node in their
  formal component with `ctx.element_bounds("local_ref_key")` without retaining
  a Rhai custom value.
- Initial controlled virtual-list reveal no longer pre-scrolls past predecessors
  when the target already fits the configured estimated viewport. Long grouped
  Commands therefore retain their first group heading at the natural scroll
  origin while genuinely offscreen initial targets are still revealed.
- Hosts can retain inactive `ScriptViewHandle` tombstones through explicit
  `suspend`/`resume`. State, native entities, input/scroll/virtual measurements,
  and the last-good tree survive; effects quiesce, subscriptions cancel, timers
  and animations freeze, overlays/focus/capture release, bounded task results
  wait for one atomic resume, and suspended hot reload migrates transactionally.
- Long-lived subscriptions must now be started by declarative component effects,
  making their cleanup, suspension, replacement, reload, and disposal ownership
  explicit.
- CodeViewer and both DiffViewer panes now apply the document typography metrics
  to line-number gutters instead of inheriting a larger ambient font.
- Sticky virtual section headers no longer create a duplicate presentation layer
  at their natural position; sticky indices remain presentation metadata rather
  than an interaction-eligibility channel.
- Command group labels now recede with regular-weight muted typography, gain
  asymmetric breathing room, and visually own indented command rows. Array and
  NativeCollection projections use the same row metric.
- Raw nodes passed through formal component slots now retain the caller's
  callback provenance recursively, including optional, array, map, object, and
  union-shaped Node props.
- Subscription delivery now defaults to a bounded FIFO instead of silently
  overwriting earlier values. Rhai callers pass an explicit options map and may
  opt into latest-only coalescing; full queues return backpressure to Rust.
- Controlled virtual collections synchronously rebuild the retained viewport
  with current Rhai state before committing a rerender, eliminating blank
  frames when Command selection moves beyond the first realized window.
- Embedded Hosts can read and observe each `ScriptViewHandle`'s complete
  effective `ThemeSnapshot`, including colors, spacing, radii, typography,
  namespaced tokens, and system-appearance changes.
- Applications can define validated global formal-component part overrides in
  `ui/styles.rhai`. File, embedded, CLI check/embed, hot reload, rollback, and
  all official component sources use the same typed `ctx.component_style`
  cascade. Explicit instance overrides remain the final application-owned
  layer.

This release intentionally removes the old global
`component_style(props, part, base)` helper. Source components use
`ctx.component_style(part, base)` so retained and deferred renderers share the
same validated stylesheet snapshot.

## 0.1.0 - 2026-09-07

Initial implementation of the stable Rhai `UiNode` boundary, source-owned
component registry, themes/locales/assets, typed capabilities and custom
primitives, file and embedded applications, hot reload, developer inspector,
native input/overlay/animation/virtual-list mechanisms, RTL layout, and the
restricted multi-window lifecycle.

The pre-release Core Runtime v2 expansion adds typed Box/Text/Image/SVG/Canvas
atoms, retained keyed reconciliation, independent formal-component rerendering
and root-dirty bailout, exact store/environment dependencies, transactional
effects/timers, native signals, retained refs/geometry/focus/scroll, atomic
capture/target/bubble events, schema-checked native Rust handlers, Host-owned
trees, generic Overlay/Layer composition, retained automation, and bounded
native collections for large Tables.

Event handlers can consume the current handler node's untracked event-time
visual bounds through `ctx.event_target_bounds()`, raw pointer/wheel
`payload.target`, or `NativeEvent::target`. The application no longer needs a
resize/store channel merely to position native UI after a click.

Automation and mounted accessibility snapshots now project only nodes presented
in the latest GPUI frame. Open Overlay content is locatable, while retained
content from a closed Overlay can no longer receive invisible automation
dispatches.

Transparent formal-component rerenders promote to the nearest replaceable
ancestor instead of failing when a parent and child share one `UiNode` root.
Runtime-error banners are selectable monospace text and can be suppressed by a
Host that surfaces `ScriptViewHandle::last_error` itself.

The generic virtual collection accepts explicit sticky section-header indices,
retains exactly one active header, and pushes it off as the next section enters.
The source Table adds controlled `group_by`/`collapsed_groups` with counts,
toggle events, source parts, Array parity, and cached NativeCollection
sort/group/collapse projection.

Composite component callbacks now cross each formal boundary through declared
events: RadioGroup forwards both pointer and roving-key changes to its caller,
and Dialog explicitly emits `open_change`. Modal Overlay focus is enforced as a
per-frame invariant, including initially-open dialogs and one-off ancestor focus
requests from embedding Hosts, so Escape dismissal remains reachable.
Callback props now carry private module-scope provenance through formal
composition instead of inferring ownership from a function name; entry and
nested component modules may use identical private handler names without
redirecting an event.

The source registry now includes the complete official component specimen,
fifteen bundled theme variants, and Theme Studio for creating, importing,
editing, validating, previewing, and saving gpui-rhai themes.
The same sources ship as the versioned `gpui-rhai-registry` crate consumed by
the publishable CLI, so installed binaries never depend on files outside their
Cargo package.

The public-launch registry expands to 50 official components. Dropdown is
destructively renamed to the strictly controlled Combobox. New source-owned
families include Alert/AlertDialog, Badge, Card/GroupBox/Empty, Kbd/Spinner,
ButtonGroup/InputGroup, Toggle/ToggleGroup, Slider, ScrollArea, ContextMenu,
Sheet, Command/CommandDialog, and application-chrome TitleBar/StatusBar.
Generic Rust mechanisms provide native range
preview with commit-only Rhai delivery, event-coordinate overlay anchors,
logical viewport-edge sheets, themed draggable overlay scrollbars, Canvas
rotation, and NativeCollection fuzzy filtering/navigation. The interactive
`component_gallery` shares Theme Studio's exhaustive specimen and switches all
bundled themes live.

Themes now require eight semantic typography roles. Official source components
and native Input/Textarea shaping consume the same live role values, functional
glyphs use a centered 24px SVG asset system, and the default radius scale is
square. Only elements that are semantically circular retain explicit radii;
Switch tracks and thumbs are rectangular.

TitleBar now accepts string-or-node `title` and `subtitle` content with a
separate required accessibility `label`, allowing structured breadcrumbs to
retain their own styles and pointer/native handlers inside the standard chrome
layout.

The unlocked macOS visual pass tightened responsive specimen wrapping, gave
Theme Studio an independently scrollable token editor, made small Command lists
shrink to their result count, and added deterministic Gallery theme, locale,
category, overlay, compact, regular, and reduced-motion launch states.

Command and CommandDialog now use caller-owned `active_value` and expose a
deduplicated `active_change(string)` event for keyboard and pointer roving
previews while preserving `action` as the explicit confirmation boundary.
Their active item is a controlled virtual-collection reveal target: keyboard
and Host updates follow beyond the visible window, while unchanged targets no
longer reset manual wheel/trackpad scrolling. Generic virtual collections now
distinguish stable-key payload updates from structural changes.

The public registry now includes `CodeViewer` and `DiffViewer` over retained
native document primitives. Direct strings and exact-reader-invalidating
`NativeTextDocument` revisions share background syntax parsing, virtual rows,
continuous source selection, search, wrapping and line navigation. DiffViewer
adds neutral left/right unified and split projections, Unicode-grapheme
intraline emphasis, expandable context, focus-scoped hunk actions, independent
split-pane horizontal scrolling and explicit left-to-right unified-patch copy.
The built-in language pack includes Rhai, Rust, Go, common web/configuration and
scripting languages; every theme materializes syntax/search/diff semantic
colors. Host-configurable resource limits, standalone examples, Gallery/Theme
Studio specimens and a release benchmark cover both document input paths.

The initial theme contract includes explicit `on_accent`, `on_danger`,
`on_warning`, and `on_success` foregrounds. Reduced motion renders looping
indicators at a static midpoint rather than moving them out of view.

Dogfooding API reset before the first published release:

- `ScriptApp` → `FileScriptView`;
- `EmbeddedScriptApp` → `EmbeddedScriptView`;
- `PreparedScriptApp` → `PreparedScriptView`;
- `ScriptAppExtension` → `ScriptViewExtension`;
- `ScriptAppError` → `ScriptViewError`;
- standalone ownership moved to `ScriptApplication`;
- existing GPUI hosts mount isolated views through `ScriptViewHost` and
  `ScriptViewHandle`.

This destructive migration was completed before `0.1.0`; the published
baseline therefore contains no compatibility aliases.

There is no earlier GPUI Rhai release to migrate from.
