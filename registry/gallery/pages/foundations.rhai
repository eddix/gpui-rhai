// Gallery foundation pages: spacing and metrics, typography, color roles, density.

import "gallery/kit" as kit;
import "patterns/description_list" as description_list;
import "components/button" as button;
import "components/input" as input;
import "components/badge" as badge;

fn pages() {
    [
        #{ id: "foundations.spacing", title: "Spacing and metrics", group: "Foundations" },
        #{ id: "foundations.typography", title: "Typography", group: "Foundations" },
        #{ id: "foundations.color", title: "Color roles", group: "Foundations" },
        #{ id: "foundations.density", title: "Density", group: "Foundations" },
    ]
}

fn view(ctx, h, v, id) {
    if id == "foundations.spacing" { spacing_page(h, v) }
    else if id == "foundations.typography" { typography_page(h, v) }
    else if id == "foundations.color" { color_page(h, v) }
    else if id == "foundations.density" { density_page(h, v) }
    else { () }
}

// A labelled bar whose width is the token itself.
fn bar(name, length) {
    row([
        text(name).with_style(style().width(theme_length("metrics.label_column")).flex_shrink(false)
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
    for metric in ["control", "control_pad", "row", "inset", "marker", "mark", "icon"] {
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
        "Weights 400 and 600 only. Body is 14/22 for CJK; the label voice is mono uppercase.", [
        kit::sec("Roles", kit::pile(roles)),
        kit::sec("Voices", kit::pile([
            sample("label", "SECTION LABEL · 标签"),
            sample("code", "cargo run --example byod_treemap"),
            sample("control_regular", "Control text 控件文字"),
        ])),
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

fn color_page(h, v) {
    kit::page(h, v, "foundations.color", "Color roles",
        "Paper, ink and one cobalt spot color. Normal is quiet: color marks only the deviation.", [
        kit::sec("Surfaces", swatches(["surface", "surface_raised", "surface_hover", "selection", "border"])),
        kit::sec("Text", swatches(["text_primary", "text_muted", "disabled", "focus_ring"])),
        kit::sec("Accent and status", swatches(["accent", "accent_hover", "success", "warning", "danger"])),
        kit::sec("Readable text", swatches(["text.accent", "text.success", "text.warning", "text.danger"])),
        kit::sec("In use", kit::line([
            badge::Badge(#{ text: "Healthy" }), badge::Badge(#{ text: "Degraded", variant: "warning" }),
            badge::Badge(#{ text: "Down", variant: "danger", emphasis: "strong" }),
        ])),
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
        "Comfortable for large displays, compact for laptops. Density changes space, never font size.", [
        kit::sec("Side by side", kit::densities(density_specimen(h, v, "c"), density_specimen(h, v, "k"))),
    ])
}
