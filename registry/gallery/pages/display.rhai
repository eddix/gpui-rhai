// Gallery spec pages: display components.

import "gallery/kit" as kit;
import "components/avatar" as avatar;
import "components/icon" as icon;
import "components/progress" as progress;
import "components/progress_tracker" as progress_tracker;
import "components/setup_guide" as setup_guide;
import "components/message_actions" as message_actions;
import "components/reaction_bar" as reaction_bar;
import "patterns/model_picker" as model_picker;
import "patterns/composer" as composer;
import "patterns/prompt_library" as prompt_library;
import "patterns/integration_picker" as integration_picker;
import "gallery/integration_settings_demo" as settings_demo;
import "gallery/comments_demo" as comments_demo;
import "gallery/writing_demo" as writing_demo;
import "gallery/review_demo" as review_demo;
import "gallery/decisions_demo" as decisions_demo;
import "gallery/gates_demo" as gates_demo;
import "gallery/decision_tree_demo" as tree_demo;
import "gallery/review_center_demo" as review_center_demo;
import "gallery/delegate_board_demo" as delegate_board_demo;
import "gallery/agent_queue_demo" as agent_queue_demo;
import "gallery/agent_lanes_demo" as agent_lanes_demo;
import "gallery/agent_handoff_demo" as agent_handoff_demo;
import "gallery/agent_inbox_demo" as agent_inbox_demo;
import "gallery/agent_plan_demo" as agent_plan_demo;
import "gallery/agent_checkpoints_demo" as agent_checkpoints_demo;
import "gallery/agent_trail_demo" as agent_trail_demo;
import "gallery/agent_tree_demo" as agent_tree_demo;
import "gallery/agent_graph_demo" as agent_graph_demo;
import "gallery/agent_fleet_demo" as agent_fleet_demo;
import "gallery/agent_monitor_demo" as agent_monitor_demo;
import "gallery/agent_steering_demo" as agent_steering_demo;
import "gallery/agent_recap_demo" as agent_recap_demo;
import "gallery/agent_findings_demo" as agent_findings_demo;
import "gallery/agent_ledger_demo" as agent_ledger_demo;
import "gallery/agent_cursors_demo" as agent_cursors_demo;
import "gallery/compact_breakdown_demo" as compact_breakdown_demo;
import "gallery/usage_meter_demo" as usage_meter_demo;
import "gallery/product_cards_demo" as product_cards_demo;
import "gallery/comparison_cards_demo" as comparison_cards;
import "gallery/communication_cards_demo" as communication_cards;
import "gallery/kpi_chart_demo" as kpi_demo;
import "gallery/breakdown_chart_demo" as breakdown_demo;
import "gallery/running_now_demo" as running_demo;
import "gallery/scheduled_runs_demo" as schedule_demo;
import "gallery/tool_calls_demo" as tool_calls_demo;
import "gallery/context_window_demo" as context_window_demo;
import "gallery/context_map_demo" as context_map_demo;
import "gallery/profile_menu_demo" as profile_menu_demo;
import "gallery/email_draft_demo" as email_draft_demo;
import "gallery/diff_timeline_demo" as diff_timeline_demo;
import "gallery/cost_waterfall_demo" as cost_waterfall_demo;
import "gallery/parallel_attempts_demo" as parallel_attempts_demo;
import "gallery/fan_out_grid_demo" as fan_out_grid_demo;
import "gallery/billing_schedule_demo" as billing_schedule_demo;
import "gallery/plan_vs_actual_demo" as plan_vs_actual_demo;
import "gallery/fallback_chain_demo" as fallback_chain_demo;
import "gallery/memory_panel_demo" as memory_panel_demo;
import "gallery/tool_access_map_demo" as tool_access_map_demo;
import "gallery/trace_waterfall_demo" as trace_waterfall_demo;
import "gallery/smart_recommendation_demo" as smart_recommendation_demo;
import "gallery/rate_limit_planner_demo" as rate_limit_planner_demo;
import "gallery/root_cause_tree_demo" as root_cause_tree_demo;
import "gallery/judge_bracket_demo" as judge_bracket_demo;
import "gallery/collaboration_stream_demo" as collaboration_stream_demo;
import "gallery/conflict_radar_demo" as conflict_radar_demo;
import "components/cited_answer" as cited_answer;
import "components/ai_answer" as ai_answer;
import "components/thinking_loader" as thinking_loader;
import "components/thinking_state" as thinking_state;
import "components/skeleton" as skeleton;
import "components/spinner" as spinner;
import "components/code_viewer" as code_viewer;
import "components/diff_viewer" as diff_viewer;
import "components/button" as button;

fn pages() {
    [
        #{ id: "avatar", title: "Avatar", group: "Display" },
        #{ id: "icon", title: "Icon", group: "Display" },
        #{ id: "progress", title: "Progress", group: "Display" },
        #{ id: "progress_tracker", title: "ProgressTracker", group: "Agents" },
        #{ id: "setup_guide", title: "SetupGuide", group: "Agents" },
        #{ id: "message_actions", title: "MessageActions", group: "Agents" },
        #{ id: "reaction_bar", title: "ReactionBar", group: "Agents" },
        #{ id: "cited_answer", title: "CitedAnswer", group: "Agents" },
        #{ id: "ai_answer", title: "AIAnswer", group: "Agents" },
        #{ id: "thinking_loader", title: "ThinkingLoader", group: "Agents" },
        #{ id: "thinking_state", title: "ThinkingState", group: "Agents" },
        #{ id: "model_picker", title: "ModelPicker", group: "Agents" },
        #{ id: "prompt_library", title: "PromptLibrary", group: "Agents" },
        #{ id: "integration_picker", title: "IntegrationPicker", group: "Agents" },
        #{ id: "integration_settings", title: "IntegrationSettings", group: "Agents" },
        #{ id: "comment_thread", title: "CommentThread", group: "Agents" },
        #{ id: "comments_panel", title: "CommentsPanel", group: "Agents" },
        #{ id: "writing_assistant", title: "WritingAssistant", group: "Agents" },
        #{ id: "approval_card", title: "ApprovalCard", group: "Agents" },
        #{ id: "permission_request", title: "PermissionRequest", group: "Agents" },
        #{ id: "decision_card", title: "DecisionCard", group: "Agents" },
        #{ id: "decision_tree", title: "DecisionTree", group: "Agents" },
        #{ id: "review_center", title: "ReviewCenter", group: "Agents" },
        #{ id: "delegate_board", title: "DelegateBoard", group: "Agents" },
        #{ id: "agent_queue", title: "AgentQueue", group: "Agents" },
        #{ id: "agent_lanes", title: "AgentLanes", group: "Agents" },
        #{ id: "agent_handoff", title: "AgentHandoff", group: "Agents" },
        #{ id: "agent_inbox", title: "AgentInbox", group: "Agents" },
        #{ id: "agent_plan", title: "AgentPlan", group: "Agents" },
        #{ id: "agent_checkpoints", title: "AgentCheckpoints", group: "Agents" },
        #{ id: "agent_trail", title: "AgentTrail", group: "Agents" },
        #{ id: "agent_tree", title: "AgentTree", group: "Agents" },
        #{ id: "agent_graph", title: "AgentGraph", group: "Agents" },
        #{ id: "agent_fleet", title: "AgentFleet", group: "Agents" },
        #{ id: "agent_monitor", title: "AgentMonitor", group: "Agents" },
        #{ id: "agent_steering", title: "AgentSteering", group: "Agents" },
        #{ id: "agent_recap", title: "AgentRecap", group: "Agents" },
        #{ id: "agent_findings", title: "AgentFindings", group: "Agents" },
        #{ id: "agent_ledger", title: "AgentLedger", group: "Agents" },
        #{ id: "agent_cursors", title: "AgentCursors", group: "Agents" },
        #{ id: "compact_breakdown", title: "CompactBreakdown", group: "Agents" },
        #{ id: "usage_meter", title: "UsageMeter", group: "Agents" },
        #{ id: "change_summary", title: "ChangeSummary", group: "Agents" },
        #{ id: "tips_panel", title: "TipsPanel", group: "Agents" },
        #{ id: "project_card", title: "ProjectCard", group: "Agents" },
        #{ id: "experiment_card", title: "ExperimentCard", group: "Agents" },
        #{ id: "notification_stack", title: "NotificationStack", group: "Agents" },
        #{ id: "away_summary", title: "AwaySummary", group: "Agents" },
        #{ id: "kpi_chart", title: "KpiChart", group: "Agents" },
        #{ id: "breakdown_chart", title: "BreakdownChart", group: "Agents" },
        #{ id: "running_now", title: "RunningNow", group: "Agents" },
        #{ id: "scheduled_runs", title: "ScheduledRuns", group: "Agents" },
        #{ id: "tool_call_card", title: "ToolCallCard", group: "Agents" },
        #{ id: "tool_call_group", title: "ToolCallGroup", group: "Agents" },
        #{ id: "context_window", title: "ContextWindow", group: "Agents" },
        #{ id: "context_map", title: "ContextMap", group: "Agents" },
        #{ id: "profile_menu", title: "ProfileMenu", group: "Agents" },
        #{ id: "email_draft", title: "EmailDraft", group: "Agents" },
        #{ id: "diff_timeline", title: "DiffTimeline", group: "Agents" },
        #{ id: "cost_waterfall", title: "CostWaterfall", group: "Agents" },
        #{ id: "parallel_attempts", title: "ParallelAttempts", group: "Agents" },
        #{ id: "fan_out_grid", title: "FanOutGrid", group: "Agents" },
        #{ id: "billing_schedule", title: "BillingSchedule", group: "Agents" },
        #{ id: "plan_vs_actual", title: "PlanVsActual", group: "Agents" },
        #{ id: "fallback_chain", title: "FallbackChain", group: "Agents" },
        #{ id: "memory_panel", title: "MemoryPanel", group: "Agents" },
        #{ id: "tool_access_map", title: "ToolAccessMap", group: "Agents" },
        #{ id: "trace_waterfall", title: "TraceWaterfall", group: "Agents" },
        #{ id: "smart_recommendation", title: "SmartRecommendation", group: "Agents" },
        #{ id: "rate_limit_planner", title: "RateLimitPlanner", group: "Agents" },
        #{ id: "root_cause_tree", title: "RootCauseTree", group: "Agents" },
        #{ id: "judge_bracket", title: "JudgeBracket", group: "Agents" },
        #{ id: "collaboration_stream", title: "CollaborationStream", group: "Agents" },
        #{ id: "conflict_radar", title: "ConflictRadar", group: "Agents" },
        #{ id: "decision_inbox", title: "DecisionInbox", group: "Agents" },
        #{ id: "approval_gates", title: "ApprovalGates", group: "Agents" },
        #{ id: "autonomy_dial", title: "AutonomyDial", group: "Agents" },
        #{ id: "skeleton", title: "Skeleton", group: "Display" },
        #{ id: "spinner", title: "Spinner", group: "Display" },
        #{ id: "code_viewer", title: "CodeViewer", group: "Display" },
        #{ id: "diff_viewer", title: "DiffViewer", group: "Display" },
    ]
}

fn view(ctx, h, v, id) {
    if id == "avatar" { avatar_page(h, v) }
    else if id == "icon" { icon_page(h, v) }
    else if id == "progress" { progress_page(h, v) }
    else if id == "progress_tracker" { tracker_page(h, v) }
    else if id == "setup_guide" { setup_page(h, v) }
    else if id == "message_actions" { message_actions_page(h,v) }
    else if id == "reaction_bar" { reaction_bar_page(h,v) }
    else if id == "cited_answer" { cited_answer_page(h,v) }
    else if id == "ai_answer" { ai_answer_page(h,v) }
    else if id == "thinking_loader" { thinking_loader_page(h,v) }
    else if id == "thinking_state" { thinking_state_page(h,v) }
    else if id == "model_picker" { model_picker_page(h,v) }
    else if id == "prompt_library" { prompt_library_page(h,v) }
    else if id == "integration_picker" { integration_picker_page(h,v) }
    else if id == "integration_settings" { integration_settings_page(h,v) }
    else if id == "comment_thread" { comment_workflow(h,v,id,"thread","CommentThread","Reply to a review and inspect assistant details.") }
    else if id == "comments_panel" { comment_workflow(h,v,id,"panel","CommentsPanel","Filter a release feed, resolve comments and propose edits.") }
    else if id == "writing_assistant" { kit::page(h,v,id,"WritingAssistant","Try a suggestion, type a prompt, or review simulated dictation.",[
        kit::sec("Writing desk",writing_demo::WritingDemo(#{key:"writing-desk"})),
    ]) }
    else if id == "approval_card" { kit::page(h,v,id,"ApprovalCard","Review a request and propose a Host-owned decision.",[
        kit::sec("Release review",review_demo::ReviewDemo(#{key:"review-demo",mode:"approval"})).audit_allow(["text-edge-misaligned"]),
    ]) }
    else if id == "permission_request" { kit::page(h,v,id,"PermissionRequest","Choose a duration and inspect the accepted grant.",[
        kit::sec("Capability request",review_demo::ReviewDemo(#{key:"permission-demo",mode:"permission"})),
    ]) }
    else if id == "decision_card" { kit::page(h,v,id,"DecisionCard","Page through choices, previews and approvals; a proposal is not a receipt.",[
        kit::sec("Decisions",decisions_demo::DecisionsDemo(#{key:"decision-demo",mode:"card"})),
    ]) }
    else if id == "decision_tree" { kit::page(h,v,id,"DecisionTree","Read what each path would change, inspect uncertain evidence, then choose a branch or write an instruction.",[
        kit::sec("Pending decisions",tree_demo::DecisionTreeDemo(#{key:"tree-demo"})),
    ]) }
    else if id == "review_center" { kit::page(h,v,id,"ReviewCenter","Prioritize work by risk and wait, read the proposed changes, and review an item with a controlled receipt.",[
        kit::sec("Review queue",review_center_demo::ReviewCenterDemo(#{key:"review-demo"})),
    ]) }
    else if id == "delegate_board" { kit::page(h,v,id,"DelegateBoard","Move a work item to an agent queue, preview a handoff, or make an automatic batch proposal.",[
        kit::sec("Sample board",delegate_board_demo::DelegateBoardDemo(#{key:"delegate-demo"})),
    ]) }
    else if id == "agent_queue" { kit::page(h,v,id,"AgentQueue","Inspect the backlog, current workers and pressure history on the same clock.",[
        kit::sec("Sample queue",agent_queue_demo::AgentQueueDemo(#{key:"queue-demo"})),
    ]) }
    else if id == "agent_lanes" { kit::page(h,v,id,"AgentLanes","Inspect timed activity, overlapping tasks and hand-offs between agents.",[
        kit::sec("Sample activity",agent_lanes_demo::AgentLanesDemo(#{key:"lanes-demo"})),
    ]) }
    else if id == "agent_handoff" { kit::page(h,v,id,"AgentHandoff","Inspect what was passed between agents, restore an omitted item, or propose moving a task.",[
        kit::sec("Sample relay",agent_handoff_demo::AgentHandoffDemo(#{key:"relay-demo"})),
    ]) }
    else if id == "agent_inbox" { kit::page(h,v,id,"AgentInbox","Browse agent tasks by state, inspect their recent steps and send proposals to the Host.",[
        kit::sec("Sample inbox",agent_inbox_demo::AgentInboxDemo(#{key:"inbox-demo"})),
    ]) }
    else if id == "agent_plan" { kit::page(h,v,id,"AgentPlan","Review an editable plan, then follow accepted execution and recovery from a failed step.",[
        kit::sec("Sample plan",agent_plan_demo::AgentPlanDemo(#{key:"plan-demo"})),
    ]) }
    else if id == "agent_checkpoints" { kit::page(h,v,id,"AgentCheckpoints","Follow a run across attempts, then ask the Host to branch from an earlier checkpoint.",[
        kit::sec("Sample history",agent_checkpoints_demo::AgentCheckpointsDemo(#{key:"history-demo"})),
    ]) }
    else if id == "agent_trail" { kit::page(h,v,id,"AgentTrail","Follow current execution, see the detailed substeps, and review a paused decision.",[
        kit::sec("Example run",agent_trail_demo::AgentTrailDemo(#{key:"trail-demo"})),
    ]) }
    else if id == "agent_tree" { kit::page(h,v,id,"AgentTree","See who dispatched each agent, fold branches and review blocked or failed work.",[
        kit::sec("Sample dispatch",agent_tree_demo::AgentTreeDemo(#{key:"tree-demo"})),
    ]) }
    else if id == "agent_graph" { kit::page(h,v,id,"AgentGraph","Follow requests and replies between agents, tools and people, then inspect a node's recent activity.",[
        kit::sec("Sample communication",agent_graph_demo::AgentGraphDemo(#{key:"graph-demo"})),
    ]) }
    else if id == "agent_fleet" { kit::page(h,v,id,"AgentFleet","Browse lead agents, inspect their branches and review tasks that need a decision.",[
        kit::sec("Sample fleet",agent_fleet_demo::AgentFleetDemo(#{key:"fleet-demo"})),
    ]) }
    else if id == "agent_monitor" { kit::page(h,v,id,"AgentMonitor","Watch one run, inspect its timed steps, and send decisions to the Host.",[
        kit::sec("Sample run",agent_monitor_demo::AgentMonitorDemo(#{key:"monitor-demo"})),
    ]) }
    else if id == "agent_steering" { kit::page(h,v,id,"AgentSteering","Queue notes for a step boundary or propose applying one immediately. Watch the accepted note move into the run.",[
        kit::sec("Sample steering",agent_steering_demo::AgentSteeringDemo(#{key:"steering-demo"})),
    ]) }
    else if id == "agent_recap" { kit::page(h,v,id,"AgentRecap","Replay the activity while you were away. Browse timeline peaks, review work, retry failures, or propose Undo.",[
        kit::sec("Sample recap",agent_recap_demo::AgentRecapDemo(#{key:"recap-demo"})),
    ]) }
    else if id == "agent_findings" { kit::page(h,v,id,"AgentFindings","Watch checks, filter the live feed by agent, and open a finding by its stable ID.",[
        kit::sec("Live verification",agent_findings_demo::AgentFindingsDemo(#{key:"findings-demo"})),
    ]) }
    else if id == "agent_ledger" { kit::page(h,v,id,"AgentLedger","Inspect a run's changes, preview cascading Undo, and restore one accepted batch without changing the others.",[
        kit::sec("Release ledger",agent_ledger_demo::AgentLedgerDemo(#{key:"ledger-demo"})),
    ]) }
    else if id == "agent_cursors" { kit::page(h,v,id,"AgentCursors","Follow a reviewer, inspect inline suggestions, discuss a comment, and switch between a document and its review table.",[
        kit::sec("Collaborative document review",agent_cursors_demo::AgentCursorsDemo(#{key:"cursors-demo"})),
    ]) }
    else { product_page(h,v,id) }
}

// A dispatch table keeps additions from growing a deeply nested Rhai expression.
fn product_page(h,v,id) {
    switch id {
    "compact_breakdown" => kit::page(h,v,id,"CompactBreakdown","Compare accepted spending across periods. Hover a segment or focus a category to inspect its share.",[
        kit::sec("Infrastructure spending",compact_breakdown_demo::CompactBreakdownDemo(#{key:"compact-breakdown-demo"})),
    ]),
    "usage_meter" => kit::page(h,v,id,"UsageMeter","Inspect spending, daily activity and plan quotas. Try a period request or the upgrade action.",[
        kit::sec("Usage and plan limits",usage_meter_demo::UsageMeterDemo(#{key:"usage-meter-demo"})),
    ]),
    "change_summary" => kit::page(h,v,id,"ChangeSummary","Compare changes. Direction and outcome are independent, and the narrow card stacks each detail.",[
        kit::sec("Metrics",product_cards_demo::ProductCardsDemo(#{key:"change-summary-demo",mode:"changes"})),
    ]),
    "tips_panel" => kit::page(h,v,id,"TipsPanel","Try accepted or refused actions inside a responsive checklist. The heading and close action stay fixed when its list scrolls.",[
        kit::sec("Workspace onboarding",product_cards_demo::ProductCardsDemo(#{key:"tips-panel-demo",mode:"tips"})),
    ]),
    "project_card" => kit::page(h,v,id,"ProjectCard","Open a project, inspect subtasks by person or ring segment, and propose completion or a new status.",[
        kit::sec("Release project",comparison_cards::ComparisonCardsDemo(#{key:"project-card-demo",mode:"project"})),
    ]),
    "experiment_card" => kit::page(h,v,id,"ExperimentCard","Compare metrics, inspect missing values, and nominate a candidate before a separate confirmation.",[
        kit::sec("Experiment comparison",comparison_cards::ComparisonCardsDemo(#{key:"experiment-card-demo",mode:"experiment"})),
    ]),
    "notification_stack" => kit::page(h,v,id,"NotificationStack","Only the front card is interactive. Try declining a dismissal or moving through notices of different heights.",[
        kit::sec("Workspace notices",communication_cards::CommunicationCardsDemo(#{key:"notice-demo",mode:"notices"})),
    ]),
    "away_summary" => kit::page(h,v,id,"AwaySummary","Catch up on work, show more, or acknowledge the summary without losing its position.",[
        kit::sec("While you were away",communication_cards::CommunicationCardsDemo(#{key:"away-demo",mode:"away"})),
    ]),
    "kpi_chart" => kit::page(h,v,id,"KpiChart","Inspect an observation by pointer or keyboard, compare series and try gaps, a smaller card or a new snapshot.",[
        kit::sec("Trend and comparison",kpi_demo::KpiChartDemo(#{key:"kpi-demo"})),
    ]),
    "breakdown_chart" => kit::page(h,v,id,"BreakdownChart","Pick a measure and grouping, then compare the same totals in the plot and table. Hover a row or use arrows inside the chart.",[
        kit::sec("Workload by area",breakdown_demo::BreakdownChartDemo(#{key:"breakdown-demo"})),
    ]),
    "running_now" => kit::page(h,v,id,"RunningNow","Inspect background tasks, navigate the popover with keys, and request a transition without changing the task before acceptance.",[
        kit::sec("Background tasks",running_demo::RunningNowDemo(#{key:"running-demo"})),
    ]),
    "scheduled_runs" => kit::page(h,v,id,"ScheduledRuns","Schedule tasks automatically or place them using the drag grip or keyboard. The calendar updates only after the Host accepts.",[
        kit::sec("Sessions and capacity",schedule_demo::ScheduledRunsDemo(#{key:"schedule-demo"})),
    ]),
    "tool_call_card" => kit::page(h,v,id,"ToolCallCard","Expand to inspect or copy arguments and results. Long output stays in a bounded viewer, and retry waits for the Host.",[
        kit::sec("One tool call",tool_calls_demo::ToolCallsDemo(#{key:"tool-demo",mode:"card"})),
    ]),
    "tool_call_group" => kit::page(h,v,id,"ToolCallGroup","Open a batch to inspect its calls. The collapsed summary follows the accepted statuses and shared clock.",[
        kit::sec("Tool calls in one turn",tool_calls_demo::ToolCallsDemo(#{key:"group-demo",mode:"group"})),
    ]),
    "context_window" => kit::page(h,v,id,"ContextWindow","Inspect the grid of accepted tokens and remove or compact context only through a Host request.",[
        kit::sec("Context and remaining room",context_window_demo::ContextWindowDemo(#{key:"window-demo"})),
    ]),
    "context_map" => kit::page(h,v,id,"ContextMap","Inspect a token treemap, switch agents, and compare the accepted snapshots on either side of a compaction.",[
        kit::sec("Context allocation",context_map_demo::ContextMapDemo(#{key:"map-demo"})),
    ]),
    "profile_menu" => kit::page(h,v,id,"ProfileMenu","Inspect the standalone account card, navigate its action list and try a Host-approved setup step.",[
        kit::sec("Account and setup",profile_menu_demo::ProfileMenuDemo(#{key:"profile-demo"})),
    ]),
    "email_draft" => kit::page(h,v,id,"EmailDraft","Review a recipient, an editable body and the Host-controlled receipt. No mail is sent by this example.",[
        kit::sec("Renewal message",email_draft_demo::EmailDraftDemo(#{key:"email-draft-demo"})),
    ]),
    "diff_timeline" => kit::page(h,v,id,"DiffTimeline","Play a typed edit ledger and inspect each change by pointer or keyboard. Future edits set both scales without exposing their content.",[
        kit::sec("Patch activity",diff_timeline_demo::DiffTimelineDemo(#{key:"diff-activity-demo"})),
    ]),
    "cost_waterfall" => kit::page(h,v,id,"CostWaterfall","Inspect exact cost and token rollups, folded sub-agents, a credit crossing zero, and a Host-controlled measure switch.",[
        kit::sec("Run ledger",cost_waterfall_demo::CostWaterfallDemo(#{key:"cost-ledger-demo"})),
    ]),
    "parallel_attempts" => kit::page(h,v,id,"ParallelAttempts","Compare live trials, switch to final scores, and exercise an accepted or rejected attempt selection with pointer or keyboard.",[
        kit::sec("Checkout race",parallel_attempts_demo::ParallelAttemptsDemo(#{key:"checkout-race-demo"})),
    ]),
    "fan_out_grid" => kit::page(h,v,id,"FanOutGrid","Explore a dense task field by mouse or keyboard. Pin a task, highlight a status without moving tasks, or ask the Host to retry the failed subset.",[
        kit::sec("Parallel indexing",fan_out_grid_demo::FanOutGridDemo(#{key:"fan-out-demo"})),
    ]),
    "billing_schedule" => kit::page(h,v,id,"BillingSchedule","Edit a local phase, or exercise a Host-approved quote with exact totals and native date and number fields.",[
        kit::sec("Scheduled charges",billing_schedule_demo::BillingScheduleDemo(#{key:"billing-schedule-demo"})),
    ]),
    "plan_vs_actual" => kit::page(h,v,id,"PlanVsActual","Compare the original schedule, observed work and forecast. Inspect direct dependencies or advance the Host clock to see the waiting steps move.",[
        kit::sec("Project roll-out",plan_vs_actual_demo::PlanVsActualDemo(#{key:"plan-vs-actual-demo"})),
    ]),
    "fallback_chain" => kit::page(h,v,id,"FallbackChain","Inspect a request, the ordered model attempts and their response. Advance the clock, accept an answer, or ask the Host to retry a failed chain.",[
        kit::sec("Answer route",fallback_chain_demo::FallbackChainDemo(#{key:"fallback-chain-demo"})),
    ]),
    "memory_panel" => kit::page(h,v,id,"MemoryPanel","Search, edit or pause saved memories. The Host decides what to persist, and only a confirmed removal offers undo.",[
        kit::sec("Workspace memory",memory_panel_demo::MemoryPanelDemo(#{key:"memory-panel-demo"})),
    ]),
    "tool_access_map" => kit::page(h,v,id,"ToolAccessMap","Browse agents and their available tools, open recorded calls, or ask the Host to replay a completed call with new input.",[
        kit::sec("Document workflow",tool_access_map_demo::ToolAccessMapDemo(#{key:"tool-map-demo"})),
    ]),
    "trace_waterfall" => kit::page(h,v,id,"TraceWaterfall","Read concurrent spans on the same timeline, fold nested work, and inspect the duration, cost, and recorded input or output of a span.",[
        kit::sec("Execution trace",trace_waterfall_demo::TraceWaterfallDemo(#{key:"trace-demo"})),
    ]),
    "smart_recommendation" => kit::page(h,v,id,"SmartRecommendation","Compare an explanation and confidence signal with alternatives. Ask the Host to accept or undo without executing the suggested change.",[
        kit::sec("Sync strategy",smart_recommendation_demo::SmartRecommendationDemo(#{key:"recommendation-demo"})),
    ]),
    "rate_limit_planner" => kit::page(h,v,id,"RateLimitPlanner","Compare planned and measured load across a day. Drag a future run, edit its forecast, or ask the Host to spread the plan; this demo never runs a job.",[
        kit::sec("Daily load",rate_limit_planner_demo::RateLimitPlannerDemo(#{key:"rate-plan-demo"})),
    ]),
    "root_cause_tree" => kit::page(h,v,id,"RootCauseTree","Follow accepted evidence through AND and OR gates. Select a hypothesis to inspect it, or ask the Host to act on a confirmed root cause.",[
        kit::sec("Failed publish investigation",root_cause_tree_demo::RootCauseTreeDemo(#{key:"root-cause-demo"})),
    ]),
    "judge_bracket" => kit::page(h,v,id,"JudgeBracket","Compare drafts in an elimination bracket. Explore byes, revised decisions, and the Host's final choice.",[
        kit::sec("Draft evaluation",judge_bracket_demo::JudgeBracketDemo(#{key:"judge-bracket-demo"})),
    ]),
    "collaboration_stream" => kit::page(h,v,id,"CollaborationStream","Follow a conversation, revisit earlier messages, and preview attachments. Replies are accepted and appended by the Host.",[
        kit::sec("Editorial review",collaboration_stream_demo::CollaborationStreamDemo(#{key:"collaboration-stream-demo"})),
    ]),
    "conflict_radar" => kit::page(h,v,id,"ConflictRadar","Review the exact lines where branch edits overlap. The Host accepts each choice and receives the final merge proposal.",[
        kit::sec("Release work",conflict_radar_demo::ConflictRadarDemo(#{key:"conflict-radar-demo"})),
    ]),
    "decision_inbox" => kit::page(h,v,id,"DecisionInbox","Review decisions, send a quick answer and wait for the Host's receipt.",[
        kit::sec("Waiting for review",decisions_demo::DecisionsDemo(#{key:"decision-demo",mode:"inbox"})),
    ]),
    "approval_gates" => kit::page(h,v,id,"ApprovalGates","Inspect a run, open the evidence and review decisions made by the Host.",[
        kit::sec("Run review",gates_demo::GatesDemo(#{key:"gates-demo",mode:"gates"})),
    ]),
    "autonomy_dial" => kit::page(h,v,id,"AutonomyDial","See what the current policy asks about; change the level or pin one action.",[
        kit::sec("Confirmation policy",gates_demo::GatesDemo(#{key:"dial-demo",mode:"dial"})),
    ]),
    "skeleton" => skeleton_page(h, v),
    "spinner" => spinner_page(h, v),
    "code_viewer" => code_viewer_page(h, v),
    "diff_viewer" => diff_viewer_page(h, v),
    _ => (),
    }
}

fn prompt_library_page(h,v){
    let templates=[
        #{id:"review/7",title:"Summarize rollout",category:"Operations",body:"Explain the rollout to {audience}. Keep the scope to {scope}.",
            variables:#{audience:#{description:"Who will read the summary?"},scope:#{description:"What should the summary cover?","default":"edge hosts"}}},
        #{id:"notes",title:"Write meeting notes",category:"Team",tone:"violet",body:"Draft notes from {source}. End with decisions."},
        #{id:"review",title:"Review a change",category:"Engineering",tone:"green",body:"Summarize the change. List evidence, risks, and a rollback plan."},
    ];
    let library=prompt_library::PromptLibrary(#{key:"library",templates:templates,value:kit::val(v,"catalog.template","review/7"),
        on_select:kit::store(h,"catalog.template"),on_use:h.catalog_use});
    let draft=composer::Composer(#{key:"generated-draft",label:"Prompt draft",value:kit::val(v,"catalog.draft",""),
        placeholder:"Use a template to populate this draft",can_submit:false,on_change:kit::store(h,"catalog.draft")});
    kit::page(h,v,"prompt_library","PromptLibrary","Fill a template and put the accepted text into a draft.",[
        kit::sec("Template library",library.with_style(style().max_width(px(730)))),
        kit::sec("Accepted draft",draft.with_style(style().max_width(px(730)))),
    ])
}
fn integration_picker_page(h,v){
    let entries=[
        #{id:"notes/7",name:"Team Notes",domain:"notes.workspace",category:"Knowledge",tone:"violet",
            description:"Find relevant team notes and link them from your workspace.",features:["Find shared pages","Link source references"]},
        #{id:"observability",name:"Observability",domain:"logs.workspace",category:"Operations",tone:"blue",
            description:"Read service health and deployment summaries.",features:["Inspect release events","Compare health checks"]},
        #{id:"archive/7",name:"Secure Archive",domain:"archive.workspace",category:"Records",tone:"amber",
            description:"Browse retained artifacts. The demo Host declines this connection.",features:["Catalog audit artifacts"]},
    ];
    let picker=integration_picker::IntegrationPicker(#{key:"connections",integrations:entries,value:kit::val(v,"catalog.integration","observability"),
        connected:kit::val(v,"catalog.connections",["notes/7"]),app_name:"Workbench",on_select:kit::store(h,"catalog.integration"),
        on_toggle:h.catalog_toggle,on_explore:kit::store(h,"catalog.explored"),
        on_close:kit::put(h,"catalog.connection_note","Close requested."),on_back:kit::put(h,"catalog.connection_note","Back requested.")});
    let explored=kit::val(v,"catalog.explored","");
    for entry in entries{
        if entry.id==explored{
            return kit::page(h,v,"integration_picker","IntegrationPicker","Open the selected integration's settings.",[
                button::Button(#{key:"back",text:"Back to integrations",size:"sm",variant:"secondary",
                    on_click:kit::put(h,"catalog.explored","")}),
                kit::sec("Integration settings",settings_demo::SettingsDemo(#{key:"entry-settings",integration:entry.id,name:entry.name})),
            ]);
        }
    }
    kit::page(h,v,"integration_picker","IntegrationPicker","Inspect availability and propose a Host-owned connection change.",[
        kit::sec("Integration directory",picker.with_style(style().max_width(px(680)))),
        kit::note(kit::val(v,"catalog.connection_note","")),
    ])
}

fn integration_settings_page(h,v){
    kit::page(h,v,"integration_settings","IntegrationSettings","Edit versioned settings and request separate Host credential actions.",[
        kit::sec("Storefront connection",settings_demo::SettingsDemo(#{key:"store-settings",integration:"storefront",name:"Storefront"})),
    ])
}

fn comment_workflow(h,v,id,mode,title,description){
    kit::page(h,v,id,title,description,[
        kit::sec("Release review",comments_demo::CommentsDemo(#{key:"review",mode:mode})),
    ])
}

fn answer_sources(){
    [#{id:"reports/trace",site:"Build log",title:"Release checks",url:"https://example.com/build",
        date:"Oct 14, 2026",quote:"The first attempt failed its health check; the retry retained the previous configuration."},
     #{id:"rollback",site:"Runbook",title:"Retry protocol",url:"https://example.com/runbook",
        quote:"Inspect the affected host and retry that host only."}]
}
fn thinking_loader_page(h,v){
    let phase=kit::val(v,"thinking.phase",0);let done=kit::val(v,"thinking.done",false);
    let labels=["Thinking","Searching release logs and previous runs","Reading sources","Writing response"];
    let loader=thinking_loader::ThinkingLoader(#{key:"controlled-think",phases:labels,phase:phase,done:done,elapsed_ms:12000});
    kit::page(h,v,"thinking_loader","ThinkingLoader","A readable activity indicator whose phases can follow the Host.",[
        kit::sec("Controlled",column([loader,row([
            button::Button(#{key:"next",text:"Next phase",variant:"secondary",on_click:kit::put(h,"thinking.phase",(phase+1)%labels.len)}),
            button::Button(#{key:"complete",text:if done{"Resume preview"}else{"Finish work"},variant:"secondary",
                on_click:kit::put(h,"thinking.done",!done)}),
        ]).with_style(style().gap(px(6)).flex_wrap())]).with_style(style().gap(px(8)).min_width(px(0)))),
        kit::sec("Constrained",row([thinking_loader::ThinkingLoader(#{key:"narrow-think",phases:labels,phase:1,size:"sm"})])
            .with_style(style().width(px(165)).max_width(relative(1.0)))),
        kit::sec("Local preview",thinking_loader::ThinkingLoader(#{key:"preview-think",phases:labels,autoplay:true,repeat:true,
            interval_ms:2200,variant:"plain"})),
    ])
}
fn thinking_state_page(h,v){
    let count=kit::val(v,"work.count",2);let done=kit::val(v,"work.done",false);
    let entries=[
        #{id:"draft",label:"Review the request",state:"done"},
        #{id:"sources",kind:"search",label:"Checked release notes",active_label:"Reading release notes",
            state:if count==2&&!done{"running"}else{"done"},sources:[
                #{id:"build",label:"Build report",href:"https://example.com/build",hint:"Current build report"},
                #{id:"retry",label:"Release runbook",href:"https://example.com/runbook",extra:true},
                #{id:"log",label:"Archived logs",href:"https://example.com/archive",extra:true},
            ]},
        #{id:"compose",label:"Summarize the result",state:if done{"done"}else{"running"}},
    ];
    let active=[];for i in 0..count{active.push(entries[i]);}
    let linked=kit::val(v,"work.source",());
    kit::page(h,v,"thinking_state","ThinkingState","Review completed steps and inspect the evidence behind a search.",[
        kit::sec("Work summary",thinking_state::ThinkingState(#{key:"release-work",
            status:if done{"done"}else{"working"},elapsed_ms:if done{12000}else{4200},
            steps:active,on_open_source:kit::store(h,"work.source")})),
        kit::sec("Host updates",row([
            button::Button(#{key:"step",text:"Publish next step",variant:"secondary",disabled:count==3,on_click:kit::put(h,"work.count",3)}),
            button::Button(#{key:"done",text:if done{"Resume work"}else{"Complete work"},variant:"secondary",
                on_click:kit::put(h,"work.done",!done)}),
        ]).with_style(style().gap(px(6)).flex_wrap())),
        kit::note(if linked==(){""}else{"Source requested (not opened): "+linked.href}),
    ])
}
fn answer_note(v){
    let result=kit::val(v,"answer.open",());
    if result!=(){kit::note("Source requested (not opened): "+result.url)}else{kit::note("")}
}
fn cited_answer_page(h,v){
    let segments=[
        #{text:"The initial deploy failed its health check (边缘节点). ",cite:["reports/trace"]},
        #{text:"The retry retained the previous configuration. ",cite:["reports/trace","rollback"]},
        #{text:"Inspect the failed host before retrying.",paragraph:true,cite:["rollback"]},
    ];
    kit::page(h,v,"cited_answer","CitedAnswer","Inspect the exact source behind an inline claim.",[
        kit::sec("Evidence",cited_answer::CitedAnswer(#{key:"rollout-evidence",segments:segments,sources:answer_sources(),
            pages_read:8,on_open_source:kit::store(h,"answer.open"),action_label:"Request follow-up",
            on_action:kit::put(h,"answer.follow","What changed during the retry?")})
            .with_style(style().width(px(630)).max_width(relative(1.0)))),
        kit::note(kit::val(v,"answer.follow","")),answer_note(v)
    ])
}
fn ai_answer_page(h,v){
    let status=kit::val(v,"answer.phase","done");
    let source=if status=="reading"{()}
        else if status=="writing"||status=="stopped"{"First, inspect the failed probe."}
        else{"Inspect the failed probe [1](gpui-cite:1).\n\nRetry only the affected host [2](gpui-cite:2)."};
    kit::page(h,v,"ai_answer","AIAnswer","A reply that keeps its content and evidence as the host changes phase.",[
        kit::sec("Response",ai_answer::AIAnswer(#{key:"rollout-answer",question:"What should I check next?",source:source,
            sources:answer_sources(),status:status,framed:true,follow_ups:["Why did the health check fail?","Show the retry protocol"],
            on_stop:kit::put(h,"answer.phase","stopped"),on_regenerate:kit::put(h,"answer.phase","reading"),
            on_feedback:h.feedback,on_follow_up:kit::store(h,"answer.follow"),on_open_source:kit::store(h,"answer.open")})
            .with_style(style().width(px(630)).max_width(relative(1.0)))),
        kit::sec("Host updates",row([
            button::Button(#{key:"reading",text:"Reading sources",variant:"secondary",on_click:kit::put(h,"answer.phase","reading")}),
            button::Button(#{key:"writing",text:"Publish first chunk",variant:"secondary",on_click:kit::put(h,"answer.phase","writing")}),
            button::Button(#{key:"done",text:"Complete response",variant:"secondary",on_click:kit::put(h,"answer.phase","done")}),
        ]).with_style(style().flex_wrap().gap(px(6)))),
        kit::note(kit::val(v,"answer.follow","")),answer_note(v),
    ])
}

fn message_actions_page(h,v){
    let version=kit::val(v,"conversation.version",1);
    let response=if version==0{"Check the running build and review its warnings."}
        else if version==1{"Run the tests, then compare the resulting package with the previous build."}
        else{`Revision ${version+1}: review the build report first, then verify the installed package.`};
    let speaking=kit::val(v,"conversation.speaking",false);
    let body=column([
        text(response).with_key("response").test_id("message-preview")
            .with_style(style().typography("body"))
            .motion(motion_transition("opacity",0.45,1.0,#{duration_ms:280,intent:"feedback"})).motion_on_change(`${version}`),
        message_actions::MessageActions(#{key:"response-actions",text:response,versions:kit::val(v,"conversation.versions",2),
            version:version,speaking:speaking,on_version_change:kit::store(h,"conversation.version"),
            on_regenerate:h.regenerate,on_speak:h.speak,on_feedback:h.feedback}),
        kit::note(if speaking{"Playback requested. The application owns actual speech."}else{kit::val(v,"conversation.feedback","")}),
    ]).with_style(style().width(px(470)).max_width(relative(1.0)).min_width(px(0)).gap(theme_spacing("sm")));
    kit::page(h,v,"message_actions","MessageActions","Copy a response, review previous versions, and submit feedback.",[
        kit::sec("Response",body)
    ])
}
fn reaction_bar_page(h,v){
    let saved=kit::val(v,"conversation.reactions",#{});
    let defaults=[#{id:"like",count:1249,active:false},#{id:"comment",count:12,active:false},
        #{id:"repost",count:3,active:true},#{id:"share",count:0,active:false}];
    let accepted=[];for item in defaults{accepted.push(if item.id in saved{saved[item.id]}else{item});}
    kit::page(h,v,"reaction_bar","ReactionBar","Local and application-owned reactions with exact accessible counts.",[
        kit::sec("Release post",kit::pile([text("The new preview is ready to try."),
            reaction_bar::ReactionBar(#{key:"release-reactions",reactions:accepted,on_react:h.react})])),
        kit::sec("Local reactions",reaction_bar::ReactionBar(#{key:"local-reactions",actions:["like","share"]})),
    ])
}
fn model_picker_page(h,v){
    let value=kit::val(v,"conversation.model",#{model:"auto",thinking:false,search:false,effort:"medium"});
    let models=[
        #{id:"auto",name:"Auto",description:"Pick the best fit for this prompt"},
        #{id:"swift",name:"Swift",description:"Quick responses",speed:5,quality:3,cost:1,latency_ms:480,price_per_million:1.2},
        #{id:"studio/v2",name:"Studio",description:"Careful reasoning and deep review",speed:2,quality:5,cost:4,latency_ms:1800,price_per_million:8.0},
        #{id:"offline",name:"Offline",description:"Unavailable in this workspace",disabled:true},
    ];
    let picker=model_picker::ModelPicker(#{key:"composer-settings",models:models,value:value,placement:"bottom",
        on_change:kit::store(h,"conversation.model")});
    let editor=composer::Composer(#{key:"model-demo",label:"Prompt",value:kit::val(v,"conversation.draft",""),
        placeholder:"Ask about the build",leading:picker,on_change:kit::store(h,"conversation.draft"),
        on_submit:kit::store(h,"conversation.sent")});
    kit::page(h,v,"model_picker","ModelPicker","Change model, effort, and tools without leaving a draft.",[
        kit::sec("Composer",column([editor,kit::note(`Model: ${value.model} · Effort: ${value.effort}`)])
            .with_style(style().width(px(540)).max_width(relative(1.0)).gap(theme_spacing("sm")))),
    ])
}

fn tracker_page(h,v){
    let items=[#{id:"spec",label:"Draft the launch checklist",done:true},#{id:"build",label:"Build the workspace"},
        #{id:"audit",label:"Run the final audit"},#{id:"publish",label:"Announce availability"}];
    let done=kit::val(v,"tracker.done",["spec"]);
    kit::page(h,v,"progress_tracker","ProgressTracker","A controlled checklist with an animated progress meter.",[
        kit::sec("Launch",progress_tracker::ProgressTracker(#{key:"release",title:"Release checklist",steps:items,
            value:done,on_change:kit::store(h,"tracker.done")})),
        kit::sec("Playback",progress_tracker::ProgressTracker(#{key:"playback",title:"Guided preview",steps:[
            #{id:"draft",label:"Draft the handoff"},#{id:"review",label:"Review the handoff"},#{id:"share",label:"Share with the team"}
        ],autoplay:true,interval_ms:2200})),
        kit::note("The second card is a local demo. Click or focus it to stop playback."),
    ])
}
fn setup_page(h,v){
    let groups=[
        #{id:"workspace",title:"Workspace",tasks:[#{id:"create",label:"Create a workspace",done:true},
            #{id:"name",label:"Name your workspace"},#{id:"invite",label:"Invite collaborators"}]},
        #{id:"publish",title:"Publishing",tasks:[#{id:"review",label:"Review the setup"},#{id:"enable",label:"Enable sharing"}]},
    ];
    let hidden=kit::val(v,"setup.hidden",false);
    let surface=if hidden{button::Button(#{key:"reopen",text:"Reopen setup guide",variant:"secondary",on_click:kit::put(h,"setup.hidden",false)})}
        else{setup_guide::SetupGuide(#{key:"onboarding",title:"Setup guide",groups:groups,value:kit::val(v,"setup.done",["create"]),
            on_change:kit::store(h,"setup.done"),on_edit:kit::put(h,"setup.message","Edit requested"),
            on_close:kit::put(h,"setup.hidden",true)})};
    kit::page(h,v,"setup_guide","SetupGuide","A grouped guide that only advances after accepted task completion.",[
        kit::sec("Workspace onboarding",surface),
        kit::note(kit::val(v,"setup.message","The first unfinished task in each group is available.")),
    ])
}
fn avatar_page(h, v) {
    kit::page(h, v, "avatar", "Avatar", "Avatars take the control height of their size, so they sit in toolbars and rows.", [
        kit::sec("Sizes", kit::sizes([
            avatar::Avatar(#{ name: "Ada Lovelace", initials: "AL", size: "xs" }),
            avatar::Avatar(#{ name: "Grace Hopper", initials: "GH", size: "sm", presence: "online" }),
            avatar::Avatar(#{ name: "Edsger Dijkstra", initials: "ED", presence: "away" }),
            avatar::Avatar(#{ name: "王小明", initials: "王", size: "lg", presence: "busy" }),
        ])),
    ])
}

fn icon_page(h, v) {
    let names = ["check", "close", "search", "plus", "minus", "info", "warning", "calendar"];
    let icons = [];
    for name in names { icons.push(icon::Icon(#{ source: asset(`app/icons/${name}`), label: name })); }
    let failed = kit::val(v,"icon.preview_failure",false);
    kit::page(h, v, "icon", "Icon", "Icons take the color of the text around them; without a size they follow the control size.", [
        kit::sec("Recoverable image",kit::pile([
            kit::line([
                image_source(asset(if failed {"app/icons/preview-unavailable"}else{"app/icons/search"}))
                    .with_key("recoverable-icon").image_fallback_text("A")
                    .with_style(style().width(px(32)).height(px(32)).background(theme_color("surface_hover")))
                    .with_part_style("image_fallback",style().typography("control").font_weight(600))
                    .accessibility_role("image").accessibility_label("Recoverable logo"),
                button::Button(#{key:"icon-fail-preview",text:if failed {"Restore image"}else{"Simulate image failure"},variant:"secondary",
                    on_click:kit::put(h,"icon.preview_failure",!failed)}),
            ]),
            kit::note("A failed resource shows an initial without changing the image's accessible name. Failure diagnostics remain available to the Host."),
        ])),
        kit::sec("Set", kit::line(icons)),
        kit::sec("In a control", kit::line([
            button::Button(#{ key: "icon-in-button", text: "Search", variant: "secondary", prefix: icon::Icon(#{ source: asset("app/icons/search") }),
                on_click: h.noop }),
            button::Button(#{ key: "icon-in-primary", text: "Add host", variant: "primary",
                prefix: icon::Icon(#{ source: asset("app/icons/plus") }), on_click: h.noop }),
        ])),
    ])
}

fn progress_page(h, v) {
    kit::page(h, v, "progress", "Progress", "A 4px track with an accent fill; indeterminate moves a stripe.", [
        kit::sec("States", kit::pile([
            progress::Progress(#{ key: "progress-upload", value: 60.0, max: 100.0, label: "Upload", width: 320 }),
            progress::Progress(#{ key: "progress-scan", label: "Scanning", indeterminate: true, width: 320 }),
        ])),
    ])
}

fn skeleton_page(h, v) {
    kit::page(h, v, "skeleton", "Skeleton", "Placeholders for fixed shapes while content loads.", [
        kit::sec("Shapes", kit::pile([
            skeleton::Skeleton(#{ key: "skeleton-title", width: 240, height: 22, animated: false }),
            skeleton::Skeleton(#{ key: "skeleton-line", width: 360, height: 14, animated: false }),
        ])),
    ])
}

fn spinner_page(h, v) {
    kit::page(h, v, "spinner", "Spinner", "A faint ring and turning arc in the inherited foreground. Rotation is native; Reduced and None hold it still.", [
        kit::sec("Sizes", kit::line([
            spinner::Spinner(#{ key: "spinner-sm", label: "Loading", size: "sm" }),
            spinner::Spinner(#{ key: "spinner-md", label: "Loading" }),
            spinner::Spinner(#{ key: "spinner-lg", label: "Loading", size: "lg" }),
        ])),
        kit::sec("Inherited foreground", kit::line([
            row([spinner::Spinner(#{key:"spinner-inherited",label:"Checking"}),text("Checking connection")])
                .with_style(style().items_center().gap(theme_spacing("sm")).text_color(theme_color("text.success"))),
            spinner::Spinner(#{key:"spinner-custom",label:"Custom speed",speed_ms:1600,
                part_styles:#{indicator:style().text_color(theme_color("text_primary"))}}),
        ])),
    ])
}

fn sample_source() {
    "fn view(ctx) {\n    button::Button(#{ text: \"Deploy\", variant: \"primary\" })\n}\n"
}

fn code_viewer_page(h, v) {
    kit::page(h, v, "code_viewer", "CodeViewer", "Syntax-highlighted source with a line gutter; no frame of its own.", [
        kit::sec("Rhai", column([code_viewer::CodeViewer(#{ key: "code-sample", label: "Sample source", source: sample_source(),
            file_name: "main.rhai", language: "rhai" })]).with_style(style().height(px(160)))),
    ])
}

fn diff_viewer_page(h, v) {
    let left = #{ source: "replicas = 3\nregion = \"eu-west\"\n", label: "Current" };
    let right = #{ source: "replicas = 5\nregion = \"eu-west\"\nzone = \"b\"\n", label: "Proposed" };
    kit::page(h, v, "diff_viewer", "DiffViewer", "Unified or split comparison with folded context.", [
        kit::sec("Unified", column([diff_viewer::DiffViewer(#{ key: "diff-sample", left: left, right: right, mode: "unified" })])
            .with_style(style().height(px(180)))),
    ])
}
