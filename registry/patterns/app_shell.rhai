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
// Props: key and label (required), main (required), title_bar (TitleBar props without
//        label), sidebar, inspector, status (StatusBar props without label), overlays,
//        sidebar_width, inspector_width.
// Events/state: the last region moved to by F6. Parts: root, sidebar, main, inspector,
//        region_frame.
// Example: AppShell(#{ key: "app", label: "Workbench", title_bar: #{ title: "Workbench" },
//              sidebar: nav_node, main: region_node, status: #{ start: [text("Ready")] } })
//
// The sidebar and inspector are raised color blocks beside the main surface; no rules.
// F6 and Shift+F6 move focus between the present regions; a region shows the 2px ink frame
// while it holds focus itself, and Tab then enters it.

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
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            title_bar: #{ schema: #{ type: "optional", value: #{ type: "object", allow_unknown: false, fields: #{
                title: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "node" }] },
                    required: true, sensitive: false },
                subtitle: #{ schema: #{ type: "optional", value: #{ type: "one_of",
                    variants: [#{ type: "string" }, #{ type: "node" }] } }, required: false, sensitive: false },
                start: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] } },
                center: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] } },
                end: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] } },
                inset_start: #{ schema: #{ type: "integer", min: 0, max: 256 }, required: false,
                    sensitive: false, "default": #{ type: "integer", value: 0 } },
            } } }, required: false, sensitive: false },
            sidebar: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            main: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            inspector: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            status: #{ schema: #{ type: "optional", value: #{ type: "object", allow_unknown: false, fields: #{
                start: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] } },
                center: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] } },
                end: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                    required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            } } }, required: false, sensitive: false },
            overlays: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            sidebar_width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false },
            inspector_width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{ region: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } } } },
        events: #{},
        slots: #{
            sidebar: #{ required: false, multiple: false }, main: #{ required: true, multiple: false },
            inspector: #{ required: false, multiple: false }, overlays: #{ required: false, multiple: true },
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
// focus itself. It is not a tab stop; Tab from it enters its first control.
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
    let count = keys.len;
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
