/* gpui-rhai
{
  "id": "components/alert_dialog",
  "export": "AlertDialog",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/button", "components/dialog"],
  "capabilities": {},
  "tokens": ["text_muted", "space.related", "typography.body"],
  "environment": ["density"]
}
*/
// Controlled confirmation dialog with explicit cancel and confirm semantics.
// State: caller owns open state; the component retains no application value.
// Example: AlertDialog(#{ key: "delete", open: true, title: "Delete item?", on_confirm: Fn("remove") })

import "components/button" as button;
import "components/dialog" as dialog;

define_component(#{
    metadata: #{ id: "components/alert_dialog", "export": "AlertDialog", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/button", "components/dialog"], capabilities: #{},
        tokens: ["text_muted", "space.related", "typography.body"],
        environment: ["density"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Identity of the dialog and the id of its overlay; keep it unique among open overlays." },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false,
                doc: "Whether the dialog is open; the caller stores the `open_change` payload and passes it back." },
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Heading of the dialog and its accessible name." },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Muted text under the title." },
            content: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Extra content after the description." },
            cancel_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Cancel" },
                doc: "Text of the cancel button." },
            confirm_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Continue" },
                doc: "Text of the confirm button." },
            confirm_variant: #{ schema: #{ type: "string", allowed: ["primary", "danger", "warning"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "danger" },
                doc: "Button variant of the confirm button." },
            on_confirm: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `()` when the confirm button is pressed; `open_change` then asks to close." },
            on_cancel: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `()` when the cancel button is pressed; Escape and backdrop presses close without it." },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `false` when either button is pressed or Escape or a backdrop press dismisses the dialog." },
        },
        state: #{ fields: #{} },
        events: #{ confirm: #{ payload: #{ type: "null" },
                doc: "Emitted when the confirm button is pressed, before `open_change`; the payload is `()`." },
            cancel: #{ payload: #{ type: "null" },
                doc: "Emitted when the cancel button is pressed, before `open_change`; the payload is `()`." },
            open_change: #{ payload: #{ type: "bool" },
                doc: "Emitted when a button is pressed or the dialog is dismissed; the payload is the requested open state." } },
        slots: #{ content: #{ required: false, multiple: false, doc: "Extra content after the description." } },
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
        content: column(body).with_style(style().gap(theme_length("space.related"))),
        actions: [cancel, confirm], initial_focus: "first", on_open_change: Fn("open_action") })
        .with_style(ctx.component_style("root", style()))
}
