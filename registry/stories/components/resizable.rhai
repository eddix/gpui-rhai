import "components/button" as button;
import "components/resizable" as resizable;

fn state_schema() { #{ fields: #{
    rect: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
        x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
        y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
        width: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false },
        height: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false },
    } }, "default": #{ type: "map", value: #{
        x: #{ type: "float", value: 48.0 }, y: #{ type: "float", value: 36.0 },
        width: #{ type: "float", value: 300.0 }, height: #{ type: "float", value: 180.0 }
    } } },
    lock_ratio: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
    last_handle: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "none" } },
} } }

fn resized(ctx, value) {
    ctx.set_state("rect", #{ x:value.x, y:value.y, width:value.width, height:value.height });
    ctx.set_state("last_handle", value.handle);
}
fn toggle_ratio(ctx, payload) { ctx.set_state("lock_ratio", !ctx.get_state("lock_ratio")); }
fn reset_rect(ctx, payload) {
    ctx.set_state("rect", #{ x:48.0, y:36.0, width:300.0, height:180.0 });
    ctx.set_state("last_handle", "none");
}

fn view(ctx) {
    let rect = ctx.get_state("rect");
    let lock_ratio = ctx.get_state("lock_ratio");
    column([
        row([
            button::Button(#{ text: if lock_ratio { "Unlock ratio" } else { "Lock 5:3 ratio" },
                variant:"outline", on_click:Fn("toggle_ratio") }),
            button::Button(#{ text:"Reset rectangle", variant:"outline", on_click:Fn("reset_rect") }),
            text(`x ${rect.x.round} · y ${rect.y.round} · ${rect.width.round} × ${rect.height.round} · ${ctx.get_state("last_handle")}`)
                .accessibility_role("status")
        ]).with_style(style().gap(theme_spacing("sm")).items_center()),
        resizable::Resizable(#{ key:"story-resizable", label:"Demo card", rect:rect,
            min_width:160.0, min_height:100.0, max_width:520.0, max_height:320.0,
            aspect_ratio: if lock_ratio { 5.0 / 3.0 } else { () }, contain:true,
            content: column([
                text("Resizable card").with_style(style().typography("title")),
                text("Drag any edge or corner. Focus a handle and use arrows; Shift accelerates.")
                    .with_style(style().typography("body_small").text_color(theme_color("text_muted")))
            ]).with_style(style().width(relative(1.0)).height(relative(1.0))
                .padding(theme_spacing("md")).gap(theme_spacing("sm"))
                .background(theme_color("surface_raised"))
                .border(px(1)).border_color(theme_color("border"))),
            on_resize:Fn("resized")
        }).with_style(style().width(relative(1.0)).height(px(390))
            .background(theme_color("surface")).border(px(1)).border_color(theme_color("border")))
    ]).with_style(style().width(relative(1.0)).height(relative(1.0))
        .padding(theme_spacing("md")).gap(theme_spacing("md")))
}
