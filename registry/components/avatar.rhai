/* gpui-rhai
{
  "id": "components/avatar",
  "export": "Avatar",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_hover", "surface", "text_primary", "success", "warning", "danger", "text_muted",
    "metrics.control", "typography.control"
  ],
  "environment": ["size", "density"]
}
*/
// Avatar displays an image handle or name-derived initials with presence.
//
// Example: Avatar(#{ name: "Ada Lovelace", initials: "AL", presence: "online" })

define_component(#{
    metadata: #{
        id: "components/avatar", "export": "Avatar", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "surface_hover", "surface", "text_primary", "success", "warning", "danger",
            "text_muted", "metrics.control", "typography.control"
        ],
        environment: ["size", "density"],
    },
    schema: #{
        props: #{
            name: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "The person's name: the accessible name, and the source of the fallback initials." },
            handle: #{
                schema: #{ type: "optional", value: #{ type: "handle", kind: "image" } },
                required: false, sensitive: false,
                doc: "Image handle of the picture, shown in the circle instead of the initials.",
            },
            initials: #{
                schema: #{ type: "optional", value: #{ type: "string" } },
                required: false, sensitive: false,
                doc: "Text shown as written when there is no `handle`; unset, the first grapheme (user-perceived character) of `name`, uppercased.",
            },
            size: #{
                schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false,
                doc: "Control size whose `metrics.control` is the diameter; without it, the environment's size.",
            },
            presence: #{
                schema: #{ type: "string", allowed: ["none", "online", "away", "busy"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "none" },
                doc: "Adds an 8px dot at the bottom right: `success` for online, `warning` away, `danger` busy.",
            },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{},
        parts: ["root", "image", "initials", "presence"],
    },
    render: Fn("render_Avatar")
});

fn presence_color(presence) {
    if presence == "online" { theme_color("success") }
    else if presence == "away" { theme_color("warning") }
    else if presence == "busy" { theme_color("danger") }
    else { rgba(0x00000000) }
}

// The longest run of leading characters that still counts as one grapheme, so a
// base letter keeps the combining marks that follow it.
fn first_grapheme(value) {
    let first = "";
    for character in value {
        let next = first + character;
        if first != "" && grapheme_count(next) > 1 { break; }
        first = next;
    }
    first
}

fn Avatar(props) { render_component("components/avatar", props) }
fn render_Avatar(ctx, props) {
    let initials = if "initials" in props && props.initials != () {
        props.initials
    } else {
        first_grapheme(props.name).to_upper()
    };
    // Avatars share the control height of their size, so they sit in toolbars and rows.
    let side = theme_length("metrics.control");
    let content = if "handle" in props && props.handle != () {
        image(props.handle).with_style(ctx.component_style("image", style().width(side).height(side)))
    } else {
        text(initials).with_style(ctx.component_style("initials",
            style().text_color(theme_color("text_primary"))))
    };
    let face = row([content]).with_style(
        style().width(side).height(side).clip()
            .items_center().justify_center().radius(side * 0.5)
            .background(theme_color("surface_hover"))
            .typography("control").font_weight(600));
    let children = [face];
    let presence = if "presence" in props { props.presence } else { "none" };
    if presence != "none" {
        // The presence dot is a semantic 8px circle with a 2px surface ring.
        children.push(row([]).with_style(ctx.component_style("presence",
            style().width(px(8)).height(px(8)).absolute().right(px(0)).bottom(px(0))
                .radius(px(4)).border(px(2)).border_color(theme_color("surface"))
                .background(presence_color(presence)))));
    }
    let root = stack(children)
        .with_style(ctx.component_style("root", style().width(side).height(side)))
        .accessibility_role("image")
        .accessibility_label(props.name);
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
