//! Chart has no fill and no frame (#131): its root and its native plot were both painted
//! with `surface`, so on a raised layer (Dialog, Sheet and Popover are surface_raised, as
//! are docked tool panels) every chart was a darker framed block.

use gpui::{TestAppContext, VisualTestContext, rgba};
use gpui_rhai::{MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::BUNDLED_CHART_SOURCES_BY_ID;

use crate::support::{Mount, Step, painted_quads, settle};

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(source)
        .modules(BUNDLED_CHART_SOURCES_BY_ID)
        .motion(MotionPreference::None)
        .id("table")
        .mount(cx);
    settle(&mut visual, Step::Tick, 3);
    (visual, view)
}

/// A line chart on a `surface_raised` box; `root` is appended to the chart's own style.
fn script(root: &str) -> String {
    format!(
        r#"import "charts/chart" as chart;
fn view(ctx){{box([
    chart::Chart(#{{key:"c",key_dimension:"id",data:[#{{id:"a",x:0,y:0}},#{{id:"b",x:1,y:1}}],
        spec:#{{title:"Control",series:[#{{key:"s",kind:"line",encode:#{{x:"x",y:"y"}}}}]}}}})
        .with_style(style().width(px(300)).height(px(160)){root})
]).with_style(style().width(px(320)).height(px(200)).background(theme_color("surface_raised")))}}"#
    )
}

/// Painted quads filled with `color`, as (width, height) in logical pixels.
fn filled(visual: &mut VisualTestContext, color: u32) -> Vec<(f32, f32)> {
    crate::support::filled(visual, color)
        .iter()
        .map(|quad| (quad.bounds.width, quad.bounds.height))
        .collect()
}

/// Painted quads with a top border in `color`.
fn framed(visual: &mut VisualTestContext, color: u32) -> usize {
    let border: gpui::Hsla = rgba(color).into();
    painted_quads(visual)
        .iter()
        .filter(|quad| quad.border_color == border && quad.border_widths[0] > 0.0)
        .count()
}

// default_dark: surface 0x151412, surface_raised 0x1d1c19, border 0x3a3833.
const SURFACE: u32 = 0x1514_12ff;
const SURFACE_RAISED: u32 = 0x1d1c_19ff;
const BORDER: u32 = 0x3a38_33ff;

#[gpui::test]
fn a_chart_on_a_raised_layer_has_no_block_of_another_surface(cx: &mut TestAppContext) {
    let (mut visual, _view) = mount(cx, script(""));
    let raised = filled(&mut visual, SURFACE_RAISED);
    assert!(!raised.is_empty(), "the raised layer was painted");
    let surface = filled(&mut visual, SURFACE);
    assert!(
        surface.is_empty(),
        "nothing on the raised layer should be painted with `surface`, got {surface:?}"
    );
    assert_eq!(framed(&mut visual, BORDER), 0, "the chart draws no frame");
}

#[gpui::test]
fn a_root_style_fills_and_frames_the_chart_once(cx: &mut TestAppContext) {
    let (mut visual, _view) = mount(
        cx,
        script(
            r#".background(theme_color("surface")).border(px(1)).border_color(theme_color("border"))"#,
        ),
    );
    // The root paints the fill and the frame (GPUI paints a border as its own quads); the
    // native plot inside it adds no fill.
    assert_eq!(filled(&mut visual, SURFACE), vec![(300.0, 160.0)]);
    assert!(
        framed(&mut visual, BORDER) > 0,
        "the root frame was painted"
    );
}
