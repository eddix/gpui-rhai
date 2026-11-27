// Gallery spec pages: list-like components, and pagination.

import "gallery/kit" as kit;
import "components/menu" as menu;
import "components/context_menu" as context_menu;
import "components/command" as command;
import "components/command_dialog" as command_dialog;
import "components/tree" as tree;
import "components/list" as list;
import "components/selectable_list" as selectable_list;
import "components/table" as table;
import "components/accordion" as accordion;
import "components/pagination" as pagination;
import "components/button" as button;

fn pages() {
    [
        #{ id: "menu", title: "Menu", group: "Lists" },
        #{ id: "context_menu", title: "ContextMenu", group: "Lists" },
        #{ id: "command", title: "Command", group: "Lists" },
        #{ id: "command_dialog", title: "CommandDialog", group: "Lists" },
        #{ id: "tree", title: "Tree", group: "Lists" },
        #{ id: "list", title: "List", group: "Lists" },
        #{ id: "selectable_list", title: "SelectableList", group: "Lists" },
        #{ id: "table", title: "Table", group: "Lists" },
        #{ id: "accordion", title: "Accordion", group: "Lists" },
        #{ id: "pagination", title: "Pagination", group: "Navigation" },
    ]
}

fn view(ctx, h, v, id) {
    if id == "menu" { menu_page(h, v) }
    else if id == "context_menu" { context_menu_page(h, v) }
    else if id == "command" { command_page(h, v) }
    else if id == "command_dialog" { command_dialog_page(h, v) }
    else if id == "tree" { tree_page(h, v) }
    else if id == "list" { list_page(h, v) }
    else if id == "selectable_list" { selectable_list_page(ctx, h, v) }
    else if id == "table" { table_page(h, v) }
    else if id == "accordion" { accordion_page(h, v) }
    else if id == "pagination" { pagination_page(h, v) }
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
    // The Button handles its own click, which stops there: it opens the menu itself.
    let trigger = button::Button(#{ key: "menu-trigger", text: "File", variant: "secondary",
        on_click: kit::put(h, "menu.open", true) });
    kit::page(h, v, "menu", "Menu", "Transient list: the cursor is a rounded selection block. Enter runs.", [
        kit::sec("Open", kit::pile([menu::Menu(#{ key: "menu-file", label: "File", trigger: trigger,
            open: kit::val(v, "menu.open", false), active_value: kit::val(v, "menu.active", "open"), items: file_items(),
            on_open_change: kit::store(h, "menu.open"), on_active_change: kit::store(h, "menu.active"),
            on_action: h.noop })])),
        kit::sec("Usage", kit::note("Press the trigger, then use the arrows; items can bind registered actions.")),
    ])
}

fn context_menu_page(h, v) {
    let target = row([text("Right-click this row, or focus it and press Shift+F10")]).with_style(style().min_height(theme_length("metrics.row"))
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
        #{ value: "deploy", label: "Deploy service", hint:"Production", group: "Release", shortcut: "cmd-d",
            logo:svg("<svg width='24' height='24' viewBox='0 0 24 24'><path fill='#0190fe' d='M12 2 22 7v10l-10 5L2 17V7z'/><path fill='white' d='M7 8h10v2H7zm0 4h10v2H7zm0 4h6v2H7z'/></svg>") },
        #{ value: "rollback", label: "Roll back", hint:"Previous release", group: "Release", icon:"app/icons/chevron_left" },
        #{ value: "logs", label: "Open logs", hint:"Live stream", group: "Observe", shortcut: "cmd-l", icon:"app/icons/search" },
        #{ value: "metrics", label: "查看指标", group: "Observe", keywords: ["metrics"] },
    ]
}

fn command_page(h, v) {
    kit::page(h, v, "command", "Command", "Fuzzy search with group labels, icons, logo tiles, hints and key legends.", [
        row([
            button::Button(#{key:"command-wide",text:"Wide panel",variant:"secondary",on_click:kit::put(h,"command.panel_width",560)}),
            button::Button(#{key:"command-narrow",text:"Narrow panel",variant:"secondary",on_click:kit::put(h,"command.panel_width",360)}),
        ]).with_style(style().gap(theme_spacing("sm")).flex_wrap()),
        kit::sec("Panel", command::Command(#{ key: "command-embedded", label: "Commands",
            query: kit::val(v, "command.query", ""), active_value: kit::val(v, "command.active", "deploy"),
            items: command_items(), max_visible: 6, on_query_change: kit::store(h, "command.query"),
            on_active_change: kit::store(h, "command.active"), on_action: kit::store(h,"command.last_action") })
            .test_id("command-panel-demo").with_style(style().width(px(kit::val(v,"command.panel_width",560))).max_width(relative(1.0)))),
        text(`Last action: ${kit::val(v,"command.last_action","none")}`).test_id("command-last-action")
            .accessibility_role("status").with_style(style().typography("caption").text_color(theme_color("collection.metadata"))),
        kit::sec("Host-owned frame and footer",column([
            command::Command(#{key:"command-host",label:"Host commands",appearance:"embedded",
                query:kit::val(v,"command.host_query",""),active_value:kit::val(v,"command.host_active","inspect"),
                items:[#{value:"inspect",label:"Inspect current theme"}],max_visible:2,
                on_query_change:kit::store(h,"command.host_query"),on_active_change:kit::store(h,"command.host_active"),
                on_action:kit::store(h,"command.host_action"),
                footer:button::Button(#{key:"command-footer-clear",text:"Clear command feedback",variant:"secondary",
                    on_click:kit::put(h,"command.last_action","none")})
            }),
            text(`Host action: ${kit::val(v,"command.host_action","none")}`).test_id("command-host-action")
                .with_style(style().typography("caption").text_color(theme_color("collection.metadata"))),
        ]).with_style(style().width(px(560)).max_width(relative(1.0)).padding(theme_spacing("md"))
            .gap(theme_spacing("sm")).radius(theme_radius("lg")).border(px(1)).border_color(theme_color("border"))
            .background(theme_color("surface_raised")))),
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
    kit::page(h, v, "tree", "Tree", "Persistent list: rows are rounded blocks and selection fills them; the cursor shows the ink frame after a navigation key.", [
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

fn albums() {
    [
        #{ key: "a1", label: "Kind of Blue, Miles Davis", title: "Kind of Blue", artist: "Miles Davis", year: "1959", tone: "accent" },
        #{ key: "a2", label: "A Love Supreme, John Coltrane", title: "A Love Supreme", artist: "John Coltrane", year: "1965", tone: "success" },
        #{ key: "a3", label: "Blue Train, John Coltrane", title: "Blue Train", artist: "John Coltrane", year: "1958", tone: "warning" },
        #{ key: "a4", label: "Mingus Ah Um, Charles Mingus", title: "Mingus Ah Um", artist: "Charles Mingus", year: "1959", tone: "danger" },
        #{ key: "a5", label: "Time Out, Dave Brubeck", title: "Time Out", artist: "Dave Brubeck Quartet", year: "1959", tone: "accent" },
    ]
}

// Play in a media row: its callback belongs to the Gallery, which bound the row factory.
fn play_album(key, ctx, payload) {
    let values = ctx.get_state("values");
    values["selectable_list.playing"] = key;
    ctx.set_state("values", values);
}

// A media row drawn by the page: a cover, two lines and a control of its own. The list keeps
// selection, the keys and the rows' accessibility; Play calls back into the Gallery.
fn album_row(playing, ctx, row) {
    let album = row.item;
    let tone = if album.tone == "accent" {"info"}else{album.tone};
    let cover = column([text(album.title.sub_string(0, 1)).with_style(style().typography("title")
        .text_color(theme_color(`badge.${tone}_strong_fg`)))])
        .with_style(style().width(px(40)).height(px(40)).flex_shrink(false).items_center().justify_center()
            .radius(theme_radius("md")).background(theme_color(`badge.${tone}_strong_bg`)));
    let lines = column([
        text(album.title).with_style(style().typography("body").whitespace_nowrap().text_ellipsis()),
        text(`${album.artist} · ${album.year}`).with_style(style().typography("caption")
            .text_color(if row.selected { theme_color("text_primary") } else { theme_color("text_muted") })
            .whitespace_nowrap().text_ellipsis()),
    ]).with_style(style().flex_col().flex_grow().min_width(px(0)));
    row([cover, lines, button::Button(#{ key: `album-play-${row.key}`, size: "sm", variant: "ghost",
        text: if playing == row.key { "Playing" } else { "Play" },
        part_styles:#{root:if row.selected {style().text_color(theme_color("text_primary"))}else{style()}},
        on_click: Fn("play_album").curry(row.key) })])
        .with_style(style().items_center().gap(theme_spacing("sm")).flex_grow().min_width(px(0)))
        // The cover's initial is artwork, not text set beside the title.
        .audit_allow(["mixed-type-in-row"])
}

fn people() {
    [
        #{ key: "p1", label: "Ada Lovelace", name: "Ada Lovelace", role: "Analyst" },
        #{ key: "p2", label: "Grace Hopper", name: "Grace Hopper", role: "Compiler" },
        #{ key: "p3", label: "Linus Torvalds", name: "Linus Torvalds", role: "Kernel" },
        #{ key: "p4", label: "Barbara Liskov", name: "Barbara Liskov", role: "Abstraction" },
    ]
}

// A bare row paints its own selected state: an accent bar at the start and a heavier name.
fn person_row(ctx, row) {
    let person = row.item;
    let mark = box([]).with_style(style().width(px(3)).self_stretch()
        .background(if row.selected { theme_color("accent") } else { rgba(0x00000000) }));
    let lines = row([
        text(person.name).with_style(style().typography("body").flex_grow().min_width(px(0))
            .font_weight(if row.selected { 600 } else { 400 }).whitespace_nowrap().text_ellipsis()),
        text(person.role).with_style(style().typography("body").text_color(if row.selected {theme_color("text_primary")}else{theme_color("collection.metadata")})),
    ]).with_style(style().flex_grow().min_width(px(0)).gap(theme_spacing("sm")).items_center()
        .padding_x(theme_length("metrics.inset")));
    row([mark, lines]).with_style(style().flex_grow().min_width(px(0)).height(theme_length("metrics.row"))
        .border_bottom(px(1)).border_color(theme_color("border")))
}

fn list_page(h, v) {
    kit::page(h, v, "list", "List", "Keyed rows on the row inset: selection fills the row; after a navigation key the focused list frames the selected row.", [
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

fn selectable_list_page(ctx, h, v) {
    kit::page(h, v, "selectable_list", "SelectableList", "Rows the page draws with List's selection, keys and focus frame: a click selects and opens, the arrows move, a control in a row is the page's.", [
        kit::sec("Media rows", selectable_list::SelectableList(#{ key: "selectable-albums",
            label: "Albums", items: albums(), selection_mode: "single",
            selected_keys: kit::val(v, "selectable_list.album", ["a2"]), height: 280.0,
            row: ctx.node_factory(Fn("album_row").curry(kit::val(v, "selectable_list.playing", ""))),
            on_selection_change: kit::store(h, "selectable_list.album"), on_row_click: h.noop })
            .with_style(style().width(px(440)))),
        kit::sec("Bare rows (row_chrome: false)", selectable_list::SelectableList(#{ key: "selectable-people",
            label: "People", items: people(), selection_mode: "multiple", row_chrome: false,
            selected_keys: kit::val(v, "selectable_list.people", ["p1", "p3"]), height: 168.0,
            row: ctx.node_factory(Fn("person_row")),
            on_selection_change: kit::store(h, "selectable_list.people"), on_row_click: h.noop })
            .with_style(style().width(px(440)))),
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
    let compact = kit::val(v,"pagination.compact",false);
    let disabled = kit::val(v,"pagination.disabled",false);
    kit::page(h, v, "pagination", "Pagination", "Ghost page targets; the current page is a selection block, not a primary action.", [
        kit::sec("Presentation", kit::line([
            button::Button(#{key:"pagination-numbers",text:if compact{"Show page numbers"}else{"Hide page numbers"},
                variant:"secondary",on_click:kit::put(h,"pagination.compact",!compact)}),
            button::Button(#{key:"pagination-enabled",text:if disabled{"Enable main pager"}else{"Disable main pager"},
                variant:"secondary",on_click:kit::put(h,"pagination.disabled",!disabled)})
        ])),
        kit::sec("Pages", pagination::Pagination(#{ key: "pagination-hosts", label: "Hosts pages", total_items: 128,
            current_page: current.current_page, page_size: current.page_size, page_size_options: [10,20,50],
            compact:compact,disabled:disabled,
            on_change: page_change(h) })),
        kit::sec("Wrapped info and controls", pagination::Pagination(#{key:"pagination-wrapped",label:"Wrapped pages",total_items:128,
            current_page:current.current_page,page_size:current.page_size,page_size_options:[10,20,50],
            on_change:page_change(h)}).with_style(style().width(px(400)))),
        kit::sec("Compact", pagination::Pagination(#{key:"pagination-compact",label:"Compact pages",total_items:128,
            current_page:current.current_page,page_size:current.page_size,compact:true,on_change:page_change(h)})),
        kit::sec("Automatic narrow layout", pagination::Pagination(#{key:"pagination-narrow",label:"Automatic narrow pages",total_items:128,
            current_page:current.current_page,page_size:current.page_size,show_summary:false,on_change:page_change(h)})
            .with_style(style().width(px(160)))),
        kit::sec("Disabled", pagination::Pagination(#{key:"pagination-disabled",label:"Disabled pages",total_items:128,
            current_page:current.current_page,page_size:current.page_size,page_size_options:[10,20,50],disabled:true,on_change:page_change(h)})),
        kit::sec("Empty collection", pagination::Pagination(#{key:"pagination-empty",label:"Empty collection pages",total_items:0,
            current_page:1,page_size:20})),
    ])
}
