/* gpui-rhai
{
  "id": "patterns/navigation_stack", "export": "NavigationStack", "version": "0.2.2",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/button", "components/menu"], "capabilities": {},
  "assets": ["icons/chevron_left.svg", "icons/chevron_right.svg"],
  "tokens": ["text_primary", "text_muted", "spacing.xs", "space.related", "space.group", "typography.title"],
  "environment": ["size", "density", "corners"]
}
*/
// Controlled visit stack. Construction ownership stays with the caller (ADR 0019).
// Back proposes a retained depth; it neither loads data nor changes routes.
import "components/button" as button;
import "components/menu" as menu;

define_component(#{
    metadata: #{ id: "patterns/navigation_stack", "export": "NavigationStack", version: "0.2.2",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/button", "components/menu"], capabilities: #{},
        assets: ["icons/chevron_left.svg", "icons/chevron_right.svg"],
        tokens: ["text_primary", "text_muted", "spacing.xs", "space.related", "space.group", "typography.title"],
        environment: ["size", "density", "corners"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable stack instance identity; independent in each view and construction scope." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of this navigation region." },
            stack: #{ schema: #{ type: "array", max_items: 128, items: #{ type: "object", allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Unique non-empty visit key, not the business record id. The same record can appear repeatedly with different keys." },
                title: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "Page heading and full accessible breadcrumb label." },
                content: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                    doc: "Eager caller-owned content, retained while covered including native scroll, editor selection and undo. Hiding does not cancel business tasks or subscriptions; ctx.is_visible() supports an explicit application pause policy." },
            } } }, required: true, sensitive: false, doc: "Root-to-top controlled entries. Must contain at least one entry; removing an entry unmounts its native presentation resources." },
            header: #{ schema: #{ type: "string", allowed: ["back", "breadcrumb", "none"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "back" }, doc: "Back to the previous title, a responsive breadcrumb trail with middle-overflow menu, or no navigation header. The current page always has a focusable heading." },
            actions: #{ schema: #{ type: "array", items: #{ type: "node" } }, required: false, sensitive: false,
                "default": #{ type: "array", value: [] }, doc: "Trailing actions in the navigation header; omitted with header none." },
            max_depth: #{ schema: #{ type: "integer", min: 1, max: 128 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 8 }, doc: "Maximum retained entries. Exceeding the bound rejects the candidate instead of evicting a visit." },
            motion_key: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" }, doc: "Non-empty enables replayed push/pop continuity, mirrored in RTL. Host reduced-motion/none preference settles it; empty disables it." },
            back_key: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" }, doc: "Optional focus-scoped raw key such as cmd+[ or alt+left. Empty binds nothing; a Host can bind/dispatch NavigateBack or call navigate_back before its outer Back." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } }, required: false, sensitive: false,
                doc: "Overrides the inherited control size for header controls." },
            on_pop: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Receives #{ depth, source }; depth is the number of entries to retain. Sources: button, breadcrumb, keyboard, pointer, host. Without it Back is disabled." },
        },
        state: #{ fields: #{
            menu_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
            menu_active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
            depth: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
            direction: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 1 } },
            current_key: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        } },
        events: #{ pop: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
            depth: #{ schema: #{ type: "integer", min: 1, max: 128 }, required: true, sensitive: false, doc: "Number of entries proposed to keep." },
            source: #{ schema: #{ type: "string", allowed: ["button", "breadcrumb", "keyboard", "pointer", "host"] }, required: true, sensitive: false, doc: "Input source of the proposal." },
        } }, doc: "Controlled pop proposal. The caller chooses whether to truncate its stack." } },
        slots: #{}, effects: ["depth"],
        parts: ["root", "header", "back", "breadcrumb", "overflow", "actions", "pages", "page", "heading"],
    }, render: Fn("render_NavigationStack")
});

fn NavigationStack(props) { render_component("patterns/navigation_stack", props) }
fn nav_pop(ctx, payload) { ctx.emit("pop", payload); event_response().prevent_default().stop() }
fn nav_button_pop(payload, ctx, event) { nav_pop(ctx, payload) }
fn nav_pointer(depth, ctx, event) {
    if event.button != "back" { return propagate(); }
    nav_pop(ctx, #{ depth: depth, source: "pointer" })
}
fn nav_menu_open(ctx, open) { ctx.set_state("menu_open", open); }
fn nav_menu_active(ctx, value) { ctx.set_state("menu_active", value); }
fn nav_menu_pop(ctx, value) { ctx.emit("pop", #{ depth: parse_int(value), source: "breadcrumb" }); }
fn nav_remember(ctx, placement) {
    let depth = placement.depth;
    let previous = ctx.get_state("depth");
    if previous != depth {
        ctx.set_state("direction", if depth < previous { -1 } else { 1 });
        ctx.set_state("depth", depth);
    }
    if previous == depth && ctx.get_state("current_key") != placement.top { ctx.set_state("direction", 1); }
    ctx.set_state("current_key", placement.top);
    ctx.set_state("menu_open", false);
}
fn nav_cleanup(ctx, value) { () }
fn nav_crumb(ctx, props, index) {
    button::Button(#{ key: `crumb:${props.stack[index].key}`, text: props.stack[index].title,
        variant: "ghost", on_click: if "on_pop" in props && props.on_pop != () { Fn("nav_button_pop").curry(#{ depth: index + 1, source: "breadcrumb" }) } else { () } })
        .with_style(ctx.component_style("breadcrumb", style().min_width(px(0))))
}
fn nav_header(ctx, props, header, enabled, rtl) {
    let depth = props.stack.len;
    let header_ref = element_ref("header");
    let trail = [];
    if header == "back" && depth > 1 {
        let chevron = image(asset(if rtl { "icons/chevron_right" } else { "icons/chevron_left" }));
        trail.push(button::Button(#{ key: "back", text: props.stack[depth - 2].title,
            icon: chevron, variant: "ghost", disabled: !enabled,
            on_click: if enabled { Fn("nav_button_pop").curry(#{ depth: depth - 1, source: "button" }) } else { () } })
            .accessibility_label(`Back to ${props.stack[depth - 2].title}`)
            .with_style(ctx.component_style("back", style().min_width(px(0)))));
    } else if header == "breadcrumb" {
        let bounds = ctx.element_bounds(header_ref);
        let narrow = bounds == () || bounds.layout.width < 640.0;
        let overflow = [];
        if depth > 4 || (narrow && depth > 3) {
            for index in 1..(depth - 2) {
                overflow.push(#{ kind: "item", value: `${index + 1}`, label: props.stack[index].title, disabled: !enabled });
            }
        }
        for index in 0..depth {
            if overflow.len > 0 && index > 1 && index < depth - 2 { continue; }
            if index > 0 { trail.push(text(if rtl { "‹" } else { "›" }).with_style(style().text_color(theme_color("text_muted")))); }
            if index == depth - 1 {
                trail.push(text(props.stack[index].title).with_key("current").accessibility_current("page")
                    .accessibility_label(props.stack[index].title).with_style(style().min_width(px(0)).text_ellipsis().whitespace_nowrap()));
            } else if overflow.len > 0 && index == 1 {
                let trigger = button::Button(#{ key: "overflow-button", text: "…", variant: "ghost", disabled: !enabled });
                trail.push(menu::Menu(#{ key: "overflow-menu", label: "Earlier pages", trigger: trigger,
                    open: enabled && ctx.get_state("menu_open"), active_value: ctx.get_state("menu_active"), items: overflow,
                    on_open_change: Fn("nav_menu_open"), on_active_change: Fn("nav_menu_active"), on_action: Fn("nav_menu_pop") })
                    .with_style(ctx.component_style("overflow", style())));
            } else { trail.push(nav_crumb(ctx, props, index)); }
        }
    }
    let actions = row(props.actions).with_key("actions").with_style(ctx.component_style("actions", style().items_center().flex_wrap().gap(theme_length("space.related"))));
    row([row(trail).with_key("trail").with_style(style().items_center().gap(theme_spacing("xs")).min_width(px(0)).flex_grow().overflow_hidden()), actions])
        .with_key("header").with_ref(header_ref)
        .with_style(ctx.component_style("header", style().items_center().justify_between().flex_wrap().min_width(px(0)).gap(theme_length("space.related"))))
}
fn render_NavigationStack(ctx, props) {
    if props.stack.len == 0 || props.stack.len > props.max_depth { throw "NavigationStack requires 1..max_depth entries"; }
    if props.motion_key.len > 128 { throw "NavigationStack motion_key must be at most 128 characters"; }
    let keys = [];
    for entry in props.stack {
        if entry.key == "" || keys.contains(entry.key) { throw "NavigationStack entry keys must be non-empty and unique per visit"; }
        keys.push(entry.key);
    }
    let enabled = props.stack.len > 1 && "on_pop" in props && props.on_pop != ();
    let rtl = ctx.text_direction() == "rtl";
    let previous = ctx.get_state("depth");
    let top = props.stack[props.stack.len - 1].key;
    let direction = if props.stack.len < previous { -1 } else if props.stack.len > previous || top != ctx.get_state("current_key") { 1 } else { ctx.get_state("direction") };
    effect("depth", #{ depth: props.stack.len, top: top, header: props.header }, Fn("nav_remember"), Fn("nav_cleanup"));
    let pages = [];
    for index in 0..props.stack.len {
        let entry = props.stack[index];
        let heading = text(entry.title).with_key("heading").tab_stop(false)
            .accessibility_role("heading").accessibility_level(2).accessibility_label(entry.title)
            .with_style(ctx.component_style("heading", style().typography("title").text_color(theme_color("text_primary"))));
        let page = column([heading, entry.content]).with_key(entry.key).retained_page(index == props.stack.len - 1).tab_stop(false)
            .accessibility_role("region").accessibility_label(entry.title)
            .with_style(ctx.component_style("page", style().gap(theme_length("space.related")).min_width(px(0)).min_height(px(0))));
        if props.motion_key != "" {
            page = page.motion(motion_transition("translate_x", (if rtl { -1.0 } else { 1.0 }) * direction * 16.0, 0.0,
                #{ duration_ms: ctx.motion_duration("fast"), easing: ctx.motion_easing("standard"), intent: "feedback" }))
                .motion_replay_key(`${props.motion_key}:${props.stack[props.stack.len - 1].key}`);
        }
        pages.push(page);
    }
    let children = [];
    if props.header != "none" { children.push(nav_header(ctx, props, props.header, enabled, rtl)); }
    children.push(column(pages).with_key("pages").with_style(ctx.component_style("pages", style().min_width(px(0)).min_height(px(0)))));
    let root = column(children).with_key(props.key).navigation_scope(props.stack.len).tab_stop(false)
        .accessibility_role("group").accessibility_label(props.label)
        .with_style(ctx.component_style("root", style().gap(theme_length("space.group")).min_width(px(0)).min_height(px(0))));
    if enabled {
        root = root.on("navigation_back", Fn("nav_pop"))
            .on("pointer_down", Fn("nav_pointer").curry(props.stack.len - 1));
        if props.back_key != "" { root = root.on_key_value(props.back_key, Fn("nav_pop"), #{ depth: props.stack.len - 1, source: "keyboard" }); }
    }
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
