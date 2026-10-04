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
// Props: key, label, content, width/height, axis, and per-axis auto/always/hidden visibility.
// Events/state: emits no events; native ScrollHandle owns only viewport offset and drag state.
// Parts: root, content, scrollbar_track, scrollbar_thumb, scrollbar_thumb_hover.
// Example: ScrollArea(#{ key: "logs", label: "Logs", height: px(240), content: long_content })

define_component(#{
    metadata: #{ id: "components/scroll_area", "export": "ScrollArea", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["scrollbar.thumb", "scrollbar.thumb_hover"],
        environment: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false },
            height: #{ schema: #{ type: "length" }, required: true, sensitive: false },
            axis: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical", "both"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "vertical" } },
            horizontal_scrollbar: #{ schema: #{ type: "string", allowed: ["auto", "always", "hidden"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "auto" } },
            vertical_scrollbar: #{ schema: #{ type: "string", allowed: ["auto", "always", "hidden"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "auto" } },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{ content: #{ required: true, multiple: false } },
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
