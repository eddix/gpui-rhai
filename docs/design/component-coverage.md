# Modern / Clean module coverage

Generated from the maintained recipes and public SDK modules; this is not a
reference-project survey. Both designs and both appearances declare every
module below. Counts include intentionally empty structural/interaction root
recipes, which must not acquire decorative surfaces.

Recipe presence does **not** mean visual or interaction acceptance. The
canonical presentation/acceptance application is `gpui-rhai gallery`.
The native `gallery::module_coverage` gate traverses its committed
composition in all four presets; importing or naming a component does not
count. Opening overlays, virtual content, native painted states, keyboard and
lifecycle behavior remain separate checks. The complete visual/motion gates
in the [design specification](modern-clean-spec.md) are still required.

| Module | Explicit part recipes |
|---|---:|
| `components/accordion` | 4 |
| `components/ai_answer` | 6 |
| `components/alert` | 4 |
| `components/alert_dialog` | 3 |
| `components/avatar` | 3 |
| `components/badge` | 3 |
| `components/breadcrumb` | 13 |
| `components/button` | 6 |
| `components/button_group` | 2 |
| `components/calendar` | 18 |
| `components/card` | 5 |
| `components/carousel` | 4 |
| `components/checkbox` | 3 |
| `components/cited_answer` | 18 |
| `components/code_viewer` | 6 |
| `components/collapsible` | 3 |
| `components/column_browser` | 4 |
| `components/combobox` | 15 |
| `components/command` | 34 |
| `components/command_dialog` | 1 |
| `components/context_menu` | 5 |
| `components/date_picker` | 13 |
| `components/date_range_picker` | 16 |
| `components/dialog` | 9 |
| `components/diff_viewer` | 6 |
| `components/divider` | 1 |
| `components/drag_source` | 1 |
| `components/draggable` | 1 |
| `components/drop_zone` | 1 |
| `components/empty` | 5 |
| `components/filter_menu` | 26 |
| `components/form_field` | 4 |
| `components/group_box` | 3 |
| `components/hover_card` | 3 |
| `components/icon` | 1 |
| `components/icon_button` | 3 |
| `components/input` | 2 |
| `components/input_group` | 3 |
| `components/kbd` | 9 |
| `components/label` | 4 |
| `components/list` | 9 |
| `components/markdown_viewer` | 9 |
| `components/menu` | 6 |
| `components/message_actions` | 12 |
| `components/number_input` | 13 |
| `components/otp_input` | 13 |
| `components/pagination` | 17 |
| `components/pan_zoom` | 1 |
| `components/popover` | 2 |
| `components/progress` | 3 |
| `components/progress_tracker` | 17 |
| `components/radio` | 5 |
| `components/radio_group` | 1 |
| `components/range_slider` | 6 |
| `components/reaction_bar` | 4 |
| `components/resizable` | 6 |
| `components/rotatable` | 1 |
| `components/scroll_area` | 4 |
| `components/select` | 12 |
| `components/selectable_card_group` | 5 |
| `components/selectable_list` | 6 |
| `components/selection_area` | 1 |
| `components/setup_guide` | 26 |
| `components/sheet` | 6 |
| `components/skeleton` | 1 |
| `components/slider` | 6 |
| `components/sliding_panel` | 1 |
| `components/sortable` | 2 |
| `components/spinner` | 2 |
| `components/split_pane` | 6 |
| `components/status_bar` | 1 |
| `components/stepper_form` | 28 |
| `components/switch` | 7 |
| `components/tab_overflow_menu` | 1 |
| `components/table` | 17 |
| `components/tabs` | 11 |
| `components/tag` | 5 |
| `components/tag_input` | 17 |
| `components/textarea` | 5 |
| `components/thinking_loader` | 7 |
| `components/thinking_state` | 10 |
| `components/time_input` | 1 |
| `components/time_range_picker` | 4 |
| `components/timeline` | 7 |
| `components/title_bar` | 3 |
| `components/toast` | 5 |
| `components/toggle` | 1 |
| `components/toggle_group` | 5 |
| `components/tooltip` | 4 |
| `components/tree` | 6 |
| `layouts/inline` | 1 |
| `layouts/region` | 8 |
| `layouts/stack` | 1 |
| `layouts/toolbar` | 1 |
| `patterns/agent_checkpoints` | 17 |
| `patterns/agent_cursors` | 36 |
| `patterns/agent_findings` | 39 |
| `patterns/agent_fleet` | 32 |
| `patterns/agent_graph` | 33 |
| `patterns/agent_handoff` | 28 |
| `patterns/agent_inbox` | 33 |
| `patterns/agent_lanes` | 20 |
| `patterns/agent_ledger` | 40 |
| `patterns/agent_monitor` | 39 |
| `patterns/agent_plan` | 21 |
| `patterns/agent_queue` | 33 |
| `patterns/agent_recap` | 36 |
| `patterns/agent_steering` | 29 |
| `patterns/agent_trail` | 25 |
| `patterns/agent_tree` | 30 |
| `patterns/app_shell` | 3 |
| `patterns/approval_card` | 28 |
| `patterns/approval_gates` | 29 |
| `patterns/autonomy_dial` | 18 |
| `patterns/away_summary` | 19 |
| `patterns/billing_schedule` | 31 |
| `patterns/breakdown_chart` | 31 |
| `patterns/change_summary` | 16 |
| `patterns/collaboration_stream` | 32 |
| `patterns/comment_thread` | 19 |
| `patterns/comments_panel` | 20 |
| `patterns/compact_breakdown` | 30 |
| `patterns/composer` | 1 |
| `patterns/conflict_radar` | 22 |
| `patterns/context_map` | 30 |
| `patterns/context_window` | 29 |
| `patterns/cost_waterfall` | 30 |
| `patterns/data_view` | 1 |
| `patterns/decision_card` | 20 |
| `patterns/decision_inbox` | 10 |
| `patterns/decision_tree` | 19 |
| `patterns/delegate_board` | 30 |
| `patterns/description_list` | 3 |
| `patterns/diff_timeline` | 39 |
| `patterns/email_draft` | 43 |
| `patterns/experiment_card` | 25 |
| `patterns/fallback_chain` | 35 |
| `patterns/fan_out_grid` | 27 |
| `patterns/form_layout` | 5 |
| `patterns/inline_state` | 4 |
| `patterns/integration_picker` | 19 |
| `patterns/integration_settings` | 19 |
| `patterns/judge_bracket` | 29 |
| `patterns/kpi_chart` | 20 |
| `patterns/list_detail` | 1 |
| `patterns/memory_panel` | 31 |
| `patterns/model_picker` | 11 |
| `patterns/navigation_stack` | 1 |
| `patterns/notification_stack` | 11 |
| `patterns/parallel_attempts` | 30 |
| `patterns/permission_request` | 28 |
| `patterns/plan_vs_actual` | 32 |
| `patterns/profile_menu` | 20 |
| `patterns/project_card` | 27 |
| `patterns/prompt_library` | 21 |
| `patterns/rate_limit_planner` | 24 |
| `patterns/review_center` | 26 |
| `patterns/root_cause_tree` | 31 |
| `patterns/run_controls` | 27 |
| `patterns/running_now` | 25 |
| `patterns/scheduled_runs` | 40 |
| `patterns/section` | 3 |
| `patterns/smart_recommendation` | 20 |
| `patterns/stat` | 4 |
| `patterns/tips_panel` | 16 |
| `patterns/tool_access_map` | 31 |
| `patterns/tool_call_card` | 18 |
| `patterns/tool_call_group` | 7 |
| `patterns/trace_waterfall` | 39 |
| `patterns/usage_meter` | 52 |
| `patterns/writing_assistant` | 20 |
| `motion/animated_tabs` | 1 |
| `motion/border_beam` | 1 |
| `motion/marquee` | 1 |
| `motion/number_ticker` | 1 |
| `motion/orbit` | 1 |
| `motion/particles` | 1 |
| `motion/reorder_list` | 2 |
| `motion/shared_layout_cards` | 2 |
| `motion/shimmer` | 1 |
| `motion/text_reveal` | 1 |
| `charts/bar_chart` | 1 |
| `charts/chart` | 1 |
| `charts/line_chart` | 1 |
| `charts/map_chart` | 1 |
| `charts/pie_chart` | 1 |

Total: **186 public modules**, excluding internal functions and
without counting each series in the generic Chart module as a separate module.

Application services, editable spreadsheet semantics and other new product
capabilities are not added by this visual migration. Existing native state,
keyboard, controlled-value and lifecycle contracts remain in force.
