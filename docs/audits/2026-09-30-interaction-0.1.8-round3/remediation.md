# 0.1.8 round 3 remediation

The findings against `815bb82b` remain release blockers until this remediation
is merged and its complete local/remote gates pass. No package, tag or GitHub
release is authorized by this document.

## Structural resolution

1. Primitive lifecycle identity is derived once from the presented retained
   key and `NodeId`; constructor-local keys no longer compete with cleanup,
   focus, accessibility or Automation identity.
2. A Host/window interceptor owns Escape cancellation for pointer, application
   drag and PanZoom wheel sessions. Virtual source leases and keyboard focus are
   independent facts; window deactivation uses the same cancellation boundary.
3. Virtual auto-scroll resolves from accepting destination ancestry. Virtual
   collection structural scopes retain their caller callback owner instead of
   becoming a synthetic event/state owner.
4. Each successful async delivery records its committed contract invalidation
   before sibling errors are summarized. Effect activation is checked at the
   instant of delivery; already-drained stale work becomes a no-op.
5. Rotatable references the actual styled Canvas and cancels active work on a
   geometry revision. Range endpoints, Table rollback points and marquee
   degeneracy now share their declared value/coordinate domains.
6. Missing collection dependencies are render-only, name-validated and bounded.
   Tree enabled ancestry is computed parent-first, while online async schema
   validation stops after the first bounded issue.

## Focused verification

- round 3 resize/identity: **6/6**;
- round 3 transform/selection: **6/6**;
- round 3 drag/sort: **5/5**;
- round 3 reviewed runtime core: **4/4**;
- virtual-row formal callback: product regression test passes;
- stale effect delivery retains `new-query`; the old characterization assertion
  now fails because it expected the former incorrect overwrite;
- 10,000-node depth-256 disabled Tree case is approximately 18 ms in the audit
  harness rather than approximately 290 ms, with equivalent unsorted results;
- caught event-phase missing names retain zero negative dependencies.

The historical matrix passes **48/48 GPUI probes and 6/6 async probes** without
changing their assertions. The complete workspace passes 472 core all-feature
tests plus every integration/example/CLI/registry target; the independent
native suite passes **133/133**. Structural performance, strict Clippy, rustdoc
with warnings denied, 69 visual baselines, crate packaging, release artifact
audit and the complete macOS release-smoke matrix also pass. Remote Linux CI is
recorded after the remediation PR is pushed.

`run-regressions.py` refreshed the eleven `baseline/` runner outputs while
validating the remediation working tree; their probe-source hashes are
unchanged and `SHA256SUMS` records the refreshed evidence. The original report,
focused probe sources and original focused logs remain unchanged.
