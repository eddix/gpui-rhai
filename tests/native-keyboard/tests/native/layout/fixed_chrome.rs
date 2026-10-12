//! Bars and a region's header, toolbar and footer keep their height when the content beside
//! them is taller than the window. A flex column shrinks every item in proportion to its
//! size, so a body whose content is taller than its region took a share of the overflow from
//! the chrome around it: a 2000px page squeezed the region header and the shell's title and
//! status bars.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::{BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID};

use crate::support::{Mount, Step, settle};

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(source)
        .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(BUNDLED_PATTERN_SOURCES_BY_ID)
        .motion(MotionPreference::None)
        .id("chrome")
        .mount(cx);
    settle(&mut visual, Step::Tick, 3);
    (visual, view)
}

/// A shell whose page region holds a scrolling body `content` pixels tall.
fn shell(content: u32) -> String {
    format!(
        r#"import "patterns/app_shell" as app_shell;
import "layouts/region" as region;
fn view(ctx) {{
    // A two-line title: taller than the header's minimum, so the header has height to give.
    let title = column([text("Page").accessibility_role("heading").accessibility_level(1),
        text("What this page is for")]);
    let body = column([box([]).with_style(style().width(px(100)).height(px({content})))])
        .with_style(style().flex_col().flex_grow().min_height(px(0)).overflow_y_scroll());
    app_shell::AppShell(#{{ key: "app", label: "App", title_bar: #{{ title: "App" }},
        main: region::Region(#{{ label: "Page", title: title, body: body,
            toolbar: text("Tools"), footer: [text("Footer")] }}),
        status: #{{ start: [text("Ready")] }} }})
}}"#
    )
}

/// Heights of the title bar and status bar, and the page region's header, toolbar and
/// footer as offsets from the region's top and bottom edges.
fn chrome(cx: &mut TestAppContext, content: u32) -> [f64; 5] {
    let (mut visual, view) = mount(cx, shell(content));
    visual.update(|_, cx| {
        let tree = view.accessibility_snapshot(cx).unwrap();
        let layout = |role: &str, name: &str| {
            tree.find_by_role_and_name(role, name)
                .next()
                .and_then(|node| node.geometry)
                .unwrap_or_else(|| panic!("no {role} {name}"))
                .layout
        };
        let region = layout("region", "Page");
        [
            layout("toolbar", "App title bar").height,
            layout("statusbar", "App status").height,
            layout("heading", "Page").y - region.y,
            layout("text", "Tools").y - region.y,
            region.y + region.height - layout("text", "Footer").y,
        ]
    })
}

#[gpui::test]
fn chrome_keeps_its_height_beside_content_taller_than_the_window(cx: &mut TestAppContext) {
    let short = chrome(cx, 100);
    let tall = chrome(cx, 2000);
    let names = ["title bar", "status bar", "header", "toolbar", "footer"];
    for ((name, short), tall) in names.iter().zip(short).zip(tall) {
        assert!(
            (short - tall).abs() < 0.01,
            "{name}: {short} beside a short page, {tall} beside a tall one"
        );
    }
}
