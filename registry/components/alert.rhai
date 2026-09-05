/* gpui-rhai
{
  "id": "components/alert",
  "export": "Alert",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/check.svg", "icons/close.svg", "icons/info.svg", "icons/warning.svg"]
}
*/
// Persistent inline feedback with semantic variants and composable actions.
// Props: title, optional description/icon/content/actions, and variant.
// Events/state: stateless and emits no events.
// Parts: root, icon, body, title, description, content, actions.
// Example: Alert(#{ title: "Saved", description: "Changes are live.", variant: "success" })

define_component(#{
    metadata: #{ id: "components/alert", "export": "Alert", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{},
        assets: ["icons/check.svg", "icons/close.svg", "icons/info.svg", "icons/warning.svg"] },
    schema: #{
        props: #{
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            icon: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            content: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            actions: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            variant: #{ schema: #{ type: "string", allowed: ["info", "success", "warning", "danger"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "info" } },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ icon: #{ required: false, multiple: false }, content: #{ required: false, multiple: false },
            actions: #{ required: false, multiple: true } },
        parts: ["root", "icon", "body", "title", "description", "content", "actions"],
    },
    render: Fn("render_Alert")
});

fn alert_color(variant) {
    if variant == "success" { theme_color("success") }
    else if variant == "warning" { theme_color("warning") }
    else if variant == "danger" { theme_color("danger") }
    else { theme_color("accent") }
}

fn alert_icon(variant) {
    if variant == "success" { "app/icons/check" }
    else if variant == "warning" { "app/icons/warning" }
    else if variant == "danger" { "app/icons/close" }
    else { "app/icons/info" }
}

fn alert_foreground(variant) {
    if variant == "success" { theme_color("on_success") }
    else if variant == "warning" { theme_color("on_warning") }
    else if variant == "danger" { theme_color("on_danger") }
    else { theme_color("on_accent") }
}

fn alert_role(variant) {
    if variant == "danger" || variant == "warning" { "alert" } else { "status" }
}

fn Alert(props) { render_component("components/alert", props) }
fn render_Alert(ctx, props) {
    let variant = if "variant" in props { props.variant } else { "info" };
    let body = [text(props.title).with_style(component_style(props, "title", style()
        .typography("body").font_weight(700)
        .text_color(theme_color("text_primary"))))];
    if "description" in props && props.description != () {
        body.push(text(props.description).with_style(component_style(props, "description", style()
            .typography("body_small").text_color(theme_color("text_muted")))));
    }
    if "content" in props && props.content != () {
        body.push(props.content.with_style(component_style(props, "content", style())));
    }
    let children = [];
    if "icon" in props && props.icon != () {
        children.push(props.icon.with_style(component_style(props, "icon", style()
            .width(px(18)).height(px(18)).items_center().justify_center()
            .text_color(alert_color(variant)))));
    } else {
        children.push(row([image_source(asset(alert_icon(variant))).with_style(style()
                .width(px(12)).height(px(12)).text_color(alert_foreground(variant)))])
            .with_style(component_style(props, "icon", style()
                .width(px(18)).height(px(18)).items_center().justify_center()
                .radius(px(9)).background(alert_color(variant))
                .text_color(alert_foreground(variant)))));
    }
    children.push(column(body).with_style(component_style(props, "body", style().gap(px(2)).flex_grow())));
    if "actions" in props && props.actions.len > 0 {
        children.push(row(props.actions).with_style(component_style(props, "actions", style()
            .gap(px(6)).items_center())));
    }
    row(children).with_style(component_style(props, "root", style()
        .padding(px(10)).gap(px(9)).items_center().radius(theme_radius("md"))
        .border(px(1)).border_start(px(3)).border_color(alert_color(variant))
        .background(theme_color("surface"))))
        .accessibility_role(alert_role(variant))
        .accessibility_label(props.title)
}
