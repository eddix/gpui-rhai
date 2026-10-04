// Gallery spec pages: direct manipulation. Every interaction has a keyboard path.

import "gallery/kit" as kit;
import "components/draggable" as draggable;
import "components/drag_source" as drag_source;
import "components/drop_zone" as drop_zone;
import "components/sortable" as sortable;
import "components/pan_zoom" as pan_zoom;
import "components/rotatable" as rotatable;
import "components/selection_area" as selection_area;
import "components/card" as card;

fn pages() {
    [
        #{ id: "draggable", title: "Draggable", group: "Interaction" },
        #{ id: "drag_source", title: "DragSource", group: "Interaction" },
        #{ id: "drop_zone", title: "DropZone", group: "Interaction" },
        #{ id: "sortable", title: "Sortable", group: "Interaction" },
        #{ id: "pan_zoom", title: "PanZoom", group: "Interaction" },
        #{ id: "rotatable", title: "Rotatable", group: "Interaction" },
        #{ id: "selection_area", title: "SelectionArea", group: "Interaction" },
    ]
}

fn view(ctx, h, v, id) {
    if id == "draggable" { draggable_page(h, v) }
    else if id == "drag_source" { drag_page(h, v, "drag_source") }
    else if id == "drop_zone" { drag_page(h, v, "drop_zone") }
    else if id == "sortable" { sortable_page(h, v) }
    else if id == "pan_zoom" { pan_zoom_page(h, v) }
    else if id == "rotatable" { rotatable_page(h, v) }
    else if id == "selection_area" { selection_page(h, v) }
    else { () }
}

fn stage(content) {
    box([content]).with_style(style().relative().width(px(480)).height(px(240))
        .background(theme_color("surface_raised")))
}

fn draggable_page(h, v) {
    let handle = row([text("Drag handle")]).with_style(style().min_height(theme_length("metrics.row"))
        .padding_x(theme_length("metrics.inset")).items_center().background(theme_color("surface_hover")));
    kit::page(h, v, "draggable", "Draggable", "A controlled position: dragging previews natively and commits once.", [
        kit::sec("Card", stage(draggable::Draggable(#{ key: "draggable-card", label: "Move card",
            position: kit::val(v, "draggable.position", #{ x: 24.0, y: 24.0 }), handle: handle,
            content: card::Card(#{ content: text("Arrows move it too.") }).with_style(style().width(px(220))),
            contain: true, snap_x: 8.0, snap_y: 8.0, on_move: kit::store(h, "draggable.position") }))),
    ])
}

fn lane(h, v, id, label_text) {
    let owner = kit::val(v, "drag.owner", "backlog");
    let children = [text(label_text.to_upper()).with_style(style().typography("label").text_color(theme_color("text_muted")))];
    if owner == id {
        children.push(drag_source::DragSource(#{ key: "drag-service", label: "Move service alpha", source_id: "service-alpha",
            payload_type: "service", payload: #{ id: "service-alpha" }, operation: "move",
            keyboard_target: if id == "backlog" { "deployed" } else { "backlog" },
            content: card::Card(#{ content: text("service-alpha") }) }));
    }
    drop_zone::DropZone(#{ key: `drop-${id}`, label: `${label_text} lane`, target_id: id, payload_types: ["service"],
        operations: ["move"], content: column(children).with_style(style().flex_col().gap(theme_length("space.related"))
            .padding(theme_length("metrics.inset"))), on_drop: h.drop.curry("drag.owner") })
        .with_style(style().width(px(240)).height(px(180)).background(theme_color("surface_raised")))
}

fn drag_page(h, v, id) {
    let title = if id == "drag_source" { "DragSource" } else { "DropZone" };
    kit::page(h, v, id, title, "Typed drag and drop; Enter on a focused source moves it to its keyboard target.", [
        kit::sec("Lanes", kit::line([lane(h, v, "backlog", "Backlog"), lane(h, v, "deployed", "Deployed")])),
    ])
}

fn sortable_page(h, v) {
    let order = kit::val(v, "sortable.order", ["build", "test", "ship", "observe"]);
    let items = [];
    for key in order {
        items.push(#{ key: key, label: key, content: row([text(key)]).with_style(style()
            .padding_x(theme_length("metrics.inset"))) });
    }
    kit::page(h, v, "sortable", "Sortable", "Keyed ordering; the handle is a tab stop and arrows move the item.", [
        kit::sec("Pipeline", sortable::Sortable(#{ key: "sortable-pipeline", label: "Pipeline", items: items,
            direction: "vertical", on_reorder: h.reorder.curry("sortable.order") }).with_style(style().width(px(320)))),
    ])
}

fn grid_scene() {
    let commands = [];
    for index in 0..9 {
        let offset = index * 40.0;
        commands.push(canvas_line(`v-${index}`, offset, 0.0, offset, 320.0, 1.0, theme_color("border")));
        commands.push(canvas_line(`h-${index}`, 0.0, offset, 320.0, offset, 1.0, theme_color("border")));
    }
    commands.push(canvas_circle("origin", 160.0, 120.0, 14.0, theme_color("accent")));
    canvas(canvas_scene(commands)).with_key("pan-canvas").with_style(style().width(relative(1.0)).height(relative(1.0)))
}

fn pan_zoom_page(h, v) {
    kit::page(h, v, "pan_zoom", "PanZoom", "Drag to pan, modifier and wheel to zoom; arrows and plus/minus from the keyboard.", [
        kit::sec("Canvas", pan_zoom::PanZoom(#{ key: "pan-zoom", label: "Pan and zoom canvas",
            transform: kit::val(v, "pan.transform", #{ x: 0.0, y: 0.0, scale: 1.0 }), min_scale: 0.5, max_scale: 3.0,
            content: grid_scene(), on_transform_change: kit::store(h, "pan.transform") })
            .with_style(style().width(px(480)).height(px(240)))),
    ])
}

fn rotatable_page(h, v) {
    let scene = canvas(canvas_scene([
        canvas_rect("arm", 100.0, 86.0, 82.0, 28.0, theme_color("accent")),
        canvas_circle("pivot", 100.0, 100.0, 7.0, theme_color("text_primary")),
    ])).with_key("rotate-canvas");
    kit::page(h, v, "rotatable", "Rotatable", "Rotation around an explicit pivot, snapped in 15° steps.", [
        kit::sec("Arm", rotatable::Rotatable(#{ key: "rotate-arm", label: "Rotate arm", angle: kit::val(v, "rotate.angle", 0.0),
            pivot: #{ x: 100.0, y: 100.0 }, snap: 15.0, content: scene,
            handle: text("↻").with_style(style().items_center().justify_center()),
            on_rotate: kit::store(h, "rotate.angle") }).with_style(style().width(px(300)).height(px(220)))),
    ])
}

fn selection_page(h, v) {
    let state = kit::val(v, "selection.state", #{ selected_keys: [], active_key: (), anchor_key: () });
    let targets = [
        #{ key: "alpha", x: 30.0, y: 30.0, width: 70.0, height: 50.0 },
        #{ key: "bravo", x: 130.0, y: 45.0, width: 74.0, height: 54.0 },
        #{ key: "charlie", x: 68.0, y: 130.0, width: 82.0, height: 58.0 },
    ];
    let commands = [];
    for target in targets {
        commands.push(canvas_rect(target.key, target.x, target.y, target.width, target.height,
            if target.key in state.selected_keys { theme_color("accent") } else { theme_color("surface_hover") }));
    }
    kit::page(h, v, "selection_area", "SelectionArea", "Click, range and marquee selection with one keyboard state.", [
        kit::sec("Nodes", selection_area::SelectionArea(#{ key: "selection-nodes", label: "Selectable nodes", targets: targets,
            selected_keys: state.selected_keys, active_key: state.active_key, anchor_key: state.anchor_key,
            content: canvas(canvas_scene(commands)).with_key("selection-canvas"),
            on_selection_change: kit::store(h, "selection.state") }).with_style(style().width(px(320)).height(px(220)))),
    ])
}
