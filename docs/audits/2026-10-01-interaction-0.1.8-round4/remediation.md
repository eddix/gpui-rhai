# Fourth-round remediation

Baseline: `f6e936a5`. This remediation extends PR #94; publication and tagging
remain deferred pending independent acceptance.

## Shared contracts

| Finding | Implemented boundary | Verification |
| --- | --- | --- |
| Full virtual lifecycle | The complete target retains old component state/recipes/resources, merges new rows and cleans removed rows, including prune-only commits. | State/incarnation probes and product effect/timer/signal/ref/rollback regression. |
| Delayed row callbacks | Seed and delayed realization use the same structural context and real callback owner. | Actual unseeded Action13 updates the root count; product regression also checks row31. |
| Canvas drawable geometry | GPUI's measured inner Canvas rect and pixel-snapped offsets are stored separately from the outer border box. Paint, inverse hit and pivot share it. | CPU paint recorder keeps the decorated pivot at `(131,51)` at 0° and 90°; padded target click selects `a`. |
| Raw row dependencies | State/store/document/environment/collection reads retain an executable caller owner. | Missing→register and existing→replace update virtual rows as well as ordinary roots. |
| Destination scrolling | Accepting targets resolve real retained ancestry. Row-gap continuation retains only a validated, unoccluded container; stationary checks run after presentation on one bounded native tick. | Foreground accepting/rejecting overlays keep background scroll at zero; historical edge and stationary scrolling still work. |
| Selection tolerance | Area and cross predicates use relative edge vectors and translation-invariant error bounds. | Equal 5×5px targets and 20×20px selections agree at zoom 10k and 1M; product metamorphic tests vary origin and scale. |
| Wheel completion | Debounce and explicit completion both invalidate their generation and release auxiliary ownership. | The first Escape reaches the application after either completion. |
| Cancelled async delivery | Activation eligibility precedes callback-owner validation. | Cancelled work is silently discarded; explicitly invoking an unmounted callback still fails. |

Issue #93 is also addressed: root and formal-component defaults report the
state field path and recursive tagged `UiValue` contract. USER_GUIDE contains
map/array/null examples; accepted encoding is unchanged.

## Evidence handling and results

Original reports, focused probes and baseline evidence remain unchanged.
`remediation-evidence/baseline/` holds a separate replay of the historical
**69 GPUI and 6 async** probes; their assertions were unchanged and all pass.
Fourth-round native groups pass **5 runtime, 6 resize, 6 drag and 3 selection**
checks. The original async characterization intentionally expects the former
error; `remediation-evidence/run-async.py` copies it into a disposable workspace
with the correct cancellation assertion, retaining the explicit-stale-call
control. All five async cases pass. Schema acceptance remains consistent in
**12,870** differential cases.

Product verification includes 476 core all-feature tests, 446 default-feature
tests, the complete workspace integration/example/CLI matrix, 133 independent
native tests, structural performance, strict Clippy and release gates. Remote
CI status is recorded on PR #94. No crate or tag is published by this work.
