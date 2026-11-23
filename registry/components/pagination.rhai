/* gpui-rhai
{
  "id": "components/pagination",
  "export": "Pagination",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/button", "components/icon_button", "components/icon", "components/select"],
  "capabilities": {},
  "assets": ["icons/chevron_left.svg", "icons/chevron_right.svg"],
  "tokens": [
    "pagination.active", "pagination.ink", "pagination.muted", "text_muted", "spacing.xxs", "spacing.xs", "spacing.sm", "space.group",
    "typography.body", "typography.pagination", "metrics.pagination_h", "metrics.pagination_pad", "metrics.pagination_icon"
  ],
  "environment": ["size", "density"]
}
*/
// Pagination composes one-based controlled page state; only popup state is local.
//
// Example: Pagination(#{ key: "users-pages", label: "Users pagination", total_items: 128, current_page: 3, page_size: 10, on_change: Fn("page_changed") })

import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/icon" as icon;
import "components/select" as select;

define_component(#{
    metadata: #{ id: "components/pagination", "export": "Pagination", version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/button", "components/icon_button", "components/icon", "components/select"], capabilities: #{},
        tokens: [
            "pagination.active", "pagination.ink", "pagination.muted", "text_muted", "spacing.xxs", "spacing.xs", "spacing.sm", "space.group",
            "typography.body", "typography.pagination", "metrics.pagination_h", "metrics.pagination_pad", "metrics.pagination_icon"
        ],
        environment: ["size", "density"],
        assets: ["icons/chevron_left.svg", "icons/chevron_right.svg"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable identity of this instance; must not be empty, and keys the per-page Select as `<key>-page-size`." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the navigation landmark." },
            total_items: #{ schema: #{ type: "integer", min: 0 }, required: true, sensitive: false,
                doc: "Number of items across all pages; the page count is this over `page_size` rounded up, at least 1." },
            current_page: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false,
                doc: "One-based page shown, at most the page count; the caller stores it from the `change` payload." },
            page_size: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false,
                doc: "Items per page; the caller stores it from the `change` payload together with `current_page`." },
            page_size_options: #{ schema: #{ type: "array", max_items: 32, items: #{ type: "integer", min: 1 } }, required: false, sensitive: false,
                "default": #{ type: "array", value: [] },
                doc: "Choices of the per-page Select, without duplicates and including `page_size`; empty hides the Select." },
            boundary_count: #{ schema: #{ type: "integer", min: 1, max: 3 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 1 }, doc: "Pages always shown at each end of the range." },
            sibling_count: #{ schema: #{ type: "integer", min: 0, max: 3 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 1 },
                doc: "Pages beside the current page. Near an end, spare slots show adjacent pages; the page/ellipsis slot count is stable." },
            compact: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Shows only Previous/Next beside the range and optional page-size field, omitting page numbers and ellipses. Independent of the compact density preference." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Disables page and page-size interaction without dimming the range text; closes the size popup." },
            show_summary: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Shows the visible item range and total, e.g. 21–30 of 128; an empty collection shows 0 of 0." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false,
                doc: "Size of the buttons and the per-page Select; `()` follows the `size` environment." },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the next `#{ current_page, page_size }` when a page, Previous, Next or a page size is picked." },
        },
        state: #{ fields: #{
            page_size_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
            page_size_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        } },
        events: #{ change: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
            current_page: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false,
                doc: "Next one-based page; 1 after a page-size change." },
            page_size: #{ schema: #{ type: "integer", min: 1 }, required: true, sensitive: false,
                doc: "Page size to show; differs from the prop only after a page-size change." },
        } }, doc: "Emitted when a page, Previous, Next or a page size is picked; the payload carries the next page and page size together." } },
        slots: #{},
        effects: ["page_size_state"],
        parts: ["root", "info", "summary", "controls", "previous", "previous_icon", "next", "next_icon", "page_slot", "page", "page_current", "ellipsis", "page_size_control", "page_size", "page_size_trigger", "page_size_indicator", "page_size_value", "page_size_label"],
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

fn page_window(total, current, boundary, sibling) {
    if total < 1 || current < 1 || current > total || boundary < 1 || boundary > 3 || sibling < 0 || sibling > 3 {
        throw "Pagination page_window requires a valid page and bounded window counts";
    }
    let count = 2 * boundary + 2 * sibling + 3;
    let result = [];
    if total <= count {
        for page in 1..(total + 1) { result.push(page); }
    } else if current <= boundary + sibling + 2 {
        for page in 1..(count - boundary) { result.push(page); }
        result.push(0);
        for offset in 0..boundary { result.push(total - boundary + 1 + offset); }
    } else if current >= total - boundary - sibling - 1 {
        for page in 1..(boundary + 1) { result.push(page); }
        result.push(0);
        let tail = count - boundary - 1;
        for offset in 0..tail { result.push(total - tail + 1 + offset); }
    } else {
        for page in 1..(boundary + 1) { result.push(page); }
        result.push(0);
        for offset in 0..(2 * sibling + 1) { result.push(current - sibling + offset); }
        result.push(0);
        for offset in 0..boundary { result.push(total - boundary + 1 + offset); }
    }
    result
}

// A current target consumes activation without proposing a page change.
fn current_page(ctx, event) {}
fn pagination_emit(ctx, state) { ctx.emit("change", state); }
fn pagination_size(ctx, value) {
    ctx.emit("change", #{ current_page: 1, page_size: parse_int(value) });
}
fn page_size_state(ctx, enabled) {
    if !enabled {
        if ctx.get_state("page_size_open") { ctx.set_state("page_size_open", false); }
        if ctx.get_state("page_size_query") != "" { ctx.set_state("page_size_query", ""); }
    }
}
fn page_size_cleanup(ctx, enabled) {}

fn page_button(ctx, page, pages, current, page_size, size, callback, disabled) {
    let selected = page == current;
    // The current page is a selection block, not a primary action.
    let current_style = if selected {
        style().background(theme_color("pagination.active")).text_color(theme_color("pagination.ink")).font_weight(500)
    } else { style() };
    let parts = if selected { ["page", "page_current"] } else { ["page"] };
    let base = style().typography("pagination").font_feature("tnum", 1);
    let node = button::Button(#{ key: `page-${page}`,
        text: `${page}`, size: size, variant: "ghost",
        hoverable: !selected && !disabled, disabled: disabled,
    }).with_key(`number-${page}`).with_style(ctx.component_style(parts, base, current_style));
    // Key by business page, not its changing window index. Native focus then
    // stays with that same target, without post-commit imperative focus work.
    node = if selected { node.on_click(Fn("current_page")).tab_stop(true).accessibility_current("page") }
    else if callback == () || disabled { node }
    else { node.on_click_value(Fn("pagination_emit"), #{ current_page: page, page_size: page_size }) };
    // Reserve the longest number at both weights in the same native grid cell.
    // This uses real font advances rather than a guessed 1ch pixel conversion.
    let children = [node.with_style(style().grid_cell(1,1))];
    for names in [["page"], ["page", "page_current"]] {
        children.push(text(`${pages}`).with_style(ctx.component_style(names, base)
            .grid_cell(1,1).invisible()));
    }
    let root = box(children).with_key(`page-slot-${page}`)
        .with_style(ctx.component_style("page_slot",style().grid_cols(1).overflow_priority(0)));
    if size == () { root } else { root.env(#{ size: size }) }
}

fn Pagination(props) { render_component("components/pagination", props) }
fn page_size_open(ctx, open) {
    ctx.set_state("page_size_open", open);
}
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
    let size = prop_or(props, "size", ());
    let compact = prop_or(props, "compact", false);
    let disabled = prop_or(props, "disabled", false);
    let callback = if "on_change" in props { props.on_change } else { () };
    effect("page_size_state", options.len > 0 && !disabled, Fn("page_size_state"), Fn("page_size_cleanup"));
    let controls = [];
    let previous_icon = icon::Icon(#{
        source: asset("app/icons/chevron_left"),
        rtl_source: asset("app/icons/chevron_right"), size: size,
    });
    let previous = icon_button::IconButton(#{
        label: ctx.t_or("pagination.previous", "Previous page"), icon: previous_icon,
        size: size, variant: "ghost", disabled: disabled || props.current_page == 1,
        part_styles: #{icon:ctx.component_style("previous_icon",style()
            .width(theme_length("metrics.pagination_icon")).height(theme_length("metrics.pagination_icon")))},
    }).with_style(ctx.component_style("previous", style()));
    controls.push(if callback == () || disabled || props.current_page == 1 { previous } else {
        previous.on_click_value(Fn("pagination_emit"), #{ current_page: props.current_page - 1, page_size: props.page_size })
    });
    let numbers = if compact { [] } else { page_window(
        pages, props.current_page,
        prop_or(props, "boundary_count", 1),
        prop_or(props, "sibling_count", 1)
    ) };
    for page in numbers {
        if page == 0 {
            controls.push(row([text("…")]).with_style(ctx.component_style("ellipsis",
            style().items_center().padding_x(theme_spacing("xs")).overflow_priority(0))));
        } else {
            controls.push(page_button(ctx, page, pages, props.current_page, props.page_size, size, callback, disabled));
        }
    }
    let next_icon = icon::Icon(#{
        source: asset("app/icons/chevron_right"),
        rtl_source: asset("app/icons/chevron_left"), size: size,
    });
    let next = icon_button::IconButton(#{
        label: ctx.t_or("pagination.next", "Next page"), icon: next_icon,
        size: size, variant: "ghost", disabled: disabled || props.current_page == pages,
        part_styles: #{icon:ctx.component_style("next_icon",style()
            .width(theme_length("metrics.pagination_icon")).height(theme_length("metrics.pagination_icon")))},
    }).with_style(ctx.component_style("next", style()));
    controls.push(if callback == () || disabled || props.current_page == pages { next } else {
        next.on_click_value(Fn("pagination_emit"), #{ current_page: props.current_page + 1, page_size: props.page_size })
    });

    let info = [];
    if options.len > 0 {
        let select_options = [];
        for option in options {
            select_options.push(#{ value: `${option}`, label: `${option}` });
        }
        info.push(row([
            text(ctx.t_or("pagination.per_page", "Items per page")).with_style(ctx.component_style("page_size_label", style())),
            select::Select(#{
                key: `${props.key}-page-size`, label: ctx.t_or("pagination.per_page", "Items per page"), options: select_options,
                value: `${props.page_size}`, clearable:false, open: !disabled && ctx.get_state("page_size_open"), disabled:disabled,
                part_styles:#{
                    trigger:ctx.component_style("page_size_trigger",style().min_height(px(0)).height(theme_length("metrics.pagination_h"))),
                    indicator:ctx.component_style("page_size_indicator",style()),
                    value:ctx.component_style("page_size_value",style()),
                },
                query: ctx.get_state("page_size_query"), size: size, width: "content",
                on_change: Fn("pagination_size"), on_open_change: Fn("page_size_open"),
                on_query_change: Fn("page_size_query")
            }).with_style(ctx.component_style("page_size", style()))
        ]).with_style(ctx.component_style("page_size_control", style()
            .gap(theme_spacing("sm")).items_center().flex_shrink(false)
            .text_color(theme_color("text_muted")))).disabled(disabled));
    }
    if prop_or(props, "show_summary", true) {
        let range = "0";
        if props.total_items > 0 {
            let first = (props.current_page - 1) * props.page_size + 1;
            // Avoid overflowing current_page * page_size on a partial last page.
            let remaining = props.total_items - first;
            let tail = if remaining < props.page_size - 1 { remaining } else { props.page_size - 1 };
            range = `${ctx.format_number(first)}–${ctx.format_number(first + tail)}`;
        }
        info.push(text([
            span(`${range} `),
            span(ctx.t_or("pagination.of", "of")).color(theme_color("pagination.muted")),
            span(` ${ctx.format_number(props.total_items)}`),
        ]).with_style(ctx.component_style("summary", style()
            .typography("body").font_feature("tnum", 1).whitespace_nowrap()
            .text_color(theme_color("text_muted")))));
    }
    let children = [];
    if info.len > 0 {
        children.push(row(info).with_style(ctx.component_style("info", style()
            .items_center().min_width(px(0)).gap(theme_length("space.group")))));
    }
    children.push(row(controls).with_style(ctx.component_style("controls",
        style().overflow_row().gap(theme_spacing("xxs")).items_center().justify_end()
            .max_width(relative(1.0)).min_width(px(0)).flex_shrink(true)
            .margin_start(auto()))).disabled(disabled));
    row(children).with_key(props.key)
        .with_style(ctx.component_style("root", style()
            .items_center().justify_between().gap(theme_length("space.group"))
            .typography("body")))
        .accessibility_role("navigation")
        .accessibility_label(props.label)
        .disabled(disabled)
}
