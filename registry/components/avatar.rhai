/* gpui-rhai
{
  "id": "components/avatar",
  "export": "Avatar",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Avatar displays an image handle or name-derived initials with presence.
// Props: name, handle, initials, size, presence. Parts: root, image, initials, presence.
// State/events: stateless; emits no events.
// Example: Avatar(#{ name: "Ada Lovelace", initials: "AL", presence: "online" })

define_component(#{
    metadata: #{
        id: "components/avatar", "export": "Avatar", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{},
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
                schema: #{ type: "string", allowed: ["sm", "md", "lg"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "md" },
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

fn avatar_size(size) {
    if size == "sm" { 28 } else if size == "lg" { 44 } else { 36 }
}

fn presence_color(presence) {
    if presence == "online" { theme_color("success") }
    else if presence == "away" { theme_color("warning") }
    else if presence == "busy" { theme_color("danger") }
    else { rgba(0x00000000) }
}

fn Avatar(props) { render_component("components/avatar", props) }
fn render_Avatar(ctx, props) {
    let size = avatar_size(if "size" in props { props.size } else { "md" });
    let initials = if "initials" in props && props.initials != () {
        props.initials
    } else if props.name.len > 0 {
        props.name[0].to_string()
    } else { "" };
    let content = if "handle" in props && props.handle != () {
        image(props.handle).with_style(component_style(props, "image", style().width(px(size)).height(px(size))))
    } else {
        text(initials).with_style(component_style(props, "initials",
            style().text_color(theme_color("text_primary"))))
    };
    let face = row([content]).with_style(
        style().width(px(size)).height(px(size)).clip()
            .items_center().justify_center().radius(theme_radius("md"))
            .border(px(1)).border_color(theme_color("border"))
            .background(theme_color("surface_raised"))
            .font_size(px(if size < 36 { 11 } else { 12 })).font_weight(700));
    let children = [face];
    let presence = if "presence" in props { props.presence } else { "none" };
    if presence != "none" {
        children.push(row([]).with_style(component_style(props, "presence",
            style().width(px(8)).height(px(8)).absolute().right(px(0)).bottom(px(0))
                .radius(px(1)).border(px(2)).border_color(theme_color("surface"))
                .background(presence_color(presence)))));
    }
    stack(children)
        .with_style(component_style(props, "root",
            style().width(px(size)).height(px(size))))
        .accessibility_role("image")
        .accessibility_label(props.name)
}
