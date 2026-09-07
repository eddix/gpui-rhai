/* gpui-rhai
{
  "id": "components/pagination",
  "export": "Pagination",
  "version": "0.1.1",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": ["components/button", "components/icon", "components/select"],
  "capabilities": {},
  "assets": ["icons/chevron_left.svg", "icons/chevron_right.svg"]
}
*/
// Pagination is a stateless Rhai composition for one-based controlled page state.
//
// Props: key, total_items, current_page, page_size/options, boundary/sibling counts,
//        summary visibility, size, and atomic on_change.
// Events: change({ current_page, page_size }).
// Example: Pagination(#{ key: "users-pages", total_items: 128, current_page: 3, page_size: 10, on_change: Fn("page_changed") })

import "components/button" as button;
import "components/icon" as icon;
import "components/select" as select;

define_component(#{
    metadata: #{ id: "components/pagination", "export": "Pagination", version: "0.1.1",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: ["components/button", "components/icon", "components/select"], capabilities: #{},
        assets: ["icons/chevron_left.svg", "icons/chevron_right.svg"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            total_items: #{ schema: #{ type: "integer", min: 0 }, required: true, sensitive: false },
            current_page: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false },
            page_size: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false },
            page_size_options: #{ schema: #{ type: "array", max_items: 32, items: #{ type: "integer", min: 1 } }, required: false, sensitive: false,
                "default": #{ type: "array", value: [] } },
            boundary_count: #{ schema: #{ type: "integer", min: 1, max: 3 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 1 } },
            sibling_count: #{ schema: #{ type: "integer", min: 0, max: 3 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 1 } },
            show_summary: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            size: #{ schema: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "md" } },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{
            page_size_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
            page_size_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        } },
        events: #{ change: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
            current_page: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false },
            page_size: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false },
        } } } },
        slots: #{},
        parts: ["root", "summary", "controls", "previous", "next", "page", "page_current", "ellipsis", "page_size", "page_size_label"],
    },
    render: Fn("render_Pagination")
});

fn prop_or(props, name, fallback) {
    if name in props { props[name] } else { fallback }
}

fn total_pages(total_items, page_size) {
    let pages = total_items / page_size + if total_items % page_size == 0 { 0 } else { 1 };
    if pages < 1 { 1 } else { pages }
}

fn add_page(pages, page, total) {
    if page >= 1 && page <= total && !(page in pages) { pages.push(page); }
    pages
}

fn page_window(total, current, boundary, sibling) {
    let pages = [];
    for page in 1..(boundary + 1) { pages = add_page(pages, page, total); }
    let start = if current - sibling < 1 { 1 } else { current - sibling };
    let end = if current + sibling > total { total } else { current + sibling };
    if start <= end {
        for page in start..(end + 1) { pages = add_page(pages, page, total); }
    }
    let tail = if total - boundary + 1 < 1 { 1 } else { total - boundary + 1 };
    if tail <= total {
        for page in tail..(total + 1) { pages = add_page(pages, page, total); }
    }
    pages.sort();
    let result = [];
    let previous = 0;
    for page in pages {
        if previous > 0 {
            let gap = page - previous;
            if gap == 2 { result.push(previous + 1); }
            else if gap > 2 { result.push(0); }
        }
        result.push(page);
        previous = page;
    }
    result
}

fn pagination_emit(ctx, state) { ctx.emit("change", state); }
fn pagination_size(ctx, value) {
    ctx.emit("change", #{ current_page: 1, page_size: parse_int(value) });
}

fn page_button(ctx, page, current, page_size, size, callback) {
    let selected = page == current;
    let node = button::Button(#{
        text: `${page}`, size: size,
        variant: if selected { "primary" } else { "outline" },
        hoverable: !selected,
    }).with_style(ctx.component_style(if selected { "page_current" } else { "page" }, style()));
    if selected { node.tab_stop(false).accessibility_current("page") }
    else if callback == () { node }
    else { node.on_click_value(Fn("pagination_emit"), #{ current_page: page, page_size: page_size }) }
}

fn Pagination(props) { render_component("components/pagination", props) }
fn page_size_open(ctx, open) { ctx.set_state("page_size_open", open); }
fn page_size_query(ctx, query) { ctx.set_state("page_size_query", query); }
fn render_Pagination(ctx, props) {
    if props.key == "" { throw "Pagination key cannot be empty"; }
    let pages = total_pages(props.total_items, props.page_size);
    if props.current_page > pages {
        throw `Pagination current_page ${props.current_page} exceeds total pages ${pages}`;
    }
    let options = prop_or(props, "page_size_options", []);
    let seen_options = [];
    for option in options {
        if option in seen_options {
            throw `Pagination page_size_options contains duplicate value ${option}`;
        }
        seen_options.push(option);
    }
    if options.len > 0 && !(props.page_size in options) {
        throw "Pagination page_size_options must contain page_size";
    }
    let size = prop_or(props, "size", "md");
    let callback = if "on_change" in props { props.on_change } else { () };
    let controls = [];
    let previous_icon = icon::Icon(#{
        source: asset("app/icons/chevron_left"),
        rtl_source: asset("app/icons/chevron_right"), size: size,
        label: ctx.t("pagination.previous")
    });
    let previous = button::Button(#{
        text: ctx.t("pagination.previous"), prefix: previous_icon,
        size: size, variant: "outline", disabled: props.current_page == 1,
    }).with_style(ctx.component_style("previous", style()));
    controls.push(if callback == () { previous } else {
        previous.on_click_value(Fn("pagination_emit"), #{ current_page: props.current_page - 1, page_size: props.page_size })
    });
    for page in page_window(
        pages, props.current_page,
        prop_or(props, "boundary_count", 1),
        prop_or(props, "sibling_count", 1)
    ) {
        if page == 0 {
            controls.push(row([text("…")]).with_style(ctx.component_style("ellipsis",
                style().items_center().padding_x(px(4)))));
        } else {
            controls.push(page_button(ctx, page, props.current_page, props.page_size, size, callback));
        }
    }
    let next_icon = icon::Icon(#{
        source: asset("app/icons/chevron_right"),
        rtl_source: asset("app/icons/chevron_left"), size: size,
        label: ctx.t("pagination.next")
    });
    let next = button::Button(#{
        text: ctx.t("pagination.next"), suffix: next_icon,
        size: size, variant: "outline", disabled: props.current_page == pages,
    }).with_style(ctx.component_style("next", style()));
    controls.push(if callback == () { next } else {
        next.on_click_value(Fn("pagination_emit"), #{ current_page: props.current_page + 1, page_size: props.page_size })
    });

    let children = [];
    if prop_or(props, "show_summary", true) {
        children.push(text(`${ctx.format_number(props.total_items)} ${ctx.t("pagination.items")} · ${ctx.t("pagination.page")} ${ctx.format_number(props.current_page)} / ${ctx.format_number(pages)}`)
            .with_style(ctx.component_style("summary", style()
                .typography("body_small").text_color(theme_color("text_muted")))));
    }
    children.push(row(controls).with_style(ctx.component_style("controls",
        style().gap(theme_spacing("xs")).items_center())));
    if options.len > 0 {
        let select_options = [];
        for option in options {
            select_options.push(#{ value: `${option}`, label: `${option}` });
        }
        children.push(row([
            text(ctx.t("pagination.per_page")).with_style(ctx.component_style("page_size_label", style())),
            select::Select(#{
                key: `${props.key}-page-size`, options: select_options,
                value: `${props.page_size}`, open: ctx.get_state("page_size_open"),
                query: ctx.get_state("page_size_query"), size: size, width: px(96),
                on_change: Fn("pagination_size"), on_open_change: Fn("page_size_open"),
                on_query_change: Fn("page_size_query")
            }).with_style(ctx.component_style("page_size", style()))
        ]).with_style(style().gap(theme_spacing("xs")).items_center()
            .typography("body_small")));
    }
    row(children).with_key(props.key)
        .with_style(ctx.component_style("root", style()
            .items_center().justify_between().gap(theme_spacing("sm"))
            .typography("body")))
        .accessibility_role("navigation")
}
