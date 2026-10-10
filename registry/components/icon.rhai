/* gpui-rhai
{
  "id": "components/icon",
  "export": "Icon",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "assets": [
    "icons/check.svg", "icons/close.svg", "icons/search.svg", "icons/help.svg",
    "icons/info.svg", "icons/warning.svg"
  ],
  "tokens": ["metrics.icon"],
  "environment": ["size", "density"]
}
*/
// Icon wraps a declarative AssetId or a runtime image handle with consistent sizing.
//
// Example: Icon(#{ source: asset("app/icons/check"), size: "sm", label: "Complete" })

define_component(#{
    metadata: #{
        id: "components/icon",
        "export": "Icon",
        version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [],
        capabilities: #{},
        tokens: ["metrics.icon"],
        environment: ["size", "density"],
        // Generic glyphs no other component declares, so apps can draw them with Icon.
        assets: [
            "icons/check.svg", "icons/close.svg", "icons/search.svg", "icons/help.svg",
            "icons/info.svg", "icons/warning.svg"
        ],
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
                doc: "The glyph: a declared asset such as `asset(\"app/icons/check\")`, or a runtime image handle.",
            },
            rtl_source: #{
                schema: #{ type: "optional", value: #{ type: "one_of", variants: [
                    #{ type: "asset" },
                    #{ type: "handle", kind: "image" },
                ] } },
                required: false,
                sensitive: false,
                doc: "Glyph drawn instead of `source` in right-to-left layouts, for directional icons such as arrows.",
            },
            size: #{
                schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false,
                sensitive: false,
                doc: "Glyph box: `xs` 12, `sm` 14, `md` 16, `lg` 20 logical pixels; unset, it follows `metrics.icon`.",
            },
            label: #{
                schema: #{ type: "optional", value: #{ type: "string" } },
                required: false,
                sensitive: false,
                doc: "Accessible name; without one, or with an empty one, the icon is decorative.",
            },
        },
        state: #{ fields: #{} },
        events: #{},
        slots: #{},
        parts: ["root"],
    },
    render: Fn("render_Icon")
});

// An explicit size is an absolute glyph box; without one the icon follows the control
// size of its environment (metrics.icon).
fn icon_box(size) {
    if size == "xs" { px(12) }
    else if size == "sm" { px(14) }
    else if size == "lg" { px(20) }
    else if size == "md" { px(16) }
    else { theme_length("metrics.icon") }
}

fn Icon(props) { render_component("components/icon", props) }
fn render_Icon(ctx, props) {
    let size = if "size" in props && props.size != () { props.size } else { () };
    let label = if "label" in props && props.label != () { props.label } else { "" };
    let node = if "rtl_source" in props && props.rtl_source != () {
        directional_image_source(props.source, props.rtl_source)
    } else {
        image_source(props.source)
    };
    // Icons take the ambient text color, so they follow the control they sit in.
    node = node.with_style(ctx.component_style("root",
        style().width(icon_box(size)).height(icon_box(size)).flex_shrink(false)));
    if label == "" {
        node.accessibility_role("presentation")
    } else {
        node.accessibility_role("image").accessibility_label(label)
    }
}
