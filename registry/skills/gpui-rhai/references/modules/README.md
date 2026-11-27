# Module reference

Generated from the schemas and header comments of the official modules; do not
edit. Regenerate with
`GPUI_RHAI_UPDATE_REFERENCE=1 cargo test -p gpui-rhai-cli reference`.

The functions, methods and native primitives a script calls are in the
[script API](../script-api.md).

## Components

| Module | Export | Purpose |
|---|---|---|
| [`components/accordion`](components/accordion.md) | `Accordion` | Controlled single/multiple accordion with Rust-side clip motion. |
| [`components/ai_answer`](components/ai_answer.md) | `AIAnswer` | AIAnswer composes a streamed reply, evidence, status, feedback and follow-ups. |
| [`components/alert`](components/alert.md) | `Alert` | Persistent inline feedback with semantic variants and composable actions. |
| [`components/alert_dialog`](components/alert_dialog.md) | `AlertDialog` | Controlled confirmation dialog with explicit cancel and confirm semantics: every close reports `confirm` or `cancel` first, Escape and backdrop presses `cancel`. |
| [`components/avatar`](components/avatar.md) | `Avatar` | Avatar displays an image handle or name-derived initials with presence. |
| [`components/badge`](components/badge.md) | `Badge` | Badge presents a soft status surface, optional dot/icon, or a numeric count. |
| [`components/breadcrumb`](components/breadcrumb.md) | `Breadcrumb` | A controlled ancestry trail. |
| [`components/button`](components/button.md) | `Button` | Button presents a desktop action: a large block with a centered label. |
| [`components/button_group`](components/button_group.md) | `ButtonGroup` | Visually joins related buttons into one horizontal or vertical control. |
| [`components/calendar`](components/calendar.md) | `Calendar` | The single calendar engine shared by DatePicker and TimeRangePicker. |
| [`components/card`](components/card.md) | `Card` | General themed content surface with optional header and footer slots. |
| [`components/carousel`](components/carousel.md) | `Carousel` | A controlled pager that shows one item at a time, for a compact place that steps through items of one kind: alerts, cards, previews. |
| [`components/checkbox`](components/checkbox.md) | `Checkbox` | Controlled checkbox with checked, unchecked, and indeterminate states. |
| [`components/cited_answer`](components/cited_answer.md) | `CitedAnswer` | CitedAnswer presents linked evidence, inline citations and a source preview. |
| [`components/code_viewer`](components/code_viewer.md) | `CodeViewer` | Native, virtualized, selectable read-only source document surface. |
| [`components/collapsible`](components/collapsible.md) | `Collapsible` | Controlled disclosure with trigger/content slots and Rust clip animation. |
| [`components/column_browser`](components/column_browser.md) | `ColumnBrowser` | Controlled Miller columns. |
| [`components/combobox`](components/combobox.md) | `Combobox` | Public Rhai choice composition over Overlay, Input, and virtual_collection. |
| [`components/command`](components/command.md) | `Command` | Embeddable keyboard-first command search with deterministic fuzzy ranking. |
| [`components/command_dialog`](components/command_dialog.md) | `CommandDialog` | Controlled modal presentation of Command without registering a global shortcut. |
| [`components/context_menu`](components/context_menu.md) | `ContextMenu` | Controlled action menu sharing Menu items and keyboard behavior, anchored at the pointer on a right-button press or at the trigger on Shift+F10 or the menu key. |
| [`components/date_picker`](components/date_picker.md) | `DatePicker` | Controlled single-date field composed from public date data helpers, atoms, and Overlay. |
| [`components/date_range_picker`](components/date_range_picker.md) | `DateRangePicker` | DateRangePicker proposes an inclusive civil-date interval and a comparison only on Apply. |
| [`components/dialog`](components/dialog.md) | `Dialog` | Dialog presents controlled modal content through the native overlay layer. |
| [`components/diff_viewer`](components/diff_viewer.md) | `DiffViewer` | Neutral two-way, read-only source comparison over the native document surface. |
| [`components/divider`](components/divider.md) | `Divider` | Divider separates adjacent content without owning layout spacing. |
| [`components/drag_source`](components/drag_source.md) | `DragSource` | Typed in-application data drag source. |
| [`components/draggable`](components/draggable.md) | `Draggable` | Controlled in-window positioning. |
| [`components/drop_zone`](components/drop_zone.md) | `DropZone` | Typed same-Host application drop target; with `os.files`, also files and folders the operating system drags in, when the Host accepts file drops. |
| [`components/empty`](components/empty.md) | `Empty` | Consistent empty-state composition with optional icon, content, and actions. |
| [`components/filter_menu`](components/filter_menu.md) | `FilterMenu` | A grouped multi-select filter surface. |
| [`components/form_field`](components/form_field.md) | `FormField` | Associates label, control, description, required state, and error content. |
| [`components/group_box`](components/group_box.md) | `GroupBox` | Labelled group of related controls or settings: a top hairline and a label-voice heading on the parent's content edge, with no surface of its own. |
| [`components/hover_card`](components/hover_card.md) | `HoverCard` | A non-modal, interactive preview that opens on intentional hover or keyboard focus. |
| [`components/icon`](components/icon.md) | `Icon` | Icon wraps a declarative AssetId or a runtime image handle with consistent sizing. |
| [`components/icon_button`](components/icon_button.md) | `IconButton` | IconButton presents one icon in a square action target. |
| [`components/input`](components/input.md) | `Input` | Input is a controlled single-line native text input. |
| [`components/input_group`](components/input_group.md) | `InputGroup` | Composes an input-like control with integrated prefix and suffix content. |
| [`components/kbd`](components/kbd.md) | `Kbd` | Kbd presents one framed key or combination, or a bare inline key legend. |
| [`components/label`](components/label.md) | `Label` | Label presents a control label with optional required and description text. |
| [`components/list`](components/list.md) | `List` | Controlled virtualized list of keyed rows: the rows Table draws, without columns. |
| [`components/markdown_viewer`](components/markdown_viewer.md) | `MarkdownViewer` | Native CommonMark/GFM, not an editor or a WebView. |
| [`components/menu`](components/menu.md) | `Menu` | Controlled in-window menu with roving selection and nested submenu nodes. |
| [`components/message_actions`](components/message_actions.md) | `MessageActions` | MessageActions presents clipboard, version, speech request and feedback controls for one message. |
| [`components/number_input`](components/number_input.md) | `NumberInput` | NumberInput is a controlled numeric field with a native editor, locale and steppers. |
| [`components/otp_input`](components/otp_input.md) | `OtpInput` | OtpInput edits one controlled verification code using a single native text field. |
| [`components/pagination`](components/pagination.md) | `Pagination` | Pagination composes one-based controlled page state; only popup state is local. |
| [`components/pan_zoom`](components/pan_zoom.md) | `PanZoom` | Controlled Canvas viewport transform with native pan and pointer-anchored zoom. |
| [`components/popover`](components/popover.md) | `Popover` | Popover renders controlled content through the native per-window overlay layer. |
| [`components/progress`](components/progress.md) | `Progress` | Determinate or indeterminate progress indicator animated entirely in Rust. |
| [`components/progress_tracker`](components/progress_tracker.md) | `ProgressTracker` | A keyed checklist with progress. |
| [`components/radio`](components/radio.md) | `Radio` | Controlled radio option. |
| [`components/radio_group`](components/radio_group.md) | `RadioGroup` | Controlled radio group with one tab stop and roving arrow-key selection. |
| [`components/range_slider`](components/range_slider.md) | `RangeSlider` | Controlled two-thumb range selection with native pointer preview and keyboard thumbs. |
| [`components/reaction_bar`](components/reaction_bar.md) | `ReactionBar` | ReactionBar shows a compact set of social actions and their exact accessible counts. |
| [`components/resizable`](components/resizable.md) | `Resizable` | Controlled single-element rectangle resizing inside one local boundary. |
| [`components/rotatable`](components/rotatable.md) | `Rotatable` | Controlled Canvas rotation around an explicit local pivot. |
| [`components/scroll_area`](components/scroll_area.md) | `ScrollArea` | Sized retained scroll viewport with themeable overlay scrollbars. |
| [`components/select`](components/select.md) | `Select` | Scalar controlled Select composed from the public Rhai Combobox component. |
| [`components/selectable_card_group`](components/selectable_card_group.md) | `SelectableCardGroup` | Small controlled rich choices with one roving selection tab stop. |
| [`components/selectable_list`](components/selectable_list.md) | `SelectableList` | The behavior of List and Table rows with rows the application draws: selection, the one tab stop and its keys, the cursor and focus frame, row and context requests, accessibility and virtualization stay the list's, and a `row` Node factory draws each row's content. |
| [`components/selection_area`](components/selection_area.md) | `SelectionArea` | Controlled Canvas click/range/marquee object selection. |
| [`components/setup_guide`](components/setup_guide.md) | `SetupGuide` | An onboarding panel with a bounded local disclosure model and Host-owned data. |
| [`components/sheet`](components/sheet.md) | `Sheet` | Controlled temporary modal panel attached to a viewport edge. |
| [`components/skeleton`](components/skeleton.md) | `Skeleton` | Content placeholder with optional Rust-side pulse animation. |
| [`components/slider`](components/slider.md) | `Slider` | Controlled single-value range input with native drag preview and commit events. |
| [`components/sliding_panel`](components/sliding_panel.md) | `SlidingPanel` | Shows `content`, and when `value` changes slides the new content in from the side of the one it replaces as it fades in: a short way (`motion_distance("moderate")`) in the `fast` duration, mirrored in right-to-left text and settled by the Host's reduced or no-motion policy. |
| [`components/sortable`](components/sortable.md) | `Sortable` | Controlled keyed ordering with native pointer targets and keyboard moves. |
| [`components/spinner`](components/spinner.md) | `Spinner` | Compact indeterminate activity indicator animated by the native runtime clock. |
| [`components/split_pane`](components/split_pane.md) | `SplitPane` | Controlled, nestable two-panel split layout with a native drag separator. |
| [`components/status_bar`](components/status_bar.md) | `StatusBar` | Compact application footer with stable logical start, center, and end regions. |
| [`components/stepper_form`](components/stepper_form.md) | `StepperForm` | StepperForm presents a reliable workflow around caller-owned keyed form content. |
| [`components/switch`](components/switch.md) | `Switch` | Controlled binary switch with disabled and loading states. |
| [`components/tab_overflow_menu`](components/tab_overflow_menu.md) | `TabOverflowMenu` | A chevron button whose menu lists every tab of a strip; picking one reports its value. |
| [`components/table`](components/table.md) | `Table` | Data-backed Table composed entirely from public Box/Text/virtual_collection APIs. |
| [`components/tabs`](components/tabs.md) | `Tabs` | Controlled tabs with one tab stop; the arrow keys select. |
| [`components/tag`](components/tag.md) | `Tag` | Tag labels a category with a removable-chip surface and an optional facet segment. |
| [`components/tag_input`](components/tag_input.md) | `TagInput` | A single native editor for a controlled, ordered list of removable tags. |
| [`components/textarea`](components/textarea.md) | `Textarea` | Textarea is a controlled multiline field with native IME, selection, wrapping, and auto-grow. |
| [`components/thinking_loader`](components/thinking_loader.md) | `ThinkingLoader` | ThinkingLoader presents a phase label and motion-owned work/completion cue. |
| [`components/thinking_state`](components/thinking_state.md) | `ThinkingState` | ThinkingState presents application-visible workflow steps and their evidence. |
| [`components/time_input`](components/time_input.md) | `TimeInput` | Controlled locale-aware clock editing; partial text remains an editable draft. |
| [`components/time_range_picker`](components/time_range_picker.md) | `TimeRangePicker` | Controlled relative windows and explicit date-time drafts, committed only on Apply. |
| [`components/timeline`](components/timeline.md) | `Timeline` | A source-owned, controlled timeline. |
| [`components/title_bar`](components/title_bar.md) | `TitleBar` | Source-owned application chrome with logical start, center, and end regions. |
| [`components/toast`](components/toast.md) | `Toast` | Controlled Toast composition over public Layer, declarative timeout, atoms, and hover events. |
| [`components/toggle`](components/toggle.md) | `Toggle` | Controlled pressed-state tool button, distinct from Checkbox and Switch. |
| [`components/toggle_group`](components/toggle_group.md) | `ToggleGroup` | ToggleGroup holds pressed state in a padded tonal track, with independent selected surfaces and a stable intrinsic label width across font weights. |
| [`components/tooltip`](components/tooltip.md) | `Tooltip` | Native delayed tooltip using the per-window overlay scheduler. |
| [`components/tree`](components/tree.md) | `Tree` | Controlled virtualized hierarchical outline over the shared Rust projection. |

## Layouts

| Module | Export | Purpose |
|---|---|---|
| [`layouts/inline`](layouts/inline.md) | `Inline` | Inline places children in a row with one spacing relationship and one control size. |
| [`layouts/region`](layouts/region.md) | `Region` | Region is one working area: a header with the region's title and actions, an optional toolbar, a body that fills the remaining height, and a footer pinned to the bottom. |
| [`layouts/stack`](layouts/stack.md) | `Stack` | Stack places children in a column with one spacing relationship between them. |
| [`layouts/toolbar`](layouts/toolbar.md) | `Toolbar` | Toolbar arranges a region's controls by role: fields at the start, then what the bar acts on, secondary actions and the one primary action at the end. |

## Patterns

| Module | Export | Purpose |
|---|---|---|
| [`patterns/agent_checkpoints`](patterns/agent_checkpoints.md) | `AgentCheckpoints` | An immutable chronological history with a controlled head. |
| [`patterns/agent_cursors`](patterns/agent_cursors.md) | `AgentCursors` | Present a collaboratively reviewed document with live grapheme-anchored cursors. |
| [`patterns/agent_findings`](patterns/agent_findings.md) | `AgentFindings` | Present a team of checking agents, live progress and a bounded newest-first feed of findings. |
| [`patterns/agent_fleet`](patterns/agent_fleet.md) | `AgentFleet` | A run overview with one expanded lead, a child roster and a task inspector. |
| [`patterns/agent_graph`](patterns/agent_graph.md) | `AgentGraph` | Requests and results travel across the accepted run's communication graph. |
| [`patterns/agent_handoff`](patterns/agent_handoff.md) | `AgentHandoff` | Inspect relay evidence, restore omitted material, or propose assigning a task. |
| [`patterns/agent_inbox`](patterns/agent_inbox.md) | `AgentInbox` | One agent-task inbox in board and list form. |
| [`patterns/agent_lanes`](patterns/agent_lanes.md) | `AgentLanes` | A clipped, interactive time window with non-overlapping agent tasks and presented hand-offs. |
| [`patterns/agent_ledger`](patterns/agent_ledger.md) | `AgentLedger` | Describe the changes an agent made and preview atomic undo or redo batches. |
| [`patterns/agent_monitor`](patterns/agent_monitor.md) | `AgentMonitor` | A single accepted run, its activity clock, and recent steps with native inspection. |
| [`patterns/agent_plan`](patterns/agent_plan.md) | `AgentPlan` | Review and edit an ordered plan, then present the Host's execution and recovery. |
| [`patterns/agent_queue`](patterns/agent_queue.md) | `AgentQueue` | Worker dispatch, interactive job inspection and queue depth on a shared Host clock. |
| [`patterns/agent_recap`](patterns/agent_recap.md) | `AgentRecap` | Replay an accepted interval of agent activity and send versioned event commands to the Host. |
| [`patterns/agent_steering`](patterns/agent_steering.md) | `AgentSteering` | Send versioned steering intent without editing the Host's accepted plan or queue. |
| [`patterns/agent_trail`](patterns/agent_trail.md) | `AgentTrail` | Follow execution and drill into recent substeps without transferring execution to the UI. |
| [`patterns/agent_tree`](patterns/agent_tree.md) | `AgentTree` | A dispatch hierarchy with separate visual, focus and accepted run state. |
| [`patterns/app_shell`](patterns/app_shell.md) | `AppShell` | AppShell is the window frame of a productivity tool: title bar, sidebar, main area, optional inspector, status bar, and keyboard regions. |
| [`patterns/approval_card`](patterns/approval_card.md) | `ApprovalCard` | ApprovalCard presents a versioned request, its evidence and the accepted receipt. |
| [`patterns/approval_gates`](patterns/approval_gates.md) | `ApprovalGates` | ApprovalGates presents the pending gates, blocked steps and accepted history of one run. |
| [`patterns/autonomy_dial`](patterns/autonomy_dial.md) | `AutonomyDial` | AutonomyDial groups actions under the currently accepted policy, with a three-way selector and per-action pins. |
| [`patterns/away_summary`](patterns/away_summary.md) | `AwaySummary` | An inline recap of work that changed while someone was away. |
| [`patterns/billing_schedule`](patterns/billing_schedule.md) | `BillingSchedule` | Build a phase and preview the first invoice and phase value without executing billing. |
| [`patterns/breakdown_chart`](patterns/breakdown_chart.md) | `BreakdownChart` | One signed measure by group and segment, with a linked summary table. |
| [`patterns/change_summary`](patterns/change_summary.md) | `ChangeSummary` | Summarize accepted changes with separate direction and outcome. |
| [`patterns/collaboration_stream`](patterns/collaboration_stream.md) | `CollaborationStream` | A chronological, virtualized agent conversation with Host-owned messages and draft. |
| [`patterns/comment_thread`](patterns/comment_thread.md) | `CommentThread` | CommentThread shows accepted messages, optional assistant details and a controlled reply field. |
| [`patterns/comments_panel`](patterns/comments_panel.md) | `CommentsPanel` | CommentsPanel filters and sorts an accepted comment feed; edit and resolve actions are proposals. |
| [`patterns/compact_breakdown`](patterns/compact_breakdown.md) | `CompactBreakdown` | A total split into stable parts, with ranked detail and a controlled period. |
| [`patterns/composer`](patterns/composer.md) | `Composer` | Controlled drafting surface. |
| [`patterns/conflict_radar`](patterns/conflict_radar.md) | `ConflictRadar` | A branch activity matrix, exact line overlap, and Host-owned merge proposals. |
| [`patterns/context_map`](patterns/context_map.md) | `ContextMap` | Inspect the area occupied by ordered context blocks, including a prior compaction. |
| [`patterns/context_window`](patterns/context_window.md) | `ContextWindow` | A token allocation field with Host-controlled removal and compaction. |
| [`patterns/cost_waterfall`](patterns/cost_waterfall.md) | `CostWaterfall` | Show a run ledger, with exact monetary rollups and independently controlled view state. |
| [`patterns/data_view`](patterns/data_view.md) | `DataView` | DataView is a browsable collection in one region: toolbar, table or list, and a footer with status, selection, count and data time. |
| [`patterns/decision_card`](patterns/decision_card.md) | `DecisionCard` | DecisionCard pages through Host-owned choices, visual picks and approvals. |
| [`patterns/decision_inbox`](patterns/decision_inbox.md) | `DecisionInbox` | DecisionInbox is a queue of decisions with two quick responses and accepted receipts. |
| [`patterns/decision_tree`](patterns/decision_tree.md) | `DecisionTree` | A geometry-backed tree of versioned choices. |
| [`patterns/delegate_board`](patterns/delegate_board.md) | `DelegateBoard` | Move work between an unassigned pile and agent queues with one versioned assignment contract. |
| [`patterns/description_list`](patterns/description_list.md) | `DescriptionList` | DescriptionList shows facts as label and value pairs on one label column. |
| [`patterns/diff_timeline`](patterns/diff_timeline.md) | `DiffTimeline` | Plot cumulative additions, deletions and signed net changes from a Host ledger. |
| [`patterns/email_draft`](patterns/email_draft.md) | `EmailDraft` | Review an authored email without giving this view transport or file access. |
| [`patterns/experiment_card`](patterns/experiment_card.md) | `ExperimentCard` | A compact experiment that unfolds into a controlled comparison. |
| [`patterns/fallback_chain`](patterns/fallback_chain.md) | `FallbackChain` | Inspect attempts in a host-owned fallback chain, without making requests. |
| [`patterns/fan_out_grid`](patterns/fan_out_grid.md) | `FanOutGrid` | Inspect a dense field of tasks and propose retrying the failed subset. |
| [`patterns/form_layout`](patterns/form_layout.md) | `FormLayout` | FormLayout aligns labels on one column, groups fields, and lines the submit row up with the fields. |
| [`patterns/inline_state`](patterns/inline_state.md) | `InlineState` | InlineState shows loading, empty, error, stale and refreshing feedback where the content would appear. |
| [`patterns/integration_picker`](patterns/integration_picker.md) | `IntegrationPicker` | IntegrationPicker browses application integrations and proposes changes. |
| [`patterns/integration_settings`](patterns/integration_settings.md) | `IntegrationSettings` | IntegrationSettings edits versioned site configuration and requests Host credential actions. |
| [`patterns/judge_bracket`](patterns/judge_bracket.md) | `JudgeBracket` | An ordered elimination bracket, with comparison and a Host-owned champion choice. |
| [`patterns/kpi_chart`](patterns/kpi_chart.md) | `KpiChart` | Multi-series KPI card with a shared sample focus and honest gaps. |
| [`patterns/list_detail`](patterns/list_detail.md) | `ListDetail` | ListDetail shows a list and the selected item's detail, side by side or stacked. |
| [`patterns/memory_panel`](patterns/memory_panel.md) | `MemoryPanel` | Edit an explicit Host memory snapshot. |
| [`patterns/model_picker`](patterns/model_picker.md) | `ModelPicker` | ModelPicker presents the application-provided model list and request options. |
| [`patterns/navigation_stack`](patterns/navigation_stack.md) | `NavigationStack` | Controlled visit stack. |
| [`patterns/notification_stack`](patterns/notification_stack.md) | `NotificationStack` | Only the front notice is in the input and accessibility trees. |
| [`patterns/parallel_attempts`](patterns/parallel_attempts.md) | `ParallelAttempts` | Compare independent attempts without letting a visual winner change Host state. |
| [`patterns/permission_request`](patterns/permission_request.md) | `PermissionRequest` | PermissionRequest displays a capability request and an accepted grant, denial or expiry. |
| [`patterns/plan_vs_actual`](patterns/plan_vs_actual.md) | `PlanVsActual` | Compare a dependency plan to observed work without starting or modifying tasks. |
| [`patterns/profile_menu`](patterns/profile_menu.md) | `ProfileMenu` | A standalone account card; the caller owns navigation, sign-out and setup. |
| [`patterns/project_card`](patterns/project_card.md) | `ProjectCard` | An expandable project summary with a segmented progress ring. |
| [`patterns/prompt_library`](patterns/prompt_library.md) | `PromptLibrary` | PromptLibrary lets the user find templates, edit variables and propose a prompt. |
| [`patterns/rate_limit_planner`](patterns/rate_limit_planner.md) | `RateLimitPlanner` | A 24-hour load forecast with native drag previews and reversible Host proposals. |
| [`patterns/review_center`](patterns/review_center.md) | `ReviewCenter` | Prioritized review queue with versioned proposals and independently inspectable evidence. |
| [`patterns/root_cause_tree`](patterns/root_cause_tree.md) | `RootCauseTree` | Shows a Host-confirmed fault tree, evidence and scoped action proposals. |
| [`patterns/run_controls`](patterns/run_controls.md) | `RunControls` | RunControls presents a Host-owned run transcript, queue and draft editor. |
| [`patterns/running_now`](patterns/running_now.md) | `RunningNow` | A quiet trigger and a live, nonmodal task list. |
| [`patterns/scheduled_runs`](patterns/scheduled_runs.md) | `ScheduledRuns` | A capacity-checked schedule board. |
| [`patterns/section`](patterns/section.md) | `Section` | Section is a titled part of a region: a subtitle, an optional description and actions on the end side, then the content. |
| [`patterns/smart_recommendation`](patterns/smart_recommendation.md) | `SmartRecommendation` | A recommendation with a confidence signal, alternatives and reversible Host decisions. |
| [`patterns/stat`](patterns/stat.md) | `Stat` | Stat shows one figure: a label-voice label, the value with its unit, and an optional change against a reference. |
| [`patterns/tips_panel`](patterns/tips_panel.md) | `TipsPanel` | Present bounded onboarding tips and propose actions to the application. |
| [`patterns/tool_access_map`](patterns/tool_access_map.md) | `ToolAccessMap` | Inspect a Host's authorized tool snapshot and recorded calls; never invokes a tool. |
| [`patterns/tool_call_card`](patterns/tool_call_card.md) | `ToolCallCard` | One read-only tool call with status, arguments, result and optional retry. |
| [`patterns/tool_call_group`](patterns/tool_call_group.md) | `ToolCallGroup` | A compact summary and lazy disclosure for several read-only tool calls. |
| [`patterns/trace_waterfall`](patterns/trace_waterfall.md) | `TraceWaterfall` | Time-based execution tree with exact rollups, a shared clock and span details. |
| [`patterns/usage_meter`](patterns/usage_meter.md) | `UsageMeter` | Present accepted usage, period history and capped or unlimited plan limits. |
| [`patterns/writing_assistant`](patterns/writing_assistant.md) | `WritingAssistant` | WritingAssistant shows a controlled conversation, suggestion chips, an editor and optional dictation. |

## Motion

| Module | Export | Purpose |
|---|---|---|
| [`motion/animated_tabs`](motion/animated_tabs.md) | `AnimatedTabs` | AnimatedTabs is a controlled `Tabs` whose selection indicator slides to the selected tab. |
| [`motion/border_beam`](motion/border_beam.md) | `BorderBeam` | BorderBeam traces an accent stroke around a rectangle in an endless loop. |
| [`motion/marquee`](motion/marquee.md) | `Marquee` | Marquee scrolls its content horizontally in a loop. |
| [`motion/number_ticker`](motion/number_ticker.md) | `NumberTicker` | NumberTicker shows an integer and fades each new value in. |
| [`motion/orbit`](motion/orbit.md) | `Orbit` | Orbit spins a satellite dot around a center dot in an endless loop. |
| [`motion/particles`](motion/particles.md) | `Particles` | Bounded deterministic particle field. |
| [`motion/reorder_list`](motion/reorder_list.md) | `ReorderList` | ReorderList slides its rows to their new places when the caller reorders `items`. |
| [`motion/shared_layout_cards`](motion/shared_layout_cards.md) | `SharedLayoutCards` | SharedLayoutCards moves the selected card from the list to a detail area with a shared-layout animation. |
| [`motion/shimmer`](motion/shimmer.md) | `Shimmer` | Shimmer is a loading placeholder that a beam sweeps across in an endless loop. |
| [`motion/text_reveal`](motion/text_reveal.md) | `TextReveal` | Grapheme-safe native text reveal. |

## Charts

| Module | Export | Purpose |
|---|---|---|
| [`charts/bar_chart`](charts/bar_chart.md) | `BarChart` | Source-owned single-series Bar adapter over Chart. |
| [`charts/chart`](charts/chart.md) | `Chart` | Native composable visualization surface. |
| [`charts/line_chart`](charts/line_chart.md) | `LineChart` | Source-owned single-series Line adapter over Chart. |
| [`charts/map_chart`](charts/map_chart.md) | `MapChart` | Source-owned single-series choropleth adapter over Chart. |
| [`charts/pie_chart`](charts/pie_chart.md) | `PieChart` | Source-owned single-series Pie adapter over Chart. |
