/* gpui-rhai
{
  "id": "components/badge",
  "export": "Badge",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["badge.neutral_bg","badge.neutral_fg","badge.neutral_strong_bg","badge.neutral_strong_fg","badge.info_bg","badge.info_fg","badge.info_strong_bg","badge.info_strong_fg","badge.success_bg","badge.success_fg","badge.success_strong_bg","badge.success_strong_fg","badge.warning_bg","badge.warning_fg","badge.warning_strong_bg","badge.warning_strong_fg","badge.danger_bg","badge.danger_fg","badge.danger_strong_bg","badge.danger_strong_fg","badge.purple_bg","badge.purple_fg","badge.purple_strong_bg","badge.purple_strong_fg","metrics.badge_lead","metrics.marker","metrics.marker_small","radius.pill","spacing.xs","text.subtle"],
  "environment": []
}
*/
// Badge presents a soft status surface, optional dot/icon, or a numeric count.
//
// Example: Badge(#{ text: "Ready", variant: "success" })
//
// `emphasis: "strong"` turns the badge into a solid status block; reserve it for
// states that require action. Markers keep their size in both densities.

define_component(#{
    metadata: #{
        id: "components/badge", "export": "Badge", version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: ["badge.neutral_bg","badge.neutral_fg","badge.neutral_strong_bg","badge.neutral_strong_fg","badge.info_bg","badge.info_fg","badge.info_strong_bg","badge.info_strong_fg","badge.success_bg","badge.success_fg","badge.success_strong_bg","badge.success_strong_fg","badge.warning_bg","badge.warning_fg","badge.warning_strong_bg","badge.warning_strong_fg","badge.danger_bg","badge.danger_fg","badge.danger_strong_bg","badge.danger_strong_fg","badge.purple_bg","badge.purple_fg","badge.purple_strong_bg","badge.purple_strong_fg","metrics.badge_lead","metrics.marker","metrics.marker_small","radius.pill","spacing.xs","text.subtle"],
        environment: [],
    },
    schema: #{
        props: #{
            text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{type:"string",value:""},
                doc: "The status label; also the accessible name." },
            variant: #{
                schema: #{ type: "string", allowed: ["neutral", "info", "accent", "success", "warning", "danger", "purple"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "neutral" },
                doc: "Paired foreground/background tone; accent is an alias for info.",
            },
            icon: #{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,
                doc:"An icon before the label; dot/live and count take precedence."},
            live: #{schema:#{type:"bool"},required:false,sensitive:false,
                "default":#{type:"bool",value:false},doc:"Shows a native-sampled pulsing status dot; honors motion preferences."},
            count: #{schema:#{type:"optional",value:#{type:"integer"}},required:false,sensitive:false,
                doc:"Count instead of text; negative values display as zero."},
            max: #{schema:#{type:"integer",min:0},required:false,sensitive:false,
                "default":#{type:"integer",value:99},doc:"Counts above this value display max+."},
            emphasis: #{
                schema: #{ type: "string", allowed: ["subtle", "strong"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "subtle" },
                doc: "`strong` draws a solid block with a paired readable label and no lamp, for states that need action.",
            },
            dot: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Shows a circular status dot before the label; strong/count variants omit it." },
            size: #{ schema: #{ type: "string", allowed: ["sm", "md"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "sm" },
                doc: "Badge height: `md` is `metrics.marker`, `sm` is `metrics.marker_small`." },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{icon:#{required:false,multiple:false,doc:"Optional leading icon."}},
        parts: ["root", "lamp", "icon", "label"],
    },
    render: Fn("render_Badge")
});

fn badge_tone(variant) { if variant == "accent" { "info" } else { variant } }
fn Badge(props) { render_component("components/badge", props) }
fn render_Badge(ctx, props) {
    let tone = badge_tone(if "variant" in props { props.variant } else { "neutral" });
    let strong = "emphasis" in props && props.emphasis == "strong";
    let live = "live" in props && props.live;
    let dot = live || ("dot" in props && props.dot);
    let count = if "count" in props && props.count != () { if props.count < 0 { 0 } else { props.count } } else { () };
    let top = if "max" in props { props.max } else { 99 };
    let label = if count != () {
        if count > top { `${ctx.format_number(top)}+` } else { ctx.format_number(count) }
    } else { props.text };
    let has_icon = "icon" in props && props.icon != ();
    if label == "" && !dot && !has_icon { return fragment([]); }
    let foreground = theme_color(`badge.${tone}_fg`);
    let fill = theme_color(`badge.${tone}_bg`);
    if strong {
        fill = theme_color(`badge.${tone}_strong_bg`);
        foreground = theme_color(`badge.${tone}_strong_fg`);
    }
    let state_style = style().background(fill).text_color(foreground);
    let small = !("size" in props) || props.size == "sm";
    let height = theme_length(if small { "metrics.marker_small" } else { "metrics.marker" });
    let children = [];
    if count == () && dot && !strong {
        let dot_ink = if tone == "neutral" { theme_color("text.subtle") } else { foreground };
        let layers = [];
        if live {
            // Native scale is a Canvas transform, not a box-layout transform.
            // Keep this vector in the same six-unit design domain as the lamp;
            // the drawable projection follows the theme's reading scale.
            let halo = canvas(canvas_scene([canvas_circle("pulse",3,3,3,dot_ink)]).view_box(0,0,6,6))
                .with_key("halo").with_style(style().absolute().left(px(0)).top(px(0))
                    .width(relative(1.0)).height(relative(1.0)))
                .motion(motion_keyframes("opacity", [
                    #{offset:0.0,value:0.5},#{offset:0.7,value:0.0},#{offset:1.0,value:0.0}
                ],#{duration_ms:1800,iterations:"infinite"}));
            for axis in ["scale_x","scale_y"] {
                halo = halo.motion(motion_keyframes(axis, [
                    #{offset:0.0,value:1.0,easing:cubic_bezier(0.22,1.0,0.36,1.0)},
                    #{offset:0.7,value:2.6},#{offset:1.0,value:2.6}
                ],#{duration_ms:1800,iterations:"infinite"}));
            }
            layers.push(halo);
        }
        children.push(row(layers).with_key("dot").with_style(ctx.component_style("lamp",
            style().relative().background(dot_ink))).accessibility_role("presentation"));
        state_style = state_style.padding_start(theme_length("metrics.badge_lead"));
    } else if count == () && has_icon {
        children.push(props.icon.with_key("icon").with_style(ctx.component_style("icon",
            style().text_color(foreground))).accessibility_role("presentation"));
        state_style = state_style.padding_start(theme_length("metrics.badge_lead"));
    }
    if count != () {
        state_style = state_style.radius(theme_radius("pill")).min_width(height).padding_x(theme_spacing("xs"));
    }
    children.push(text(label).with_key("label").with_style(ctx.component_style("label",
        style().min_width(px(0)).whitespace_nowrap().text_ellipsis().font_feature("tnum",1))));
    row(children).with_style(ctx.component_style("root",
        style().items_center().justify_center().min_height(height).flex_shrink(false), state_style))
        .env(#{size:if small{"sm"}else{"md"}})
        .accessibility_role("status").accessibility_label(label)
}
