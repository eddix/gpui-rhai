/* gpui-rhai
{
  "id": "components/tag",
  "export": "Tag",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/close.svg"]
}
*/
// Compact semantic label with optional close action.
// Props: text, variant, closable, disabled, on_close.
// Events: close(string). Parts: root, label, close.
// State: stateless and controlled by caller props.
// Example: Tag(#{ text: "Rust", closable: true, on_close: Fn("closed") })

define_component(#{
    metadata: #{
        id: "components/tag", "export": "Tag", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{}, assets: ["icons/close.svg"],
    },
    schema: #{
        props: #{
            text: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            variant: #{
                schema: #{ type: "string", allowed: ["neutral", "accent", "success", "warning", "danger"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "neutral" },
            },
            closable: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            disabled: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            on_close: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
            },
        },
        state: #{ fields: #{} },
        events: #{ close: #{ payload: #{ type: "string" } } },
        slots: #{}, parts: ["root", "label", "close"],
    },
    render: Fn("render_Tag")
});

fn tag_color(variant) {
    theme_color("surface_raised")
}

fn tag_foreground(variant) {
    if variant == "accent" { theme_color("accent") }
    else if variant == "success" { theme_color("success") }
    else if variant == "warning" { theme_color("warning") }
    else if variant == "danger" { theme_color("danger") }
    else { theme_color("text_primary") }
}

fn tag_border(variant) {
    if variant == "accent" { theme_color("accent") }
    else if variant == "success" { theme_color("success") }
    else if variant == "warning" { theme_color("warning") }
    else if variant == "danger" { theme_color("danger") }
    else { theme_color("border") }
}

fn Tag(props) { render_component("components/tag", props) }
fn render_Tag(ctx, props) {
    let variant = if "variant" in props { props.variant } else { "neutral" };
    let closable = if "closable" in props { props.closable } else { false };
    let disabled = if "disabled" in props { props.disabled } else { false };
    let children = [text(props.text).with_style(component_style(props, "label", style()))];
    if closable {
        let close = image_source(asset("app/icons/close"))
            .with_style(component_style(props, "close", style()
                .width(px(14)).height(px(14)).items_center().justify_center()
                .text_color(if disabled { theme_color("disabled") } else { tag_foreground(variant) })
                .radius(theme_radius("sm")).hover(style().background(theme_color("surface_hover")))))
            .accessibility_role("button")
            .accessibility_label(props.text);
        if !disabled && "on_close" in props && props.on_close != () {
            close = close.on_click_value(props.on_close, props.text);
        }
        children.push(close);
    }
    row(children)
        .with_style(component_style(props, "root",
            style().height(px(22)).padding_x(px(6)).gap(px(6)).items_center()
                .radius(theme_radius("sm"))
                .border(px(1)).border_color(tag_border(variant))
                .background(tag_color(variant))
                .typography("body_small").font_weight(700)
                .text_color(if disabled { theme_color("disabled") } else { tag_foreground(variant) })
                .disabled(style().opacity(0.68))
        ))
        .disabled(disabled)
        .accessibility_role("status")
        .accessibility_label(props.text)
}
