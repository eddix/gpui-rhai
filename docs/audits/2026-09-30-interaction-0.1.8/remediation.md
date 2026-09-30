# 0.1.8 interaction audit remediation

The findings against `0b9b887c` were accepted as release blockers. The
candidate was returned to Draft and remediated by shared runtime boundary,
instead of by changing the failing assertions or adding one-off component
conditions.

## Closed work packages

1. **Ownership and cancellation**: interaction owners include retained mount
   identity; one Host retains every View's capture route; suspend cancellation
   clears native signals through a lifecycle-only direct lane without
   re-entering the active Entity update; logical source unmount cancels
   application drag.
2. **Controlled replacement**: successful script rerenders invalidate older
   gestures; RangeSlider and Resizable track source/constraint changes; Table
   clears rejected width overrides; RangeSlider keeps one global step origin.
3. **Presented geometry**: pointer drops consume GPUI hitboxes, content masks
   and paint order; invalid affine composition safely rejects; rotation pivot
   initialization and four-corner marquee geometry share presented transforms.
4. **Collection identity and projection**: Sortable channels are scoped by the
   formal component instance; virtual drag sessions carry a source index and
   do not scan unrelated collections; Tree validates the complete graph with
   cached depth and uses one effective active/reveal policy; realized virtual
   items move without prefix copies (PR #88's root fix).
5. **Delivery and tooling boundaries**: task/subscription values are recursively
   preflighted against the Engine limits; error values are bounded; CLI check
   separates Host-dependent init from static/View validation; Hosts can
   explicitly register a new NativeCollection after mount.

## Independent regression result

The original GPUI probes now pass without weakening their expected behavior:

- runtime core: **4/4**;
- drag/drop/sortable: **5/5**;
- resize/range/composition: **10/10**;
- transform/selection: **6/6**.

The main all-target/all-feature workspace suite and strict Clippy also pass on
the remediated tree. The original reports remain unchanged as evidence of the
rejected candidate; this file records the subsequent implementation result.
