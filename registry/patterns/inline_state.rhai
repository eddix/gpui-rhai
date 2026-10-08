/* gpui-rhai
{
  "id": "patterns/inline_state",
  "export": "InlineState",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/spinner"],
  "capabilities": {},
  "tokens": [
    "metrics.row", "metrics.inset", "space.related", "spacing.xxs", "spacing.sm",
    "surface_raised", "text_primary", "text_muted", "danger", "warning", "typography.body",
    "typography.subtitle", "typography.caption", "typography.code"
  ],
  "environment": ["density"]
}
*/
// InlineState shows loading, empty, error, stale and refreshing feedback where the
// content would appear.
//
// Props: key, state (loading|empty|error|stale|refreshing) and title are required;
//        description, detail (raw error text, selectable; error, stale and refreshing),
//        actions. Long text wraps in a narrow container.
// Events/state: none. Parts: root, title, description, detail, actions, lamp.
// Example: InlineState(#{ key: "hosts", state: "empty", title: "No hosts yet",
//              description: "Register a host to see it here.", actions: [add_button] })
//
// Loading and empty center in the space; errors read top-down from the content edge with
// the raw error selectable; stale and refreshing are one quiet line above kept data. The
// title is required because only the caller knows what is loading or why it is empty.

import "components/spinner" as spinner;

define_component(#{
    metadata: #{
        id: "patterns/inline_state", "export": "InlineState", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/spinner"], capabilities: #{},
        tokens: [
            "metrics.row", "metrics.inset", "space.related", "spacing.xxs", "spacing.sm",
            "surface_raised", "text_primary", "text_muted", "danger", "warning",
            "typography.body", "typography.subtitle", "typography.caption", "typography.code"
        ],
        environment: ["density"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            state: #{ schema: #{ type: "string", allowed: ["loading", "empty", "error", "stale", "refreshing"] },
                required: true, sensitive: false },
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            detail: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            actions: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ actions: #{ required: false, multiple: true } },
        parts: ["root", "title", "description", "detail", "actions", "lamp"],
    },
    render: Fn("render_InlineState")
});

fn InlineState(props) { render_component("patterns/inline_state", props) }
fn optional(props, name) { if name in props && props[name] != () { props[name] } else { () } }
// Raw error text: selectable, so it can be copied.
fn detail_text(ctx, detail) {
    text(detail).selectable(true).with_style(ctx.component_style("detail", style()
        .min_width(px(0)).typography("code").text_color(theme_color("text_muted"))))
}
fn lamp(ctx, color) {
    // The 6px status lamp is a structural constant.
    row([]).with_style(ctx.component_style("lamp", style().width(px(6)).height(px(6)).background(color)))
}

fn render_InlineState(ctx, props) {
    let state = props.state;
    let title = props.title;
    let description = optional(props, "description");
    let actions = if "actions" in props { props.actions } else { [] };
    if state == "stale" || state == "refreshing" {
        let line = [];
        line.push(if state == "stale" { lamp(ctx, theme_color("warning")) }
            else { spinner::Spinner(#{ key: `${props.key}-spinner`, label: title, size: "xs" }) });
        // Text items shrink below their line, so a narrow container wraps them.
        line.push(text(title).with_style(ctx.component_style("title", style()
            .min_width(px(0)).text_color(theme_color("text_primary")))));
        if description != () {
            line.push(text(description).with_style(ctx.component_style("description", style()
                .min_width(px(0)).text_color(theme_color("text_muted")))));
        }
        for action in actions { line.push(action); }
        let line_style = style().flex_row().items_center().gap(theme_spacing("sm"))
            .min_height(theme_length("metrics.row")).min_width(px(0));
        let root_style = style().padding_x(theme_length("metrics.inset")).min_width(px(0))
            .background(if state == "stale" { theme_color("surface_raised") } else { rgba(0x00000000) })
            .typography("body");
        let detail = optional(props, "detail");
        if detail == () {
            return row(line).with_key(props.key)
                .with_style(ctx.component_style("root", root_style.flex_row().items_center()
                    .gap(theme_spacing("sm")).min_height(theme_length("metrics.row"))))
                .accessibility_role("status").accessibility_label(title);
        }
        // The raw error under the line, from the marker's edge, as in the error state.
        return column([
            row(line).with_style(line_style),
            detail_text(ctx, detail).with_style(style().padding_bottom(theme_spacing("sm"))),
        ]).with_key(props.key)
            .with_style(ctx.component_style("root", root_style.flex_col()))
            .accessibility_role("status").accessibility_label(title);
    }
    let children = [];
    if state == "loading" {
        children.push(row([
            spinner::Spinner(#{ key: `${props.key}-spinner`, label: title, size: "sm" }),
            text(title).with_style(ctx.component_style("title", style().min_width(px(0))
                .text_color(theme_color("text_muted")))),
        ]).with_style(style().flex_row().items_center().gap(theme_spacing("sm")).min_width(px(0))
            .max_width(relative(1.0))));
    } else if state == "error" {
        children.push(row([
            lamp(ctx, theme_color("danger")),
            text(title).with_style(ctx.component_style("title", style().min_width(px(0))
                .font_weight(600).text_color(theme_color("text_primary")))),
        ]).with_style(style().flex_row().items_center().gap(theme_spacing("sm")).min_width(px(0))
            .max_width(relative(1.0))));
    } else {
        children.push(text(title).with_style(ctx.component_style("title", style()
            .typography("subtitle").text_color(theme_color("text_primary")))));
    }
    if description != () && state != "loading" {
        children.push(text(description).with_style(ctx.component_style("description", style()
            .text_color(theme_color("text_muted")))));
    }
    let detail = optional(props, "detail");
    if detail != () && state == "error" { children.push(detail_text(ctx, detail)); }
    if actions.len > 0 && state != "loading" {
        children.push(row(actions).with_style(ctx.component_style("actions", style()
            .flex_row().items_center().gap(theme_length("space.related")))));
    }
    // Loading and empty center in the space they replace; errors read from the content edge.
    // Children of the items_start / centered column keep their line width unless capped, so
    // the marker rows above carry max_width(100%).
    let centered = state != "error";
    let root = style().flex_col().gap(theme_spacing("sm"))
        .min_height(theme_length("metrics.row") * 4).padding(theme_length("metrics.inset"))
        .typography("body");
    root = if centered { root.flex_grow().items_center().justify_center() } else { root.items_start() };
    column(children).with_key(props.key).with_style(ctx.component_style("root", root))
        .accessibility_role(if state == "error" { "alert" } else { "status" })
        .accessibility_label(title)
}
