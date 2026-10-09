/* gpui-rhai
{
  "id": "components/tag",
  "export": "Tag",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/close.svg"],
  "tokens": [
    "surface_hover", "text_primary", "disabled", "focus_ring", "tag.facet", "control.hover",
    "text.accent", "text.success", "text.warning", "text.danger", "accent", "on_accent",
    "success", "on_success", "warning", "on_warning", "danger", "on_danger",
    "metrics.marker", "spacing.xs", "spacing.sm", "radius.sm",
    "typography.caption", "typography.label"
  ],
  "environment": ["corners"]
}
*/
// Tag labels a category: a flat tonal tape with an optional facet segment.
//
// State: stateless and controlled by caller props.
// Example: Tag(#{ facet: "env", text: "prod", variant: "danger" })
//
// Variants mark a category, not a status. Without a facet they color the text;
// with a facet they fill the facet segment. Markers keep their size in both densities.

define_component(#{
    metadata: #{
        id: "components/tag", "export": "Tag", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{}, assets: ["icons/close.svg"],
        tokens: [
            "surface_hover", "text_primary", "disabled", "focus_ring", "tag.facet", "control.hover",
            "text.accent", "text.success", "text.warning", "text.danger", "accent", "on_accent",
            "success", "on_success", "warning", "on_warning", "danger", "on_danger",
            "metrics.marker", "spacing.xs", "spacing.sm", "radius.sm",
            "typography.caption", "typography.label"
        ],
        environment: ["corners"],
    },
    schema: #{
        props: #{
            text: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "The value shown; it is also the payload of `close`." },
            facet: #{ schema: #{ type: "optional", value: #{ type: "string" } },
                required: false, sensitive: false,
                doc: "Key drawn uppercase in a segment before the text, as in `ENV prod`; empty means none." },
            variant: #{
                schema: #{ type: "string", allowed: ["neutral", "accent", "success", "warning", "danger"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "neutral" },
                doc: "Category color: it colors the text, or fills the facet segment when there is a `facet`.",
            },
            closable: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Adds a close button after the text.",
            },
            close_label: #{ schema: #{ type: "optional", value: #{ type: "string" } },
                required: false, sensitive: false,
                doc: "Accessible name of the close button; defaults to `Remove <text>`." },
            disabled: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Draws the tag in the `disabled` color and makes the close button inert.",
            },
            on_close: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
                doc: "Called with the tag's `text` when the close button is clicked; not called while `disabled`.",
            },
        },
        state: #{ fields: #{} },
        events: #{ close: #{ payload: #{ type: "string" },
            doc: "Emitted when the close button is clicked; the payload is the tag's `text`." } },
        slots: #{}, parts: ["root", "facet", "label", "close"],
    },
    render: Fn("render_Tag")
});

fn tag_text_color(variant) {
    if variant == "neutral" { theme_color("text_primary") } else { theme_color(`text.${variant}`) }
}

fn Tag(props) { render_component("components/tag", props) }
fn render_Tag(ctx, props) {
    let variant = if "variant" in props { props.variant } else { "neutral" };
    let closable = if "closable" in props { props.closable } else { false };
    let disabled = if "disabled" in props { props.disabled } else { false };
    let facet = if "facet" in props && props.facet != () && props.facet != "" { props.facet } else { () };
    let children = [];
    if facet != () {
        let facet_fill = if variant == "neutral" { theme_color("tag.facet") } else { theme_color(variant) };
        let facet_text = if variant == "neutral" { theme_color("text_primary") }
            else { theme_color(`on_${variant}`) };
        // The facet carries the tag's start corners: clipping is rectangular.
        children.push(row([text(facet.to_upper())]).with_style(ctx.component_style("facet", style()
            .flex_row().items_center().self_stretch().radius_start(theme_radius("sm"))
            .padding_x(theme_spacing("sm"))
            .background(if disabled { theme_color("surface_hover") } else { facet_fill })
            .text_color(if disabled { theme_color("disabled") } else { facet_text })
            .typography("label"))));
    }
    let value_color = if disabled { theme_color("disabled") }
        else if facet != () { theme_color("text_primary") }
        else { tag_text_color(variant) };
    let value = [text(props.text).with_style(ctx.component_style("label", style()))];
    if closable {
        let label = if "close_label" in props && props.close_label != () { props.close_label }
            else { `Remove ${props.text}` };
        // The 12px close glyph and 2px focus frame are structural constants.
        let close = row([image_source(asset("app/icons/close"))
                .with_style(style().width(px(12)).height(px(12)))])
            .with_style(ctx.component_style("close", style()
                .items_center().justify_center().radius(theme_radius("sm"))
                .border(px(1)).border_color(rgba(0x00000000))
                .text_color(value_color)
                .hover(style().background(theme_color("control.hover")))
                .focus(style().border_color(theme_color("focus_ring")))))
            .accessibility_role("button")
            .accessibility_label(label);
        if !disabled && "on_close" in props && props.on_close != () {
            close = close.on_click_value(props.on_close, props.text);
        }
        value.push(close);
    }
    children.push(row(value).with_style(style()
        .flex_row().items_center().gap(theme_spacing("xs")).padding_x(theme_spacing("sm"))));
    row(children)
        .with_style(ctx.component_style("root", style()
            .flex_row().items_center()
            .min_height(theme_length("metrics.marker"))
            .radius(theme_radius("sm"))
            .background(theme_color("surface_hover"))
            .text_color(value_color)
            .typography("caption")))
        .disabled(disabled)
        .accessibility_role("status")
        .accessibility_label(if facet != () { `${facet}: ${props.text}` } else { props.text })
}
