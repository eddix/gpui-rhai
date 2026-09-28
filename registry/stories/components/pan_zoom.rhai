import "components/pan_zoom" as pan_zoom;

fn state_schema() { #{ fields: #{
    viewport: #{ schema:#{type:"object",allow_unknown:false,fields:#{
        x:#{schema:#{type:"number"},required:true,sensitive:false},
        y:#{schema:#{type:"number"},required:true,sensitive:false},
        scale:#{schema:#{type:"number",exclusive_min:0.0},required:true,sensitive:false},
    }}, "default":#{type:"map",value:#{
        x:#{type:"float",value:0.0},y:#{type:"float",value:0.0},scale:#{type:"float",value:1.0}
    }} },
} } }

fn changed(ctx, value) { ctx.set_state("viewport", value); }

fn scene() {
    let commands = [];
    for index in 0..11 {
        let offset = index * 30.0;
        commands.push(canvas_line(`vertical-${index}`, offset, 0.0, offset, 300.0, 1.0, theme_color("border")));
        commands.push(canvas_line(`horizontal-${index}`, 0.0, offset, 300.0, offset, 1.0, theme_color("border")));
    }
    commands.push(canvas_circle("origin", 150.0, 150.0, 16.0, theme_color("accent")));
    commands.push(canvas_rect("node-a", 54.0, 70.0, 46.0, 32.0, theme_color("success")));
    commands.push(canvas_rect("node-b", 210.0, 196.0, 52.0, 38.0, theme_color("warning")));
    canvas(canvas_scene(commands)).with_key("pan-zoom-canvas")
        .with_style(style().width(relative(1.0)).height(relative(1.0)))
}

fn view(ctx) {
    let viewport = ctx.get_state("viewport");
    column([
        text("CANVAS PAN + ZOOM").with_style(style().typography("heading")),
        text("Drag to pan. Use Command/Control + wheel to zoom at the pointer; arrows and +/- are keyboard equivalents.")
            .with_style(style().typography("body_small").text_color(theme_color("text_muted"))),
        text(`x ${viewport.x}, y ${viewport.y}, scale ${viewport.scale}`)
            .accessibility_role("status"),
        pan_zoom::PanZoom(#{ key:"canvas-viewport", label:"Pan and zoom canvas",
            transform:viewport, min_scale:0.5, max_scale:4.0, content:scene(),
            on_transform_change:Fn("changed") })
            .with_style(style().width(relative(1.0)).height(px(320)))
    ]).with_style(style().width(relative(1.0)).height(relative(1.0))
        .padding(theme_spacing("md")).gap(theme_spacing("sm")))
}
