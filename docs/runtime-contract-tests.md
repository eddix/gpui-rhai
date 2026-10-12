# Runtime contract index

This index maps model invariants to executable product tests. Independent audit
programs remain unchanged evidence; they are not substituted for these tests.

| ID / contract | Product test entry and controls |
| --- | --- |
| WIN-01 exact mount ownership/deferred revocation | `window_authority.rs`: same/new Host, same/new ID, Focus/Close, retained clone control |
| WIN-02 enqueue origin/peer pump/reservation | `window_command_origin.rs`, `window_command_deliveries.rs`: timer/input/task/subscription, mixed success order, live/suspend versus dispose/drop/close/remount |
| VIRT-01 final invocation manifest/scoped atomic state deltas | `crates/gpui-rhai/tests/virtual_transactions.rs`: sibling/overlap order, batch, failure retry; existing lifecycle transparent-wrapper tests |
| READ-01 direct/item contributions and rollback | `crates/gpui-rhai/tests/virtual_read_contributions.rs`: raw/formal, root/shared/path, 80-key history, late registration, failed candidate |
| TABLE-01 shared shell/clipping/controlled columns | `table_boundaries.rs`: four states, fill/fixed height, final/sole divider, rejection |
| TABLE-02 resolved extent/RTL/native wheel | `table_extent.rs`: fixed/mixed, Array/Native, state transitions, viewport resize, actual tail interaction, zero-Rhai X/Y controls, theme insets; `four_column_registry_width_descriptors_share_the_actual_native_plan` measures the original 120/flex(2)/30%/90 specimen rather than its obsolete per-leaf style representation |
| TABLE-03 measured-frame demand | `table_frame_demand.rs`: initial/changed viewport converges through scheduled frames alone, zero-Rhai follow-up and bounded idle; extra refresh is only an after-assertion driver control |
| TABLE-04 direction continuity | Native `table_extent::direction_changes_preserve_logical_scroll_distance_and_clamp`: Array/Native × LTR↔RTL, nonzero position, reduced range and switch-back; header/body alignment remains checked |
| TABLE-05 one keyboard contract for both data sources | `table_keyboard.rs` `both_data_sources_follow_the_same_keyboard_contract`: Array and NativeCollection, plain, filtered, paged, sorted and collapsed groups, Enter (PR #111 R3) |
| NODE-01 caller layout presentation | Core lifecycle `table_modifiers_replay_{track,column}_across_incremental_updates_and_failed_candidates`: internal/external metadata, repeated child-only rerender, style control, failed candidate and retry; internal values update while caller decoration remains authoritative |
| REF-01 binding continuity | `element_ref_subscriptions.rs`: unknown→bound, same-node, rebind, removed, formal virtual row controls |
| REF-02 retarget/contribution/teardown | `element_ref.rs` units: contribution retarget, last-item prune, direct+ref coexistence, provider and owner final-manifest removal |
| REF-03 last-good rollback/resume | Native `failed_effect_commit_restores_ref_binding_and_native_subscription`, `suspend_resume_preserves_logical_ref_readers`; snapshot and unchanged-binding unit controls |
| POLICY-01 independent Host policy | Root/native `host_execution_policy.rs`: real over-default finite work, cumulative siblings, fresh delayed/retained rounds, parser defaults and extension/secondary precedence; engine candidate and diagnostic quota units |
| KEY-01 one grammar and executable phase | `node_key_contract.rs` and node units: canonical names, Capture/Target/Bubble, focus/stop/disabled/fallback, input/IME and Host Escape owner; `key_phases.rs` `every_phase_matches_qualified_names_first_and_plain_names_as_fallback`: a modified key such as Shift+F6 matches the same way in all three phases (PR #111 R4) |
| LAYOUT-01 implicit stretch equals explicit stretch | `stretch_layout.rs` `implicit_stretch_lays_out_like_an_explicit_stretch`: margin, logical start margin, auto margin, padding and border, margin with padding, max and min width, in LTR and RTL; `a_signal_selected_style_stretches_like_the_same_static_style`: margin, start margin, auto margin, self alignment, absolute position and width applied through `signal_style`, switched on, off and on (PR #111 R1, S3) |
| HIT-01 decorative grips take presses only where visible | `control_visuals.rs` `a_split_pane_grip_takes_no_press_where_it_is_covered_or_clipped`, `a_resizable_grip_takes_no_press_where_it_is_covered_or_clipped`, with the overhang controls `split_pane_grip_overhang_starts_a_drag_and_shows_hover` and `resizable_grip_overhang_starts_its_handle_drag` (PR #111 S1) |
| ORIGIN-01 follow-up work keeps its origin | `invocation_origin.rs` `capability_calls_carry_their_origin`: an action dispatched or an event emitted from a click, an automation command, a task completion, a timer or an effect runs with that origin after the callback that queued it returns (PR #111 S2); `async_capability_origin.rs`: task and subscription handlers receive the `InvocationContext` (#133) |
| REGION-01 a non-scrolling body clips | `region_toolbar.rs` `a_region_body_clips_unless_it_scrolls`: painting and hits stop at the region's edge, inside presses still land (PR #111 S4) |
| THEME-01 resolved environment | Root/native `resolved_theme.rs`: lightweight identity, init-before-native-environment, effects/scope/borrow/override and secondary failure; core appearance ingestion unit |
| THEME-02 native notification ingress | Actual macOS `tests/native-keyboard/src/bin/theme_appearance_probe.rs` and `theme_appearance_lifecycle.rs`: genuine idle Light/Dark, primary/secondary init-once, explicit effects, independent Runtime views/fixed control, suspended pending/resume, old disposed Handle/remount, token-only native repaint; these are structured-result native app gates, not TestPlatform or OS-global preference changes |
| THEME-03 reload validates like preparation | `token_reload.rs`, run with `--features dev-reload`: `a_token_edit_preparation_would_reject_keeps_the_last_good_theme`, `a_new_component_requirement_and_its_token_apply_in_one_batch`, `a_component_that_needs_a_missing_token_is_not_reloaded` (PR #111 R2) |
| THEME-04 environment tables are complete | Core unit `theme::tests::environment_tables_must_cover_every_declared_combination`: a missing default branch, a missing non-default branch and a missing multi-axis combination are rejected; complete tables resolve (PR #111 R5) |
| ASSET-01 explicit Host publication | `asset_refresh_host.rs`: typed worker/foreground bridge, stable ID+decoded pixels, explicit shared-registry dual-window repaint |

The fixed matrix covers identity, lifecycle, sequential versus batched targets,
retained/added/pruned contributions, actual geometry and failure rollback. The
candidate verification manifest records which entries have run and on which
source SHA; entries in this index are not themselves a claim of a passing gate.

Native filenames above are under `tests/native-keyboard/tests/`. The normal
termination matrix additionally remains in that workspace's `keyboard.rs`
(commit/reject/Escape/deactivate/suspend/dispose/offscreen controls). No test
may increase `RUST_MIN_STACK`, global Rhai budgets or parser defaults to pass.
