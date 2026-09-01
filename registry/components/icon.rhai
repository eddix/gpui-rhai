/* gpui-rhai
{
  "id": "components/icon",
  "export": "Icon",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/check.svg", "icons/close.svg"]
}
*/
// Icon wraps a declarative AssetId or a runtime image handle with consistent sizing.
//
// Props: source, optional rtl_source, size, and label.
// State/events: none. Parts: root.
// Example: Icon(#{ source: asset("app/icons/check"), size: "sm", label: "Complete" })

define_component(#{
    metadata: #{
        id: "components/icon",
        "export": "Icon",
        version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [],
        capabilities: #{},
        assets: ["icons/check.svg", "icons/close.svg"],
    },
    schema: #{
        props: #{
            source: #{
                schema: #{ type: "one_of", variants: [
                    #{ type: "asset" },
                    #{ type: "handle", kind: "image" },
                ] },
                required: true,
                sensitive: false,
            },
            rtl_source: #{
                schema: #{ type: "optional", value: #{ type: "one_of", variants: [
                    #{ type: "asset" },
                    #{ type: "handle", kind: "image" },
                ] } },
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
    render: Fn("render_Icon")
});

fn icon_size(size) {
    if size == "xs" { 12 }
    else if size == "sm" { 16 }
    else if size == "lg" { 24 }
    else { 20 }
}

fn Icon(props) { render_component("components/icon", props) }
fn render_Icon(ctx, props) {
    let size = if "size" in props { props.size } else { "md" };
    let label = if "label" in props { props.label } else { "" };
    let node = if "rtl_source" in props && props.rtl_source != () {
        directional_image_source(props.source, props.rtl_source)
    } else {
        image_source(props.source)
    };
    node
        .with_style(component_style(props, "root",
            style()
                .width(px(icon_size(size)))
                .height(px(icon_size(size)))
                .text_color(theme_color("text_primary"))
                .opacity(0.92)
        ))
        .accessibility_role("image")
        .accessibility_label(label)
}
