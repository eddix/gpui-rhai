/* gpui-rhai
{
  "id": "components/dialog",
  "export": "Dialog",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_raised", "border", "text_primary", "focus_ring", "spacing.xl", "space.group",
    "space.related", "radius.lg", "typography.title", "typography.body"
  ],
  "environment": ["density", "corners"]
}
*/
// Dialog presents controlled modal content through the native overlay layer.
//
// Props: key, open, title (also the accessible name), title_visible (default true),
//        content/actions slots, dismissal policy, on_open_change.
// Events: open_change(bool). State: controlled by the caller.
// Parts: root, backdrop, panel, title, content, actions, focus_frame.
// Example: Dialog(#{ key: "confirm", open: true, title: "Confirm", content: text("Continue?") })

define_component(#{
    metadata: #{
        id: "components/dialog",
        "export": "Dialog",
        version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [],
        capabilities: #{},
        tokens: [
            "surface_raised", "border", "text_primary", "focus_ring", "spacing.xl",
            "space.group", "space.related", "radius.lg", "typography.title", "typography.body"
        ],
        environment: ["density", "corners"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            title_visible: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
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
        parts: ["root", "backdrop", "panel", "title", "content", "actions", "focus_frame"],
    },
    render: Fn("render_Dialog")
});

fn optional_bool(props, name, fallback) {
    if name in props { props[name] } else { fallback }
}

fn Dialog(props) { render_component("components/dialog", props) }

// The 2px focus frame, drawn over the panel's edge while the panel itself holds focus: the
// panel's hairline (painted over its children) turns the focus color and this frame adds
// the inner pixel. A control inside shows its own frame instead.
fn panel_focus_frame(ctx) {
    row([]).with_style(ctx.component_style("focus_frame", style().absolute()
        .left(offset_px(-1)).right(offset_px(-1)).top(offset_px(-1)).bottom(offset_px(-1))
        .radius(theme_radius("lg")).border(px(2)).border_color(rgba(0x00000000))
        .group_focus(style().border_color(theme_color("focus_ring")))))
}

fn forward_open_change(ctx, open) {
    ctx.emit("open_change", open);
}

fn render_Dialog(ctx, props) {
    let children = [];
    // A hidden title still names the dialog (accessibility_label below).
    if optional_bool(props, "title_visible", true) {
        children.push(text(props.title).with_style(ctx.component_style("title",
            style().typography("title")
                .text_color(theme_color("text_primary"))
        )));
    }
    children.push(props.content.with_style(ctx.component_style("content",
        style().typography("body"))));
    if "actions" in props && props.actions != () {
        children.push(
            row(props.actions)
                .with_style(ctx.component_style("actions",
                style().gap(theme_length("space.related")).justify_end()))
        );
    }
    children.push(panel_focus_frame(ctx));
    let panel = column(children)
        .with_style(ctx.component_style("panel",
            // Overlays are raised blocks with a hairline; no shadow.
            style()
                .relative()
                .width(px(420))
                // The overlay caps the panel at 90% of the window; it may fill that.
                .max_width(relative(1.0))
                .padding(theme_spacing("xl"))
                .gap(theme_length("space.group"))
                .radius(theme_radius("lg"))
                .border(px(1))
                .border_color(theme_color("border"))
                .group_focus(style().border_color(theme_color("focus_ring")))
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
            // The anchor is empty: the dialog opens from application state, never
            // by activating its trigger, so the trigger takes no focus.
            activate_on_trigger: false,
            dismiss_on_escape: optional_bool(props, "dismiss_on_escape", true),
            dismiss_on_outside: optional_bool(props, "dismiss_on_outside", true),
            initial_focus: if "initial_focus" in props { props.initial_focus } else { "panel" },
        }
    ).with_key(props.key)
        .with_style(ctx.component_style("root", style()));
    if "on_open_change" in props && props.on_open_change != () {
        node.on_open_change(Fn("forward_open_change"))
    } else {
        node
    }
}
