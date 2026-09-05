/* gpui-rhai
{
  "id": "components/dialog",
  "export": "Dialog",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Dialog presents controlled modal content through the native overlay layer.
//
// Props: key, open, title, content/actions slots, dismissal policy,
//        on_open_change.
// Events: open_change(bool). State: controlled by the caller.
// Parts: root, backdrop, panel, title, content, actions.
// Example: Dialog(#{ key: "confirm", open: true, title: "Confirm", content: text("Continue?") })

define_component(#{
    metadata: #{
        id: "components/dialog",
        "export": "Dialog",
        version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [],
        capabilities: #{},
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            actions: #{
                schema: #{ type: "array", items: #{ type: "node" }, max_items: 8 },
                required: false,
                sensitive: false,
                "default": #{ type: "array", value: [] },
            },
            dismiss_on_escape: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: true },
            },
            dismiss_on_outside: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: true },
            },
            initial_focus: #{
                schema: #{ type: "string", allowed: ["panel", "first"] },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "panel" },
            },
            on_open_change: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
            },
        },
        state: #{ fields: #{} },
        events: #{ open_change: #{ payload: #{ type: "bool" } } },
        slots: #{
            content: #{ required: true, multiple: false },
            actions: #{ required: false, multiple: true },
        },
        parts: ["root", "backdrop", "panel", "title", "content", "actions"],
    },
    render: Fn("render_Dialog")
});

fn optional_bool(props, name, fallback) {
    if name in props { props[name] } else { fallback }
}

fn Dialog(props) { render_component("components/dialog", props) }

fn forward_open_change(ctx, open) {
    ctx.emit("open_change", open);
}

fn render_Dialog(ctx, props) {
    let children = [
        text(props.title).with_style(component_style(props, "title",
            style().typography("title").font_weight(700)
                .text_color(theme_color("text_primary"))
        )),
        props.content.with_style(component_style(props, "content",
            style().typography("body"))),
    ];
    if "actions" in props && props.actions != () {
        children.push(
            row(props.actions)
                .with_style(component_style(props, "actions",
                    style().gap(px(8)).justify_end().padding_top(px(4))
                        .border_top(px(1)).border_color(theme_color("border"))))
        );
    }
    let panel = column(children)
        .with_style(component_style(props, "panel",
            style()
                .width(px(420))
                .max_width(relative(0.9))
                .padding(px(16))
                .gap(px(12))
                .radius(theme_radius("md"))
                .border(px(1))
                .border_color(theme_color("border"))
                .background(theme_color("surface"))
        ))
        .accessibility_role("dialog")
        .accessibility_label(props.title);
    let node = overlay(
        row([]),
        panel,
        #{
            id: props.key,
            kind: "dialog",
            placement: "center",
            open: props.open,
            gap: 0,
            modal: true,
            dismiss_on_escape: optional_bool(props, "dismiss_on_escape", true),
            dismiss_on_outside: optional_bool(props, "dismiss_on_outside", true),
            initial_focus: if "initial_focus" in props { props.initial_focus } else { "panel" },
        }
    ).with_key(props.key)
        .with_style(component_style(props, "root", style()));
    if "on_open_change" in props && props.on_open_change != () {
        node.on_open_change(Fn("forward_open_change"))
    } else {
        node
    }
}
