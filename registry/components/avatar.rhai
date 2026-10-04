/* gpui-rhai
{
  "id": "components/avatar",
  "export": "Avatar",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_hover", "surface", "text_primary", "success", "warning", "danger", "text_muted",
    "metrics.control", "typography.control"
  ],
  "environment": ["size", "density"]
}
*/
// Avatar displays an image handle or name-derived initials with presence.
// Props: name, handle, initials, size, presence. Parts: root, image, initials, presence.
// State/events: stateless; emits no events.
// Example: Avatar(#{ name: "Ada Lovelace", initials: "AL", presence: "online" })

define_component(#{
    metadata: #{
        id: "components/avatar", "export": "Avatar", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "surface_hover", "surface", "text_primary", "success", "warning", "danger",
            "text_muted", "metrics.control", "typography.control"
        ],
        environment: ["size", "density"],
    },
    schema: #{
        props: #{
            name: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            handle: #{
                schema: #{ type: "optional", value: #{ type: "handle", kind: "image" } },
                required: false, sensitive: false,
            },
            initials: #{
                schema: #{ type: "optional", value: #{ type: "string" } },
                required: false, sensitive: false,
            },
            size: #{
                schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false,
            },
            presence: #{
                schema: #{ type: "string", allowed: ["none", "online", "away", "busy"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "none" },
            },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{},
        parts: ["root", "image", "initials", "presence"],
    },
    render: Fn("render_Avatar")
});

fn presence_color(presence) {
    if presence == "online" { theme_color("success") }
    else if presence == "away" { theme_color("warning") }
    else if presence == "busy" { theme_color("danger") }
    else { rgba(0x00000000) }
}

fn Avatar(props) { render_component("components/avatar", props) }
fn render_Avatar(ctx, props) {
    let initials = if "initials" in props && props.initials != () {
        props.initials
    } else if props.name.len > 0 {
        props.name[0].to_string()
    } else { "" };
    // Avatars share the control height of their size, so they sit in toolbars and rows.
    let side = theme_length("metrics.control");
    let content = if "handle" in props && props.handle != () {
        image(props.handle).with_style(ctx.component_style("image", style().width(side).height(side)))
    } else {
        text(initials).with_style(ctx.component_style("initials",
            style().text_color(theme_color("text_primary"))))
    };
    let face = row([content]).with_style(
        style().width(side).height(side).clip()
            .items_center().justify_center().radius(side * 0.5)
            .background(theme_color("surface_hover"))
            .typography("control").font_weight(600));
    let children = [face];
    let presence = if "presence" in props { props.presence } else { "none" };
    if presence != "none" {
        // The presence dot is a semantic 8px circle with a 2px surface ring.
        children.push(row([]).with_style(ctx.component_style("presence",
            style().width(px(8)).height(px(8)).absolute().right(px(0)).bottom(px(0))
                .radius(px(4)).border(px(2)).border_color(theme_color("surface"))
                .background(presence_color(presence)))));
    }
    let root = stack(children)
        .with_style(ctx.component_style("root", style().width(side).height(side)))
        .accessibility_role("image")
        .accessibility_label(props.name);
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
