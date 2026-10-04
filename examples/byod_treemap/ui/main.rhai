// A disk usage treemap written without the design language: no token base, no
// official components, raw boxes and the app's own theme vocabulary. It shows
// that the runtime imposes no design; the layout below is a squarified treemap.

fn data() {
    [
        #{ name: "src", size: 4200, kind: "code" },
        #{ name: "assets", size: 3100, kind: "media" },
        #{ name: "target", size: 2600, kind: "other" },
        #{ name: "fixtures", size: 1400, kind: "data" },
        #{ name: "docs", size: 900, kind: "code" },
        #{ name: "vendor", size: 700, kind: "other" },
        #{ name: "scripts", size: 380, kind: "code" },
        #{ name: "logs", size: 260, kind: "data" },
        #{ name: "tmp", size: 120, kind: "other" },
    ]
}

fn state_schema() {
    #{ fields: #{ selected: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } } } }
}

fn select(ctx, index) { ctx.set_state("selected", index); }

fn worst_ratio(row, side, scale) {
    let total = 0.0;
    let largest = 0.0;
    let smallest = 1.0e18;
    for item in row {
        let area = item.size * scale;
        total += area;
        if area > largest { largest = area; }
        if area < smallest { smallest = area; }
    }
    let side2 = side * side;
    let total2 = total * total;
    let a = side2 * largest / total2;
    let b = total2 / (side2 * smallest);
    if a > b { a } else { b }
}

// Lay one row along the shorter side of the free rectangle. Rhai passes arrays
// by value, so the placed cells and the remaining rectangle are returned.
fn place_row(row, rect, scale) {
    let total = 0.0;
    for item in row { total += item.size * scale; }
    let horizontal = rect.w >= rect.h;
    let thickness = if horizontal { total / rect.h } else { total / rect.w };
    let offset = 0.0;
    let cells = [];
    for item in row {
        let length = item.size * scale / thickness;
        cells.push(if horizontal {
            #{ item: item, x: rect.x, y: rect.y + offset, w: thickness, h: length }
        } else {
            #{ item: item, x: rect.x + offset, y: rect.y, w: length, h: thickness }
        });
        offset += length;
    }
    let rest = if horizontal { #{ x: rect.x + thickness, y: rect.y, w: rect.w - thickness, h: rect.h } }
        else { #{ x: rect.x, y: rect.y + thickness, w: rect.w, h: rect.h - thickness } };
    #{ cells: cells, rest: rest }
}

fn squarify(items, width, height) {
    let total = 0.0;
    for item in items { total += item.size; }
    let scale = width * height / total;
    let rect = #{ x: 0.0, y: 0.0, w: width, h: height };
    let out = [];
    let row = [];
    for item in items {
        let side = if rect.w < rect.h { rect.w } else { rect.h };
        let candidate = row + [item];
        if row.len == 0 || worst_ratio(candidate, side, scale) <= worst_ratio(row, side, scale) {
            row = candidate;
        } else {
            let placed = place_row(row, rect, scale);
            out += placed.cells;
            rect = placed.rest;
            row = [item];
        }
    }
    if row.len > 0 { out += place_row(row, rect, scale).cells; }
    out
}

fn tile(ctx, index, cell, selected) {
    let label = column([
        text(cell.item.name).with_style(style().text_color(theme_color("ink")).font_weight(600)),
        text(`${cell.item.size} MB`).with_style(style().text_color(theme_color("faint"))),
    ]).with_style(style().padding(theme_length("treemap.pad")));
    row([label])
        .with_key(cell.item.name)
        .with_style(style().absolute()
            .left(px(cell.x)).top(px(cell.y)).width(px(cell.w)).height(px(cell.h))
            .border(theme_length("treemap.gutter"))
            .border_color(if selected { theme_color("pick") } else { theme_color("ground") })
            .background(theme_color(`tile.${cell.item.kind}`))
            .clip())
        .accessibility_role("button")
        .accessibility_label(`${cell.item.name}, ${cell.item.size} MB`)
        .on_click_value(Fn("select"), index)
}

fn view(ctx) {
    let items = data();
    let cells = squarify(items, 760.0, 440.0);
    let selected = ctx.get_state("selected");
    let tiles = [];
    for index in 0..cells.len { tiles.push(tile(ctx, index, cells[index], index == selected)); }
    let current = cells[selected].item;
    let count = cells.len;
    let bar = row([
        text("disk usage").with_style(style().text_color(theme_color("faint"))),
        text(`${current.name} · ${current.size} MB`).with_style(style().text_color(theme_color("ink"))),
    ]).with_style(style().height(theme_length("treemap.bar")).items_center().justify_between()
        .padding_x(theme_length("treemap.pad")).border_bottom(px(1)).border_color(theme_color("rule")));
    column([
        bar,
        box(tiles).with_style(style().relative().width(px(760)).height(px(440))),
    ]).with_style(style().width(relative(1.0)).height(relative(1.0)).gap(theme_length("treemap.pad"))
        .padding(theme_length("treemap.pad")).background(theme_color("ground")))
        .tab_stop(true)
        .accessibility_role("group").accessibility_label("Treemap")
        .on_key_value("right", Fn("select"), (selected + 1) % count)
        .on_key_value("left", Fn("select"), (selected + count - 1) % count)
}
