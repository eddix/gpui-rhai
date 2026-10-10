// Gallery spec pages: display components.

import "gallery/kit" as kit;
import "components/avatar" as avatar;
import "components/icon" as icon;
import "components/progress" as progress;
import "components/skeleton" as skeleton;
import "components/spinner" as spinner;
import "components/code_viewer" as code_viewer;
import "components/diff_viewer" as diff_viewer;
import "components/button" as button;

fn pages() {
    [
        #{ id: "avatar", title: "Avatar", group: "Display" },
        #{ id: "icon", title: "Icon", group: "Display" },
        #{ id: "progress", title: "Progress", group: "Display" },
        #{ id: "skeleton", title: "Skeleton", group: "Display" },
        #{ id: "spinner", title: "Spinner", group: "Display" },
        #{ id: "code_viewer", title: "CodeViewer", group: "Display" },
        #{ id: "diff_viewer", title: "DiffViewer", group: "Display" },
    ]
}

fn view(ctx, h, v, id) {
    if id == "avatar" { avatar_page(h, v) }
    else if id == "icon" { icon_page(h, v) }
    else if id == "progress" { progress_page(h, v) }
    else if id == "skeleton" { skeleton_page(h, v) }
    else if id == "spinner" { spinner_page(h, v) }
    else if id == "code_viewer" { code_viewer_page(h, v) }
    else if id == "diff_viewer" { diff_viewer_page(h, v) }
    else { () }
}

fn avatar_page(h, v) {
    kit::page(h, v, "avatar", "Avatar", "Avatars take the control height of their size, so they sit in toolbars and rows.", [
        kit::sec("Sizes", kit::sizes([
            avatar::Avatar(#{ name: "Ada Lovelace", initials: "AL", size: "xs" }),
            avatar::Avatar(#{ name: "Grace Hopper", initials: "GH", size: "sm", presence: "online" }),
            avatar::Avatar(#{ name: "Edsger Dijkstra", initials: "ED", presence: "away" }),
            avatar::Avatar(#{ name: "王小明", initials: "王", size: "lg", presence: "busy" }),
        ])),
    ])
}

fn icon_page(h, v) {
    let names = ["check", "close", "search", "plus", "minus", "info", "warning", "calendar"];
    let icons = [];
    for name in names { icons.push(icon::Icon(#{ source: asset(`app/icons/${name}`), label: name })); }
    kit::page(h, v, "icon", "Icon", "Icons take the color of the text around them; without a size they follow the control size.", [
        kit::sec("Set", kit::line(icons)),
        kit::sec("In a control", kit::line([
            button::Button(#{ key: "icon-in-button", text: "Search", prefix: icon::Icon(#{ source: asset("app/icons/search") }),
                on_click: h.noop }),
            button::Button(#{ key: "icon-in-primary", text: "Add host", variant: "primary",
                prefix: icon::Icon(#{ source: asset("app/icons/plus") }), on_click: h.noop }),
        ])),
    ])
}

fn progress_page(h, v) {
    kit::page(h, v, "progress", "Progress", "A 4px track with an accent fill; indeterminate moves a stripe.", [
        kit::sec("States", kit::pile([
            progress::Progress(#{ key: "progress-upload", value: 60.0, max: 100.0, label: "Upload", width: 320 }),
            progress::Progress(#{ key: "progress-scan", label: "Scanning", indeterminate: true, width: 320 }),
        ])),
    ])
}

fn skeleton_page(h, v) {
    kit::page(h, v, "skeleton", "Skeleton", "Placeholders for fixed shapes while content loads.", [
        kit::sec("Shapes", kit::pile([
            skeleton::Skeleton(#{ key: "skeleton-title", width: 240, height: 22, animated: false }),
            skeleton::Skeleton(#{ key: "skeleton-line", width: 360, height: 14, animated: false }),
        ])),
    ])
}

fn spinner_page(h, v) {
    kit::page(h, v, "spinner", "Spinner", "Indeterminate activity in the accent color.", [
        kit::sec("Sizes", kit::line([
            spinner::Spinner(#{ key: "spinner-sm", label: "Loading", size: "sm" }),
            spinner::Spinner(#{ key: "spinner-md", label: "Loading" }),
            spinner::Spinner(#{ key: "spinner-lg", label: "Loading", size: "lg" }),
        ])),
    ])
}

fn sample_source() {
    "fn view(ctx) {\n    button::Button(#{ text: \"Deploy\", variant: \"primary\" })\n}\n"
}

fn code_viewer_page(h, v) {
    kit::page(h, v, "code_viewer", "CodeViewer", "Syntax-highlighted source with a line gutter; no frame of its own.", [
        kit::sec("Rhai", column([code_viewer::CodeViewer(#{ key: "code-sample", label: "Sample source", source: sample_source(),
            file_name: "main.rhai", language: "rhai" })]).with_style(style().height(px(160)))),
    ])
}

fn diff_viewer_page(h, v) {
    let left = #{ source: "replicas = 3\nregion = \"eu-west\"\n", label: "Current" };
    let right = #{ source: "replicas = 5\nregion = \"eu-west\"\nzone = \"b\"\n", label: "Proposed" };
    kit::page(h, v, "diff_viewer", "DiffViewer", "Unified or split comparison with folded context.", [
        kit::sec("Unified", column([diff_viewer::DiffViewer(#{ key: "diff-sample", left: left, right: right, mode: "unified" })])
            .with_style(style().height(px(180)))),
    ])
}
