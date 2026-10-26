# ADR 0025: Host error reporting

## Status

Accepted, 2026-10-22, for #185. Extends the last-good-tree contract in
[embedding](../embedding.md#last-good-trees-and-host-visible-failures).

## Context

A mounted view's render, callback, delivery, effect and reload failures roll
back and are recorded as the view's failure: the built-in banner shows it, and
`ScriptViewHandle::last_error` / `last_diagnostic` return it until a successful
reload or `clear_error`. A Host learns of a failure only by polling those, and
nothing reaches stderr except `ScriptApplication`'s fatal startup errors.

An unattended run (CI, a smoke test, a timeout harness) therefore shows a blank
or error window and a clean console. disktree lost a long bisect to this (its
gap G5) and worked around it by polling `last_error` from its root render.

## Decision

- `ScriptViewConfig::on_error(observer)` calls the observer once for each new
  failure, with a `ScriptErrorReport`: the view ID, the message `last_error`
  returns and the `Diagnostic` `last_diagnostic` returns.
- The observer runs on the foreground after the frame that presents the
  failure, deferred past the view's render, so it may read the handle. Failures
  recorded before that frame report the latest, as `last_error` keeps only the
  latest. A frame or a successful event after it reports nothing; the next
  failure does.
- `ScriptViewConfig::mirror_errors_to_stderr(true)` prints each new failure as
  ``gpui-rhai: view `id`: message``. Embedded views default to off: a Host owns
  its process's output. `ScriptApplication` defaults to on, with
  `ScriptApplication::mirror_errors_to_stderr(false)` to opt out: a standalone
  application has no other channel, and a silent failure costs more than a
  line on stderr.
- Script windows a view opens inherit its reporting.
- No environment variable: the Host decides, in code, what its process prints.

### Recoverable presentation diagnostics (#205, 2026-10-26)

The observer is also the Host reporting channel for nonfatal native image
failures; it is not restricted to failed script transactions. Reports carry
`ScriptErrorDomain::Transaction` or `Presentation`. The latter includes a
`PresentationDiagnostic` (node ID/path, logical asset ID, bounded message),
leaves `last_error`/`last_diagnostic` unchanged, and does not roll back a tree.
Foreground delivery uses the same deferred observer and stderr policy.

The per-view/generation active-error set deduplicates unchanged failures and
is bounded to 256 entries with deterministic admission (not an eviction/repaint
loop). Messages and logical resource/path strings are bounded; image bytes and
script state are not recorded. Recovery removes an entry, unmount prunes it,
and reload/disposal invalidate old sinks. Native slot rendering may enqueue
after the root render; the existing foreground pump drains that queue too.

## Consequences

- Hosts stop polling. Transaction-domain reports describe the banner and
  `last_error`; presentation-domain reports describe recoverable placeholders.
- Detection is per frame. A failure is reported when the view next renders,
  which is also when its banner first appears; a view that never renders again
  reports nothing more, as its banner shows nothing more.
- The callback type is `Fn(&ScriptErrorReport, &mut App)`; it receives no
  window, because the report outlives the frame it came from.
