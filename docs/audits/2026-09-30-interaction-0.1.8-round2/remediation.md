# 0.1.8 round 2 remediation

The findings against merge commit `d77b4b49` were accepted as release blockers.
No package, tag or GitHub release was created. The implementation was corrected
at shared ownership and commit boundaries rather than by weakening the supplied
probes.

## Structural changes

1. **Native lifecycle access**: primitive suspend/resume/compensation and
   coordinator cleanup share a reentrancy-safe direct signal read/write lease.
2. **Render contract invalidation**: node, native, timer, task and subscription
   paths mark gesture invalidation after the same successful script render
   commit. Virtual realization is tracked separately as presentation work.
3. **Proposal completion**: Table pointer/keyboard/autofit use one cleanup
   contract. Native key policies return `PrimitiveSemanticProposal`; Automation
   executes it synchronously and observes the actual callback result.
4. **Virtual drag ownership**: a collection/index/source snapshot renews the
   logical source lease while the member remains valid. GPUI `ListState` joins
   native edge auto-scroll and wakes the virtual realization pump directly.
5. **Geometry and constraints**: RangeSlider solves the feasible pair jointly;
   Rotatable reads content geometry and includes its size in the presentation
   token; degenerate marquee polygons and single-selection keyboard output obey
   one invariant; PanZoom Escape owns wheel cancellation.
6. **Data/tooling boundaries**: missing NativeCollection reads are retained as
   exact dependencies; Tree receives its nearest enabled parent from the Rust
   projection; Rhai delivery quotas run before schema issue construction; CLI
   reports static-only validation when a real Host init is required.

## Regression result

- original round 1 GPUI probes: **25/25**;
- round 2 runtime core: **4/4**;
- round 2 resize/range: **6/6**;
- round 2 transform/selection: **6/6**;
- round 2 drag/sortable: **5/5**;
- round 2 tooling native: **2/2**;
- round 2 Rhai async: **6/6**;
- collection/live-register and complex Tree probes produce the expected ready
  state and bounded operation result.

The unchanged workspace matrix also passes: 467 core all-feature tests, 133
independent native/keyboard tests, strict Clippy, rustdoc with warnings denied,
the structural performance suite, 69 visual baselines, release artifact audit,
crate packaging and the complete macOS release smoke matrix. Table regression
coverage additionally distinguishes controlled rejection (clear the preview)
from an unobserved native resize (retain the committed local width).

The original round 2 report and evidence remain unchanged. This file records
the subsequent remediation and does not itself authorize publishing.
