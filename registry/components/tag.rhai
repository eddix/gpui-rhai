/* gpui-rhai
{
  "id": "components/tag",
  "export": "Tag",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "assets": [],
  "tokens": ["metrics.tag_h", "metrics.tag_radius", "metrics.tag_pad", "metrics.tag_end", "metrics.tag_gap", "metrics.tag_close", "metrics.tag_icon", "tag.normal", "tag.text_accent", "tag.text_success", "tag.text_warning", "tag.text_danger", "tag.facet_neutral", "tag.facet_accent", "tag.facet_success", "tag.facet_warning", "tag.facet_danger", "typography.tag"],
  "environment": []
}
*/
// Tag labels a category with a removable-chip surface and an optional facet segment.
//
// State: stateless and controlled by caller props.
// Example: Tag(#{ facet: "env", text: "prod", variant: "danger" })
//
// Variants mark a category, not a status. Without a facet they color the text;
// with a facet they use a paired soft fill and ink. Markers keep their size in both densities.

define_component(#{
    metadata: #{
        id: "components/tag", "export": "Tag", version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{}, assets: [],
        tokens: ["metrics.tag_h", "metrics.tag_radius", "metrics.tag_pad", "metrics.tag_end", "metrics.tag_gap", "metrics.tag_close", "metrics.tag_icon", "tag.normal", "tag.text_accent", "tag.text_success", "tag.text_warning", "tag.text_danger", "tag.facet_neutral", "tag.facet_accent", "tag.facet_success", "tag.facet_warning", "tag.facet_danger", "typography.tag"],
        environment: [],
    },
    schema: #{
        props: #{
            text: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "The value shown; it is also the payload of `close`." },
            facet: #{ schema: #{ type: "optional", value: #{ type: "string" } },
                required: false, sensitive: false,
                doc: "Key drawn as written in a segment before the text, as in `env prod`; empty means none." },
            variant: #{
                schema: #{ type: "string", allowed: ["neutral", "accent", "success", "warning", "danger"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "neutral" },
                doc: "Category tone: it colors the text, or supplies a paired soft fill/foreground for a facet segment.",
            },
            closable: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Adds a close button after the text.",
            },
            prefix: #{ schema: #{ type: "optional", value: #{ type: "node" } },
                required: false, sensitive: false, doc: "An optional avatar or icon before the label, inside the value segment." },
            close_tab_stop: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Whether the close button owns a Tab stop; use false for an editor that navigates its chip rows itself." },
            close_label: #{ schema: #{ type: "optional", value: #{ type: "string" } },
                required: false, sensitive: false,
                doc: "Accessible name of the close button; defaults to the localized `tag.remove` message, `Remove <text>` in English." },
            disabled: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Dims the complete chip to 55% opacity and makes the close button inert.",
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
        slots: #{}, parts: ["root", "facet", "value", "label", "close", "close_icon", "prefix"],
    },
    render: Fn("render_Tag")
});

fn Tag(props) { render_component("components/tag", props) }
fn render_Tag(ctx, props) {
    let variant = if "variant" in props { props.variant } else { "neutral" };
    let closable = if "closable" in props { props.closable } else { false };
    let disabled = if "disabled" in props { props.disabled } else { false };
    let facet = if "facet" in props && props.facet != () && props.facet != "" { props.facet } else { () };
    let children = [];
    if facet != () {
        children.push(row([text(facet)]).with_key("facet")
            .with_style(ctx.component_style("facet", style()
                .flex_row().items_center().self_stretch().flex_shrink(false)
                .min_height(theme_length("metrics.tag_h"))
                .radius_start(theme_length("metrics.tag_radius")).radius_end(px(0))
                .padding_x(theme_length("metrics.tag_pad")),
                ctx.theme_style(`tag.facet_${variant}`))));
    }
    let value = [];
    if "prefix" in props && props.prefix != () {
        value.push(row([props.prefix]).with_key("prefix")
            .with_style(ctx.component_style("prefix",style().flex_shrink(false).items_center())));
    }
    value.push(text(props.text).with_key("label").with_style(ctx.component_style("label", style()
        .min_width(px(0)).whitespace_nowrap().text_ellipsis())));
    if closable {
        let label = if "close_label" in props && props.close_label != () { props.close_label }
            else { ctx.t_or("tag.remove", "Remove {name}", #{ name: props.text }) };
        let glyph = svg(`<svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M3 3l6 6M9 3l-6 6"/></svg>`)
            .with_style(ctx.component_style("close_icon", style()
                .width(theme_length("metrics.tag_icon")).height(theme_length("metrics.tag_icon"))))
            .accessibility_role("presentation");
        let close = row([glyph]).with_key("close")
            .with_style(ctx.component_style("close", style()
                .width(theme_length("metrics.tag_close")).height(theme_length("metrics.tag_close"))
                .flex_shrink(false).items_center().justify_center().focus(style())))
            .disabled(disabled)
            .tab_stop(!disabled && props.close_tab_stop && "on_close" in props && props.on_close != ())
            .accessibility_role("button")
            .accessibility_label(label);
        if !disabled && "on_close" in props && props.on_close != () {
            close = close.on_click_value(props.on_close, props.text);
        }
        value.push(close);
    }
    children.push(row(value).with_key("value").with_style(ctx.component_style("value", style()
        .flex_row().min_width(px(0)).items_center().gap(theme_length("metrics.tag_gap"))
        .padding_start(theme_length("metrics.tag_pad")),
        style().padding_end(theme_length(if closable {"metrics.tag_end"} else {"metrics.tag_pad"})))));
    row(children)
        .with_style(ctx.component_style("root", style()
            .flex_row().min_width(px(0)).items_center()
            .min_height(theme_length("metrics.tag_h"))
            .radius(theme_length("metrics.tag_radius")).typography("tag"),
            ctx.theme_style(if facet != () || variant == "neutral" {"tag.normal"} else {`tag.text_${variant}`})))
        .disabled(disabled)
        .accessibility_role("status")
        .accessibility_label(if facet != () { `${facet}: ${props.text}` } else { props.text })
}
