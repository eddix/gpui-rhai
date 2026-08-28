/* gpui-rhai
{
  "id": "components/icon",
  "export": "Icon",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Icon wraps a Rust-loaded image handle with consistent size semantics.
//
// Props: handle (required LTR image handle), rtl_handle, size, label.
// State/events: none. Parts: root.
// Example: Icon(#{ handle: check_image, size: "sm", label: "Complete" })

export_component(#{
    metadata: #{
        id: "components/icon",
        "export": "Icon",
        version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [],
        capabilities: #{},
    },
    schema: #{
        props: #{
            handle: #{
                schema: #{ type: "handle", kind: "image" },
                required: true,
                sensitive: false,
            },
            rtl_handle: #{
                schema: #{ type: "optional", value: #{ type: "handle", kind: "image" } },
                required: false,
                sensitive: false,
            },
            size: #{
                schema: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "md" },
            },
            label: #{
                schema: #{ type: "optional", value: #{ type: "string" } },
                required: false,
                sensitive: false,
            },
        },
        state: #{ fields: #{} },
        events: #{},
        slots: #{},
        parts: ["root"],
    },
});

fn icon_size(size) {
    if size == "xs" { 12 }
    else if size == "sm" { 16 }
    else if size == "lg" { 24 }
    else { 20 }
}

fn Icon(props) {
    component_render("components/icon", props, Fn("render_Icon"))
}
fn render_Icon(ctx, props) {
    let size = if "size" in props { props.size } else { "md" };
    let label = if "label" in props { props.label } else { "" };
    let node = if "rtl_handle" in props && props.rtl_handle != () {
        directional_image(props.handle, props.rtl_handle)
    } else {
        image(props.handle)
    };
    node
        .with_style(component_style(props, "root",
            style()
                .width(px(icon_size(size)))
                .height(px(icon_size(size)))
                .text_color(theme_color("text_primary"))
        ))
        .accessibility_role("image")
        .accessibility_label(label)
}
