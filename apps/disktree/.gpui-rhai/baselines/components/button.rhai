/* gpui-rhai
{
  "id": "components/button",
  "export": "Button",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_hover", "text_primary", "accent", "accent_hover", "on_accent", "danger",
    "on_danger", "warning", "on_warning", "success", "on_success", "border",
    "focus_ring", "disabled", "control.hover", "control.border", "control.danger_hover", "control.warning_hover", "control.success_hover", "metrics.control", "metrics.control_pad", "metrics.icon",
    "spacing.xs", "radius.md", "typography.control", "typography.label"
  ],
  "environment": ["size", "density"]
}
*/
// Button presents a desktop action: a large block with a centered label.
//
// State: stateless and controlled by caller props.
// Example: Button(#{ text: "Deploy", variant: "primary", action: "deploy.start" })
//
// `size` is inherited from the environment unless set. `action` dispatches a registered
// action on click and derives the enabled state and shortcut legend from it.

define_component(#{
    metadata: #{
        id: "components/button",
        "export": "Button",
        version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [],
        capabilities: #{},
        tokens: [
            "surface_hover", "text_primary", "accent", "accent_hover", "on_accent", "danger",
            "on_danger", "warning", "on_warning", "success", "on_success", "border",
            "focus_ring", "disabled", "control.hover", "control.border", "control.danger_hover", "control.warning_hover", "control.success_hover", "metrics.control", "metrics.control_pad", "metrics.icon",
            "spacing.xs", "radius.md", "typography.control", "typography.label"
        ],
        environment: ["size", "density"],
    },
    schema: #{
        props: #{
            text: #{
                schema: #{ type: "string" },
                required: true,
                sensitive: false,
                doc: "Label of the button; also its accessible name, except while `loading` shows `loading_text`.",
            },
            prefix: #{
                schema: #{ type: "optional", value: #{ type: "node" } },
                required: false,
                sensitive: false,
                doc: "Node before the label, such as an icon, drawn in the button's text color.",
            },
            suffix: #{
                schema: #{ type: "optional", value: #{ type: "node" } },
                required: false,
                sensitive: false,
                doc: "Node after the label, such as a disclosure chevron, drawn in the button's text color.",
            },
            variant: #{
                schema: #{
                    type: "string",
                    allowed: [
                        "primary", "secondary", "danger", "warning",
                        "success", "ghost", "outline"
                    ]
                },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "secondary" },
                doc: "Fill: `primary` accent, `danger`/`warning`/`success` status colors, `secondary` neutral, `ghost` bare, `outline` framed.",
            },
            size: #{
                schema: #{
                    type: "optional",
                    value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] }
                },
                required: false,
                sensitive: false,
                doc: "Control size of the button; unset, it inherits the `size` environment.",
            },
            disabled: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Blocks clicks and greys the button but keeps the variant's silhouette; a disabled `action` disables it too.",
            },
            loading: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Blocks clicks and shows the disabled look while work runs, with a turning spinner in the label's color: in place of `prefix`, else before the label, which then widens the button. Reduced or no motion holds the spinner still.",
            },
            loading_text: #{
                schema: #{ type: "string" },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "" },
                doc: "Replaces the label and the accessible name while `loading`; the button keeps the width of `text` unless this is wider. Empty leaves the label as it is.",
            },
            hoverable: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Changes the fill on hover; `false` keeps it still.",
            },
            on_click: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
                doc: "Called when the button is pressed; not while disabled or loading, and not when `action` is set.",
            },
            action: #{
                schema: #{ type: "optional", value: #{ type: "string" } },
                required: false,
                sensitive: false,
                doc: "Registered action dispatched on click, in place of `on_click`; it also sets the enabled state and the shortcut legend.",
            },
            shortcut: #{
                schema: #{ type: "optional", value: #{ type: "string" } },
                required: false,
                sensitive: false,
                doc: "Key legend after the label, such as `cmd-s`, formatted for the platform; it binds no key and overrides the `action` legend.",
            },
        },
        state: #{ fields: #{} },
        events: #{ click: #{ payload: #{ type: "null" },
            doc: "Emitted when the button is pressed while enabled, not loading and without an `action`." } },
        slots: #{
            prefix: #{ required: false, multiple: false, doc: "Node before the label." },
            suffix: #{ required: false, multiple: false, doc: "Node after the label." },
        },
        parts: ["root", "label", "prefix", "suffix", "shortcut", "spinner"],
    },
    render: Fn("render_Button")
});

fn button_fill(variant) {
    if variant == "primary" { theme_color("accent") }
    else if variant == "danger" || variant == "warning" || variant == "success" {
        theme_color(variant)
    } else if variant == "secondary" { theme_color("surface_hover") }
    else { rgba(0x00000000) }
}

// The reserved 2px border takes the fill (invisible) except for outline,
// whose functional frame uses control.border, not a decorative divider.
fn button_frame(variant) {
    if variant == "outline" { theme_color("control.border") }
    else { button_fill(variant) }
}

fn button_hover(variant) {
    if variant == "primary" { theme_color("accent_hover") }
    else if variant == "secondary" { theme_color("control.hover") }
    else if variant == "ghost" || variant == "outline" { theme_color("surface_hover") }
    else { theme_color(`control.${variant}_hover`) }
}

fn button_foreground(variant) {
    if variant == "primary" { theme_color("on_accent") }
    else if variant == "danger" || variant == "warning" || variant == "success" {
        theme_color(`on_${variant}`)
    } else { theme_color("text_primary") }
}

fn run_action(action, ctx, payload) {
    ctx.dispatch_action(action, ());
}

fn Button(props) { render_component("components/button", props) }

// The loading indicator: Spinner's five strokes in the disabled label's color, turning on
// the runtime clock (reduced or no motion holds it still), centered in an icon-sized slot
// so it takes a prefix icon's place exactly. Drawn here rather than through
// components/spinner, so a Button needs no other component. The strokes span 14 logical
// pixels, 12 at the xs and sm sizes; an inherited size cannot be read, so it takes 14.
fn loading_spinner(ctx, size) {
    let diameter = if size == "xs" || size == "sm" { 12.0 } else { 14.0 };
    let scale = diameter / 16.0;
    let width = 1.7 * scale;
    let color = theme_color("disabled");
    let scene = canvas_scene([
        canvas_line("a", 8.0 * scale, 1.0 * scale, 8.0 * scale, 4.0 * scale, width, color),
        canvas_line("b", 12.95 * scale, 3.05 * scale, 10.8 * scale, 5.2 * scale, width, color),
        canvas_line("c", 15.0 * scale, 8.0 * scale, 12.0 * scale, 8.0 * scale, width, color),
        canvas_line("d", 12.95 * scale, 12.95 * scale, 10.8 * scale, 10.8 * scale, width, color),
        canvas_line("e", 8.0 * scale, 15.0 * scale, 8.0 * scale, 12.0 * scale, width, color)
    ]);
    let strokes = canvas(scene).with_key("loading-spinner")
        .with_style(style().width(px(diameter)).height(px(diameter)).flex_shrink(false))
        .motion(motion_transition("rotate", 0.0, 360.0,
            #{ duration_ms: ctx.motion_duration("ambient"), easing: "linear",
                iterations: "infinite", intent: "essential" }));
    row([strokes]).with_style(ctx.component_style("spinner",
            style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
                .items_center().justify_center().flex_shrink(false)))
        .accessibility_role("presentation")
}

fn render_Button(ctx, props) {
    let variant = if "variant" in props { props.variant } else { "secondary" };
    let action = if "action" in props && props.action != () { props.action } else { () };
    let action_enabled = action == () || ctx.action_enabled(action);
    let disabled = (if "disabled" in props { props.disabled } else { false }) || !action_enabled;
    let loading = if "loading" in props { props.loading } else { false };
    let loading_text = if "loading_text" in props { props.loading_text } else { "" };
    let hoverable = if "hoverable" in props { props.hoverable } else { true };
    let has_click = "on_click" in props && props.on_click != ();
    let interactive = !disabled && !loading && (has_click || action != ());
    let show_loading_text = loading && loading_text != "";
    // A caller-written shortcut gets the same platform legend as a bound action.
    let shortcut = if "shortcut" in props && props.shortcut != () {
        key_shortcut(props.shortcut)
    } else if action != () {
        ctx.action_shortcut(action)
    } else { () };

    let foreground = button_foreground(variant);
    let base = style()
        .flex_row()
        .items_center()
        .justify_center()
        .min_height(theme_length("metrics.control"))
        .padding_x(theme_length("metrics.control_pad"))
        .gap(theme_spacing("xs"))
        .radius(theme_radius("md"));
    base = base
        .border(px(2))
        .border_color(button_frame(variant))
        .background(button_fill(variant))
        .text_color(foreground)
        .typography("control")
        .font_weight(600);
    // Disabled keeps the variant's silhouette: blocks turn into a neutral block, an
    // outline keeps its frame, a ghost stays bare; only the color goes.
    let bare = variant == "ghost" || variant == "outline";
    let disabled_fill = if bare { rgba(0x00000000) } else { theme_color("surface_hover") };
    let disabled_frame = if variant == "outline" { theme_color("border") } else { disabled_fill };
    base = base
        .focus(style().border_color(theme_color("focus_ring")))
        .active(style().opacity(0.86))
        .disabled(style()
            .background(disabled_fill)
            .border_color(disabled_frame)
            .text_color(theme_color("disabled")));
    if hoverable && !disabled && !loading {
        base = base.hover(style().background(button_hover(variant)));
    }

    let children = [];
    if loading {
        children.push(loading_spinner(ctx, if "size" in props { props.size } else { () }));
    } else if "prefix" in props && props.prefix != () {
        children.push(props.prefix.with_style(ctx.component_style("prefix",
            style().text_color(foreground).disabled(style().text_color(theme_color("disabled"))))));
    }
    let label = if show_loading_text {
        // The idle label stays laid out at zero height and unpainted, so the
        // label box is as wide as the wider of the two texts.
        column([
            text(loading_text),
            text(props.text).with_style(style().height(px(0)).invisible())
                .accessibility_role("presentation")
        ]).with_style(ctx.component_style("label", style().items_center()))
    } else {
        text(props.text).with_style(ctx.component_style("label", style()))
    };
    if shortcut == () {
        children.push(label);
    } else {
        // The inline key legend of Menu and Command (the label voice), in the button's own
        // color, on the label's baseline as words in one line are set.
        let legend = text(shortcut.label).with_style(ctx.component_style("shortcut",
            style().typography("label").opacity(0.7)));
        children.push(row([label, legend]).with_style(style().items_baseline().gap(theme_spacing("xs"))));
    }
    if "suffix" in props && props.suffix != () {
        children.push(props.suffix.with_style(ctx.component_style("suffix",
            style().text_color(foreground).disabled(style().text_color(theme_color("disabled"))))));
    }
    let button = row(children)
        .with_style(ctx.component_style("root", base))
        .disabled(disabled || loading)
        .accessibility_role("button")
        .accessibility_label(if show_loading_text { loading_text } else { props.text });
    if "size" in props && props.size != () {
        button = button.env(#{ size: props.size });
    }
    if shortcut != () {
        button = button.accessibility_key_shortcuts(shortcut.keystrokes);
    }
    if !interactive {
        button
    } else if action != () {
        button.on_click(Fn("run_action").curry(action))
    } else {
        button.on_click(props.on_click)
    }
}
