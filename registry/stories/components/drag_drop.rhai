import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;

fn state_schema() { #{ fields: #{
    owner: #{ schema:#{type:"string",allowed:["backlog","deployed"]},
        "default":#{type:"string",value:"backlog"} },
    status: #{ schema:#{type:"string"}, "default":#{type:"string",value:"Ready"} },
} } }

fn dropped(ctx, value) {
    if value.payload.id == "service-alpha" {
        ctx.set_state("owner", value.target_id);
        ctx.set_state("status", `${value.operation} service-alpha → ${value.target_id}`);
    }
}

fn drag_ended(ctx, value) {
    if value.cancelled { ctx.set_state("status", "Drag cancelled"); }
    else if !value.accepted { ctx.set_state("status", "No matching target"); }
}

fn service_card(ctx) {
    let other = if ctx.get_state("owner") == "backlog" { "deployed" } else { "backlog" };
    drag_source::DragSource(#{ key:"service-alpha", label:"Move service alpha",
        source_id:"service-alpha", payload_type:"service", payload:#{id:"service-alpha"},
        operation:"move", keyboard_target:other,
        content:column([
            text("SERVICE ALPHA").with_style(style().typography("body").font_weight(700)),
            text("Drag, or focus and press Enter").with_style(style().typography("caption")
                .text_color(theme_color("text_muted")))
        ]).with_style(style().padding(theme_spacing("sm")).gap(theme_spacing("xxs"))
            .background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border"))),
        on_drag_end:Fn("drag_ended")
    })
}

fn lane(ctx, id, label) {
    let children = [text(label).with_style(style().typography("caption").font_weight(700)
        .text_color(theme_color("text_muted")))];
    if ctx.get_state("owner") == id { children.push(service_card(ctx)); }
    else { children.push(text("Drop service here").with_style(style().typography("body_small")
        .text_color(theme_color("text_muted")))); }
    drop_zone::DropZone(#{ key:`lane-${id}`, label:`${label} drop zone`, target_id:id,
        payload_types:["service"], operations:["move"],
        content:column(children).with_style(style().width(relative(1.0)).height(relative(1.0))
            .padding(theme_spacing("md")).gap(theme_spacing("sm"))),
        on_drop:Fn("dropped")
    }).with_style(style().width(relative(0.5)).height(px(190))
        .border(px(1)).border_color(theme_color("border")).background(theme_color("surface")))
}

fn view(ctx) {
    column([
        text("TYPED APPLICATION DRAG").with_style(style().typography("heading")),
        text(ctx.get_state("status")).accessibility_role("status"),
        row([lane(ctx,"backlog","BACKLOG"),lane(ctx,"deployed","DEPLOYED")])
            .with_style(style().width(relative(1.0)).gap(theme_spacing("md")))
    ]).with_style(style().width(relative(1.0)).height(relative(1.0))
        .padding(theme_spacing("md")).gap(theme_spacing("md")))
}
