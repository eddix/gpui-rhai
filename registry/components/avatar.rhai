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

export_component(#{
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
});

fn avatar_size(size) {
    if size == "sm" { 28 } else if size == "lg" { 48 } else { 36 }
}

fn presence_color(presence) {
    if presence == "online" { theme_color("success") }
    else if presence == "away" { theme_color("warning") }
    else if presence == "busy" { theme_color("danger") }
    else { rgba(0x00000000) }
}

fn Avatar(props) {
    component_render("components/avatar", props, Fn("render_Avatar"))
}
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
    let children = [
        row([content]).with_style(
            style().width(px(size)).height(px(size)).padding(px(6))
                .items_center().justify_center().radius(px(size / 2))
                .background(theme_color("surface_hover"))
        )
    ];
    let presence = if "presence" in props { props.presence } else { "none" };
    if presence != "none" {
        children.push(text("●").with_style(component_style(props, "presence",
            style().text_color(presence_color(presence)))));
    }
    row(children)
        .with_style(component_style(props, "root", style().gap(px(2)).items_center()))
        .accessibility_role("image")
        .accessibility_label(props.name)
}
