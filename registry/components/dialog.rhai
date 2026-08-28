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

export_component(#{
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
});

fn optional_bool(props, name, fallback) {
    if name in props { props[name] } else { fallback }
}

fn Dialog(props) {
    component_render("components/dialog", props, Fn("render_Dialog"))
}
fn render_Dialog(ctx, props) {
    let children = [
        text(props.title).with_style(component_style(props, "title",
            style().font_size(rem(1.125)).text_color(theme_color("text_primary"))
        )),
        props.content.with_style(component_style(props, "content", style())),
    ];
    if "actions" in props && props.actions != () {
        children.push(
            row(props.actions)
                .with_style(component_style(props, "actions",
                    style().gap(px(8)).justify_end().padding_top(px(8))))
        );
    }
    let panel = column(children)
        .with_style(component_style(props, "panel",
            style()
                .width(px(420))
                .max_width(relative(0.9))
                .padding(px(20))
                .gap(px(16))
                .radius(px(10))
                .border(px(1))
                .border_color(theme_color("border"))
                .background(theme_color("surface_raised"))
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
        }
    ).with_key(props.key)
        .with_style(component_style(props, "root", style()));
    if "on_open_change" in props && props.on_open_change != () {
        node.on_open_change(props.on_open_change)
    } else {
        node
    }
}
