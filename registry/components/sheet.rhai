/* gpui-rhai
{
  "id": "components/sheet",
  "export": "Sheet",
  "version": "0.1.1",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled temporary modal panel attached to a viewport edge.
// Props: key, open, side, title, content/actions, size, dismissal policy, and on_open_change.
// Events: open_change(bool).
// State: caller owns open state.
// Parts: root, backdrop, panel, title, content, actions.
// Example: Sheet(#{ key: "inspector", open: true, side: "end", title: "Inspector", content: panel, on_open_change: Fn("opened") })

define_component(#{
    metadata: #{ id: "components/sheet", "export": "Sheet", version: "0.1.1",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            side: #{ schema: #{ type: "string", allowed: ["start", "end", "top", "bottom"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "end" } },
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            actions: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            size: #{ schema: #{ type: "integer", min: 160, max: 960 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 360 } },
            dismiss_on_escape: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            dismiss_on_outside: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            initial_focus: #{ schema: #{ type: "string", allowed: ["panel", "first"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "first" } },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{ open_change: #{ payload: #{ type: "bool" } } },
        slots: #{ content: #{ required: true, multiple: false }, actions: #{ required: false, multiple: true } },
        parts: ["root", "backdrop", "panel", "title", "content", "actions"],
    },
    render: Fn("render_Sheet")
});

fn Sheet(props) { render_component("components/sheet", props) }
fn open_action(ctx, open) { ctx.emit("open_change", open); }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }

fn render_Sheet(ctx, props) {
    let side = prop_or(props, "side", "end");
    let size = prop_or(props, "size", 360);
    let children = [text(props.title).with_style(ctx.component_style("title", style()
        .typography("title").font_weight(700)
        .text_color(theme_color("text_primary"))))];
    children.push(props.content.with_style(ctx.component_style("content", style().flex_grow())));
    if "actions" in props && props.actions.len > 0 {
        children.push(row(props.actions).with_style(ctx.component_style("actions", style()
            .padding_top(px(10)).gap(px(8)).justify_end()
            .border_top(px(1)).border_color(theme_color("border")))));
    }
    let panel_style = style().padding(px(16)).gap(px(12))
        .background(theme_color("surface")).border(px(1)).border_color(theme_color("border"));
    if side == "start" || side == "end" {
        panel_style = panel_style.width(px(size)).height(relative(1.0));
    } else {
        panel_style = panel_style.width(relative(1.0)).height(px(size));
    }
    let panel = column(children).with_style(ctx.component_style("panel", panel_style))
        .accessibility_role("dialog").accessibility_label(props.title);
    let node = overlay(row([]), panel, #{
        id: props.key, kind: "sheet", placement: side, open: props.open, gap: 0,
        modal: true, activate_on_trigger: false,
        dismiss_on_escape: prop_or(props, "dismiss_on_escape", true),
        dismiss_on_outside: prop_or(props, "dismiss_on_outside", true),
        initial_focus: prop_or(props, "initial_focus", "first")
    }).with_key(props.key).with_style(ctx.component_style("root", style()));
    if "on_open_change" in props && props.on_open_change != () {
        node.on_open_change(Fn("open_action"))
    } else { node }
}
