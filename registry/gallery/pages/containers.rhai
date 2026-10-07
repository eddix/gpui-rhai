// Gallery spec pages: containers and window chrome.

import "gallery/kit" as kit;
import "components/card" as card;
import "components/group_box" as group_box;
import "components/alert" as alert;
import "components/empty" as empty;
import "components/collapsible" as collapsible;
import "components/divider" as divider;
import "components/scroll_area" as scroll_area;
import "components/split_pane" as split_pane;
import "components/resizable" as resizable;
import "components/title_bar" as title_bar;
import "components/status_bar" as status_bar;
import "components/button" as button;
import "components/badge" as badge;

fn pages() {
    [
        #{ id: "card", title: "Card", group: "Containers" },
        #{ id: "group_box", title: "GroupBox", group: "Containers" },
        #{ id: "alert", title: "Alert", group: "Containers" },
        #{ id: "empty", title: "Empty", group: "Containers" },
        #{ id: "collapsible", title: "Collapsible", group: "Containers" },
        #{ id: "divider", title: "Divider", group: "Containers" },
        #{ id: "scroll_area", title: "ScrollArea", group: "Containers" },
        #{ id: "split_pane", title: "SplitPane", group: "Containers" },
        #{ id: "resizable", title: "Resizable", group: "Containers" },
        #{ id: "title_bar", title: "TitleBar", group: "Containers" },
        #{ id: "status_bar", title: "StatusBar", group: "Containers" },
    ]
}

fn view(ctx, h, v, id) {
    if id == "card" { card_page(h, v) }
    else if id == "group_box" { group_box_page(h, v) }
    else if id == "alert" { alert_page(h, v) }
    else if id == "empty" { empty_page(h, v) }
    else if id == "collapsible" { collapsible_page(h, v) }
    else if id == "divider" { divider_page(h, v) }
    else if id == "scroll_area" { scroll_area_page(h, v) }
    else if id == "split_pane" { split_pane_page(h, v) }
    else if id == "resizable" { resizable_page(h, v) }
    else if id == "title_bar" { title_bar_page(h, v) }
    else if id == "status_bar" { status_bar_page(h, v) }
    else { () }
}

fn card_page(h, v) {
    kit::page(h, v, "card", "Card", "A color block; the outline variant is for cards on a raised surface.", [
        kit::sec("Variants", kit::line([
            card::Card(#{ header: text("Capacity"), content: text("128 hosts in 4 regions.") }).with_style(style().width(px(240))),
            card::Card(#{ header: text("Outline"), content: text("On a raised panel."), variant: "outline" })
                .with_style(style().width(px(240))),
        ])),
    ])
}

fn group_box_page(h, v) {
    kit::page(h, v, "group_box", "GroupBox", "A hairline rule and a label-voice heading on the content edge.", [
        kit::sec("Group", group_box::GroupBox(#{ label: "Notifications", description: "Where alerts are sent.",
            content: text("Email and 飞书") }).with_style(style().width(px(360)))),
    ])
}

fn alert_page(h, v) {
    kit::page(h, v, "alert", "Alert", "A quiet raised block marked by a square status lamp.", [
        kit::sec("Variants", kit::pile([
            alert::Alert(#{ title: "Deploy paused", description: "Two hosts failed health checks.", variant: "warning" }),
            alert::Alert(#{ title: "Rollback finished", variant: "success" }),
            alert::Alert(#{ title: "Quota exceeded", description: "New hosts cannot start.", variant: "danger",
                actions: [button::Button(#{ key: "alert-quota", text: "Request quota", size: "sm", on_click: h.noop })] }),
            alert::Alert(#{ title: "Maintenance on Sunday", variant: "info" }),
        ]).with_style(style().width(px(480)))),
    ])
}

fn empty_page(h, v) {
    kit::page(h, v, "empty", "Empty", "Quiet, centered type that says why it is empty and what to do next.", [
        kit::sec("State", empty::Empty(#{ title: "No hosts yet", description: "Register a host to see it here.",
            actions: [button::Button(#{ key: "empty-add", text: "Register host", variant: "primary", on_click: h.noop })] })),
    ])
}

fn collapsible_page(h, v) {
    kit::page(h, v, "collapsible", "Collapsible", "One disclosure row with the ink focus frame drawn over it.", [
        kit::sec("Disclosure", collapsible::Collapsible(#{ key: "collapsible-advanced", open: kit::val(v, "collapsible.open", true),
            trigger: text("Advanced options"), content: text("Scheduling, limits and probes."), content_height: 40,
            on_open_change: kit::store(h, "collapsible.open") }).with_style(style().width(px(420)))),
    ])
}

fn divider_page(h, v) {
    kit::page(h, v, "divider", "Divider", "A 1px hairline in the border role.", [
        kit::sec("Horizontal", column([text("Above"), divider::Divider(#{}), text("Below")])
            .with_style(style().flex_col().gap(theme_length("space.related")).width(px(320)))),
    ])
}

fn scroll_area_page(h, v) {
    let lines = [];
    for index in 1..41 { lines.push(text(`12:${index + 10} edge-0${index % 9} health check ok`)); }
    kit::page(h, v, "scroll_area", "ScrollArea", "Scrolls only; framing belongs to the layout around it.", [
        kit::sec("Logs", scroll_area::ScrollArea(#{ key: "scroll-logs", label: "Logs", height: px(200),
            content: column(lines).with_style(style().flex_col().typography("code").text_color(theme_color("text_muted")))})
            .with_style(style().width(px(420)))),
    ])
}

fn split_pane_page(h, v) {
    kit::page(h, v, "split_pane", "SplitPane", "An 8px grab zone with a hairline; keyboard focus turns it into an ink bar.", [
        kit::sec("Panes", column([split_pane::SplitPane(#{ key: "split-main", label: "Resize panes",
            size: kit::val(v, "split.size", 0.35), start: text("Navigation"), end: text("Content"),
            on_resize: kit::store(h, "split.size") })]).with_style(style().height(px(160)).width(px(520)))),
        kit::sec("Grip", column([split_pane::SplitPane(#{ key: "split-grip", label: "Resize panes with grip",
            size: kit::val(v, "split.grip_size", 0.5), start: text("Source"), end: text("Preview"),
            handle: grip_dots(), on_resize: kit::store(h, "split.grip_size") })])
            .with_style(style().height(px(160)).width(px(520)))),
    ])
}

// Two columns of three dots: a grip wider than the 8px lane, drawn above both panes.
fn grip_dot() { box([]).with_style(style().width(px(2)).height(px(2)).background(theme_color("text_muted"))) }
fn grip_column() { column([grip_dot(), grip_dot(), grip_dot()]).with_style(style().gap(px(3))) }
fn grip_dots() {
    row([grip_column(), grip_column()]).with_style(style().gap(px(3)).padding_x(px(3)).padding_y(px(6)))
}

fn resizable_page(h, v) {
    let rect = kit::val(v, "resizable.rect", #{ x: 12.0, y: 12.0, width: 220.0, height: 120.0 });
    kit::page(h, v, "resizable", "Resizable", "Edge and corner handles; focused handles turn ink.", [
        kit::sec("Panel", box([resizable::Resizable(#{ key: "resizable-panel", label: "Resize panel", rect: rect,
            min_width: 120.0, min_height: 80.0, max_width: 400.0, max_height: 200.0,
            handles: ["e", "s", "se"], content: card::Card(#{ content: text("Drag the edges") }),
            on_resize: kit::store(h, "resizable.rect") })]).with_style(style().relative().width(px(440)).height(px(240)))),
    ])
}

fn title_bar_page(h, v) {
    kit::page(h, v, "title_bar", "TitleBar", "Window chrome: title and subtitle, centered tools, end actions.", [
        kit::sec("Bar", title_bar::TitleBar(#{ label: "Example window", title: "Fleet console", subtitle: "production",
            end: [button::Button(#{ key: "title-bar-deploy", text: "Deploy", variant: "primary", size: "sm", on_click: h.noop })] })),
    ])
}

fn status_bar_page(h, v) {
    kit::page(h, v, "status_bar", "StatusBar", "Caption fields in muted text; the start truncates first.", [
        kit::sec("Bar", status_bar::StatusBar(#{ label: "Example status", start: [badge::Badge(#{ text: "Connected" })],
            end: [text("eu-west"), text("v0.2.0")] })),
    ])
}
