import "components/accordion" as accordion;
import "components/avatar" as avatar;
import "components/button" as button;
import "components/checkbox" as checkbox;
import "components/collapsible" as collapsible;
import "components/date_picker" as date_picker;
import "components/dialog" as dialog;
import "components/divider" as divider;
import "components/dropdown" as dropdown;
import "components/form_field" as form_field;
import "components/icon" as icon;
import "components/input" as input;
import "components/label" as label;
import "components/menu" as menu;
import "components/pagination" as pagination;
import "components/popover" as popover;
import "components/progress" as progress;
import "components/radio_group" as radio_group;
import "components/select" as select;
import "components/skeleton" as skeleton;
import "components/switch" as switch_component;
import "components/table" as table;
import "components/tabs" as tabs;
import "components/tag" as tag;
import "components/textarea" as textarea;
import "components/toast" as toast;
import "components/tooltip" as tooltip;

fn color_field(name, value) {
    row([
        row([]).with_style(style().width(px(18)).height(px(18))
            .border(px(1)).border_color(theme_color("border"))
            .background(theme_color(name))),
        column([
            text(name).with_style(style().font_size(px(10)).font_weight(700)
                .text_color(theme_color("text_muted"))),
            input::Input(#{
                key: `studio-${name}`, value: value, placeholder: "#rrggbb",
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
        dialog_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        popover_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        menu_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_MENU__ } },
        toast_visible: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_TOAST__ } },
    } }
}

fn set_dialog(ctx, value) { ctx.set_state("dialog_open", value); }
fn set_popover(ctx, value) { ctx.set_state("popover_open", value); }
fn set_menu(ctx, value) { ctx.set_state("menu_open", value); }
fn show_dialog(ctx, payload) { ctx.set_state("dialog_open", true); }
fn show_toast(ctx, payload) { ctx.set_state("toast_visible", true); }
fn dismiss_toast(ctx, payload) { ctx.set_state("toast_visible", false); }
fn noop(ctx, payload) { handled() }

fn init(ctx) {
    ctx.set_theme(__PREVIEW_FAMILY__, __PREVIEW_VARIANT__);
    ctx.set_locale(__VISUAL_LOCALE__);
}

fn editor(ctx) {
    let mode = ctx.get_state("mode");
    column([
        column([
            text("THEME STUDIO").with_style(style().font_size(px(16)).font_weight(700)),
            text("Edit semantic tokens. Preview real components.")
                .with_style(style().font_size(px(10)).text_color(theme_color("text_muted")))
        ]).with_style(style().gap(px(2)).padding_bottom(px(8))
            .border_bottom(px(1)).border_color(theme_color("border"))),
        label::Label(#{ text: "Theme file", description: "Open edits the file; Import creates a copy." }),
        input::Input(#{ key: "studio-path", value: ctx.get_state("path"),
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
        dropdown::Dropdown(#{
            key: "studio-builtin", placeholder: "Load a bundled theme", searchable: true,
            options: __BUILTIN_OPTIONS__, on_change: native_handler("theme_studio.builtin")
        }),
        divider::Divider(#{}),
        input::Input(#{ key: "studio-family", value: ctx.get_state("family"),
            placeholder: "Family", on_change: native_handler("theme_studio.set_family") }),
        input::Input(#{ key: "studio-variant", value: ctx.get_state("variant"),
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
            .font_size(px(10)).line_height(px(14)).text_color(theme_color("text_muted")))
    ]).with_style(style().width(px(340)).padding(px(14)).gap(px(7))
        .border_end(px(1)).border_color(theme_color("border"))
        .background(theme_color("surface")))
}

fn specimen_title(title, detail) {
    column([
        text(title).with_style(style().font_size(px(12)).font_weight(700)),
        text(detail).with_style(style().font_size(px(10)).line_height(px(14))
            .text_color(theme_color("text_muted")))
    ]).with_style(style().gap(px(2)).padding_bottom(px(4))
        .border_bottom(px(1)).border_color(theme_color("border")))
}

fn specimen_actions() {
    column([
        specimen_title("ACTIONS", "Variants, sizes, tags and disabled states."),
        row([
            button::Button(#{ text: "Primary", variant: "primary" }),
            button::Button(#{ text: "Secondary", variant: "secondary" }),
            button::Button(#{ text: "Outline", variant: "outline" }),
            button::Button(#{ text: "Ghost", variant: "ghost" }),
            button::Button(#{ text: "Danger", variant: "danger" }),
        ]).with_style(style().gap(px(5))),
        row([
            button::Button(#{ text: "XS", size: "xs", variant: "outline" }),
            button::Button(#{ text: "SM", size: "sm", variant: "outline" }),
            button::Button(#{ text: "MD", size: "md", variant: "outline" }),
            button::Button(#{ text: "LG", size: "lg", variant: "outline" }),
            button::Button(#{ text: "Disabled", disabled: true }),
            button::Button(#{ text: "Loading", loading: true, variant: "secondary" }),
        ]).with_style(style().gap(px(5)).items_center()),
        row([
            tag::Tag(#{ text: "ACCENT", variant: "accent" }),
            tag::Tag(#{ text: "SUCCESS", variant: "success" }),
            tag::Tag(#{ text: "WARNING", variant: "warning" }),
            tag::Tag(#{ text: "DANGER", variant: "danger" }),
        ]).with_style(style().gap(px(5)))
    ]).with_style(style().gap(px(7)))
}

fn specimen_inputs() {
    let field = input::Input(#{ key: "specimen-field", value: "Ada Lovelace",
        placeholder: "Name" });
    column([
        specimen_title("INPUTS", "Idle, populated, error, read-only and multiline."),
        row([
            input::Input(#{ key: "specimen-input", value: "", placeholder: "Placeholder" }),
            input::Input(#{ key: "specimen-error", value: "Invalid", error: true }),
            input::Input(#{ key: "specimen-readonly", value: "Read only", read_only: true }),
        ]).with_style(style().gap(px(6))),
        form_field::FormField(#{ id: "specimen-name", label: "Name", required: true,
            control: field, description: "One shared field contract", error: "Example error" }),
        textarea::Textarea(#{ key: "specimen-textarea", value: "Two lines\nof themed text",
            rows: 2, max_length: 120, show_count: true }),
        row([
            select::Select(#{ key: "specimen-select", value: "nord", clearable: true,
                options: [#{ value: "nord", label: "Nord" }, #{ value: "gruvbox", label: "Gruvbox" }] }),
            dropdown::Dropdown(#{ key: "specimen-dropdown", selected: ["dark"],
                options: [#{ value: "dark", label: "Dark" }, #{ value: "light", label: "Light" }] }),
            date_picker::DatePicker(#{ key: "specimen-date", value: "2026-09-01",
                placeholder: "Date", presets: [#{ label: "Today", value: "2026-09-01" }] }),
        ]).with_style(style().gap(px(6)))
    ]).with_style(style().gap(px(7)))
}

fn specimen_choices() {
    column([
        specimen_title("CHOICES", "Persistent selection remains distinct from focus."),
        row([
            checkbox::Checkbox(#{ label: "Checked", checked: true }),
            checkbox::Checkbox(#{ label: "Mixed", checked: false, indeterminate: true }),
            checkbox::Checkbox(#{ label: "Disabled", checked: false, disabled: true }),
        ]).with_style(style().gap(px(8))),
        radio_group::RadioGroup(#{ value: "fast", label: "Mode", orientation: "horizontal",
            options: [#{ value: "safe", label: "Safe" }, #{ value: "fast", label: "Fast" }] }),
        row([
            switch_component::Switch(#{ label: "Enabled", checked: true }),
            switch_component::Switch(#{ label: "Off", checked: false }),
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

fn specimen_navigation() {
    column([
        specimen_title("NAVIGATION", "Tabs, accordion and collapsible use tiled state."),
        tabs::Tabs(#{ label: "Preview tabs", value: "status", tabs: [
            #{ value: "status", label: "Status", content: text("Foreground healthy") },
            #{ value: "trace", label: "Trace", content: text("No failed deliveries") },
            #{ value: "locked", label: "Locked", content: text(""), disabled: true }
        ] }),
        accordion::Accordion(#{ key: "specimen-accordion", expanded: ["runtime"], mode: "multiple",
            items: [
                #{ key: "runtime", title: "Runtime", content: text("All checks pass"), content_height: 34 },
                #{ key: "theme", title: "Theme", content: text("Semantic tokens active"), content_height: 34 }
            ] }),
        collapsible::Collapsible(#{ key: "specimen-collapsible", open: true,
            trigger: text("Release details"), content: text("Source-owned component content"),
            content_height: 36 })
    ]).with_style(style().gap(px(7)))
}

fn specimen_data() {
    let rows = [
        #{ id: "one", name: "Ada", state: "Active", score: 98 },
        #{ id: "two", name: "Lin", state: "Away", score: 84 },
        #{ id: "three", name: "Grace", state: "Offline", score: 91 },
    ];
    let columns = [
        #{ key: "name", title: "Name", width: #{ kind: "flex", value: 160 }, sortable: true },
        #{ key: "state", title: "State", width: #{ kind: "fixed", value: 120 } },
        #{ key: "score", title: "Score", width: #{ kind: "fixed", value: 90 }, align: "end" },
    ];
    column([
        specimen_title("DATA", "Real virtualized rows and pagination."),
        table::Table(#{ key: "specimen-table", label: "Users", row_key: "id",
            rows: rows, columns: columns, height: 126, striped: true,
            selection_mode: "multiple", selected_keys: ["two"],
            sort: #{ key: "name", direction: "ascending" } }),
        pagination::Pagination(#{ key: "specimen-pages", total_items: 128,
            current_page: 3, page_size: 10, page_size_options: [10, 25, 50] })
    ]).with_style(style().gap(px(7)))
}

fn specimen_overlays(ctx) {
    let menu_node = menu::Menu(#{ key: "specimen-menu",
        trigger: button::Button(#{ text: "Menu", size: "sm", variant: "outline" }),
        open: ctx.get_state("menu_open"), active_value: "open",
        items: [#{ kind: "item", value: "open", label: "Open", checked: true },
            #{ kind: "item", value: "disabled", label: "Disabled", disabled: true }],
        on_action: Fn("noop"), on_active_change: Fn("noop"), on_open_change: Fn("set_menu") });
    let popover_node = popover::Popover(#{ key: "specimen-popover",
        trigger: button::Button(#{ text: "Popover", size: "sm", variant: "outline" }),
        content: text("Opaque themed popover"), open: ctx.get_state("popover_open"),
        on_open_change: Fn("set_popover") });
    let tooltip_node = tooltip::Tooltip(#{ key: "specimen-tooltip",
        trigger: button::Button(#{ text: "Tooltip", size: "sm", variant: "ghost" }),
        content: text("Delayed native tooltip"), placement: "top" });
    let dialog_node = dialog::Dialog(#{ key: "specimen-dialog", title: "Confirm theme",
        content: text("Dialog colors come from the active draft."), open: ctx.get_state("dialog_open"),
        actions: [button::Button(#{ text: "Cancel", variant: "outline", on_click: Fn("noop") }),
            button::Button(#{ text: "Apply", variant: "primary", on_click: Fn("noop") })],
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
    column([
        row([
            column([
                text("COMPONENT SPECIMEN").with_style(style().font_size(px(16)).font_weight(700)),
                text("Every official component · one live semantic contract")
                    .with_style(style().font_size(px(10)).text_color(theme_color("text_muted")))
            ]).with_style(style().gap(px(2))),
            tag::Tag(#{ text: `${ctx.get_state("family")} / ${ctx.get_state("variant")}`, variant: "accent" })
        ]).with_style(style().justify_between().items_center().padding_bottom(px(8))
            .border_bottom(px(1)).border_color(theme_color("border"))),
        row([
            column([specimen_actions(), specimen_choices(), specimen_status()])
                .with_style(style().width(relative(0.5)).gap(px(14))),
            column([specimen_inputs(), specimen_navigation(), specimen_overlays(ctx)])
                .with_style(style().width(relative(0.5)).gap(px(14)))
        ]).with_style(style().gap(px(14)).items_start()),
        specimen_data()
    ]).with_style(style().padding(px(16)).gap(px(14)).background(theme_color("surface"))
        .font_size(px(12)).line_height(px(16)))
}

fn view(ctx) {
    row([
        editor(ctx),
        column([specimen(ctx)]).with_style(style().flex_grow().overflow_y_scroll())
    ]).with_style(style().width(relative(1.0)).height(relative(1.0))
        .background(theme_color("surface")))
}
