/* gpui-rhai
{
  "id": "components/scroll_area",
  "export": "ScrollArea",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["scrollbar.thumb", "scrollbar.thumb_hover"],
  "environment": []
}
*/
// Sized retained scroll viewport with themeable overlay scrollbars.
// Example: ScrollArea(#{ key: "logs", label: "Logs", height: px(240), content: long_content })

define_component(#{
    metadata: #{ id: "components/scroll_area", "export": "ScrollArea", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["scrollbar.thumb", "scrollbar.thumb_hover"],
        environment: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable identity of the viewport; the native scroll handle keeps the scroll offset and scrollbar drag state under it." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the scroll region." },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "Node that scrolls inside the viewport." },
            width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false,
                doc: "Viewport width; `()` leaves the width to the surrounding layout." },
            height: #{ schema: #{ type: "length" }, required: true, sensitive: false,
                doc: "Viewport height; the area keeps it whatever the size of its content." },
            axis: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical", "both"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "vertical" },
                doc: "Directions the content scrolls in; the scrollbar of an axis that does not scroll stays hidden." },
            horizontal_scrollbar: #{ schema: #{ type: "string", allowed: ["auto", "always", "hidden"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "auto" },
                doc: "When the horizontal bar shows: `auto` on hover and for 1.4 s after scrolling, `always` whenever content overflows." },
            vertical_scrollbar: #{ schema: #{ type: "string", allowed: ["auto", "always", "hidden"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "auto" },
                doc: "When the vertical bar shows: `auto` on hover and for 1.4 s after scrolling, `always` whenever content overflows." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ content: #{ required: true, multiple: false, doc: "Node that scrolls inside the viewport." } },
        parts: ["root", "content", "scrollbar_track", "scrollbar_thumb", "scrollbar_thumb_hover"],
    },
    render: Fn("render_ScrollArea")
});

fn ScrollArea(props) { render_component("components/scroll_area", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn render_ScrollArea(ctx, props) {
    let axis = prop_or(props, "axis", "vertical");
    let horizontal = if axis == "vertical" { "hidden" }
        else { prop_or(props, "horizontal_scrollbar", "auto") };
    let vertical = if axis == "horizontal" { "hidden" }
        else { prop_or(props, "vertical_scrollbar", "auto") };
    // A scroll area only scrolls; framing belongs to the layout around it.
    let base = style().height(props.height).relative();
    if "width" in props && props.width != () { base = base.width(props.width); }
    if axis == "horizontal" { base = base.overflow_x_scroll(); }
    else if axis == "both" { base = base.overflow_scroll(); }
    else { base = base.overflow_y_scroll(); }
    box([props.content.with_style(ctx.component_style("content", style()))])
        .with_key(props.key).with_style(ctx.component_style("root", base))
        .with_part_style("scrollbar_track", ctx.component_style("scrollbar_track",
            style().background(rgba(0x00000000))))
        .with_part_style("scrollbar_thumb", ctx.component_style("scrollbar_thumb",
            style().background(theme_color("scrollbar.thumb"))))
        .with_part_style("scrollbar_thumb_hover", ctx.component_style("scrollbar_thumb_hover",
            style().background(theme_color("scrollbar.thumb_hover"))))
        .scrollbars(horizontal, vertical)
        .accessibility_role("region").accessibility_label(props.label)
}
