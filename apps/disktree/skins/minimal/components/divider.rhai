/* gpui-rhai
{
  "id": "components/divider",
  "export": "Divider",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["border"],
  "environment": []
}
*/
// Divider separates adjacent content without owning layout spacing.
//
// Example: Divider(#{ orientation: "horizontal" })

define_component(#{
    metadata: #{
        id: "components/divider",
        "export": "Divider",
        version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [],
        capabilities: #{},
        tokens: ["border"],
        environment: [],
    },
    schema: #{
        props: #{
            orientation: #{
                schema: #{ type: "string", allowed: ["horizontal", "vertical"] },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "horizontal" },
                doc: "`horizontal` draws a full-width line; `vertical` a full-height one.",
            },
            thickness: #{
                schema: #{ type: "integer", min: 1 },
                required: false,
                sensitive: false,
                "default": #{ type: "integer", value: 1 },
                doc: "Line thickness in logical pixels; values above 8 are drawn as 8.",
            },
        },
        state: #{ fields: #{} },
        events: #{},
        slots: #{},
        parts: ["root"],
    },
    render: Fn("render_Divider")
});

fn Divider(props) { render_component("components/divider", props) }
fn render_Divider(ctx, props) {
    let orientation = if "orientation" in props {
        props.orientation
    } else {
        "horizontal"
    };
    let requested_thickness = if "thickness" in props { props.thickness } else { 1 };
    let thickness = if requested_thickness < 1 {
        1
    } else if requested_thickness > 8 {
        8
    } else {
        requested_thickness
    };
    let base = style().background(theme_color("border"));
    let divider_style = if orientation == "vertical" {
        base.width(px(thickness)).height(relative(1.0))
    } else {
        base.width(relative(1.0)).height(px(thickness))
    };

    row([])
        .with_style(ctx.component_style("root", divider_style))
        .accessibility_role("separator")
        .accessibility_orientation(orientation)
}
