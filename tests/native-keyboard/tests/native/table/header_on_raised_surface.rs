//! Table's header is unfilled like its rows (#129): on a raised layer (Dialog, Sheet and
//! Popover are surface_raised) it took `surface` and became a band across the table.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::*;

use crate::support::{Mount, Step, filled, settle};

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(source)
        .motion(MotionPreference::None)
        .id("table")
        .mount(cx);
    settle(&mut visual, Step::Tick, 3);
    (visual, view)
}

const SCRIPT: &str = r#"import "components/table" as table;
fn view(ctx){box([
    table::Table(#{key:"rows",label:"Rows",row_key:"id",height:120,
        columns:[#{key:"name",title:"Name",width:#{kind:"flex",value:1.0}}],rows:[#{id:"1",name:"one"},#{id:"2",name:"two"}]})
]).with_style(style().width(px(320)).height(px(200)).background(theme_color("surface_raised")))}"#;

#[gpui::test]
fn a_table_on_a_raised_layer_has_no_band_of_another_surface(cx: &mut TestAppContext) {
    let (mut visual, _view) = mount(cx, SCRIPT.to_owned());
    // default_dark: surface 0x151412, surface_raised 0x1d1c19.
    let raised = filled(&mut visual, 0x1d1c_19ff);
    assert!(!raised.is_empty(), "the raised layer was painted");
    let surface = filled(&mut visual, 0x1514_12ff);
    assert!(
        surface.is_empty(),
        "nothing on the raised layer should be painted with `surface`, got {surface:?}"
    );
}
