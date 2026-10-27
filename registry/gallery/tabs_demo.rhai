// Tab collections a specimen owns for the Tabs page: the open keys in order and the selected
// one, so adding, closing and reordering are real.
import "components/tabs" as tabs;
import "components/button" as button;
import "components/icon_button" as icon_button;

define_component(#{
    metadata: #{ id: "gallery/tabs_demo", "export": "TabsDemo", version: "0.3.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/tabs", "components/button", "components/icon_button"], capabilities: #{},
        tokens: ["tabbar.active", "metrics.inset", "border"], environment: [] },
    schema: #{ props: #{ mode: #{ schema: #{ type: "string", allowed: ["repos", "files", "vertical"] },
            required: true, sensitive: false, doc: "Which collection the specimen shows." } },
        state: #{ fields: #{
            order: #{ schema: #{ type: "array", items: #{ type: "string" } }, "default": #{ type: "array", value: [] } },
            value: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
            added: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
        } },
        events: #{}, slots: #{}, parts: ["root"], effects: ["seed"] },
    render: Fn("render_TabsDemo")
});

fn TabsDemo(props) { render_component("gallery/tabs_demo", props) }

fn demo_keys(mode) {
    if mode == "repos" { ["gpui-rhai", "oh-my-byted", "disktree", "infra-config", "bytestate"] }
    else if mode == "files" { ["main", "lib", "app", "theme", "styles", "readme", "changelog", "cargo"] }
    else { ["inbox", "drafts", "sent", "archive"] }
}
fn demo_seed(mode, ctx, deps) {
    if ctx.get_state("order").len == 0 && ctx.get_state("added") == 0 {
        let keys = demo_keys(mode);
        ctx.set_state("order", keys);
        ctx.set_state("value", keys[if mode == "files" { 1 } else { 0 }]);
    }
}
fn demo_unseed(ctx, deps) {}
fn demo_label(mode, key) {
    let files = #{ main: "main.rs", lib: "lib.rs", app: "app.rs", theme: "theme.rhai",
        styles: "styles.rhai", readme: "README.md", changelog: "CHANGELOG.md", cargo: "Cargo.toml" };
    let boxes = #{ inbox: "Inbox", drafts: "Drafts", sent: "Sent", archive: "Archive" };
    if mode == "files" && key in files { files[key] }
    else if mode == "vertical" && key in boxes { boxes[key] }
    else { key }
}
// Open merge requests per repository; a new repository has none yet.
fn demo_count(key) {
    let counts = #{ "gpui-rhai": 12, "oh-my-byted": 3, "disktree": 0, "infra-config": 128,
        inbox: 4, drafts: 0 };
    if key in counts { counts[key] } else if key.starts_with("repo-") { 0 } else { () }
}
fn demo_items(mode, order) {
    let items = [];
    for key in order {
        let item = #{ value: key, label: demo_label(mode, key), closable: true };
        let count = demo_count(key);
        if count != () && mode != "files" { item.count = count; }
        if key == "oh-my-byted" || key == "inbox" { item.tone = "accent"; item.tone_label = "has updates"; }
        if key == "bytestate" { item.tone = "danger"; item.tone_label = "sync failed"; }
        if mode == "files" { item.dirty = key == "lib" || key == "styles"; }
        if mode != "files" {
            let detail = if count == () { "No count." } else { `${count} open.` };
            item.content = text(`${item.label}: ${detail}`);
        }
        items.push(item);
    }
    items
}
fn demo_change(ctx, value) { ctx.set_state("value", value); }
fn demo_close(ctx, request) {
    ctx.set_state("order", ctx.get_state("order").filter(|key| key != request.value));
    if ctx.get_state("value") == request.value { ctx.set_state("value", request.next); }
}
fn demo_add(mode, ctx, payload) {
    let added = ctx.get_state("added") + 1;
    let key = if mode == "repos" { `repo-${added}` } else if mode == "files" { `new-${added}.rs` } else { `folder-${added}` };
    let order = ctx.get_state("order");
    order.push(key);
    ctx.set_state("order", order);
    ctx.set_state("added", added);
    ctx.set_state("value", key);
}
fn demo_reorder(ctx, request) {
    let order = ctx.get_state("order").filter(|key| key != request.value);
    let placed = [];
    for key in order {
        if key == request.anchor && request.placement == "before" { placed.push(request.value); }
        placed.push(key);
        if key == request.anchor && request.placement == "after" { placed.push(request.value); }
    }
    ctx.set_state("order", placed);
}
fn demo_step(step, ctx, payload) {
    let order = ctx.get_state("order");
    if order.len == 0 { return; }
    let at = 0;
    for index in 0..order.len { if order[index] == ctx.get_state("value") { at = index; } }
    ctx.set_state("value", order[(at + order.len + step) % order.len]);
}
// The Remove button closes the selected tab from outside the strip, with the same successor.
fn demo_remove(mode, ctx, payload) {
    let value = ctx.get_state("value");
    if value == "" { return; }
    demo_close(ctx, #{ value: value, next: tabs::close_successor(demo_items(mode, ctx.get_state("order")), value) });
}

fn render_TabsDemo(ctx, props) {
    let mode = props.mode;
    effect("seed", mode, Fn("demo_seed").curry(mode), Fn("demo_unseed"));
    let items = demo_items(mode, ctx.get_state("order"));
    let config = #{ key: `${props.key}-tabs`, value: ctx.get_state("value"), tabs: items, reorderable: true,
        on_change: Fn("demo_change"), on_close: Fn("demo_close"), on_add: Fn("demo_add").curry(mode),
        on_reorder: Fn("demo_reorder") };
    if mode == "repos" {
        config.label = "Watched repositories";
        config.variant = "underline";
        config.add_label = "Watch a repository";
        config.close_label = "Stop watching";
        config.start = [
            icon_button::IconButton(#{ icon: image_source(asset("app/icons/chevron_left")), label: "Previous repository",
                on_click: Fn("demo_step").curry(-1) }),
            icon_button::IconButton(#{ icon: image_source(asset("app/icons/chevron_right")), label: "Next repository",
                on_click: Fn("demo_step").curry(1) }),
        ];
        config.end = [button::Button(#{ text: "Remove", variant: "ghost",
            disabled: ctx.get_state("value") == "", on_click: Fn("demo_remove").curry(mode) })];
        return tabs::Tabs(config).with_style(ctx.component_style("root", style().width(px(640))));
    }
    if mode == "files" {
        config.label = "Open files";
        config.variant = "bar";
        config.overflow_menu = true;
        config.add_label = "New file";
        config.panel = false;
        let names = demo_label(mode, ctx.get_state("value"));
        let editor = box([text(if items.len == 0 { "No open files." } else { `Editing ${names}` })])
            .with_style(style().height(px(96)).padding(theme_length("metrics.inset"))
                .background(theme_color("tabbar.active")).border(px(1)).border_top(px(0))
                .border_color(theme_color("border")));
        return column([tabs::Tabs(config), editor]).with_style(ctx.component_style("root", style().width(px(560))));
    }
    config.label = "Mailboxes";
    config.orientation = "vertical";
    config.add_label = "New folder";
    tabs::Tabs(config).with_style(ctx.component_style("root", style().width(px(420)).height(px(220))))
}
