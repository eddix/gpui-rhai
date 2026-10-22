// disktree-rhai default skin.
//
// M3: navigation and the full interaction set. The mosaic is laid out once
// per (scan, crumbs, metric, depth, viewport) in the layout effect and then
// only projected while painting: zoom and pan live in a view transform
// (#{scale, ox, oy}), never a re-layout. Keys and the wheel drive the same
// primitives tobi uses — descend/ascend, geometric neighbour movement,
// ranked sibling cycling, anchored zoom that enters a directory at its
// ceiling and ascends at the floor, history on alt-arrows and side buttons.
// The minimal skin keeps its chrome on the official 0.2 component set
// (runtime API 3) — button, progress, spinner, keycaps and the stat pattern —
// while the mosaic stays bespoke canvas and the controls stay keyboard-only.
import "components/button" as button;
import "components/kbd" as kbd;
import "components/progress" as progress;
import "components/spinner" as spinner;
import "components/status_bar" as status_bar;
import "patterns/stat" as stat;
import "core" as core;

// ---------------------------------------------------------------------------
// ScanRoot
// ---------------------------------------------------------------------------

define_component(#{
    metadata: #{
        id: "skin/scan_root",
        "export": "ScanRoot",
        version: "0.4.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [
            "components/button", "components/kbd", "components/progress",
            "components/spinner", "components/status_bar", "patterns/stat",
        ],
        capabilities: #{
            "disktree.scan": "*",
            "disktree.tree": "*",
            "disktree.open": "*",
            "disktree.git": "*",
            "disktree.args": "*",
        },
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
        },
        state: #{
            fields: #{
                epoch: #{
                    schema: #{ type: "integer" },
                    "default": #{ type: "integer", value: 0 },
                },
                running: #{
                    schema: #{ type: "bool" },
                    "default": #{ type: "bool", value: true },
                },
                metric: #{
                    schema: #{ type: "string", allowed: ["size", "files"] },
                    "default": #{ type: "string", value: "size" },
                },
                apparent: #{
                    schema: #{ type: "bool" },
                    "default": #{ type: "bool", value: false },
                },
                no_hidden: #{
                    schema: #{ type: "bool" },
                    "default": #{ type: "bool", value: false },
                },
                depth: #{
                    schema: #{ type: "integer", min: 1, max: 6 },
                    "default": #{ type: "integer", value: 3 },
                },
                mode: #{
                    schema: #{ type: "integer", min: 0, max: 2 },
                    "default": #{ type: "integer", value: 0 },
                },
                help_open: #{
                    schema: #{ type: "bool" },
                    "default": #{ type: "bool", value: false },
                },
                sidebar_open: #{
                    schema: #{ type: "bool" },
                    "default": #{ type: "bool", value: true },
                },
                find: #{
                    schema: #{ type: "string", max_length: 64 },
                    "default": #{ type: "string", value: "" },
                },
                find_open: #{
                    schema: #{ type: "bool" },
                    "default": #{ type: "bool", value: false },
                },
            },
        },
        events: #{},
        slots: #{},
        parts: ["root"],
        effects: ["scan", "layout"],
    },
    render: Fn("render_ScanRoot"),
});

fn ScanRoot() {
    render_component("skin/scan_root", #{ key: "root" })
}

// -- scan lifecycle -----------------------------------------------------------

fn scan_start(ctx, deps) {
    // deps: [running, epoch, apparent, no_hidden]. The metric is not here on
    // purpose: the tree children re-rank per metric on demand, like tobi's
    // in-place re-aggregation, so size/files switching never rescans.
    let running = deps[0];
    if !running {
        return ();
    }
    let args = ctx.get_app_store("disktree", "args");
    let request = #{
        path: args.path,
        apparent_size: deps[2],
        no_hidden: deps[3],
    };
    let root = core::display_path(args.path, args.home);
    ctx.set_app_store("disktree", "scan", #{
        phase: "scanning",
        root: root,
        files: 0,
        dirs: 0,
        bytes: 0,
        errors: 0,
        messages: [],
        space: (),
        device: (),
        insights: [],
        elapsed_ms: 0,
        now: 0,
    });
    // A fresh scan invalidates the mosaic until it lands.
    reset_derived_layout(ctx);
    ctx.set_app_store(
        "disktree",
        "cursor",
        #{ crumbs: (), name: "", x: 0.0, y: 0.0, selected: (), active: false }
    );
    ctx.start_subscription(
        "disktree.scan",
        "start",
        request,
        Fn("scan_on_next"),
        Fn("scan_on_error"),
        #{ delivery: "latest", capacity: 4, throttle_ms: 100 }
    );
}

fn scan_stop(ctx, deps) {
    ()
}

fn scan_on_next(ctx, event) {
    let kind = event.type;
    let scan = ctx.get_app_store("disktree", "scan");
    if kind == "progress" {
        scan.files = event.files;
        scan.dirs = event.dirs;
        scan.bytes = event.bytes;
        scan.errors = event.errors;
        ctx.set_app_store("disktree", "scan", scan);
        return ();
    }
    if kind == "done" {
        scan.phase = "done";
        scan.files = event.files;
        scan.dirs = event.dirs;
        scan.bytes = event.bytes;
        scan.errors = event.errors;
        scan.messages = event.messages;
        scan.space = event.space;
        scan.device = event.device;
        scan.insights = event.insights;
        scan.elapsed_ms = event.elapsed_ms;
        scan.now = event.now;
        // A new tree invalidates the navigation crumbs and the mosaic, but
        // history survives: entries resolve lazily and prune themselves.
        reset_derived_layout(ctx);
        ctx.set_app_store("disktree", "scan", scan);
        return ();
    }
    if kind == "cancelled" {
        scan.phase = "cancelled";
        scan.files = event.files;
        scan.dirs = event.dirs;
        scan.bytes = event.bytes;
        scan.errors = event.errors;
        ctx.set_app_store("disktree", "scan", scan);
        return ();
    }
    if kind == "failed" {
        scan.phase = "failed";
        scan.messages = [event.error];
        ctx.set_app_store("disktree", "scan", scan);
        return ();
    }
    ()
}

fn scan_on_error(ctx, error) {
    let scan = ctx.get_app_store("disktree", "scan");
    scan.phase = "failed";
    scan.messages = [`${error}`];
    ctx.set_app_store("disktree", "scan", scan);
}

// -- actions ----------------------------------------------------------------

fn scan_cancel(ctx, payload) {
    ctx.set_state("running", false);
    let scan = ctx.get_app_store("disktree", "scan");
    scan.phase = "cancelled";
    ctx.set_app_store("disktree", "scan", scan);
}

fn scan_again(ctx, payload) {
    let epoch = ctx.get_state("epoch");
    ctx.set_state("epoch", epoch + 1);
    ctx.set_state("running", true);
}

/// The hover lookup table: crumb-key → tile, built once per layout.
fn tiles_by_key(tiles) {
    // Crumb-key to tile INDEX: a store read clones whatever it holds, so
    // holding tile maps here would deep-copy the whole tile table a second
    // time on every render. Indices clone as integers.
    let by_key = #{};
    let i = 0;
    while i < tiles.len() {
        let tile = tiles[i];
        if tile.kind != "others" {
            by_key[crumb_key(tile.crumbs)] = i;
        }
        i += 1;
    }
    by_key
}

/// O(1) tile lookup for pointer events and label clicks.
fn tile_lookup(layout, key) {
    let index = layout.by_key[key];
    if index == () {
        return ();
    }
    layout.tiles[index]
}

/// Everything the mosaic paints that does not change with hover or
/// selection: fills, strips, hatch, labels. Cached in the store and
/// rebuilt only from event-phase code (layout effect, zoom, mode) — view()
/// just assembles it, which is what keeps a hover crossing cheap.
/// The static paint, in BASE coordinates: zoom and pan never rebuild it,
/// the render projects each row through the view. Rows are plain data
/// (the store cannot hold command objects); labels cache area-sorted
/// candidate indices.
fn build_paint(ctx, tiles, mode, root_crumbs) {
    let light = is_light(ctx);
    let scan = ctx.get_app_store("disktree", "scan");
    // 过滤降淡：打字时引擎逐瓦片回答是否保持点亮；Enter 才落地过滤布局。
    let needle = "";
    if ctx.get_state("find_open") {
        needle = ctx.get_state("find");
    }
    let out_keys = #{};
    if needle.trim() != "" {
        let i = 0;
        while i < tiles.len() {
            let tile = tiles[i];
            let out = true;
            if tile.kind != "others" {
                let reply = ctx.call_capability("disktree.tree", "kept", #{
                    base: root_crumbs,
                    needle: needle,
                    crumbs: tile.crumbs,
                });
                if reply != "out" {
                    out = false;
                }
            }
            if out {
                out_keys[tile_key(tile)] = true;
            }
            i += 1;
        }
    }
    let base = [];
    let i = 0;
    while i < tiles.len() {
        base = base + tile_base_commands(tiles[i], mode, light, scan.now, out_keys);
        i += 1;
    }
    #{
        base: base,
        labels: sorted_label_candidates(tiles),
        out_keys: out_keys,
    }
}

/// The light/dark branch of the palette, from the resolved theme variant.
fn is_light(ctx) {
    let variant = ctx.theme_variant();
    variant != () && variant.mode == "light"
}

// -- crumbs helpers -------------------------------------------------------------

/// Crumbs as a compact string, for effect dependency comparison and canvas
/// command keys.
fn crumb_key(crumbs) {
    if crumbs.len() == 0 {
        return "";
    }
    let key = `${crumbs[0]}`;
    let i = 1;
    while i < crumbs.len() {
        key = `${key}.${crumbs[i]}`;
        i += 1;
    }
    key
}

fn empty_layout() {
    #{
        valid: false,
        crumbs: [],
        nav_key: "",
        metric: "size",
        depth: 3,
        width: 0.0,
        height: 0.0,
        tiles: [],
        total: 0,
        root: "",
        view: core::view_identity(),
        back: [],
        forward: [],
        names: [],
        insights: [],
        focused: false,
        filter: "",
        find_summary: (),
        by_key: #{},
        gen: 0,
        anim: anim_idle(),
    }
}

/// The paint slots and the one-slot tile cache live in their own app-store
/// values, not inside the layout: rhai's array limit counts nested array
/// elements cumulatively per store VALUE (docs/UPSTREAM_GAPS.md G9), and
/// tiles + their crumb arrays + a cache slot + paint rows in one value
/// crosses 10k on real trees. Reset together with the layout on every fresh
/// scan.
fn reset_derived_layout(ctx) {
    ctx.set_app_store("disktree", "layout", empty_layout());
    ctx.set_app_store("disktree", "paint0", ());
    ctx.set_app_store("disktree", "paint1", ());
    ctx.set_app_store("disktree", "paint2", ());
    ctx.set_app_store("disktree", "cache", []);
}

/// The idle animation state.
fn anim_idle() {
    #{ active: false, from: core::view_identity(), step: 0, steps: 0 }
}

/// The view that makes `body` fill the viewport, centred on its middle:
/// where a grown-into-place navigation starts (or ends, reversed).
fn zoom_start(body, width, height) {
    let scale = core::view_fit_scale(body, width, height);
    if scale <= 1.05 {
        return ();
    }
    #{
        scale: scale,
        ox: body.x + body.w / 2.0 - (width / 2.0) / scale,
        oy: body.y + body.h / 2.0 - (height / 2.0) / scale,
    }
}

/// Ease-out cubic for the zoom steps.
fn anim_ease(t) {
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

/// One view on the way from `from` to identity at fraction `t`.
fn view_lerp(from, t) {
    #{
        scale: from.scale + (1.0 - from.scale) * t,
        ox: from.ox * (1.0 - t),
        oy: from.oy * (1.0 - t),
    }
}

/// One 16ms step of a navigation transition: the view eases from the
/// zoomed start toward identity, and the last step lands exactly on it.
fn anim_tick(ctx, step) {
    let layout = ctx.get_app_store("disktree", "layout");
    let anim = layout.anim;
    if !anim.active {
        return ();
    }
    let next = step + 1;
    if next >= anim.steps {
        layout.view = core::view_identity();
        layout.anim = anim_idle();
        ctx.set_app_store("disktree", "layout", layout);
        return ();
    }
    layout.view = view_lerp(anim.from, anim_ease(next.to_float() / anim.steps.to_float()));
    anim.step = next;
    layout.anim = anim;
    ctx.set_app_store("disktree", "layout", layout);
}

/// A gesture (wheel, zoom keys) takes the view back: stop any transition.
fn cancel_anim(ctx, layout) {
    if layout.anim.active {
        layout.anim = anim_idle();
        ctx.set_app_store("disktree", "layout", layout);
    }
}


/// Empty the three paint slots (own store values, see reset_derived_layout).
fn reset_paint(ctx) {
    ctx.set_app_store("disktree", "paint0", ());
    ctx.set_app_store("disktree", "paint1", ());
    ctx.set_app_store("disktree", "paint2", ());
}

// -- layout -------------------------------------------------------------------

/// Recompute the treemap for the current viewport. Runs in the layout
/// effect (event phase), so capability reads and store writes are legal
/// here; view() only reads the cached tiles.
fn layout_recompute(ctx, deps) {
    // deps: [phase, nav_key, metric, depth, width, height]
    let phase = deps[0];
    if phase != "done" {
        return ();
    }
    let width = deps[4];
    let height = deps[5];
    if width < 10.0 || height < 10.0 {
        return ();
    }
    let previous = ctx.get_app_store("disktree", "layout");
    // 参数化缓存：同一 (metric, depth, crumbs, size, filter) 的往返切换
    // （t 循环、[/] 试深度）直接复用瓦片，重算只发生在真正新参数上。
    let cache_key = `${deps[2]}:${deps[3]}:${deps[1]}:${deps[4].to_int()}x${deps[5].to_int()}:${deps[6]}`;
    // The one cache slot lives in its own store value (G9: array budgets are
    // cumulative per store value).
    let previous_cache = ctx.get_app_store("disktree", "cache");
    let cached = ();
    let kept_cache = [];
    for slot in previous_cache {
        if slot[0] == cache_key {
            cached = slot;
        } else {
            kept_cache.push(slot);
        }
    }
    if cached != () {
        let layout = ctx.get_app_store("disktree", "layout");
        layout.tiles = cached[1];
        layout.by_key = cached[2];
        layout.gen += 1;
        layout.valid = true;
        layout.anim = anim_idle();
        ctx.set_app_store("disktree", "layout", layout);
        // A restored layout invalidates every paint; rebuild the one the
        // user is actually looking at, or the next render is blank.
        let mode_now = ctx.get_state("mode");
        reset_paint(ctx);
        ctx.set_app_store("disktree", `paint${mode_now}`, build_paint(ctx, layout.tiles, mode_now, layout.crumbs));
        return ();
    }
    let crumbs = [];
    let view = core::view_identity();
    let back = previous.back;
    let forward = previous.forward;
    let focused = previous.focused;
    let filter = previous.filter;
    let find_summary = previous.find_summary;
    if previous.valid {
        crumbs = previous.crumbs;
        view = previous.view;
    }
    // Entering a directory: the transition starts zoomed on its tile in
    // the old mosaic, so the new contents grow into place.
    let anim_from = ();
    if previous.valid && previous.crumbs.len() < crumbs.len()
        && core::crumb_starts_with(crumbs, previous.crumbs)
    {
        let entered = tile_lookup(previous, crumb_key(crumbs));
        if entered != () {
            anim_from = zoom_start(core::tile_body(entered), width, height);
        }
    }
    // The tile budget scales with the crumb depth: each tile carries its own
    // crumb array, and rhai's array limit counts them cumulatively (G9), so
    // a deep current directory earns fewer tiles.
    let per_tile = crumbs.len() + deps[3] + 2;
    let budget = 8500 / per_tile;
    if budget > 2000 {
        budget = 2000;
    }
    if budget < 250 {
        budget = 250;
    }
    let tiles = core::place_dir(
        ctx,
        crumbs,
        core::rect(0.0, 0.0, width, height),
        deps[2],
        core::layout_options(deps[3]),
        0,
        deps[6],
        budget
    );
    let scan = ctx.get_app_store("disktree", "scan");
    let layout = empty_layout();
    layout.valid = true;
    layout.crumbs = crumbs;
    layout.nav_key = deps[1];
    layout.metric = deps[2];
    layout.depth = deps[3];
    layout.width = width;
    layout.height = height;
    layout.tiles = tiles;
    layout.total = scan.bytes;
    layout.root = scan.root;
    layout.view = view;
    layout.back = back;
    layout.forward = forward;
    layout.names = crumb_names(ctx, crumbs, deps[2]);
    layout.insights = insight_rows(ctx, scan.insights, deps[2]);
    layout.filter = filter;
    layout.find_summary = find_summary;
    layout.by_key = tiles_by_key(tiles);
    // Ascending: start zoomed on the directory just left, in the new
    // mosaic, easing out to the whole view.
    if anim_from == () && previous.valid && previous.crumbs.len() > crumbs.len()
        && core::crumb_starts_with(previous.crumbs, crumbs)
    {
        let departed = tile_lookup(layout, crumb_key(previous.crumbs));
        if departed != () {
            anim_from = zoom_start(core::tile_body(departed), width, height);
        }
    }
    if anim_from != () {
        layout.view = anim_from;
        let steps = ctx.motion_duration("normal") / 16;
        if steps > 24 {
            steps = 24;
        }
        if steps >= 2 {
            layout.anim = #{ active: true, from: anim_from, step: 0, steps: steps };
        }
    }
    kept_cache.push([cache_key, tiles, layout.by_key]);
    while kept_cache.len() > 1 {
        kept_cache.remove(0);
    }
    ctx.set_app_store("disktree", "cache", kept_cache);
    layout.gen += 1;
    // Build the paint for the mode on screen, not always mode 0: a metric
    // switch re-runs this effect right after set_mode filled the new
    // mode's slot, and rebuilding only mode 0 would blank the mosaic.
    let mode_now = ctx.get_state("mode");
    reset_paint(ctx);
    ctx.set_app_store("disktree", `paint${mode_now}`, build_paint(ctx, tiles, mode_now, crumbs));
    if focused {
        layout.focused = true;
    } else {
        // The keyboard belongs to the mosaic once there is one.
        layout.focused = true;
        ctx.set_app_store("disktree", "layout", layout);
        ctx.focus("root");
        return ();
    }
    ctx.set_app_store("disktree", "layout", layout);
}

fn layout_stop(ctx, deps) {
    ()
}

/// The names along the current crumb path, for the breadcrumb bar.
fn crumb_names(ctx, crumbs, metric) {
    let names = [];
    let walk = [];
    let i = 0;
    while i < crumbs.len() {
        let reply = ctx.call_capability("disktree.tree", "children", #{
            crumbs: walk,
            metric: metric,
            max: 96,
        });
        let wanted = crumbs[i];
        let found = "";
        for entry in reply.entries {
            if entry.index == wanted {
                found = entry.name;
            }
        }
        names.push(found);
        walk.push(wanted);
        i += 1;
    }
    names
}

/// Worth-a-look rows resolved to display names, computed here so view()
/// never walks the tree.
fn insight_rows(ctx, insights, metric) {
    let rows = [];
    for finding in insights {
        let names = crumb_names(ctx, finding.crumbs, metric);
        let label = "";
        if names.len() >= 2 {
            label = `${names[names.len() - 2]}/${names[names.len() - 1]}`;
        } else if names.len() == 1 {
            label = names[0];
        }
        rows.push(#{
            label: label,
            detail: finding.detail,
            bytes: finding.bytes,
            crumbs: finding.crumbs,
        });
    }
    rows
}

// -- navigation ----------------------------------------------------------------

/// The entry a crumb path points at, or `()`. Direct children of the parent
/// are ranked and addressed by their original index.
fn node_entry(ctx, crumbs) {
    if crumbs == () || crumbs.len() == 0 {
        return ();
    }
    let parent = core::nav_parent(crumbs);
    let metric = ctx.get_state("metric");
    let reply = ctx.call_capability("disktree.tree", "children", #{
        crumbs: parent,
        metric: metric,
        max: 96,
    });
    let wanted = crumbs[crumbs.len() - 1];
    for entry in reply.entries {
        if entry.index == wanted {
            return entry;
        }
    }
    ()
}

fn current_path(ctx, crumbs) {
    ctx.call_capability("disktree.tree", "path", #{ crumbs: crumbs })
}

/// Apply a navigation move: crumbs + view reset; the nav_key dep change
/// recomputes the layout.
fn apply_navigation(ctx, crumbs) {
    let layout = ctx.get_app_store("disktree", "layout");
    layout.crumbs = crumbs;
    layout.nav_key = crumb_key(crumbs);
    layout.view = core::view_identity();
    ctx.set_app_store("disktree", "layout", layout);
    let cursor = ctx.get_app_store("disktree", "cursor");
    cursor.selected = crumbs;
    cursor.crumbs = ();
    cursor.name = "";
    cursor.active = false;
    ctx.set_app_store("disktree", "cursor", cursor);
}

fn remember(ctx, layout) {
    let path = current_path(ctx, layout.crumbs);
    let back = layout.back;
    if back.len() == 0 || back[back.len() - 1] != path {
        back.push(path);
        if back.len() > 32 {
            back.remove(0);
        }
    }
    layout.back = back;
    layout.forward = [];
}

/// Enter the directory at `crumbs`: must be below the current root, a
/// directory, and non-empty.
fn nav_enter(ctx, crumbs) {
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid {
        return ();
    }
    if crumbs == () || crumbs.len() <= layout.crumbs.len() {
        return ();
    }
    if !core::crumb_starts_with(crumbs, layout.crumbs) {
        return ();
    }
    let entry = node_entry(ctx, crumbs);
    if entry == () || entry.kind != "dir" || entry.files + entry.dirs == 0 {
        return ();
    }
    remember(ctx, layout);
    apply_navigation(ctx, crumbs);
}

/// One level up; the directory we leave stays selected in the new frame.
fn nav_ascend(ctx) {
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid {
        return ();
    }
    let parent = core::nav_parent(layout.crumbs);
    if parent == () {
        return ();
    }
    remember(ctx, layout);
    apply_navigation(ctx, parent);
}

/// A jump from the breadcrumb bar: no single region to move, it lands.
fn nav_go_to(ctx, crumbs) {
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid || !core::crumb_starts_with(crumbs, layout.crumbs) {
        return ();
    }
    if crumb_key(crumbs) != layout.nav_key {
        remember(ctx, layout);
    }
    apply_navigation(ctx, crumbs);
}

/// History on alt-arrows and side buttons: browser stacks of paths, resolved
/// lazily so rescans prune what they no longer hold.
fn nav_history(ctx, going_back) {
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid {
        return ();
    }
    let stack = [];
    if going_back {
        stack = layout.back;
    } else {
        stack = layout.forward;
    }
    let crumbs = ();
    while stack.len() > 0 {
        let path = stack.pop();
        let resolved = ctx.call_capability("disktree.tree", "resolve", path);
        if resolved != () && crumb_key(resolved) != layout.nav_key {
            crumbs = resolved;
            break;
        }
    }
    if going_back {
        layout.back = stack;
    } else {
        layout.forward = stack;
    }
    if crumbs == () {
        ctx.set_app_store("disktree", "layout", layout);
        return ();
    }
    if going_back {
        layout.forward = push_path(layout.forward, current_path(ctx, layout.crumbs));
    } else {
        layout.back = push_path(layout.back, current_path(ctx, layout.crumbs));
    }
    ctx.set_app_store("disktree", "layout", layout);
    apply_navigation(ctx, crumbs);
}

fn push_path(stack, path) {
    if stack.len() > 0 && stack[stack.len() - 1] == path {
        return stack;
    }
    stack.push(path);
    if stack.len() > 32 {
        stack.remove(0);
    }
    stack
}

fn action_back(ctx, payload) {
    nav_history(ctx, true);
}

fn action_forward(ctx, payload) {
    nav_history(ctx, false);
}

/// enter: open what is selected (a file opens its parent), or the largest
/// entry of the current directory when nothing is.
fn nav_descend(ctx) {
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid {
        return ();
    }
    let cursor = ctx.get_app_store("disktree", "cursor");
    let selected = cursor.selected;
    if selected != () && selected.len() > layout.crumbs.len()
        && core::crumb_starts_with(selected, layout.crumbs)
    {
        let entry = node_entry(ctx, selected);
        if entry == () {
            return ();
        }
        if entry.kind == "dir" {
            nav_enter(ctx, selected);
        } else {
            nav_enter(ctx, core::nav_parent(selected));
        }
        return ();
    }
    // Nothing below the root is selected: the largest entry.
    let metric = ctx.get_state("metric");
    let reply = ctx.call_capability("disktree.tree", "children", #{
        crumbs: layout.crumbs,
        metric: metric,
        max: 1,
    });
    if reply.entries.len() == 0 {
        return ();
    }
    let crumbs = layout.crumbs + [reply.entries[0].index];
    nav_enter(ctx, crumbs);
}

/// Arrows and hjkl: the best same-depth neighbour, starting from the hover
/// when the pointer moved last (tobi's adopt_pointer_target).
fn nav_move(ctx, direction) {
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid {
        return ();
    }
    let cursor = ctx.get_app_store("disktree", "cursor");
    let selected = cursor.selected;
    if cursor.active && cursor.crumbs != () {
        selected = cursor.crumbs;
    }
    if selected == () {
        // The first drawn tile is the largest thing on screen.
        for tile in layout.tiles {
            if tile.kind != "others" {
                set_selection(ctx, tile.crumbs);
                deactivate_pointer(ctx);
                return ();
            }
        }
        return ();
    }
    let next = core::nav_neighbour(layout.tiles, layout.view, selected, direction);
    if next != () {
        set_selection(ctx, next);
    } else if (direction == "left" || direction == "up") {
        // Walking off the edge upwards selects the parent of the root.
        let parent = core::nav_parent(layout.crumbs);
        if parent != () {
            set_selection(ctx, parent);
        }
    }
    deactivate_pointer(ctx);
}

/// tab: the next sibling by rank (next-largest by the active metric).
fn nav_cycle(ctx, step) {
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid {
        return ();
    }
    let cursor = ctx.get_app_store("disktree", "cursor");
    let selected = cursor.selected;
    let parent = layout.crumbs;
    if selected != () && selected.len() > layout.crumbs.len() {
        parent = core::nav_parent(selected);
    }
    let metric = ctx.get_state("metric");
    let reply = ctx.call_capability("disktree.tree", "children", #{
        crumbs: parent,
        metric: metric,
        max: 96,
    });
    let entries = reply.entries;
    if entries.len() == 0 {
        return ();
    }
    let position = -1;
    if selected != () && selected.len() == parent.len() + 1 {
        let last = selected[selected.len() - 1];
        let i = 0;
        while i < entries.len() {
            if entries[i].index == last {
                position = i;
            }
            i += 1;
        }
    }
    let next = 0;
    if position >= 0 {
        next = (position + step) % entries.len();
        if next < 0 {
            next += entries.len();
        }
    } else if step < 0 {
        next = entries.len() - 1;
    }
    set_selection(ctx, parent + [entries[next].index]);
    deactivate_pointer(ctx);
}

fn set_selection(ctx, crumbs) {
    let cursor = ctx.get_app_store("disktree", "cursor");
    cursor.selected = crumbs;
    ctx.set_app_store("disktree", "cursor", cursor);
}

fn deactivate_pointer(ctx) {
    let cursor = ctx.get_app_store("disktree", "cursor");
    if cursor.active {
        cursor.active = false;
        ctx.set_app_store("disktree", "cursor", cursor);
    }
}

// -- zoom ----------------------------------------------------------------------

/// Zoom toward a screen point. With `descend`, the wheel stops magnifying at
/// the point where the directory under the pointer exactly fits the
/// viewport, and the next notch goes inside it; at the floor, zooming out
/// ascends. That is what keeps the two gestures continuous.
fn zoom_at(ctx, x, y, factor, descend) {
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid {
        return ();
    }
    layout.anim = anim_idle();
    let view = layout.view;
    if factor < 1.0 && view.scale <= 1.0 + 0.000001 {
        if descend {
            nav_ascend(ctx);
        }
        return ();
    }
    let target = ();
    let ceiling = 5.0;
    if descend {
        let found = core::zoom_target(
            layout.tiles,
            view,
            layout.crumbs.len(),
            x,
            y
        );
        if found != () {
            target = found.crumbs;
            let fit = core::view_fit_scale(
                core::tile_body(found),
                layout.width,
                layout.height
            );
            ceiling = fit;
            if ceiling < 1.0 {
                ceiling = 1.0;
            }
            if ceiling > 5.0 {
                ceiling = 5.0;
            }
        }
    }
    if factor > 1.0 && target != () && view.scale >= ceiling - 0.000001 {
        nav_enter(ctx, target);
        return ();
    }
    layout.view = core::view_clamped(
        core::view_zoomed_at(view, x, y, factor, ceiling),
        layout.width,
        layout.height
    );
    ctx.set_app_store("disktree", "layout", layout);
}

fn zoom_reset(ctx, payload) {
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid {
        return ();
    }
    layout.view = core::view_identity();
    layout.anim = anim_idle();
    ctx.set_app_store("disktree", "layout", layout);
}

// -- keys ----------------------------------------------------------------------

/// The one key dispatcher: every registration passes its key name back as
/// the payload.
fn on_key(ctx, name) {
    let scan = ctx.get_app_store("disktree", "scan");
    if ctx.get_state("help_open") {
        if name == "escape" || name == "?" || name == "q" {
            ctx.set_state("help_open", false);
        }
        return ();
    }
    // The find field owns the keyboard while it is open: no editor state to
    // keep in sync with the tree, and Escape always means give it back.
    if ctx.get_state("find_open") {
        if name == "escape" {
            find_close(ctx);
        } else if name == "enter" {
            find_apply(ctx);
        } else if name == "backspace" {
            find_pop(ctx);
        } else {
            find_push(ctx, name);
        }
        return ();
    }
    if scan.phase == "scanning" {
        // While the walk runs, the keyboard only stops it.
        if name == "escape" {
            scan_cancel(ctx, ());
        }
        return ();
    }
    if name == "enter" {
        nav_descend(ctx);
    } else if name == "left" || name == "h" {
        nav_move(ctx, "left");
    } else if name == "right" || name == "l" {
        nav_move(ctx, "right");
    } else if name == "up" || name == "k" {
        nav_move(ctx, "up");
    } else if name == "down" || name == "j" {
        nav_move(ctx, "down");
    } else if name == "tab" {
        nav_cycle(ctx, 1);
    } else if name == "backspace" || name == "u" {
        nav_ascend(ctx);
    } else if name == "/" || name == "s" {
        ctx.set_state("find_open", true);
    } else if name == "escape" {
        let layout = ctx.get_app_store("disktree", "layout");
        let cursor = ctx.get_app_store("disktree", "cursor");
        // A filter is the first thing Escape takes away.
        if layout.filter != "" {
            clear_filter(ctx);
        } else if cursor.selected != () {
            set_selection(ctx, ());
        } else {
            nav_ascend(ctx);
        }
    } else if name == "[" {
        adjust_depth(ctx, -1);
    } else if name == "]" {
        adjust_depth(ctx, 1);
    } else if name == "-" {
        zoom_centered(ctx, 1.0 / 1.25);
    } else if name == "=" || name == "+" {
        zoom_centered(ctx, 1.25);
    } else if name == "0" {
        zoom_reset(ctx, ());
    } else if name == "t" {
        cycle_mode(ctx);
    } else if name == "r" {
        scan_again(ctx, ());
    } else if name == "i" {
        let hidden = ctx.get_state("no_hidden");
        ctx.set_state("no_hidden", !hidden);
    } else if name == "d" {
        let apparent = ctx.get_state("apparent");
        ctx.set_state("apparent", !apparent);
    } else if name == "p" {
        let open = ctx.get_state("sidebar_open");
        ctx.set_state("sidebar_open", !open);
    } else if name == "o" {
        reveal_target(ctx);
    } else if name == "e" {
        cycle_theme(ctx);
    } else if name == "?" {
        ctx.set_state("help_open", true);
    } else if name == "q" {
        ctx.close_window(ctx.window_id());
    }
}

/// `-` and `=` zoom on the centre, without the descend gesture.
fn zoom_centered(ctx, factor) {
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid {
        return ();
    }
    zoom_at(ctx, layout.width / 2.0, layout.height / 2.0, factor, false);
}

fn adjust_depth(ctx, step) {
    let depth = ctx.get_state("depth") + step;
    if depth < 1 {
        depth = 1;
    }
    if depth > 6 {
        depth = 6;
    }
    ctx.set_state("depth", depth);
}

/// t: the Size | Files | Age choice — what areas measure and what colour
/// says. Age keeps areas by size, since age has no area of its own.
fn cycle_mode(ctx) {
    let mode = ctx.get_state("mode");
    set_mode(ctx, (mode + 1) % 3);
}

fn set_mode(ctx, mode) {
    // Fills follow the mode; each mode's paint is built once per layout
    // generation and reused after. Build BEFORE switching state so the
    // render never sees an empty slot.
    let layout = ctx.get_app_store("disktree", "layout");
    if layout.valid {
        let slot = ctx.get_app_store("disktree", `paint${mode}`);
        if slot == () {
            ctx.set_app_store("disktree", `paint${mode}`, build_paint(ctx, layout.tiles, mode, layout.crumbs));
        }
    }
    ctx.set_state("mode", mode);
    if mode == 1 {
        ctx.set_state("metric", "files");
    } else {
        ctx.set_state("metric", "size");
    }
}

/// o: show the selection (or the hover, or the current directory) in the
/// file manager.
fn reveal_target(ctx) {
    let cursor = ctx.get_app_store("disktree", "cursor");
    let layout = ctx.get_app_store("disktree", "layout");
    let crumbs = layout.crumbs;
    if cursor.selected != () {
        crumbs = cursor.selected;
    } else if cursor.crumbs != () {
        crumbs = cursor.crumbs;
    }
    if !core::crumb_starts_with(crumbs, layout.crumbs) {
        crumbs = layout.crumbs;
    }
    if crumbs == () {
        return ();
    }
    let path = current_path(ctx, crumbs);
    ctx.start_task(
        "disktree.open",
        "reveal",
        path,
        Fn("reveal_done"),
        Fn("reveal_failed")
    );
}

fn reveal_done(ctx, payload) {
    ()
}

fn reveal_failed(ctx, error) {
    ()
}

// -- find ----------------------------------------------------------------------

/// The live match summary for the current needle, from the tree capability.
fn find_refresh(ctx) {
    let find = ctx.get_state("find");
    let layout = ctx.get_app_store("disktree", "layout");
    // 降淡跟着按键走：立即按当前 needle 重建屏幕上的 paint。
    if layout.valid {
        let mode_now = ctx.get_state("mode");
        ctx.set_app_store(
            "disktree",
            `paint${mode_now}`,
            build_paint(ctx, layout.tiles, mode_now, layout.crumbs)
        );
    }
    if find.trim() == "" || !layout.valid {
        layout.find_summary = ();
        ctx.set_app_store("disktree", "layout", layout);
        return ();
    }
    let summary = ctx.call_capability("disktree.tree", "matches", #{
        crumbs: layout.crumbs,
        needle: find,
    });
    layout.find_summary = summary;
    ctx.set_app_store("disktree", "layout", layout);
}

fn find_push(ctx, ch) {
    let find = ctx.get_state("find");
    if find.len() >= 64 {
        return ();
    }
    ctx.set_state("find", `${find}${ch}`);
    find_refresh(ctx);
}

fn find_pop(ctx) {
    let find = ctx.get_state("find");
    if find.len() <= 1 {
        ctx.set_state("find", "");
    } else {
        ctx.set_state("find", find.sub_string(0, find.len() - 1));
    }
    find_refresh(ctx);
}

/// Enter: lay out only the matches, with the largest selected.
fn find_apply(ctx) {
    ctx.set_state("find_open", false);
    let find = ctx.get_state("find");
    if find.trim() == "" {
        return ();
    }
    find_refresh(ctx);
    let layout = ctx.get_app_store("disktree", "layout");
    if layout.find_summary == () || layout.find_summary.count == 0 {
        return ();
    }
    layout.filter = find;
    ctx.set_app_store("disktree", "layout", layout);
    // The largest match under the current root.
    let metric = ctx.get_state("metric");
    let reply = ctx.call_capability("disktree.tree", "children", #{
        crumbs: layout.crumbs,
        metric: metric,
        max: 1,
        needle: find,
    });
    if reply.entries.len() > 0 {
        let crumbs = layout.crumbs + [reply.entries[0].index];
        set_selection(ctx, crumbs);
    }
}

fn find_close(ctx) {
    ctx.set_state("find_open", false);
    ctx.set_state("find", "");
    find_refresh(ctx);
}

/// Escape with a filter on: back to the whole directory.
fn clear_filter(ctx) {
    let layout = ctx.get_app_store("disktree", "layout");
    layout.filter = "";
    ctx.set_app_store("disktree", "layout", layout);
    ctx.set_state("find", "");
    ctx.set_state("find_open", false);
}

// -- pointer & wheel -------------------------------------------------------------

/// A canvas key like `f:2.5.1` or `o:2.5.1` — the tile key after the
/// first ':'.
fn key_tail(key) {
    let marker = key.index_of(':');
    if marker < 0 {
        return "";
    }
    key.sub_string(marker + 1)
}

fn tile_key(tile) {
    let key = crumb_key(tile.crumbs);
    if tile.kind == "others" {
        return `${key}_o`;
    }
    key
}

fn clear_hover(ctx) {
    let cursor = ctx.get_app_store("disktree", "cursor");
    if cursor.crumbs != () {
        ctx.set_app_store(
            "disktree",
            "cursor",
            #{
                crumbs: (),
                name: "",
                x: 0.0,
                y: 0.0,
                selected: cursor.selected,
                active: false,
            }
        );
    }
}

fn on_pointer_move(ctx, event) {
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid {
        return ();
    }
    if event.canvas_key == () {
        clear_hover(ctx);
        return ();
    }
    let tile = tile_lookup(layout, key_tail(event.canvas_key));
    if tile == () {
        clear_hover(ctx);
        return ();
    }
    let cursor = ctx.get_app_store("disktree", "cursor");
    let same = cursor.crumbs != () && crumb_key(cursor.crumbs) == crumb_key(tile.crumbs);
    if !same {
        ctx.set_app_store(
            "disktree",
            "cursor",
            #{
                crumbs: tile.crumbs,
                name: tile.entry.name,
                x: event.local.x,
                y: event.local.y,
                selected: cursor.selected,
                active: true,
            }
        );
    } else if !cursor.active {
        cursor.active = true;
        ctx.set_app_store("disktree", "cursor", cursor);
    }
    ()
}

/// Click: select, then act on a repeat click, like a file manager. The side
/// buttons walk history. Marks (ctrl/middle-click) belong to the deletion
/// system, which this skin does not have.
fn on_pointer_down(ctx, event) {
    ctx.focus("root");
    let button = event.button;
    if button == () {
        return ();
    }
    if button == "back" {
        nav_history(ctx, true);
        return ();
    }
    if button == "forward" {
        nav_history(ctx, false);
        return ();
    }
    if button == "right" {
        // 右键：在文件管理器中显示所指瓦片，同 tobi。
        let layout = ctx.get_app_store("disktree", "layout");
        if layout.valid && event.canvas_key != () {
            let tile = tile_lookup(layout, key_tail(event.canvas_key));
            if tile != () && tile.kind != "others" {
                reveal_tile(ctx, tile.crumbs);
            }
        }
        return ();
    }
    if button != "left" {
        return ();
    }
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid {
        return ();
    }
    let tile = ();
    if event.canvas_key != () {
        tile = tile_lookup(layout, key_tail(event.canvas_key));
    }
    let cursor = ctx.get_app_store("disktree", "cursor");
    if tile != () {
        if event.click_count >= 2 {
            set_selection(ctx, tile.crumbs);
            nav_descend(ctx);
            return ();
        }
        let activate = cursor.selected != ()
            && crumb_key(cursor.selected) == tile_key(tile)
            && tile.kind == "dir";
        ctx.set_app_store(
            "disktree",
            "cursor",
            #{
                crumbs: tile.crumbs,
                name: tile.entry.name,
                x: event.local.x,
                y: event.local.y,
                selected: tile.crumbs,
                active: true,
            }
        );
        if activate {
            nav_descend(ctx);
        }
        return ();
    }
    // A click on nothing drawable drops the selection.
    set_selection(ctx, ());
}

/// The wheel zooms toward the pointer; shift pans a magnified view. Pixel
/// deltas are normalized at 24px per notch, like tobi's line conversion.
fn on_wheel(ctx, event) {
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid {
        return ();
    }
    let lines = event.delta.y / 24.0;
    if lines < 0.0001 && lines > -0.0001 {
        return ();
    }
    if event.modifiers.shift {
        let view = layout.view;
        let oy = view.oy - lines * 40.0 / view.scale;
        view.oy = 0.0;
        if oy > 0.0 {
            view.oy = oy;
        }
        layout.view = view;
        ctx.set_app_store("disktree", "layout", layout);
        return ();
    }
    let factor = 1.0 / 1.15;
    if lines > 0.0 {
        factor = 1.15;
    }
    zoom_at(ctx, event.local.x, event.local.y, factor, true);
}

// -- themes ----------------------------------------------------------------------

/// Every mounted palette: the skin's own two first, then gpui-rhai's
/// bundled set. Entries are [family, name, label].
fn theme_entries() {
    [
        ["Disktree", "Minimal Gray", "Minimal Gray"],
        ["Disktree", "Flexoki Light", "Flexoki Light"],
        ["Default", "Dark", "Default Dark"],
        ["Default", "Light", "Default Light"],
        ["Nord", "Dark", "Nord Dark"],
        ["Gruvbox", "Dark", "Gruvbox Dark"],
        ["Catppuccin", "Latte", "Catppuccin Latte"],
        ["Catppuccin", "Mocha", "Catppuccin Mocha"],
        ["Everforest", "Dark", "Everforest Dark"],
        ["Tokyo Night", "Night", "Tokyo Night"],
        ["Tokyo Night", "Storm", "Tokyo Storm"],
        ["Aetheria", "Dark", "Aetheria Dark"],
        ["Ethereal", "Dark", "Ethereal Dark"],
        ["Futurism", "Dark", "Futurism Dark"],
        ["Hackerman", "Dark", "Hackerman Dark"],
        ["Hermarchy", "Dark", "Hermarchy Dark"],
        ["Retro 82", "Dark", "Retro 82"],
    ]
}

fn theme_key(family, name) {
    `${family}::${name}`
}

/// A theme switch re-colours the mosaic: the paint rows carry resolved
/// colours, so the on-screen mode's paint is rebuilt for the new palette.
fn repaint_after_theme(ctx) {
    let layout = ctx.get_app_store("disktree", "layout");
    if layout.valid {
        let mode_now = ctx.get_state("mode");
        ctx.set_app_store(
            "disktree",
            `paint${mode_now}`,
            build_paint(ctx, layout.tiles, mode_now, layout.crumbs)
        );
    }
}

/// `e`: the next palette, wrapping.
fn cycle_theme(ctx) {
    let variant = ctx.theme_variant();
    let current = "";
    if variant != () {
        current = theme_key(variant.family, variant.name);
    }
    let entries = theme_entries_from(ctx);
    let next = 0;
    let i = 0;
    while i < entries.len() {
        if theme_key(entries[i].family, entries[i].name) == current {
            next = (i + 1) % entries.len();
        }
        i += 1;
    }
    let pick = entries[next];
    ctx.set_theme(pick.family, pick.name);
    repaint_after_theme(ctx);
}

// -- toolbar actions -------------------------------------------------------------

fn toggle_hidden(ctx, payload) {
    let hidden = ctx.get_state("no_hidden");
    ctx.set_state("no_hidden", !hidden);
}

fn toggle_apparent(ctx, payload) {
    let apparent = ctx.get_state("apparent");
    ctx.set_state("apparent", !apparent);
}

fn mode_clicked(ctx, mode) {
    set_mode(ctx, mode);
}

fn depth_clicked(ctx, step) {
    adjust_depth(ctx, step);
}

fn help_clicked(ctx, payload) {
    let open = ctx.get_state("help_open");
    ctx.set_state("help_open", !open);
}

/// A crumb segment was clicked: jump there.
fn crumb_clicked(ctx, index) {
    let layout = ctx.get_app_store("disktree", "layout");
    let crumbs = [];
    let i = 0;
    while i <= index && i < layout.crumbs.len() {
        crumbs.push(layout.crumbs[i]);
        i += 1;
    }
    nav_go_to(ctx, crumbs);
}

fn back_clicked(ctx, payload) {
    nav_history(ctx, true);
}

fn forward_clicked(ctx, payload) {
    nav_history(ctx, false);
}

fn insight_clicked(ctx, index) {
    let layout = ctx.get_app_store("disktree", "layout");
    if index >= 0 && index < layout.insights.len() {
        set_selection(ctx, layout.insights[index].crumbs);
    }
}

fn open_clicked(ctx, crumbs) {
    nav_enter(ctx, crumbs);
}

fn reveal_clicked(ctx, crumbs) {
    reveal_tile(ctx, crumbs);
}

/// Reveal one crumb path in the file manager: the `o` key and a
/// right-click on a tile share it.
fn reveal_tile(ctx, crumbs) {
    if crumbs == () || crumbs.len() == 0 {
        return ();
    }
    let path = current_path(ctx, crumbs);
    ctx.start_task(
        "disktree.open",
        "reveal",
        path,
        Fn("reveal_done"),
        Fn("reveal_failed")
    );
}

// -- rendering ----------------------------------------------------------------

fn render_ScanRoot(ctx, props) {
    let running = ctx.get_state("running");
    let metric = ctx.get_state("metric");
    let apparent = ctx.get_state("apparent");
    let no_hidden = ctx.get_state("no_hidden");
    let depth = ctx.get_state("depth");
    let mode = ctx.get_state("mode");
    let help_open = ctx.get_state("help_open");
    let sidebar_open = ctx.get_state("sidebar_open");
    let find_open = ctx.get_state("find_open");
    effect(
        "scan",
        [running, ctx.get_state("epoch"), apparent, no_hidden],
        Fn("scan_start"),
        Fn("scan_stop")
    );

    let scan = ctx.get_app_store("disktree", "scan");
    let layout = ctx.get_app_store("disktree", "layout");
    let cursor = ctx.get_app_store("disktree", "cursor");

    // The viewport drives the layout through a tracked bounds read; the
    // effect deps recompute only when something real changed.
    let viewport_ref = element_ref("mosaic");
    let raw_bounds = ctx.element_bounds(viewport_ref);
    let size = view_size(raw_bounds);
    let nav_key = "";
    if layout.valid {
        nav_key = layout.nav_key;
    }
    let filter = layout.filter;
    effect(
        "layout",
        [scan.phase, nav_key, metric, depth, size[0], size[1], filter],
        Fn("layout_recompute"),
        Fn("layout_stop")
    );

    // A navigation transition steps itself: while one runs, redeclare the
    // timer with the next step number (the changed payload re-arms it).
    if layout.anim.active {
        timeout("nav-anim", 16, false, Fn("anim_tick"), layout.anim.step);
    }

    // The resolved variant drives the canvas palette; the read is a tracked
    // theme dependency, so a system appearance flip re-renders the mosaic.
    let light = is_light(ctx);
    // 极简皮肤没有侧栏：只有马赛克。
    // The active mode's paint, from its own store value (G9). `()` until the
    // layout effect fills it; mosaic_nodes renders an empty mosaic then.
    let paint = ctx.get_app_store("disktree", `paint${mode}`);
    let with_side = stage(scan, layout, cursor, metric, mode, viewport_ref, help_open, light, paint);
    let find = ctx.get_state("find");
    let body_rows = [header(metric, apparent, no_hidden, depth, mode, layout)];
    if find_open || layout.filter != "" {
        body_rows.push(find_bar(find_open, find, layout));
    }
    body_rows.push(with_side);
    body_rows.push(footer(scan, layout));
    with_keys(
        column(body_rows)
            .with_key("scan_root")
            .with_ref(element_ref("root"))
            .with_style(
                ctx.component_style(
                    "root",
                    style()
                        .flex_col()
                        .width(relative(1.0))
                        .height(relative(1.0))
                        .background(theme_color("surface"))
                )
            )
    )
}

/// Element bounds may be () on the very first render; the runtime re-renders
/// once prepaint reports geometry.
fn view_size(bounds) {
    if bounds == () {
        return [0.0, 0.0];
    }
    let layout_bounds = bounds.layout;
    if layout_bounds == () || layout_bounds.width == () {
        return [0.0, 0.0];
    }
    [layout_bounds.width, layout_bounds.height]
}

/// Every key the mosaic answers, attached to whichever node holds the
/// viewport (the scan panel while reading, the mosaic afterwards).
fn with_keys(node) {
    node
        .on_key_value("enter", Fn("on_key"), "enter")
        .on_key_value("left", Fn("on_key"), "left")
        .on_key_value("right", Fn("on_key"), "right")
        .on_key_value("up", Fn("on_key"), "up")
        .on_key_value("down", Fn("on_key"), "down")
        .on_key_value("h", Fn("on_key"), "h")
        .on_key_value("j", Fn("on_key"), "j")
        .on_key_value("k", Fn("on_key"), "k")
        .on_key_value("l", Fn("on_key"), "l")
        .on_key_value("tab", Fn("on_key"), "tab")
        .on_key_value("backspace", Fn("on_key"), "backspace")
        .on_key_value("u", Fn("on_key"), "u")
        .on_key_value("escape", Fn("on_key"), "escape")
        .on_key_value("[", Fn("on_key"), "[")
        .on_key_value("]", Fn("on_key"), "]")
        .on_key_value("-", Fn("on_key"), "-")
        .on_key_value("=", Fn("on_key"), "=")
        .on_key_value("+", Fn("on_key"), "+")
        .on_key_value("0", Fn("on_key"), "0")
        .on_key_value("t", Fn("on_key"), "t")
        .on_key_value("r", Fn("on_key"), "r")
        .on_key_value("i", Fn("on_key"), "i")
        .on_key_value("d", Fn("on_key"), "d")
        .on_key_value("p", Fn("on_key"), "p")
        .on_key_value("o", Fn("on_key"), "o")
        .on_key_value("?", Fn("on_key"), "?")
        .on_key_value("q", Fn("on_key"), "q")
        .on_key_value("s", Fn("on_key"), "s")
        .on_key_value("a", Fn("on_key"), "a")
        .on_key_value("b", Fn("on_key"), "b")
        .on_key_value("c", Fn("on_key"), "c")
        .on_key_value("e", Fn("on_key"), "e")
        .on_key_value("f", Fn("on_key"), "f")
        .on_key_value("g", Fn("on_key"), "g")
        .on_key_value("k", Fn("on_key"), "k")
        .on_key_value("m", Fn("on_key"), "m")
        .on_key_value("n", Fn("on_key"), "n")
        .on_key_value("v", Fn("on_key"), "v")
        .on_key_value("w", Fn("on_key"), "w")
        .on_key_value("x", Fn("on_key"), "x")
        .on_key_value("y", Fn("on_key"), "y")
        .on_key_value("z", Fn("on_key"), "z")
        .on_key_value("1", Fn("on_key"), "1")
        .on_key_value("2", Fn("on_key"), "2")
        .on_key_value("3", Fn("on_key"), "3")
        .on_key_value("4", Fn("on_key"), "4")
        .on_key_value("5", Fn("on_key"), "5")
        .on_key_value("6", Fn("on_key"), "6")
        .on_key_value("7", Fn("on_key"), "7")
        .on_key_value("8", Fn("on_key"), "8")
        .on_key_value("9", Fn("on_key"), "9")
        .on_key_value(".", Fn("on_key"), ".")
        .on_key_value("_", Fn("on_key"), "_")
        .on_key_value("space", Fn("on_key"), " ")
}

fn logo() {
    row([
        text("disktree").with_style(
            style().font_size(px(15.0)).font_weight(700).text_color(theme_color("text_primary"))
        ),
        text("-rhai").with_style(
            style().font_size(px(15.0)).font_weight(700).text_color(theme_color("accent"))
        ),
    ])
    .with_style(style().flex_row())
}

/// The header: logo and history buttons, breadcrumbs, and the scan controls
/// (mode segmented, hidden/apparent checkboxes, depth stepper).
fn header(metric, apparent, no_hidden, depth, mode, layout) {
    column([
        row([logo(), breadcrumbs(layout)])
            .with_style(
                style()
                    .flex_row()
                    .items_center()
                    .width(relative(1.0))
                    .gap(px(10.0))
            ),
        // 极简皮肤：模式/深度/可见性全部交给键盘，控件行不画。
        row([])
        .with_style(
            style()
                .flex_row()
                .items_center()
                .gap(px(14.0))
                .width(relative(1.0))
            ),
    ])
    // 极简皮肤不用 TitleBar 组件,但窗口现在是全高内容视图:头部自己作为
    // 拖拽区(inset 起点给 macOS 红绿灯留位)。
    .window_drag_area()
    .with_style(
        style()
            .flex_col()
            .gap(px(8.0))
            .padding_start(px(78.0))
            .padding_end(px(16.0))
            .padding_y(px(10.0))
            .border_bottom(px(1.0))
            .border_color(theme_color("border"))
    )
}

fn history_button(label, enabled, handler) {
    button::Button(#{ text: label, variant: "ghost", disabled: !enabled, on_click: handler })
}

/// The breadcrumb bar: history buttons, the root path, then the crumb path.
/// Each crumb jumps straight to its directory.
fn breadcrumbs(layout) {
    let nodes = [
        history_button("<", layout.valid && layout.back.len() > 0, Fn("back_clicked")),
        history_button(">", layout.valid && layout.forward.len() > 0, Fn("forward_clicked")),
    ];
    let parts = split_path(layout.root);
    for part in parts {
        nodes.push(
            text(part).with_style(
                style()
                    .font_size(px(12.0))
                    .text_color(theme_color("text_muted"))
                    .whitespace_nowrap().text_ellipsis()
            )
        );
        nodes.push(
            text("/").with_style(
                style().font_size(px(12.0)).text_color(theme_color("border"))
            )
        );
    }
    let i = 0;
    while i < layout.names.len() {
        let active = i == layout.names.len() - 1;
        let label = layout.names[i];
        if label == "" {
            label = "?";
        }
        nodes.push(crumb_segment(label, active, i));
        if !active {
            nodes.push(
                text("/").with_style(
                    style().font_size(px(12.0)).text_color(theme_color("border"))
                )
            );
        }
        i += 1;
    }
    row(nodes)
        .with_style(
            style()
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .flex_grow()
                .overflow_hidden()
        )
}

fn crumb_segment(label, active, index) {
    let weight = 400;
    if active {
        weight = 700;
    }
    box([text(label).with_style(
        style()
            .font_size(px(12.0))
            .font_weight(weight)
            .text_color(theme_color("text_primary"))
            .whitespace_nowrap().text_ellipsis()
            .max_width(px(180.0))
    )])
    .on_click_value(Fn("crumb_clicked"), index)
    .with_style(
        style()
            .padding_x(px(6.0))
            .padding_y(px(2.0))
            .flex_shrink(false)
            .hover(style().background(theme_color("surface_raised")))
    )
}

/// Split a display path into its visible segments. A `~` home shortening
/// stays one segment.
fn split_path(path) {
    let parts = [];
    let rest = path;
    if rest.len() == 0 {
        return parts;
    }
    if rest.sub_string(0, 1) == "~" {
        parts.push("~");
        if rest.len() > 1 {
            rest = rest.sub_string(2);
        } else {
            return parts;
        }
    }
    if rest.len() == 0 {
        return parts;
    }
    let start = 0;
    if rest.sub_string(0, 1) == "/" {
        start = 1;
    }
    let i = start;
    while i <= rest.len() {
        let cut = rest.index_of("/", i);
        if cut < 0 {
            if i < rest.len() {
                parts.push(rest.sub_string(i));
            }
            break;
        }
        if cut > i {
            parts.push(rest.sub_string(i, cut - i));
        }
        i = cut + 1;
    }
    parts
}

fn checkbox(label, checked, handler) {
    let mark = " ";
    if checked {
        mark = "×";
    }
    let mark_color = theme_color("text_muted");
    if checked {
        mark_color = theme_color("accent");
    }
    row([
        box([text(mark).with_style(
            style()
                .font_size(px(10.0))
                .text_color(mark_color)
        )])
        .with_style(
            style()
                .width(px(14.0))
                .height(px(14.0))
                .items_center()
                .justify_center()
                .flex_shrink(false)
                .border(px(1.0))
                .border_color(theme_color("border"))
        ),
        text(label).with_style(
            style().font_size(px(11.0)).text_color(theme_color("text_muted"))
        ),
    ])
    .on_click(handler)
    .with_style(
        style()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .flex_shrink(false)
    )
}

/// Size | Files | Age: what areas measure and what colour says.
fn mode_segmented(mode) {
    row([
        segment("Size", mode == 0, 0),
        segment("Files", mode == 1, 1),
        segment("Age", mode == 2, 2),
    ])
    .with_style(
        style()
            .flex_row()
            .items_center()
            .border(px(1.0))
            .border_color(theme_color("border"))
            .flex_shrink(false)
    )
}

fn segment(label, active, mode) {
    let color = theme_color("text_muted");
    let weight = 400;
    let background = theme_color("surface");
    if active {
        color = theme_color("on_accent");
        weight = 600;
        background = theme_color("accent");
    }
    box([text(label).with_style(
        style()
            .font_size(px(11.0))
            .font_weight(weight)
            .text_color(color)
    )])
    .on_click_value(Fn("mode_clicked"), mode)
    .with_style(
        style()
            .padding_x(px(10.0))
            .padding_y(px(4.0))
            .background(background)
    )
}

fn depth_stepper(depth) {
    row([
        text(`depth ${depth}`).with_style(
            style().font_size(px(11.0)).text_color(theme_color("text_muted"))
        ),
        step_button("−", -1, depth <= 1),
        step_button("+", 1, depth >= 6),
    ])
    .with_style(
        style()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .padding_x(px(10.0))
            .padding_y(px(3.0))
            .border(px(1.0))
            .border_color(theme_color("border"))
            .flex_shrink(false)
    )
}

fn step_button(label, step, disabled) {
    let color = theme_color("text_muted");
    if disabled {
        color = theme_color("border");
    }
    let node = box([text(label).with_style(
        style()
            .font_size(px(11.0))
            .text_color(color)
            .width(px(16.0))
            .flex_shrink(false)
    )]);
    if disabled {
        return node;
    }
    node.on_click_value(Fn("depth_clicked"), step)
}

/// tobi's progress estimate: a slow curve that never quite reaches 1.
fn progress_estimate(bytes) {
    if bytes <= 0 {
        return 0.06;
    }
    let gb = bytes.to_float() / 1073741824.0;
    0.06 + 0.93 * (1.0 - 1.0 / (1.0 + gb / 25.0))
}

fn scan_panel(scan) {
    let reading = scan.phase == "scanning";
    let status = if reading {
        `Reading ${scan.root}…`
    } else if scan.phase == "cancelled" {
        `Stopped reading ${scan.root}`
    } else if scan.phase == "failed" {
        `Reading ${scan.root} failed`
    } else {
        `Read ${scan.root}`
    };
    let status_row = row([
        if reading {
            spinner::Spinner(#{ key: "scan-spinner", label: "Reading" })
        } else {
            box([])
        },
        text(status).with_style(
            style()
                .typography("subtitle")
                .text_color(theme_color("text_primary"))
        ),
    ])
    .with_style(style().flex_row().items_center().gap(px(10.0)).padding_bottom(px(14.0)));
    let figures = stat::stats([
        #{ label: "files", value: core::human_count(scan.files) },
        #{ label: "directories", value: core::human_count(scan.dirs) },
        #{ label: "measured", value: core::human_bytes(scan.bytes) },
        #{ label: "unreadable", value: core::human_count(scan.errors) },
    ]);
    let action = if reading {
        button::Button(#{ text: "Cancel", variant: "outline", on_click: Fn("scan_cancel") })
    } else {
        button::Button(#{ text: "Scan again", variant: "primary", on_click: Fn("scan_again") })
    };
    let note = if scan.phase == "cancelled" {
        "Nothing is shown from a scan that did not finish. Scan again to retry."
    } else if scan.phase == "failed" {
        if scan.messages.len() > 0 { scan.messages[0] } else { "the scan failed" }
    } else {
        "Sizes are allocated blocks, like du: hard links count once, symlinks are not followed, hidden files are included."
    };

    column([
        status_row,
        figures,
        box([progress::Progress(#{
            key: "scan-progress",
            label: "Scan progress",
            value: progress_estimate(scan.bytes) * 100.0,
            max: 100.0,
            width: 504,
        })])
        .with_style(style().padding_y(px(14.0))),
        row([
            text(note).with_style(
                style()
                    .typography("caption")
                    .text_color(theme_color("text_muted"))
                    .flex_grow()
            ),
            action,
        ])
        .with_style(style().flex_row().items_center().gap(px(12.0))),
    ])
    .with_style(
        style()
            .flex_col()
            .width(px(560.0))
            .padding(px(28.0))
            .gap(px(6.0))
            .self_center()
    )
}

/// Styles reject negative lengths; a sliver under zoom can go below zero.
fn clamp_min(v) {
    if v < 0.0 {
        return 0.0;
    }
    v
}

/// The find field: a three-key input (type, backspace, enter) with a live
/// match count; once applied it states what the mosaic is filtered by.
fn find_bar(open, find, layout) {
    let left = "";
    let right = "";
    if open {
        let needle = find;
        if needle == "" {
            needle = "type to filter";
        }
        left = `find: ${needle}`;
        right = "enter apply · esc close";
    } else {
        left = `filter: ${layout.filter}`;
    }
    if layout.find_summary != () {
        let summary = layout.find_summary;
        right = `${core::human_count(summary.count)} matches · ${core::human_bytes(summary.bytes)} · esc clears`;
    }
    row([
        text(left).with_style(
            style()
                .font_size(px(12.0))
                .font_weight(600)
                .text_color(theme_color("text_primary"))
                .flex_grow()
        ),
        text(right).with_style(
            style().font_size(px(11.0)).text_color(theme_color("text_muted"))
        ),
    ])
    .with_key("find_bar")
    .with_style(
        style()
            .flex_row()
            .items_center()
            .padding_x(px(16.0))
            .padding_y(px(6.0))
            .border_bottom(px(1.0))
            .border_color(theme_color("border"))
    )
}

/// The main stage: the scan panel while scanning, the mosaic when a layout
/// is cached. The help overlay floats above both.
fn stage(scan, layout, cursor, metric, mode, viewport_ref, help_open, light, paint) {
    let content = ();
    if !layout.valid {
        content = box([scan_panel(scan)])
            .with_key("mosaic")
            .with_ref(viewport_ref)
            .with_style(
                style()
                    .flex_grow()
                    .width(relative(1.0))
                    .flex_col()
                    .justify_center()
            );
    } else {
        content = box(
            mosaic_nodes(layout, cursor, metric, scan, paint)
        )
            .with_key("mosaic")
            .with_ref(viewport_ref)
            .with_style(
                style()
                    .flex_grow()
                    .width(relative(1.0))
                    .relative()
                    .overflow_hidden()
                    .margin_x(px(4.0))
                    .margin_y(px(4.0))
            )
            .on("wheel", Fn("on_wheel"));
    }
    if help_open {
        content = box([content, help_overlay()])
            .with_style(style().relative().flex_grow().width(relative(1.0)));
    }
    content
}

/// The mosaic as three layers: the cached base canvas (fills, strips,
/// hatch — identical nodes across hover crossings, so the retained renderer
/// keeps it untouched), a tiny ring canvas rebuilt per crossing, and the
/// cached labels. Pointer events ride the base canvas.
fn mosaic_nodes(layout, cursor, metric, scan, paint) {
    if paint == () {
        paint = #{ base: [], labels: [], out_keys: #{} };
    }
    let base = canvas(canvas_scene(paint_commands(paint.base, layout.view)))
        .with_key("mosaic_canvas")
        .with_style(
            style()
                .absolute()
                .left(px(0.0))
                .top(px(0.0))
                .width(px(layout.width))
                .height(px(layout.height))
        )
        .on("pointer_move", Fn("on_pointer_move"))
        .on("pointer_down", Fn("on_pointer_down"));
    let rings = canvas(canvas_scene(ring_commands(layout, cursor)))
        .with_key("mosaic_rings")
        .with_style(
            style()
                .absolute()
                .left(px(0.0))
                .top(px(0.0))
                .width(px(layout.width))
                .height(px(layout.height))
        );
    [base, rings]
        + label_nodes(paint.labels, layout, metric, paint.out_keys)
        + tooltip_nodes(layout, cursor, metric, scan)
}

/// Cached base-coordinate rows back into canvas commands, projected
/// through the view: the only work a render does over the paint.
fn paint_commands(rows, view) {
    let scale = view.scale;
    let ox = view.ox;
    let oy = view.oy;
    let commands = [];
    // Rows are maps, not arrays: rhai's data-size limit counts array
    // elements cumulatively across the whole store value (10k), while maps
    // get their own far larger budget — see docs/UPSTREAM_GAPS.md G9.
    for row in rows {
        if row.t == "r" {
            commands.push(canvas_rect(
                row.k,
                (row.x - ox) * scale,
                (row.y - oy) * scale,
                row.w * scale,
                row.h * scale,
                rgba(row.c)
            ));
        } else if row.t == "p" {
            // Slash lines 6px apart (screen space), each the exact
            // 45-degree segment inside the projected rect.
            let r = row.r;
            let x0 = (r.x - ox) * scale;
            let y0 = (r.y - oy) * scale;
            let w = r.w * scale;
            let h = r.h * scale;
            let segments = [];
            let x = x0 - h;
            while x < x0 + w {
                let from = 0.0;
                if x0 - x > from {
                    from = x0 - x;
                }
                let to = h;
                if x0 + w - x < to {
                    to = x0 + w - x;
                }
                if to - from > 1.0 * scale {
                    segments.push(path_move(x + from, y0 + h - from));
                    segments.push(path_line(x + to, y0 + h - to));
                }
                x += 6.0;
            }
            if segments.len() > 0 {
                commands.push(canvas_stroke_path(row.k, segments, row.w, rgba(row.c)));
            }
        }
    }
    commands
}

/// The commands of one tile: fill (category, or age in Age mode), category
/// strip (top level), hatch for reclaimable space, then outline rings.
/// Fill, strip, hatch — the per-tile paint that never changes with hover,
/// as plain rows: ["r", key, x, y, w, h, color] / ["l", key, x1, y1, x2,
/// y2, width, color]. Colors are already resolved ints, so turning a row
/// back into a command at render is one native call.
fn tile_base_commands(tile, mode, light, now, out_keys) {
    let key = tile_key(tile);
    let dim = key in out_keys;
    let r = tile.rect;
    let fill = ();
    if tile.kind == "others" {
        // The merged tail has no entry of its own; it wears the neutral tone.
        fill = core::category_fill_mode("other", tile.depth, light, dim);
    } else if mode == 2 {
        fill = core::age_fill_mode(core::age_days(tile.entry.modified, now), light, dim);
    } else {
        fill = core::category_fill_mode(tile.entry.category, tile.depth, light, dim);
    }
    let rows = [
        #{ t: "r", k: `f:${key}`, x: r.x, y: r.y, w: r.w, h: r.h, c: fill },
    ];
    // 极简皮肤：不画分类条纹，不画回收斜纹——面就是信息。
    rows
}

/// Hover and selection rings: the only per-crossing paint, two commands at
/// most, rebuilt on every render from the cursor.
fn ring_commands(layout, cursor) {
    let commands = [];
    let selected_key = "";
    if cursor.selected != () {
        selected_key = crumb_key(cursor.selected);
    }
    let targets = [];
    if cursor.crumbs != () {
        let hover_key = crumb_key(cursor.crumbs);
        // Hovering the selection is one ring, not two: a scene rejects
        // duplicate keys, and the selected style wins.
        if hover_key != selected_key {
            targets.push(["hover", hover_key]);
        }
    }
    if selected_key != "" {
        targets.push(["selected", selected_key]);
    }
    for pair in targets {
        let kind = pair[0];
        let key = pair[1];
        let tile = tile_lookup(layout, key);
        if tile == () {
            continue;
        }
        let color = 0xc0caf5cc;
        let width = 1.0;
        if kind == "selected" {
            color = 0xe0af68ff;
            width = 2.0;
        }
        let r = core::view_project(layout.view, tile.rect);
        commands.push(
            canvas_stroke_path(
                `o:${key}`,
                [
                    path_move(r.x, r.y),
                    path_line(r.x + r.w, r.y),
                    path_line(r.x + r.w, r.y + r.h),
                    path_line(r.x, r.y + r.h),
                    path_close(),
                ],
                width,
                rgba(color)
            )
        );
    }
    commands
}

/// Absolutely-positioned labels: name (bold in a header band) plus size,
/// projected with the tiles, at most 150 by area, only on tiles big enough
/// to read (54x15px, like tobi's screen-space thresholds).
/// Area-sorted label candidates, largest first: the sort is the expensive
/// part, the screen-space threshold is applied per render (it follows the
/// zoom).
fn sorted_label_candidates(tiles) {
    // Indices of label-worthy tiles, area-sorted largest first: the sort is
    // the expensive part and it never changes with hover; the screen-space
    // threshold is applied per render (it follows the zoom).
    let candidates = [];
    let i = 0;
    while i < tiles.len() {
        let tile = tiles[i];
        if tile.rect.w >= 20.0 && tile.rect.h >= 8.0 {
            candidates.push([tile.rect.w * tile.rect.h, i]);
        }
        i += 1;
    }
    let sorted = [];
    for pair in candidates {
        let j = 0;
        while j < sorted.len() && sorted[j][0] >= pair[0] {
            j += 1;
        }
        sorted.insert(j, pair);
    }
    let indices = [];
    for pair in sorted {
        indices.push(pair[1]);
    }
    indices
}
/// The label nodes for one render: screen-space threshold over the cached,
/// area-sorted candidates, at most 150.
fn label_nodes(indices, layout, metric, out_keys) {
    let labels = [];
    let count = 0;
    for index in indices {
        if count >= 150 {
            break;
        }
        let tile = layout.tiles[index];
        let r = core::view_project(layout.view, tile.rect);
        if r.w < 54.0 || r.h < 15.0 {
            continue;
        }
        let label = tile_label(tile, metric, layout.view, tile_key(tile) in out_keys);
        if label != () {
            labels.push(label);
            count += 1;
        }
    }
    labels
}

fn tile_label(tile, metric, view, dim) {
    let key = tile_key(tile);
    let r = core::view_project(view, tile.rect);
    if tile.kind == "others" {
        return text(`+${tile.others} more`)
            .with_key(`l:${key}`)
            .with_style(
                style()
                    .absolute()
                    .left(px(clamp_min(r.x + 5.0)))
                    .top(px(clamp_min(r.y + 3.0)))
                    .width(px(clamp_min(r.w - 8.0)))
                    .font_size(px(11.0))
                    .text_color(theme_color("text_muted"))
                    .overflow_hidden()
                    .whitespace_nowrap().text_ellipsis()
            );
    }
    let value = if metric == "files" {
        core::human_count(tile.entry.files)
    } else {
        core::human_bytes_short(tile.entry.bytes)
    };
    let name = tile.entry.name;
    if tile.header != () {
        // A subdivided directory: bold name in its band, size at the end.
        let band = core::view_project(view, tile.header);
        let label_color = theme_color("text_primary");
        let value_color = theme_color("text_muted");
        if dim {
            label_color = theme_color("disabled");
            value_color = theme_color("disabled");
        }
        return row([
            text(name).with_style(
                style()
                    .font_size(px(12.0))
                    .font_weight(700)
                    .text_color(label_color)
                    .overflow_hidden()
                    .whitespace_nowrap().text_ellipsis()
                    .flex_grow()
            ),
            text(value).with_style(
                style()
                    .font_size(px(11.0))
                    .text_color(value_color)
                    .whitespace_nowrap().text_ellipsis()
            ),
        ])
            .with_key(`l:${key}`)
            .on_click_value(Fn("tile_clicked"), key)
            .with_style(
                style()
                    .absolute()
                    .left(px(clamp_min(band.x + 5.0)))
                    .top(px(clamp_min(band.y + 2.0)))
                    .width(px(clamp_min(band.w - 10.0)))
                    .height(px(clamp_min(band.h)))
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .overflow_hidden()
            );
    }
    // A closed tile: name, with the size beneath when there is height.
    let stacked = r.h > 30.0;
    let label_color = theme_color("text_primary");
    let value_color = theme_color("text_muted");
    if dim {
        label_color = theme_color("disabled");
        value_color = theme_color("disabled");
    }
    let nodes = [
        text(name).with_style(
            style()
                .font_size(px(11.0))
                .text_color(label_color)
                .overflow_hidden()
                .whitespace_nowrap().text_ellipsis()
        ),
    ];
    if stacked {
        nodes.push(
            text(value).with_style(
                style()
                    .font_size(px(11.0))
                    .text_color(value_color)
                    .overflow_hidden()
                    .whitespace_nowrap().text_ellipsis()
            )
        );
    }
    column(nodes)
        .with_key(`l:${key}`)
        .on_click_value(Fn("tile_clicked"), key)
        .with_style(
            style()
                .absolute()
                .left(px(clamp_min(r.x + 5.0)))
                .top(px(clamp_min(r.y + 3.0)))
                .width(px(r.w - 8.0))
                .flex_col()
                .overflow_hidden()
        )
}

/// A label was clicked: treat it as a click on its tile.
fn tile_clicked(ctx, key) {
    let layout = ctx.get_app_store("disktree", "layout");
    if !layout.valid {
        return ();
    }
    let tile = tile_lookup(layout, key);
    if tile == () {
        return ();
    }
    let cursor = ctx.get_app_store("disktree", "cursor");
    let activate = cursor.selected != ()
        && crumb_key(cursor.selected) == tile_key(tile)
        && tile.kind == "dir";
    set_selection(ctx, tile.crumbs);
    if activate {
        nav_descend(ctx);
    }
}

/// The hover tooltip: a small card following the cursor, flipped away from
/// the window edges.
fn tooltip_nodes(layout, cursor, metric, scan) {
    if cursor.crumbs == () {
        return [];
    }
    let key = crumb_key(cursor.crumbs);
    let tile = ();
    let i = layout.tiles.len() - 1;
    while i >= 0 {
        let candidate = layout.tiles[i];
        if candidate.kind != "others" && crumb_key(candidate.crumbs) == key {
            tile = candidate;
        }
        i -= 1;
    }
    if tile == () {
        return [];
    }
    let value = if metric == "files" {
        `${core::human_count(tile.entry.files)} files`
    } else {
        core::human_bytes(tile.entry.bytes)
    };
    let kind = if tile.kind == "dir" {
        if tile.entry.dirs > 1 {
            `folder · ${core::human_count(tile.entry.dirs)} dirs`
        } else {
            "folder"
        }
    } else {
        "file"
    };
    let x = cursor.x + 14.0;
    if x > layout.width - 260.0 {
        x = cursor.x - 250.0;
    }
    let y = cursor.y + 16.0;
    if y > layout.height - 90.0 {
        y = cursor.y - 76.0;
    }
    [
        column([
            text(tile.entry.name).with_style(
                style()
                    .font_size(px(12.0))
                    .font_weight(600)
                    .text_color(theme_color("text_primary"))
                    .overflow_hidden()
                    .whitespace_nowrap().text_ellipsis()
                    .width(px(216.0))
            ),
            text(`${value} · ${kind} · ${ago_label(tile.entry.modified, scan.now)}`).with_style(
                style().font_size(px(11.0)).text_color(theme_color("text_muted"))
            ),
        ])
            .with_key("tooltip")
            .with_style(
                style()
                    .absolute()
                    .left(px(x))
                    .top(px(y))
                    .width(px(240.0))
                    .padding_x(px(10.0))
                    .padding_y(px(7.0))
                    .background(theme_color("surface_raised"))
                    .border(px(1.0))
                    .border_color(theme_color("border"))
                    .flex_col()
                    .gap(px(2.0))
            ),
    ]
}

/// "today" / "3 d ago" / "2 mo ago" — coarse, like a sidebar figure.
fn ago_label(modified, now) {
    let days = core::age_days(modified, now);
    if days <= 0 {
        return "today";
    }
    if days == 1 {
        return "yesterday";
    }
    if days < 30 {
        return `${days} d ago`;
    }
    if days < 365 {
        return `${days / 30} mo ago`;
    }
    `${days / 365} y ago`
}

// -- help overlay ----------------------------------------------------------------

/// One help row: the chord as real Kbd keycaps, the description beside it.
fn help_row(keys, description) {
    row([
        kbd::Kbd(#{ keys: keys, label: description })
            .with_style(style().flex_shrink(false).width(px(150.0))),
        text(description).with_style(
            style()
                .typography("body_small")
                .text_color(theme_color("text_muted"))
                .flex_grow()
        ),
    ])
    .with_style(style().flex_row().items_center().gap(px(10.0)).width(relative(1.0)))
}

fn help_overlay() {
    let rows = [
        help_row(["enter"], "open the selected directory"),
        help_row(["u", "⌫"], "up one level"),
        help_row(["h", "j", "k", "l"], "move between tiles"),
        help_row(["tab"], "next sibling by size"),
        help_row(["t"], "size · files · age"),
        help_row(["[", "]"], "fewer or more levels"),
        help_row(["-", "=", "0"], "zoom out, in, reset"),
        help_row(["wheel"], "zoom toward the pointer"),
        help_row(["shift", "wheel"], "pan"),
        help_row(["i", "d"], "hidden files · apparent size"),
        help_row(["r"], "rescan"),
        help_row(["e"], "cycle the theme"),
        help_row(["alt", "←", "→"], "history back / forward"),
        help_row(["p"], "show or hide this panel"),
        help_row(["o"], "reveal in the file manager"),
        help_row(["esc"], "close · drop selection · up"),
        help_row(["q"], "quit"),
    ];
    box([
        column([
            row([
                text("keys").with_style(
                    style()
                        .typography("title")
                        .text_color(theme_color("text_primary"))
                        .flex_grow()
                ),
                kbd::Kbd(#{ keys: ["?", "esc"], label: "Close help" }),
            ])
            .with_style(style().flex_row().items_center().width(relative(1.0))),
        ] + rows)
        .with_style(
            style()
                .flex_col()
                .gap(px(8.0))
                .width(px(460.0))
                .padding(px(24.0))
                .background(theme_color("surface"))
                .border(px(1.0))
                .border_color(theme_color("border"))
        ),
    ])
        .with_key("help")
        .with_style(
            style()
                .absolute()
                .left(px(0.0))
                .top(px(0.0))
                .width(relative(1.0))
                .height(relative(1.0))
                .items_center()
                .justify_center()
                .background(theme_color("scrim"))
        )
        .on("pointer_down", Fn("help_background"))
}

fn help_background(ctx, event) {
    ctx.set_state("help_open", false);
}

fn footer(scan, layout) {
    let status = if scan.phase == "scanning" {
        `scanning · ${core::human_count(scan.files)} entries · ${core::human_bytes(scan.bytes)}`
    } else if scan.phase == "done" && layout.valid {
        let tiles = layout.tiles.len();
        `${core::human_bytes(scan.bytes)} · ${core::human_count(scan.files)} files · ${core::human_count(scan.dirs)} dirs · ${tiles} tiles`
    } else {
        ""
    };
    row([
        row([
            footer_hint(["enter"], "open"),
            footer_hint(["u"], "up"),
            footer_hint(["/"], "filter"),
            footer_hint(["t"], "mode"),
            footer_hint(["[", "]"], "depth"),
            footer_hint(["?"], "keys"),
        ])
        .with_style(style().flex_row().items_center().gap(px(10.0))),
        text(status),
    ])
    .with_style(
        style()
            .flex_row()
            .items_center()
            .justify_between()
            .padding_x(px(16.0))
            .padding_y(px(6.0))
            .border_top(px(1.0))
            .border_color(theme_color("border"))
            .typography("caption")
            .text_color(theme_color("text_muted"))
    )
}

/// One chord + verb pair of the key legend.
fn footer_hint(keys, label) {
    row([
        kbd::Kbd(#{ keys: keys, label: label, appearance: "inline" }),
        text(label),
    ])
    .with_style(style().flex_row().items_center().gap(px(4.0)).flex_shrink(false))
}

// ---------------------------------------------------------------------------
// App entry
// ---------------------------------------------------------------------------

fn init(ctx) {
    let args = ctx.call_capability("disktree.args", "get", ());
    let home = ctx.call_capability("disktree.args", "home", ());
    // The colour mode comes from the host: --theme flag, else the Omarchy
    // theme, else dark. The theme switch itself is a script call — skins
    // own their variants.
    // The colour mode comes from the host: --theme flag, else the Omarchy
    // theme, else dark. "system" follows the window appearance; the skin
    // reads the resolved variant back at render (ctx.theme_variant).
    if args.theme_mode == "light" {
        ctx.set_theme("Disktree", "Flexoki Light");
    } else if args.theme_mode == "system" {
        ctx.set_theme_system("Disktree");
    } else {
        ctx.set_theme("Disktree", "Minimal Gray");
    }
    ctx.set_app_store("disktree", "args", #{ path: args.path, home: home });
    ctx.set_app_store("disktree", "scan", #{
        phase: "idle",
        root: core::display_path(args.path, home),
        files: 0,
        dirs: 0,
        bytes: 0,
        errors: 0,
        messages: [],
        space: (),
        device: (),
        insights: [],
        elapsed_ms: 0,
        now: 0,
    });
    reset_derived_layout(ctx);
    ctx.set_app_store(
        "disktree",
        "cursor",
        #{ crumbs: (), name: "", x: 0.0, y: 0.0, selected: (), active: false }
    );
    // History chords arrive as host-bound actions.
    ctx.register_action("disktree.back", Fn("action_back"));
    ctx.register_action("disktree.forward", Fn("action_forward"));
}

fn view(ctx) {
    ScanRoot()
}
