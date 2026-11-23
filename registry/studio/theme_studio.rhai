import "components/accordion" as accordion;
import "components/avatar" as avatar;
import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/checkbox" as checkbox;
import "components/collapsible" as collapsible;
import "components/date_picker" as date_picker;
import "components/calendar" as calendar;
import "components/time_input" as time_input;
import "components/markdown_viewer" as markdown_viewer;
import "components/time_range_picker" as time_range_picker;
import "components/date_range_picker" as date_range_picker;
import "components/timeline" as timeline;
import "components/dialog" as dialog;
import "components/divider" as divider;
import "components/combobox" as combobox;
import "components/form_field" as form_field;
import "components/icon" as icon;
import "components/input" as input;
import "components/label" as label;
import "components/menu" as menu;
import "components/pagination" as pagination;
import "components/popover" as popover;
import "components/progress" as progress;
import "components/radio_group" as radio_group;
import "components/radio" as radio;
import "components/select" as select;
import "components/skeleton" as skeleton;
import "components/switch" as switch_component;
import "components/table" as table;
import "components/split_pane" as split_pane;
import "components/resizable" as resizable;
import "components/draggable" as draggable;
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
import "components/sortable" as sortable;
import "components/pan_zoom" as pan_zoom;
import "components/rotatable" as rotatable;
import "components/selection_area" as selection_area;
import "components/tree" as tree;
import "components/list" as list;
import "components/selectable_list" as selectable_list;
import "components/column_browser" as column_browser;
import "components/selectable_card_group" as selectable_card_group;
import "components/tabs" as tabs;
import "components/breadcrumb" as breadcrumb;
import "components/tab_overflow_menu" as tab_overflow_menu;
import "components/sliding_panel" as sliding_panel;
import "components/carousel" as carousel;
import "components/tag" as tag;
import "components/textarea" as textarea;
import "components/toast" as toast;
import "components/tooltip" as tooltip;
import "components/hover_card" as hover_card;
import "components/number_input" as number_input;
import "components/otp_input" as otp_input;
import "components/tag_input" as tag_input;
import "components/filter_menu" as filter_menu;
import "components/stepper_form" as stepper_form;
import "components/progress_tracker" as progress_tracker;
import "components/setup_guide" as setup_guide;
import "components/message_actions" as message_actions;
import "components/reaction_bar" as reaction_bar;
import "components/cited_answer" as cited_answer;
import "components/ai_answer" as ai_answer;
import "components/thinking_loader" as thinking_loader;
import "components/thinking_state" as thinking_state;
import "patterns/model_picker" as model_picker;
import "patterns/run_controls" as run_controls;
import "patterns/prompt_library" as prompt_library;
import "patterns/integration_picker" as integration_picker;
import "patterns/integration_settings" as integration_settings;
import "patterns/comment_thread" as comment_thread;
import "patterns/comments_panel" as comments_panel;
import "patterns/writing_assistant" as writing_assistant;
import "patterns/approval_card" as approval_card;
import "patterns/permission_request" as permission_request;
import "patterns/decision_card" as decision_card;
import "patterns/decision_tree" as decision_tree;
import "patterns/review_center" as review_center;
import "patterns/delegate_board" as delegate_board;
import "patterns/agent_queue" as agent_queue;
import "patterns/agent_lanes" as agent_lanes;
import "patterns/agent_handoff" as agent_handoff;
import "patterns/agent_inbox" as agent_inbox;
import "patterns/agent_plan" as agent_plan;
import "patterns/agent_checkpoints" as agent_checkpoints;
import "patterns/agent_trail" as agent_trail;
import "patterns/agent_tree" as agent_tree;
import "patterns/agent_graph" as agent_graph;
import "patterns/agent_fleet" as agent_fleet;
import "patterns/agent_monitor" as agent_monitor;
import "patterns/agent_steering" as agent_steering;
import "patterns/agent_recap" as agent_recap;
import "patterns/agent_findings" as agent_findings;
import "patterns/agent_ledger" as agent_ledger;
import "patterns/agent_cursors" as agent_cursors;
import "patterns/compact_breakdown" as compact_breakdown;
import "patterns/usage_meter" as usage_meter;
import "patterns/change_summary" as change_summary;
import "patterns/tips_panel" as tips_panel;
import "patterns/project_card" as project_card;
import "patterns/experiment_card" as experiment_card;
import "patterns/notification_stack" as notification_stack;
import "patterns/away_summary" as away_summary;
import "patterns/kpi_chart" as kpi_chart;
import "patterns/breakdown_chart" as breakdown_chart;
import "patterns/running_now" as running_now;
import "patterns/scheduled_runs" as scheduled_runs;
import "patterns/tool_call_card" as tool_call_card;
import "patterns/tool_call_group" as tool_call_group;
import "patterns/context_window" as context_window;
import "patterns/context_map" as context_map;
import "patterns/profile_menu" as profile_menu;
import "patterns/email_draft" as email_draft;
import "patterns/diff_timeline" as diff_timeline;
import "patterns/cost_waterfall" as cost_waterfall;
import "patterns/parallel_attempts" as parallel_attempts;
import "patterns/fan_out_grid" as fan_out_grid;
import "patterns/billing_schedule" as billing_schedule;
import "patterns/plan_vs_actual" as plan_vs_actual;
import "patterns/fallback_chain" as fallback_chain;
import "patterns/memory_panel" as memory_panel;
import "patterns/tool_access_map" as tool_access_map;
import "patterns/trace_waterfall" as trace_waterfall;
import "patterns/smart_recommendation" as smart_recommendation;
import "patterns/rate_limit_planner" as rate_limit_planner;
import "patterns/root_cause_tree" as root_cause_tree;
import "patterns/judge_bracket" as judge_bracket;
import "patterns/collaboration_stream" as collaboration_stream;
import "patterns/conflict_radar" as conflict_radar;
import "patterns/decision_inbox" as decision_inbox;
import "patterns/approval_gates" as approval_gates;
import "patterns/autonomy_dial" as autonomy_dial;
import "components/alert" as alert;
import "components/alert_dialog" as alert_dialog;
import "components/badge" as badge;
import "components/button_group" as button_group;
import "components/card" as card;
import "components/command" as command;
import "components/command_dialog" as command_dialog;
import "components/context_menu" as context_menu;
import "components/empty" as empty;
import "components/group_box" as group_box;
import "components/input_group" as input_group;
import "components/kbd" as kbd;
import "components/sheet" as sheet;
import "components/slider" as slider;
import "components/range_slider" as range_slider;
import "components/spinner" as spinner;
import "components/toggle" as toggle;
import "components/toggle_group" as toggle_group;
import "components/scroll_area" as scroll_area;
import "components/title_bar" as title_bar;
import "components/status_bar" as status_bar;
import "components/code_viewer" as code_viewer;
import "components/diff_viewer" as diff_viewer;

// A specimen row the Studio draws in a SelectableList: a name and a muted count.
fn specimen_author_row(ctx, row) {
    row([text(row.item.name).with_style(style().flex_grow()),
        text(row.item.themes).with_style(style().text_color(theme_color("text_muted")))])
        .with_style(style().flex_grow().items_center().gap(theme_spacing("sm")))
}

fn color_field(name, value) {
    row([
        row([]).with_style(style().width(px(18)).height(px(18))
            .border(px(1)).border_color(theme_color("border"))
            .background(theme_color(name))),
        column([
            text(name).with_style(style().typography("caption").font_weight(700)
                .text_color(theme_color("text_muted"))),
            input::Input(#{
                key: `studio-${name}`, label: `${name} color`, value: value, placeholder: "#rrggbb",
                on_change: native_handler(`theme_studio.set_${name}`)
            })
        ]).with_style(style().flex_grow().gap(px(2)))
    ]).with_style(style().gap(px(6)).items_center())
}

fn state_schema() {
    #{ fields: #{
        preview_section: #{schema:#{type:"string",allowed:["controls","navigation","data","product"]},"default":#{type:"string",value:"controls"}},
        specimen_column_path: #{schema:#{type:"array",items:#{type:"string"}},"default":#{type:"array",value:[]}},
        specimen_card_values: #{schema:#{type:"array",items:#{type:"string"}},"default":#{type:"array",value:[#{type:"string",value:"east"}]}},
        specimen_clock: #{schema:#{type:"string"},"default":#{type:"string",value:""}},
        specimen_number: #{schema:#{type:"optional",value:#{type:"number"}},"default":#{type:"integer",value:4000}},
        specimen_otp: #{schema:#{type:"string"},"default":#{type:"string",value:""},sensitive:true},
        specimen_tags: #{schema:#{type:"array",items:#{type:"string"}},"default":#{type:"array",value:[#{type:"string",value:"Design"}]}},
        specimen_wizard: #{schema:#{type:"string"},"default":#{type:"string",value:"editing"}},
        specimen_time_range: #{schema:#{type:"ui_value"},"default":#{type:"map",value:#{kind:#{type:"string",value:"relative"},duration_ms:#{type:"integer",value:3600000}}}},
        specimen_date_range: #{schema:#{type:"ui_value"},"default":#{type:"null"}},
        path: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __PATH__ } },
        family: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __FAMILY__ } },
        variant: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __VARIANT__ } },
        mode: #{ schema: #{ type: "string", allowed: ["light", "dark"] },
            "default": #{ type: "string", value: __MODE__ } },
        status: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __STATUS__ } },
        color_surface: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_SURFACE__ } },
        color_surface_raised: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_SURFACE_RAISED__ } },
        color_surface_hover: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_SURFACE_HOVER__ } },
        color_text_primary: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_TEXT_PRIMARY__ } },
        color_text_muted: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_TEXT_MUTED__ } },
        color_accent: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_ACCENT__ } },
        color_accent_hover: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_ACCENT_HOVER__ } },
        color_on_accent: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_ON_ACCENT__ } },
        color_danger: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_DANGER__ } },
        color_on_danger: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_ON_DANGER__ } },
        color_warning: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_WARNING__ } },
        color_on_warning: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_ON_WARNING__ } },
        color_success: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_SUCCESS__ } },
        color_on_success: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_ON_SUCCESS__ } },
        color_border: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_BORDER__ } },
        color_focus_ring: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_FOCUS_RING__ } },
        color_selection: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_SELECTION__ } },
        color_disabled: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __COLOR_DISABLED__ } },
        dialog_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_DIALOG__ } },
        popover_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_POPOVER__ } },
        menu_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_MENU__ } },
        toast_visible: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_TOAST__ } },
        builtin_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        builtin_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        interaction_status: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "Ready — controls are live" } },
        interaction_count: #{ schema: #{ type: "integer", min: 0 },
            "default": #{ type: "integer", value: 0 } },
        specimen_input: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        specimen_field: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "Ada Lovelace" } },
        specimen_textarea: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "Two lines\nof themed text" } },
        specimen_select: #{ schema: #{ type: "string", allowed: ["nord", "gruvbox"] },
            "default": #{ type: "string", value: "nord" } },
        specimen_select_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        specimen_select_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        specimen_combo: #{ schema: #{ type: "array", max_items: 1, items: #{ type: "string" } },
            "default": #{ type: "array", value: [#{ type: "string", value: "dark" }] } },
        specimen_combo_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        specimen_combo_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        specimen_date: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "2026-09-01" } },
        specimen_checked: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } },
        specimen_mixed: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        specimen_mixed_indeterminate: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } },
        specimen_radio_mode: #{ schema: #{ type: "string", allowed: ["safe", "fast"] },
            "default": #{ type: "string", value: "fast" } },
        specimen_radio_target: #{ schema: #{ type: "string", allowed: ["local", "remote"] },
            "default": #{ type: "string", value: "local" } },
        specimen_switch_enabled: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } },
        specimen_switch_off: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        specimen_tab_equal: #{ schema: #{ type: "string", allowed: ["status", "trace"] },
            "default": #{ type: "string", value: "trace" } },
        specimen_tab_content: #{ schema: #{ type: "string", allowed: ["status", "trace"] },
            "default": #{ type: "string", value: "status" } },
        specimen_accordion: #{ schema: #{ type: "array", max_items: 2, items: #{ type: "string" } },
            "default": #{ type: "array", value: [#{ type: "string", value: "runtime" }] } },
        specimen_collapsible: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } },
        specimen_table_selected: #{ schema: #{ type: "array", max_items: 3, items: #{ type: "string" } },
            "default": #{ type: "array", value: [#{ type: "string", value: "two" }] } },
        specimen_table_sort_key: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "name" } },
        specimen_table_sort_direction: #{ schema: #{ type: "string", allowed: ["none", "ascending", "descending"] },
            "default": #{ type: "string", value: "ascending" } },
        specimen_collapsed_groups: #{ schema: #{ type: "array", max_items: 3, items: #{ type: "string" } },
            "default": #{ type: "array", value: [] } },
        specimen_split_size: #{ schema: #{ type: "number", min: 0.0, max: 1.0 },
            "default": #{ type: "float", value: 0.42 } },
        specimen_resizable_rect: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
            x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
            y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
            width: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false },
            height: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false },
        } }, "default": #{ type: "map", value: #{
            x: #{ type: "float", value: 14.0 }, y: #{ type: "float", value: 12.0 },
            width: #{ type: "float", value: 190.0 }, height: #{ type: "float", value: 72.0 }
        } } },
        specimen_draggable_position: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
            x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
            y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
        } }, "default": #{ type: "map", value: #{
            x: #{ type: "float", value: 14.0 }, y: #{ type: "float", value: 12.0 }
        } } },
        specimen_sort_order: #{ schema: #{ type: "array", max_items: 3, items: #{ type: "string" } },
            "default": #{ type: "array", value: [#{ type: "string", value: "build" },
                #{ type: "string", value: "test" }, #{ type: "string", value: "ship" }] } },
        specimen_pan_zoom: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
            x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
            y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
            scale: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false },
        } }, "default": #{ type: "map", value: #{
            x: #{ type: "float", value: 0.0 }, y: #{ type: "float", value: 0.0 },
            scale: #{ type: "float", value: 1.0 }
        } } },
        specimen_rotation: #{ schema: #{ type: "number" },
            "default": #{ type: "float", value: 0.0 } },
        specimen_selection: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "string" } },
            "default": #{ type: "array", value: [] } },
        specimen_selection_active: #{ schema: #{ type: "optional", value: #{ type: "string" } },
            "default": #{ type: "null" } },
        specimen_selection_anchor: #{ schema: #{ type: "optional", value: #{ type: "string" } },
            "default": #{ type: "null" } },
        specimen_tree_expanded: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "string" } },
            "default": #{ type: "array", value: [#{ type: "string", value: "root" }] } },
        specimen_tree_selected: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "string" } },
            "default": #{ type: "array", value: [] } },
        specimen_tree_active: #{ schema: #{ type: "optional", value: #{ type: "string" } },
            "default": #{ type: "string", value: "root" } },
        specimen_page: #{ schema: #{ type: "integer", min: 1, max: 128 },
            "default": #{ type: "integer", value: 3 } },
        specimen_page_size: #{ schema: #{ type: "integer", allowed: [10, 25, 50] },
            "default": #{ type: "integer", value: 10 } },
        specimen_menu_active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "open" } },
        specimen_alignment: #{ schema: #{ type: "string", allowed: ["left", "center", "right"] },
            "default": #{ type: "string", value: "left" } },
        p0_input: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        p0_pinned: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } },
        p0_toggle_values: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "string" } },
            "default": #{ type: "array", value: [#{ type: "string", value: "bold" }] } },
        p0_slider: #{ schema: #{ type: "number", min: 0.0, max: 100.0 },
            "default": #{ type: "float", value: 42.0 } },
        specimen_range: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
            low: #{ schema: #{ type: "number" }, required: true, sensitive: false },
            high: #{ schema: #{ type: "number" }, required: true, sensitive: false },
        } }, "default": #{ type: "map", value: #{
            low: #{ type: "float", value: 20.0 }, high: #{ type: "float", value: 80.0 }
        } } },
        p0_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        p0_command_active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "new" } },
        p0_command_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_COMMAND__ } },
        p0_sheet_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_SHEET__ } },
        p0_alert_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_ALERT__ } },
        p0_context_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        p0_context_active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "copy" } },
        gallery_theme_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        gallery_theme_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        gallery_category: #{ schema: #{ type: "string", allowed: ["all", "foundations", "forms", "navigation", "documents", "overlays", "product"] },
            "default": #{ type: "string", value: __GALLERY_CATEGORY__ } },
    } }
}

fn set_dialog(ctx, value) { ctx.set_state("dialog_open", value); }
fn set_popover(ctx, value) { ctx.set_state("popover_open", value); }
fn set_menu(ctx, value) { ctx.set_state("menu_open", value); }
fn show_dialog(ctx, payload) { ctx.set_state("dialog_open", true); }
fn show_toast(ctx, payload) { ctx.set_state("toast_visible", true); }
fn dismiss_toast(ctx, payload) { ctx.set_state("toast_visible", false); }
fn set_builtin_open(ctx, value) { ctx.set_state("builtin_open", value); }
fn set_builtin_query(ctx, value) { ctx.set_state("builtin_query", value); }
fn record_action(label, ctx, payload) {
    ctx.set_state("interaction_count", ctx.get_state("interaction_count") + 1);
    ctx.set_state("interaction_status", label);
    handled()
}
fn record_value(label, ctx, value) {
    ctx.set_state("interaction_count", ctx.get_state("interaction_count") + 1);
    ctx.set_state("interaction_status", `${label}: ${value}`);
    handled()
}
fn set_specimen_input(ctx, value) { ctx.set_state("specimen_input", value); record_value("Input", ctx, value); }
fn set_specimen_number(ctx,value) {ctx.set_state("specimen_number",value);record_value("Number input",ctx,value);}
fn set_specimen_otp(ctx,value) {ctx.set_state("specimen_otp",value);}
fn set_specimen_tags(ctx,value) {ctx.set_state("specimen_tags",value);record_value("Tags",ctx,value.len);}
fn save_specimen_wizard(ctx,payload){ctx.set_state("specimen_wizard","succeeded");}
fn reset_specimen_wizard(ctx,payload){ctx.set_state("specimen_wizard","editing");}
fn set_specimen_field(ctx, value) { ctx.set_state("specimen_field", value); record_value("Name", ctx, value); }
fn set_specimen_textarea(ctx, value) { ctx.set_state("specimen_textarea", value); record_action("Notes updated", ctx, ()); }
fn set_specimen_select(ctx, value) { if value != () { ctx.set_state("specimen_select", value); record_value("Theme", ctx, value); } }
fn set_specimen_select_open(ctx, value) { ctx.set_state("specimen_select_open", value); }
fn set_specimen_select_query(ctx, value) { ctx.set_state("specimen_select_query", value); }
fn set_specimen_combo(ctx, value) { ctx.set_state("specimen_combo", value); record_action("Appearance changed", ctx, ()); }
fn set_specimen_combo_open(ctx, value) { ctx.set_state("specimen_combo_open", value); }
fn set_specimen_combo_query(ctx, value) { ctx.set_state("specimen_combo_query", value); }
fn set_specimen_date(ctx, value) { if value != () { ctx.set_state("specimen_date", value); record_value("Date", ctx, value); } }
fn set_specimen_checked(ctx, value) { ctx.set_state("specimen_checked", value.checked); record_value("Checked", ctx, value.checked); }
fn set_specimen_mixed(ctx, value) {
    ctx.set_state("specimen_mixed", value.checked);
    ctx.set_state("specimen_mixed_indeterminate", value.indeterminate);
    record_value("Mixed", ctx, value.checked);
}
fn set_specimen_radio_mode(ctx, value) { ctx.set_state("specimen_radio_mode", value); record_value("Mode", ctx, value); }
fn set_specimen_radio_target(ctx, value) { ctx.set_state("specimen_radio_target", value); record_value("Target", ctx, value); }
fn set_specimen_switch_enabled(ctx, value) { ctx.set_state("specimen_switch_enabled", value); record_value("Enabled", ctx, value); }
fn set_specimen_switch_off(ctx, value) { ctx.set_state("specimen_switch_off", value); record_value("Secondary switch", ctx, value); }
fn set_specimen_tab_equal(ctx, value) { ctx.set_state("specimen_tab_equal", value); record_value("Equal tab", ctx, value); }
fn set_specimen_tab_content(ctx, value) { ctx.set_state("specimen_tab_content", value); record_value("Content tab", ctx, value); }
fn choose_specimen_ancestor(ctx, value) { record_value("Breadcrumb",ctx,value); }
fn set_specimen_accordion(ctx, value) { ctx.set_state("specimen_accordion", value); record_action("Accordion changed", ctx, ()); }
fn set_specimen_collapsible(ctx, value) { ctx.set_state("specimen_collapsible", value); record_value("Release details", ctx, value); }
fn set_specimen_split_size(ctx, value) { ctx.set_state("specimen_split_size", value); record_action("Split resized", ctx, ()); }
fn set_specimen_resizable(ctx, value) {
    ctx.set_state("specimen_resizable_rect", value);
    record_action("Rectangle resized", ctx, ());
}
fn set_specimen_draggable(ctx, value) {
    ctx.set_state("specimen_draggable_position", value);
    record_action("Card moved", ctx, ());
}
fn specimen_index(values, key) {
    for index in 0..values.len { if values[index] == key { return index; } }
    -1
}
fn set_specimen_sortable(ctx, value) {
    let order = ctx.get_state("specimen_sort_order");
    let source = specimen_index(order, value.source_key);
    if source < 0 { return; }
    let moved = order.remove(source);
    let anchor = specimen_index(order, value.anchor_key);
    if anchor < 0 { return; }
    order.insert(if value.placement == "after" { anchor + 1 } else { anchor }, moved);
    ctx.set_state("specimen_sort_order", order);
    record_action("Order changed", ctx, ());
}
fn set_specimen_pan_zoom(ctx, value) {
    ctx.set_state("specimen_pan_zoom", value);
    record_action("Canvas transformed", ctx, ());
}
fn set_specimen_rotation(ctx, value) { ctx.set_state("specimen_rotation", value); record_action("Canvas rotated", ctx, ()); }
fn set_specimen_selection(ctx, value) {
    ctx.set_state("specimen_selection", value.selected_keys);
    ctx.set_state("specimen_selection_active", value.active_key);
    ctx.set_state("specimen_selection_anchor", value.anchor_key);
    record_action("Canvas selection changed", ctx, ());
}
fn set_specimen_tree_expanded(ctx,value){ctx.set_state("specimen_tree_expanded",value);record_action("Tree expanded",ctx,());}
fn set_specimen_tree_selected(ctx,value){ctx.set_state("specimen_tree_selected",value);record_action("Tree selected",ctx,());}
fn set_specimen_tree_active(ctx,value){ctx.set_state("specimen_tree_active",value);}
fn set_specimen_table_selection(ctx, value) { ctx.set_state("specimen_table_selected", value); record_action("Table selection changed", ctx, ()); }
fn set_specimen_table_sort(ctx, value) {
    if value == () {
        ctx.set_state("specimen_table_sort_direction", "none");
        record_action("Table sort cleared", ctx, ());
    } else {
        ctx.set_state("specimen_table_sort_key", value.key);
        ctx.set_state("specimen_table_sort_direction", value.direction);
        record_value("Table sort", ctx, `${value.key} ${value.direction}`);
    }
}
fn set_specimen_group(ctx, value) {
    let groups = ctx.get_state("specimen_collapsed_groups");
    let next = [];
    for group in groups { if group != value { next.push(group); } }
    if !(value in groups) { next.push(value); }
    ctx.set_state("specimen_collapsed_groups", next);
    record_value("Group", ctx, value);
}
fn set_specimen_page(ctx, value) {
    ctx.set_state("specimen_page", value.current_page);
    ctx.set_state("specimen_page_size", value.page_size);
    record_value("Page", ctx, value.current_page);
}
fn set_specimen_menu_active(ctx, value) { ctx.set_state("specimen_menu_active", value); }
fn specimen_menu_action(ctx, value) { ctx.set_state("menu_open", false); record_value("Menu", ctx, value); }
fn set_specimen_alignment(value, ctx, payload) { ctx.set_state("specimen_alignment", value); record_value("Alignment", ctx, value); }
fn set_p0_input(ctx, value) { ctx.set_state("p0_input", value); record_action("Search updated", ctx, ()); }
fn set_p0_pinned(ctx, value) { ctx.set_state("p0_pinned", value); record_value("Pinned", ctx, value); }
fn close_dialog_with(label, ctx, payload) { ctx.set_state("dialog_open", false); record_action(label, ctx, payload); }
fn confirm_p0_alert(ctx, payload) { ctx.set_state("p0_alert_open", false); record_action("Alert confirmed", ctx, payload); }
fn cancel_p0_alert(ctx, payload) { ctx.set_state("p0_alert_open", false); record_action("Alert cancelled", ctx, payload); }
fn set_p0_toggles(ctx, value) { ctx.set_state("p0_toggle_values", value); record_action("Formatting changed", ctx, ()); }
fn set_p0_slider(ctx, value) { ctx.set_state("p0_slider", value); record_value("Volume", ctx, value); }
fn set_specimen_range(ctx, value) { ctx.set_state("specimen_range", value); record_action("Range changed", ctx, ()); }
fn set_p0_query(ctx, value) { ctx.set_state("p0_query", value); }
fn set_p0_command_active(ctx, value) { ctx.set_state("p0_command_active", value); }
fn set_p0_command_open(ctx, value) { ctx.set_state("p0_command_open", value); }
fn set_p0_sheet_open(ctx, value) { ctx.set_state("p0_sheet_open", value); }
fn set_p0_alert_open(ctx, value) { ctx.set_state("p0_alert_open", value); }
fn set_p0_context_open(ctx, value) { ctx.set_state("p0_context_open", value); }
fn set_p0_context_active(ctx, value) { ctx.set_state("p0_context_active", value); }
fn open_p0_command(ctx, payload) { ctx.set_state("p0_command_open", true); }
fn open_p0_sheet(ctx, payload) { ctx.set_state("p0_sheet_open", true); }
fn open_p0_alert(ctx, payload) { ctx.set_state("p0_alert_open", true); }
fn set_gallery_theme_open(ctx, value) { ctx.set_state("gallery_theme_open", value); }
fn set_gallery_theme_query(ctx, value) { ctx.set_state("gallery_theme_query", value); }
fn set_gallery_category(category, ctx, payload) { ctx.set_state("gallery_category", category); }
fn gallery_theme_key(variant) { `${variant.family.len}:${variant.family}${variant.name}` }
fn set_gallery_theme(ctx, values) {
    if values.len == 0 { return; }
    let value = values[0];
    for variant in ctx.theme_variants() {
        if gallery_theme_key(variant) == value {ctx.set_theme(variant.family,variant.name);break;}
    }
    ctx.set_state("gallery_theme_open", false);
    ctx.set_state("gallery_theme_query", "");
}
fn noop(ctx, payload) { handled() }

fn init(ctx) {
    ctx.set_theme(__PREVIEW_FAMILY__, __PREVIEW_VARIANT__);
    ctx.set_locale(__VISUAL_LOCALE__);
}

fn editor(ctx) {
    let mode = ctx.get_state("mode");
    column([
        column([
            text("THEME STUDIO").with_style(style().typography("heading")),
            text("Edit semantic tokens. Preview real components.")
                .with_style(style().typography("caption").text_color(theme_color("text_muted")))
        ]).with_style(style().gap(px(2)).padding_bottom(px(8))
            .border_bottom(px(1)).border_color(theme_color("border"))),
        label::Label(#{ text: "Theme file", description: "Open edits the file; Import creates a copy." }),
        input::Input(#{ key: "studio-path", label: "Theme file", value: ctx.get_state("path"),
            placeholder: "ui/theme.rhai", on_change: native_handler("theme_studio.set_path") }),
        row([
            button::Button(#{ text: "New", size: "sm", variant: "outline",
                on_click: native_handler("theme_studio.new") }),
            button::Button(#{ text: "Open", size: "sm", variant: "outline",
                on_click: native_handler("theme_studio.open") }),
            button::Button(#{ text: "Import", size: "sm", variant: "outline",
                on_click: native_handler("theme_studio.import") }),
            button::Button(#{ text: "Save", size: "sm", variant: "primary",
                on_click: native_handler("theme_studio.save") }),
        ]).with_style(style().gap(px(4))),
        combobox::Combobox(#{
            key: "studio-builtin", label: "Bundled theme", placeholder: "Load a bundled theme", searchable: true,
            options: __BUILTIN_OPTIONS__, selected: [], open: ctx.get_state("builtin_open"),
            query: ctx.get_state("builtin_query"),
            on_change: native_handler("theme_studio.builtin"),
            on_open_change: Fn("set_builtin_open"), on_query_change: Fn("set_builtin_query")
        }),
        divider::Divider(#{}),
        input::Input(#{ key: "studio-family", label: "Theme family", value: ctx.get_state("family"),
            placeholder: "Family", on_change: native_handler("theme_studio.set_family") }),
        input::Input(#{ key: "studio-variant", label: "Theme variant", value: ctx.get_state("variant"),
            placeholder: "Variant", on_change: native_handler("theme_studio.set_variant") }),
        row([
            button::Button(#{ text: "Dark", size: "sm",
                variant: if mode == "dark" { "primary" } else { "outline" },
                on_click: native_handler("theme_studio.mode_dark") }),
            button::Button(#{ text: "Light", size: "sm",
                variant: if mode == "light" { "primary" } else { "outline" },
                on_click: native_handler("theme_studio.mode_light") }),
            button::Button(#{ text: "Derive", size: "sm", variant: "secondary",
                on_click: native_handler("theme_studio.derive") }),
        ]).with_style(style().gap(px(4))),
        color_field("surface", ctx.get_state("color_surface")),
        color_field("surface_raised", ctx.get_state("color_surface_raised")),
        color_field("surface_hover", ctx.get_state("color_surface_hover")),
        color_field("text_primary", ctx.get_state("color_text_primary")),
        color_field("text_muted", ctx.get_state("color_text_muted")),
        color_field("accent", ctx.get_state("color_accent")),
        color_field("accent_hover", ctx.get_state("color_accent_hover")),
        color_field("on_accent", ctx.get_state("color_on_accent")),
        color_field("danger", ctx.get_state("color_danger")),
        color_field("on_danger", ctx.get_state("color_on_danger")),
        color_field("warning", ctx.get_state("color_warning")),
        color_field("on_warning", ctx.get_state("color_on_warning")),
        color_field("success", ctx.get_state("color_success")),
        color_field("on_success", ctx.get_state("color_on_success")),
        color_field("border", ctx.get_state("color_border")),
        color_field("focus_ring", ctx.get_state("color_focus_ring")),
        color_field("selection", ctx.get_state("color_selection")),
        color_field("disabled", ctx.get_state("color_disabled")),
        text(ctx.get_state("status")).with_style(style().padding(px(8))
            .border(px(1)).border_color(theme_color("border"))
            .typography("caption").text_color(theme_color("text_muted")))
    ]).with_style(style().width(px(340)).height(relative(1.0)).min_height(px(0))
        .flex_shrink(false).overflow_y_scroll().padding(px(14)).gap(px(7))
        .border_end(px(1)).border_color(theme_color("border"))
        .background(theme_color("surface")))
        .scrollbars("hidden", "auto")
}

fn specimen_title(title, detail) {
    column([
        text(title).with_style(style().typography("body").font_weight(700)),
        text(detail).with_style(style().typography("caption")
            .text_color(theme_color("text_muted")))
    ]).with_style(style().gap(px(2)).padding_bottom(px(4))
        .border_bottom(px(1)).border_color(theme_color("border")))
}

fn specimen_typography() {
    column([
        specimen_title("TYPOGRAPHY", "Eight semantic roles share one theme-owned rhythm."),
        text("Display Large · 28 / 36").with_style(style().typography("display_large")),
        text("Display · 24 / 32").with_style(style().typography("display")),
        text("Heading · 18 / 24").with_style(style().typography("heading")),
        text("Title · 16 / 22").with_style(style().typography("title")),
        text("Subtitle · 14 / 20").with_style(style().typography("subtitle")),
        text("Body · 13 / 18").with_style(style().typography("body")),
        text("Body Small · 12 / 16").with_style(style().typography("body_small")),
        text("CAPTION · 11 / 16").with_style(style().typography("caption")),
    ]).with_style(style().gap(px(4)))
}

fn specimen_actions(ctx) {
    let search_icon = svg("<svg width='24' height='24' viewBox='0 0 24 24' fill='none'><circle cx='11' cy='11' r='6' stroke='currentColor' stroke-width='2'/><path d='M16 16L21 21' stroke='currentColor' stroke-width='2'/></svg>");
    let selected_search_icon = svg("<svg width='24' height='24' viewBox='0 0 24 24' fill='none'><circle cx='11' cy='11' r='6' stroke='currentColor' stroke-width='2'/><path d='M16 16L21 21' stroke='currentColor' stroke-width='2'/></svg>");
    let launch_icon = svg("<svg width='24' height='24' viewBox='0 0 24 24' fill='none'><path d='M5 12H19M13 6L19 12L13 18' stroke='currentColor' stroke-width='2'/></svg>");
    column([
        specimen_title("ACTIONS", "Variants, sizes, tags and disabled states."),
        row([
            button::Button(#{ text: "Primary", variant: "primary", on_click: Fn("record_action").curry("Primary action") }),
            button::Button(#{ text: "Secondary", variant: "secondary", on_click: Fn("record_action").curry("Secondary action") }),
            button::Button(#{ text: "Outline", variant: "outline", on_click: Fn("record_action").curry("Outline action") }),
            button::Button(#{ text: "Ghost", variant: "ghost", on_click: Fn("record_action").curry("Ghost action") }),
            button::Button(#{ text: "Danger", variant: "danger", on_click: Fn("record_action").curry("Danger action") }),
        ]).with_style(style().gap(px(5))),
        row([
            button::Button(#{ text: "XS", size: "xs", variant: "outline", on_click: Fn("record_action").curry("XS button") }),
            button::Button(#{ text: "SM", size: "sm", variant: "outline", on_click: Fn("record_action").curry("SM button") }),
            button::Button(#{ text: "MD", size: "md", variant: "outline", on_click: Fn("record_action").curry("MD button") }),
            button::Button(#{ text: "LG", size: "lg", variant: "outline", on_click: Fn("record_action").curry("LG button") }),
            button::Button(#{ text: "Disabled", disabled: true }),
            button::Button(#{ text: "Loading", loading: true, variant: "secondary" }),
        ]).with_style(style().gap(px(5)).items_center()),
        row([
            icon_button::IconButton(#{ icon: search_icon, label: "Search", size: "md", variant: "outline",
                on_click: Fn("record_action").curry("Search icon button") }),
            icon_button::IconButton(#{ icon: selected_search_icon, label: "Selected search",
                selected: true, size: "md", variant: "ghost", on_click: Fn("record_action").curry("Selected icon button") }),
            button::Button(#{ text: "Launch", prefix: launch_icon, variant: "secondary",
                on_click: Fn("record_action").curry("Launch") }),
        ]).with_style(style().gap(px(5)).items_center()),
        row([
            button::Button(#{ text: "Density", size: "md", variant: "primary",
                on_click: Fn("record_action").curry("Density button") }),
            badge::Badge(#{ text: "Density", size: "md", variant: "accent" }),
            button::Button(#{ text: "Compact", size: "xs", variant: "outline",
                on_click: Fn("record_action").curry("Compact button") }),
            badge::Badge(#{ text: "Compact", size: "sm", variant: "neutral" }),
        ]).with_style(style().gap(px(8)).items_center()),
        row([
            tag::Tag(#{ text: "ACCENT", variant: "accent" }),
            tag::Tag(#{ text: "SUCCESS", variant: "success" }),
            tag::Tag(#{ text: "WARNING", variant: "warning" }),
            tag::Tag(#{ text: "DANGER", variant: "danger" }),
        ]).with_style(style().gap(px(5))),
        text(`Last interaction: ${ctx.get_state("interaction_status")}`)
            .with_style(style().typography("caption").text_color(theme_color("text_muted")))
    ]).with_style(style().gap(px(7)))
}

fn specimen_inputs(ctx) {
    let field = input::Input(#{ key: "specimen-field", label: "Name", value: ctx.get_state("specimen_field"),
        placeholder: "Name", on_change: Fn("set_specimen_field") });
    column([
        specimen_title("INPUTS", "Idle, populated, error, read-only and multiline."),
        row([
            input::Input(#{ key: "specimen-input", label: "Empty input", value: ctx.get_state("specimen_input"),
                placeholder: "Placeholder", on_change: Fn("set_specimen_input") })
                .with_style(style().width(px(160))),
            input::Input(#{ key: "specimen-error", label: "Invalid input", value: "Invalid", error: true })
                .with_style(style().width(px(160))),
            input::Input(#{ key: "specimen-readonly", label: "Read-only input", value: "Read only", read_only: true })
                .with_style(style().width(px(160))),
        ]).with_style(style().gap(px(6)).flex_wrap()),
        form_field::FormField(#{ id: "specimen-name", label: "Name", required: true,
            control: field, description: "One shared field contract", error: "Example error" }),
        number_input::NumberInput(#{key:"specimen-number",label:"Token budget",value:ctx.get_state("specimen_number"),
            min:0,max:8000,step:100,unit:"tokens",on_change:Fn("set_specimen_number")})
            .with_style(style().width(px(225))),
        otp_input::OtpInput(#{key:"specimen-otp",label:"Verification code",value:ctx.get_state("specimen_otp"),
            length:4,on_change:Fn("set_specimen_otp")}),
        tag_input::TagInput(#{key:"specimen-tags",label:"Labels",values:ctx.get_state("specimen_tags"),max:6,
            on_change:Fn("set_specimen_tags")}),
        filter_menu::FilterMenu(#{key:"specimen-filters",title:"Filters",groups:[
            #{id:"state",label:"State",options:[#{id:"active",label:"Active"},#{id:"paused",label:"Paused"}]}
        ],default_value:#{state:["active"]},hints:false}),
        stepper_form::StepperForm(#{key:"specimen-stepper",status:ctx.get_state("specimen_wizard"),steps:[
            #{id:"theme",title:"Theme",content:text("Preview the active design.")},
            #{id:"confirm",title:"Confirm",review:[#{key:"design",label:"Design",text:"Current preset",edit_step:"theme"}]},
        ],on_submit:Fn("save_specimen_wizard"),on_reset:Fn("reset_specimen_wizard")}),
        progress_tracker::ProgressTracker(#{key:"specimen-checklist",steps:[
            #{id:"draft",label:"Draft",done:true},#{id:"review",label:"Review"}
        ]}),
        setup_guide::SetupGuide(#{key:"specimen-setup",groups:[
            #{id:"intro",title:"First steps",tasks:[#{id:"choose",label:"Choose a color"},#{id:"preview",label:"Preview the design"}]}
        ]}),
        message_actions::MessageActions(#{key:"specimen-message",text:"Review the active design",versions:3}),
        reaction_bar::ReactionBar(#{key:"specimen-reactions"}),
        cited_answer::CitedAnswer(#{key:"specimen-cite",segments:[#{text:"Preview follows the active design.",cite:["guide"]}],
            sources:[#{id:"guide",title:"Design guide",site:"Guide",quote:"Preview follows the active design."}]}),
        ai_answer::AIAnswer(#{key:"specimen-answer",question:"What am I previewing?",source:"The active design.",
            framed:true}),
        thinking_loader::ThinkingLoader(#{key:"specimen-loader",phases:["Reviewing the palette"],phase:0}),
        thinking_state::ThinkingState(#{key:"specimen-work",status:"done",elapsed_ms:10500,
            steps:[#{id:"review",label:"Review theme roles"}]}),
        model_picker::ModelPicker(#{key:"specimen-model",models:[#{id:"auto",name:"Auto",description:"Choose automatically"}]}),
        run_controls::RunControls(#{key:"specimen-run",run_id:"theme/preview",agent_name:"Preview assistant",
            status:"stopped",draft:"",prompt:"Review this palette",answer:"The current design is ready to review.",context:0.42}),
        prompt_library::PromptLibrary(#{key:"specimen-prompts",templates:[
            #{id:"review",title:"Review palette",category:"Design",body:"Write a summary for {audience}."}
        ]}),
        integration_picker::IntegrationPicker(#{key:"specimen-tools",integrations:[
            #{id:"notes",name:"Team Notes",domain:"notes.local",tone:"violet",description:"Preview the integration tiles."}
        ]}),
        integration_settings::IntegrationSettings(#{key:"specimen-settings",integration:"notes",name:"Team Notes",meta:"Document sync",
            value:#{enabled:true,site:"notes.example",zone:"Etc/UTC"},revision:0,
            zones:[#{id:"Etc/UTC",label:"UTC"}],credential_present:true}),
        comment_thread::CommentThread(#{key:"specimen-thread",title:"Review thread",draft:"",messages:[
            #{id:"theme",author:"Casey Chen",time:"2 min ago",text:"This heading follows the active design."},
        ]}),
        comments_panel::CommentsPanel(#{key:"specimen-comments",title:"Team comments",draft:"",comments:[
            #{id:"question",author:"Casey Chen",time:"5 min ago",text:"Does the new palette work here?"},
            #{id:"draft",author:"Taylor Kim",time:"2 min ago",timestamp:1,text:"/Edit Add a note to the preview.",edit_state:"applied"},
        ]}),
        writing_assistant::WritingAssistant(#{key:"specimen-writer",conversation:"theme-preview",revision:0,draft:"",messages:[],
            suggestions:[#{id:"draft",label:"Draft a reply"},#{id:"tone",label:"Fix the tone"}]}),
        approval_card::ApprovalCard(#{key:"specimen-approval",request:"review",revision:0,title:"Review changes",risk:"medium",
            details:[#{label:"Area",value:"Theme palette"}],remaining_ms:34000}),
        permission_request::PermissionRequest(#{key:"specimen-request",request:"sample",revision:0,agent:"Review assistant",action:"read sample data",
            preview:[#{id:"view",label:"Current preview"}]}),
        decision_card::DecisionCard(#{key:"specimen-decision",decisions:[#{id:"preview",revision:0,question:"Which layout should we preview?",
            options:[#{id:"short",title:"Compact",recommended:true},#{id:"long",title:"Expanded"}]}]}),
        decision_tree::DecisionTree(#{key:"specimen-tree",allow_other:false,decisions:[#{id:"theme",revision:0,agent:"Theme assistant",
            question:"Where should this theme be applied?",options:[#{id:"preview",label:"Stay in preview",effects:["No preferences change"]},
                #{id:"later",label:"Save for later",effects:["Keep current settings"]}]}]}),
        review_center::ReviewCenter(#{key:"specimen-reviews",now_ms:420000,reviewed:1,items:[#{id:"palette",revision:0,
            title:"Review the palette",agent:"Theme assistant",kind:"Design",risk:"medium",confidence:0.83,created_ms:1000,
            body:"Preview the text and borders before accepting."}]}),
        delegate_board::DelegateBoard(#{key:"specimen-delegation",board:"studio",revision:0,now_ms:1000,
            agents:[#{id:"lettering",name:"Typography",rate_cents:3},#{id:"preview",name:"Preview"}],
            tasks:[#{id:"sample",title:"Verify glyph layout",estimate:6},#{id:"layout",title:"Check native layout",estimate:10,agent:"preview"}],
            suggestion:#{task:"sample",agent:"lettering"}}),
        agent_queue::AgentQueue(#{key:"specimen-queue",queue:"studio/queue",revision:0,now_ms:50000,backpressure:3,
            workers:[#{id:"preview",name:"Preview worker"}],
            jobs:[#{id:"spacing",name:"Check spacing",status:"queued",kind:"tool",enqueued_ms:35000},
                #{id:"type",name:"Inspect typography",status:"running",kind:"human",enqueued_ms:10000,started_ms:20000,worker:"preview"}]}),
        agent_lanes::AgentLanes(#{key:"specimen-lanes",timeline:"studio/lanes",revision:0,now_ms:300000,span_ms:300000,
            agents:[#{id:"design",name:"Design"},#{id:"preview",name:"Preview"}],
            tasks:[#{id:"palette",agent:"design",title:"Set palette",status:"done",start_ms:50000,end_ms:160000,handoff_to:"verify"},
                #{id:"verify",agent:"preview",title:"Verify preview",status:"running",start_ms:160000}]}),
        agent_handoff::AgentHandoff(#{key:"specimen-handoff",relay:"studio/relay",revision:0,
            agents:[#{id:"design",name:"Design"},#{id:"preview",name:"Preview"}],
            tasks:[#{id:"test",title:"Preview the palette",status:"working",agent:"preview"}],
            handoffs:[#{id:"palette",from:"design",to:"preview",task:"test",items:[#{id:"note",label:"Contrast notes",kind:"note"}]}]}),
        agent_inbox::AgentInbox(#{key:"specimen-agent-inbox",inbox:"studio/inbox",revision:0,now_ms:65000,
            tasks:[#{id:"check",title:"Check component spacing",agent:"Preview assistant",status:"running",started_ms:30000},
                #{id:"palette",title:"Review the palette",agent:"Design assistant",status:"waiting",started_ms:10000}]}),
        agent_plan::AgentPlan(#{key:"specimen-agent-plan",plan:"studio/plan",revision:0,now_ms:10000,status:"running",title:"Validate the theme",
            steps:[#{id:"palette",title:"Load palette",status:"done",elapsed_ms:1000},
                #{id:"verify",title:"Check active controls",status:"active",started_ms:6000,tool:"preview"},
                #{id:"save",title:"Review readability"}]}),
        agent_checkpoints::AgentCheckpoints(#{key:"specimen-checkpoints",run:"studio/history",revision:0,task:"Theme preview",
            checkpoints:[#{id:"base",title:"Load palette",files:2},#{id:"first",title:"Check the first preview",parent:"base",status:"stopped"},
                #{id:"branch",title:"Preview the new spacing",parent:"base",status:"running",elapsed_ms:16000}]}),
        agent_trail::AgentTrail(#{key:"specimen-trail",run:"studio/trail",revision:0,now_ms:12000,title:"Check the new skin",agent:"Preview assistant",
            steps:[#{id:"setup",title:"Load the theme",state:"done",elapsed_ms:1000},#{id:"inspect",title:"Review controls",state:"running",started_ms:3500,
                substeps:[#{id:"labels",title:"Review labels"},#{id:"controls",title:"Inspect focus states",state:"running"}]},
                #{id:"approval",title:"Review example",state:"waiting"}]}),
        agent_tree::AgentTree(#{key:"specimen-dispatch",run:"studio/agents",revision:0,now_ms:42000,root:"preview",supervisor:"observer",
            agents:[#{id:"preview",name:"Preview",task:"Check the theme",state:"running",started_ms:10000,cost_micros:30000},
                #{id:"contrast",parent:"preview",name:"Contrast",task:"Review the palette",state:"waiting",cost_micros:10000},
                #{id:"observer",name:"Supervisor",task:"Watch the discussion",state:"done",cost_micros:12000}]}),
        agent_graph::AgentGraph(#{key:"specimen-graph",run:"studio/graph",revision:0,now_ms:35000,title:"Communication sample",height:294,
            nodes:[#{id:"lead",name:"Coordinator",kind:"agent",status:"working"},#{id:"review",name:"Review",kind:"agent",status:"waiting"},
                #{id:"palette",name:"Palette",kind:"tool"},#{id:"observer",name:"Observer",kind:"user"}],
            edges:[#{from:"lead",to:"review"},#{from:"lead",to:"palette"},#{from:"review",to:"observer"}]}),
        agent_fleet::AgentFleet(#{key:"specimen-fleet",run:"studio/fleet",revision:0,now_ms:42000,
            agents:[#{id:"lead",name:"Preview team",task:"Check the theme",state:"running",cost_micros:30000},
                #{id:"contrast",parent:"lead",name:"Contrast reviewer",task:"Review the palette",state:"waiting",
                    steps:[#{id:"inspect",title:"Inspect the light palette"}]},#{id:"cleanup",name:"Cleanup",task:"Archive the previews",state:"queued"}]}),
        agent_monitor::AgentMonitor(#{key:"specimen-monitor",run:"studio/monitor",revision:0,now_ms:21000,started_ms:1000,state:"running",
            name:"Theme preview",role:"Acceptance",cost_micros:20000,
            steps:[#{id:"palette",label:"Load palette",state:"done",kind:"read",elapsed_ms:7000},
                #{id:"checks",label:"Measure spacing",state:"running",kind:"run",started_ms:12000}]}),
        agent_steering::AgentSteering(#{key:"specimen-steering",run:"studio/steering",revision:0,now_ms:42000,title:"Review the theme",
            steps:[#{id:"setup",title:"Load preset",status:"done",elapsed_ms:10000},
                #{id:"review",title:"Review spacing",status:"active",started_ms:22000}],
            queue:[#{id:"read",text:"Check the focus border in narrow layouts."}],compose:false}),
        agent_recap::AgentRecap(#{key:"specimen-recap",run:"studio/recap",revision:0,since_ms:0,until_ms:14400000,replay_ms:0,limit:1,
            events:[#{id:"summary",at_ms:2700000,kind:"done",title:"Prepare a summary",detail:"Preview assistant",count:12},
                #{id:"review",at_ms:5400000,kind:"needs",title:"Review the preview",detail:"Check the color palette"},
                #{id:"build",at_ms:10800000,kind:"failed",title:"Preview build failed",detail:"Missing preview asset"}]}),
        agent_findings::AgentFindings(#{key:"specimen-findings",run:"studio/findings",revision:0,now_ms:42000,
            agents:[#{id:"preview",name:"Preview review",step:"Checking reading scale",done:4,total:7},
                #{id:"design",name:"Design review",done:5,total:5}],
            findings:[#{id:"focus",agent:"preview",title:"Focus indicator placement",severity:"warning",location:"button.rhai:34",at_ms:38000}]}),
        agent_ledger::AgentLedger(#{key:"specimen-ledger",run:"studio/ledger",revision:0,now_ms:42000,agent:"Preview assistant",point_at:"root",
            entries:[#{id:"root",action:"Update accent",target:"Theme",system:"Studio",at_ms:10000,change:#{before:"Blue",after:"Violet"}},
                #{id:"page",action:"Rebuild sample",system:"Gallery",at_ms:18000,detail:"All controls",depends_on:["root"]},
                #{id:"mail",action:"Send summary",system:"Mail",at_ms:22000,detail:"One recipient",reversible:false,depends_on:["root"]}]}),
        agent_cursors::AgentCursors(#{key:"specimen-cursors",document:"studio/review",revision:0,height:168,
            title:"Theme review",agents:[#{id:"preview",name:"Review assistant",cursor:#{target:"intro",at:5,select:9}}],
            blocks:[#{id:"intro",kind:"paragraph",text:"Check the palette and text spacing in this preview."}],
            suggestions:[#{id:"type",agent:"preview",block:"intro",at:27,remove:7,insert:"rhythm",note:"Use one term consistently."}]}),
        decision_inbox::DecisionInbox(#{key:"specimen-inbox",decisions:[#{id:"preview",revision:0,title:"Update the sample?",
            primary:"Update",secondary:"Wait"}]}),
        approval_gates::ApprovalGates(#{key:"specimen-gates",run:"sample",gates:[#{id:"preview",revision:0,agent:"Review assistant",
            kind:"message",action:"Share the preview",reason:"Check a sample update.",status:"open"}]}),
        autonomy_dial::AutonomyDial(#{key:"specimen-dial",policy:"sample-policy",revision:0,level:"risky",pinned:[],
            actions:[#{id:"preview",risk:"safe",label:"Read preview"},#{id:"publish",risk:"risky",label:"Update sample"}]}),
        textarea::Textarea(#{ key: "specimen-textarea", label: "Notes", value: ctx.get_state("specimen_textarea"),
            rows: 2, max_length: 120, show_count: true, on_change: Fn("set_specimen_textarea") }),
        row([
            select::Select(#{ key: "specimen-select", label: "Theme", value: ctx.get_state("specimen_select"),
                open: ctx.get_state("specimen_select_open"), query: ctx.get_state("specimen_select_query"),
                width: px(160), options: [#{ value: "nord", label: "Nord" }, #{ value: "gruvbox", label: "Gruvbox" }],
                on_change: Fn("set_specimen_select"), on_open_change: Fn("set_specimen_select_open"),
                on_query_change: Fn("set_specimen_select_query") }),
            combobox::Combobox(#{ key: "specimen-combobox", label: "Appearance", selected: ctx.get_state("specimen_combo"),
                open: ctx.get_state("specimen_combo_open"), query: ctx.get_state("specimen_combo_query"),
                width: px(160), options: [#{ value: "dark", label: "Dark" }, #{ value: "light", label: "Light" }],
                on_change: Fn("set_specimen_combo"), on_open_change: Fn("set_specimen_combo_open"),
                on_query_change: Fn("set_specimen_combo_query") }),
            date_picker::DatePicker(#{ key: "specimen-date", label: "Appointment date", value: ctx.get_state("specimen_date"),
                width: px(160), placeholder: "Date", presets: [#{ label: "Today", value: "2026-09-01" }],
                on_change: Fn("set_specimen_date") }),
        ]).with_style(style().gap(px(6)).flex_wrap())
    ]).with_style(style().gap(px(7)))
}

fn specimen_product_cards() {
    column([
        specimen_title("PRODUCT CARDS", "Live examples of product and workflow components."),
        compact_breakdown::CompactBreakdown(#{key:"specimen-breakdown",series:"studio/cost",revision:0,title:"Infrastructure cost",
            items:[#{id:"inference",label:"Inference",value:84.0},#{id:"storage",label:"Storage",value:12.0},
                #{id:"compute",label:"Background compute",value:4.0}],delta:12.4,prefix:"$",period:"month",
            periods:[#{id:"week",label:"Week"},#{id:"month",label:"Month"},#{id:"year",label:"Year"}]}),
        usage_meter::UsageMeter(#{key:"specimen-usage",series:"studio/usage",revision:0,total:1000,
            used:[#{id:"compute",label:"Compute",value:640},#{id:"search",label:"Search",value:180}],
            limits:[#{id:"seats",label:"Seats",used:8,total:10},#{id:"projects",label:"Projects",used:12}]}),
        change_summary::ChangeSummary(#{key:"specimen-changes",period:"This week",changes:[
            #{id:"latency",label:"Latency",change:"Down 14 ms to 62 ms",tone:"good",direction:"down"},
            #{id:"traffic",label:"Requests",change:"Within the usual range"}]}),
        tips_panel::TipsPanel(#{key:"specimen-tips",series:"studio/tips",revision:0,tips:[
            #{id:"keys",title:"Inspect shortcuts",description:"Use the command menu to find the current key binding.",action:"Show shortcuts"},
            #{id:"theme",title:"Pick your theme",description:"Keep the layout and choose new colours.",action:"Choose theme",done:true}]}),
        project_card::ProjectCard(#{key:"specimen-project",project:"preview",revision:0,title:"Improve gallery",status:"progress",due:"Oct 24",open:true,trigger:"click",
            tasks:[#{id:"review",label:"Review examples",done:true},#{id:"build",label:"Build the preview"}]}),
        experiment_card::ExperimentCard(#{key:"specimen-experiment",experiment:"preview",revision:0,title:"Compare onboarding",open:true,trigger:"click",
            metrics:[#{id:"conversion",label:"Conversion",suffix:"%"}],
            variants:[#{id:"base",short:"A",name:"Baseline",values:#{conversion:10.0}},#{id:"new",short:"B",name:"New flow",values:#{conversion:12.0}}]}),
        notification_stack::NotificationStack(#{key:"specimen-stack",stack:"preview",revision:0,notices:[
            #{id:"preview",badge:"Notice",title:"Theme ready",text:"Review the updated design before applying it."},
            #{id:"later",title:"Next",text:"Queued behind the preview."}]}),
        away_summary::AwaySummary(#{key:"specimen-away",session:"studio-preview",revision:0,since_ms:0,now_ms:8040000,items:[
            #{id:"check",kind:"needs_you",title:"Review contrast",detail:"The new palette is ready."},
            #{id:"load",kind:"done",title:"Loaded preview"}]}),
        kpi_chart::KpiChart(#{key:"specimen-kpi",metric:"theme-demo",revision:0,title:"Weekly use",value:"42",delta:6,
            samples:[#{id:"mon",label:"Mon"},#{id:"wed",label:"Wed"},#{id:"fri",label:"Fri"}],
            series:[#{id:"runs",label:"Runs",values:[18,24,42]},#{id:"reviews",label:"Reviews",values:[12,(),29]}]}),
        breakdown_chart::BreakdownChart(#{key:"specimen-split",dataset:"preview/workload",title:"Workload",
            records:[#{area:"api",kind:"team"},#{area:"ops",kind:"batch"},#{area:"api",kind:"team"}],
            dimensions:[#{id:"area",label:"Area",levels:[#{id:"api",label:"API"},#{id:"ops",label:"Operations"}]},
                #{id:"kind",label:"Kind",levels:[#{id:"team",label:"Team"},#{id:"batch",label:"Batch"}]}],
            measures:[#{id:"count",label:"Count",unit:"tasks"}]}),
        running_now::RunningNow(#{key:"specimen-runs",board:"preview/runs",revision:0,
            tasks:[#{id:"index",title:"Index documents",step:"Reading local sources",status:"running",done:1,total:4},
                #{id:"review",title:"Review theme",step:"Needs approval",status:"needs_you",done:2,total:3}]}),
        scheduled_runs::ScheduledRuns(#{key:"specimen-week",board:"preview/week",revision:0,mode:"manual",
            days:[#{id:"mon",label:"Mon",date:"23"},#{id:"tue",label:"Tue",date:"24"}],
            periods:[#{id:"am",label:"07:00",limit:100},#{id:"pm",label:"12:00",limit:100}],
            current:#{day:"mon",period:"pm"},
            tasks:[#{id:"verify",title:"Verify the release",units:30},#{id:"photos",title:"Transfer photos",units:65}],
            assignments:[#{id:"sample",task:"verify",day:"mon",period:"pm",units:30,ordinal:1,origin:"booked"}]}),
        tool_call_card::ToolCallCard(#{key:"specimen-tool",session:"preview/diagnostics",revision:0,default_open:true,now_ms:1000,
            entry:#{id:"metadata",name:"read_metadata",status:"success",icon:"file",duration_ms:150,
                arguments:#{source:"theme"},result:#{status:"ready",count:2}}}),
        tool_call_group::ToolCallGroup(#{key:"specimen-tool-group",session:"preview/batch",revision:0,now_ms:1000,
            calls:[#{id:"search",name:"search_assets",status:"running",icon:"search",started_ms:0},
                #{id:"scan",name:"scan_palette",status:"success",duration_ms:350}]}),
        context_window::ContextWindow(#{key:"specimen-context",context:"theme-preview",revision:0,limit:100000,cells:54,columns:18,model:"Demo model",
            parts:[#{id:"system",label:"System",tokens:12000,locked:true},#{id:"tools",label:"Tools",tokens:16000},
                #{id:"chat",label:"Conversation",tokens:52000}]}),
        context_map::ContextMap(#{key:"specimen-map",context:"theme-preview",revision:0,limit:100000,model:"Demo model",
            blocks:[#{id:"system",label:"System",kind:"system",tokens:12000},#{id:"tools",label:"Tools",kind:"tools",tokens:16000},
                #{id:"chat",label:"Conversation",kind:"chat",tokens:52000}]}),
        profile_menu::ProfileMenu(#{key:"specimen-profile",name:"Ada Lovelace",email:"ada@example.test",
            setup:#{done:2,total:5,next:"Choose a profile picture"}}),
        email_draft::EmailDraft(#{key:"specimen-email",draft:"preview/email",revision:0,reference:"SUB-1182",
            account:"Northwind Logistics",amount:"$8,900",status:"In review",agent:"Renewal agent",
            contacts:[#{id:"ada",name:"Ada Lovelace",email:"ada@example.test"}],
            tones:[#{id:"friendly",label:"Friendly",body:"Hi {first}, your quote is ready to review."}]}),
        diff_timeline::DiffTimeline(#{key:"specimen-diff",run:"theme-preview/patch",agent:#{id:"demo",name:"Patch agent"},context:"Review",
            edits:[#{id:"a",at:4,file:"src/lib.rs",added:0,removed:10},#{id:"b",at:24,file:"src/app.rs",added:29,removed:1}]}),
        cost_waterfall::CostWaterfall(#{key:"specimen-cost",run:"theme-preview/cost",task:"Review",
            steps:[#{id:"a",name:"Summarize",kind:"llm",status:"done",cost_micros:29000,tokens_in:680,tokens_out:130},
                #{id:"b",name:"Credit",kind:"tool",status:"done",cost_micros:-34000}]}),
        parallel_attempts::ParallelAttempts(#{key:"specimen-attempts",run:"theme-preview/race",task:"Review candidates",
            attempts:[#{id:"fixture",name:"Fixture",model:"Atlas",status:"done",score:96,cost_micros:87000,duration_ms:4820},
                #{id:"timing",name:"Timing",model:"Vista",status:"failed",progress:0.38,cost_micros:34000,duration_ms:2270}]}),
        fan_out_grid::FanOutGrid(#{key:"specimen-fan",run:"theme-preview/fan",job:"Index sources",now_ms:5100,
            tasks:[#{id:"docs",label:"Docs",status:"done",cost_micros:24000,end_ms:3400},#{id:"notes",label:"Notes",status:"retrying"},
                #{id:"assets",label:"Assets",status:"failed",error:"No manifest"},#{id:"code",label:"Code",status:"running"},
                #{id:"archive",label:"Archive",status:"queued"}]}),
        billing_schedule::BillingSchedule(#{key:"specimen-billing",schedule:"theme-preview/billing",
            draft:#{phase:"Pilot",start:"2026-02-01",duration:"3m",lines:[
                #{id:"starter",kind:"product",name:"Starter licence",repeat:"monthly",timing:"advance",price_minor:25000}]}}),
        plan_vs_actual::PlanVsActual(#{key:"specimen-plan-diff",run:"theme-preview/plan-diff",now_ms:42000,task:"Ship a report",steps:[
            #{id:"build",title:"Build report",planned_ms:30000,start_ms:0,spent_micros:120000},
            #{id:"review",title:"Review and send",planned_ms:30000,depends_on:["build"],planned_micros:250000}]}),
        fallback_chain::FallbackChain(#{key:"specimen-fallback",run:"theme-preview/fallback",now_ms:5200,route:"demo/auto",attempts:[
            #{id:"primary",model:"Atlas",status:"failed",duration_ms:1800,reason:"Rate limited"},
            #{id:"backup",model:"Nova",status:"trying",started_ms:3200,cost_micros:8500}]}),
        memory_panel::MemoryPanel(#{key:"specimen-memory",workspace:"theme-preview/memory",memories:[
            #{id:"lang",text:"Writes Rust utilities",category:"Work",source:"Added by you"},
            #{id:"tea",text:"Prefers tea",category:"Preferences",paused:true}]}),
        tool_access_map::ToolAccessMap(#{key:"specimen-tool-map",session:"theme-preview/calls",tools:[
            #{id:"read",name:"Read documents"},#{id:"formats",name:"Check styles"},#{id:"notes",name:"Collect notes"}],
            agents:[#{id:"planner",name:"Planner",model:"Atlas",status:"done",tools:[#{id:"read",calls:[
                #{id:"open-brief",summary:"Open brief",status:"done",input:"brief.md",output:"Opened",duration_ms:130}
            ]},#{id:"notes"}]}]}),
        trace_waterfall::TraceWaterfall(#{key:"specimen-trace",session:"theme-preview/trace",now_ms:7800,spans:[
            #{id:"plan",name:"Review documents",kind:"agent",status:"running",start_ms:0},
            #{id:"outline",parent:"plan",name:"Draft outline",kind:"llm",status:"done",start_ms:640,end_ms:5200,cost_micros:23000},
            #{id:"read",parent:"plan",name:"Read brief",kind:"tool",status:"failed",start_ms:5400,end_ms:7100}]}),
        smart_recommendation::SmartRecommendation(#{key:"preview-recommendation",session:"preview/sync",options:[
            #{id:"delta",label:"Incremental sync",reason:"Only transfer the changed files and keep the current index.",signal:3},
            #{id:"all",label:"Full sync",reason:"Rebuild the workspace from its source copy.",signal:1}]}),
        rate_limit_planner::RateLimitPlanner(#{key:"preview-rate",session:"preview/load",limit:40000,now:430,runs:[
            #{id:"scan",name:"Index notes",at:405,peak:18000},#{id:"docs",name:"Review guides",at:500,peak:28000},
            #{id:"digest",name:"Draft digest",at:510,peak:22000}]}),
        root_cause_tree::RootCauseTree(#{key:"preview-root-cause",session:"theme-preview/fault",causes:[
            #{id:"publish",title:"Could not publish report",detail:"Draft unchanged",gate:"or"},
            #{id:"policy",title:"Publish policy",parent:"publish",status:"checking",files:["config/review.rhai"]},
            #{id:"artifact",title:"Missing artifact",parent:"publish",status:"ruled-out"}]}),
        judge_bracket::JudgeBracket(#{key:"preview-bracket",session:"theme-preview/bracket",contenders:[
            #{id:"outline",name:"Concise outline",output:"Explain the change, then show an example."},
            #{id:"check",name:"Checklist draft",output:"Check scope, links and examples."}],
            verdicts:#{"0:0":#{a:"outline",b:"check",winner:"outline",scores:[8.3,7.4]}}}),
        collaboration_stream::CollaborationStream(#{key:"preview-stream",session:"theme-preview/stream",agents:[#{id:"editor",name:"Editor"}],
            messages:[#{id:"edit",from:"editor",to:["you"],kind:"question",text:"Is the summary ready?",at:"14:10"},
                #{id:"answer",from:"you",to:["editor"],kind:"reply",reply_to:"edit",text:"Ready to review.",at:"14:12"}],max_height:190}),
        conflict_radar::ConflictRadar(#{key:"preview-radar",session:"theme-preview/radar",agents:[#{id:"a",name:"Atlas"},#{id:"b",name:"Scribe"}],
            files:[#{id:"guide",path:"release/guide.md",edits:[#{id:"a",agent:"a",start:10,end:16,revision:0},#{id:"b",agent:"b",start:14,end:22,revision:0}]}]}),
    ]).with_style(style().padding(px(16)).gap(px(14)).typography("body"))
}

fn specimen_choices(ctx) {
    column([
        specimen_title("CHOICES", "Persistent selection remains distinct from focus."),
        row([
            checkbox::Checkbox(#{ label: "Checked", checked: ctx.get_state("specimen_checked"),
                on_change: Fn("set_specimen_checked") }),
            checkbox::Checkbox(#{ label: "Mixed", checked: ctx.get_state("specimen_mixed"),
                indeterminate: ctx.get_state("specimen_mixed_indeterminate"), on_change: Fn("set_specimen_mixed") }),
            checkbox::Checkbox(#{ label: "Disabled", checked: false, disabled: true }),
        ]).with_style(style().gap(px(8))),
        radio_group::RadioGroup(#{ value: ctx.get_state("specimen_radio_mode"), label: "Mode", orientation: "horizontal",
            options: [#{ value: "safe", label: "Safe" }, #{ value: "fast", label: "Fast" }],
            on_change: Fn("set_specimen_radio_mode") }),
        row([
            radio::Radio(#{ value: "local", selected: ctx.get_state("specimen_radio_target") == "local",
                label: "Local", on_select: Fn("set_specimen_radio_target") }),
            radio::Radio(#{ value: "remote", selected: ctx.get_state("specimen_radio_target") == "remote",
                label: "Remote", on_select: Fn("set_specimen_radio_target") })
        ]).with_style(style().gap(px(8))),
        row([
            switch_component::Switch(#{ label: "Enabled", checked: ctx.get_state("specimen_switch_enabled"),
                on_change: Fn("set_specimen_switch_enabled") }),
            switch_component::Switch(#{ label: "Off", checked: ctx.get_state("specimen_switch_off"),
                on_change: Fn("set_specimen_switch_off") }),
            switch_component::Switch(#{ label: "Unavailable", checked: false, disabled: true }),
        ]).with_style(style().gap(px(8)))
    ]).with_style(style().gap(px(7)))
}

fn specimen_status() {
    column([
        specimen_title("STATUS", "Avatar, progress, skeleton and asset tint."),
        row([
            avatar::Avatar(#{ name: "Ada Lovelace", initials: "AL", presence: "online" }),
            avatar::Avatar(#{ name: "Lin Chen", initials: "LC", presence: "away" }),
            icon::Icon(#{ source: asset("app/icons/check"), size: "lg", label: "Complete" }),
            column([
                progress::Progress(#{ key: "specimen-progress", value: 72.0, max: 100.0, label: "Build" }),
                progress::Progress(#{ key: "specimen-indeterminate", indeterminate: true, label: "Sync" }),
            ]).with_style(style().gap(px(6))),
            column([
                skeleton::Skeleton(#{ key: "specimen-sk-a", width: 120, height: 10 }),
                skeleton::Skeleton(#{ key: "specimen-sk-b", width: 180, height: 10, animated: false }),
            ]).with_style(style().gap(px(6)))
        ]).with_style(style().gap(px(8)).items_center())
    ]).with_style(style().gap(px(7)))
}

fn specimen_chrome(ctx) {
    column([
        specimen_title("APPLICATION CHROME", "Composable title and status bars; native window authority stays in Rust."),
        column([
            title_bar::TitleBar(#{ label: "Workbench", title: "Workbench",
                subtitle: text("main").with_style(style().typography("caption")
                    .text_color(theme_color("text_muted"))),
                center: [text("Editor")],
                end: [button::Button(#{ text: "Run", size: "xs", variant: "ghost",
                    on_click: Fn("record_action").curry("Workbench run") })] }),
            row([text("Application content")]).with_style(style()
                .height(px(42)).padding_x(px(10)).items_center()
                .background(theme_color("surface"))),
            status_bar::StatusBar(#{ label: "Workbench status",
                start: [text("Ready")], center: [text("main")],
                end: [text("Ln 24, Col 8"), kbd::Kbd(#{ text: "UTF-8" })] })
        ]).with_style(style().border(px(1)).border_color(theme_color("border")))
    ]).with_style(style().gap(px(7)))
}

fn specimen_navigation(ctx) {
    let music_icon = svg("<svg width='24' height='24' viewBox='0 0 24 24' fill='none'><path d='M9 18V5L20 3V16M9 9L20 7' stroke='currentColor' stroke-width='2'/><circle cx='6' cy='18' r='3' stroke='currentColor' stroke-width='2'/><circle cx='17' cy='16' r='3' stroke='currentColor' stroke-width='2'/></svg>");
    let trace_icon = svg("<svg width='24' height='24' viewBox='0 0 24 24' fill='none'><path d='M3 12H7L10 5L14 19L17 12H21' stroke='currentColor' stroke-width='2'/></svg>");
    let lock_icon = svg("<svg width='24' height='24' viewBox='0 0 24 24' fill='none'><rect x='5' y='10' width='14' height='10' stroke='currentColor' stroke-width='2'/><path d='M8 10V7A4 4 0 0116 7V10' stroke='currentColor' stroke-width='2'/></svg>");
    column([
        specimen_title("NAVIGATION", "Tabs use one continuous track and one inset selection surface, or labels on a hairline with an ink bar."),
        tabs::Tabs(#{ label: "Equal preview tabs", value: ctx.get_state("specimen_tab_equal"), layout: "equal",
            motion_key: "studio-equal-tabs", tabs: [
            #{ value: "status", label: "Status", icon: music_icon, content: text("Foreground healthy") },
            #{ value: "trace", label: "Trace delivery", icon: trace_icon, content: text("No failed deliveries") },
            #{ value: "locked", label: "Locked", icon: lock_icon, label_visible: false,
                content: text(""), disabled: true }
        ], on_change: Fn("set_specimen_tab_equal") }).with_style(style().width(px(420))),
        tabs::Tabs(#{ label: "Content preview tabs", value: ctx.get_state("specimen_tab_content"), layout: "content",
            motion_key: "studio-content-tabs", tabs: [
            #{ value: "status", label: "Status", content: text("Foreground healthy") },
            #{ value: "trace", label: "详细追踪记录", content: text("No failed deliveries") },
            #{ value: "locked", label: "Locked", content: text(""), disabled: true }
        ], on_change: Fn("set_specimen_tab_content") }),
        tabs::Tabs(#{ label: "Underline preview tabs", value: ctx.get_state("specimen_tab_content"),
            variant: "underline", motion_key: "studio-underline-tabs", tabs: [
            #{ value: "status", label: "Status", content: text("Foreground healthy") },
            #{ value: "trace", label: "详细追踪记录", content: text("No failed deliveries") },
            #{ value: "locked", label: "Locked", content: text(""), disabled: true }
        ], on_change: Fn("set_specimen_tab_content") }).with_style(style().width(px(420))),
        breadcrumb::Breadcrumb(#{key:"specimen-breadcrumb",label:"Theme path",max_items:3,items:[
            #{value:"work",label:"Workspace"},#{value:"themes",label:"Installed themes"},
            #{value:"clean",label:"Clean"},#{value:"dark",label:"Dark"},
        ],on_navigate:Fn("choose_specimen_ancestor")})
            .with_style(style().width(px(260)).max_width(relative(1.0))),
        accordion::Accordion(#{ key: "specimen-accordion", expanded: ctx.get_state("specimen_accordion"), mode: "multiple",
            items: [
                #{ key: "runtime", title: "Runtime", content: text("All checks pass"), content_height: 34 },
                #{ key: "theme", title: "Theme", content: text("Semantic tokens active"), content_height: 34 }
            ], on_change: Fn("set_specimen_accordion") }),
        collapsible::Collapsible(#{ key: "specimen-collapsible", open: ctx.get_state("specimen_collapsible"),
            trigger: text("Release details"), content: text("Source-owned component content"),
            content_height: 36, on_open_change: Fn("set_specimen_collapsible") }),
        tree::Tree(#{key:"specimen-tree",label:"Theme files",items:[
            #{key:"root",parent:(),label:"themes"},#{key:"dark",parent:"root",label:"dark.rhai"},
            #{key:"light",parent:"root",label:"light.rhai"}],expanded:ctx.get_state("specimen_tree_expanded"),
            selected_keys:ctx.get_state("specimen_tree_selected"),active_key:ctx.get_state("specimen_tree_active"),height:112.0,
            on_expanded_change:Fn("set_specimen_tree_expanded"),on_selection_change:Fn("set_specimen_tree_selected"),
            on_active_change:Fn("set_specimen_tree_active")}),
        list::List(#{key:"specimen-list",label:"Theme releases",height:96.0,items:[
            #{key:"r2",title:"Paper 2.1",meta:"Today",badge:#{text:"Draft",variant:"accent"}},
            #{key:"r1",title:"Paper 2.0",secondary:"published",meta:"Mon"}]}),
        selectable_list::SelectableList(#{key:"specimen-selectable-list",label:"Theme authors",height:88.0,
            selection_mode:"single",selected_keys:["a1"],row:ctx.node_factory(Fn("specimen_author_row")),items:[
            #{key:"a1",label:"Ada",name:"Ada",themes:"4 themes"},#{key:"a2",label:"Grace",name:"Grace",themes:"2 themes"}]}),
        row([
            tabs::Tabs(#{key:"specimen-tab-bar",label:"Theme files",value:"dark",variant:"bar",panel:false,tabs:[
                #{value:"dark",label:"dark.rhai",closable:true,count:12},
                #{value:"light",label:"light.rhai",dirty:true,count:3,tone:"accent",tone_label:"changed"}]}),
            tab_overflow_menu::TabOverflowMenu(#{key:"specimen-tab-menu",label:"All theme files",value:"dark",
                items:[#{value:"dark",label:"dark.rhai"},#{value:"light",label:"light.rhai"}]}),
        ]).with_style(style().items_center()),
        carousel::Carousel(#{key:"specimen-carousel",label:"Theme notes",index:1,indicator:"dots",items:[
            #{label:"Contrast",content:text("Primary text holds 7:1 on every surface")},
            #{label:"Steps",content:text("Surface steps stay perceptually even")},
            #{label:"Accent",content:text("The accent keeps its original hue")}]}),
        sliding_panel::SlidingPanel(#{key:"specimen-sliding",value:"dark",order:["dark","light"],
            content:text("A new panel slides in from the side it comes from")}),
    ]).with_style(style().gap(px(7)))
}

fn specimen_data(ctx) {
    let rows = [
        #{ id: "one", name: "Ada", state: "Active", score: 98 },
        #{ id: "two", name: "Lin", state: "Away", score: 84 },
        #{ id: "three", name: "Grace", state: "Offline", score: 91 },
    ];
    let columns = [
        #{ key: "name", title: "Name", width: #{ kind: "flex", value: 1 }, sortable: true },
        #{ key: "state", title: "State", width: #{ kind: "fixed", value: 120 } },
        #{ key: "score", title: "Score", width: #{ kind: "fixed", value: 90 }, align: "end" },
    ];
    let direction = ctx.get_state("specimen_table_sort_direction");
    let current_sort = if direction == "none" { () } else {
        #{ key: ctx.get_state("specimen_table_sort_key"), direction: direction }
    };
    let sortable_items = [];
    for key in ctx.get_state("specimen_sort_order") {
        sortable_items.push(#{ key:key, label:key,
            content:text(key.to_upper()).with_style(style().padding(theme_spacing("sm"))) });
    }
    column([
        specimen_title("DATA", "Real virtualized rows and pagination."),
        split_pane::SplitPane(#{ key: "specimen-split", label: "Resize data navigator",
            size: ctx.get_state("specimen_split_size"), min_start: 100.0, min_end: 140.0,
            start: text("Navigator").with_style(style().padding(px(10)).border(px(1)).border_color(theme_color("border"))),
            end: text("Preview").with_style(style().padding(px(10)).border(px(1)).border_color(theme_color("border"))),
            on_resize: Fn("set_specimen_split_size")
        }).with_style(style().width(relative(1.0)).height(px(72))),
        resizable::Resizable(#{ key: "specimen-resizable", label: "Resize specimen card",
            rect: ctx.get_state("specimen_resizable_rect"), min_width:120.0, min_height:52.0,
            max_width:360.0, max_height:120.0,
            content: text("Resizable surface").with_style(style().padding(px(10))
                .background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border"))),
            on_resize: Fn("set_specimen_resizable")
        }).with_style(style().width(relative(1.0)).height(px(112))
            .border(px(1)).border_color(theme_color("border"))),
        draggable::Draggable(#{ key: "specimen-draggable", label: "Move specimen card",
            position: ctx.get_state("specimen_draggable_position"), contain: true,
            handle: text("DRAG HANDLE").with_style(style().height(px(26)).padding_x(theme_spacing("sm"))
                .items_center().typography("caption").font_weight(700).background(theme_color("surface_hover"))),
            content: text("Movable surface").with_style(style().width(px(170)).height(px(40))
                .padding(theme_spacing("sm")).background(theme_color("surface_raised"))
                .border(px(1)).border_color(theme_color("border"))),
            on_move: Fn("set_specimen_draggable")
        }).with_style(style().width(relative(1.0)).height(px(108))
            .border(px(1)).border_color(theme_color("border"))),
        row([
            drag_source::DragSource(#{ key:"specimen-drag-source", label:"Move artifact",
                source_id:"studio-artifact", payload_type:"artifact", payload:#{id:"studio-artifact"},
                operation:"move", keyboard_target:"studio-drop-zone",
                content:text("DRAG ARTIFACT").with_style(style().height(px(48)).padding(theme_spacing("sm"))
                    .items_center().background(theme_color("surface_raised"))
                    .border(px(1)).border_color(theme_color("border"))),
                on_drag_end:Fn("record_action").curry("Drag finished")
            }).with_style(style().width(relative(0.5))),
            drop_zone::DropZone(#{ key:"specimen-drop-zone", label:"Artifact drop zone",
                target_id:"studio-drop-zone", payload_types:["artifact"], operations:["move"],
                content:text("DROP ZONE").with_style(style().height(px(48)).padding(theme_spacing("sm"))
                    .items_center().background(theme_color("surface"))
                    .border(px(1)).border_color(theme_color("border"))),
                on_drop:Fn("record_action").curry("Drop committed")
            }).with_style(style().width(relative(0.5)))
        ]).with_style(style().gap(theme_spacing("sm"))),
        sortable::Sortable(#{ key:"specimen-sortable", label:"Release order",
            items:sortable_items, direction:"horizontal", on_reorder:Fn("set_specimen_sortable") }),
        pan_zoom::PanZoom(#{ key:"specimen-pan-zoom", label:"Canvas viewport",
            transform:ctx.get_state("specimen_pan_zoom"), min_scale:0.5, max_scale:3.0,
            content:canvas(canvas_scene([
                canvas_circle("center",90.0,55.0,18.0,theme_color("accent")),
                canvas_rect("node",24.0,22.0,34.0,24.0,theme_color("success"))
            ])).with_key("specimen-pan-zoom-canvas"),
            on_transform_change:Fn("set_specimen_pan_zoom")
        }).with_style(style().width(relative(1.0)).height(px(110))),
        rotatable::Rotatable(#{key:"specimen-rotatable",label:"Rotate canvas",
            angle:ctx.get_state("specimen_rotation"),pivot:#{x:80.0,y:50.0},snap:15.0,
            content:canvas(canvas_scene([
                canvas_rect("arm",80.0,42.0,56.0,16.0,theme_color("accent")),
                canvas_circle("pivot",80.0,50.0,5.0,theme_color("warning"))
            ])).with_key("specimen-rotation-canvas"),
            handle:text("↻"),on_rotate:Fn("set_specimen_rotation")
        }).with_style(style().width(relative(1.0)).height(px(100))),
        selection_area::SelectionArea(#{key:"specimen-selection",label:"Select canvas nodes",
            targets:[#{key:"one",x:18.0,y:18.0,width:46.0,height:30.0},
                #{key:"two",x:92.0,y:48.0,width:52.0,height:34.0}],
            selected_keys:ctx.get_state("specimen_selection"),
            active_key:ctx.get_state("specimen_selection_active"),
            anchor_key:ctx.get_state("specimen_selection_anchor"),
            content:canvas(canvas_scene([
                canvas_rect("one",18.0,18.0,46.0,30.0,theme_color("accent")),
                canvas_rect("two",92.0,48.0,52.0,34.0,theme_color("surface_hover"))
            ])).with_key("specimen-selection-canvas"),on_selection_change:Fn("set_specimen_selection")
        }).with_style(style().width(relative(1.0)).height(px(100))),
        table::Table(#{ key: "specimen-table", label: "Users", row_key: "id",
            rows: rows, columns: columns, height: 126, striped: true, resizable_columns: true,
            selection_mode: "multiple", selected_keys: ctx.get_state("specimen_table_selected"),
            sort: current_sort, group_by: "state", collapsed_groups: ctx.get_state("specimen_collapsed_groups"),
            on_selection_change: Fn("set_specimen_table_selection"), on_sort_change: Fn("set_specimen_table_sort"),
            on_row_click: Fn("record_value").curry("Row"), on_group_toggle: Fn("set_specimen_group"),
            on_column_resize: Fn("record_action").curry("Column resized") }),
        pagination::Pagination(#{ key: "specimen-pages", label: "Users pagination", total_items: 128,
            current_page: ctx.get_state("specimen_page"), page_size: ctx.get_state("specimen_page_size"),
            page_size_options: [10, 25, 50], on_change: Fn("set_specimen_page") })
    ]).with_style(style().gap(px(7)))
}

fn set_specimen_column_path(ctx, value) { ctx.set_state("specimen_column_path", value.path); }
fn set_specimen_card_values(ctx, value) { ctx.set_state("specimen_card_values", value); }
fn set_specimen_clock(ctx, value) { ctx.set_state("specimen_clock", value); }
fn set_specimen_time_range(ctx, value) { ctx.set_state("specimen_time_range", value); }
fn set_specimen_date_range(ctx, value) { ctx.set_state("specimen_date_range", value); }
fn specimen_date_range(ctx) {
    let selection=ctx.get_state("specimen_date_range");
    date_range_picker::DateRangePicker(#{key:"specimen-date-range",label:"Date range",today:"2026-10-16",
        value:if selection==(){#{start:"2026-10-01",end:"2026-10-08"}}else{selection.range},
        compare_mode:if selection==(){"off"}else{selection.compare_mode},on_change:Fn("set_specimen_date_range")})
}
fn specimen_time(ctx) {
    let clock=ctx.get_state("specimen_clock");
    column([
        calendar::Calendar(#{key:"specimen-calendar",label:"Calendar",value:ctx.get_state("specimen_date"),on_change:Fn("set_specimen_date")}),
        time_input::TimeInput(#{key:"specimen-clock",label:"Time",value:if clock==""{ctx.format_time(0,"short","UTC")}else{clock},on_change:Fn("set_specimen_clock")}),
        time_range_picker::TimeRangePicker(#{key:"specimen-time-range",label:"Query range",value:ctx.get_state("specimen_time_range"),on_change:Fn("set_specimen_time_range")}),
        specimen_date_range(ctx),
        timeline::Timeline(#{key:"specimen-timeline",label:"Deployment",height:160,items:[
            #{key:"build",title:"Build",status:"success"},#{key:"deploy",title:"Deploy",status:"running",current:true}]}),
    ]).with_style(style().gap(theme_spacing("sm")))
}
fn specimen_columns(ctx) {
    let path = ctx.get_state("specimen_column_path");
    let columns = [#{key:"theme-family",title:"Family",items:[
        #{key:"default",title:"Default",has_children:true},
        #{key:"catppuccin",title:"Catppuccin",has_children:true}]}];
    if path.len > 0 { columns.push(#{key:`variants-${path[0]}`,title:"Variant",items:[
        #{key:"light",title:"Light"},#{key:"dark",title:"Dark"}]}); }
    column_browser::ColumnBrowser(#{key:"specimen-column-browser",label:"Theme hierarchy",
        path:path,columns:columns,height:140.0,column_width:144.0,
        on_select:Fn("set_specimen_column_path")})
}

fn specimen_documents() {
    let left = "fn deploy(host) {\n    let port = 8080;\n    log(`deploy ${host}:${port}`);\n}\n";
    let right = "fn deploy(host) {\n    let port = 8443;\n    log(`deploy ${host}:${port}`);\n    verify(host);\n}\n";
    column([
        specimen_title("DOCUMENTS", "Native code and neutral two-way diff surfaces."),
        code_viewer::CodeViewer(#{ key: "gallery-code", label: "Rhai source",
            file_name: "deploy.rhai", language: "rhai", source: right,
            style: style().height(px(180)) }),
        text("UNIFIED").with_style(style().typography("caption")
            .font_weight(700).text_color(theme_color("text_muted"))),
        diff_viewer::DiffViewer(#{ key: "gallery-diff-unified", mode: "unified",
            left: #{ source: left, label: "Server A", file_name: "deploy.rhai", language: "rhai" },
            right: #{ source: right, label: "Server B", file_name: "deploy.rhai", language: "rhai" },
            style: style().height(px(190)) }),
        text("SPLIT").with_style(style().typography("caption")
            .font_weight(700).text_color(theme_color("text_muted"))),
        diff_viewer::DiffViewer(#{ key: "gallery-diff", mode: "split",
            left: #{ source: left, label: "Server A", file_name: "deploy.rhai", language: "rhai" },
            right: #{ source: right, label: "Server B", file_name: "deploy.rhai", language: "rhai" },
            style: style().height(px(230)) })
    ]).with_style(style().gap(px(7)))
}

fn p0_commands() {
    [
        #{ value: "new", label: "New window", keywords: ["create"], group: "Window", shortcut: "⌘N" },
        #{ value: "theme", label: "Switch theme", keywords: ["appearance", "color"], group: "Preferences", shortcut: "⌘T" },
        #{ value: "disabled", label: "Unavailable action", group: "Preferences", disabled: true }
    ]
}

fn specimen_p0(ctx) {
    let joined = button_group::ButtonGroup(#{ label: "Alignment", buttons: [
        button::Button(#{ text: "Left", variant: if ctx.get_state("specimen_alignment") == "left" { "secondary" } else { "outline" },
            size: "sm", on_click: Fn("set_specimen_alignment").curry("left") }),
        button::Button(#{ text: "Center", variant: if ctx.get_state("specimen_alignment") == "center" { "secondary" } else { "outline" },
            size: "sm", on_click: Fn("set_specimen_alignment").curry("center") }),
        button::Button(#{ text: "Right", variant: if ctx.get_state("specimen_alignment") == "right" { "secondary" } else { "outline" },
            size: "sm", on_click: Fn("set_specimen_alignment").curry("right") })
    ] });
    let grouped_input = input_group::InputGroup(#{ label: "Search", prefix: icon::Icon(#{
            source: asset("app/icons/search"), size: "md", label: "Search" }),
        suffix: kbd::Kbd(#{ text: "⌘ K" }),
        control: input::Input(#{ key: "p0-input", label: "Search", value: ctx.get_state("p0_input"),
            placeholder: "Search", on_change: Fn("set_p0_input") }) });
    let contextual = context_menu::ContextMenu(#{ key: "p0-context", label: "Card actions",
        trigger: card::Card(#{ content: text("Right-click this card") }),
        open: ctx.get_state("p0_context_open"), active_value: ctx.get_state("p0_context_active"),
        items: [#{ kind: "item", value: "copy", label: "Copy", shortcut: "⌘C" },
            #{ kind: "item", value: "delete", label: "Delete" }],
        on_action: Fn("record_value").curry("Context menu"), on_active_change: Fn("set_p0_context_active"),
        on_open_change: Fn("set_p0_context_open") });
    column([
        specimen_title("PUBLIC-LAUNCH FOUNDATIONS", "Semantic surfaces, joined controls, native range input and keyboard command UI."),
        alert::Alert(#{ title: "Runtime ready", description: "All component sources use the public runtime.", variant: "success" }),
        row([
            badge::Badge(#{ text: "NEW", variant: "accent" }),
            badge::Badge(#{ text: "HEALTHY", variant: "success", dot: true }),
            badge::Badge(#{ text: "WARNING", variant: "warning" }),
            spinner::Spinner(#{ key: "p0-spinner", label: "Loading", size: "sm" }),
            kbd::Kbd(#{ text: "⌘ ⇧ P", label: "Command Shift P" })
        ]).with_style(style().gap(px(6)).items_center().flex_wrap()),
        joined,
        grouped_input,
        scroll_area::ScrollArea(#{ key: "p0-scroll", label: "Scrollable specimen", height: px(96), vertical_scrollbar: "always",
            content: column([
                text("Scrollable line 1").with_style(style().height(px(28)).items_center()),
                text("Scrollable line 2").with_style(style().height(px(28)).items_center()),
                text("Scrollable line 3").with_style(style().height(px(28)).items_center()),
                text("Scrollable line 4").with_style(style().height(px(28)).items_center()),
                text("Scrollable line 5").with_style(style().height(px(28)).items_center())
            ]).with_style(style().padding(px(8))) }),
        row([
            toggle::Toggle(#{ text: "Pinned", pressed: ctx.get_state("p0_pinned"),
                on_pressed_change: Fn("set_p0_pinned") }),
            toggle_group::ToggleGroup(#{ key: "p0-formats", label: "Formatting", mode: "multiple",
                values: ctx.get_state("p0_toggle_values"),
                items: [#{ value: "bold", label: "Bold" }, #{ value: "italic", label: "Italic" }],
                on_change: Fn("set_p0_toggles") }),
            slider::Slider(#{ key: "p0-slider", label: "Volume", value: ctx.get_state("p0_slider"),
                min: 0.0, max: 100.0, step: 1.0, on_change: Fn("set_p0_slider") }),
            range_slider::RangeSlider(#{ key:"p0-range", label:"Deployment window",
                values:ctx.get_state("specimen_range"), min:0.0, max:100.0, step:5.0,
                minimum_gap:10.0, on_change:Fn("set_specimen_range") })
        ]).with_style(style().gap(px(10)).items_center().flex_wrap()),
        row([
            card::Card(#{ header: text("Card"), content: text("Composable content surface") }),
            group_box::GroupBox(#{ label: "Group box", description: "Related desktop controls",
                content: text("Persistent layout content") }),
            empty::Empty(#{ title: "Nothing here", description: "An empty state with a clear next action.",
                actions: [button::Button(#{ text: "Create", size: "sm",
                    on_click: Fn("record_action").curry("Create empty-state item") })] })
        ]).with_style(style().gap(px(8)).items_start().flex_wrap()),
        command::Command(#{ key: "p0-command", label: "Commands", query: ctx.get_state("p0_query"),
            active_value: ctx.get_state("p0_command_active"), items: p0_commands(), max_visible: 5,
            on_query_change: Fn("set_p0_query"), on_active_change: Fn("set_p0_command_active"),
            on_action: Fn("record_value").curry("Command") }),
        contextual,
        row([
            button::Button(#{ text: "Command dialog", size: "sm", variant: "outline", on_click: Fn("open_p0_command") }),
            button::Button(#{ text: "Sheet", size: "sm", variant: "outline", on_click: Fn("open_p0_sheet") }),
            button::Button(#{ text: "Alert dialog", size: "sm", variant: "danger", on_click: Fn("open_p0_alert") })
        ]).with_style(style().gap(px(6)).flex_wrap()),
        command_dialog::CommandDialog(#{ key: "p0-command-dialog", open: ctx.get_state("p0_command_open"),
            query: ctx.get_state("p0_query"), active_value: ctx.get_state("p0_command_active"),
            label: "Command palette", items: p0_commands(), on_query_change: Fn("set_p0_query"),
            on_active_change: Fn("set_p0_command_active"), on_action: Fn("record_value").curry("Command dialog"),
            on_open_change: Fn("set_p0_command_open") }),
        sheet::Sheet(#{ key: "p0-sheet", open: ctx.get_state("p0_sheet_open"), side: "end",
            title: "Inspector", content: text("A temporary viewport-edge panel."),
            on_open_change: Fn("set_p0_sheet_open") }),
        alert_dialog::AlertDialog(#{ key: "p0-alert-dialog", open: ctx.get_state("p0_alert_open"),
            title: "Apply destructive action?", description: "This is the standard confirmation boundary.",
            on_confirm: Fn("confirm_p0_alert"), on_cancel: Fn("cancel_p0_alert"), on_open_change: Fn("set_p0_alert_open") })
    ]).with_style(style().gap(px(8)))
}

fn specimen_overlays(ctx) {
    let menu_node = menu::Menu(#{ key: "specimen-menu", label: "Example menu",
        trigger: button::Button(#{ text: "Menu", size: "sm", variant: "outline" }),
        open: ctx.get_state("menu_open"), active_value: ctx.get_state("specimen_menu_active"),
        items: [#{ kind: "item", value: "open", label: "Open", checked: true },
            #{ kind: "item", value: "disabled", label: "Disabled", disabled: true }],
        on_action: Fn("specimen_menu_action"), on_active_change: Fn("set_specimen_menu_active"), on_open_change: Fn("set_menu") });
    let popover_node = popover::Popover(#{ key: "specimen-popover", label: "Example popover",
        trigger: button::Button(#{ text: "Popover", size: "sm", variant: "outline" }),
        content: text("Opaque themed popover"), open: ctx.get_state("popover_open"),
        on_open_change: Fn("set_popover") });
    let tooltip_node = tooltip::Tooltip(#{ key: "specimen-tooltip", label: "Example tooltip",
        trigger: button::Button(#{ text: "Tooltip", size: "sm", variant: "ghost" }),
        content: text("Delayed native tooltip"), placement: "top" });
    let hover_node = hover_card::HoverCard(#{key:"specimen-hover-card",label:"Preview surface",
        trigger:button::Button(#{text:"Preview",size:"sm",variant:"ghost"}),
        content:column([text("Live themed preview"),button::Button(#{text:"Keep preview",
            variant:"secondary",size:"sm",on_click:Fn("record_value").curry("Hover preview")})]).with_style(style().gap(px(8)))});
    let dialog_node = dialog::Dialog(#{ key: "specimen-dialog", title: "Confirm theme",
        content: text("Dialog colors come from the active draft."), open: ctx.get_state("dialog_open"),
        actions: [button::Button(#{ text: "Cancel", variant: "outline",
                on_click: Fn("close_dialog_with").curry("Dialog cancelled") }),
            button::Button(#{ text: "Apply", variant: "primary",
                on_click: Fn("close_dialog_with").curry("Dialog applied") })],
        on_open_change: Fn("set_dialog") });
    let toast_items = if ctx.get_state("toast_visible") {
        [#{ id: "studio-toast", title: "Theme applied", message: "Live preview updated",
            variant: "success", duration_ms: 60000 }]
    } else { [] };
    column([
        specimen_title("OVERLAYS", "Open real menu, popover, dialog, tooltip and toast surfaces."),
        row([
            menu_node, popover_node, tooltip_node, hover_node,
            button::Button(#{ text: "Dialog", size: "sm", variant: "outline", on_click: Fn("show_dialog") }),
            button::Button(#{ text: "Toast", size: "sm", variant: "outline", on_click: Fn("show_toast") }),
        ]).with_style(style().gap(px(5)).items_center().flex_wrap()),
        dialog_node,
        toast::Toast(#{ key: "specimen-toasts", items: toast_items, on_dismiss: Fn("dismiss_toast") })
    ]).with_style(style().gap(px(7)))
}

fn specimen_controls(ctx) {
    let left = column([specimen_typography(), specimen_actions(ctx), specimen_choices(ctx), specimen_status(), specimen_chrome(ctx)])
        .with_style(style().flex_grow().min_width(px(0)).gap(px(14)));
    let right = column([specimen_inputs(ctx)])
        .with_style(style().flex_grow().min_width(px(0)).gap(px(14)));
    let columns = if ctx.viewport_class() == "wide" {
        row([left, right]).with_style(style().gap(px(14)).items_start())
    } else {
        column([left, right]).with_style(style().gap(px(14)))
    };
    column([
        row([
            column([
                text("COMPONENT SPECIMEN").with_style(style().typography("heading")),
                text("Typography, form fields and controls · live component previews")
                    .with_style(style().typography("caption").text_color(theme_color("text_muted")))
            ]).with_style(style().gap(px(2))),
            tag::Tag(#{ text: `${ctx.get_state("family")} / ${ctx.get_state("variant")}`, variant: "accent" })
        ]).with_style(style().justify_between().items_center().padding_bottom(px(8))
            .border_bottom(px(1)).border_color(theme_color("border"))),
        columns,
        selectable_card_group::SelectableCardGroup(#{key:"specimen-cards",label:"Deployment region",
            values:ctx.get_state("specimen_card_values"),on_change:Fn("set_specimen_card_values"),
            items:[#{value:"east",label:"East",content:text("Healthy")},
                #{value:"west",label:"West",content:text("Standby")}]}),
        specimen_p0(ctx)
    ]).with_style(style().padding(px(16)).gap(px(14)).typography("body"))
}

fn specimen_data_and_documents(ctx) {
    column([
        specimen_data(ctx),
        specimen_columns(ctx),
        specimen_time(ctx),
        markdown_viewer::MarkdownViewer(#{key:"specimen-markdown",label:"Markdown",source:"## Native content\n\n**Readable** text and `inline code`.\n\n- A semantic list\n- Theme typography"})
            .with_style(style().height(px(200))),
        specimen_documents()
    ]).with_style(style().padding(px(16)).gap(px(14)).typography("body"))
}

fn specimen_navigation_and_overlays(ctx) {
    column([specimen_navigation(ctx), specimen_overlays(ctx)])
        .with_style(style().padding(px(16)).gap(px(14)).typography("body"))
}

fn gallery_theme_options(ctx) {
    let variants=ctx.theme_variants();let options=[];
    for variant in variants {
        let siblings=variants.filter(|other|other.family==variant.family).len;
        options.push(#{value:gallery_theme_key(variant),label:if siblings>1{`${variant.family} · ${variant.name}`}else{variant.family}});
    }
    options
}

fn gallery_header(ctx) {
    let categories = [];
    for entry in [#{ value: "all", label: "Overview" }, #{ value: "foundations", label: "Foundations" },
        #{ value: "forms", label: "Forms" }, #{ value: "navigation", label: "Navigation & data" },
        #{ value: "documents", label: "Code & diff" },
        #{ value: "overlays", label: "Commands & overlays" },
        #{ value: "product", label: "Product cards" }] {
        categories.push(button::Button(#{ text: entry.label, size: "sm",
            variant: if ctx.get_state("gallery_category") == entry.value { "primary" } else { "ghost" } })
            .on_click_value(Fn("set_gallery_category").curry(entry.value), ()));
    }
    row(categories).with_style(style().gap(px(4)).padding_x(px(14)).padding_y(px(8))
        .flex_wrap().border_bottom(px(1)).border_color(theme_color("border"))
        .background(theme_color("surface")))
}

fn gallery_title_bar(ctx) {
    let compact = ctx.viewport_class() == "compact";
    let themes = combobox::Combobox(#{ key: "gallery-theme", label: "Gallery theme", options: gallery_theme_options(ctx), mode: "single",
            selected: [gallery_theme_key(ctx.theme_variant())], open: ctx.get_state("gallery_theme_open"),
            query: ctx.get_state("gallery_theme_query"), searchable: true, placeholder: "Theme",
            width: px(if compact { 150 } else { 240 }), size: "xs",
            on_change: Fn("set_gallery_theme"), on_open_change: Fn("set_gallery_theme_open"),
            on_query_change: Fn("set_gallery_theme_query") });
    let breadcrumb = row([
        text("GPUI RHAI").with_style(style().typography("body_small").font_weight(700)),
        text("/").with_style(style().typography("caption").text_color(theme_color("text_muted"))),
        text("COMPONENT GALLERY").with_style(style().typography("body_small")
            .text_color(theme_color("text_muted")))
    ]).with_style(style().min_width(px(0)).gap(px(6)).items_center().overflow_hidden());
    let config = #{ label: "GPUI Rhai Component Gallery", title: breadcrumb,
        inset_start: 70, end: [themes] };
    if !compact { config.subtitle = "62 source-owned components · native GPUI behavior · live themes"; }
    title_bar::TitleBar(config)
}

fn gallery_status_bar(ctx) {
    status_bar::StatusBar(#{ label: "Component Gallery status",
        start: [text(`62 components · ${ctx.get_state("interaction_count")} interactions`)],
        center: [text(ctx.get_state("interaction_status"))],
        end: [text(ctx.get_state("gallery_category")), text(ctx.viewport_class())] })
}

fn gallery_content(ctx) {
    let category = ctx.get_state("gallery_category");
    if category == "foundations" {
        column([specimen_actions(ctx), specimen_status(), specimen_chrome(ctx), specimen_p0(ctx)])
            .with_style(style().padding(px(14)).gap(px(14)))
    } else if category == "forms" {
        column([specimen_inputs(ctx), specimen_choices(ctx)])
            .with_style(style().padding(px(14)).gap(px(14)))
    } else if category == "navigation" {
        column([specimen_navigation(ctx), specimen_data(ctx)])
            .with_style(style().padding(px(14)).gap(px(14)))
    } else if category == "documents" {
        column([specimen_documents()])
            .with_style(style().padding(px(14)).gap(px(14)))
    } else if category == "overlays" {
        column([specimen_overlays(ctx)])
            .with_style(style().padding(px(14)).gap(px(14)))
    } else if category == "product" {
        specimen_product_cards()
    } else { specimen_controls(ctx) }
}

fn set_preview_section(section, ctx, payload) { ctx.set_state("preview_section", section); }

fn studio_preview(ctx) {
    let section = ctx.get_state("preview_section");
    let choices = [];
    for entry in [#{value:"controls",label:"Controls"}, #{value:"navigation",label:"Navigation & overlays"},
        #{value:"data",label:"Data & documents"}, #{value:"product",label:"Product cards"}] {
        choices.push(button::Button(#{key:entry.value, text:entry.label, size:"sm",
            variant:if section == entry.value {"primary"} else {"ghost"}})
            .accessibility_pressed(section == entry.value)
            .with_key(`preview-switch-${entry.value}`).on_click_value(Fn("set_preview_section").curry(entry.value), ()));
    }
    let preview = if section == "navigation" { specimen_navigation_and_overlays(ctx) }
        else if section == "data" { specimen_data_and_documents(ctx) }
        else if section == "product" { specimen_product_cards() }
        else { specimen_controls(ctx) };
    column([
        row(choices).with_style(style().gap(px(4)).padding_x(px(14)).padding_y(px(8)).flex_shrink(false).flex_wrap()
            .border_bottom(px(1)).border_color(theme_color("border"))),
        column([preview]).with_key(`preview-contents-${section}`).with_style(style().flex_grow().min_height(px(0)).overflow_y_scroll())
    ]).with_style(style().flex_grow().min_width(px(0)).min_height(px(0)))
}

fn view(ctx) {
    if __GALLERY_ONLY__ {
        column([
            gallery_title_bar(ctx),
            gallery_header(ctx),
            column([gallery_content(ctx)]).with_style(style().flex_grow().min_height(px(0))
                .overflow_y_scroll().background(theme_color("surface"))),
            gallery_status_bar(ctx)
        ]).with_style(style().width(relative(1.0)).height(relative(1.0)).background(theme_color("surface")))
    } else {
        row([
            editor(ctx),
            studio_preview(ctx)
        ]).with_style(style().width(relative(1.0)).height(relative(1.0))
            .background(theme_color("surface")))
    }
}
