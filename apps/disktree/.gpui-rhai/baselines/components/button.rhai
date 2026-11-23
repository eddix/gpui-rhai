/* gpui-rhai
{
  "id": "components/button",
  "export": "Button",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/spinner"],
  "capabilities": {},
  "tokens": ["accent","accent_hover","border","button.danger_bg","button.ghost_fg","button.primary_bg","button.primary_fg","button.secondary_bg","control.border","control.danger_hover","control.hover","control.success_hover","control.warning_hover","danger","disabled","focus_ring","metrics.button_gap","metrics.button_h","metrics.button_icon","metrics.button_icon_pad","metrics.button_pad","metrics.button_radius","metrics.control","metrics.control_pad","metrics.icon","on_accent","on_danger","on_success","on_warning","radius.md","spacing.xs","success","surface_hover","text_primary","typography.button","typography.control","typography.label","warning"],
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

import "components/spinner" as spinner;

define_component(#{
    metadata: #{
        id: "components/button",
        "export": "Button",
        version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/spinner"],
        capabilities: #{},
        tokens: ["accent","accent_hover","border","button.danger_bg","button.ghost_fg","button.primary_bg","button.primary_fg","button.secondary_bg","control.border","control.danger_hover","control.hover","control.success_hover","control.warning_hover","danger","disabled","focus_ring","metrics.button_gap","metrics.button_h","metrics.button_icon","metrics.button_icon_pad","metrics.button_pad","metrics.button_radius","metrics.control","metrics.control_pad","metrics.icon","on_accent","on_danger","on_success","on_warning","radius.md","spacing.xs","success","surface_hover","text_primary","typography.button","typography.control","typography.label","warning"],
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
                        "primary", "secondary", "danger", "destructive", "warning",
                        "success", "ghost", "outline"
                    ]
                },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "primary" },
                doc: "Preset-owned fill: `primary` ink/reverse ink, `secondary` surface, `ghost` bare, `danger`/`destructive` soft red; warning, success and outline are derived variants.",
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
                doc: "Blocks clicks without applying disabled opacity. Dims the label and replaces an existing prefix with a native rotating indicator; never synthesizes a new prefix. Reduced or no motion holds the indicator still.",
            },
            loading_text: #{
                schema: #{ type: "string" },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "" },
                doc: "Replaces the visible label and accessible name while loading, without an invisible idle-label spacer. Empty retains the ordinary label.",
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

fn button_tone(variant) { if variant == "destructive" { "danger" } else { variant } }
fn button_fill(variant) {
    let tone = button_tone(variant);
    if tone == "primary" { theme_color("button.primary_bg") }
    else if tone == "secondary" { theme_color("button.secondary_bg") }
    else if tone == "danger" { theme_color("button.danger_bg") }
    else if tone == "warning" || tone == "success" { theme_color(`badge.${tone}_bg`) }
    else { rgba(0x00000000) }
}
fn button_foreground(variant) {
    let tone = button_tone(variant);
    if tone == "primary" { theme_color("button.primary_fg") }
    else if tone == "ghost" { theme_color("button.ghost_fg") }
    else if tone == "danger" || tone == "warning" || tone == "success" { theme_color(`badge.${tone}_fg`) }
    else { theme_color("text_primary") }
}
fn button_recipe(ctx, variant) { ctx.theme_style(`button.${button_tone(variant)}`) }
fn paint_duration(ctx) {
    let theme = ctx.theme_variant();
    if theme != () && theme.family == "Clean" { 140 } else { 160 }
}
fn icon_entry(icon, loading) {
    row([icon]).with_key("icon-entry")
        .with_style(style().items_center().justify_center().flex_shrink(false))
        .accessibility_role("presentation")
        .motion(motion_transition("opacity", 0.0, 1.0,
            #{duration_ms:520,easing:cubic_bezier(0.25,1.0,0.5,1.0)}))
        .motion(motion_transition("foreground_blur", 2.0, 0.0,
            #{duration_ms:520,easing:cubic_bezier(0.25,1.0,0.5,1.0)}))
        .motion(motion_transition("scale_x", 0.7, 1.0,
            #{duration_ms:520,easing:cubic_bezier(0.25,1.0,0.5,1.0)}))
        .motion(motion_transition("scale_y", 0.7, 1.0,
            #{duration_ms:520,easing:cubic_bezier(0.25,1.0,0.5,1.0)}))
        .motion_on_change(if loading { "spinner" } else { "icon" })
}

fn run_action(action, ctx, payload) {
    ctx.dispatch_action(action, ());
}

fn Button(props) { render_component("components/button", props) }

fn loading_spinner(ctx) {
    // Loading actions retain their own 24-unit geometry and 900ms turn;
    // standalone indicators share path construction, not these metrics.
    let scene = spinner::ring_scene(24.0,8.5,2.0,0.2);
    let mark = canvas(scene)
        .with_key("loading-spinner")
        .with_style(ctx.component_style("spinner", style()
            .width(theme_length("metrics.button_icon")).height(theme_length("metrics.button_icon"))
            .flex_shrink(false)))
        .motion(motion_transition("rotate", 0.0, 360.0,
            #{ duration_ms: 900, easing: "linear", iterations: "infinite", intent: "essential" }));
    row([mark]).with_style(style().items_center().justify_center().flex_shrink(false))
        .accessibility_role("presentation")
}

fn render_Button(ctx, props) {
    let variant = if "variant" in props { props.variant } else { "primary" };
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

    let base = style().flex_row().items_center().justify_center()
        .min_height(theme_length("metrics.button_h"))
        .padding_x(theme_length("metrics.button_pad"))
        .gap(theme_length("metrics.button_gap"))
        .radius(theme_length("metrics.button_radius"))
        .min_width(px(0)).clip()
        .border(px(1)).border_color(rgba(0x00000000))
        .typography("button").font_weight(500).focus(style());
    let state_style = button_recipe(ctx, variant)
        .disabled(style().opacity(if disabled { 0.4 } else { 1.0 }));
    if !hoverable || loading { state_style = state_style.without_hover().without_active(); }
    if "prefix" in props && props.prefix != () {
        state_style = state_style.padding_start(theme_length("metrics.button_icon_pad"));
    }
    if "suffix" in props && props.suffix != () {
        state_style = state_style.padding_end(theme_length("metrics.button_icon_pad"));
    }

    let children = [];
    let has_prefix = "prefix" in props && props.prefix != ();
    if has_prefix {
        let prefix = if loading {
            loading_spinner(ctx)
        } else { props.prefix };
        children.push(icon_entry(prefix.with_style(ctx.component_style("prefix", style())), loading));
    }
    let displayed_text = if show_loading_text { loading_text } else { props.text };
    let loading_opacity = if paint_duration(ctx) == 140 { 0.50 } else { 0.55 };
    let label_text = text(displayed_text)
        .with_key("label")
        .with_style(ctx.component_style("label", style().opacity(if loading { loading_opacity } else { 1.0 })));
    // Entry alpha belongs to a structural parent: it multiplies the label's
    // controlled/Host opacity rather than replacing that public part style.
    let label = row([label_text]).with_key("label-entry")
        .with_style(style().min_width(px(0)))
        .accessibility_role("presentation")
        .motion(motion_transition("opacity", 0.0, 1.0,
            #{duration_ms:480,easing:cubic_bezier(0.25,1.0,0.5,1.0)}))
        .motion(motion_transition("translate_y_fraction", 0.25, 0.0,
            #{duration_ms:480,easing:cubic_bezier(0.25,1.0,0.5,1.0)}))
        .motion(motion_transition("foreground_blur", 3.0, 0.0,
            #{duration_ms:480,easing:cubic_bezier(0.25,1.0,0.5,1.0)}))
        .motion_on_change(displayed_text);
    if shortcut == () {
        children.push(label);
    } else {
        // The inline key legend of Menu and Command (the label voice), in the button's own
        // color, on the label's baseline as words in one line are set.
        let legend = text(shortcut.label).with_style(ctx.component_style("shortcut",
            style().typography("label").opacity(1.0)));
        children.push(row([label, legend]).with_style(style().items_baseline().gap(theme_spacing("xs"))));
    }
    if "suffix" in props && props.suffix != () {
        children.push(props.suffix.with_style(ctx.component_style("suffix",
            style())));
    }
    let button = row(children)
        .with_style(ctx.component_style("root", base, state_style))
        .paint_transition(paint_duration(ctx), cubic_bezier(0.25,0.1,0.25,1.0))
        .intrinsic_width_motion(displayed_text, 480, cubic_bezier(0.25,1.0,0.5,1.0))
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
