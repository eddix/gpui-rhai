# 0.1.8 runtime contract index

This index maps model invariants to executable product tests. Independent audit
programs remain unchanged evidence; they are not substituted for these tests.

| ID / contract | Product test entry and controls |
| --- | --- |
| WIN-01 exact mount ownership/deferred revocation | `window_authority.rs`: same/new Host, same/new ID, Focus/Close, retained clone control |
| WIN-02 enqueue origin/peer pump/reservation | `window_command_origin.rs`, `window_command_deliveries.rs`: timer/input/task/subscription, mixed success order, live/suspend versus dispose/drop/close/remount |
| VIRT-01 final invocation manifest/scoped atomic state deltas | `crates/gpui-rhai/tests/virtual_transactions.rs`: sibling/overlap order, batch, failure retry; existing lifecycle transparent-wrapper tests |
| READ-01 direct/item contributions and rollback | `crates/gpui-rhai/tests/virtual_read_contributions.rs`: raw/formal, root/shared/path, 80-key history, late registration, failed candidate |
| TABLE-01 shared shell/clipping/controlled columns | `table_boundaries.rs`: four states, fill/fixed height, final/sole divider, rejection |
| TABLE-02 resolved extent/RTL/native wheel | `table_extent.rs`: fixed/mixed, Array/Native, state transitions, viewport resize, actual tail interaction, zero-Rhai X/Y controls, theme insets |
| REF-01 binding continuity | `element_ref_subscriptions.rs`: unknown→bound, same-node, rebind, removed, formal virtual row controls |
| REF-02 retarget/contribution/teardown | `element_ref.rs` units: contribution retarget, last-item prune, direct+ref coexistence, provider and owner final-manifest removal |
| REF-03 last-good rollback/resume | Native `failed_effect_commit_restores_ref_binding_and_native_subscription`, `suspend_resume_preserves_logical_ref_readers`; snapshot and unchanged-binding unit controls |

The fixed matrix covers identity, lifecycle, sequential versus batched targets,
retained/added/pruned contributions, actual geometry and failure rollback. The
candidate verification manifest records which entries have run and on which
source SHA; entries in this index are not themselves a claim of a passing gate.

Native filenames above are under `tests/native-keyboard/tests/`. The normal
termination matrix additionally remains in that workspace's `keyboard.rs`
(commit/reject/Escape/deactivate/suspend/dispose/offscreen controls). No test
may increase `RUST_MIN_STACK`, global Rhai budgets or parser defaults to pass.
