# Recipes

Complete views copied from the examples in `crates/gpui-rhai/examples/`, which
are built, rendered into checked-in baselines and audited by the test suite.
Start from the closest one; the module reference has every prop they use.

## Settings panel

A Region with a FormLayout of settings: Switch, RadioGroup, ToggleGroup and a searchable Combobox, plus an Accordion and a Popover. From [`settings_panel.rs`](https://github.com/eddix/gpui-rhai/blob/main/crates/gpui-rhai/examples/settings_panel.rs).

```rhai
import "layouts/region" as region;
import "layouts/stack" as stack;
import "patterns/form_layout" as form_layout;
import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/icon" as icon;
import "components/combobox" as combobox_component;
import "components/popover" as popover;
import "components/switch" as switch_component;
import "components/radio_group" as radio_group;
import "components/toggle_group" as toggle_group;
import "components/accordion" as accordion;

fn state_schema() {
    #{
        fields: #{
            theme: #{
                schema: #{ type: "array", max_items: 1, items: #{ type: "string" } },
                "default": #{
                    type: "array",
                    value: [#{ type: "string", value: "__VISUAL_THEME__" }]
                },
            },
            theme_open: #{
                schema: #{ type: "bool" },
                "default": #{ type: "bool", value: false },
            },
            theme_query: #{
                schema: #{ type: "string" },
                "default": #{ type: "string", value: "" },
            },
            help_open: #{
                schema: #{ type: "bool" },
                "default": #{ type: "bool", value: false },
            },
            locale: #{
                schema: #{ type: "string", allowed: ["en", "zh-CN", "ar"] },
                "default": #{ type: "string", value: "__VISUAL_LOCALE__" },
            },
            notifications: #{
                schema: #{ type: "bool" },
                "default": #{ type: "bool", value: true },
            },
            density: #{
                schema: #{ type: "string", allowed: ["comfortable", "compact"] },
                "default": #{ type: "string", value: "comfortable" },
            },
            expanded: #{
                schema: #{ type: "array", max_items: 8, items: #{ type: "string" } },
                "default": #{ type: "array", value: [] },
            },
        },
    }
}

fn choose_theme(ctx, values) {
    ctx.set_state("theme", values);
    ctx.set_state("theme_open", false);
    if values.len == 0 { return; }
    let choice = values[0];
    if choice == "default-light" { ctx.set_theme("Default", "Light"); }
    else if choice == "default-dark" { ctx.set_theme("Default", "Dark"); }
    else if choice == "tokyo-night" { ctx.set_theme("Tokyo Night", "Night"); }
    else if choice == "tokyo-storm" { ctx.set_theme("Tokyo Night", "Storm"); }
    else if choice == "catppuccin-latte" { ctx.set_theme("Catppuccin", "Latte"); }
    else if choice == "catppuccin-mocha" { ctx.set_theme("Catppuccin", "Mocha"); }
    else if choice == "ethereal" { ctx.set_theme("Ethereal", "Dark"); }
    else if choice == "everforest" { ctx.set_theme("Everforest", "Dark"); }
    else if choice == "gruvbox" { ctx.set_theme("Gruvbox", "Dark"); }
    else if choice == "hackerman" { ctx.set_theme("Hackerman", "Dark"); }
    else if choice == "nord" { ctx.set_theme("Nord", "Dark"); }
    else if choice == "retro-82" { ctx.set_theme("Retro 82", "Dark"); }
    else if choice == "hermarchy" { ctx.set_theme("Hermarchy", "Dark"); }
    else if choice == "futurism" { ctx.set_theme("Futurism", "Dark"); }
    else if choice == "aetheria" { ctx.set_theme("Aetheria", "Dark"); }
}

fn set_theme_open(ctx, open) { ctx.set_state("theme_open", open); }
fn set_theme_query(ctx, query) { ctx.set_state("theme_query", query); }
fn set_help_open(ctx, open) { ctx.set_state("help_open", open); }
fn open_help(ctx, payload) { ctx.set_state("help_open", true); }
fn reset_theme(ctx, payload) { choose_theme(ctx, ["default-dark"]); }
fn set_language(ctx, values) {
    if values.len == 0 { return; }
    ctx.set_locale(values[0]);
    ctx.set_state("locale", values[0]);
}
fn set_notifications(ctx, checked) { ctx.set_state("notifications", checked); }
fn set_density(ctx, value) { ctx.set_state("density", value); }
fn set_expanded(ctx, values) { ctx.set_state("expanded", values); }

fn init(ctx) {
    choose_theme(ctx, ["__VISUAL_THEME__"]);
    ctx.set_locale("__VISUAL_LOCALE__");
}

fn theme_options() {
    [
        #{ value: "default-light", label: "Default Light", keywords: ["light"] },
        #{ value: "default-dark", label: "Default Dark", keywords: ["dark"] },
        #{ value: "tokyo-night", label: "Tokyo Night", keywords: ["dark", "blue"] },
        #{ value: "tokyo-storm", label: "Tokyo Storm", keywords: ["dark", "blue"] },
        #{ value: "catppuccin-latte", label: "Catppuccin Latte", keywords: ["light"] },
        #{ value: "catppuccin-mocha", label: "Catppuccin Mocha", keywords: ["dark"] },
        #{ value: "ethereal", label: "Ethereal", keywords: ["dark", "blue"] },
        #{ value: "everforest", label: "Everforest", keywords: ["dark", "green"] },
        #{ value: "gruvbox", label: "Gruvbox", keywords: ["dark", "warm"] },
        #{ value: "hackerman", label: "Hackerman", keywords: ["dark", "green"] },
        #{ value: "nord", label: "Nord", keywords: ["dark", "blue"] },
        #{ value: "retro-82", label: "Retro 82", keywords: ["dark", "retro"] },
        #{ value: "hermarchy", label: "Hermarchy", keywords: ["dark", "cyan"] },
        #{ value: "futurism", label: "Futurism", keywords: ["dark", "magenta"] },
        #{ value: "aetheria", label: "Aetheria", keywords: ["dark", "teal"] }
    ]
}

fn help(ctx) {
    popover::Popover(#{
        key: "settings-help", label: "Settings help", placement: "bottom", align: "end",
        trigger: icon_button::IconButton(#{
            key: "help-trigger", label: "Help", variant: "ghost",
            icon: icon::Icon(#{ source: asset("app/icons/help") }), on_click: Fn("open_help")
        }),
        content: text("Theme changes keep component state and the compiled Rhai AST."),
        open: ctx.get_state("help_open"), on_open_change: Fn("set_help_open")
    })
}

fn appearance(ctx) {
    #{ title: "Appearance", fields: [
        #{ label: "Color theme", description: "Search, or use the arrow keys, Enter and Escape.",
            control: combobox_component::Combobox(#{
                key: "theme-picker", label: "Color theme", options: theme_options(),
                mode: "single", selected: ctx.get_state("theme"), open: ctx.get_state("theme_open"),
                searchable: true, query: ctx.get_state("theme_query"), width: px(240),
                placeholder: "Choose a theme", search_placeholder: "Search themes",
                empty_text: ctx.t("common.no_results"), max_visible: 6,
                on_change: Fn("choose_theme"), on_open_change: Fn("set_theme_open"),
                on_query_change: Fn("set_theme_query")
            }) },
        #{ label: "Language / 语言", control: toggle_group::ToggleGroup(#{
            key: "language", label: "Language", values: [ctx.get_state("locale")],
            items: [#{ value: "en", label: "English" }, #{ value: "zh-CN", label: "简体中文" }],
            allow_empty: false, on_change: Fn("set_language")
        }) },
        #{ label: "Density", control: radio_group::RadioGroup(#{
            value: ctx.get_state("density"), label: "Interface density", orientation: "horizontal",
            options: [#{ value: "comfortable", label: "Comfortable" }, #{ value: "compact", label: "Compact" }],
            on_change: Fn("set_density")
        }) },
    ] }
}

fn notifications(ctx) {
    #{ title: "Notifications", fields: [
        #{ label: "Deploys", control: switch_component::Switch(#{
            checked: ctx.get_state("notifications"), label: "Notify when a deploy finishes",
            on_change: Fn("set_notifications")
        }) },
    ] }
}

fn advanced(ctx) {
    accordion::Accordion(#{
        key: "advanced-settings", expanded: ctx.get_state("expanded"), mode: "multiple",
        items: [
            #{ key: "behavior", title: "Behavior", content_height: 44,
                content: text("Notifications and density are keyed Rhai state.") },
            #{ key: "appearance", title: "Appearance", content_height: 44,
                content: text("Theme changes do not recompile scripts.") }
        ],
        on_change: Fn("set_expanded")
    })
}

fn view(ctx) {
    let form = form_layout::FormLayout(#{
        key: "settings-form", label: "Preferences", groups: [appearance(ctx), notifications(ctx)],
        submit: [button::Button(#{ key: "reset", text: "Reset theme", on_click: Fn("reset_theme") })]
    });
    let body = column([stack::Stack(#{ gap: "section", children: [form, advanced(ctx)] })])
        .with_style(style().flex_col().flex_grow().min_height(px(0)).overflow_y_scroll());
    // The region fills the window, so its footer sits at the bottom.
    column([region::Region(#{
        label: "Settings", title: "Settings", actions: [help(ctx)], body: body,
        footer: [text("Changes apply immediately, without recompiling.")]
    })]).with_style(style().flex_col().width(relative(1.0)).height(relative(1.0)))
        .env(#{ density: ctx.get_state("density") })
}
```

## Form with validation, dialog and toast

A FormLayout of Input, Textarea, Select, DatePicker, Checkbox, RadioGroup and Switch fields, validated on submit, with a Dialog and a Toast. From [`form_showcase.rs`](https://github.com/eddix/gpui-rhai/blob/main/crates/gpui-rhai/examples/form_showcase.rs).

```rhai
import "layouts/region" as region;
import "layouts/stack" as stack;
import "patterns/form_layout" as form_layout;
import "patterns/description_list" as description_list;
import "components/input" as input;
import "components/textarea" as textarea;
import "components/select" as select;
import "components/date_picker" as date_picker;
import "components/checkbox" as checkbox;
import "components/radio_group" as radio_group;
import "components/switch" as switch_component;
import "components/toggle_group" as toggle_group;
import "components/button" as button;
import "components/dialog" as dialog;
import "components/toast" as toast;

fn state_schema() {
    #{ fields: #{
        name: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "__VISUAL_NAME__" } },
        accepted: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_ACCEPTED__ } },
        plan: #{ schema: #{ type: "string", allowed: ["personal", "team"] },
            "default": #{ type: "string", value: "personal" } },
        updates: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } },
        country: #{ schema: #{ type: "optional", value: #{ type: "string" } },
            "default": __VISUAL_COUNTRY__ },
        country_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        country_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        appointment: #{ schema: #{ type: "optional", value: #{ type: "string" } },
            "default": __VISUAL_APPOINTMENT__ },
        notes: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "__VISUAL_NOTES__" } },
        fixed_note: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "Fixed\nrows" } },
        limit_note: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "0123456789" } },
        locale: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "__VISUAL_LOCALE__" } },
        dialog_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_DIALOG__ } },
        toast_visible: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_TOAST__ } },
    } }
}

fn set_name(ctx, value) { ctx.set_state("name", value); }
fn set_accepted(ctx, value) { ctx.set_state("accepted", value.checked); }
fn set_plan(ctx, value) { ctx.set_state("plan", value); }
fn set_updates(ctx, value) { ctx.set_state("updates", value); }
fn set_country(ctx, value) { ctx.set_state("country", value); }
fn set_country_open(ctx, value) { ctx.set_state("country_open", value); }
fn set_country_query(ctx, value) { ctx.set_state("country_query", value); }
fn set_appointment(ctx, value) { ctx.set_state("appointment", value); }
fn set_notes(ctx, value) { ctx.set_state("notes", value); }
fn set_limit_note(ctx, value) { ctx.set_state("limit_note", value); }
fn set_language(ctx, values) {
    if values.len == 0 { return; }
    ctx.set_locale(values[0]);
    ctx.set_state("locale", values[0]);
}
fn set_dialog(ctx, open) { ctx.set_state("dialog_open", open); }
fn close_dialog(ctx, payload) { ctx.set_state("dialog_open", false); }
fn open_dialog(ctx, payload) {
    if ctx.get_state("name") != "" { ctx.set_state("dialog_open", true); }
}
fn confirm(ctx, payload) {
    ctx.set_state("dialog_open", false);
    ctx.set_state("toast_visible", true);
}
fn dismiss_toast(ctx, id) { ctx.set_state("toast_visible", false); }

fn init(ctx) {
    let theme = "__VISUAL_THEME__";
    if theme == "default-light" { ctx.set_theme("Default", "Light"); }
    else if theme == "default-dark" { ctx.set_theme("Default", "Dark"); }
    else if theme == "tokyo-night" { ctx.set_theme("Tokyo Night", "Night"); }
    else if theme == "tokyo-storm" { ctx.set_theme("Tokyo Night", "Storm"); }
    else if theme == "catppuccin-latte" { ctx.set_theme("Catppuccin", "Latte"); }
    else if theme == "catppuccin-mocha" { ctx.set_theme("Catppuccin", "Mocha"); }
    ctx.set_locale("__VISUAL_LOCALE__");
}

fn heading() {
    column([
        text("Profile").with_style(style().typography("title").text_color(theme_color("text_primary")))
            .accessibility_role("heading").accessibility_level(1),
        text("Fields validate as you type; Review confirms the profile.").with_style(style()
            .typography("caption").text_color(theme_color("text_muted"))),
    ]).with_style(style().flex_col().gap(theme_spacing("xxs")).min_width(px(0)))
}

fn language(ctx) {
    toggle_group::ToggleGroup(#{
        key: "language", label: "Language", size: "sm", values: [ctx.get_state("locale")],
        items: [#{ value: "en", label: "English" }, #{ value: "zh-CN", label: "简体中文" }],
        allow_empty: false, on_change: Fn("set_language")
    })
}

fn account(ctx, name, error) {
    #{ title: "Account", fields: [
        #{ label: "Name", required: true, description: "Shown to collaborators.", error: error,
            control: input::Input(#{
                key: "name", label: "Name", value: name, placeholder: "Ada Lovelace",
                error: error != (), on_change: Fn("set_name")
            }).with_style(style().width(px(280))) },
        #{ label: "Plan", control: radio_group::RadioGroup(#{
            value: ctx.get_state("plan"), label: "Plan", orientation: "horizontal",
            options: [#{ value: "personal", label: "Personal" }, #{ value: "team", label: "Team" }],
            on_change: Fn("set_plan")
        }) },
        #{ label: "Terms", control: checkbox::Checkbox(#{
            checked: ctx.get_state("accepted"), label: "I accept the project terms",
            on_change: Fn("set_accepted")
        }) },
        #{ label: "Updates", control: switch_component::Switch(#{
            checked: ctx.get_state("updates"), label: "Product updates by email",
            on_change: Fn("set_updates")
        }) },
    ] }
}

fn details(ctx) {
    #{ title: "Details", fields: [
        #{ label: "Country or region", control: select::Select(#{
            key: "country", label: "Country or region", value: ctx.get_state("country"),
            open: ctx.get_state("country_open"), query: ctx.get_state("country_query"),
            searchable: true, clearable: true, placeholder: "Choose a country", width: px(280),
            options: [
                #{ value: "cn", label: "China", group: "Asia", keywords: ["zhongguo"] },
                #{ value: "jp", label: "Japan", group: "Asia" },
                #{ value: "fr", label: "France", group: "Europe" },
                #{ value: "de", label: "Germany", group: "Europe" }
            ], on_change: Fn("set_country"), on_open_change: Fn("set_country_open"),
            on_query_change: Fn("set_country_query")
        }) },
        #{ label: "Appointment", control: date_picker::DatePicker(#{
            key: "appointment", label: "Appointment date", value: ctx.get_state("appointment"),
            min_date: "2026-08-30", max_date: "2026-12-31", clearable: true,
            placeholder: "Choose a date", width: px(280),
            presets: [#{ label: "Launch", value: "2026-09-01" }, #{ label: "Review", value: "2026-10-15" }],
            on_change: Fn("set_appointment")
        }) },
        #{ label: "Notes", error: if ctx.get_state("notes") == "" { "Add a note for the reviewer." } else { () },
            control: textarea::Textarea(#{
                key: "notes", label: "Notes", value: ctx.get_state("notes"),
                placeholder: "Add feedback or context", min_rows: 3, max_rows: 6,
                max_length: 240, show_count: true,
                error: ctx.get_state("notes") == "", on_change: Fn("set_notes")
            }).with_style(style().width(px(360))) },
        #{ label: "Fixed note", control: textarea::Textarea(#{
            key: "fixed-note", label: "Fixed note", value: ctx.get_state("fixed_note"),
            rows: 2, read_only: true
        }).with_style(style().width(px(360))) },
        #{ label: "Limited note", control: textarea::Textarea(#{
            key: "limit-note", label: "Limited note", value: ctx.get_state("limit_note"),
            min_rows: 1, max_rows: 2, max_length: 10, show_count: true,
            on_change: Fn("set_limit_note")
        }).with_style(style().width(px(360))) },
    ] }
}

fn review(ctx, name) {
    dialog::Dialog(#{
        key: "review", open: ctx.get_state("dialog_open"), title: "Review profile",
        content: description_list::DescriptionList(#{ label: "Profile", items: [
            #{ label: "Name", value: name }, #{ label: "Plan", value: ctx.get_state("plan") },
        ] }),
        actions: [
            button::Button(#{ key: "review-cancel", text: "Cancel", variant: "ghost", on_click: Fn("close_dialog") }),
            button::Button(#{ key: "review-confirm", text: "Confirm", variant: "primary", on_click: Fn("confirm") })
        ],
        on_open_change: Fn("set_dialog")
    })
}

fn view(ctx) {
    let name = ctx.get_state("name");
    let error = if name == "" { "Enter a name." } else { () };
    let toasts = [];
    if ctx.get_state("toast_visible") {
        toasts.push(#{
            id: "saved", title: "Profile saved",
            message: `Saved ${name} on the ${ctx.get_state("plan")} plan.`,
            variant: "success", region: "bottom_right", duration_ms: __TOAST_DURATION__,
        });
    }
    let form = form_layout::FormLayout(#{
        key: "profile-form", label: "Profile", groups: [account(ctx, name, error), details(ctx)],
        submit: [button::Button(#{ key: "review", text: "Review", variant: "primary", on_click: Fn("open_dialog") })]
    });
    let body = column([form, review(ctx, name),
        toast::Toast(#{ key: "form-toasts", items: toasts, on_dismiss: Fn("dismiss_toast") })])
        .with_style(style().flex_col().flex_grow().min_height(px(0)).overflow_y_scroll());
    column([region::Region(#{ label: "Profile", title: heading(), actions: [language(ctx)], body: body })])
        .with_style(style().flex_col().width(relative(1.0)).height(relative(1.0)))
}
```

## Data view with a table

A DataView around a Table with a filter, sorting, selection, paging, and loading and empty states. From [`data_table.rs`](https://github.com/eddix/gpui-rhai/blob/main/crates/gpui-rhai/examples/data_table.rs).

```rhai
import "patterns/data_view" as data_view;
import "components/table" as table;
import "components/pagination" as pagination;
import "components/toggle_group" as toggle_group;
import "components/toggle" as toggle;
import "components/input" as input;

fn state_schema() {
    #{ fields: #{
        query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        current_page: #{ schema: #{ type: "integer", min: 1 }, "default": #{ type: "integer", value: 1 } },
        page_size: #{ schema: #{ type: "integer", min: 1 }, "default": #{ type: "integer", value: 25 } },
        sort: #{ schema: #{ type: "optional", value: #{ type: "map", values: #{ type: "ui_value" } } }, "default": #{ type: "null" } },
        selected: #{ schema: #{ type: "array", max_items: 500, items: #{ type: "string" } }, "default": #{ type: "array", value: __VISUAL_SELECTED__ } },
        loading: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: __VISUAL_LOADING__ } },
        selection_mode: #{ schema: #{ type: "string", allowed: ["single", "multiple"] },
            "default": #{ type: "string", value: "multiple" } },
        group_by: #{ schema: #{ type: "optional", value: #{ type: "string" } },
            "default": __VISUAL_GROUP_BY__ },
        collapsed_groups: #{ schema: #{ type: "array", max_items: 16,
            items: #{ type: "string" } }, "default": #{ type: "array", value: [] } },
    } }
}

fn generated_row(index) {
    let display_index = if index < 10 { `00${index}` }
        else if index < 100 { `0${index}` } else { `${index}` };
    let status = if index % 3 == 0 { "Active" } else if index % 3 == 1 { "Away" } else { "Offline" };
    #{
        id: `user-${index}`,
        name: `User ${display_index}`,
        email: `user-${display_index}@example.com`,
        score: ((index * 37) % 1000) / 10.0,
        joined: if index % 2 == 0 { "2026-08-30" } else { "2026-08-31" },
        status: status,
        // Normal is quiet: only "Away" deviates enough to take a color.
        tone: if status == "Away" { "warning" } else { "neutral" },
    }
}

fn matching_rows(ctx) {
    let query = ctx.get_state("query").to_lower();
    let rows = [];
    for index in 0..__ROW_COUNT__ {
        let row = generated_row(index);
        if query == "" || row.name.to_lower().contains(query) || row.email.contains(query) {
            rows.push(row);
        }
    }
    let sort = ctx.get_state("sort");
    if sort != () && sort.key == "name" && sort.direction == "descending" { rows.reverse(); }
    rows
}

fn page_rows(ctx, rows) {
    let start = (ctx.get_state("current_page") - 1) * ctx.get_state("page_size");
    let end = if start + ctx.get_state("page_size") > rows.len {
        rows.len
    } else { start + ctx.get_state("page_size") };
    if start < end { rows.extract(start, end - start) } else { [] }
}

fn set_query(ctx, value) {
    ctx.set_state("query", value);
    ctx.set_state("current_page", 1);
    ctx.set_state("selected", []);
}

fn set_sort(ctx, value) {
    ctx.set_state("sort", value);
    ctx.set_state("current_page", 1);
    ctx.set_state("selected", []);
}
fn set_selection(ctx, value) { ctx.set_state("selected", value); }
fn row_clicked(ctx, key) { () }
fn set_selection_mode(ctx, values) {
    if values.len == 0 { return; }
    ctx.set_state("selection_mode", values[0]);
    ctx.set_state("selected", []);
}
fn set_pagination(ctx, value) {
    ctx.set_state("current_page", value.current_page);
    ctx.set_state("page_size", value.page_size);
    ctx.set_state("selected", []);
}
fn set_loading(ctx, pressed) { ctx.set_state("loading", pressed); }
fn set_grouping(ctx, pressed) {
    ctx.set_state("group_by", if pressed { "status" } else { () });
    ctx.set_state("collapsed_groups", []);
}
fn toggle_group_rows(ctx, group) {
    let collapsed = ctx.get_state("collapsed_groups");
    let next = [];
    let found = false;
    for value in collapsed {
        if value == group { found = true; }
        else { next.push(value); }
    }
    if !found { next.push(group); }
    ctx.set_state("collapsed_groups", next);
}

fn init(ctx) {
    let theme = "__VISUAL_THEME__";
    if theme == "default-dark" { ctx.set_theme("Default", "Dark"); }
    else if theme == "tokyo-night" { ctx.set_theme("Tokyo Night", "Night"); }
    else if theme == "catppuccin-mocha" { ctx.set_theme("Catppuccin", "Mocha"); }
    else { ctx.set_theme("Default", "Light"); }
    ctx.set_locale("__VISUAL_LOCALE__");
}

fn table_columns() {
    [
        #{ key: "name", title: "Name", width: #{ kind: "fixed", value: 180 }, sortable: true },
        #{ key: "email", title: "Email", width: #{ kind: "flex", value: 1 } },
        #{ key: "score", title: "Score", width: #{ kind: "fixed", value: 96 }, numeric: true },
        #{ key: "joined", title: "Joined", width: #{ kind: "fixed", value: 128 }, numeric: true },
        #{ key: "status", title: "Status", width: #{ kind: "fixed", value: 120 },
            adornments: [#{ text_key: "status", variant_key: "tone", dot: true }] },
    ]
}

fn actions(ctx) {
    let actions = [
        toggle_group::ToggleGroup(#{
            key: "selection-mode", label: "Selection", allow_empty: false,
            values: [ctx.get_state("selection_mode")],
            items: [#{ value: "single", label: "Single" }, #{ value: "multiple", label: "Multiple" }],
            on_change: Fn("set_selection_mode")
        }),
        toggle::Toggle(#{ key: "loading", text: "Loading", pressed: ctx.get_state("loading"),
            on_pressed_change: Fn("set_loading") }),
    ];
    if __GROUP_CONTROL__ {
        actions.push(toggle::Toggle(#{ key: "grouping", text: "Group by status",
            pressed: ctx.get_state("group_by") != (), on_pressed_change: Fn("set_grouping") }));
    }
    actions
}

fn filter_field(ctx) {
    input::Input(#{ key: "users-filter", label: "Filter users", value: ctx.get_state("query"),
        placeholder: "Filter by name or email", on_change: Fn("set_query") })
        .with_style(style().width(px(240)))
}

fn view(ctx) {
    let selected = ctx.get_state("selected");
    let rows = matching_rows(ctx);
    let grid = table::Table(#{
        key: "users", label: "Users", rows: page_rows(ctx, rows), row_key: "id",
        columns: table_columns(), fill_height: true, resizable_columns: true,
        empty_text: "No users match the filter",
        selection_mode: ctx.get_state("selection_mode"),
        selected_keys: selected, sort: ctx.get_state("sort"),
        group_by: ctx.get_state("group_by"),
        collapsed_groups: ctx.get_state("collapsed_groups"),
        on_sort_change: Fn("set_sort"), on_selection_change: Fn("set_selection"),
        on_row_click: Fn("row_clicked"), on_group_toggle: Fn("toggle_group_rows")
    });
    let pager = pagination::Pagination(#{
        key: "users-pages", label: "Users pagination", total_items: rows.len,
        current_page: ctx.get_state("current_page"), page_size: ctx.get_state("page_size"),
        page_size_options: [10, 25, 50, 100], show_summary: false, on_change: Fn("set_pagination")
    });
    // Loading and empty are shown in place of the table by DataView.
    let state = if ctx.get_state("loading") {
        #{ state: "loading", title: "Loading users" }
    } else if __ROW_COUNT__ == 0 {
        #{ state: "empty", title: "No users yet", description: "Invite people to see them here." }
    } else { () };
    column([data_view::DataView(#{
        key: "users-view", label: "Users", title: "Users", body: grid, state: state,
        toolbar: #{ filters: [filter_field(ctx)], actions: actions(ctx) },
        footer: #{ status: pager, selection: `${selected.len} selected`, count: `${rows.len} users` }
    })]).with_style(style().flex_col().width(relative(1.0)).height(relative(1.0)))
}
```

## Dashboard

Sections of Stat figures, DescriptionLists, Tabs, Tags and Progress. From [`dashboard_layout.rs`](https://github.com/eddix/gpui-rhai/blob/main/crates/gpui-rhai/examples/dashboard_layout.rs).

```rhai
import "layouts/region" as region;
import "layouts/stack" as stack;
import "layouts/inline" as inline;
import "patterns/section" as section;
import "patterns/stat" as stat;
import "patterns/description_list" as description_list;
import "components/tabs" as tabs;
import "components/tag" as tag;
import "components/avatar" as avatar;
import "components/progress" as progress;
import "components/popover" as popover;

fn state_schema() {
    #{ fields: #{
        tab: #{ schema: #{ type: "string", allowed: ["overview", "activity"] },
            "default": #{ type: "string", value: "overview" } },
        profile_open: #{ schema: #{ type: "bool" },
            "default": #{ type: "bool", value: false } },
    } }
}

fn set_tab(ctx, value) { ctx.set_state("tab", value); }
fn set_profile_open(ctx, open) { ctx.set_state("profile_open", open); }
fn ignore(ctx, value) { () }

fn init(ctx) {
    let theme = "__VISUAL_THEME__";
    if theme == "default-light" { ctx.set_theme("Default", "Light"); }
    else if theme == "default-dark" { ctx.set_theme("Default", "Dark"); }
    else if theme == "tokyo-night" { ctx.set_theme("Tokyo Night", "Night"); }
    else if theme == "tokyo-storm" { ctx.set_theme("Tokyo Night", "Storm"); }
    else if theme == "catppuccin-latte" { ctx.set_theme("Catppuccin", "Latte"); }
    else if theme == "catppuccin-mocha" { ctx.set_theme("Catppuccin", "Mocha"); }
    ctx.set_locale("__VISUAL_LOCALE__");
}

fn heading() {
    column([
        text("Runtime dashboard").with_style(style().typography("title").text_color(theme_color("text_primary")))
            .accessibility_role("heading").accessibility_level(1),
        text("Source-owned Rhai components").with_style(style().typography("caption")
            .text_color(theme_color("text_muted"))),
    ]).with_style(style().flex_col().gap(theme_spacing("xxs")).min_width(px(0)))
}

fn profile(ctx) {
    popover::Popover(#{
        key: "profile", label: "Profile", placement: "bottom", align: "end",
        trigger: avatar::Avatar(#{ name: "Ada Lovelace", initials: "AL", presence: "online" }),
        content: description_list::DescriptionList(#{ label: "Profile", items: [
            #{ label: "Name", value: "Ada Lovelace" }, #{ label: "Role", value: "Runtime maintainer" },
        ] }),
        open: ctx.get_state("profile_open"), on_open_change: Fn("set_profile_open")
    })
}

fn overview() {
    stack::Stack(#{ gap: "section", children: [
        inline::Inline(#{ children: [
            tag::Tag(#{ facet: "env", text: "production" }),
            tag::Tag(#{ facet: "lang", text: "rust", closable: true, on_close: Fn("ignore") })
        ] }),
        stat::stats([
            #{ label: "Hosts", value: "128" },
            #{ label: "Healthy", value: "127" },
            #{ label: "Deploy", value: "72", unit: "%" },
        ]),
        section::Section(#{ title: "Deployment", description: "Rolling out 1.4.2 to eu-west.",
            content: progress::Progress(#{ key: "deployment", value: 72, max: 100, label: "Deployment" }) }),
        section::Section(#{ title: "Synchronization", description: "Background sync of the host inventory.",
            content: progress::Progress(#{ key: "sync", indeterminate: true, label: "Synchronization" }) })
    ] })
}

fn activity() {
    description_list::DescriptionList(#{ label: "Activity", items: [
        #{ label: "09:42", value: "Release candidate built", numeric: true },
        #{ label: "09:44", value: "Integration checks passed", numeric: true },
        #{ label: "09:47", value: "Deployment started", numeric: true },
    ] })
}

fn view(ctx) {
    let selected = ctx.get_state("tab");
    let switcher = tabs::Tabs(#{
        value: selected, label: "Dashboard sections", panel: false,
        tabs: [#{ value: "overview", label: "Overview" }, #{ value: "activity", label: "Activity" }],
        on_change: Fn("set_tab")
    });
    // The region fills the window, so its footer sits at the bottom.
    column([region::Region(#{
        label: "Runtime dashboard", title: heading(), actions: [profile(ctx)], toolbar: switcher,
        body: if selected == "activity" { activity() } else { overview() },
        footer: [text("Updated 09:47")]
    })]).with_style(style().flex_col().width(relative(1.0)).height(relative(1.0)))
}
```
