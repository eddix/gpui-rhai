# ADR 0017: Native read-only document surfaces

Status: Accepted

## Context

Applications need syntax-aware source display and neutral comparison of two
text documents. Implementing either as Rhai-created Text rows would materialize
large payloads as Dynamic, run callbacks near layout, prevent continuous
selection across virtual rows, and make split alignment and search state depend
on script throughput. Building a complete code editor first would add mutation,
IME, multi-cursor, undo, completion, diagnostic and LSP authority that neither
read-only product requires.

The runtime pins Rhai 1.26 without `sync`; Engine, Dynamic, FnPtr and stored call
contexts cannot enter background workers. Existing NativeCollection and formal
component boundaries establish that source components may wrap public native
mechanisms without receiving privileged private constructors.

## Decision

- Add `CodeViewer` and `DiffViewer` as source-owned official Rhai components over
  public lifecycle primitives.
- Accept direct strings and opaque immutable `NativeTextDocument` revisions.
  Named Host documents use exact component-reader invalidation; revision numbers
  for one identity must increase.
- Move only typed immutable text, syntax registries, limits and owned results to
  background jobs. Atomically commit the latest matching document/search/diff
  job and discard obsolete work.
- Keep selection, search, folding, hunk focus and scrolling in retained native
  entities. Expose focus-scoped actions and schema-checked location activation;
  never invoke Rhai per line, pointer move, search match or GPUI layout pass.
- Build wrap/fold visual-row projections in cancellable background jobs and
  atomically replace the last complete projection after resize or mode changes.
- Use neutral left/right diff status. Unified patch output gains direction only
  through an explicit copy command.
- Use virtual fixed-height visual rows. Soft wrapping expands logical lines into
  aligned Unicode-grapheme display segments before layout; split panes share
  vertical rows and keep independent horizontal offsets.
- Use `syntect` with the pure-Rust regex backend for syntax parsing and
  `similar` with Unicode/intraline support for deterministic hunk construction.
  Rhai, TypeScript/TSX, JSONC, TOML and Dockerfile grammars are project-owned;
  other launch languages use the bundled syntax set. Hosts may register more
  syntaxes before compile.
- Enforce Host-configurable byte, line, combined-diff, hunk and deadline limits.
  Syntax failure degrades to plain text; hard resource/diff failure is visible.
  Bound pathological intraline work by treating an oversized replaced line pair
  as one changed range.
- Materialize theme-specific `syntax.*`, `document.*` and `diff.*` colors from
  semantic anchors while preserving explicit namespace overrides.

## Consequences

CodeViewer remains useful without creating a hidden editor product. DiffViewer
reuses the immutable document model but owns a separate aligned-row projection.
A future CodeEditor can reuse syntax, line indexing, selection/search and theme
services while adding an independent mutable buffer and input command model.

Direct Rhai strings remain convenient but are still subject to Rhai's 1 MiB
string budget. NativeTextDocument is the normal path for large or frequently
replaced Host data. Background latency can exceed one frame without blocking
the UI; stale presentation is visibly marked and cannot be copied or activated.

## Evidence

- Core tests cover the complete built-in language list, Rhai/Go tokenization,
  Host syntax extension, CRLF/terminal newline behavior, Unicode intraline
  ranges, whitespace policy, context folds, tab/source offsets, revision
  monotonicity and configurable limits.
- Native GPUI tests mount both surfaces, exercise exact Host replacement,
  cross-line original-text copy, search input, hunk navigation and explicit
  unified-patch copy.
- Theme tests require every bundled variant to resolve the complete document
  palette and prove explicit values win.
- `scripts/benchmark.sh` records both Rhai input boundaries plus native syntax
  and diff work over a deterministic 20,000-line workload.
