// The gpui-rhai Gallery: the component catalog and the acceptance application of
// docs/design/composition.md. It is assembled from patterns/app_shell and the L2
// layouts. Audit findings stay visible, including authored palette risks.

import "patterns/app_shell" as app_shell;
import "layouts/region" as region;
import "components/tree" as tree;
import "components/toggle_group" as toggle_group;
import "components/select" as select;
import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/command_dialog" as command_dialog;
import "components/code_viewer" as code_viewer;
import "components/dialog" as dialog;
import "components/split_pane" as split_pane;
import "gallery/pages/foundations" as foundations;
import "gallery/pages/markers" as markers;
import "gallery/pages/fields" as fields;
import "gallery/pages/lists" as lists;
import "gallery/pages/browsers" as browsers;
import "gallery/pages/inputs_and_choices" as inputs_and_choices;
import "gallery/pages/navigation" as navigation_pages;
import "gallery/pages/temporal" as temporal;
import "gallery/pages/scrolling" as scrolling;
import "gallery/pages/markdown" as markdown_pages;
import "gallery/pages/messaging" as messaging;
import "gallery/pages/overlays" as overlays;
import "gallery/pages/containers" as containers;
import "gallery/pages/display" as display;
import "gallery/pages/interaction" as interaction;
import "gallery/pages/composition" as composition;
import "gallery/pages/scenes" as scenes;
import "gallery/pages/effects" as effects;
import "gallery/pages/charts" as chart_pages;

fn state_schema() {
    #{ fields: #{
        page: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "button" } },
        density: #{ schema: #{ type: "string", allowed: ["comfortable", "compact"] },
            "default": #{ type: "string", value: "comfortable" } },
        scale: #{ schema: #{ type: "string", allowed: ["100", "110", "125", "150", "175", "200"] },
            "default": #{ type: "string", value: "100" } },
        locale: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "en" } },
        audit: #{ schema: #{ type: "integer", min: -1 }, "default": #{ type: "integer", value: -1 } },
        audit_report: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "The Host has not published an audit yet." } },
        audit_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        palette_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        palette_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        palette_active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        locale_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        theme_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        titlebar_inset: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
        nav_active: #{ schema: #{ type: "optional", value: #{ type: "string" } }, "default": #{ type: "null" } },
        nav_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        values: #{ schema: #{ type: "map", values: #{ type: "ui_value" } }, "default": #{ type: "map", value: #{} } },
    } }
}

// The Host replaces this map with the launch selection (page, density, locale) and, when
// the window has no platform title bar, the room its window buttons need. The launch
// theme is the Host's primary theme.
fn launch() { #{ page: "button", density: "comfortable", locale: "en", titlebar_inset: 0 } }

fn init(ctx) {
    let start = launch();
    ctx.set_state("page", start.page);
    ctx.set_state("density", start.density);
    ctx.set_state("titlebar_inset", start.titlebar_inset);
    if start.locale != "en" {
        ctx.set_state("locale", start.locale);
        ctx.set_locale(start.locale);
    }
    ctx.register_action("gallery.palette", Fn("open_palette"));
    ctx.register_action("gallery.toggle_source", Fn("toggle_source"));
    ctx.register_action("gallery.toggle_density", Fn("toggle_density"));
    ctx.register_action("gallery.toggle_mode", Fn("toggle_mode"));
    ctx.register_action("gallery.browse_pages", Fn("open_navigation"));
    ctx.register_action("gallery.go", Fn("go_to"));
    ctx.register_action("gallery.set_density", Fn("set_density_action"));
    ctx.register_action("gallery.set_scale", Fn("set_scale_action"));
    ctx.register_action("gallery.audit", Fn("set_audit"));
}

// ---- handlers -------------------------------------------------------------------------

fn noop(ctx, payload) {}
fn gallery_composer_submit(ctx, value) { inputs_and_choices::composer_send(ctx, value); }
fn gallery_composer_stop(ctx, payload) { inputs_and_choices::composer_stop(ctx, payload); }
fn gallery_inspect_region(value, ctx, payload) { inputs_and_choices::inspect_region(value, ctx, payload); }
fn set_value(key, ctx, value) {
    let values = ctx.get_state("values");
    values[key] = value;
    ctx.set_state("values", values);
}
fn catalog_use(ctx,selected){
    let values=ctx.get_state("values");values["catalog.draft"]=selected.text;values["catalog.used"]=selected.id;
    ctx.set_state("values",values);
}
fn catalog_toggle(ctx,proposal){
    let values=ctx.get_state("values");
    if proposal.id=="archive/7"{
        values["catalog.connection_note"]="Host declined the archive request.";
    }else{
        let current=if "catalog.connections" in values{values["catalog.connections"]}else{["notes/7"]};
        let retained=[];for id in current{if id!=proposal.id{retained.push(id);}}
        if proposal.connected{retained.push(proposal.id);}
        values["catalog.connections"]=retained;
        values["catalog.connection_note"]="Host accepted the integration change.";
    }
    ctx.set_state("values",values);
}
fn gallery_react(ctx,value){
    let values=ctx.get_state("values");
    let reactions=if "conversation.reactions" in values{values["conversation.reactions"]}else{#{}};
    reactions[value.id]=value;set_value("conversation.reactions",ctx,reactions);
}
fn gallery_regenerate(ctx,version){
    let values=ctx.get_state("values");
    let n=if "conversation.versions" in values{values["conversation.versions"]}else{2};
    if n<1000{
        values["conversation.versions"]=n+1;values["conversation.version"]=n;
        ctx.set_state("values",values);
    }
}
fn gallery_speak(ctx,request){set_value("conversation.speaking",ctx,request.playing);}
fn gallery_feedback(ctx,feedback){set_value("conversation.feedback",ctx,if feedback.reason!=(){"Feedback received"}
    else if feedback.kind==(){"Vote removed"}else if feedback.kind=="up"{"Upvote recorded"}else{"Downvote recorded"});}
fn chart_select(ctx, payload) { set_value("charts.selection",ctx,[payload.datum_key]); }
fn chart_brush(ctx, payload) { set_value("charts.selection",ctx,payload.keys); }
fn set_checked(key, ctx, payload) { set_value(key, ctx, payload.checked); }
// Store a fixed value under `key`, ignoring the event payload (button presses).
fn put_value(key, value, ctx, payload) { set_value(key, ctx, value); }
// Remove the item whose `id` is the payload from the list under `key` (Toast dismissals).
fn remove_item(key, ctx, id) {
    let values = ctx.get_state("values");
    let kept = [];
    if key in values { for item in values[key] { if item.id != id { kept.push(item); } } }
    values[key] = kept;
    ctx.set_state("values", values);
}
// Not `index_of`: an entry function wins over the built-in string method of the
// same arity inside imported modules, which would break Command's fuzzy search.
fn order_position(values, key) {
    for index in 0..values.len { if values[index] == key { return index; } }
    -1
}
// Sortable reorder: move the source next to its anchor in the stored order.
fn reorder(key, ctx, value) {
    let values = ctx.get_state("values");
    if !(key in values) { return; }
    let order = values[key];
    let source = order_position(order, value.source_key);
    if source < 0 { return; }
    let moved = order.remove(source);
    let anchor = order_position(order, value.anchor_key);
    if anchor < 0 { return; }
    order.insert(if value.placement == "after" { anchor + 1 } else { anchor }, moved);
    set_value(key, ctx, order);
}
fn drop_to(key, ctx, value) { set_value(key, ctx, value.target_id); }
fn go_to(ctx, page) {
    if type_of(page) == "string" && page != "" {
        ctx.set_state("page", page);
        ctx.set_state("nav_open", false);
    }
}
fn open_palette(ctx, payload) {
    ctx.set_state("nav_open", false);
    ctx.set_state("locale_open", false);
    ctx.set_state("theme_open", false);
    ctx.set_state("palette_query", "");
    ctx.set_state("palette_open", true);
}
fn palette_open(ctx, open) { ctx.set_state("palette_open", open); }
fn palette_query(ctx, query) { ctx.set_state("palette_query", query); }
fn palette_active(ctx, value) { ctx.set_state("palette_active", value); }
fn palette_run(ctx, value) {
    if value.starts_with("page:") { go_to(ctx, value.sub_string(5)); }
    if value.starts_with("scale:") { set_scale_action(ctx, value.sub_string(6)); }
    if value.starts_with("theme:") { choose_theme(ctx, value.sub_string(6)); }
    if value.starts_with("locale:") { choose_locale(ctx, value.sub_string(7)); }
    ctx.set_state("palette_open", false);
}
fn toggle_source(ctx, payload) { set_value("gallery.source_open", ctx, !source_open(ctx.get_state("values"))); }
fn source_open(values) { "gallery.source_open" in values && values["gallery.source_open"] }
fn source_resized(ctx, size) { set_value("gallery.source_size", ctx, size); }
fn toggle_density(ctx, payload) {
    ctx.set_state("density", if ctx.get_state("density") == "compact" { "comfortable" } else { "compact" });
}
fn set_density_action(ctx, density) { if density == "compact" || density == "comfortable" { ctx.set_state("density", density); } }
fn set_density(ctx, values) { if values.len > 0 { ctx.set_state("density", values[0]); } }
fn scale_steps() { ["100", "110", "125", "150", "175", "200"] }
fn set_scale_action(ctx, scale) { if scale in scale_steps() { ctx.set_state("scale", scale); } }
// Light and dark are variants of one family; families with one mode stay as they are.
fn toggle_mode(ctx, payload) {
    let current = ctx.theme_variant();
    for variant in ctx.theme_variants() {
        if variant.family == current.family && variant.mode != current.mode {
            ctx.set_theme(variant.family, variant.name);
            return;
        }
    }
}
fn choose_theme(ctx, value) {
    if value == () { return; }
    for variant in ctx.theme_variants() {
        if theme_key(variant)==value {ctx.set_theme(variant.family,variant.name);return;}
    }
}
fn theme_key(variant) { `${variant.family.len}:${variant.family}${variant.name}` }
fn theme_open(ctx, open) { ctx.set_state("theme_open", open); }
fn choose_locale(ctx, value) {
    if value == () { return; }
    ctx.set_state("locale", value);
    ctx.set_locale(value);
}
fn gallery_locales() {
    [
        #{ value: "en", label: "English", search: "English" },
        #{ value: "zh-CN", label: "简体中文", search: "Simplified Chinese" },
        #{ value: "ar", label: "العربية", search: "Arabic" },
    ]
}
fn locale_open(ctx, open) { ctx.set_state("locale_open", open); }
fn set_audit(ctx, report) { ctx.set_state("audit", report.count); ctx.set_state("audit_report",report.report); }
fn show_audit(ctx, payload) { ctx.set_state("audit_open",true); }
fn close_audit(ctx, payload) { ctx.set_state("audit_open",false); }
fn audit_open(ctx, open) { ctx.set_state("audit_open",open); }
fn navigation_uses_drawer(class, scale) {
    class == "compact" || (class == "regular" && scale in ["150", "175", "200"])
}
fn open_navigation(ctx, payload) {
    if !navigation_uses_drawer(ctx.viewport_class(), ctx.get_state("scale")) { return; }
    ctx.set_state("palette_open", false);
    ctx.set_state("nav_active", ctx.get_state("page"));
    ctx.set_state("nav_open", true);
}
fn close_navigation(ctx, payload) { ctx.set_state("nav_open", false); }
fn nav_select(ctx, keys) {
    if keys.len > 0 && !keys[0].starts_with("group:") { go_to(ctx, keys[0]); }
}
fn nav_active(ctx, key) { ctx.set_state("nav_active", key); }
fn nav_expand(ctx, keys) { set_value("gallery.nav_expanded", ctx, keys); }

// ---- pages ----------------------------------------------------------------------------

// The sidebar groups in order: the chapters of docs/design/atoms.md, then composition,
// scenes and the motion pack. Every page names one of them in its `group`. Page modules
// only organize the source, so a new module adds pages to these groups, never a group.
fn groups() {
    ["Foundations", "Markers", "Fields", "Lists", "Navigation", "Overlays", "Containers",
        "Display", "Interaction", "Layouts and patterns", "Charts", "Scenes", "Effects"]
}

// The page modules, in the order their pages join a group.
fn modules() {
    [
        #{ id: "gallery/pages/foundations", pages: foundations::pages() },
        #{ id: "gallery/pages/markers", pages: markers::pages() },
        #{ id: "gallery/pages/fields", pages: fields::pages() },
        #{ id: "gallery/pages/lists", pages: lists::pages() },
        #{ id: "gallery/pages/browsers", pages: browsers::pages() },
        #{ id: "gallery/pages/navigation", pages: navigation_pages::pages() },
        #{ id: "gallery/pages/overlays", pages: overlays::pages() },
        #{ id: "gallery/pages/containers", pages: containers::pages() },
        #{ id: "gallery/pages/scrolling", pages: scrolling::pages() },
        #{ id: "gallery/pages/display", pages: display::pages() },
        #{ id: "gallery/pages/markdown", pages: markdown_pages::pages() },
        #{ id: "gallery/pages/interaction", pages: interaction::pages() },
        #{ id: "gallery/pages/composition", pages: composition::pages() },
        #{ id: "gallery/pages/inputs_and_choices", pages: inputs_and_choices::pages() },
        #{ id: "gallery/pages/temporal", pages: temporal::pages() },
        #{ id: "gallery/pages/scenes", pages: scenes::pages() },
        #{ id: "gallery/pages/messaging", pages: messaging::pages() },
        #{ id: "gallery/pages/effects", pages: effects::pages() },
        #{ id: "gallery/pages/charts", pages: chart_pages::pages() },
    ]
}

fn sections() {
    let titles = groups();
    let sections = [];
    for title in titles { sections.push(#{ title: title, pages: [] }); }
    for source in modules() {
        for page in source.pages {
            let at = if "group" in page { order_position(titles, page.group) } else { -1 };
            if at < 0 { throw `Gallery page ${page.id} names no group of groups()`; }
            sections[at].pages.push(page);
        }
    }
    sections
}

fn all_pages() {
    let pages = [];
    for section in sections() { for page in section.pages { pages.push(page); } }
    pages
}

fn page_title(id) {
    for page in all_pages() { if page.id == id { return page.title; } }
    id
}

// The module whose source the inspector shows for a page.
fn page_module(id) {
    for source in modules() {
        for page in source.pages { if page.id == id { return source.id; } }
    }
    "gallery/pages/markers"
}

fn page_view(ctx, h, v, id) {
    let node = foundations::view(ctx, h, v, id);
    if node == () { node = markers::view(ctx, h, v, id); }
    if node == () { node = fields::view(ctx, h, v, id); }
    if node == () { node = lists::view(ctx, h, v, id); }
    if node == () { node = browsers::view(ctx, h, v, id); }
    if node == () { node = inputs_and_choices::view(ctx, h, v, id); }
    if node == () { node = navigation_pages::view(ctx, h, v, id); }
    if node == () { node = temporal::view(ctx, h, v, id); }
    if node == () { node = scrolling::view(ctx, h, v, id); }
    if node == () { node = markdown_pages::view(ctx, h, v, id); }
    if node == () { node = messaging::view(ctx, h, v, id); }
    if node == () { node = overlays::view(ctx, h, v, id); }
    if node == () { node = containers::view(ctx, h, v, id); }
    if node == () { node = display::view(ctx, h, v, id); }
    if node == () { node = interaction::view(ctx, h, v, id); }
    if node == () { node = composition::view(ctx, h, v, id); }
    if node == () { node = scenes::view(ctx, h, v, id); }
    if node == () { node = effects::view(ctx, h, v, id); }
    if node == () { node = chart_pages::view(ctx, h, v, id); }
    if node == () { node = markers::view(ctx, h, v, "button"); }
    node
}

// ---- shell ----------------------------------------------------------------------------

fn handlers() {
    #{ set: Fn("set_value"), put: Fn("put_value"), checked: Fn("set_checked"), reorder: Fn("reorder"), drop: Fn("drop_to"),
        remove: Fn("remove_item"), noop: Fn("noop"), navigate: Fn("go_to"),
        composer_submit: Fn("gallery_composer_submit"), composer_stop: Fn("gallery_composer_stop"),
        react:Fn("gallery_react"),regenerate:Fn("gallery_regenerate"),speak:Fn("gallery_speak"),feedback:Fn("gallery_feedback"),
        catalog_use:Fn("catalog_use"),catalog_toggle:Fn("catalog_toggle"),
        inspect_region: Fn("gallery_inspect_region"),chart_select:Fn("chart_select"),chart_brush:Fn("chart_brush") }
}

fn navigation(ctx, v, page) {
    let items = [];
    let expanded = [];
    for section in sections() {
        let group = `group:${section.title}`;
        items.push(#{ key: group, label: section.title });
        expanded.push(group);
        for entry in section.pages { items.push(#{ key: entry.id, parent: group, label: entry.title }); }
    }
    let open = if "gallery.nav_expanded" in v { v["gallery.nav_expanded"] } else { expanded };
    let active = ctx.get_state("nav_active");
    let list = tree::Tree(#{ key: "gallery-nav", label: "Gallery pages", items: items, expanded: open,
        selected_keys: [page], active_key: if active == () { page } else { active }, fill_height: true,
        on_selection_change: Fn("nav_select"), on_active_change: Fn("nav_active"),
        on_expanded_change: Fn("nav_expand") });
    region::Region(#{ label: "Navigation", body: list, inset: false })
}

// Every control fits the title bar of a wide window. Narrower, the Density toggle gives
// way first (the palette still sets it and the status bar shows it), and
// in a compact window the theme picker too (the palette still selects any loaded theme), so the
// title stays readable and no control runs past the window's edge.
fn title_controls(ctx, class, drawer_layout) {
    let locale = select::Select(#{ key: "gallery-locale", label: "Locale", value: ctx.get_state("locale"),
        open: ctx.get_state("locale_open"), query: "", size: "sm", width: theme_length("metrics.label_column") * 0.9375,
        options: gallery_locales().map(|locale| #{ value: locale.value, label: locale.label }),
        on_change: Fn("choose_locale"), on_open_change: Fn("locale_open") });
    if drawer_layout {
        let commands = icon_button::IconButton(#{ key: "gallery-palette", label: "Commands",
            icon: image_source(asset("app/icons/search")), variant: "ghost", size: "sm", action: "gallery.palette" });
        return [row([locale.with_style(style().overflow_priority(0)), commands])
            .with_key("compact-title-controls")
            .with_style(style().overflow_row().max_width(relative(1.0)).flex_shrink(true)
                .gap(theme_length("space.related")).justify_end())];
    }
    let controls = [];
    if class == "wide" {
        controls.push(toggle_group::ToggleGroup(#{ key: "gallery-density", label: "Density",
            values: [ctx.get_state("density")], allow_empty: false, size: "sm",
            items: [#{ value: "comfortable", label: "Comfortable" }, #{ value: "compact", label: "Compact" }],
            on_change: Fn("set_density") }));
    }
    let current = ctx.theme_variant();
    controls.push(select::Select(#{ key: "gallery-theme", label: "Theme", value: theme_key(current),
        open: ctx.get_state("theme_open"), query: "", size: "sm", width: theme_length("metrics.label_column") * 1.375,
        options: theme_options(ctx),
        on_change: Fn("choose_theme"), on_open_change: Fn("theme_open") }));
    controls.push(locale);
    controls.push(button::Button(#{ key: "gallery-palette", text: "Commands", variant: "ghost", size: "sm",
        action: "gallery.palette" }));
    controls
}

// One option per loaded variant; a family with several variants names the variant.
fn theme_options(ctx) {
    let variants = ctx.theme_variants();
    let options = [];
    for variant in variants {
        let siblings = variants.filter(|other| other.family == variant.family).len;
        options.push(#{ value: theme_key(variant),
            label: if siblings > 1 { `${variant.family} · ${variant.name}` } else { variant.family } });
    }
    options
}

fn palette_items(ctx) {
    let items = [];
    for section in sections() {
        for entry in section.pages {
            items.push(#{ value: `page:${entry.id}`, label: entry.title, group: section.title });
        }
    }
    items.push(#{ value: "action:density", label: "Toggle density", group: "Gallery", action: "gallery.toggle_density" });
    items.push(#{ value: "action:mode", label: "Toggle light and dark", group: "Gallery", action: "gallery.toggle_mode" });
    for step in scale_steps() {
        items.push(#{ value: `scale:${step}`, label: `Reading scale ${step}%`, group: "Gallery" });
    }
    items.push(#{ value: "action:source", label: "Toggle source", group: "Gallery", action: "gallery.toggle_source" });
    for variant in ctx.theme_variants() {
        items.push(#{ value: `theme:${theme_key(variant)}`, label: `Theme: ${theme_label(variant)}`, group: "Themes" });
    }
    for locale in gallery_locales() {
        items.push(#{ value: `locale:${locale.value}`, label: `Language: ${locale.label}`,
            keywords: [locale.search], group: "Languages" });
    }
    items
}

fn source_inspector(ctx, page) {
    let source_id = page_module(page);
    let document = source_id;
    document.replace("/", ".");
    let viewer = code_viewer::CodeViewer(#{ key: "gallery-source", label: `${source_id} source`,
        file_name: `${source_id}.rhai`, language: "rhai",
        source: ctx.get_native_text_document(`gallery.source.${document}`) });
    region::Region(#{ label: "Source", title: "Source", body: viewer, bleed: true })
}

// A pane's content fills it: SplitPane's panes are boxes with no direction of their own.
fn pane_fill(node) {
    column([node]).with_style(style().flex_col().width(relative(1.0)).height(relative(1.0))
        .min_width(px(0)).min_height(px(0)))
}

// The divider shows a bar only while the pointer is on it (muted) or drags it (accent,
// longer); at rest and under keyboard focus the line and the ink bar are enough.
// The page and its source side by side. The split is always there and collapses the source
// while it is hidden, so showing the source never remounts the page.
fn main_view(ctx, h, v, page) {
    let open = source_open(v);
    split_pane::SplitPane(#{ key: "gallery-split", label: "Resize source",
        size: if "gallery.source_size" in v { v["gallery.source_size"] } else { 0.6 },
        min_start: 360.0, min_end: 240.0, end_collapsed: !open,
        start: pane_fill(page_view(ctx, h, v, page)),
        end: if open { pane_fill(source_inspector(ctx, page))
            .with_style(style().background(theme_color("surface_raised"))) } else { box([]) },
        on_resize: Fn("source_resized") })
}

fn theme_label(variant) { `${variant.family} ${variant.name}` }

fn status_items(ctx, drawer_layout) {
    let label = |name, value| text(`${name} ${value}`);
    let density = label.call("Density", ctx.get_state("density"));
    let theme = label.call("Theme", theme_label(ctx.theme_variant()));
    let locale = label.call("Locale", ctx.get_state("locale"));
    let scale = ctx.get_state("scale");
    let start = [density, theme, locale];
    // The reading scale shows once it leaves 100%.
    if scale != "100" {
        let reading = label.call("Scale", `${scale}%`);
        if drawer_layout {
            // In the compact enlarged bar, every optional label disappears
            // whole instead of painting the first few letters under a clip.
            start = [row([theme.with_style(style().overflow_priority(2)),
                reading.with_style(style().overflow_priority(3)), density.with_style(style().overflow_priority(1)),
                locale.with_style(style().overflow_priority(0))]).with_key("compact-status")
                .with_style(style().overflow_row().max_width(relative(1.0))
                    .flex_shrink(true).gap(theme_spacing("md")))];
        } else {
            start.push(reading);
        }
    }
    let audit = ctx.get_state("audit");
    #{
        start: start,
        end: [text(if audit < 0 { "Audit pending" } else { `Audit ${audit} findings` })
            .with_key("audit-report-link")
            .with_style(style().text_color(theme_color("text_primary"))
                .focus(style().focus_ring(px(1),px(1),theme_color("focus_ring"))))
            .on_click(Fn("show_audit"))
            .accessibility_role("button").accessibility_label("Open audit report")
            .accessibility_value(if audit < 0 { "pending" } else { audit }),
            text("⌘K")],
    }
}

// Without a platform title bar, the Gallery's own bar moves the window and starts after
// the window buttons.
fn title_bar(ctx, page, class, drawer_layout) {
    let bar = #{ title: "GPUI RHAI", subtitle: page_title(page), end: title_controls(ctx, class, drawer_layout) };
    if drawer_layout {
        // Give the optional end cells a bounded lane to measure against.
        bar.part_styles = #{end: style().min_width(px(0)).overflow_hidden()};
        let edge = if ctx.text_direction() == "rtl" { 10 } else { 6 };
        let glyph = svg(`<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.4'><rect x='2' y='3' width='12' height='10' rx='1.5'/><path d='M${edge} 3v10'/></svg>`);
        bar.start = [icon_button::IconButton(#{key:"gallery-browse-pages", label:"Browse pages",
            variant:"ghost", size:"sm", icon:glyph, action:"gallery.browse_pages"})
            .with_style(style().flex_shrink(false))
            .accessibility_expanded(ctx.get_state("nav_open"))];
    }
    let inset = ctx.get_state("titlebar_inset");
    if inset > 0 {
        bar.inset_start = inset;
        bar.window_drag = true;
    }
    bar
}

fn view(ctx) {
    let page = ctx.get_state("page");
    let v = ctx.get_state("values");
    let h = handlers();
    let class = ctx.viewport_class();
    let drawer_layout = navigation_uses_drawer(class, ctx.get_state("scale"));
    let sidebar_mode = if !drawer_layout { "dock" } else if ctx.get_state("nav_open") { "overlay" } else { "hidden" };
    let palette = command_dialog::CommandDialog(#{ key: "gallery-palette-dialog", label: "Go to",
        open: ctx.get_state("palette_open"), query: ctx.get_state("palette_query"),
        active_value: ctx.get_state("palette_active"), items: palette_items(ctx),
        on_open_change: Fn("palette_open"), on_query_change: Fn("palette_query"),
        on_active_change: Fn("palette_active"), on_action: Fn("palette_run") });
    let audit = dialog::Dialog(#{key:"gallery-audit",open:ctx.get_state("audit_open"),title:"Audit findings",
        width:px(760),on_open_change:Fn("audit_open"),
        content:code_viewer::CodeViewer(#{key:"audit-evidence",label:"Audit evidence",language:"text",
            source:ctx.get_state("audit_report"),height:"content",max_height:px(320),wrap:"viewport"}),
        actions:[button::Button(#{key:"close-audit",text:"Close",variant:"secondary",size:"sm",on_click:Fn("close_audit")})]});
    let shell = #{ key: "gallery", label: "gpui-rhai Gallery",
        title_bar: title_bar(ctx, page, class, drawer_layout),
        sidebar: navigation(ctx, v, page), main: main_view(ctx, h, v, page),
        sidebar_mode: sidebar_mode, on_sidebar_dismiss: Fn("close_navigation"),
        status: status_items(ctx, drawer_layout), overlays: [palette,audit] };
    app_shell::AppShell(shell).env(#{ density: ctx.get_state("density"), scale: ctx.get_state("scale") })
}
