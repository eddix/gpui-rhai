//! The Gallery application: every story, acceptance gates, the title bar and the catalog story.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Render, TestAppContext, WindowHandle};
use gpui_rhai::{PreparedScriptView, ScriptViewConfig, ScriptViewHandle, ScriptViewHost};

mod acceptance;
mod affine_resize;
mod agent_checkpoints;
mod agent_cursors;
mod agent_findings;
mod agent_fleet;
mod agent_graph;
mod agent_handoff;
mod agent_inbox;
mod agent_lanes;
mod agent_ledger;
mod agent_monitor;
mod agent_plan;
mod agent_queue;
mod agent_recap;
mod agent_steering;
mod agent_trail;
mod agent_tree;
mod application_content;
mod audit_report;
mod away_summary;
mod billing_schedule;
mod breakdown_chart;
mod catalog_story;
mod change_summary;
mod collaboration_stream;
mod compact_breakdown;
mod conflict_radar;
mod context_map;
mod context_window;
mod contrast_roles;
mod conversation;
mod cost_waterfall;
mod decision_tree;
mod decisions;
mod delegate_board;
mod diff_timeline;
mod document_anchors;
mod email_draft;
mod experiment_card;
mod fallback_chain;
mod fan_out_grid;
mod gates;
mod inputs_and_choices;
mod installed_starter;
mod judge_bracket;
mod kpi_chart;
mod memory_panel;
mod modern_clean_controls;
mod module_coverage;
mod navigation_workflow;
mod notification_stack;
mod overlay_triggers;
mod parallel_attempts;
mod plan_vs_actual;
mod profile_menu;
mod project_card;
mod rate_limit_planner;
mod responsive_grid;
mod review_center;
mod root_cause_tree;
mod running_now;
mod scheduled_runs;
mod shell_demo;
mod smart_recommendation;
mod stories;
mod tips_panel;
mod title_bar;
mod tool_access_map;
mod tool_calls;
mod trace_waterfall;
mod usage_meter;

/// Mounts a Gallery view the CLI prepared as the root of a new window and runs until parked.
/// `root` builds the window root from the host and the view; `bind_keys` installs the view's
/// key bindings on the host first.
fn mount_prepared<R: Render>(
    cx: &mut TestAppContext,
    prepared: PreparedScriptView,
    (host_id, view_id): (&str, &str),
    bind_keys: bool,
    root: fn(ScriptViewHost, ScriptViewHandle) -> R,
) -> (WindowHandle<R>, ScriptViewHandle) {
    let bindings = prepared.key_bindings().to_vec();
    let (host_id, view_id) = (host_id.to_owned(), view_id.to_owned());
    let mounted = Rc::new(RefCell::new(None));
    let capture = Rc::clone(&mounted);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(host_id, cx).unwrap();
        if bind_keys {
            host.bind_keys(bindings, cx).unwrap();
        }
        let view = prepared
            .mount(ScriptViewConfig::new(view_id), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        root(host, view)
    });
    cx.run_until_parked();
    let view = mounted.borrow().clone().unwrap();
    (window, view)
}
