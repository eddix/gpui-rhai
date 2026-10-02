## Core convergence candidate

Preserves #101's authored Table changes and replaces partial #100/#103 policy
interfaces with the explicit native `PreparedScriptView::mount_window` adapter.
Ordinary embedded views remain restricted. No merge, publication or tag is
authorized by this PR's CI status.

### Final model

- Window commands capture enqueue origin and target reservation; a peer pump
  cannot provide new authority. Revocation, native closure, same-name targets
  and pending Open cancellation use exact mount/registration identity.
- Table header/body use one viewport-based fixed/percent/flex/override plan and
  real direct-child horizontal extent. Four states and RTL retain logical
  offset. Feasible styled constraints share that same plan.
- Logical ElementRef subscriptions persist across appearance/rebind/removal;
  current node observation migrates without losing direct/item contributions.
- Existing scoped batch deltas, final invocation ownership, Style COW and
  resource rollback remain intact.

### Independent checkpoint C

Product SHA: `99f77b1afce4c326536f1bbccf67df0890a43854`; documentation/evidence
head: `6327deace858492535723307ba95a5c15bd6f55f`.

The first independent check of `c6c1dc77` found a real prepaint scheduling P1.
It remains preserved as a failing report. The corrected candidate independently
passes the unchanged probe 2/2, fixed native matrix 113/113, core 494/494 and
virtual transaction/contribution matrix 5/5. Follow-up frames execute no Rhai
and settle to zero pending frame callbacks.

Reports and commands:
`docs/audits/2026-10-02-release-convergence/checkpoint-c-99f77b1a-review.zh-CN.md`.
This is approval of A–C only, not complete release acceptance. The prior full
native run of `c6c1dc77` was 186/186; it does not replace the final RC gate.

### Remaining release work

#96/#97/#99 are independent stacked integration packages, not additions hidden
in #104. The final combined candidate still needs the plan's complete toolchain,
platform, performance, packaging and manual gates. Five new Table screenshots
remain pending precise temporary floating permission; old candidates are not
post-fix baselines. disktree-rhai is excluded throughout.
