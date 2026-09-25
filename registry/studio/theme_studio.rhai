import "components/accordion" as accordion;
import "components/avatar" as avatar;
import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/checkbox" as checkbox;
import "components/collapsible" as collapsible;
import "components/date_picker" as date_picker;
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
import "components/tabs" as tabs;
import "components/tag" as tag;
import "components/textarea" as textarea;
import "components/toast" as toast;
import "components/tooltip" as tooltip;
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
import "components/spinner" as spinner;
import "components/toggle" as toggle;
import "components/toggle_group" as toggle_group;
import "components/scroll_area" as scroll_area;
import "components/title_bar" as title_bar;
import "components/status_bar" as status_bar;
import "components/code_viewer" as code_viewer;
import "components/diff_viewer" as diff_viewer;

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
        p0_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        p0_command_active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "new" } },
        p0_command_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_COMMAND__ } },
        p0_sheet_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_SHEET__ } },
        p0_alert_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_ALERT__ } },
        p0_context_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        p0_context_active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "copy" } },
        gallery_theme: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: __GALLERY_THEME__ } },
        gallery_theme_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        gallery_theme_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        gallery_category: #{ schema: #{ type: "string", allowed: ["all", "foundations", "forms", "navigation", "documents", "overlays"] },
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
fn set_specimen_accordion(ctx, value) { ctx.set_state("specimen_accordion", value); record_action("Accordion changed", ctx, ()); }
fn set_specimen_collapsible(ctx, value) { ctx.set_state("specimen_collapsible", value); record_value("Release details", ctx, value); }
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
fn set_gallery_theme(ctx, values) {
    if values.len == 0 { return; }
    let value = values[0];
    if value == "default_dark" { ctx.set_theme("Default", "Dark"); }
    else if value == "default_light" { ctx.set_theme("Default", "Light"); }
    else if value == "tokyo_night" { ctx.set_theme("Tokyo Night", "Night"); }
    else if value == "tokyo_storm" { ctx.set_theme("Tokyo Night", "Storm"); }
    else if value == "catppuccin_latte" { ctx.set_theme("Catppuccin", "Latte"); }
    else if value == "catppuccin_mocha" { ctx.set_theme("Catppuccin", "Mocha"); }
    else if value == "ethereal" { ctx.set_theme("Ethereal", "Dark"); }
    else if value == "everforest" { ctx.set_theme("Everforest", "Dark"); }
    else if value == "gruvbox" { ctx.set_theme("Gruvbox", "Dark"); }
    else if value == "hackerman" { ctx.set_theme("Hackerman", "Dark"); }
    else if value == "nord" { ctx.set_theme("Nord", "Dark"); }
    else if value == "retro_82" { ctx.set_theme("Retro 82", "Dark"); }
    else if value == "hermarchy" { ctx.set_theme("Hermarchy", "Dark"); }
    else if value == "futurism" { ctx.set_theme("Futurism", "Dark"); }
    else if value == "aetheria" { ctx.set_theme("Aetheria", "Dark"); }
    ctx.set_state("gallery_theme", value);
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
        specimen_title("NAVIGATION", "Tabs use one continuous track and one inset selection surface."),
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
        accordion::Accordion(#{ key: "specimen-accordion", expanded: ctx.get_state("specimen_accordion"), mode: "multiple",
            items: [
                #{ key: "runtime", title: "Runtime", content: text("All checks pass"), content_height: 34 },
                #{ key: "theme", title: "Theme", content: text("Semantic tokens active"), content_height: 34 }
            ], on_change: Fn("set_specimen_accordion") }),
        collapsible::Collapsible(#{ key: "specimen-collapsible", open: ctx.get_state("specimen_collapsible"),
            trigger: text("Release details"), content: text("Source-owned component content"),
            content_height: 36, on_open_change: Fn("set_specimen_collapsible") })
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
    column([
        specimen_title("DATA", "Real virtualized rows and pagination."),
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
                min: 0.0, max: 100.0, step: 1.0, on_change: Fn("set_p0_slider") })
        ]).with_style(style().gap(px(10)).items_center().flex_wrap()),
        row([
            card::Card(#{ header: text("Card"), content: text("Composable content surface"), elevated: true }),
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
            menu_node, popover_node, tooltip_node,
            button::Button(#{ text: "Dialog", size: "sm", variant: "outline", on_click: Fn("show_dialog") }),
            button::Button(#{ text: "Toast", size: "sm", variant: "outline", on_click: Fn("show_toast") }),
        ]).with_style(style().gap(px(5)).items_center()),
        dialog_node,
        toast::Toast(#{ key: "specimen-toasts", items: toast_items, on_dismiss: Fn("dismiss_toast") })
    ]).with_style(style().gap(px(7)))
}

fn specimen(ctx) {
    let left = column([specimen_typography(), specimen_actions(ctx), specimen_choices(ctx), specimen_status(), specimen_chrome(ctx)])
        .with_style(style().flex_grow().min_width(px(0)).gap(px(14)));
    let right = column([specimen_inputs(ctx), specimen_navigation(ctx), specimen_overlays(ctx)])
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
                text("Every official component · one live semantic contract")
                    .with_style(style().typography("caption").text_color(theme_color("text_muted")))
            ]).with_style(style().gap(px(2))),
            tag::Tag(#{ text: `${ctx.get_state("family")} / ${ctx.get_state("variant")}`, variant: "accent" })
        ]).with_style(style().justify_between().items_center().padding_bottom(px(8))
            .border_bottom(px(1)).border_color(theme_color("border"))),
        columns,
        specimen_data(ctx),
        specimen_documents(),
        specimen_p0(ctx)
    ]).with_style(style().padding(px(16)).gap(px(14)).typography("body"))
}

fn gallery_theme_options() {
    [
        #{ value: "default_dark", label: "Default Dark" }, #{ value: "default_light", label: "Default Light" },
        #{ value: "tokyo_night", label: "Tokyo Night" }, #{ value: "tokyo_storm", label: "Tokyo Storm" },
        #{ value: "catppuccin_latte", label: "Catppuccin Latte" }, #{ value: "catppuccin_mocha", label: "Catppuccin Mocha" },
        #{ value: "ethereal", label: "Ethereal" }, #{ value: "everforest", label: "Everforest" },
        #{ value: "gruvbox", label: "Gruvbox" }, #{ value: "hackerman", label: "Hackerman" },
        #{ value: "nord", label: "Nord" }, #{ value: "retro_82", label: "Retro 82" },
        #{ value: "hermarchy", label: "Hermarchy" }, #{ value: "futurism", label: "Futurism" },
        #{ value: "aetheria", label: "Aetheria" }
    ]
}

fn gallery_header(ctx) {
    let categories = [];
    for entry in [#{ value: "all", label: "All" }, #{ value: "foundations", label: "Foundations" },
        #{ value: "forms", label: "Forms" }, #{ value: "navigation", label: "Navigation & data" },
        #{ value: "documents", label: "Code & diff" },
        #{ value: "overlays", label: "Commands & overlays" }] {
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
    let themes = combobox::Combobox(#{ key: "gallery-theme", label: "Gallery theme", options: gallery_theme_options(), mode: "single",
            selected: [ctx.get_state("gallery_theme")], open: ctx.get_state("gallery_theme_open"),
            query: ctx.get_state("gallery_theme_query"), searchable: true, placeholder: "Theme",
            width: px(if compact { 150 } else { 240 }), trigger_height: 26,
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
    if !compact { config.subtitle = "51 source-owned components · native GPUI behavior · live themes"; }
    title_bar::TitleBar(config)
}

fn gallery_status_bar(ctx) {
    status_bar::StatusBar(#{ label: "Component Gallery status",
        start: [text(`51 components · ${ctx.get_state("interaction_count")} interactions`)],
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
        column([specimen_overlays(ctx), specimen_p0(ctx)])
            .with_style(style().padding(px(14)).gap(px(14)))
    } else { specimen(ctx) }
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
            column([specimen(ctx)]).with_style(style().flex_grow().overflow_y_scroll())
        ]).with_style(style().width(relative(1.0)).height(relative(1.0))
            .background(theme_color("surface")))
    }
}
