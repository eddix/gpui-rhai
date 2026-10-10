/* gpui-rhai
{
  "id": "components/sheet",
  "export": "Sheet",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_raised", "border", "text_primary", "focus_ring", "spacing.xl", "space.group",
    "space.related", "typography.title"
  ],
  "environment": ["density"]
}
*/
// Controlled temporary modal panel attached to a viewport edge.
// State: caller owns open state.
// Example: Sheet(#{ key: "inspector", open: true, side: "end", title: "Inspector", content: panel, on_open_change: Fn("opened") })

define_component(#{
    metadata: #{ id: "components/sheet", "export": "Sheet", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "surface_raised", "border", "text_primary", "focus_ring", "spacing.xl",
            "space.group", "space.related", "typography.title"
        ],
        environment: ["density"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Identity of the sheet and the id of its overlay; keep it unique among open overlays." },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false,
                doc: "Whether the sheet is open; the caller stores the `open_change` payload and passes it back." },
            side: #{ schema: #{ type: "string", allowed: ["start", "end", "top", "bottom"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "end" },
                doc: "Window edge the sheet attaches to; `start` and `end` follow the layout direction." },
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Heading of the panel and the sheet's accessible name." },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "Body of the sheet; it fills the panel between the title and the actions." },
            actions: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Buttons in a row at the end of the panel, after the content." },
            size: #{ schema: #{ type: "integer", min: 160, max: 960 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 360 },
                doc: "Width of a `start` or `end` sheet, or height of a `top` or `bottom` one, in logical pixels." },
            dismiss_on_escape: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Whether Escape asks to close the sheet through `open_change`." },
            dismiss_on_outside: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Whether a press on the backdrop asks to close the sheet through `open_change`." },
            initial_focus: #{ schema: #{ type: "string", allowed: ["panel", "first"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "first" },
                doc: "What takes focus when the sheet opens: `first` its first focusable control, `panel` the panel itself." },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `false` when Escape or a backdrop press dismisses the sheet." },
        },
        state: #{ fields: #{} }, events: #{ open_change: #{ payload: #{ type: "bool" },
            doc: "Emitted when Escape or a backdrop press dismisses the sheet; the payload is the requested open state." } },
        slots: #{ content: #{ required: true, multiple: false, doc: "Body of the sheet." },
            actions: #{ required: false, multiple: true, doc: "Buttons in a row at the end of the panel." } },
        parts: ["root", "backdrop", "panel", "title", "content", "actions", "focus_frame"],
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
        .typography("title")
        .text_color(theme_color("text_primary"))))];
    children.push(props.content.with_style(ctx.component_style("content", style().flex_grow())));
    if "actions" in props && props.actions.len > 0 {
        children.push(row(props.actions).with_style(ctx.component_style("actions", style()
        .gap(theme_length("space.related")).justify_end())));
    }
    // A raised block with a hairline on the edge that faces the window.
    let panel_style = style().padding(theme_spacing("xl")).gap(theme_length("space.group"))
        .background(theme_color("surface_raised")).border_color(theme_color("border"));
    panel_style = if side == "start" { panel_style.border_end(px(1)) }
        else if side == "end" { panel_style.border_start(px(1)) }
        else if side == "top" { panel_style.border_bottom(px(1)) }
        else { panel_style.border_top(px(1)) };
    if side == "start" || side == "end" {
        panel_style = panel_style.width(px(size)).height(relative(1.0));
    } else {
        panel_style = panel_style.width(relative(1.0)).height(px(size));
    }
    // The 2px focus frame while the panel itself holds focus, over the hairline too.
    let frame = style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0));
    frame = if side == "start" { frame.inset_end(offset_px(-1)) }
        else if side == "end" { frame.inset_start(offset_px(-1)) }
        else if side == "top" { frame.bottom(offset_px(-1)) }
        else { frame.top(offset_px(-1)) };
    children.push(row([]).with_style(ctx.component_style("focus_frame", frame
        .border(px(2)).border_color(rgba(0x00000000))
        .group_focus(style().border_color(theme_color("focus_ring"))))));
    panel_style = panel_style.relative()
        .group_focus(style().border_color(theme_color("focus_ring")));
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
