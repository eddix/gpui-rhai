// Shared treemap geometry — the skin-level half of the disktree engine.
//
// This is a faithful Rhai port of tobi/disktree crates/disktree-core/src/treemap.rs
// (squarify / worst_ratio) plus the hit-test rule. Skins import this module to get
// the default squarified layout, or ignore it and bring their own geometry:
// the contract with the host is tiles-in-viewport-pixels, nothing more.
//
// Rectangles are plain maps #{ x, y, w, h } in unzoomed viewport pixels.

/// A rectangle in viewport pixels.
fn rect(x, y, w, h) {
    #{ x: x, y: y, w: w, h: h }
}

fn rect_contains(r, x, y) {
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

/// Shrink on every side, never past empty.
fn rect_inset(r, padding) {
    let w = r.w - padding * 2.0;
    let h = r.h - padding * 2.0;
    #{
        x: r.x + padding,
        y: r.y + padding,
        w: if w < 0.0 { 0.0 } else { w },
        h: if h < 0.0 { 0.0 } else { h },
    }
}

/// Worst (largest) aspect ratio in `areas[from..=to]` laid along a side.
/// Direct port of tobi's worst_ratio, index-based so no row copy is made:
/// the row-growth scan calls this once per candidate tile.
fn worst_ratio_range(areas, from, to, row_sum, side) {
    if row_sum <= 0.0 || side <= 0.0 {
        return 1.0e308;
    }
    let thickness = row_sum / side;
    let worst = 0.0;
    let i = from;
    while i <= to {
        let area = areas[i];
        if area > 0.0 && thickness > 0.0 {
            let other = area / thickness;
            let ratio = if thickness / other > other / thickness {
                thickness / other
            } else {
                other / thickness
            };
            if ratio > worst {
                worst = ratio;
            }
        }
        i += 1;
    }
    worst
}

/// Split an area into one rectangle per value, proportional to it.
/// Returned in the order of `values`, not in layout order.
/// Direct port of tobi's squarify (Bruls, Huizing and van Wijk).
///
/// CONTRACT: `values` must already be ranked largest-first, like tobi's
/// place_children feeds it. The host's tree data is ranked per metric; a skin
/// calling this with unsorted values gets a valid but ugly mosaic.
/// Slice-and-dice: 每层交替方向、按值比例顺序切分。极简皮肤的 tiling——
/// 与默认皮肤的 squarify 相比，窄长条暴露层级顺序。
/// Slice-and-dice: 每层交替方向、按值比例顺序切分。极简皮肤的 tiling——
/// 与默认皮肤的 squarify 相比，窄长条暴露层级顺序。
fn squarify(values, area) {
    let rects = [];
    if values.len() == 0 || area.w <= 0.0 || area.h <= 0.0 {
        return rects;
    }
    let total = 0;
    for v in values {
        total += v;
    }
    if total <= 0 {
        return rects;
    }
    let x = area.x;
    let y = area.y;
    let w = area.w;
    let h = area.h;
    let horizontal = w >= h;
    let i = 0;
    while i < values.len() {
        let share = values[i].to_float() / total.to_float();
        if horizontal {
            let seg = w * share;
            rects.push(rect(x, y, seg, h));
            x += seg;
            w -= seg;
        } else {
            let seg = h * share;
            rects.push(rect(x, y, w, seg));
            y += seg;
            h -= seg;
        }
        horizontal = !horizontal;
        i += 1;
    }
    rects
}





/// The deepest tile containing a point: children are emitted after their
/// parent and inset inside it, so the last match in reverse order wins.
/// `tiles` is an array of tile maps, each with a `rect` field.
fn hit(tiles, x, y) {
    let i = tiles.len() - 1;
    while i >= 0 {
        let t = tiles[i];
        if rect_contains(t.rect, x, y) {
            return t;
        }
        i -= 1;
    }
    ()
}

// ---------------------------------------------------------------------------
// Formatting — port of tobi/disktree size.rs. Binary units like `du -h`:
// one decimal below 10, none above. Integer arithmetic, so no float
// formatting artifacts.
// ---------------------------------------------------------------------------

fn scaled(bytes) {
    // Returns [value_in_tenths, unit_index]: e.g. 1536 -> [15, 1] (1.5 KiB).
    // Rounded division, so the 9.95-boundary behaves like the original
    // (9.95 and up prints as the next whole unit).
    // The unit steps at 1023.5 of a unit, not 1024 (tobi size.rs PR #70):
    // the unit is chosen against exact byte thresholds, the value rounded
    // to tenths once at the end — no intermediate rounding.
    let step_bytes = [
        1024,
        1048064,
        1073217536,
        1098974756864,
        1125350151028736,
        1152358554653425664,
    ];
    let unit_bytes = [
        1,
        1024,
        1048576,
        1073741824,
        1099511627776,
        1125899906842624,
    ];
    let unit = 0;
    while unit + 1 < 6 && bytes >= step_bytes[unit] {
        unit += 1;
    }
    let value = (bytes * 10 + unit_bytes[unit] / 2) / unit_bytes[unit];
    [value, unit]
}

fn size_units() {
    ["B", "KiB", "MiB", "GiB", "TiB", "PiB"]
}

/// `1.4 GiB`, `523 MiB`, `12 B`.
fn human_bytes(bytes) {
    let pair = scaled(bytes);
    let value = pair[0];
    let unit = pair[1];
    if unit == 0 {
        return `${value / 10} B`;
    }
    let whole = value / 10;
    let tenths = value % 10;
    if whole < 10 && tenths != 0 {
        return `${whole}.${tenths} ${size_units()[unit]}`;
    }
    if whole < 10 && tenths == 0 {
        return `${whole}.0 ${size_units()[unit]}`;
    }
    `${whole} ${size_units()[unit]}`
}

/// `1.5KiB`, `512KiB`: for tile labels, where columns are scarce.
fn human_bytes_short(bytes) {
    let pair = scaled(bytes);
    let value = pair[0];
    let unit = pair[1];
    if unit == 0 {
        return `${value / 10}B`;
    }
    let whole = value / 10;
    let tenths = value % 10;
    if whole < 10 && tenths != 0 {
        return `${whole}.${tenths}${size_units()[unit]}`;
    }
    if whole < 10 && tenths == 0 {
        return `${whole}.0${size_units()[unit]}`;
    }
    `${whole}${size_units()[unit]}`
}

/// `1.2k`, `3.4M`, `812`: file counts.
fn human_count(count) {
    if count < 10000 {
        return `${count}`;
    }
    let value = count.to_float();
    if value < 999_950.0 {
        return `${fmt_tenths(value / 1000.0)}k`;
    }
    if value < 999_950_000.0 {
        return `${fmt_tenths(value / 1_000_000.0)}M`;
    }
    `${fmt_tenths(value / 1000000000.0)}G`
}

fn fmt_tenths(value) {
    // One decimal, rounded, via integer math: 1234.56 -> "1.2".
    let tenths = (value * 10.0).round().to_int();
    let whole = tenths / 10;
    let frac = tenths % 10;
    if frac == 0 {
        return `${whole}.0`;
    }
    `${whole}.${frac}`
}

/// `~/`-abbreviate a path for display: home replaced, like tobi's display_path.
fn display_path(path, home) {
    if home != "" && (path == home || (path.len() > home.len() && path.starts_with(home + "/"))) {
        return "~" + path.sub_string(home.len());
    }
    path
}

// ---------------------------------------------------------------------------
// Layout — port of tobi/disktree treemap.rs place_children, adapted to the
// skin contract: directory contents arrive through the host's
// `disktree.tree.children` capability (ranked, capped, tail-merged), and
// tiles carry crumbs that address the scanned tree.
// ---------------------------------------------------------------------------

/// The layout knobs, mirroring tobi's LayoutOptions defaults.
fn layout_options(max_depth) {
    #{
        max_depth: max_depth,
        padding: 1.0,
        padding_outer: 3.0,
        min_tile: 5.0,
        max_children: 96,
        header: 20.0,
        header_inner: 15.0,
    }
}

/// The band a directory keeps for its name, or `()` when the tile is too
/// small to leave its children a usable area below it. The top level gets
/// the full header; deeper directories a slimmer band.
fn header_band(r, options, depth) {
    let height = if depth == 0 { options.header } else { options.header_inner };
    let body = r.h - height;
    if r.w < 44.0 || body < options.min_tile * 3.0 {
        return ();
    }
    rect(r.x, r.y, r.w, height)
}

/// Entry value by the active metric: bytes or file count.
fn entry_value_metric(entry, metric) {
    if metric == "files" {
        entry.files
    } else {
        entry.bytes
    }
}

/// Lay out everything beneath the directory at `crumbs` inside `area`.
///
/// Emits tiles depth-first, parent before children — the order that lets
/// the view paint in sequence and hit-test in reverse. Recursion replaces
/// this whole function when a skin swaps the tiling algorithm; the tile
/// map shape is the only contract:
///   #{ crumbs, name, kind, others, count, bytes, files, category, reclaim,
///      read_error, rect, header, depth, index }
/// The engine caps rhai arrays at 10,000 entries; a deep layout on a large
/// tree can exceed that. `budget` is the remaining tile allowance for this
/// subtree: when it runs out the deepest levels stop subdividing — their
/// tiles stay whole, area still accounted.
fn place_dir(ctx, crumbs, area, metric, options, depth, needle, budget) {
    let out = [];
    if area.w <= 0.0 || area.h <= 0.0 || budget <= 0 {
        return out;
    }
    let request = #{
        crumbs: crumbs,
        metric: metric,
        max: options.max_children,
    };
    if needle != "" {
        request.needle = needle;
    }
    let reply = ctx.call_capability("disktree.tree", "children", request);
    let entries = reply.entries;
    if entries.len() == 0 {
        return out;
    }

    // Values for squarify: entries are already ranked by the capability,
    // then the tail merges into one Others value so its area is still
    // accounted for.
    let values = [];
    for entry in entries {
        values.push(entry_value_metric(entry, metric));
    }
    if reply.tail_count > 0 && reply.tail_value > 0 {
        values.push(reply.tail_value);
    }
    let rects = squarify(values, area);

    let padding = if depth == 0 { options.padding_outer } else { options.padding };
    let slot = 0;
    while slot < rects.len() {
        let raw = rects[slot];
        let r = rect_inset(raw, padding);
        if r.w >= options.min_tile && r.h >= options.min_tile {
            if slot < entries.len() {
                let entry = entries[slot];
                let child_crumbs = crumbs + [entry.index];
                let subdividable = entry.kind == "dir" && depth + 1 < options.max_depth;
                // A directory that is about to be subdivided claims a header
                // band for its own name. With no room for one it stays
                // whole: a name over its children is worse than one level
                // less of detail.
                let header = ();
                if subdividable {
                    header = header_band(r, options, depth);
                }
                out.push(#{
                    crumbs: child_crumbs,
                    kind: entry.kind,
                    others: 0,
                    entry: entry,
                    rect: r,
                    header: header,
                    depth: depth,
                });
                if header != () && budget - out.len() > 40 {
                    let body = rect(r.x, header.y + header.h, r.w, r.y + r.h - header.y - header.h);
                    out = out + place_dir(
                        ctx,
                        child_crumbs,
                        body,
                        metric,
                        options,
                        depth + 1,
                        needle,
                        budget - out.len()
                    );
                }
            } else {
                // The tail of a long child list, merged so its area is
                // accounted for instead of silently dropped.
                out.push(#{
                    crumbs: crumbs,
                    kind: "others",
                    others: reply.tail_count,
                    entry: (),
                    rect: r,
                    header: (),
                    depth: depth,
                });
            }
        }
        slot += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// Palette — port of tobi/disktree palette.rs (dark theme). Each category is
// a hue + chroma; fills desaturate with depth, then blend toward the
// surface. Skins replace this section to re-colour the mosaic.
// ---------------------------------------------------------------------------

fn category_base(category) {
    // #{ h: hue 0..1, c: chroma }
    if category == "code" { return #{ h: 0.605, c: 1.0 }; }
    if category == "agent" { return #{ h: 0.065, c: 1.0 }; }
    if category == "toolchain" { return #{ h: 0.415, c: 1.0 }; }
    if category == "synced" { return #{ h: 0.535, c: 1.0 }; }
    if category == "git" { return #{ h: 0.955, c: 1.0 }; }
    if category == "media" { return #{ h: 0.745, c: 1.0 }; }
    if category == "cache" { return #{ h: 0.125, c: 0.95 }; }
    if category == "documents" { return #{ h: 0.6, c: 0.18 }; }
    #{ h: 0.0, c: 0.08 }
}

// Dark-theme surface tones, matching ui/theme.rhai's tokens. A function,
// not a module constant: rhai functions do not see module-level consts.
fn theme_rgb() {
    #{
        surface: [0x1a, 0x1b, 0x26],
        bright: [0xc0, 0xca, 0xf5],
        foreground: [0xc0, 0xca, 0xf5],
    }
}

fn hue_channel(t, p, q) {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
    if t < 0.5 { return q; }
    if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    p
}

/// HSL to RGB, all channels 0..1. Port of the usual hsl_to_rgb.
fn hsl_to_rgb(h, s, l) {
    if s == 0.0 {
        return [l, l, l];
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    [
        hue_channel(h + 1.0 / 3.0, p, q),
        hue_channel(h, p, q),
        hue_channel(h - 1.0 / 3.0, p, q),
    ]
}

fn clamp01(v) {
    if v < 0.0 { return 0.0; }
    if v > 1.0 { return 1.0; }
    v
}

fn mix(a, b, t) {
    // Linear blend of two [r, g, b] triples (0..1).
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// A tile fill for a category at a nesting depth, dark theme:
/// s = 0.26*chroma, l = 0.215 + 0.028*depth (depth capped at 4), blended
/// 12% toward the surface. Returns 0xRRGGBBAA for rgba().
/// One palette per theme mode. Dark follows tobi's hsl ramp; light inverts
/// it for Flexoki paper: pale tints that darken slightly with depth.
fn category_fill_mode(category, depth, light, dim) {
    // 极简皮肤：颜色不分类，只按深度退灰——信息形态由位置与标签承担。
    // dim: 过滤时未命中的瓦片几乎完全退回底色。
    let d = if depth > 4 { 4 } else { depth };
    let surface = ();
    let lightness = ();
    if light {
        surface = [1.0, 0.988, 0.941];
        lightness = 0.93 - 0.028 * d;
    } else {
        surface = [theme_rgb().surface[0] / 255.0, theme_rgb().surface[1] / 255.0, theme_rgb().surface[2] / 255.0];
        lightness = 0.30 - 0.035 * d;
    }
    let rgb = hsl_to_rgb(0.62, 0.03, lightness);
    let blend = 0.15;
    if dim {
        blend = 0.84;
    }
    rgba_bytes(mix(rgb, surface, blend), 1.0)
}

/// The stronger accent for a category's strip and legend chips.
fn category_accent(category) {
    let base = category_base(category);
    let rgb = hsl_to_rgb(base.h, 0.42 * base.c, 0.52);
    rgba_bytes(rgb, 1.0)
}

/// The five age buckets' fills, from freshest (theme accent) to oldest
/// (grey). Used in Age metric mode.
fn age_fill_mode(age_days, light, dim) {
    // 极简皮肤：年龄只调明度，不引色相。
    let rgb = ();
    if light {
        if age_days <= 7 { rgb = hsl_to_rgb(0.62, 0.02, 0.80); }
        else if age_days <= 30 { rgb = hsl_to_rgb(0.62, 0.02, 0.86); }
        else { rgb = hsl_to_rgb(0.62, 0.02, 0.92); }
    } else {
        if age_days <= 7 { rgb = hsl_to_rgb(0.62, 0.02, 0.34); }
        else if age_days <= 30 { rgb = hsl_to_rgb(0.62, 0.02, 0.27); }
        else { rgb = hsl_to_rgb(0.62, 0.02, 0.21); }
    }
    let surface = ();
    if light {
        surface = [1.0, 0.988, 0.941];
    } else {
        let tone = theme_rgb().surface;
        surface = [tone[0] / 255.0, tone[1] / 255.0, tone[2] / 255.0];
    }
    if dim {
        return rgba_bytes(mix(rgb, surface, 0.84), 1.0);
    }
    rgba_bytes(rgb, 1.0)
}

/// Age of a node in whole days from its mtime.
fn age_days(modified, now) {
    if modified <= 0 {
        return 99999;
    }
    let seconds = now - modified;
    if seconds < 0 {
        return 0;
    }
    seconds / 86400
}

fn rgba_bytes(rgb, alpha) {
    // [r, g, b] in 0..1 plus alpha 0..1 to 0xRRGGBBAA.
    let r = (clamp01(rgb[0]) * 255.0).round().to_int();
    let g = (clamp01(rgb[1]) * 255.0).round().to_int();
    let b = (clamp01(rgb[2]) * 255.0).round().to_int();
    let a = (clamp01(alpha) * 255.0).round().to_int();
    r * 0x1000000 + g * 0x10000 + b * 0x100 + a
}

/// Hatch color for reclaimable tiles.
fn hatch_color() {
    rgba_bytes([theme_rgb().bright[0] / 255.0, theme_rgb().bright[1] / 255.0, theme_rgb().bright[2] / 255.0], 0.16)
}

// ---------------------------------------------------------------------------
// View — port of tobi/disktree state.rs `View`. All viewport geometry stays
// in the base space of an unzoomed layout, so pan and zoom never require a
// re-layout: tiles are laid out once at viewport size and projected while
// painting. A view is #{ scale, ox, oy }.
// ---------------------------------------------------------------------------

fn view_identity() {
    #{ scale: 1.0, ox: 0.0, oy: 0.0 }
}

/// Base-space rect to screen space.
fn view_project(view, r) {
    rect(
        (r.x - view.ox) * view.scale,
        (r.y - view.oy) * view.scale,
        r.w * view.scale,
        r.h * view.scale
    )
}

/// Screen point to base-space point.
fn view_unproject(view, x, y) {
    [x / view.scale + view.ox, y / view.scale + view.oy]
}

/// Zoom by `factor` toward the screen point, no further than `ceiling`. The
/// base point under the anchor stays put, which makes a wheel zoom feel
/// pointed at the tile rather than at the window.
fn view_zoomed_at(view, x, y, factor, ceiling) {
    let scale = view.scale * factor;
    if scale < 1.0 {
        scale = 1.0;
    }
    if scale > ceiling {
        scale = ceiling;
    }
    let base = view_unproject(view, x, y);
    #{
        scale: scale,
        ox: base[0] - x / scale,
        oy: base[1] - y / scale,
    }
}

/// The scale at which `r` exactly fits the viewport.
fn view_fit_scale(r, w, h) {
    if r.w <= 0.0 || r.h <= 0.0 {
        return 1.0;
    }
    let by_width = w / r.w;
    let by_height = h / r.h;
    if by_width < by_height { by_width } else { by_height }
}

/// Keep the viewport inside the layout: no empty margins, ever.
fn view_clamped(view, w, h) {
    let max_x = w - w / view.scale;
    let max_y = h - h / view.scale;
    let ox = view.ox;
    if ox < 0.0 { ox = 0.0; }
    if ox > max_x { ox = max_x; }
    let oy = view.oy;
    if oy < 0.0 { oy = 0.0; }
    if oy > max_y { oy = max_y; }
    #{ scale: view.scale, ox: ox, oy: oy }
}

// ---------------------------------------------------------------------------
// Navigation — ports of tobi's Direction scoring, move_selection, and the
// zoom enter/ascend targets. All work on tile maps with `crumbs` and `rect`
// in base space, plus the active view for projection.
// ---------------------------------------------------------------------------

/// Distance from `from` to `r` along the axis, or `()` when `r` is not that
/// way. `direction` is "left" | "right" | "up" | "down".
fn nav_gap(direction, from, r) {
    let epsilon = 0.5;
    if direction == "right" {
        let gap = r.x - (from.x + from.w);
        if gap < -epsilon { return (); }
        if gap < 0.0 { return 0.0; }
        return gap;
    }
    if direction == "left" {
        let gap = from.x - (r.x + r.w);
        if gap < -epsilon { return (); }
        if gap < 0.0 { return 0.0; }
        return gap;
    }
    if direction == "down" {
        let gap = r.y - (from.y + from.h);
        if gap < -epsilon { return (); }
        if gap < 0.0 { return 0.0; }
        return gap;
    }
    let gap = from.y - (r.y + r.h);
    if gap < -epsilon { return (); }
    if gap < 0.0 { return 0.0; }
    gap
}

/// How far off the direction's axis `r` sits, so the nearest tile in the
/// direction wins rather than any tile in that half-plane.
fn nav_offset(direction, from, r) {
    if direction == "left" || direction == "right" {
        let overlap_end = from.y + from.h;
        let edge = r.y + r.h;
        if edge < overlap_end { overlap_end = edge; }
        let start = from.y;
        if r.y > start { start = r.y; }
        let overlap = overlap_end - start;
        if overlap < 0.0 { overlap = 0.0; }
        let lesser = from.h;
        if r.h < lesser { lesser = r.h; }
        let slack = lesser - overlap;
        if slack < 0.0 { return 0.0; }
        return slack;
    }
    let overlap_end = from.x + from.w;
    let edge = r.x + r.w;
    if edge < overlap_end { overlap_end = edge; }
    let start = from.x;
    if r.x > start { start = r.x; }
    let overlap = overlap_end - start;
    if overlap < 0.0 { overlap = 0.0; }
    let lesser = from.w;
    if r.w < lesser { lesser = r.w; }
    let slack = lesser - overlap;
    if slack < 0.0 { return 0.0; }
    slack
}

/// The best same-depth neighbour of `current` towards `direction`, over
/// drawn tiles projected through `view`. `()` when nothing lies that way.
fn nav_neighbour(tiles, view, current, direction) {
    let from = ();
    let i = 0;
    while i < tiles.len() {
        let t = tiles[i];
        if t.kind != "others" && crumb_eq(t.crumbs, current) {
            from = view_project(view, t.rect);
        }
        i += 1;
    }
    if from == () {
        return ();
    }
    let depth = current.len();
    let best_score = 0.0;
    let best = ();
    let j = 0;
    while j < tiles.len() {
        let t = tiles[j];
        if t.kind == "others" || t.crumbs.len() != depth || crumb_eq(t.crumbs, current) {
            j += 1;
            continue;
        }
        let r = view_project(view, t.rect);
        let gap = nav_gap(direction, from, r);
        if gap == () {
            j += 1;
            continue;
        }
        let score = nav_offset(direction, from, r) * 2.5 + gap;
        if best == () || score < best_score {
            best = t.crumbs;
            best_score = score;
        }
        j += 1;
    }
    best
}

/// The parent of a crumb path (built by loop: computed slice bounds are a
/// rhai trap, see docs/UPSTREAM_GAPS.md G3).
fn nav_parent(crumbs) {
    if crumbs.len() == 0 {
        return ();
    }
    let out = [];
    let i = 0;
    while i < crumbs.len() - 1 {
        out.push(crumbs[i]);
        i += 1;
    }
    out
}

/// Whether two crumb paths are equal (rhai `==` on arrays of ints would
/// also work; a named helper keeps the intent readable in skins).
fn crumb_eq(a, b) {
    if a.len() != b.len() {
        return false;
    }
    let i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Whether `path` begins with `prefix`.
fn crumb_starts_with(path, prefix) {
    if path.len() < prefix.len() {
        return false;
    }
    let i = 0;
    while i < prefix.len() {
        if path[i] != prefix[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Where a drawn directory's contents sit: its tile below the name band.
/// This, not the whole tile, is the region zooming into it magnifies.
fn tile_body(tile) {
    if tile.header != () {
        return rect(
            tile.rect.x,
            tile.header.y + tile.header.h,
            tile.rect.w,
            tile.rect.y + tile.rect.h - tile.header.y - tile.header.h
        );
    }
    tile.rect
}

/// The deepest drawn directory under a screen point that has contents to
/// show: what zooming at that point is zooming into. `root_len` is the
/// current root's crumb length; ancestors of it are not on screen. The
/// hovered tile may be a file or the Others tail — their crumb prefixes
/// still name directories worth zooming into.
fn zoom_target(tiles, view, root_len, x, y) {
    let point = view_unproject(view, x, y);
    let hovered = hit(tiles, point[0], point[1]);
    if hovered == () {
        return ();
    }
    let i = hovered.crumbs.len();
    while i > root_len {
        let j = 0;
        while j < tiles.len() {
            let t = tiles[j];
            if t.kind != "others" && t.crumbs.len() == i && crumb_starts_with(hovered.crumbs, t.crumbs)
                && t.entry != () && t.entry.files + t.entry.dirs > 0
            {
                return t;
            }
            j += 1;
        }
        i -= 1;
    }
    ()
}
