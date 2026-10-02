# ADR 0022 supplement: final ownership and commit model

- Status: Accepted implementation contract
- Target: 0.1.8
- Date: 2026-10-02

This supplement defines the final model, not the sequence of audit fixes.

## Owners and identities

| Boundary | Authoritative identity | Not an authority |
| --- | --- | --- |
| Native window command | Enqueue origin, source mount lease, target registration | The view pumping the shared queue |
| Formal component lifetime | Committed invocation graph and incarnation | Visible root-node markers |
| Virtual render batch | Scope-local candidate deltas, one final manifest | A target's old whole-state snapshot |
| Tracked read | Executable component owner plus contribution identity | A structural row namespace |
| Element geometry | Logical ElementRef subscription and its current NodeId binding | The first resolved NodeId forever |
| Table scroll | Viewport, resolved column extent, one native horizontal offset | Clipped descendant overflow |
| Canvas interaction | Actual drawable bounds and presented transforms | Decorated outer layout bounds |

## Commit and revocation

Window origin is captured before enqueue. A pump schedules work but supplies no
permission. Source revocation or target replacement cancels pending and deferred
work. Open owns one reservation; cancellation releases only that reservation.
Trusted Rust requests explicitly use Host origin. Initial script work receives
its mount qualification before execution; the subsequent native binding can
complete that same qualification, never replace it with another mount.

A virtual foreground batch merges only each target's scoped state delta. It
publishes one invocation manifest, then prunes incarnations, handlers and
resources against that final graph. Failure restores the prior state and read
contributions. Independent successful async deliveries stay committed even if
a later delivery fails; their queued commands retain their own origin.

Reads retain both their executable owner and their direct/item contribution.
Item pruning releases only that contribution, not another item or a direct root
read. Failed candidates restore the complete dependency snapshot. Logical ref
subscriptions persist while their contribution lives. Bind, rebind and unbind
invalidate the live owner and migrate node geometry observation; unchanged
bindings and geometry do not repeatedly notify.

Table column widths are resolved once from the viewport: fixed and percentage
widths first, flexible widths from remaining viewport space, bounded by each
column's feasible styled border-box minimum and accepted native overrides.
Declared min/max widths govern native resize/autofit proposals. The sum is the
content extent, not a new percentage base. Header and body consume that same
resolution and offset. Clipping, vertical virtualization, RTL reachability and
controlled proposal rollback remain separate contracts.

## Verification

The maintained contract index is [runtime-contract-tests.md](../runtime-contract-tests.md).
Tests must establish actual realization, advance foreground/background clocks
and inspect presented/native outcomes. Last-good rollback, stale origin,
cross-scope order, contribution pruning and geometry rebind use small fixed
fixtures; a green build alone does not establish these invariants.
