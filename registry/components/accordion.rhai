/* gpui-rhai
{
  "id": "components/accordion",
  "export": "Accordion",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/minus.svg", "icons/plus.svg"],
  "tokens": [
    "border", "surface_hover", "focus_ring", "text_primary", "text_muted", "disabled",
    "metrics.row", "metrics.inset", "metrics.icon", "typography.body"
  ],
  "environment": ["density"]
}
*/
// Controlled single/multiple accordion with Rust-side clip motion.
// Props: key, items ({key, title, content, content_height?, disabled?}; content_height is
//        pixels or a Length, omitted the panel fits its content), expanded, mode, disabled,
//        on_change.
// State/events: controlled expanded values; emits change(array<string>).
// Example: Accordion(#{ key: "faq", items: items, expanded: [], on_change: Fn("changed") })

define_component(#{
    metadata: #{
        id: "components/accordion", "export": "Accordion", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "border", "surface_hover", "focus_ring", "text_primary", "text_muted", "disabled",
            "metrics.row", "metrics.inset", "metrics.icon", "typography.body"
        ],
        environment: ["density"],
        assets: ["icons/minus.svg", "icons/plus.svg"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            expanded: #{ schema: #{ type: "array", max_items: 256, items: #{ type: "string" } }, required: true, sensitive: false },
            mode: #{ schema: #{ type: "string", allowed: ["single", "multiple"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "single" } },
            items: #{
                schema: #{ type: "array", max_items: 256, items: #{
                    type: "object", allow_unknown: false, fields: #{
                        key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                        title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                        content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
                        content_height: #{ schema: #{ type: "optional", value: #{ type: "one_of", variants: [
                            #{ type: "integer", min: 0 }, #{ type: "length" }] } }, required: false,
                            sensitive: false },
                        disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                            "default": #{ type: "bool", value: false } },
                    },
                } }, required: true, sensitive: false,
            },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{ change: #{ payload: #{ type: "array", max_items: 256, items: #{ type: "string" } } } },
        slots: #{}, parts: ["root", "item", "trigger", "content", "focus_frame"],
    },
    render: Fn("render_Accordion")
});

fn has_value(values, key) { key in values }

fn toggled(values, key, mode) {
    if mode == "single" {
        if has_value(values, key) { [] } else { [key] }
    } else {
        let next = [];
        for value in values { if value != key { next.push(value); } }
        if !has_value(values, key) { next.push(key); }
        next
    }
}

fn Accordion(props) { render_component("components/accordion", props) }
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
fn render_Accordion(ctx, props) {
    let mode = if "mode" in props { props.mode } else { "single" };
    let all_disabled = if "disabled" in props { props.disabled } else { false };
    let callback = if "on_change" in props { props.on_change } else { () };
    let items = [];
    for item in props.items {
        let open = has_value(props.expanded, item.key);
        let item_disabled = all_disabled || ("disabled" in item && item.disabled);
        // Headers are list rows: metrics.row high, text at metrics.inset. Focus is a 2px
        // frame drawn over the row so it never moves the title.
        let trigger_style = style().height(theme_length("metrics.row"))
            .padding_x(theme_length("metrics.inset"))
            .justify_between().items_center().typography("body").font_weight(600)
            .text_color(theme_color("text_primary"));
        trigger_style = trigger_style
            .hover(style().background(theme_color("surface_hover")))
            .focus(style())
            .disabled(style().text_color(theme_color("disabled")));
        let focus_frame = row([]).with_style(ctx.component_style("focus_frame", style()
            .absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
            .border(px(2)).border_color(rgba(0x00000000))
            .group_focus(style().border_color(theme_color("focus_ring")))));
        let trigger = row([
            text(item.title),
            image_source(asset(if open { "app/icons/minus" } else { "app/icons/plus" }))
                .with_style(style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
                    .text_color(if open { theme_color("text_primary") } else { theme_color("text_muted") })),
            focus_frame,
        ])
            .with_key(`${item.key}-trigger`)
            .with_style(ctx.component_style("trigger", trigger_style))
            .disabled(item_disabled)
            .accessibility_role("button")
            .accessibility_expanded(open);
        if !item_disabled && callback != () {
            trigger = trigger.on_click_value(callback, toggled(props.expanded, item.key, mode));
        }
        let height = if "content_height" in item { item.content_height } else { () };
        let panel_content = item.content.with_style(ctx.component_style("content", style()
                .padding_x(theme_length("metrics.inset")).padding_bottom(theme_length("metrics.inset"))
                .typography("body")));
        let panel = disclosure_panel(ctx, item.key, open, height, panel_content);
        items.push(column([trigger, panel]).with_key(item.key)
            .with_style(ctx.component_style("item", style()
                .border_bottom(px(1)).border_color(theme_color("border")))));
    }
    // A Swiss list: hairlines between sections, no enclosing box.
    column(items).with_key(props.key)
        .with_style(ctx.component_style("root", style()
            .border_top(px(1)).border_color(theme_color("border"))))
}
