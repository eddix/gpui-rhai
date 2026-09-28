import "components/button" as button;
import "components/draggable" as draggable;

fn state_schema() { #{ fields: #{
    position: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
        x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
        y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
    } }, "default": #{ type: "map", value: #{
        x: #{ type: "float", value: 56.0 }, y: #{ type: "float", value: 48.0 }
    } } },
} } }

fn moved(ctx, value) { ctx.set_state("position", value); }
fn reset(ctx, payload) { ctx.set_state("position", #{ x:56.0, y:48.0 }); }

fn view(ctx) {
    let position = ctx.get_state("position");
    let handle = row([
        text("Drag handle").with_style(style().typography("body").font_weight(700)),
        text("⌘").with_style(style().typography("body_small").text_color(theme_color("text_muted")))
    ]).with_style(style().height(px(38)).items_center().justify_between()
        .padding_x(theme_spacing("sm")).background(theme_color("surface_hover")));
    let card = column([
        text("The card keeps its controlled position. Dragging previews in Rust and commits once.")
            .with_style(style().typography("body_small").text_color(theme_color("text_muted"))
                .padding(theme_spacing("sm")))
    ]).with_style(style().width(px(260)).height(px(94))
        .background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border")));
    column([
        row([
            button::Button(#{ text:"Reset", variant:"outline", on_click:Fn("reset") }),
            text(`x ${position.x.round} · y ${position.y.round}`).accessibility_role("status")
        ]).with_style(style().gap(theme_spacing("sm")).items_center()),
        draggable::Draggable(#{ key:"story-draggable", label:"Move demo card",
            position:position, handle:handle, content:card,
            contain:true, snap_x:8.0, snap_y:8.0, on_move:Fn("moved")
        }).with_style(style().width(relative(1.0)).height(px(360))
            .background(theme_color("surface")).border(px(1)).border_color(theme_color("border")))
    ]).with_style(style().width(relative(1.0)).height(relative(1.0))
        .padding(theme_spacing("md")).gap(theme_spacing("md")))
}
