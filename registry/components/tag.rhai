/* gpui-rhai
{
  "id": "components/tag",
  "export": "Tag",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Compact semantic label with optional close action.
// Props: text, variant, closable, disabled, on_close.
// Events: close(string). Parts: root, label, close.
// State: stateless and controlled by caller props.
// Example: Tag(#{ text: "Rust", closable: true, on_close: Fn("closed") })

export_component(#{
    metadata: #{
        id: "components/tag", "export": "Tag", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{},
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
});

fn tag_color(variant) {
    if variant == "accent" { theme_color("accent") }
    else if variant == "success" { theme_color("success") }
    else if variant == "warning" { theme_color("warning") }
    else if variant == "danger" { theme_color("danger") }
    else { theme_color("surface_hover") }
}

fn tag_foreground(variant) {
    if variant == "accent" { theme_color("on_accent") }
    else if variant == "success" { theme_color("on_success") }
    else if variant == "warning" { theme_color("on_warning") }
    else if variant == "danger" { theme_color("on_danger") }
    else { theme_color("text_primary") }
}

fn Tag(props) {
    component_render("components/tag", props, Fn("render_Tag"))
}
fn render_Tag(ctx, props) {
    let variant = if "variant" in props { props.variant } else { "neutral" };
    let closable = if "closable" in props { props.closable } else { false };
    let disabled = if "disabled" in props { props.disabled } else { false };
    let children = [text(props.text).with_style(component_style(props, "label", style()))];
    if closable {
        let close = text("×")
            .with_style(component_style(props, "close", style()))
            .accessibility_role("button")
            .accessibility_label(props.text);
        if !disabled && "on_close" in props && props.on_close != () {
            close = close.on_click_value(props.on_close, props.text);
        }
        children.push(close);
    }
    row(children)
        .with_style(component_style(props, "root",
            style().height(px(24)).padding_x(px(8)).gap(px(6)).items_center()
                .radius(px(12)).background(tag_color(variant))
                .text_color(if disabled { theme_color("disabled") } else { tag_foreground(variant) })
        ))
        .disabled(disabled)
        .accessibility_role("status")
        .accessibility_label(props.text)
}
