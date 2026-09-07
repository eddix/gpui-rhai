/* gpui-rhai
{
  "id": "components/alert_dialog",
  "export": "AlertDialog",
  "version": "0.1.1",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": ["components/button", "components/dialog"],
  "capabilities": {}
}
*/
// Controlled confirmation dialog with explicit cancel and confirm semantics.
// Props: key, open, title, description/content, labels, confirm variant, and callbacks.
// Events: confirm(null), cancel(null), and open_change(bool).
// State: caller owns open state; the component retains no application value.
// Parts: root, description, content, actions, cancel, confirm.
// Example: AlertDialog(#{ key: "delete", open: true, title: "Delete item?", on_confirm: Fn("remove") })

import "components/button" as button;
import "components/dialog" as dialog;

define_component(#{
    metadata: #{ id: "components/alert_dialog", "export": "AlertDialog", version: "0.1.1",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: ["components/button", "components/dialog"], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            content: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            cancel_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Cancel" } },
            confirm_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Continue" } },
            confirm_variant: #{ schema: #{ type: "string", allowed: ["primary", "danger", "warning"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "danger" } },
            on_confirm: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_cancel: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{ confirm: #{ payload: #{ type: "null" } }, cancel: #{ payload: #{ type: "null" } },
            open_change: #{ payload: #{ type: "bool" } } },
        slots: #{ content: #{ required: false, multiple: false } },
        parts: ["root", "description", "content", "actions", "cancel", "confirm"],
    },
    render: Fn("render_AlertDialog")
});

fn AlertDialog(props) { render_component("components/alert_dialog", props) }
fn confirm_action(ctx, payload) { ctx.emit("confirm", ()); ctx.emit("open_change", false); }
fn cancel_action(ctx, payload) { ctx.emit("cancel", ()); ctx.emit("open_change", false); }
fn open_action(ctx, open) { ctx.emit("open_change", open); }

fn render_AlertDialog(ctx, props) {
    let body = [];
    if "description" in props && props.description != () {
        body.push(text(props.description).with_style(ctx.component_style("description", style()
            .typography("body").text_color(theme_color("text_muted")))));
    }
    if "content" in props && props.content != () {
        body.push(props.content.with_style(ctx.component_style("content", style())));
    }
    let cancel = button::Button(#{ text: if "cancel_label" in props { props.cancel_label } else { "Cancel" },
        variant: "secondary", on_click: Fn("cancel_action") })
        .with_style(ctx.component_style("cancel", style()));
    let confirm = button::Button(#{ text: if "confirm_label" in props { props.confirm_label } else { "Continue" },
        variant: if "confirm_variant" in props { props.confirm_variant } else { "danger" },
        on_click: Fn("confirm_action") })
        .with_style(ctx.component_style("confirm", style()));
    dialog::Dialog(#{ key: props.key, open: props.open, title: props.title,
        content: column(body).with_style(style().gap(px(8))),
        actions: [cancel, confirm], initial_focus: "first", on_open_change: Fn("open_action") })
        .with_style(ctx.component_style("root", style()))
}
