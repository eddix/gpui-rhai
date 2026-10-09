# History

Audits, plans, research and ledgers from earlier rounds are kept in git history,
not in the current tree. Their lasting requirements were merged into the
maintained documents: the design specification in [design/](design/), the
decision records in [adr/](adr/), the [release checklist](release-checklist.md)
and the tests. Each link below opens a record as it was at
`decb1b4`, the last commit that contained it. They are written mostly in
Chinese and record what was true for that round, not the current API.

## Audits

| Date | Record | Subject |
|---|---|---|
| 2026-09-08 | [2026-09-08](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-09-08) | Second audit: the runtime re-derived from its invariants |
| 2026-09-20 | [2026-09-20/component-freeze-0.1.2](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-09-20/component-freeze-0.1.2) | 0.1.2 component foundation freeze |
| 2026-09-20 | [2026-09-20-motion-runtime-2](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-09-20-motion-runtime-2) | Motion Runtime 2 design and implementation review |
| 2026-09-21 | [2026-09-21-issue-fixes-6373e4b3](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-09-21-issue-fixes-6373e4b3) | Acceptance of the issue fixes in 6373e4b3, SVG follow-up |
| 2026-09-23 | [2026-09-23-charts-e09b5b2f](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-09-23-charts-e09b5b2f) | Chart Runtime reviews and contract remediation |
| 2026-09-24 | [2026-09-24-ui-theme-e5d26877](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-09-24-ui-theme-e5d26877) | UI and theme consistency remediation |
| 2026-09-25 | [2026-09-25-gpui-pre-upgrade](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-09-25-gpui-pre-upgrade) | gpui-pre migration candidate (0.1.6) |
| 2026-09-26 | [2026-09-26-gallery](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-09-26-gallery) | 0.1.7 Gallery acceptance application |
| 2026-09-27 | [2026-09-27-gallery-interactions](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-09-27-gallery-interactions) | Gallery interaction review remediation |
| 2026-09-28 | [2026-09-28-interaction-foundation](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-09-28-interaction-foundation) | 0.1.8 audit of duplicated interaction foundations |
| 2026-09-30 | [2026-09-30-interaction-0.1.8](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-09-30-interaction-0.1.8) | 0.1.8 issue triage and interaction adversarial matrix |
| 2026-09-30 | [2026-09-30-interaction-0.1.8-round2](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-09-30-interaction-0.1.8-round2) | 0.1.8 remediation, round 2 |
| 2026-09-30 | [2026-09-30-interaction-0.1.8-round3](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-09-30-interaction-0.1.8-round3) | 0.1.8 remediation, round 3 |
| 2026-10-01 | [2026-10-01-dogfooding-pr-triage](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-10-01-dogfooding-pr-triage) | Dogfooding PR triage; issues #91 and #93 |
| 2026-10-01 | [2026-10-01-interaction-0.1.8-round4](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-10-01-interaction-0.1.8-round4) | 0.1.8 remediation, round 4 |
| 2026-10-02 | [2026-10-02-interaction-0.1.8-round5](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-10-02-interaction-0.1.8-round5) | 0.1.8 remediation, round 5 |
| 2026-10-02 | [2026-10-02-interaction-0.1.8-round6](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-10-02-interaction-0.1.8-round6) | 0.1.8 round 6 handoff |
| 2026-10-02 | [2026-10-02-interaction-0.1.8-round7](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-10-02-interaction-0.1.8-round7) | 0.1.8 round 7 independent acceptance |
| 2026-10-02 | [2026-10-02-release-convergence](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-10-02-release-convergence) | 0.1.8 release convergence checkpoints |
| 2026-10-02 | [2026-10-02-window-table-adapters](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-10-02-window-table-adapters) | Embedded windows, Table boundaries and node style storage |
| 2026-10-04 | [2026-10-04-release-candidate-review](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-10-04-release-candidate-review) | 0.1.8 R8 convergence candidate review |
| 2026-10-04 | [2026-10-04-table-local-fixes](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-10-04-table-local-fixes) | R8-T1 and R8-T2 local fixes, independent re-verification |
| 2026-10-07 | [2026-10-07-0.2.0-candidate](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/audits/2026-10-07-0.2.0-candidate) | 0.2.0 candidate real-window checks (the CGEvent harness is now [scripts/macos-real-window](../scripts/macos-real-window/)) |

## Plans and research

| Date | Record | Subject |
|---|---|---|
| 2026-09-24 | [2026-09-24-gpui-backend-selection](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/research/2026-09-24-gpui-backend-selection) | Choosing the GPUI line to migrate to: upstream, gpui-pre or a kit |
| 2026-09-24 | [2026-09-24-gpui-omarchy](https://github.com/eddix/gpui-rhai/tree/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/research/2026-09-24-gpui-omarchy) | What gpui-rhai can borrow from gpui-omarchy |
| 2026-10-02 | [2026-10-02-0.1.8-release-convergence.zh-CN.md](https://github.com/eddix/gpui-rhai/blob/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/plans/2026-10-02-0.1.8-release-convergence.zh-CN.md) | 0.1.8 convergence and release plan |
| 2026-10-04 | [2026-10-04-design-system-0.2.zh-CN.md](https://github.com/eddix/gpui-rhai/blob/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/plans/2026-10-04-design-system-0.2.zh-CN.md) | 0.2.0 design system plan and the PR #111 closing consensus |

## Ledgers and old gates

- [IMPLEMENTATION_PLAN.md](https://github.com/eddix/gpui-rhai/blob/decb1b40db335eb4b447efe32b55dd89c1bfc754/IMPLEMENTATION_PLAN.md): the Core Runtime v2
  implementation plan, the delivery order for 0.1.6 through 0.1.8.
- [core-runtime-v2-audit.md](https://github.com/eddix/gpui-rhai/blob/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/core-runtime-v2-audit.md): the
  Core Runtime v2 evidence ledger for those releases.
- [release-checklist.md](https://github.com/eddix/gpui-rhai/blob/decb1b40db335eb4b447efe32b55dd89c1bfc754/docs/release-checklist.md): the release
  checklist with the gates specific to 0.1.0 through 0.1.8.
