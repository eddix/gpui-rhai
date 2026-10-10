// Gallery spec pages: the marker system (Button, IconButton, Toggle, ToggleGroup,
// ButtonGroup, Badge, Tag, Kbd).

import "gallery/kit" as kit;
import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/toggle" as toggle;
import "components/toggle_group" as toggle_group;
import "components/button_group" as button_group;
import "components/badge" as badge;
import "components/tag" as tag;
import "components/kbd" as kbd;

fn pages() {
    [
        #{ id: "button", title: "Button", group: "Markers" },
        #{ id: "icon_button", title: "IconButton", group: "Markers" },
        #{ id: "toggle", title: "Toggle", group: "Markers" },
        #{ id: "toggle_group", title: "ToggleGroup", group: "Markers" },
        #{ id: "button_group", title: "ButtonGroup", group: "Markers" },
        #{ id: "badge", title: "Badge", group: "Markers" },
        #{ id: "tag", title: "Tag", group: "Markers" },
        #{ id: "kbd", title: "Kbd", group: "Markers" },
    ]
}

fn view(ctx, h, v, id) {
    if id == "button" { button_page(h, v) }
    else if id == "icon_button" { icon_button_page(h, v) }
    else if id == "toggle" { toggle_page(h, v) }
    else if id == "toggle_group" { toggle_group_page(h, v) }
    else if id == "button_group" { button_group_page(h, v) }
    else if id == "badge" { badge_page(h, v) }
    else if id == "tag" { tag_page(h, v) }
    else if id == "kbd" { kbd_page(h, v) }
    else { () }
}

fn btn(h, key, label, variant) {
    button::Button(#{ key: key, text: label, variant: variant, on_click: h.noop })
}

fn button_sizes(h, suffix) {
    kit::sizes([
        button::Button(#{ key: `btn-xs-${suffix}`, text: "Extra small", size: "xs", on_click: h.noop }),
        button::Button(#{ key: `btn-sm-${suffix}`, text: "Small", size: "sm", on_click: h.noop }),
        button::Button(#{ key: `btn-md-${suffix}`, text: "Medium", on_click: h.noop }),
        button::Button(#{ key: `btn-lg-${suffix}`, text: "Large", size: "lg", on_click: h.noop }),
    ])
}

fn button_page(h, v) {
    let variants = kit::variants([
        btn(h, "btn-primary", "Deploy", "primary"), btn(h, "btn-secondary", "Export", "secondary"),
        btn(h, "btn-outline", "Duplicate", "outline"), btn(h, "btn-ghost", "Cancel", "ghost"),
    ]);
    let verdicts = kit::variants([
        btn(h, "btn-success", "Approve", "success"), btn(h, "btn-warning", "Force sync", "warning"),
        btn(h, "btn-danger", "Delete", "danger"),
    ]);
    let states = kit::line([
        button::Button(#{ key: "btn-disabled", text: "Disabled", disabled: true }),
        button::Button(#{ key: "btn-loading", text: "Save", loading: true, loading_text: "Saving…" }),
        button::Button(#{ key: "btn-shortcut", text: "Search", shortcut: "⌘K", on_click: h.noop }),
        button::Button(#{ key: "btn-cjk", text: "部署到生产", on_click: h.noop }),
    ]);
    kit::page(h, v, "button", "Button", "A desktop action: a block with a centered label. One solid button per group.", [
        kit::sec("Variants", kit::pile([variants, kit::note("Verdict buttons are the group's solid action."), verdicts])
            .audit_allow(["multiple-solid-actions"])),
        kit::sec("Sizes", button_sizes(h, "sizes")),
        kit::sec("Density", kit::densities(button_sizes(h, "c"), button_sizes(h, "k"))),
        kit::sec("States", states),
    ])
}

fn icon(name) { image_source(asset(`app/icons/${name}`)) }

fn icon_buttons(h, suffix) {
    kit::variants([
        icon_button::IconButton(#{ key: `ib-search-${suffix}`, icon: icon("search"), label: "Search", on_click: h.noop }),
        icon_button::IconButton(#{ key: `ib-plus-${suffix}`, icon: icon("plus"), label: "Add", variant: "secondary", on_click: h.noop }),
        icon_button::IconButton(#{ key: `ib-check-${suffix}`, icon: icon("check"), label: "Done", variant: "primary", on_click: h.noop }),
        icon_button::IconButton(#{ key: `ib-info-${suffix}`, icon: icon("info"), label: "Details", selected: true, on_click: h.noop }),
        icon_button::IconButton(#{ key: `ib-close-${suffix}`, icon: icon("close"), label: "Close", variant: "outline", on_click: h.noop }),
    ])
}

fn icon_button_page(h, v) {
    kit::page(h, v, "icon_button", "IconButton", "A square action target with one icon and an accessible label.", [
        kit::sec("Variants", icon_buttons(h, "variants")),
        kit::sec("Density", kit::densities(icon_buttons(h, "c"), icon_buttons(h, "k"))),
        kit::sec("States", kit::line([
            icon_button::IconButton(#{ key: "ib-disabled", icon: icon("close"), label: "Close", disabled: true }),
            icon_button::IconButton(#{ key: "ib-loading", icon: icon("search"), label: "Searching", loading: true }),
        ])),
    ])
}

fn toggle_page(h, v) {
    let bold = kit::val(v, "toggle.bold", true);
    let italic = kit::val(v, "toggle.italic", false);
    kit::page(h, v, "toggle", "Toggle", "Pressed tool state. Pressing fills the block; the label never changes weight.", [
        kit::sec("States", kit::line([
            toggle::Toggle(#{ key: "tg-bold", text: "Bold", pressed: bold, on_pressed_change: kit::store(h, "toggle.bold") }),
            toggle::Toggle(#{ key: "tg-italic", text: "Italic", pressed: italic, on_pressed_change: kit::store(h, "toggle.italic") }),
            toggle::Toggle(#{ key: "tg-off", text: "Disabled", pressed: false, disabled: true }),
        ])),
    ])
}

fn view_items() {
    [#{ value: "grid", label: "Grid" }, #{ value: "list", label: "List" }, #{ value: "board", label: "Board" },
     #{ value: "calendar", label: "日历", disabled: true }]
}

fn toggle_group_page(h, v) {
    let single = kit::val(v, "tgroup.view", ["list"]);
    let multi = kit::val(v, "tgroup.format", ["b"]);
    let format_items = [#{ value: "b", label: "Bold" }, #{ value: "i", label: "Italic" }, #{ value: "u", label: "Underline" }];
    kit::page(h, v, "toggle_group", "ToggleGroup", "Segments joined in one frame. Arrows move between segments; Space presses.", [
        kit::sec("Single", kit::line([toggle_group::ToggleGroup(#{ key: "tgroup-view", label: "View", values: single,
            items: view_items(), on_change: kit::store(h, "tgroup.view") })])),
        kit::sec("Multiple", kit::line([toggle_group::ToggleGroup(#{ key: "tgroup-format", label: "Format", mode: "multiple",
            values: multi, items: format_items, on_change: kit::store(h, "tgroup.format") })])),
        kit::sec("Density", kit::densities(
            toggle_group::ToggleGroup(#{ key: "tgroup-c", label: "View", values: single, items: view_items(),
                on_change: kit::store(h, "tgroup.view") }),
            toggle_group::ToggleGroup(#{ key: "tgroup-k", label: "View", values: single, items: view_items(),
                on_change: kit::store(h, "tgroup.view") }))),
    ])
}

fn button_group_page(h, v) {
    kit::page(h, v, "button_group", "ButtonGroup", "Related actions joined with a 2px seam; each keeps its own focus frame.", [
        kit::sec("Variants", kit::line([
            button_group::ButtonGroup(#{ label: "Pager", buttons: [btn(h, "bg-prev", "Previous", "secondary"),
                btn(h, "bg-next", "Next", "secondary")] }),
            button_group::ButtonGroup(#{ label: "Range", buttons: [btn(h, "bg-day", "Day", "outline"),
                btn(h, "bg-week", "Week", "outline"), btn(h, "bg-month", "Month", "outline")] }),
        ])),
    ])
}

fn badge_page(h, v) {
    kit::page(h, v, "badge", "Badge", "A read-only status: a square lamp and a label. Strong emphasis only when action is needed.", [
        kit::sec("Subtle", kit::line([
            badge::Badge(#{ text: "Idle" }), badge::Badge(#{ text: "Queued", variant: "accent" }),
            badge::Badge(#{ text: "Healthy", variant: "success" }), badge::Badge(#{ text: "Degraded", variant: "warning" }),
            badge::Badge(#{ text: "Down", variant: "danger" }), badge::Badge(#{ text: "运行中", variant: "success" }),
        ])),
        kit::sec("Strong", kit::line([
            badge::Badge(#{ text: "Needs review", variant: "warning", emphasis: "strong" }),
            badge::Badge(#{ text: "Blocked", variant: "danger", emphasis: "strong" }),
            badge::Badge(#{ text: "Draft", emphasis: "strong" }),
        ])),
        kit::sec("Sizes", kit::line([badge::Badge(#{ text: "Medium" }), badge::Badge(#{ text: "Small", size: "sm" }),
            badge::Badge(#{ text: "Text only", dot: false })])),
    ])
}

fn tag_page(h, v) {
    let removed = kit::val(v, "tag.removed", ()) != ();
    let removable = if removed { kit::note("Removed.") }
        else { tag::Tag(#{ text: "可移除", closable: true, on_close: kit::store(h, "tag.removed") }) };
    kit::page(h, v, "tag", "Tag", "A category label on a flat tape, optionally with a facet segment.", [
        kit::sec("Categories", kit::line([
            tag::Tag(#{ text: "frontend" }), tag::Tag(#{ text: "accent", variant: "accent" }),
            tag::Tag(#{ text: "success", variant: "success" }), tag::Tag(#{ text: "warning", variant: "warning" }),
            tag::Tag(#{ text: "danger", variant: "danger" }),
        ])),
        kit::sec("Facets", kit::line([
            tag::Tag(#{ facet: "team", text: "infra" }), tag::Tag(#{ facet: "env", text: "staging", variant: "warning" }),
            tag::Tag(#{ facet: "region", text: "us-east", variant: "accent" }),
        ])),
        kit::sec("States", kit::line([removable, tag::Tag(#{ text: "disabled", disabled: true })])),
    ])
}

fn kbd_page(h, v) {
    kit::page(h, v, "kbd", "Kbd", "Key legends: framed keycaps, or an inline legend inside menus and buttons.", [
        kit::sec("Keycaps", kit::line([kbd::Kbd(#{ text: "Esc" }), kbd::Kbd(#{ text: "↵" }),
            kbd::Kbd(#{ keys: ["⌘", "⇧", "P"], label: "Command Shift P" })])),
        kit::sec("Inline", kit::line([kbd::Kbd(#{ text: "⌘K", appearance: "inline" }),
            kbd::Kbd(#{ text: "Ctrl+Shift+K", appearance: "inline" })])),
    ])
}
