// Gallery spec pages: list-like components.

import "gallery/kit" as kit;
import "components/menu" as menu;
import "components/context_menu" as context_menu;
import "components/command" as command;
import "components/command_dialog" as command_dialog;
import "components/tree" as tree;
import "components/list" as list;
import "components/table" as table;
import "components/accordion" as accordion;
import "components/pagination" as pagination;
import "components/tabs" as tabs;
import "components/button" as button;

fn pages() {
    [
        #{ id: "menu", title: "Menu", group: "Lists" },
        #{ id: "context_menu", title: "ContextMenu", group: "Lists" },
        #{ id: "command", title: "Command", group: "Lists" },
        #{ id: "command_dialog", title: "CommandDialog", group: "Lists" },
        #{ id: "tree", title: "Tree", group: "Lists" },
        #{ id: "list", title: "List", group: "Lists" },
        #{ id: "table", title: "Table", group: "Lists" },
        #{ id: "accordion", title: "Accordion", group: "Lists" },
        #{ id: "pagination", title: "Pagination", group: "Lists" },
        #{ id: "tabs", title: "Tabs", group: "Lists" },
    ]
}

fn view(ctx, h, v, id) {
    if id == "menu" { menu_page(h, v) }
    else if id == "context_menu" { context_menu_page(h, v) }
    else if id == "command" { command_page(h, v) }
    else if id == "command_dialog" { command_dialog_page(h, v) }
    else if id == "tree" { tree_page(h, v) }
    else if id == "list" { list_page(h, v) }
    else if id == "table" { table_page(h, v) }
    else if id == "accordion" { accordion_page(h, v) }
    else if id == "pagination" { pagination_page(h, v) }
    else if id == "tabs" { tabs_page(h, v) }
    else { () }
}

fn file_items() {
    [
        #{ kind: "label", label: "Document" },
        #{ kind: "item", value: "new", label: "New file", shortcut: "⌘N" },
        #{ kind: "item", value: "open", label: "Open…", shortcut: "⌘O" },
        #{ kind: "item", value: "wrap", label: "Word wrap", checked: true },
        #{ kind: "separator" },
        #{ kind: "item", value: "close", label: "关闭窗口", shortcut: "⌘W", disabled: true },
    ]
}

fn menu_page(h, v) {
    let trigger = button::Button(#{ key: "menu-trigger", text: "File", variant: "secondary", on_click: h.noop });
    kit::page(h, v, "menu", "Menu", "Transient list: the cursor is a selection block with the indicator bar. Enter runs.", [
        kit::sec("Open", column([menu::Menu(#{ key: "menu-file", label: "File", trigger: trigger,
            open: kit::val(v, "menu.open", false), active_value: kit::val(v, "menu.active", "open"), items: file_items(),
            on_open_change: kit::store(h, "menu.open"), on_active_change: kit::store(h, "menu.active"),
            on_action: h.noop })])),
        kit::sec("Usage", kit::note("Press the trigger, then use the arrows; items can bind registered actions.")),
    ])
}

fn context_menu_page(h, v) {
    let target = row([text("Right-click this row")]).with_style(style().min_height(theme_length("metrics.row"))
        .padding_x(theme_length("metrics.inset")).items_center().background(theme_color("surface_raised")));
    kit::page(h, v, "context_menu", "ContextMenu", "A menu anchored at the pointer.", [
        kit::sec("Target", context_menu::ContextMenu(#{ key: "context-menu", label: "Row actions", trigger: target,
            open: kit::val(v, "context.open", false), active_value: kit::val(v, "context.active", "new"),
            items: file_items(), on_open_change: kit::store(h, "context.open"),
            on_active_change: kit::store(h, "context.active"), on_action: h.noop })),
    ])
}

fn command_items() {
    [
        #{ value: "deploy", label: "Deploy service", group: "Release", shortcut: "⌘D" },
        #{ value: "rollback", label: "Roll back", group: "Release" },
        #{ value: "logs", label: "Open logs", group: "Observe", shortcut: "⌘L" },
        #{ value: "metrics", label: "查看指标", group: "Observe", keywords: ["metrics"] },
    ]
}

fn command_page(h, v) {
    kit::page(h, v, "command", "Command", "Fuzzy search over commands with group labels and key legends.", [
        kit::sec("Embedded", command::Command(#{ key: "command-embedded", label: "Commands",
            query: kit::val(v, "command.query", ""), active_value: kit::val(v, "command.active", "deploy"),
            items: command_items(), max_visible: 6, on_query_change: kit::store(h, "command.query"),
            on_active_change: kit::store(h, "command.active"), on_action: h.noop })
            .with_style(style().width(px(360)))),
    ])
}

fn command_dialog_page(h, v) {
    let opener = button::Button(#{ key: "command-dialog-open", text: "Open palette", on_click: kit::put(h, "command_dialog.open", true) });
    kit::page(h, v, "command_dialog", "CommandDialog", "The command palette in a modal dialog; Esc closes and restores focus.", [
        kit::sec("Dialog", kit::pile([opener, command_dialog::CommandDialog(#{ key: "command-dialog", label: "Commands",
            open: kit::val(v, "command_dialog.open", false), query: kit::val(v, "command_dialog.query", ""),
            active_value: kit::val(v, "command_dialog.active", "deploy"), items: command_items(),
            on_open_change: kit::store(h, "command_dialog.open"), on_query_change: kit::store(h, "command_dialog.query"),
            on_active_change: kit::store(h, "command_dialog.active"), on_action: h.noop })])),
    ])
}

fn tree_items() {
    [
        #{ key: "workspace", label: "workspace" }, #{ key: "src", parent: "workspace", label: "src" },
        #{ key: "lib", parent: "src", label: "lib.rs" }, #{ key: "app", parent: "src", label: "app.rs" },
        #{ key: "ui", parent: "workspace", label: "ui" }, #{ key: "theme", parent: "ui", label: "theme.rhai" },
        #{ key: "readme", label: "README.md" },
    ]
}

fn tree_page(h, v) {
    kit::page(h, v, "tree", "Tree", "Persistent list: selection carries the indicator bar; the focused cursor shows the ink frame.", [
        kit::sec("Outline", tree::Tree(#{ key: "tree-files", label: "Files", items: tree_items(), height: 224.0,
            expanded: kit::val(v, "tree.expanded", ["workspace", "src"]), selected_keys: kit::val(v, "tree.selected", ["app"]),
            active_key: kit::val(v, "tree.active", "app"), on_expanded_change: kit::store(h, "tree.expanded"),
            on_selection_change: kit::store(h, "tree.selected"), on_active_change: kit::store(h, "tree.active") })
            .with_style(style().width(px(280)))),
    ])
}

fn ticket_items() {
    [
        #{ key: "t1", title: "Deploy api-gateway", secondary: "#4812 · ada", meta: "09:41",
            badge: #{ text: "Running", variant: "accent" } },
        #{ key: "t2", title: "Roll back worker-07", secondary: "#4809 · grace", meta: "09:12",
            badge: #{ text: "Failed", variant: "danger" } },
        #{ key: "t3", title: "Rotate certificates", secondary: "#4801 · linus", meta: "昨天",
            badge: #{ text: "Done", variant: "success" } },
        #{ key: "t4", title: "Archived ticket", secondary: "#4700", meta: "Mon", disabled: true },
    ]
}

fn list_page(h, v) {
    kit::page(h, v, "list", "List", "Keyed rows on the row inset: selection carries the indicator bar; the focused list shows the ink frame on the selected row.", [
        kit::sec("Single selection", list::List(#{ key: "list-tickets", label: "Tickets", items: ticket_items(),
            selection_mode: "single", selected_keys: kit::val(v, "list.selected", ["t2"]), height: 128.0,
            badge_width: 80.0,
            on_selection_change: kit::store(h, "list.selected"), on_row_click: h.noop })
            .with_style(style().width(px(440)))),
        kit::sec("Two lines, dividers", list::List(#{ key: "list-two-line", label: "Tickets, two lines",
            items: ticket_items(), secondary_layout: "below", dividers: true, height: 196.0,
            badge_width: 80.0,
            on_row_click: h.noop }).with_style(style().width(px(440)))),
    ])
}

fn hosts() {
    [
        #{ id: "a", host: "api-01", region: "us-east", cpu: "42.1", mem: "61.0", state: "Running", tone: "neutral" },
        #{ id: "b", host: "api-02", region: "us-east", cpu: "7.0", mem: "12.5", state: "Running", tone: "neutral" },
        #{ id: "c", host: "worker-07", region: "eu-west", cpu: "98.6", mem: "88.2", state: "Degraded", tone: "warning" },
        #{ id: "d", host: "缓存-03", region: "ap-sg", cpu: "13.25", mem: "40.0", state: "Down", tone: "danger" },
    ]
}

fn host_columns() {
    [
        #{ key: "host", title: "Host", width: #{ kind: "flex", value: 2.0 }, sortable: true },
        #{ key: "region", title: "Region", width: #{ kind: "fixed", value: 120.0 } },
        #{ key: "cpu", title: "CPU %", width: #{ kind: "fixed", value: 96.0 }, numeric: true, sortable: true },
        #{ key: "mem", title: "Mem GiB", width: #{ kind: "fixed", value: 96.0 }, numeric: true },
        #{ key: "status", title: "State", width: #{ kind: "fixed", value: 140.0 },
            adornments: [#{ text_key: "state", variant_key: "tone", dot: true }] },
    ]
}

fn pods() {
    [
        #{ id: "p1", pod: "dp-09aa6aec4c-55d96c7899-kbzgl", ip: "10.0.12.40", region: "us-east", state: "Running" },
        #{ id: "p2", pod: "dp-09aa6aec4c-55d96c7899-0lO1I", ip: "10.0.12.41", region: "us-east", state: "Running" },
        #{ id: "p3", pod: "dp-7f3c21b0de-6b8d9f4c2a-xq7pz", ip: "10.8.3.102", region: "eu-west", state: "Pending" },
    ]
}

// The row menu a right press or Shift+F10 asks for, at the request's anchor.
fn host_menu(h, v) {
    let request = kit::val(v, "table.context", ());
    menu::Menu(#{ key: "table-host-menu", label: "Host actions", trigger: row([]),
        open: request != (), anchor: if request != () { request.anchor } else { () },
        activate_on_trigger: false, active_value: kit::val(v, "table.context_active", "open"),
        items: [#{ kind: "item", value: "open", label: "Open" },
            #{ kind: "item", value: "copy", label: "Copy host name" }],
        on_open_change: kit::put(h, "table.context", ()), on_action: kit::put(h, "table.context", ()),
        on_active_change: kit::store(h, "table.context_active") })
}

fn table_page(h, v) {
    kit::page(h, v, "table", "Table", "Label-voice header, numeric columns end-aligned, no zebra. Arrows move the selection.", [
        kit::sec("Hosts", column([table::Table(#{ key: "table-hosts", label: "Hosts", row_key: "id", height: 200.0,
            columns: host_columns(), rows: hosts(), sort: kit::val(v, "table.sort", ()),
            selected_keys: kit::val(v, "table.selected", ["c"]), selection_mode: "single",
            on_selection_change: kit::store(h, "table.selected"), on_sort_change: kit::store(h, "table.sort"),
            on_context_request: kit::store(h, "table.context") }), host_menu(h, v)])),
        kit::sec("Grouped, identifiers in the code role", table::Table(#{ key: "table-pods", label: "Pods",
            row_key: "id", height: 200.0, group_by: "region", rows: pods(),
            columns: [#{ key: "pod", title: "Pod", width: #{ kind: "flex", value: 2.0 }, typography: "code" },
                #{ key: "ip", title: "IP", width: #{ kind: "fixed", value: 140.0 }, typography: "code" },
                #{ key: "region", title: "Region", width: #{ kind: "fixed", value: 120.0 } },
                #{ key: "state", title: "State", width: #{ kind: "fixed", value: 120.0 } }] })),
    ])
}

fn accordion_page(h, v) {
    let items = [
        #{ key: "general", title: "General", content: text("Name, owner and region.") },
        #{ key: "advanced", title: "Advanced", content: text("Scheduling and limits.") },
        #{ key: "danger", title: "危险操作", content: text("Delete the service.") },
    ];
    kit::page(h, v, "accordion", "Accordion", "Sections as a Swiss list: hairlines between, no enclosing box.", [
        kit::sec("Sections", accordion::Accordion(#{ key: "accordion-settings", items: items,
            expanded: kit::val(v, "accordion.expanded", ["general"]), on_change: kit::store(h, "accordion.expanded") })
            .with_style(style().width(px(420)))),
    ])
}

fn page_change(h) { kit::store(h, "pagination.state") }

fn pagination_page(h, v) {
    let current = kit::val(v, "pagination.state", #{ current_page: 3, page_size: 10 });
    kit::page(h, v, "pagination", "Pagination", "Ghost page targets; the current page is a selection block, not a primary action.", [
        kit::sec("Pages", pagination::Pagination(#{ key: "pagination-hosts", label: "Hosts pages", total_items: 128,
            current_page: current.current_page, page_size: current.page_size, on_change: page_change(h) })),
    ])
}

fn tab_list() {
    [
        #{ value: "overview", label: "Overview", content: text("Service overview.") },
        #{ value: "logs", label: "Logs", content: text("Recent log lines.") },
        #{ value: "traces", label: "调用链", content: text("Trace samples.") },
        #{ value: "settings", label: "Settings", content: text("Configuration."), disabled: true },
    ]
}

fn tabs_page(h, v) {
    let value = kit::val(v, "tabs.value", "overview");
    kit::page(h, v, "tabs", "Tabs", "One track, one raised thumb that carries the list's focus.", [
        kit::sec("With panel", tabs::Tabs(#{ label: "Service", value: value, tabs: tab_list(),
            on_change: kit::store(h, "tabs.value") })),
        kit::sec("View switcher", tabs::Tabs(#{ label: "View", value: value, tabs: tab_list(), panel: false, size: "sm",
            on_change: kit::store(h, "tabs.value") })),
    ])
}
