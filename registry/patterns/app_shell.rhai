/* gpui-rhai
{
  "id": "patterns/app_shell",
  "export": "AppShell",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/title_bar", "components/status_bar"],
  "capabilities": {},
  "tokens": ["surface", "surface_raised", "focus_ring", "metrics.label_column"],
  "environment": []
}
*/
// AppShell is the window frame of a productivity tool: title bar, sidebar, main area,
// optional inspector, status bar, and keyboard regions.
//
// Example: AppShell(#{ key: "app", label: "Workbench", title_bar: #{ title: "Workbench" },
//              sidebar: nav_node, main: region_node, status: #{ start: [text("Ready")] } })
//
// The sidebar and inspector are raised color blocks beside the main surface; no rules.
// F6 and Shift+F6 move focus from the region that holds it to the next or previous present
// region; a region shows the 2px ink frame while it holds focus itself, and Tab then enters
// it. With focus outside every region they go on from the last region F6 moved to, which the
// shell keeps as its only state.

import "components/title_bar" as title_bar;
import "components/status_bar" as status_bar;

define_component(#{
    metadata: #{
        id: "patterns/app_shell", "export": "AppShell", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/title_bar", "components/status_bar"], capabilities: #{},
        tokens: ["surface", "surface_raised", "focus_ring", "metrics.label_column"],
        environment: [],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Identity of the shell; the keys of its regions, which F6 focuses, derive from it." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the shell; it also names the title bar (`<label> title bar`) and the status bar (`<label> status`)." },
            title_bar: #{ schema: #{ type: "optional", value: #{ type: "object", allow_unknown: false, fields: #{
                title: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "node" }] },
                    required: true, sensitive: false,
                    doc: "Title text, or a node such as a breadcrumb; a string is set semibold and the subtitle truncates before it." },
                subtitle: #{ schema: #{ type: "optional", value: #{ type: "one_of",
                    variants: [#{ type: "string" }, #{ type: "node" }] } }, required: false, sensitive: false,
                    doc: "Muted text or node on the title's line; it truncates first." },
                start: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] },
                    doc: "Nodes before the title, at the start edge." },
                center: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] },
                    doc: "Nodes centered in the bar." },
                end: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] },
                    doc: "Nodes at the end edge; they keep their width while the title truncates." },
                inset_start: #{ schema: #{ type: "integer", min: 0, max: 256 }, required: false,
                    sensitive: false, "default": #{ type: "integer", value: 0 },
                    doc: "Logical pixels kept free at the start edge, room for the macOS window buttons." },
                window_drag: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                    "default": #{ type: "bool", value: false },
                    doc: "Whether pressing the bar's background moves the window and a double press zooms it." },
            } } }, required: false, sensitive: false,
                doc: "TitleBar props without `label`, for the bar across the top; omitted, the shell has no title bar." },
            sidebar: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Raised region before the main area, and an F6 stop." },
            main: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "The main working area; it takes the free width and is always an F6 stop." },
            inspector: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Raised region after the main area, and an F6 stop." },
            status: #{ schema: #{ type: "optional", value: #{ type: "object", allow_unknown: false, fields: #{
                start: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] },
                    doc: "Fields at the start edge; they truncate when space runs out." },
                center: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] },
                    doc: "Fields centered in the bar." },
                end: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] },
                    doc: "Fields at the end edge; they keep their width." },
            } } }, required: false, sensitive: false,
                doc: "StatusBar props without `label`, for the bar along the bottom; omitted, the shell has no status bar." },
            overlays: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Nodes placed after the bars, for overlays such as dialogs, sheets and toasts." },
            sidebar_width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false,
                doc: "Width of the sidebar; omitted, twice `metrics.label_column`." },
            inspector_width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false,
                doc: "Width of the inspector; omitted, 320 logical pixels." },
        },
        state: #{ fields: #{ region: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } } } },
        events: #{},
        slots: #{
            sidebar: #{ required: false, multiple: false, doc: "Raised region before the main area." },
            main: #{ required: true, multiple: false, doc: "The main working area." },
            inspector: #{ required: false, multiple: false, doc: "Raised region after the main area." },
            overlays: #{ required: false, multiple: true, doc: "Overlays such as dialogs, sheets and toasts." },
        },
        parts: ["root", "sidebar", "main", "inspector", "region_frame"],
    },
    render: Fn("render_AppShell")
});

fn AppShell(props) { render_component("patterns/app_shell", props) }
fn has(map, name) { name in map && map[name] != () }

fn move_region(ctx, target) {
    ctx.set_state("region", target.index);
    // Headless lifecycles have no window to move focus in.
    try { ctx.focus(target.key); } catch { }
}

// A keyboard region: focusable as a whole so F6 can land on it, framed while it holds
// focus itself. It is also a tab stop, before its own controls: a node must be one to own
// the focus its frame shows.
fn region_pane(ctx, key, part, content, pane) {
    let frame = row([]).with_style(ctx.component_style("region_frame", style()
        .absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
        .border(px(2)).border_color(rgba(0x00000000))
        .group_focus(style().border_color(theme_color("focus_ring")))));
    column([content, frame])
        .with_key(key).with_ref(element_ref(key))
        .with_style(ctx.component_style(part, pane.focus(style())))
        .tab_group()
}

fn render_AppShell(ctx, props) {
    let panes = [];
    let keys = [];
    if has(props, "sidebar") {
        let key = `${props.key}-sidebar`;
        let width = if has(props, "sidebar_width") { props.sidebar_width }
            else { theme_length("metrics.label_column") * 2 };
        panes.push(region_pane(ctx, key, "sidebar", props.sidebar, style()
            .flex_col().relative().width(width).flex_shrink(false).min_height(px(0))
            .background(theme_color("surface_raised"))));
        keys.push(key);
    }
    let main_key = `${props.key}-main`;
    panes.push(region_pane(ctx, main_key, "main", props.main, style()
        .flex_col().relative().flex_grow().min_width(px(0)).min_height(px(0))
        .background(theme_color("surface"))));
    keys.push(main_key);
    if has(props, "inspector") {
        let key = `${props.key}-inspector`;
        let width = if has(props, "inspector_width") { props.inspector_width } else { px(320) };
        panes.push(region_pane(ctx, key, "inspector", props.inspector, style()
            .flex_col().relative().width(width).flex_shrink(false).min_height(px(0))
            .background(theme_color("surface_raised"))));
        keys.push(key);
    }
    let count = keys.len;
    for index in 0..count {
        let next = (index + 1) % count;
        let previous = (index + count - 1) % count;
        panes[index] = panes[index]
            .on_key_value("f6", Fn("move_region"), #{ index: next, key: keys[next] })
            .on_key_value("shift+f6", Fn("move_region"), #{ index: previous, key: keys[previous] });
    }
    let children = [];
    if has(props, "title_bar") {
        let bar = props.title_bar;
        bar.label = `${props.label} title bar`;
        children.push(title_bar::TitleBar(bar));
    }
    children.push(row(panes).with_style(style().flex_row().flex_grow().min_height(px(0))));
    if has(props, "status") {
        let bar = props.status;
        bar.label = `${props.label} status`;
        children.push(status_bar::StatusBar(bar));
    }
    for overlay in if "overlays" in props { props.overlays } else { [] } { children.push(overlay); }
    // Focus outside the regions: the title bar, the status bar or an overlay.
    let current = ctx.get_state("region") % count;
    let next = (current + 1) % count;
    let previous = (current + count - 1) % count;
    column(children).with_key(props.key)
        .with_style(ctx.component_style("root", style()
            .flex_col().width(relative(1.0)).height(relative(1.0)).background(theme_color("surface"))))
        .accessibility_role("group").accessibility_label(props.label)
        .on_key_value("f6", Fn("move_region"), #{ index: next, key: keys[next] })
        .on_key_value("shift+f6", Fn("move_region"), #{ index: previous, key: keys[previous] })
}
