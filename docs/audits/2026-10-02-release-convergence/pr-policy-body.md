## D1: trusted Host execution policy

Stacked on the independently verified #104 core. Preserves original #96 commit
and author, then corrects its global parser relaxation and weak workload probe.
Do not merge/publish/tag until the final combined RC is independently accepted.

- Default operation quota remains 1M; global/function parser defaults remain
  64/32 in both profiles. Host builders/RuntimeEngine configure them separately.
- Zero becomes one, not unlimited. One private policy applier covers File,
  Embedded and secondary factories; trusted extensions run afterwards.
- Nested/sibling execution remains cumulatively bounded; retained/delayed calls
  start fresh rounds. Reload candidates have independent policy storage.
- Timing-span `operations` is unchanged. New `operation_limit` and
  `round_operations` let diagnostics report the failed round's real quota and
  consumption, including partial/incremental components.
- Rust breaking cleanup: `MAX_SCRIPT_OPERATIONS` is replaced by the accurately
  named `DEFAULT_SCRIPT_OPERATION_LIMIT`; no compatibility alias. ExecutionTiming
  literal initializers must include the two new fields. Rhai Runtime API stays 2.

Validation: 497 core unit tests, six public policy fixtures in debug and release,
three actual native mount/secondary-window fixtures. The same scalar workload
really consumes 1,350,012 operations: default rejects at 1,000,001, raised2M
succeeds with result450000, lower500k rejects at500001. The parser fixture
rejects function32/48 and admits64, without changing the normal default.
Structured metadata also proves that a failed component delta below quota does
not hide a cumulative rejection. Strict root checks pass; final combined native,
platform, performance and release gates remain the RC's responsibility.

Plan/evidence: `docs/plans/2026-10-02-0.1.8-release-convergence.zh-CN.md`,
`docs/audits/2026-10-02-release-convergence/evidence/d1-*.log`.
