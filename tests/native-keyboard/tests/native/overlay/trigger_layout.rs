//! An overlay lays its trigger out as the trigger would lay out without it
//! (feedback #26): a sidebar row wrapped in a ContextMenu still fills the
//! sidebar's width, short title or long, and a long title still truncates
//! instead of pushing the row past the sidebar.

use gpui::TestAppContext;
use gpui_rhai::*;

use crate::support::{Mount, last_error, node_by_test_id};

const SOURCE: &str = r#"import "components/context_menu" as context_menu;
fn noop(ctx,payload){()}
fn row_for(id,title){
    row([text(title).test_id(`title-${id}`)
        .with_style(style().whitespace_nowrap().text_ellipsis().min_width(px(0)))])
        .test_id(`row-${id}`).accessibility_role("group").accessibility_label(id)
        .with_style(style().padding_x(px(8)).min_width(px(0)))
}
fn view(ctx){
    column([
        row_for("plain","Plain"),
        context_menu::ContextMenu(#{key:"short-menu",label:"Short session",
            trigger:row_for("short","Short"),
            open:false,active_value:"rename",items:[#{kind:"item",value:"rename",label:"Rename"}],
            on_open_change:Fn("noop"),on_active_change:Fn("noop"),on_action:Fn("noop")}),
        context_menu::ContextMenu(#{key:"menu",label:"Session",
            trigger:row_for("wrapped","A session whose title is much longer than the sidebar is wide"),
            open:false,active_value:"rename",items:[#{kind:"item",value:"rename",label:"Rename"}],
            on_open_change:Fn("noop"),on_active_change:Fn("noop"),on_action:Fn("noop")}),
    ]).test_id("sidebar").accessibility_role("group").accessibility_label("Sidebar")
        .with_style(style().width(px(200)))
}"#;

#[gpui::test]
fn a_row_in_a_context_menu_fills_its_column_and_truncates(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(SOURCE)
        .motion(MotionPreference::None)
        .id("trigger")
        .mount(cx);
    visual.run_until_parked();
    assert_eq!(last_error(&mut visual, &view), None);
    let visual_of = |visual: &mut gpui::VisualTestContext, id: &str| {
        node_by_test_id(visual, &view, id).geometry.unwrap().visual
    };
    let sidebar = visual_of(&mut visual, "sidebar");
    let plain = visual_of(&mut visual, "row-plain");
    let short = visual_of(&mut visual, "row-short");
    let wrapped = visual_of(&mut visual, "row-wrapped");
    let title = visual_of(&mut visual, "title-wrapped");
    assert!((plain.width - sidebar.width).abs() < 0.5, "{plain:?} {sidebar:?}");
    assert!(
        (short.width - plain.width).abs() < 0.5,
        "a short wrapped row stretches as the plain one does: {short:?} {plain:?}"
    );
    assert!(
        (wrapped.width - plain.width).abs() < 0.5,
        "the wrapped row is as wide as the plain one: {wrapped:?} {plain:?}"
    );
    assert!(
        title.x + title.width <= sidebar.x + sidebar.width + 0.5,
        "the title truncates inside the sidebar: {title:?} {sidebar:?}"
    );
}
