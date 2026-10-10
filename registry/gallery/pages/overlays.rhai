// Gallery spec pages: overlays.

import "gallery/kit" as kit;
import "components/dialog" as dialog;
import "components/alert_dialog" as alert_dialog;
import "components/sheet" as sheet;
import "components/popover" as popover;
import "components/tooltip" as tooltip;
import "components/toast" as toast;
import "components/button" as button;
import "components/input" as input;
import "patterns/description_list" as description_list;

fn pages() {
    [
        #{ id: "dialog", title: "Dialog", group: "Overlays" },
        #{ id: "alert_dialog", title: "AlertDialog", group: "Overlays" },
        #{ id: "sheet", title: "Sheet", group: "Overlays" },
        #{ id: "popover", title: "Popover", group: "Overlays" },
        #{ id: "tooltip", title: "Tooltip", group: "Overlays" },
        #{ id: "toast", title: "Toast", group: "Overlays" },
    ]
}

fn view(ctx, h, v, id) {
    if id == "dialog" { dialog_page(h, v) }
    else if id == "alert_dialog" { alert_dialog_page(h, v) }
    else if id == "sheet" { sheet_page(h, v) }
    else if id == "popover" { popover_page(h, v) }
    else if id == "tooltip" { tooltip_page(h, v) }
    else if id == "toast" { toast_page(h, v) }
    else { () }
}

fn opener(h, key, label_text, state_key) {
    button::Button(#{ key: key, text: label_text, on_click: kit::put(h, state_key, true) })
}

fn dialog_page(h, v) {
    let close = kit::put(h, "dialog.open", false);
    let content = column([
        kit::note("Renaming changes the address of every host in the service."),
        input::Input(#{ key: "dialog-name", label: "New name", value: kit::val(v, "dialog.name", "api-gateway"),
            on_change: kit::store(h, "dialog.name") }),
    ]).with_style(style().flex_col().gap(theme_length("space.related")));
    let actions = [
        button::Button(#{ key: "dialog-cancel", text: "Cancel", variant: "ghost", on_click: close }),
        button::Button(#{ key: "dialog-save", text: "Rename", variant: "primary", on_click: close }),
    ];
    kit::page(h, v, "dialog", "Dialog", "A raised block with a hairline; actions sit at the end, the confirming one last.", [
        kit::sec("Open", kit::pile([opener(h, "dialog-open", "Rename service", "dialog.open"),
            dialog::Dialog(#{ key: "dialog-rename", open: kit::val(v, "dialog.open", false), title: "Rename service",
                content: content, actions: actions, initial_focus: "first", on_open_change: kit::store(h, "dialog.open") })])),
    ])
}

fn alert_dialog_page(h, v) {
    kit::page(h, v, "alert_dialog", "AlertDialog", "Destructive confirmation: the danger action is not the default for Enter.", [
        kit::sec("Open", kit::pile([opener(h, "alert-open", "Delete service", "alert.open"),
            alert_dialog::AlertDialog(#{ key: "alert-delete", open: kit::val(v, "alert.open", false),
                title: "Delete api-gateway?", description: "Its hosts stop serving traffic immediately.",
                confirm_label: "Delete", on_open_change: kit::store(h, "alert.open") })])),
    ])
}

fn sheet_page(h, v) {
    let details = description_list::DescriptionList(#{ label: "Host details", items: [
        #{ label: "Region", value: "eu-west" }, #{ label: "Address", value: "10.0.4.17", identifier: true },
    ] });
    kit::page(h, v, "sheet", "Sheet", "A raised side panel with a hairline only on the edge facing the window.", [
        kit::sec("Open", kit::pile([opener(h, "sheet-open", "Show details", "sheet.open"),
            sheet::Sheet(#{ key: "sheet-details", open: kit::val(v, "sheet.open", false), side: "end", title: "worker-07",
                content: details, on_open_change: kit::store(h, "sheet.open") })])),
    ])
}

fn popover_page(h, v) {
    let trigger = button::Button(#{ key: "popover-trigger", text: "Filters", on_click: kit::put(h, "popover.open", true) });
    kit::page(h, v, "popover", "Popover", "Anchored content aligned to the trigger's start edge.", [
        kit::sec("Open", popover::Popover(#{ key: "popover-filters", label: "Filters", trigger: trigger,
            open: kit::val(v, "popover.open", false), content: kit::note("Only hosts with alerts in the last hour."),
            on_open_change: kit::store(h, "popover.open") })),
    ])
}

fn tooltip_page(h, v) {
    kit::page(h, v, "tooltip", "Tooltip", "Ink and paper inverted, with the action's key legend.", [
        kit::sec("Hover", kit::line([
            tooltip::Tooltip(#{ key: "tooltip-deploy", label: "Deploy", shortcut: "⌘D",
                trigger: button::Button(#{ key: "tooltip-trigger", text: "Deploy", on_click: h.noop }),
                content: text("Deploy the selected service") }),
            tooltip::Tooltip(#{ key: "tooltip-help", label: "Help",
                trigger: button::Button(#{ key: "tooltip-help-trigger", text: "Help", variant: "ghost", on_click: h.noop }),
                content: text("Opens the documentation") }),
        ])),
    ])
}

fn toast_page(h, v) {
    let shown = kit::val(v, "toast.items", []);
    let push = kit::put(h, "toast.items", [
        #{ id: "saved", title: "Changes saved", variant: "success", duration_ms: 4000 },
        #{ id: "sync", title: "Sync delayed", message: "Retrying in 30 seconds.", variant: "warning", paused: true },
    ]);
    kit::page(h, v, "toast", "Toast", "Notifications as raised blocks; status toasts carry the square lamp.", [
        kit::sec("Show", kit::pile([
            button::Button(#{ key: "toast-show", text: "Show notifications", on_click: push }),
            toast::Toast(#{ key: "toast-region", items: shown, on_dismiss: kit::store(h, "toast.dismissed") }),
        ])),
    ])
}
