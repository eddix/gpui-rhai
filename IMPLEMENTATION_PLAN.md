# GPUI Rhai complex-controls implementation plan

## 1. Purpose and authority

This plan delivers DatePicker, Select, Table, Pagination, and Textarea against
the normative specifications under `docs/components/`. It assumes the existing
runtime, registry, examples, and test infrastructure are complete foundations;
it does not repeat the historical M0-M2 bootstrap plan.

Work is dependency-ordered and gate-driven. There are no calendar estimates.
A phase is complete only after its automated exit gate passes and its public
documentation agrees with the implementation.

`INTENT.md` remains the product authority. The five component specifications
are the API and behavior authority for this expansion. When an implementation
detail conflicts with a speculative historical prompt, the prompt has no
standing.

**Status: completed on 2026-08-30.** The final gate passed 214 workspace tests,
16 native GPUI integration tests, strict Clippy and rustdoc, all release example
smokes plus Data Table selected/loading/empty states, release-artifact and
41-file visual-baseline audits, offline package verification, bounded Table
performance probes, and the unlocked macOS interaction matrix including real
`鼠须管` Textarea candidate commit and composition cancellation.

## 2. Fixed decisions

These decisions are not reopened during implementation without an explicit
design discussion:

1. The crate and component source versions remain `0.1.0` and
   `RUNTIME_API_VERSION` remains 1.
2. The repository is in dogfooding mode. Build the best final SDK shape without
   compatibility aliases, dual parsers, deprecated entry points, or application
   migration code. Compatibility work begins only after an explicit release
   signal from the maintainer.
3. Business values are controlled props. Keyed native entities own only
   interaction transients such as focus, selection, open panels, search text,
   visible month, scroll offsets, and text layout.
4. Rhai `view(ctx)` remains one complete transactional declaration. Rust must
   never call Rhai lazily from GPUI scroll or layout callbacks.
5. Component source owns composition, visual policy, structural size helpers,
   assets, and style parts. Rust owns input, focus, geometry, scrolling,
   Gregorian arithmetic, overlay coordination, and bounded native realization.
6. Themes own cross-component semantic tokens, not a token for every component
   metric. Rust contains no component palette or hidden visual-size constants.
7. Pinned GPUI public APIs are the only GPUI dependency. `gpui-component` is
   forbidden directly and through optional features.

## 3. Dependency graph

```text
Schema ───────────────┬───────────────┬─────────────────────────┐
Locale + Clock ───────┼──── DatePicker├──── Table formatting ───┤
Asset-backed Icon ────┼──── DatePicker├──── Pagination ─────────┤
Text editing core ────┴──── Textarea  │                         │
Choice-list core ─────────── Select ──┴──── Pagination           │
Virtual-list mechanics ──────────────── Table ──────────────────┘

All five components -> CLI registry -> form_showcase/data_table -> full gates
```

The implementation order is therefore:

1. public foundations;
2. Textarea;
3. Select;
4. DatePicker;
5. Table;
6. Pagination;
7. integration, documentation, and visual certification.

## 4. Phase 1: public foundations

### 4.1 Schema model

- Replace unit numeric schemas with integer/float/number schemas that may carry
  inclusive or exclusive bounds while preserving concise source syntax for
  unconstrained numbers.
- Add `one_of` with deterministic branch diagnostics.
- Add `Length` as a first-class component schema value.
- Add a schema that accepts exactly values convertible to `UiValue`; do not add
  an unrestricted `Dynamic`/`Any` escape hatch.
- Ensure default-value validation, sensitive-path redaction, serialization,
  editor metadata, component invocation, and error paths cover every new type.
- Add component-owned asset paths to `ComponentMetadata`, cross-check the Rhai
  header/export declarations, and feed them into CLI dependency installation.
- Migrate official component schemas directly to the best representation where
  existing hand validation can be removed.

**Exit gate:** schema unit tests, registry compilation, editor metadata
snapshots, CLI `check`, and every existing component contract pass with no
compatibility parser.

### 4.2 Gregorian date, locale, number, and Clock

- Add a small validated Gregorian date model with strict `YYYY-MM-DD` parsing,
  formatting, comparison, leap-year/month length, weekday, add-day, and
  add-month operations. It is an internal/native model; the Rhai ABI remains a
  string.
- Add required `CalendarMetadata` and `NumberMetadata` to `LocaleBundle` using
  the contract in `docs/locale-and-rtl.md`.
- Validate month/weekday cardinality, first weekday, digit set, separators,
  group sizes, and the supported date-pattern grammar at locale load time.
- Implement general locale date and decimal-number formatting in
  `LocaleManager`, then expose restricted read APIs through `UiContext`.
- Add a host-injectable `CalendarClock` abstraction. The default reports the
  host system-local Gregorian date; tests use a fixed implementation.
- Migrate English, Simplified Chinese, and Arabic registry bundles in one step.
- Prove app/window/subtree locale precedence and hot switching include the new
  metadata without changing AST or component/native state identity.

**Exit gate:** exhaustive Gregorian boundary tests, locale decode/fallback/
scope tests, format snapshots for all registry locales, fixed-Clock tests, and
live-switch regression tests pass.

### 4.3 Declarative assets and Icon

- Add an asset-backed image source to `UiNode` and its renderer path.
- Preload component-declared assets transactionally during file and embedded
  script preparation. Rendering may only resolve cache entries.
- Redesign Icon around `source` and optional `rtl_source`, each a schema union of
  `AssetId` and image handle.
- Keep dynamic lifecycle image loading for capability/application images.
- Add the minimal chevron, calendar, clear, disclosure, and sort SVGs required
  by the five components, using `currentColor` and explicit RTL pairs where
  directional.
- Update CLI asset dependency resolution, hot refresh, embedding, attribution,
  and failure diagnostics.

**Exit gate:** file/embedded equivalence, missing/malformed asset preparation
failure, no-render-I/O assertion, theme recoloring, hot refresh, RTL selection,
and dynamic-handle Icon tests pass.

### Phase 1 gate

- All existing examples and native keyboard tests still pass.
- The dependency graph remains free of `gpui-component`.
- Public docs and rustdoc describe only the new best API, with no old aliases.

## 5. Phase 2: shared editing core and Textarea

### 5.1 Refactor without behavior loss

- Extract the current buffer into a shared editing module responsible for:
  - UTF-8/UTF-16 range conversion;
  - extended-grapheme cursor boundaries and counts;
  - forward/reversed selection and marked ranges;
  - controlled-value reconciliation;
  - clipboard cut/copy/paste;
  - replacement capacity under `max_length`;
  - IME marked-text commit clamping.
- Keep separate Input and Textarea entities/elements. Do not branch one large
  renderer on a `multiline` boolean.
- Preserve Input's single-line paste normalization, Enter submit behavior,
  mouse hit testing, selection, IME, focus, and callback payloads through
  explicit regression tests.

### 5.2 Multiline native element

- Add Textarea actions/key context for visual-line Up/Down, logical line
  Home/End, selection variants, and newline insertion.
- Use GPUI wrapped-line layout for soft wrapping and cache the mapping between
  UTF-8 offsets, visual lines, x coordinates, and painted origins.
- Paint one caret and all selection rectangles intersecting the viewport.
- Implement mouse point-to-line/offset hit testing and drag selection across
  scrolled lines.
- Implement IME character/range bounds against the correct wrapped line.
- Retain preferred x for vertical movement and scroll the caret into view.
- Compute auto-grow from visual lines after width is known. Clamp to min/max;
  fixed rows bypass auto-grow. Avoid update/layout feedback loops.
- Implement one-shot mount autofocus and normal Tab/Shift-Tab traversal.

### 5.3 Rhai component

- Register a keyed native Textarea primitive with typed props/events.
- Author `registry/components/textarea.rhai`, formal schema, size helpers,
  error/disabled/read-only policy, count/limit display, semantic metadata, and
  all declared parts.
- Expose a general grapheme-count helper only if the source counter needs it;
  do not duplicate Unicode segmentation in Rhai.

**Phase 2 gate:** shared-core unit tests, all Input regressions, native multiline
keyboard/mouse/IME/clipboard tests, large-paste/delete/width-change probes,
Textarea component snapshots, and representative visual states pass.

## 6. Phase 3: choice-list core and Select

### 6.1 Private core extraction

- Refactor Dropdown option validation, filtering, type-ahead, focus movement,
  disabled skipping, virtualized visible rows, scroll-to-focus, and overlay
  lifecycle into a private choice-list core.
- Extend the core option model with optional group display identity while
  preserving stable option-value identity.
- Model selectable options and nonselectable group headers explicitly; do not
  fake headers as disabled options.
- Keep Dropdown's advanced array selection, multiple mode, custom slots, and
  controlled open/query APIs intact in its new best internal shape.

### 6.2 Select adapter and source

- Add a scalar optional-value Select spec/adapter over the core.
- Reset internal search query on every close and selection.
- Emit scalar string or `()` on select/clear; never expose a one-element array.
- Implement fixed trigger, form placeholder/error/disabled/size policy,
  localized labels, parts, and normalized combobox/listbox semantics.
- Author `registry/components/select.rhai` and document the Select/Dropdown/
  Popover/Menu taxonomy in both relevant component headers.

**Phase 3 gate:** Dropdown regression suite, Select schema/value/group/search/
clear tests, 10,000-option bounded realization, native keyboard/type-ahead/
search/dismiss/focus tests, RTL, accessibility assertions, and visuals pass.

## 7. Phase 4: DatePicker

### 7.1 Native model and element

- Add DatePicker spec, validation, outcome, and keyed transient state modules.
- Build a fixed 42-cell month model from locale first-weekday, controlled value,
  fixed/system Clock today, min/max, and presets.
- Reuse the existing host overlay coordinator for placement, outside click,
  Escape ordering, panel bounds, and focus restoration.
- Implement logical day/week/month/year keyboard movement, disabled-range
  suppression, month-nav disabled state, selection/clear commit, and transient
  visible-month reset rules.
- Synchronize external value/locale/theme changes without discarding native
  focus/identity incorrectly.

### 7.2 Source and formatting

- Author `registry/components/date_picker.rhai` with strict schemas, size
  helpers, asset IDs, part styles, semantic metadata, and optional presets.
- Use only LocaleManager calendar data and `format_date`; no English literals or
  locale-ID branches may appear in DatePicker source/native code.

**Phase 4 gate:** date/model unit tests, December/January and leap-year cases,
min/max/preset behavior, fixed-Clock locale snapshots, native keyboard/
dismiss/focus tests, hot locale/theme switching, and visual baselines pass.

## 8. Phase 5: Table

### 8.1 Data and column model

- Add Table row/cell, format, width-track, sort, selection, spec, validation,
  outcome, and metrics models.
- Require stable unique string identity from `row_key`.
- Validate tagged fixed/percent/flex widths, format descriptors, sortable keys,
  controlled sort, controlled selected keys, and scalar/default cell types.
- Resolve fixed and percent widths before weighted flex. Produce horizontal
  content width rather than squeezing declared tracks when space is insufficient.
- Format text/number/date through the common locale service.

### 8.2 Complete-tree custom renderers

- Bind column `cell_renderer` callbacks to their defining component/module and
  current script generation.
- Evaluate them during the Rhai Table construction/view transaction with the
  documented cell-context map.
- Store returned nodes only for custom columns and bind their event/component
  scopes normally.
- Never evaluate render callbacks from `uniform_list`, request-layout,
  prepaint, paint, or scroll handlers.
- Add metrics that report scalar rows, custom nodes built, and GPUI rows
  realized separately.

### 8.3 Native element

- Build a keyed Table Entity with vertical `UniformListScrollHandle`, shared
  horizontal ScrollHandle, active row identity, and focused header/body state.
- Render sticky header outside the vertical list but inside the shared
  horizontal coordinate space.
- Render fixed-height, single-line, clipped/truncated visible rows and native
  selection indicators styled through parts.
- Implement loading precedence, viewport-sized default Skeleton rows, custom
  loading/empty slots, striped state, and semantic colors.
- Implement sort cycling, single/multiple/select-all outcomes, row click, and
  row-level keyboard navigation with scroll-to-active behavior.
- Preserve row focus and selection identity through reorder/filter/page changes.

### 8.4 Rhai component

- Author `registry/components/table.rhai` with a dependency on Skeleton, schema,
  size/row metrics, all parts, slots, callbacks, and semantic metadata.
- Keep Pagination, data slicing, filtering, fetching, and sorting algorithms out
  of Table.

**Phase 5 gate:** model/schema tests, 10,000-row bounded scalar realization,
custom-renderer cost metrics, sticky/synchronized scroll native tests, sorting/
selection/row-key reconciliation, keyboard/focus/semantics, loading/empty/
striped/overflow/RTL visuals, and no-scroll-time-Rhai assertions pass.

## 9. Phase 6: Pagination

- Author a pure `registry/components/pagination.rhai` depending on Button, Icon,
  and Select. Do not add a native node.
- Implement one-based page validation, zero-items-as-1/1, total-page arithmetic,
  bounded boundary/sibling counts, and the deterministic ellipsis algorithm.
- Use curried internal `FnPtr` callbacks to bind next page/state to ordinary
  Button events.
- Emit one atomic `{ current_page, page_size }` payload; reset page to 1 on a
  page-size change.
- Add optional Select page-size control, locale-formatted neutral summary,
  directional assets, accessible labels, size helpers, parts, and metadata.

**Phase 6 gate:** exhaustive page-window transitions, invalid controlled state,
atomic callback, dependency resolution, keyboard/focus/RTL, schemas/snapshots,
and visual states pass. Runtime Rust source has no Pagination node/module.

## 10. Phase 7: registry, examples, and certification

### 10.1 Registry and CLI

- Bundle all five sources and assets in `BundledRegistry`.
- Encode the exact dependency graph and verify install ordering.
- Update init/check/metadata/diff/update/embed fixtures and editor snippets.
- Prove file-backed and embedded production sources behave identically.
- Keep crate/component version 0.1.0 and Runtime API 1; do not add migration
  adapters or application-side fixups.

### 10.2 Examples

- Reorganize `form_showcase` into bounded sections/tabs and add:
  - searchable/grouped/clearable Select;
  - fixed-Clock appointment DatePicker with min/max, presets, clear, and live
    English/Simplified Chinese switching;
  - feedback Textarea showing auto-grow, max length, count, error, and fixed-row
    variants.
- Add independent `data_table` using deterministic generated data and:
  - scalar text/number/date formatting;
  - a custom Tag/action cell;
  - fixed/percent/flex columns and horizontal overflow;
  - sticky header, stripes, loading, and empty states;
  - controlled sorting and single/multiple/select-all behavior;
  - Pagination with Select page size and localized summary.
- Add README files, release embedding, smoke registration, bundle building, and
  artifact audit coverage for `data_table`.

### 10.3 Automated and manual evidence

- Extend registry source tests and normalized `UiNode` snapshots.
- Extend `tests/native-keyboard` with all component interaction matrices.
- Add deterministic performance metrics and protect the existing VirtualList
  and Input numbers from regression.
- Capture the visual matrix in `docs/visual-testing.md` across representative
  themes, English/Simplified Chinese, and RTL.
- Complete unlocked macOS mouse, keyboard, focus, IME candidate-window,
  clipboard, overlay, horizontal scroll, and accessibility inspection.
- If an unlocked interactive session is unavailable, automated work may finish
  with one explicitly recorded pending manual gate; the components are not
  called fully certified until that gate is executed.

**Phase 7 gate:** CLI, all tests, examples, rustdoc, strict Clippy, release
smoke, artifact audit, performance probes, baseline audit, and manual matrix
pass or the sole manual gate is explicitly pending.

## 11. Required test matrix

### Pure logic

- schema branches/bounds/defaults and precise error paths;
- Gregorian date arithmetic and patterns;
- locale number/date output;
- grapheme/UTF-16 editing operations;
- choice-list filter/group/focus;
- Table widths/format/sort/selection;
- Pagination page windows.

### Runtime and source contracts

- official component header/export/schema agreement;
- callback module scope and hot-reload generation invalidation;
- controlled props versus native transient synchronization;
- theme/locale switch without AST or identity loss;
- file/embedded source and asset equivalence;
- CLI dependency and metadata snapshots.

### Native GPUI

- request-layout/prepaint/paint without panic;
- focus traversal/restoration and logical RTL keys;
- IME range/candidate bounds and clipboard;
- overlay placement/dismiss hierarchy;
- vertical/horizontal scroll and bounded realization;
- no Rhai execution from layout/scroll paths.

### Visual and performance

- all new parts/states in Default Light/Dark plus representative Tokyo Night
  and Catppuccin variants;
- fixed-Clock locale/RTL cases;
- large scalar Table and explicit custom-render cost;
- Textarea paste/delete/resize stability;
- no regression of existing 1,000-node, Input, Dropdown, and VirtualList probes.

## 12. Risk register

### Multiline text layout

Risk: wrapped-line geometry, UTF-16 IME ranges, and selection rectangles diverge.

Mitigation: one offset/visual-line model feeds paint, hit testing, movement, and
IME bounds; randomized round-trip tests cover Unicode boundaries.

### Auto-grow feedback

Risk: width-dependent wrapping and height invalidation oscillate.

Mitigation: measure from the current assigned content width, publish height only
when the clamped row count changes, and assert convergence under paste/delete/
resize probes.

### Table custom rendering

Risk: users interpret native row virtualization as lazy Rhai callback execution.

Mitigation: keep scroll-time callbacks forbidden, expose separate metrics, and
document/probe eager custom-node cost everywhere performance is claimed.

### Dual-axis Table scrolling

Risk: header and body widths/offsets drift or vertical realization receives the
wrong viewport.

Mitigation: one column-layout result and one horizontal handle feed both; native
tests inspect measured bounds and offsets before screenshot tests.

### Locale scope creep

Risk: a partial formatter grows into an incorrect Intl/CLDR implementation.

Mitigation: limit the contract to Gregorian presentation and decimal digits/
grouping. Currency, plural prose, and other calendars remain explicit non-goals.

### Asset render I/O

Risk: direct `AssetId` support accidentally loads files during paint.

Mitigation: preparation preloads declarations; renderer accepts only cached
resolution and treats a miss as an invariant diagnostic.

## 13. Completion definition

The expansion is complete only when:

1. all five component specifications match source schemas, native behavior,
   docs, CLI metadata, and examples;
2. every phase exit gate passes;
3. Table scalar rows are genuinely data-driven and bounded while custom-cell
   Rhai cost is explicit;
4. Textarea passes real multiline IME, selection, clipboard, scrolling, and
   auto-grow evidence without changing Input behavior;
5. locale/theme/Clock/asset hot changes preserve controlled values and native
   identity;
6. no `gpui-component` dependency, compatibility shim, hidden visual constant,
   scroll-time Rhai callback, or speculative excluded API remains;
7. the only permissible unfinished item is a clearly recorded manual macOS
   gate waiting for an unlocked interactive session.
