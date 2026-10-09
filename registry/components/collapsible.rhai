/* gpui-rhai
{
  "id": "components/collapsible",
  "export": "Collapsible",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/chevron_down.svg", "icons/chevron_right.svg"],
  "tokens": [
    "surface_hover", "focus_ring", "text_primary", "text_muted", "disabled", "metrics.row",
    "metrics.inset", "metrics.icon", "spacing.xs", "typography.body"
  ],
  "environment": ["density"]
}
*/
// Controlled disclosure with trigger/content slots and Rust clip animation.
// Props: key, open, trigger, content, content_height (optional: pixels or a Length;
//        omitted, the panel fits its content), disabled, on_open_change.
// State/events: controlled open state; emits open_change(bool).
// Example: Collapsible(#{ key: "details", open: true, trigger: text("Details"), content: text("Body") })

define_component(#{
    metadata: #{
        id: "components/collapsible", "export": "Collapsible", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        assets: ["icons/chevron_down.svg", "icons/chevron_right.svg"],
        tokens: [
            "surface_hover", "focus_ring", "text_primary", "text_muted", "disabled",
            "metrics.row", "metrics.inset", "metrics.icon", "spacing.xs", "typography.body"
        ],
        environment: ["density"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            trigger: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            content_height: #{ schema: #{ type: "optional", value: #{ type: "one_of", variants: [
                #{ type: "integer", min: 0 }, #{ type: "length" }] } }, required: false, sensitive: false },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{ open_change: #{ payload: #{ type: "bool" } } },
        slots: #{
            trigger: #{ required: true, multiple: false },
            content: #{ required: true, multiple: false },
        },
        parts: ["root", "trigger", "content", "indicator", "focus_frame"],
    },
    render: Fn("render_Collapsible")
});

fn Collapsible(props) { render_component("components/collapsible", props) }
// The panel's open height: an integer as given; otherwise measured after layout, from a
// Length (a zero-width sizer resolves it in this subtree's density) or from the content
// itself, so a density switch changes it. On the first frame it is unknown and an open
// panel takes the content's own height. The transition starts where it ends when first
// declared, so a panel mounted open does not grow in; toggles animate from the current
// height.
fn disclosure_panel(ctx, key, open, height, content) {
    let children = [];
    let target = ();
    // Only a measured panel owns an element ref, so a fixed height renders without a lifecycle.
    let measure = if type_of(height) == "i64" { () } else { element_ref(`${key}-measure`) };
    if type_of(height) == "i64" {
        target = height * 1.0;
        children.push(content);
    } else if height == () {
        children.push(column([content]).with_key(`${key}-measure`).with_ref(measure)
            .with_style(style().flex_shrink(false)));
    } else {
        children.push(content);
        children.push(row([]).with_key(`${key}-measure`).with_ref(measure)
            .with_style(style().absolute().top(px(0)).left(px(0)).width(px(0)).height(height)));
    }
    if target == () {
        let bounds = ctx.element_bounds(measure);
        if bounds != () { target = bounds.layout.height; }
    }
    let panel_style = style().clip().relative();
    if !open { panel_style = panel_style.height(px(0)); }
    else if target != () { panel_style = panel_style.height(px(target)); }
    let panel = column(children).with_key(`${key}-content`).with_style(panel_style);
    if target == () { return panel; }
    let end = if open { target } else { 0.0 };
    panel.motion(motion_transition("clip_height", end, end,
        #{ duration_ms: ctx.motion_duration("fast"), easing: ctx.motion_easing("entrance"),
            intent: "feedback" }))
}
fn render_Collapsible(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    // The header is a list row around the caller's trigger content; focus is a 2px
    // frame drawn over it.
    let focus_frame = row([]).with_style(ctx.component_style("focus_frame", style()
        .absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
        .border(px(2)).border_color(rgba(0x00000000))
        .group_focus(style().border_color(theme_color("focus_ring")))));
    let indicator = image_source(asset(if props.open { "app/icons/chevron_down" } else { "app/icons/chevron_right" }))
        .with_style(ctx.component_style("indicator", style()
            .width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
            .text_color(theme_color("text_muted"))));
    let header_style = style().min_height(theme_length("metrics.row"))
        .padding_x(theme_length("metrics.inset")).gap(theme_spacing("xs")).items_center()
        .typography("body").font_weight(600).text_color(theme_color("text_primary"));
    header_style = header_style
        .hover(style().background(theme_color("surface_hover")))
        .focus(style())
        .disabled(style().text_color(theme_color("disabled")));
    let trigger = row([indicator, props.trigger.with_style(style().flex_grow()), focus_frame])
        .with_style(ctx.component_style("trigger", header_style))
        .disabled(disabled)
        .accessibility_role("button")
        .accessibility_expanded(props.open);
    if !disabled && "on_open_change" in props && props.on_open_change != () {
        trigger = trigger.on_click_value(props.on_open_change, !props.open);
    }
    let height = if "content_height" in props { props.content_height } else { () };
    let panel_content = props.content.with_style(ctx.component_style("content", style()
            .padding_x(theme_length("metrics.inset")).padding_bottom(theme_length("metrics.inset"))
            .typography("body")));
    let panel = disclosure_panel(ctx, props.key, props.open, height, panel_content);
    column([trigger, panel]).with_key(props.key)
        .with_style(ctx.component_style("root", style()))
}
