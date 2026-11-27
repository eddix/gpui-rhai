// Gallery foundation pages: spacing and metrics, typography, color roles, density.

import "gallery/kit" as kit;
import "patterns/description_list" as description_list;
import "components/button" as button;
import "components/input" as input;
import "components/badge" as badge;
import "components/resizable" as resizable;
import "components/split_pane" as split_pane;
import "components/table" as table;
import "components/slider" as slider;
import "components/range_slider" as range_slider;
import "components/scroll_area" as scroll_area;

fn pages() {
    [
        #{ id: "foundations.spacing", title: "Spacing and metrics", group: "Foundations" },
        #{ id: "foundations.typography", title: "Typography", group: "Foundations" },
        #{ id: "foundations.color", title: "Color roles", group: "Foundations" },
        #{ id: "foundations.density", title: "Density", group: "Foundations" },
        #{ id: "foundations.responsive_grid", title: "Responsive grid", group: "Foundations" },
        #{ id: "foundations.group_opacity", title: "Group opacity", group: "Foundations" },
        #{ id: "foundations.foreground_blur", title: "Foreground blur", group: "Foundations" },
        #{ id: "foundations.backdrop_blur", title: "Backdrop blur", group: "Foundations" },
        #{ id: "foundations.affine", title: "Visual transforms", group: "Foundations" },
    ]
}

fn view(ctx, h, v, id) {
    if id == "foundations.spacing" { spacing_page(h, v) }
    else if id == "foundations.typography" { typography_page(h, v) }
    else if id == "foundations.color" { color_page(h, v) }
    else if id == "foundations.density" { density_page(h, v) }
    else if id == "foundations.responsive_grid" { responsive_grid_page(h, v) }
    else if id == "foundations.group_opacity" { group_opacity_page(h, v) }
    else if id == "foundations.foreground_blur" { foreground_blur_page(h, v) }
    else if id == "foundations.backdrop_blur" { backdrop_blur_page(h, v) }
    else if id == "foundations.affine" { affine_page(h, v) }
    else { () }
}

fn opacity_swatch(isolated, alpha) {
    let left = box([]).with_style(style().absolute().left(px(12)).top(px(10))
        .width(px(60)).height(px(54)).radius(theme_radius("sm")).background(theme_color("accent")));
    let right = box([]).with_style(style().absolute().left(px(46)).top(px(30))
        .width(px(60)).height(px(54)).radius(theme_radius("sm")).background(theme_color("success")));
    column([
        text(if isolated { "Isolated group" } else { "Per primitive" }).with_style(style().typography("body")),
        box([left,right]).with_style(style().relative().width(px(120)).height(px(96))
            .opacity(alpha).isolated_opacity(isolated))
    ]).with_style(style().gap(theme_spacing("xs")))
}

fn foreground_blur_page(h, v) {
    let sharp = kit::val(v, "foreground_blur.sharp", false);
    let sigma = if sharp { 0 } else { 2.5 };
    kit::page(h, v, "foundations.foreground_blur", "Foreground blur",
        "The filter softens the composed glyphs or shapes. The backdrop and input geometry stay unchanged.", [
            kit::sec("Text and shapes", kit::pile([
                row([
                    text("Foreground").with_style(style().typography("heading")
                        .text_color(theme_color("text_primary"))),
                ]).with_style(style().foreground_blur(sigma)),
                box([opacity_swatch(true,0.85)]).with_style(style().foreground_blur(sigma)),
                button::Button(#{key:"switch-blur",text:if sharp {"Soften foreground"} else {"Sharpen foreground"},
                    on_click:kit::put(h,"foreground_blur.sharp",!sharp)})
            ]))
        ])
}

fn backdrop_blur_page(h, v) {
    let sharp = kit::val(v, "backdrop_blur.sharp", false);
    let combined = kit::val(v, "backdrop_blur.combined", false);
    let turned = kit::val(v, "backdrop_blur.turned", false);
    let palette = ["accent", "surface", "success", "surface", "text_primary", "surface"];
    let layers = [];
    for i in 0..12 {
        layers.push(box([]).with_style(style().absolute().left(px(24*i)).top(px(0))
            .width(px(24)).height(px(112)).background(theme_color(palette[i % 6]))));
    }
    layers.push(row([text(if combined {"Composed panel"}else{"Crisp text"}).with_style(style().typography("heading"))])
        .with_key("backdrop-specimen")
        .with_style(style().absolute().left(px(36)).right(px(36)).top(px(24)).height(px(64))
            .items_center().justify_center().radius(theme_radius("sm"))
            .background(alpha(theme_color("surface"),0.62)).text_color(theme_color("text_primary"))
            .backdrop_blur(if sharp { 0 } else { 8 })
            .isolated_opacity(combined).opacity(if combined {0.55}else{1.0})
            .foreground_blur(if combined {1.5}else{0.0})
            .transform(#{rotate:if turned {8}else{0}})));
    kit::page(h, v, "foundations.backdrop_blur", "Backdrop blur",
        "Backdrop blur reads only the stripes already painted under the panel. Combine filters to soften and fade the complete panel once.", [
            kit::sec("Live destination", kit::pile([
                box(layers).with_style(style().relative().width(px(288)).height(px(112))),
                kit::line([
                    button::Button(#{key:"switch-backdrop",text:if sharp {"Soften backdrop"} else {"Sharpen backdrop"},
                        on_click:kit::put(h,"backdrop_blur.sharp",!sharp)}),
                    button::Button(#{key:"switch-combined",text:if combined {"Use only backdrop"} else {"Combine filters"},
                        variant:"outline",on_click:kit::put(h,"backdrop_blur.combined",!combined)})
                ]),
                button::Button(#{key:"switch-backdrop-rotation",text:if turned {"Straighten panel"} else {"Rotate panel"},
                    variant:"outline",on_click:kit::put(h,"backdrop_blur.turned",!turned)})
            ]))
        ])
}

fn affine_page(h, v) {
    let turned = kit::val(v, "affine.turned", true);
    let value = kit::val(v, "affine.text", "Edit this on the diagonal");
    let editor = column([
        input::Input(#{key:"affine-editor",label:"Transformed editor",value:value,
            on_change:kit::store(h,"affine.text")}),
        text("Drag across the text or click to move the caret.")
            .with_style(style().typography("caption").text_color(theme_color("text_muted")))
    ]).with_key("affine-content").with_style(style().width(px(292))
        .padding(theme_spacing("sm")).gap(theme_spacing("xs"))
        .background(theme_color("surface_raised")).radius(theme_radius("sm"))
        .transform(#{rotate:if turned { 12 } else { 0 }}));
    kit::page(h, v, "foundations.affine", "Visual transforms",
        "Pointer hits, text carets and resize handles follow the painted axes; the surrounding layout does not rotate.", [
            kit::sec("Rotate in place", kit::pile([
                kit::line_top([
                    box([editor]).with_style(style().width(px(342)).height(px(166))
                        .flex_row().items_center().justify_center()),
                    affine_resize_stage(affine_scroll(), "affine-scroll-stage", turned),
                ]),
                button::Button(#{key:"switch-affine",text:if turned {"Straighten specimens"} else {"Rotate specimens"},
                    on_click:kit::put(h,"affine.turned",!turned)})
            ])),
            kit::sec("Resize on the diagonal", kit::line_top([
                affine_resize_stage(affine_split(h,v), "affine-split-stage", turned),
                affine_resize_stage(affine_resize(h,v), "affine-resize-stage", turned),
            ])),
            kit::sec("Resize a column or adjust a value", kit::line_top([
                affine_resize_stage(affine_table(h,v), "affine-table-stage", turned),
                affine_stage(affine_sliders(h,v), "affine-slider-stage", turned, 208),
            ])),
        ])
}

fn affine_resize_stage(content, key, turned) {
    affine_stage(content,key,turned,122)
}

fn affine_scroll() {
    let lines = [];
    for index in 1..25 {
        lines.push(text(`Probe ${index} · ready`).with_style(style().typography("body")));
    }
    scroll_area::ScrollArea(#{key:"affine-log",label:"Transformed scroll area",width:relative(1.0),
        axis:"both",height:px(122),vertical_scrollbar:"always",horizontal_scrollbar:"always",
        content:column(lines).with_style(style().width(px(420)).flex_shrink(false).padding(theme_spacing("sm")))})
}

fn affine_stage(content, key, turned, height) {
    let layer = box([content]).with_style(style().width(px(262)).height(px(height))
        .background(theme_color("surface_raised")).transform(#{rotate:if turned{12}else{0}}));
    box([layer]).with_key(key).with_style(style().width(px(308)).height(px(height+70))
        .flex_row().items_center().justify_center())
}

fn affine_pane(title, paper) {
    box([text(title)]).with_style(style()
        .width(relative(1.0)).height(relative(1.0)).padding(theme_spacing("sm"))
        .background(paper))
}

fn affine_split(h, v) {
    let start = affine_pane("Start", theme_color("surface_raised"))
        .accessibility_role("region").accessibility_label("Transformed start pane");
    split_pane::SplitPane(#{key:"affine-split",label:"Transformed split",
        size:kit::val(v,"affine.split",0.5),min_start:50.0,min_end:50.0,
        start:start,end:affine_pane("End",theme_color("surface")),
        on_resize:kit::store(h,"affine.split")})
}

fn affine_resize(h, v) {
    let panel = box([text("Card")]).accessibility_role("region").accessibility_label("Transformed card surface")
        .with_style(style().width(relative(1.0)).height(relative(1.0)).padding(theme_spacing("sm"))
            .background(theme_color("surface")).radius(theme_radius("sm")));
    resizable::Resizable(#{key:"affine-card",label:"Transformed card",content:panel,handles:["se"],
        rect:kit::val(v,"affine.rect",#{x:18.0,y:16.0,width:132.0,height:66.0}),
        min_width:68.0,min_height:34.0,on_resize:kit::store(h,"affine.rect")})
}

fn affine_table(h, v) {
    let width = kit::val(v, "affine.column", #{width:#{kind:"fixed",value:110.0}}).width;
    table::Table(#{key:"affine-table",label:"Transformed table",row_key:"id",height:70.0,
        columns:[
            #{key:"task",title:"Task",width:width,resizable:true,min_width:64.0,max_width:180.0},
            #{key:"state",title:"State",width:#{kind:"fixed",value:148.0}},
        ],rows:[#{id:"prepare",task:"Prepare",state:"Ready"},#{id:"ship",task:"Ship",state:"Waiting"}],
        on_column_resize:kit::store(h,"affine.column")})
        .with_style(style().width(relative(1.0)))
}

fn affine_sliders(h, v) {
    let rail = #{track:style().width(px(156))};
    column([
        slider::Slider(#{key:"affine-level",label:"Transformed level",step:5.0,part_styles:rail,
            value:kit::val(v,"affine.level",35.0),on_change:kit::store(h,"affine.level")}),
        range_slider::RangeSlider(#{key:"affine-span",label:"Transformed span",step:5.0,
            minimum_gap:10.0,part_styles:rail,
            values:kit::val(v,"affine.span",#{low:25.0,high:75.0}),on_change:kit::store(h,"affine.span")}),
    ]).with_style(style().gap(theme_spacing("sm")).padding(theme_spacing("sm")))
}

fn group_opacity_page(h, v) {
    let opaque = kit::val(v, "group_opacity.opaque", false);
    let alpha = if opaque { 1.0 } else { 0.5 };
    kit::page(h, v, "foundations.group_opacity", "Group opacity",
        "Both specimens use the same two opaque shapes. Isolated opacity fades the composite once, so the rear color does not show through the front shape.", [
            kit::sec("Compositing", kit::pile([
                kit::line([opacity_swatch(false,alpha), opacity_swatch(true,alpha)]),
                button::Button(#{key:"switch-opacity",text:if opaque {"Show half opacity"} else {"Show full opacity"},
                    on_click:kit::put(h,"group_opacity.opaque",!opaque)})
            ]))
        ])
}

// A labelled bar whose width is the token itself. The label column takes one and a
// half label columns so the longest metric name stays on one line.
fn bar(name, length) {
    row([
        text(name).with_style(style().width(theme_length("metrics.label_column") * 1.5).flex_shrink(false)
            .typography("code").text_color(theme_color("text_muted"))),
        row([]).with_style(style().width(length).height(theme_length("metrics.mark") * 0.5)
            .background(theme_color("accent"))),
    ]).with_style(style().flex_row().items_center().gap(theme_length("space.related"))
        .min_height(theme_length("metrics.marker")))
}

fn spacing_page(h, v) {
    let scale = [];
    for step in ["xxs", "xs", "sm", "md", "lg", "xl"] { scale.push(bar(`spacing.${step}`, theme_spacing(step))); }
    let aliases = [];
    for alias in ["unit", "related", "group", "section"] { aliases.push(bar(`space.${alias}`, theme_length(`space.${alias}`))); }
    let metrics = [];
    for metric in ["control", "control_pad", "field_pad", "multiline_pad", "row", "inset", "marker", "mark", "icon"] {
        metrics.push(bar(`metrics.${metric}`, theme_length(`metrics.${metric}`)));
    }
    kit::page(h, v, "foundations.spacing", "Spacing and metrics",
        "Every length a component uses comes from these tokens; compact shrinks the outer steps.", [
        kit::sec("Scale", kit::pile(scale)),
        kit::sec("Relationships", kit::pile(aliases)),
        kit::sec("Metrics", kit::pile(metrics)),
    ])
}

fn sample(role, content) {
    row([
        text(role).with_style(style().width(theme_length("metrics.label_column")).flex_shrink(false)
            .typography("code").text_color(theme_color("text_muted"))),
        text(content).with_style(style().typography(role).text_color(theme_color("text_primary"))),
    ]).with_style(style().flex_row().items_center().gap(theme_length("space.related")))
        .audit_allow(["mixed-type-in-row"])
}

fn typography_page(h, v) {
    let roles = [];
    for role in ["display_large", "display", "heading", "title", "subtitle", "body", "body_small", "caption"] {
        roles.push(sample(role, "Deploy 128 hosts 部署到生产环境"));
    }
    kit::page(h, v, "foundations.typography", "Typography",
        "Inter carries the UI in weights 400, 500 and 600. Typography follows the selected design and component role; code keeps its own monospace family.", [
        kit::sec("Roles", kit::pile(roles)),
        kit::sec("Voices", kit::pile([
            sample("label", "Section label · 标签"),
            sample("code", "cargo run --example byod_treemap"),
            sample("control_regular", "Control text 控件文字"),
        ])),
        kit::sec("Inline code", text([span("Run "),
            span("cargo run --example byod_treemap").typography("code").background(theme_color("surface_raised")),
            span(" to open the treemap; pods such as "), span("dp-09aa6aec4c-0lO1I").typography("code"),
            span(" keep 0, O, 1, l and I apart.")])
            .with_style(style().typography("body").text_color(theme_color("text_primary")))),
    ])
}

fn swatch(role) {
    row([
        row([]).with_style(style().width(theme_length("metrics.control")).height(theme_length("metrics.control"))
            .background(theme_color(role)).border(px(1)).border_color(theme_color("border"))),
        text(role).with_style(style().typography("code").text_color(theme_color("text_primary"))),
    ]).with_style(style().flex_row().items_center().gap(theme_length("space.unit"))
        .width(theme_length("metrics.label_column") * 2))
}

fn swatches(roles) {
    let items = [];
    for role in roles { items.push(swatch(role)); }
    kit::line(items)
}

fn palette_pill(hue) {
    row([
        box([]).with_style(style().width(theme_length("metrics.mark")).height(theme_length("metrics.mark"))
            .radius(theme_radius("full")).background(theme_color(`palette.${hue}`)).flex_shrink(false)),
        text(`palette.${hue}`).with_style(style().typography("code").text_color(theme_color(`palette.${hue}_ink`))),
    ]).with_style(style().flex_row().items_center().gap(theme_spacing("xs"))
        .padding_x(theme_spacing("sm")).padding_y(theme_spacing("xxs"))
        .radius(theme_radius("md")).background(theme_color(`palette.${hue}_soft`))
        .border(px(1)).border_color(theme_color(`palette.${hue}_border`)))
}

fn color_page(h, v) {
    let palette_pills = [];
    for hue in ["blue", "violet", "teal", "green", "amber", "rose", "orange", "slate"] {
        palette_pills.push(palette_pill(hue));
    }
    kit::page(h, v, "foundations.color", "Color roles",
        "Paired surfaces, borders, semantic accents, and non-semantic categorical hues follow the selected color theme independently of Modern or Clean design.", [
        kit::sec("Surfaces", swatches(["surface_sunken", "surface", "surface_raised", "surface_elevated", "surface_hover", "selection"])),
        kit::sec("Borders", swatches(["border_subtle", "border", "border_strong"])),
        kit::sec("Text", swatches(["text_primary", "text_muted", "disabled", "focus_ring"])),
        kit::sec("Accent and status", swatches(["accent", "accent_hover", "success", "warning", "danger"])),
        kit::sec("Readable text", swatches(["text.accent", "text.success", "text.warning", "text.danger"])),
        kit::sec("Categorical palette", kit::line(palette_pills)),
        kit::sec("In use", kit::line([
            badge::Badge(#{ text: "Healthy" }), badge::Badge(#{ text: "Degraded", variant: "warning" }),
            badge::Badge(#{ text: "Down", variant: "danger", emphasis: "strong" }),
        ])),
        kit::sec("Native foreground", row([
            canvas(canvas_scene([canvas_rect("ink",0,0,20,20,current_color())]).view_box(0,0,20,20))
                .with_style(style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon")).flex_shrink(false)),
            text("Hover or focus: the graphic and text share one native foreground")
        ]).with_style(style().items_center().gap(theme_spacing("sm")).padding(theme_spacing("sm"))
            .text_color(theme_color("text_primary")).hover(style().text_color(theme_color("text.accent")))
            .focus(style().text_color(theme_color("text.success"))))
            .accessibility_role("button").accessibility_label("Live Canvas color").on_click(h.noop)),
    ])
}

fn density_specimen(h, v, suffix) {
    kit::pile([
        kit::line([
            input::Input(#{ key: `density-input-${suffix}`, label: "Host", value: kit::val(v, "density.host", "edge-03"),
                on_change: kit::store(h, "density.host") }).with_style(style().width(px(180))),
            button::Button(#{ key: `density-apply-${suffix}`, text: "Apply", on_click: h.noop }),
        ]),
        description_list::DescriptionList(#{ label: `Metrics ${suffix}`, items: [
            #{ label: "Control", value: "metrics.control" }, #{ label: "Row", value: "metrics.row" },
        ] }),
    ])
}

fn density_page(h, v) {
    kit::page(h, v, "foundations.density", "Density",
        "Comfortable for large displays, compact for laptops. Density changes space, never font size; the reading scale enlarges type and the controls around it together.", [
        kit::sec("Side by side", kit::densities(density_specimen(h, v, "c"), density_specimen(h, v, "k"))),
        kit::sec("Reading scale", kit::scales([density_specimen(h, v, "s100"),
            density_specimen(h, v, "s125"), density_specimen(h, v, "s150")])),
    ])
}

// A plain page-module callback, not an entry-module handler-map workaround.
// Its lexical environment is this module; its state owner remains the Gallery.
fn select_grid_tile(index, ctx, event) {
    let values = ctx.get_state("values");
    values["grid.selected"] = index;
    ctx.set_state("values", values);
}

fn responsive_grid_page(h, v) {
    let selected = kit::val(v, "grid.selected", -1);
    let tiles = [];
    for index in 0..20 {
        let cover_style = style().width(relative(1.0)).aspect_ratio(1)
            .background(if index == selected { theme_color("selection") } else { theme_color("surface_raised") })
            .border(px(1)).border_color(theme_color("border"));
        let cover = column([text(`${index + 1}`).with_style(style().typography("display"))])
            .with_style(cover_style.flex_col().items_center().justify_center());
        let caption = text(`Item ${index + 1}`).with_style(style().typography("body"));
        let tile = column([cover, caption]).with_key(`grid-item-${index}`)
            .with_style(style().width(relative(1.0)).gap(theme_spacing("xs")))
            .accessibility_role("button").accessibility_label(`Select item ${index + 1}`)
            .on_click(Fn("select_grid_tile").curry(index));
        tiles.push(tile);
    }
    let grid = box(tiles).with_key("responsive-grid")
        .with_style(style().width(relative(1.0)).auto_grid_columns(theme_length("metrics.label_column"),
            theme_length("metrics.label_column") * 1.25, theme_length("metrics.label_column") * 1.75)
            .gap(theme_spacing("sm")));
    let choose_first = button::Button(#{key:"grid-select-first", text:"Select first item",
        on_click:Fn("select_grid_tile").curry(0)});
    kit::page(h, v, "foundations.responsive_grid", "Responsive grid",
        "Resize the window: 128–224px auto-fit tracks, equal last-row widths and square covers. Layout runs natively; selection is an ordinary module callback.", [
        kit::sec("Selection", kit::line([choose_first, kit::note(if selected < 0 { "No selection" } else { `Selected item ${selected + 1}` })])),
        kit::sec("Twenty items", grid),
    ])
}
