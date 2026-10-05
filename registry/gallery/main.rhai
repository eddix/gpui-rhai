// The gpui-rhai Gallery: the component catalog and the acceptance application of
// docs/design/composition.md. It is assembled from patterns/app_shell and the L2
// layouts; every page must pass the productivity audit in both densities.

import "patterns/app_shell" as app_shell;
import "layouts/region" as region;
import "components/tree" as tree;
import "components/toggle_group" as toggle_group;
import "components/select" as select;
import "components/button" as button;
import "components/command_dialog" as command_dialog;
import "components/code_viewer" as code_viewer;
import "gallery/pages/foundations" as foundations;
import "gallery/pages/markers" as markers;
import "gallery/pages/fields" as fields;
import "gallery/pages/lists" as lists;
import "gallery/pages/overlays" as overlays;
import "gallery/pages/containers" as containers;
import "gallery/pages/display" as display;
import "gallery/pages/interaction" as interaction;
import "gallery/pages/composition" as composition;
import "gallery/pages/scenes" as scenes;
import "gallery/pages/effects" as effects;

fn state_schema() {
    #{ fields: #{
        page: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "button" } },
        density: #{ schema: #{ type: "string", allowed: ["comfortable", "compact"] },
            "default": #{ type: "string", value: "comfortable" } },
        locale: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "en" } },
        audit: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
        palette_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        palette_query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        palette_active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        locale_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        theme_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        titlebar_inset: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
        nav_active: #{ schema: #{ type: "optional", value: #{ type: "string" } }, "default": #{ type: "null" } },
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
    ctx.register_action("gallery.go", Fn("go_to"));
    ctx.register_action("gallery.set_density", Fn("set_density_action"));
    ctx.register_action("gallery.audit", Fn("set_audit"));
}

// ---- handlers -------------------------------------------------------------------------

fn noop(ctx, payload) {}
fn set_value(key, ctx, value) {
    let values = ctx.get_state("values");
    values[key] = value;
    ctx.set_state("values", values);
}
fn set_checked(key, ctx, payload) { set_value(key, ctx, payload.checked); }
// Store a fixed value under `key`, ignoring the event payload (button presses).
fn put_value(key, value, ctx, payload) { set_value(key, ctx, value); }
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
fn go_to(ctx, page) { if type_of(page) == "string" && page != "" { ctx.set_state("page", page); } }
fn open_palette(ctx, payload) {
    ctx.set_state("palette_query", "");
    ctx.set_state("palette_open", true);
}
fn palette_open(ctx, open) { ctx.set_state("palette_open", open); }
fn palette_query(ctx, query) { ctx.set_state("palette_query", query); }
fn palette_active(ctx, value) { ctx.set_state("palette_active", value); }
fn palette_run(ctx, value) {
    if value.starts_with("page:") { ctx.set_state("page", value.sub_string(5)); }
    ctx.set_state("palette_open", false);
}
fn toggle_source(ctx, payload) { set_value("gallery.source_open", ctx, !source_open(ctx.get_state("values"))); }
fn source_open(values) { "gallery.source_open" in values && values["gallery.source_open"] }
fn toggle_density(ctx, payload) {
    ctx.set_state("density", if ctx.get_state("density") == "compact" { "comfortable" } else { "compact" });
}
fn set_density_action(ctx, density) { if density == "compact" || density == "comfortable" { ctx.set_state("density", density); } }
fn set_density(ctx, values) { if values.len > 0 { ctx.set_state("density", values[0]); } }
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
    let parts = value.split("|");
    if parts.len == 2 { ctx.set_theme(parts[0], parts[1]); }
}
fn theme_open(ctx, open) { ctx.set_state("theme_open", open); }
fn choose_locale(ctx, value) {
    if value == () { return; }
    ctx.set_state("locale", value);
    ctx.set_locale(value);
}
fn locale_open(ctx, open) { ctx.set_state("locale_open", open); }
fn set_audit(ctx, count) { ctx.set_state("audit", count); }
fn nav_select(ctx, keys) {
    if keys.len > 0 && !keys[0].starts_with("group:") { ctx.set_state("page", keys[0]); }
}
fn nav_active(ctx, key) { ctx.set_state("nav_active", key); }
fn nav_expand(ctx, keys) { set_value("gallery.nav_expanded", ctx, keys); }

// ---- pages ----------------------------------------------------------------------------

fn sections() {
    [
        #{ title: "Foundations", pages: foundations::pages() },
        #{ title: "Markers", pages: markers::pages() },
        #{ title: "Fields", pages: fields::pages() },
        #{ title: "Lists", pages: lists::pages() },
        #{ title: "Overlays", pages: overlays::pages() },
        #{ title: "Containers", pages: containers::pages() },
        #{ title: "Display", pages: display::pages() },
        #{ title: "Interaction", pages: interaction::pages() },
        #{ title: "Layouts and patterns", pages: composition::pages() },
        #{ title: "Scenes", pages: scenes::pages() },
        #{ title: "Effects", pages: effects::pages() },
    ]
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

fn page_module(id) {
    for section in sections() {
        for page in section.pages {
            if page.id == id {
                return if section.title == "Layouts and patterns" { "gallery/pages/composition" }
                    else { `gallery/pages/${section.title.to_lower()}` };
            }
        }
    }
    "gallery/pages/markers"
}

fn page_view(ctx, h, v, id) {
    let node = foundations::view(ctx, h, v, id);
    if node == () { node = markers::view(ctx, h, v, id); }
    if node == () { node = fields::view(ctx, h, v, id); }
    if node == () { node = lists::view(ctx, h, v, id); }
    if node == () { node = overlays::view(ctx, h, v, id); }
    if node == () { node = containers::view(ctx, h, v, id); }
    if node == () { node = display::view(ctx, h, v, id); }
    if node == () { node = interaction::view(ctx, h, v, id); }
    if node == () { node = composition::view(ctx, h, v, id); }
    if node == () { node = scenes::view(ctx, h, v, id); }
    if node == () { node = effects::view(ctx, h, v, id); }
    if node == () { node = markers::view(ctx, h, v, "button"); }
    node
}

// ---- shell ----------------------------------------------------------------------------

fn handlers() {
    #{ set: Fn("set_value"), put: Fn("put_value"), checked: Fn("set_checked"), reorder: Fn("reorder"), drop: Fn("drop_to"),
        noop: Fn("noop"), navigate: Fn("go_to") }
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

fn title_controls(ctx) {
    let density = toggle_group::ToggleGroup(#{ key: "gallery-density", label: "Density",
        values: [ctx.get_state("density")], allow_empty: false, size: "sm",
        items: [#{ value: "comfortable", label: "Comfortable" }, #{ value: "compact", label: "Compact" }],
        on_change: Fn("set_density") });
    let current = ctx.theme_variant();
    let theme = select::Select(#{ key: "gallery-theme", label: "Theme", value: `${current.family}|${current.name}`,
        open: ctx.get_state("theme_open"), query: "", size: "sm", width: px(176), options: theme_options(ctx),
        on_change: Fn("choose_theme"), on_open_change: Fn("theme_open") });
    let locale = select::Select(#{ key: "gallery-locale", label: "Locale", value: ctx.get_state("locale"),
        open: ctx.get_state("locale_open"), query: "", size: "sm", width: px(120),
        options: [#{ value: "en", label: "English" }, #{ value: "zh-CN", label: "简体中文" }, #{ value: "ar", label: "العربية" }],
        on_change: Fn("choose_locale"), on_open_change: Fn("locale_open") });
    let palette = button::Button(#{ key: "gallery-palette", text: "Commands", variant: "ghost", size: "sm",
        action: "gallery.palette" });
    [density, theme, locale, palette]
}

// One option per loaded variant; a family with several variants names the variant.
fn theme_options(ctx) {
    let variants = ctx.theme_variants();
    let options = [];
    for variant in variants {
        let siblings = variants.filter(|other| other.family == variant.family).len;
        options.push(#{ value: `${variant.family}|${variant.name}`,
            label: if siblings > 1 { `${variant.family} · ${variant.name}` } else { variant.family } });
    }
    options
}

fn palette_items() {
    let items = [];
    for section in sections() {
        for entry in section.pages {
            items.push(#{ value: `page:${entry.id}`, label: entry.title, group: section.title });
        }
    }
    items.push(#{ value: "action:density", label: "Toggle density", group: "Gallery", action: "gallery.toggle_density" });
    items.push(#{ value: "action:mode", label: "Toggle light and dark", group: "Gallery", action: "gallery.toggle_mode" });
    items.push(#{ value: "action:source", label: "Toggle source", group: "Gallery", action: "gallery.toggle_source" });
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

fn theme_label(variant) { `${variant.family} ${variant.name}` }

fn status_items(ctx) {
    let label = |name, value| text(`${name} ${value}`);
    #{
        start: [label.call("DENSITY", ctx.get_state("density")), label.call("THEME", theme_label(ctx.theme_variant())),
            label.call("LOCALE", ctx.get_state("locale"))],
        end: [text(`AUDIT ${ctx.get_state("audit")}`).accessibility_role("status").accessibility_label("Audit findings"),
            text("⌘K")],
    }
}

// Without a platform title bar, the Gallery's own bar moves the window and starts after
// the window buttons.
fn title_bar(ctx, page) {
    let bar = #{ title: "GPUI RHAI", subtitle: page_title(page), end: title_controls(ctx) };
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
    let palette = command_dialog::CommandDialog(#{ key: "gallery-palette-dialog", label: "Go to",
        open: ctx.get_state("palette_open"), query: ctx.get_state("palette_query"),
        active_value: ctx.get_state("palette_active"), items: palette_items(),
        on_open_change: Fn("palette_open"), on_query_change: Fn("palette_query"),
        on_active_change: Fn("palette_active"), on_action: Fn("palette_run") });
    let shell = #{ key: "gallery", label: "gpui-rhai Gallery",
        title_bar: title_bar(ctx, page),
        sidebar: navigation(ctx, v, page), main: page_view(ctx, h, v, page),
        status: status_items(ctx), overlays: [palette] };
    if source_open(v) { shell.inspector = source_inspector(ctx, page); }
    app_shell::AppShell(shell).env(#{ density: ctx.get_state("density") })
}
